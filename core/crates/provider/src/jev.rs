//! Jev `DecisionProvider` over the documented TypeSafe HTTP contract.
//!
//! Compiled only with `--features jev` (OFF by default — the daemon keeps using
//! `MockProvider` until the redact + consent + shadow gates clear per ADR-0009).
//!
//! Wire shape mirrors `eval/jev_client/client.py`, which was built ONLY from the
//! public spec (https://docs.typesafe.ai/api, https://docs.typesafe.ai/models,
//! verified 2026-09-20 against the public spec):
//! ```text
//! POST {base}/v1/systemone, Authorization: Bearer <key>
//! body {"state", "model", "questions": {id: {"type","instructions","criteria"}}}
//! ok   {"model", "answers": {id: Answer}, "usage": {...}}
//! err  401 / 422 / 429 / 529
//! ```
//! Strictness (fail-safe): anything outside the spec shape — missing answers,
//! wrong types, unknown options, non-finite or out-of-range numbers — maps to
//! `ProviderError::{Parse,…}`, which the caller maps to `ask`, never `allow`.
//! Transport + serde only: no policy, no thresholds, no training logs.
//!
//! Privacy: the API key travels in memory and in the `Authorization` header only —
//! it is never logged, never appears in error strings, and `Debug` redacts it.
//! The caller MUST pass an already-redacted event (daemon redacts first); the
//! `state` sent carries only `tool_kind` + `redacted_payload` (privacy-minimal).

use std::time::Duration;

use serde_json::Value;

use crate::trait_def::{DecisionProvider, ProviderError, TypedAnswer, TypedAnswers, TypedQuestion};
use algo_types::{ToolBefore, ToolKind};

/// Default endpoint (override via `ALGO_JEV_BASE_URL`, e.g. tests).
pub const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";
/// Spec path (verified 2026-09-20).
pub const API_PATH: &str = "/v1/systemone";
/// Default model alias (server echoes the versioned id per request).
pub const DEFAULT_MODEL: &str = "jev-latest";
/// Per-attempt timeout (plan P1-05: 700ms).
pub const REQUEST_TIMEOUT: Duration = Duration::from_millis(700);
/// Single retry wait on 429/529 (plan P1-05: 200ms budget).
pub const RETRY_WAIT: Duration = Duration::from_millis(200);

/// Real Jev client: pooled blocking HTTP (sync trait), one batched request for
/// all questions, 700ms per-attempt timeout, one retry on 429/529.
///
/// DROP RULE: the inner blocking client owns a lazily-built tokio runtime, so a
/// `JevProvider` must be dropped on a blocking thread (or after the async
/// runtime shuts down) — dropping it inside async context panics on the runtime
/// drop. Long-lived pools (daemon) uphold this naturally; tests do it explicitly.
pub struct JevProvider {
    client: reqwest::blocking::Client,
    api_key: String,
    base_url: String,
    model: String,
    timeout: Duration,
    retry_wait: Duration,
}

// Manual Debug: the key must never appear in logs or dumps.
impl std::fmt::Debug for JevProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JevProvider")
            .field("api_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .field("retry_wait", &self.retry_wait)
            .finish_non_exhaustive()
    }
}

/// Outcome of one HTTP attempt: success, one retry (429/529), or a typed error.
enum SendOutcome {
    Ok(Value),
    /// Retry once; carries the capped wait (`retry-after` clamped to the budget).
    Retry(Option<Duration>),
    Err(ProviderError),
}

impl JevProvider {
    /// Key + endpoint + model are caller-supplied (daemon reads `ALGO_JEV_*` env).
    /// Empty key is an `Auth` error (config problem → caller maps to ask).
    ///
    /// BUILD/DROP RULE: the blocking client builds (and later drops) a helper
    /// tokio runtime, so `new` must run **outside async context** (plain thread
    /// or `spawn_blocking`) — building inside async code panics. Same for drop:
    /// keep the provider in a pool that outlives the async runtime, or drop it
    /// on a blocking thread.
    pub fn new(api_key: String, base_url: String, model: String) -> Result<Self, ProviderError> {
        if api_key.is_empty() {
            // No payload on Auth by design (trait_def): detail lives in the
            // caller-side reason string, never alongside the key.
            return Err(ProviderError::Auth);
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| ProviderError::Net(format!("http client build: {e}")))?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            timeout: REQUEST_TIMEOUT,
            retry_wait: RETRY_WAIT,
        })
    }

    /// Key from env-only (`ALGO_JEV_API_KEY`); endpoint/model overridable for tests.
    /// Never reads files, argv, or logs the key.
    pub fn from_env() -> Result<Self, ProviderError> {
        let key = std::env::var("ALGO_JEV_API_KEY").unwrap_or_default();
        let base =
            std::env::var("ALGO_JEV_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let model = std::env::var("ALGO_JEV_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        Self::new(key, base, model)
    }

    /// Test-only shorter budgets (keeps the suite fast). Rebuilds the client so
    /// the per-request timeout actually applies.
    #[cfg(test)]
    fn with_timeouts(
        mut self,
        timeout: Duration,
        retry_wait: Duration,
    ) -> Result<Self, ProviderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProviderError::Net(format!("http client build: {e}")))?;
        self.client = client;
        self.timeout = timeout;
        self.retry_wait = retry_wait;
        Ok(self)
    }

    /// Build the spec request body. Validates question shapes client-side
    /// (choice 2..=255 options, score 2..=10 levels — spec limits from the dossier).
    fn build_body(
        event: &ToolBefore,
        model: &str,
        qs: &[TypedQuestion],
    ) -> Result<Value, ProviderError> {
        if qs.is_empty() {
            return Err(ProviderError::Parse("no questions in batch".to_string()));
        }
        // Privacy-minimal state: kind + redacted payload only. The caller MUST
        // have redacted first (daemon redacts before judging); nothing else
        // from the event (session, paths, identity) leaves the machine here.
        let kind = ToolKind::try_from(event.tool_kind)
            .map(|k| k.as_str_name().to_string())
            .unwrap_or_else(|_| ToolKind::Other.as_str_name().to_string());
        let mut questions = serde_json::Map::new();
        for q in qs {
            let (id, obj) = Self::build_question(q)?;
            if questions.insert(id.clone(), obj).is_some() {
                return Err(ProviderError::Parse(format!(
                    "duplicate question id '{id}'"
                )));
            }
        }
        Ok(serde_json::json!({
            "state": {"tool_kind": kind, "redacted_payload": event.redacted_payload},
            "model": model,
            "questions": questions,
        }))
    }

    fn build_question(q: &TypedQuestion) -> Result<(String, Value), ProviderError> {
        match q {
            TypedQuestion::Bool { id, prompt } => Ok((
                id.clone(),
                serde_json::json!({"type": "noul", "instructions": prompt}),
            )),
            TypedQuestion::Choice {
                id,
                prompt,
                options,
            } => {
                if !(2..=255).contains(&options.len()) {
                    return Err(ProviderError::Parse(format!(
                        "choice '{id}' needs 2..=255 options per spec, got {}",
                        options.len()
                    )));
                }
                let criteria: serde_json::Map<String, Value> =
                    options.iter().map(|o| (o.clone(), Value::Null)).collect();
                Ok((
                    id.clone(),
                    serde_json::json!({"type": "choice", "instructions": prompt, "criteria": criteria}),
                ))
            }
            TypedQuestion::Score { id, prompt, levels } => {
                if !(2..=10).contains(&levels.len()) {
                    return Err(ProviderError::Parse(format!(
                        "score '{id}' needs 2..=10 levels per spec, got {}",
                        levels.len()
                    )));
                }
                Ok((
                    id.clone(),
                    serde_json::json!({"type": "score", "instructions": prompt, "criteria": levels}),
                ))
            }
        }
    }

    /// POST with one retry on 429/529 (wait capped at the retry budget).
    fn post(&self, body: &Value) -> Result<Value, ProviderError> {
        let url = format!("{}{}", self.base_url, API_PATH);
        let mut retried = false;
        loop {
            match self.send_once(&url, body) {
                SendOutcome::Ok(v) => return Ok(v),
                SendOutcome::Err(e) => return Err(e),
                SendOutcome::Retry(wait) => {
                    if retried {
                        return Err(ProviderError::Net(
                            "429/529 exhausted after 1 retry".to_string(),
                        ));
                    }
                    retried = true;
                    std::thread::sleep(wait.unwrap_or(self.retry_wait));
                }
            }
        }
    }

    fn send_once(&self, url: &str, body: &Value) -> SendOutcome {
        // Key travels in the header from memory only — never in the URL, never logged.
        let resp = match self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body)
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return SendOutcome::Err(ProviderError::Timeout),
            Err(e) => return SendOutcome::Err(ProviderError::Net(transport_msg(&e))),
        };
        let status = resp.status();
        if status.is_success() {
            let data: Value = match resp.json() {
                Ok(v) => v,
                Err(_) => {
                    return SendOutcome::Err(ProviderError::Parse("unparsable 200 body".into()))
                }
            };
            if data.get("answers").and_then(Value::as_object).is_none()
                || data.get("model").and_then(Value::as_str).is_none()
            {
                return SendOutcome::Err(ProviderError::Parse(
                    "200 body outside spec shape (need model+answers)".into(),
                ));
            }
            return SendOutcome::Ok(data);
        }
        match status.as_u16() {
            401 => SendOutcome::Err(ProviderError::Auth),
            422 => SendOutcome::Err(ProviderError::Parse(
                "422 request failed validation (client bug)".into(),
            )),
            429 | 529 => SendOutcome::Retry(retry_after(resp.headers(), self.retry_wait)),
            code => SendOutcome::Err(ProviderError::Net(format!("unexpected status {code}"))),
        }
    }

    /// Strict per-question answer parsing. Unknown shapes/options and
    /// non-finite or out-of-range numbers are `Parse` (→ ask), never coerced.
    fn parse_answers(data: &Value, qs: &[TypedQuestion]) -> Result<TypedAnswers, ProviderError> {
        let answers = data
            .get("answers")
            .and_then(Value::as_object)
            .ok_or_else(|| ProviderError::Parse("missing answers object".to_string()))?;
        qs.iter()
            .map(|q| {
                let (id, kind) = match q {
                    TypedQuestion::Bool { id, .. } => (id, "noul"),
                    TypedQuestion::Choice { id, .. } => (id, "choice"),
                    TypedQuestion::Score { id, .. } => (id, "score"),
                };
                let a = answers
                    .get(id)
                    .and_then(Value::as_object)
                    .ok_or_else(|| ProviderError::Parse(format!("missing answer '{id}'")))?;
                if a.get("type").and_then(Value::as_str) != Some(kind) {
                    return Err(ProviderError::Parse(format!(
                        "answer '{id}' wrong type (expected {kind})"
                    )));
                }
                match q {
                    TypedQuestion::Bool { id, .. } => {
                        let p = a
                            .get("probability")
                            .and_then(Value::as_f64)
                            .ok_or_else(|| {
                                ProviderError::Parse(format!("answer '{id}' missing probability"))
                            })?;
                        if !(0.0..=1.0).contains(&p) {
                            return Err(ProviderError::Parse(format!(
                                "answer '{id}' probability out of range"
                            )));
                        }
                        // OUR mapping convention (documented, not vendor behavior):
                        // threshold at 0.5, confidence = distance from the boundary.
                        Ok(TypedAnswer::Bool {
                            id: id.clone(),
                            value: p >= 0.5,
                            confidence: (2.0 * (p - 0.5)).abs(),
                        })
                    }
                    TypedQuestion::Choice { id, options, .. } => {
                        let c = a.get("choice").and_then(Value::as_str).ok_or_else(|| {
                            ProviderError::Parse(format!("answer '{id}' missing choice"))
                        })?;
                        if !options.iter().any(|o| o == c) {
                            return Err(ProviderError::Parse(format!(
                                "answer '{id}' unknown option"
                            )));
                        }
                        let confidence = finite_unit(a.get("confidence"), id, "confidence")?;
                        Ok(TypedAnswer::Choice {
                            id: id.clone(),
                            choice: c.to_string(),
                            confidence,
                        })
                    }
                    TypedQuestion::Score { id, .. } => {
                        let score = a.get("score").and_then(Value::as_f64).ok_or_else(|| {
                            ProviderError::Parse(format!("answer '{id}' missing score"))
                        })?;
                        if !score.is_finite() {
                            return Err(ProviderError::Parse(format!(
                                "answer '{id}' non-finite score"
                            )));
                        }
                        let confidence = finite_unit(a.get("confidence"), id, "confidence")?;
                        Ok(TypedAnswer::Score {
                            id: id.clone(),
                            score,
                            confidence,
                        })
                    }
                }
            })
            .collect()
    }
}

/// Finite f64 in [0,1], else `Parse` (→ ask).
fn finite_unit(v: Option<&Value>, id: &str, field: &str) -> Result<f64, ProviderError> {
    let x = v
        .and_then(Value::as_f64)
        .ok_or_else(|| ProviderError::Parse(format!("answer '{id}' missing {field}")))?;
    if !(0.0..=1.0).contains(&x) {
        return Err(ProviderError::Parse(format!(
            "answer '{id}' {field} out of range"
        )));
    }
    Ok(x)
}

/// Transport error text. reqwest never echoes request headers (where the key
/// lives) — only the URL, which carries no key. Tests assert the sentinel key
/// never appears in any error string we produce.
fn transport_msg(e: &reqwest::Error) -> String {
    format!("transport: {e}")
}

/// `retry-after` (seconds) clamped DOWN to the retry budget. Absent/unparsable → None (caller uses the budget).
fn retry_after(headers: &reqwest::header::HeaderMap, budget: Duration) -> Option<Duration> {
    let v: f64 = headers
        .get("retry-after")?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    if v <= 0.0 || !v.is_finite() {
        return None;
    }
    Some(Duration::from_secs_f64(v).min(budget))
}

impl DecisionProvider for JevProvider {
    fn judge(
        &self,
        event: &ToolBefore,
        qs: &[TypedQuestion],
    ) -> Result<TypedAnswers, ProviderError> {
        let body = Self::build_body(event, &self.model, qs)?;
        let data = self.post(&body)?;
        Self::parse_answers(&data, qs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::map_to_ask;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use std::thread;

    /// One scripted HTTP response per accepted connection.
    struct Script {
        status: u16,
        body: &'static str,
        delay_ms: u64,
    }

    struct Stub {
        url: String,
        hits: Arc<AtomicUsize>,
        bodies: Arc<Mutex<Vec<String>>>,
    }

    /// Minimal std-only HTTP stub (no wiremock dep): serves the script in order,
    /// records raw request bodies, counts connections.
    fn start_stub(script: Vec<Script>) -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let url = format!("http://{}", listener.local_addr().expect("stub addr"));
        let hits = Arc::new(AtomicUsize::new(0));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let (h, b) = (Arc::clone(&hits), Arc::clone(&bodies));
        thread::spawn(move || {
            for spec in script {
                let Ok((stream, _)) = listener.accept() else {
                    break;
                };
                h.fetch_add(1, Ordering::SeqCst);
                if spec.delay_ms > 0 {
                    thread::sleep(Duration::from_millis(spec.delay_ms));
                }
                let mut reader = BufReader::new(stream.try_clone().expect("clone stub stream"));
                let mut content_len = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let t = line.trim_end().to_ascii_lowercase();
                    if t.is_empty() {
                        break;
                    }
                    if let Some(v) = t.strip_prefix("content-length:") {
                        content_len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0u8; content_len];
                let _ = reader.read_exact(&mut body);
                b.lock()
                    .expect("stub lock")
                    .push(String::from_utf8_lossy(&body).into_owned());
                let mut stream = reader.into_inner();
                let resp = format!(
                    "HTTP/1.1 {} x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    spec.status,
                    spec.body.len(),
                    spec.body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        Stub { url, hits, bodies }
    }

    const SENTINEL: &str = "SENTINEL_KEY_ABC123";

    fn provider_for(url: &str) -> JevProvider {
        JevProvider::new(
            SENTINEL.to_string(),
            url.to_string(),
            "jev-test".to_string(),
        )
        .expect("provider")
        .with_timeouts(Duration::from_millis(120), Duration::from_millis(5))
        .expect("budgets")
    }

    fn tool_before() -> ToolBefore {
        ToolBefore {
            event_id: "evt-1".into(),
            timestamp: None,
            agent: None,
            tool_kind: ToolKind::Shell as i32,
            redacted_payload: "ls -la".into(),
            privacy_mode: 0,
            shell_argv: vec!["ls".into()],
            file_path: None,
        }
    }

    fn decision_q() -> Vec<TypedQuestion> {
        vec![TypedQuestion::Choice {
            id: "decision".into(),
            prompt: "Choose.".into(),
            options: vec!["allow".into(), "ask".into(), "deny".into()],
        }]
    }

    fn ok_choice(choice: &str) -> &'static str {
        match choice {
            "allow" => {
                r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"allow","confidence":0.9}},"usage":{}}"#
            }
            _ => {
                r#"{"model":"jev-1.13.0","answers":{"decision":{"type":"choice","choice":"ask","confidence":0.5}},"usage":{}}"#
            }
        }
    }

    #[test]
    fn proves_timeout_maps_to_ask() {
        let stub = start_stub(vec![Script {
            status: 200,
            body: ok_choice("ask"),
            delay_ms: 500,
        }]);
        let p = provider_for(&stub.url);
        let err = p
            .judge(&tool_before(), &decision_q())
            .expect_err("must time out");
        assert_eq!(err, ProviderError::Timeout);
        // Caller maps every provider error to ask (never allow).
        let d = map_to_ask(err, "t");
        assert_eq!(d.action, algo_types::Action::Ask as i32);
        assert!(!format!("{d:?}").contains(SENTINEL));
    }

    #[test]
    fn success_choice_parses_and_posts_spec_shape() {
        let stub = start_stub(vec![Script {
            status: 200,
            body: ok_choice("allow"),
            delay_ms: 0,
        }]);
        let p = provider_for(&stub.url);
        let ans = p.judge(&tool_before(), &decision_q()).expect("200 parses");
        assert_eq!(
            ans,
            vec![TypedAnswer::Choice {
                id: "decision".into(),
                choice: "allow".into(),
                confidence: 0.9
            }]
        );
        assert_eq!(stub.hits.load(Ordering::SeqCst), 1);
        // Spec request shape: state + model + typed questions (mirrors eval/jev_client).
        let bodies = stub.bodies.lock().expect("lock");
        let sent: Value = serde_json::from_str(&bodies[0]).expect("request is JSON");
        assert_eq!(sent["model"], "jev-test");
        assert_eq!(sent["state"]["redacted_payload"], "ls -la");
        assert_eq!(sent["questions"]["decision"]["type"], "choice");
        assert_eq!(sent["questions"]["decision"]["instructions"], "Choose.");
    }

    #[test]
    fn auth_401_maps_to_auth() {
        let stub = start_stub(vec![Script {
            status: 401,
            body: "{}",
            delay_ms: 0,
        }]);
        let p = provider_for(&stub.url);
        let err = p.judge(&tool_before(), &decision_q()).expect_err("401");
        assert_eq!(err, ProviderError::Auth);
        assert!(!format!("{err}").contains(SENTINEL));
    }

    #[test]
    fn validation_422_maps_to_parse() {
        let stub = start_stub(vec![Script {
            status: 422,
            body: "{}",
            delay_ms: 0,
        }]);
        let p = provider_for(&stub.url);
        let err = p.judge(&tool_before(), &decision_q()).expect_err("422");
        assert!(matches!(err, ProviderError::Parse(_)));
    }

    #[test]
    fn retry_429_then_200_succeeds_once() {
        let stub = start_stub(vec![
            Script {
                status: 429,
                body: "{}",
                delay_ms: 0,
            },
            Script {
                status: 200,
                body: ok_choice("ask"),
                delay_ms: 0,
            },
        ]);
        let p = provider_for(&stub.url);
        let ans = p.judge(&tool_before(), &decision_q()).expect("retry wins");
        assert!(matches!(ans[0], TypedAnswer::Choice { .. }));
        assert_eq!(stub.hits.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn exhausted_429_maps_to_net() {
        let stub = start_stub(vec![
            Script {
                status: 429,
                body: "{}",
                delay_ms: 0,
            },
            Script {
                status: 429,
                body: "{}",
                delay_ms: 0,
            },
        ]);
        let p = provider_for(&stub.url);
        let err = p.judge(&tool_before(), &decision_q()).expect_err("429 x2");
        assert!(matches!(err, ProviderError::Net(_)));
        assert_eq!(stub.hits.load(Ordering::SeqCst), 2);
        assert!(!format!("{err}").contains(SENTINEL));
    }

    #[test]
    fn malformed_and_mismatched_bodies_are_parse() {
        for (name, body) in [
            ("bad json", "not-json{{{"),
            ("missing answers", r#"{"model":"x"}"#),
            ("missing id", r#"{"model":"x","answers":{}}"#),
            (
                "wrong type",
                r#"{"model":"x","answers":{"decision":{"type":"noul","probability":0.1}}}"#,
            ),
            (
                "unknown option",
                r#"{"model":"x","answers":{"decision":{"type":"choice","choice":"maybe","confidence":0.5}}}"#,
            ),
        ] {
            let stub = start_stub(vec![Script {
                status: 200,
                body,
                delay_ms: 0,
            }]);
            let p = provider_for(&stub.url);
            let err = p
                .judge(&tool_before(), &decision_q())
                .expect_err(format!("{name} must not parse").as_str());
            assert!(matches!(err, ProviderError::Parse(_)), "{name}");
        }
    }

    #[test]
    fn bool_and_score_answers_parse() {
        let qs = vec![
            TypedQuestion::Bool {
                id: "b".into(),
                prompt: "Safe?".into(),
            },
            TypedQuestion::Score {
                id: "s".into(),
                prompt: "Risk?".into(),
                levels: vec!["low".into(), "high".into()],
            },
        ];
        let body = r#"{"model":"m","answers":{
            "b":{"type":"noul","probability":0.8},
            "s":{"type":"score","score":0.7,"confidence":0.6}}}"#;
        let stub = start_stub(vec![Script {
            status: 200,
            body,
            delay_ms: 0,
        }]);
        let p = provider_for(&stub.url);
        let ans = p.judge(&tool_before(), &qs).expect("typed answers parse");
        // probability 0.8 -> value=true, confidence=|2p-1| (float compare with epsilon).
        match &ans[0] {
            TypedAnswer::Bool {
                id,
                value,
                confidence,
            } => {
                assert_eq!(id, "b");
                assert!(value);
                assert!((confidence - 0.6).abs() < 1e-12);
            }
            other => panic!("expected Bool, got {other:?}"),
        }
        assert_eq!(
            ans[1],
            TypedAnswer::Score {
                id: "s".into(),
                score: 0.7,
                confidence: 0.6
            }
        );
    }

    #[test]
    fn invalid_question_shapes_rejected_client_side() {
        let p = provider_for("http://127.0.0.1:1");
        // No request is sent: validation happens before any network.
        let bad_choice = vec![TypedQuestion::Choice {
            id: "c".into(),
            prompt: "x".into(),
            options: vec!["only-one".into()],
        }];
        assert!(matches!(
            p.judge(&tool_before(), &bad_choice),
            Err(ProviderError::Parse(_))
        ));
        let empty: Vec<TypedQuestion> = vec![];
        assert!(matches!(
            p.judge(&tool_before(), &empty),
            Err(ProviderError::Parse(_))
        ));
    }

    #[test]
    fn empty_key_is_auth_and_debug_redacts_key() {
        let err =
            JevProvider::new(String::new(), "http://x".into(), "m".into()).expect_err("empty key");
        assert!(matches!(err, ProviderError::Auth));
        let p = provider_for("http://127.0.0.1:1");
        let dbg = format!("{p:?}");
        assert!(!dbg.contains(SENTINEL));
        assert!(dbg.contains("[REDACTED]"));
    }

    #[test]
    fn proves_ask_on_all_jev_variants() {
        for err in [
            ProviderError::Timeout,
            ProviderError::Auth,
            ProviderError::Net("x".into()),
            ProviderError::Parse("x".into()),
        ] {
            let d = map_to_ask(err, "t");
            assert_eq!(d.action, algo_types::Action::Ask as i32);
            assert_eq!(d.source_level, algo_types::SourceLevel::Fallback as i32);
        }
    }

    #[test]
    fn question_builders_match_spec() {
        // Request shapes mirror eval/jev_client builders (spec §"Question types").
        let qs = vec![
            TypedQuestion::Bool {
                id: "b".into(),
                prompt: "Safe?".into(),
            },
            TypedQuestion::Choice {
                id: "c".into(),
                prompt: "Pick.".into(),
                options: vec!["a".into(), "b".into()],
            },
            TypedQuestion::Score {
                id: "s".into(),
                prompt: "Rate.".into(),
                levels: vec!["l0".into(), "l1".into()],
            },
        ];
        let body = JevProvider::build_body(&tool_before(), "m", &qs).expect("builds");
        assert_eq!(
            body["questions"]["b"],
            serde_json::json!({"type": "noul", "instructions": "Safe?"})
        );
        assert_eq!(
            body["questions"]["c"]["criteria"],
            serde_json::json!({"a": null, "b": null})
        );
        assert_eq!(
            body["questions"]["s"]["criteria"],
            serde_json::json!(["l0", "l1"])
        );
    }

    #[test]
    fn from_env_missing_key_is_auth() {
        // Save/restore so the suite never depends on ambient env.
        let saved = std::env::var("ALGO_JEV_API_KEY").ok();
        std::env::remove_var("ALGO_JEV_API_KEY");
        let err = JevProvider::from_env().expect_err("no key");
        assert!(matches!(err, ProviderError::Auth));
        if let Some(v) = saved {
            std::env::set_var("ALGO_JEV_API_KEY", v);
        }
    }
}
