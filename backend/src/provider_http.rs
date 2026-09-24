//! Shared HTTPS helpers for OAuth providers with fail-closed bounds.
//!
//! Every provider response body is size-capped BEFORE parsing (a compromised
//! or middleboxed endpoint must not be able to OOM the backend by streaming
//! gigabytes into `resp.json()`). Timeouts come from the shared client.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    Transport,
    TooLarge,
    BadJson,
    BadStatus(u16),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => write!(f, "provider unreachable"),
            Self::TooLarge => write!(f, "provider response too large"),
            Self::BadJson => write!(f, "provider protocol error"),
            Self::BadStatus(_) => write!(f, "provider protocol error"),
        }
    }
}
impl std::error::Error for HttpError {}

/// Read a response with an explicit byte cap, then parse JSON.
pub async fn bounded_json(resp: reqwest::Response, max_bytes: usize) -> Result<Value, HttpError> {
    // Pre-check declared length when present (fail fast, still verify below).
    // Dropping here keeps a poisoned oversized body out of the pool.
    if let Some(len) = resp.content_length() {
        if len > max_bytes as u64 {
            drop(resp);
            return Err(HttpError::TooLarge);
        }
    }
    let bytes = resp.bytes().await.map_err(|_| HttpError::Transport)?;
    if bytes.len() > max_bytes {
        return Err(HttpError::TooLarge);
    }
    serde_json::from_slice::<Value>(&bytes).map_err(|_| HttpError::BadJson)
}

/// application/x-www-form-urlencoded POST with JSON accept header.
pub async fn post_form(
    http: &reqwest::Client,
    url: &str,
    params: &[(&str, &str)],
    max_bytes: usize,
) -> Result<Value, HttpError> {
    let resp = http
        .post(url)
        .header("Accept", "application/json")
        .header("User-Agent", "algo-backend/0.1")
        .form(params)
        .send()
        .await
        .map_err(|_| HttpError::Transport)?;
    bounded_json(resp, max_bytes).await
}

/// GET with optional bearer token.
pub async fn get_json(
    http: &reqwest::Client,
    url: &str,
    bearer: Option<&str>,
    extra_headers: &[(&str, &str)],
    max_bytes: usize,
) -> Result<(u16, Value), HttpError> {
    let mut req = http
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "algo-backend/0.1");
    for (k, v) in extra_headers {
        req = req.header(*k, *v);
    }
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await.map_err(|_| HttpError::Transport)?;
    let status = resp.status().as_u16();
    // Error bodies are small rejects; parse only success bodies fully.
    if !(200..300).contains(&status) {
        return Err(HttpError::BadStatus(status));
    }
    Ok((status, bounded_json(resp, max_bytes).await?))
}
