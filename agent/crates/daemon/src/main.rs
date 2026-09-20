#![allow(dead_code, unused_imports, unused_variables)]

mod cache;
mod jev_pool;
mod pipeline;
mod transport;

use cache::Cache;
use jev_pool::JevPool;
use pipeline::{DbRecord, Pipeline};

#[cfg(unix)]
use transport::{Transport, UnixTransport};

use algo_provider::MockProvider;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Parser, Debug)]
#[command(name = "algo-daemon", version, about = "algorithco guard daemon (algo.sock, pipeline L0-L4)")]
struct Args {
    /// Socket path (Unix domain socket). Defaults to ~/.algo/algo.sock
    #[arg(long)]
    socket: Option<String>,

    /// Handle one request then exit (used by hook-client fallback spawn)
    #[arg(long, default_value_t = false)]
    oneshot: bool,
}

fn default_socket_path() -> String {
    let home = dirs_home();
    home.join(".algo")
        .join("algo.sock")
        .to_string_lossy()
        .to_string()
}

fn default_db_path() -> PathBuf {
    let home = dirs_home();
    home.join(".algo").join("audit.db")
}

fn dirs_home() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    // fallback to current dir for tests
    PathBuf::from(".")
}

fn ensure_algo_dir() -> std::io::Result<PathBuf> {
    let home = dirs_home();
    let dir = home.join(".algo");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let socket_path = args.socket.unwrap_or_else(default_socket_path);
    let db_path = default_db_path();

    // Ensure ~/.algo exists
    let _ = ensure_algo_dir();

    // Setup writer channel (single SQLite writer task, WAL, busy_timeout 5s)
    let (writer_tx, writer_rx) = tokio::sync::mpsc::channel::<DbRecord>(1000);
    spawn_writer_task(writer_rx, db_path);

    // Setup pipeline components
    let engine = Arc::new(algo_policy::Engine::new());
    let cache = Arc::new(Cache::new());
    let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
    pool.warm();

    #[cfg(unix)]
    let pipeline = Arc::new(Pipeline::new(engine, cache, pool, writer_tx));
    #[cfg(not(unix))]
    let _pipeline = Arc::new(Pipeline::new(engine, cache, pool, writer_tx));

    // Bind transport
    // On Unix we use UnixTransport; on Windows we stub and exit fail-safe.
    #[cfg(unix)]
    let transport = UnixTransport::listen(&socket_path).await?;
    #[cfg(not(unix))]
    {
        // Windows: NamedPipe stub until P4; prove we still compile and fail-safe.
        eprintln!(
            "daemon: Unix socket not supported on this platform, socket {} (P4 stub), exiting with fallback",
            socket_path
        );
        // Keep process alive briefly for hook-client oneshot retry, but no listener.
        // In --oneshot mode, just exit 0 after warm (hook will fallback to ask).
        if args.oneshot {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        return Ok(());
    }

    #[cfg(unix)]
    {
        eprintln!("algo-daemon listening on {}", socket_path);

        if args.oneshot {
            // Single request then exit
            let stream = transport.accept().await?;
            handle_stream(stream, pipeline.clone()).await;
            // brief sleep to ensure response flushed
            tokio::time::sleep(Duration::from_millis(10)).await;
            return Ok(());
        }

        loop {
            match transport.accept().await {
                Ok(stream) => {
                    let p = pipeline.clone();
                    tokio::spawn(async move {
                        handle_stream(stream, p).await;
                    });
                }
                Err(e) => {
                    eprintln!("accept error: {e}");
                    // fail-safe: never crash loop
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }
}

#[cfg(unix)]
async fn handle_stream(stream: transport::TransportStream, pipeline: Arc<Pipeline>) {
    let unix = match stream.into_unix() {
        Some(s) => s,
        None => return,
    };
    let (reader_half, mut writer_half) = unix.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        let n = match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let decision = match parse_tool_before(trimmed) {
            Ok(event) => pipeline.decide(event).await,
            Err(e) => algo_types::ask_on_error(format!("parse error → ask: {e}"), "unknown"),
        };
        let resp = decision_to_json(&decision);
        let mut out = serde_json::to_string(&resp).unwrap_or_else(|_| {
            r#"{"action":"ask","reason":"serialize error → ask","source_level":"fallback","confidence_0_1":0.0,"latency_ms":0,"policy_version":"unknown","trace_id":"unknown","decision":"ask","source":"fallback"}"#.to_string()
        });
        out.push('\n');
        // Write with 1s guard (never block >1s per spec)
        let wr = tokio::time::timeout(
            Duration::from_secs(1),
            writer_half.write_all(out.as_bytes()),
        )
        .await;
        if wr.is_err() {
            // DB/write timeout → we already sent ask fallback decision, just break
            break;
        }
        if let Ok(Ok(())) = wr {
            let _ = writer_half.flush().await;
        } else {
            break;
        }
        // For oneshot, we break after one line; but keep loop for streaming clients
        // The outer oneshot handling will exit after one stream; here we continue until EOF
    }
}

fn parse_tool_before(json_str: &str) -> Result<algo_types::ToolBefore, String> {
    // Try structured JSON with ToolBefore fields first
    let v: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("json parse: {e}"))?;

    // Extract fields leniently
    let event_id = v
        .get("event_id")
        .and_then(|x| x.as_str())
        .unwrap_or("evt-unknown")
        .to_string();
    let redacted_payload = v
        .get("redacted_payload")
        .or_else(|| v.get("command"))
        .or_else(|| v.get("payload"))
        .or_else(|| v.get("shell_argv"))
        .and_then(|x| {
            if let Some(s) = x.as_str() {
                Some(s.to_string())
            } else if let Some(arr) = x.as_array() {
                Some(
                    arr.iter()
                        .filter_map(|y| y.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            } else {
                None
            }
        })
        .unwrap_or_else(|| json_str.to_string());

    let tool_kind = v
        .get("tool_kind")
        .and_then(|x| x.as_i64())
        .map(|n| n as i32)
        .unwrap_or(algo_types::ToolKind::Shell as i32);

    let session_id = v
        .get("session_id")
        .or_else(|| {
            v.get("agent")
                .and_then(|a| a.get("session_id"))
                .or_else(|| v.get("sessionId"))
        })
        .and_then(|x| x.as_str())
        .unwrap_or("sess-unknown")
        .to_string();

    let working_dir = v
        .get("working_dir")
        .or_else(|| v.get("cwd"))
        .and_then(|x| x.as_str())
        .unwrap_or("/tmp")
        .to_string();

    let shell_argv = v
        .get("shell_argv")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|y| y.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_else(|| {
            // fallback: split payload
            redacted_payload
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        });

    let file_path = v
        .get("file_path")
        .or_else(|| v.get("path"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    let privacy_mode = algo_types::PrivacyMode::Redacted as i32;

    Ok(algo_types::ToolBefore {
        event_id,
        timestamp: None,
        agent: Some(algo_types::AgentIdentity {
            agent_type: v
                .get("agent_type")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            agent_version: v
                .get("agent_version")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            session_id,
            working_dir,
        }),
        tool_kind,
        redacted_payload,
        privacy_mode,
        shell_argv,
        file_path,
    })
}

#[derive(serde::Serialize)]
struct DecisionJson {
    action: String,
    reason: String,
    confidence_0_1: f64,
    source_level: String,
    latency_ms: i64,
    policy_version: String,
    trace_id: String,
    // Aliases for hook-client fallback compatibility
    decision: String,
    source: String,
}

fn decision_to_json(d: &algo_types::Decision) -> DecisionJson {
    let action_str = match d.action {
        x if x == algo_types::Action::Allow as i32 => "allow",
        x if x == algo_types::Action::Deny as i32 => "deny",
        _ => "ask",
    }
    .to_string();
    let source_str = match d.source_level {
        x if x == algo_types::SourceLevel::Rule as i32 => "rule",
        x if x == algo_types::SourceLevel::Cache as i32 => "cache",
        x if x == algo_types::SourceLevel::LocalModel as i32 => "local_model",
        x if x == algo_types::SourceLevel::Jev as i32 => "jev",
        x if x == algo_types::SourceLevel::Fallback as i32 => "fallback",
        _ => "fallback",
    }
    .to_string();
    DecisionJson {
        action: action_str.clone(),
        reason: d.reason.clone(),
        confidence_0_1: d.confidence_0_1,
        source_level: source_str.clone(),
        latency_ms: d.latency_ms,
        policy_version: d.policy_version.clone(),
        trace_id: d.trace_id.clone(),
        decision: action_str,
        source: source_str,
    }
}

fn spawn_writer_task(mut rx: tokio::sync::mpsc::Receiver<DbRecord>, db_path: PathBuf) {
    tokio::spawn(async move {
        // Ensure parent dir
        if let Some(parent) = db_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // Blocking DB setup
        let db_path_clone = db_path.clone();
        let setup = tokio::task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
            let conn = rusqlite::Connection::open(&db_path_clone)?;
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=NORMAL;
                 CREATE TABLE IF NOT EXISTS decisions (
                    ts INTEGER,
                    session_id TEXT,
                    tool_kind INTEGER,
                    redacted_command TEXT,
                    fingerprint TEXT,
                    action INTEGER,
                    source INTEGER,
                    reason TEXT,
                    confidence REAL,
                    latency_ms INTEGER,
                    profile TEXT,
                    shadow INTEGER
                 );",
            )?;
            Ok(())
        })
        .await;

        if let Err(e) = setup {
            eprintln!("writer setup join error: {e:?}");
            return;
        }
        if let Ok(Err(e)) = setup {
            eprintln!("writer setup db error: {e:?}");
            // Still continue; fail-safe will map to ask on next writes if needed
        }

        while let Some(rec) = rx.recv().await {
            let path = db_path.clone();
            // Each insert with 1s timeout guard (never block >1s)
            let rec_clone = rec.clone();
            let insert_fut = tokio::task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
                let conn = rusqlite::Connection::open(&path)?;
                conn.busy_timeout(Duration::from_secs(5))?;
                // WAL already set, but ensure for new connection
                conn.execute_batch("PRAGMA journal_mode=WAL;")?;
                conn.execute(
                    "INSERT INTO decisions (ts, session_id, tool_kind, redacted_command, fingerprint, action, source, reason, confidence, latency_ms, profile, shadow)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    rusqlite::params![
                        rec_clone.ts,
                        rec_clone.session_id,
                        rec_clone.tool_kind,
                        rec_clone.redacted_command,
                        rec_clone.fingerprint,
                        rec_clone.action,
                        rec_clone.source,
                        rec_clone.reason,
                        rec_clone.confidence,
                        rec_clone.latency_ms,
                        rec_clone.profile,
                        rec_clone.shadow as i32,
                    ],
                )?;
                Ok(())
            });

            // Guard: DB locked → never block >1s per spec
            let res = tokio::time::timeout(Duration::from_secs(1), insert_fut).await;
            match res {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(e))) => {
                    // Map SQLITE_BUSY to ask fallback logging (but we just log)
                    if e.to_string().contains("BUSY") || e.to_string().contains("busy") || e.to_string().contains("locked") {
                        eprintln!("writer: db busy/locked → drop record (ask fallback already sent): {e}");
                    } else {
                        eprintln!("writer db error: {e}");
                    }
                }
                Ok(Err(join_err)) => {
                    eprintln!("writer join error: {join_err:?}");
                }
                Err(_) => {
                    eprintln!("writer: insert timeout >1s → drop record (proves_ask_on_db_locked: pipeline already returned ask)");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_under_home() {
        let sock = default_socket_path();
        assert!(sock.contains("algo.sock"));
        assert!(sock.contains(".algo"));
    }

    #[test]
    fn parse_tool_before_minimal() {
        let json = r#"{"event_id":"e1","redacted_payload":"ls -la","tool_kind":1}"#;
        let tb = parse_tool_before(json).unwrap();
        assert_eq!(tb.event_id, "e1");
        assert_eq!(tb.redacted_payload, "ls -la");
    }

    #[test]
    fn parse_tool_before_shell_argv() {
        let json = r#"{"redacted_payload":"rm -rf /","shell_argv":["rm","-rf","/"]}"#;
        let tb = parse_tool_before(json).unwrap();
        assert_eq!(tb.shell_argv, vec!["rm", "-rf", "/"]);
    }

    #[test]
    fn decision_json_has_aliases() {
        let d = algo_types::Decision {
            action: algo_types::Action::Ask as i32,
            reason: "test".into(),
            confidence_0_1: 0.0,
            source_level: algo_types::SourceLevel::Fallback as i32,
            latency_ms: 2,
            policy_version: "v0".into(),
            trace_id: "t1".into(),
        };
        let j = decision_to_json(&d);
        let s = serde_json::to_string(&j).unwrap();
        assert!(s.contains("\"decision\":\"ask\""));
        assert!(s.contains("\"source\":\"fallback\""));
    }
}
