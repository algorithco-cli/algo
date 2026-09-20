#![allow(dead_code, unused_imports, unused_variables)]

use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(not(unix))]
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser, Debug)]
#[command(name = "algo-hook-client", version, about = "Tiny hook client (fail-safe, never non-zero)")]
struct Args {
    /// Read JSON from stdin (if true, reads stdin; otherwise also reads stdin as fallback)
    #[arg(long, default_value_t = false)]
    stdin: bool,

    /// Socket path (uds|pipe). Defaults to ~/.algo/algo.sock
    #[arg(long)]
    socket: Option<String>,

    /// Overall timeout in ms (default 1200). Connect 100ms, request 1000ms per spec.
    #[arg(long, default_value_t = 1200)]
    timeout: u64,
}

fn default_socket_path() -> String {
    let home = dirs_home();
    home.join(".algo")
        .join("algo.sock")
        .to_string_lossy()
        .to_string()
}

fn dirs_home() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

fn fallback_ask_json() -> String {
    // Must match spec: {"decision":"ask","reason":"daemon unreachable → ask (fail-safe)","source":"fallback"}
    // Also include full Decision shape for daemon consumers
    let v = serde_json::json!({
        "decision": "ask",
        "action": "ask",
        "reason": "daemon unreachable → ask (fail-safe)",
        "source": "fallback",
        "source_level": "fallback",
        "confidence_0_1": 0.0,
        "latency_ms": 0,
        "policy_version": env!("CARGO_PKG_VERSION"),
        "trace_id": "fallback"
    });
    v.to_string()
}

#[tokio::main]
async fn main() {
    // Hook client must never exit non-zero per spec
    let args = Args::parse();
    let socket_path = args.socket.unwrap_or_else(default_socket_path);

    // Read stdin JSON
    let payload = read_stdin().await;

    // payload may be empty if no stdin; treat as empty JSON object to still trigger daemon call
    let payload_str = if payload.trim().is_empty() {
        // If --stdin was requested but empty, still send minimal event
        r#"{"event_id":"evt-hook","redacted_payload":"","tool_kind":1}"#.to_string()
    } else {
        payload.trim().to_string()
    };

    // First attempt
    match try_call(&socket_path, &payload_str).await {
        Ok(resp) => {
            println!("{}", resp.trim());
            std::process::exit(0);
        }
        Err(_) => {
            // Try spawn daemon --oneshot once, wait 200ms, retry
            let _ = try_spawn_daemon_oneshot(&socket_path);
            tokio::time::sleep(Duration::from_millis(200)).await;
            match try_call(&socket_path, &payload_str).await {
                Ok(resp) => {
                    println!("{}", resp.trim());
                    std::process::exit(0);
                }
                Err(_) => {
                    // Final fallback: print ask JSON + exit 0 (never ambiguous non-zero)
                    println!("{}", fallback_ask_json());
                    std::process::exit(0);
                }
            }
        }
    }
}

async fn read_stdin() -> String {
    // Use tokio async stdin
    let mut buf = String::new();
    let mut stdin = tokio::io::stdin();
    // Try to read with a short timeout to avoid hanging when no input
    let res = tokio::time::timeout(Duration::from_secs(2), async {
        let mut reader = BufReader::new(&mut stdin);
        // Read until EOF
        reader.read_line(&mut buf).await?;
        // If we got a line, try to read remaining (in case JSON spans multiple lines)
        // For simplicity, read one line; if JSON is multiline, it will still be partially read,
        // but hook payloads are single-line NDJSON.
        // To handle full stdin, read rest:
        let mut rest = String::new();
        // Non-blocking read of remainder with timeout
        let _ = tokio::time::timeout(Duration::from_millis(100), reader.read_line(&mut rest)).await;
        buf.push_str(&rest);
        Ok::<(), std::io::Error>(())
    })
    .await;

    // If timeout or error, fallback to blocking read via std
    if res.is_err() {
        // Try blocking read for tests that pipe via std
        // We already have buf maybe partially
        if buf.trim().is_empty() {
            // Sync fallback: read from std::io::stdin synchronously with timeout?
            // Just return what we have
        }
    }
    buf
}

async fn try_call(socket_path: &str, payload: &str) -> Result<String, String> {
    // connect_timeout 100ms, request_timeout 1000ms per spec
    #[cfg(unix)]
    {
        let stream = tokio::time::timeout(
            Duration::from_millis(100),
            tokio::net::UnixStream::connect(socket_path),
        )
        .await
        .map_err(|_| "connect timeout 100ms".to_string())?
        .map_err(|e| format!("connect error: {e}"))?;

        let (reader_half, mut writer_half) = stream.into_split();
        // Send JSON line
        let mut msg = payload.to_string();
        if !msg.ends_with('\n') {
            msg.push('\n');
        }

        let write_fut = async {
            writer_half.write_all(msg.as_bytes()).await?;
            writer_half.flush().await?;
            // Shutdown write half to signal EOF for oneshot? Keep open for read.
            Ok::<(), std::io::Error>(())
        };

        tokio::time::timeout(Duration::from_millis(1000), write_fut)
            .await
            .map_err(|_| "write timeout 1000ms".to_string())?
            .map_err(|e| format!("write error: {e}"))?;

        let mut reader = BufReader::new(reader_half);
        let mut line = String::new();
        tokio::time::timeout(Duration::from_millis(1000), reader.read_line(&mut line))
            .await
            .map_err(|_| "read timeout 1000ms".to_string())?
            .map_err(|e| format!("read error: {e}"))?;

        if line.trim().is_empty() {
            return Err("empty response".to_string());
        }
        // Validate it is JSON
        let _: serde_json::Value =
            serde_json::from_str(line.trim()).map_err(|e| format!("invalid json response: {e}"))?;
        Ok(line)
    }
    #[cfg(not(unix))]
    {
        let _ = socket_path;
        let _ = payload;
        Err("Unix socket not supported on this platform (P4)".to_string())
    }
}

fn try_spawn_daemon_oneshot(socket_path: &str) -> Result<(), String> {
    // Try to spawn daemon --oneshot. Binary may be algo-daemon in PATH or sibling.
    let candidates = [
        "algo-daemon".to_string(),
        format!("{}/algo-daemon", dirs_home().join(".algo").join("bin").to_string_lossy()),
        // For dev: cargo run's binary nearby
        "./target/debug/algo-daemon".to_string(),
        "./target/release/algo-daemon".to_string(),
    ];

    // Allow override via env
    let mut bins: Vec<String> = Vec::new();
    if let Ok(env_bin) = std::env::var("ALGO_DAEMON_BIN") {
        bins.push(env_bin);
    }
    bins.extend(candidates);

    for bin in bins {
        let res = Command::new(&bin)
            .arg("--oneshot")
            .arg("--socket")
            .arg(socket_path)
            .spawn();
        if res.is_ok() {
            return Ok(());
        }
    }
    Err("failed to spawn daemon".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_ask() {
        let s = fallback_ask_json();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["decision"], "ask");
        assert_eq!(v["source"], "fallback");
        assert_eq!(v["action"], "ask");
    }

    #[test]
    fn default_socket_contains_algo_sock() {
        let p = default_socket_path();
        assert!(p.contains("algo.sock"));
    }

    #[tokio::test]
    async fn try_call_fails_when_daemon_down() {
        let r = try_call("/tmp/nonexistent-algo-test.sock", r#"{"event_id":"e1"}"#).await;
        assert!(r.is_err(), "should fail when daemon down");
    }

    // Kill-daemon test: hook-client exits 0 with ask/source:fallback is integration-level.
    // We test that fallback path returns valid JSON and would be exit 0.
    #[tokio::test]
    async fn kill_daemon_fallback_is_valid_json() {
        // Simulate kill-daemon: use nonexistent socket, ensure fallback is used
        let payload = r#"{"event_id":"e1","redacted_payload":"ls -la"}"#;
        let res = try_call("/tmp/kill-daemon-nonexistent.sock", payload).await;
        assert!(res.is_err());
        let fallback = fallback_ask_json();
        let v: serde_json::Value = serde_json::from_str(&fallback).unwrap();
        assert_eq!(v["decision"], "ask");
        assert_eq!(v["source"], "fallback");
    }
}
