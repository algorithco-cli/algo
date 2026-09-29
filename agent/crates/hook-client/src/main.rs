use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(not(unix))]
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser, Debug)]
#[command(
    name = "algo-hook-client",
    version,
    about = "Tiny hook client (fail-safe, never non-zero)"
)]
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
    // C6: Claude PreToolUse shape per current docs
    // ([VERIFY 2026-09-28](https://code.claude.com/docs/en/hooks)): exit 0 with
    // hookSpecificOutput.permissionDecision=ask (escalate to user). Legacy
    // decision/action/source aliases kept for daemon consumers.
    let v = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": "daemon unreachable → ask (fail-safe)"
        },
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

fn paused_file_path() -> PathBuf {
    // Mirror CLI resolve_home: ALGO_HOME > HOME/USERPROFILE > "."
    // CLI writes to <home>/.algo/paused via cmd_pause; hook must check same place first.
    if let Ok(v) = std::env::var("ALGO_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v).join(".algo").join("paused");
        }
    }
    dirs_home().join(".algo").join("paused")
}

fn is_paused() -> bool {
    is_paused_at(&paused_file_path())
}

fn is_paused_at(p: &std::path::Path) -> bool {
    // Fail-safe: any I/O error => NOT paused (proceed to daemon, which fails to ask).
    // Only an existing file triggers bypass-allow. Never allow-on-error.
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

fn paused_allow_json() -> String {
    // P1-08: `algo pause` touches ~/.algo/paused — hook-client checks first,
    // instant bypass even daemon-dead. Intentional allow (not an error path).
    let v = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": "paused → allow (bypass)"
        },
        "decision": "allow",
        "action": "allow",
        "reason": "paused → allow (bypass)",
        "source": "paused",
        "source_level": "paused",
        "confidence_0_1": 1.0,
        "latency_ms": 0,
        "policy_version": env!("CARGO_PKG_VERSION"),
        "trace_id": "paused"
    });
    v.to_string()
}

/// C6: normalize any daemon response (new hookSpecificOutput shape or legacy
/// decision/action aliases) to canonical Claude PreToolUse JSON, and map the
/// exit code per docs: deny → 2 (blocking error), allow/ask → 0.
/// Ask stays exit 0 (non-blocking escalate-to-user); only deny blocks.
fn to_claude_output(raw: &str) -> (String, i32) {
    let v: serde_json::Value = match serde_json::from_str(raw.trim()) {
        Ok(v) => v,
        Err(_) => {
            return (fallback_ask_json(), 0);
        }
    };
    let decision = v
        .get("hookSpecificOutput")
        .and_then(|h| h.get("permissionDecision"))
        .and_then(|d| d.as_str())
        .or_else(|| v.get("decision").and_then(|d| d.as_str()))
        .or_else(|| v.get("action").and_then(|d| d.as_str()))
        .unwrap_or("ask");
    let permission = match decision.trim().to_ascii_lowercase().as_str() {
        "allow" | "approve" => "allow",
        "deny" | "block" => "deny",
        _ => "ask",
    };
    let reason = v
        .get("hookSpecificOutput")
        .and_then(|h| h.get("permissionDecisionReason"))
        .and_then(|r| r.as_str())
        .or_else(|| v.get("reason").and_then(|r| r.as_str()))
        .unwrap_or("ask (fail-safe)");
    let out = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": permission,
            "permissionDecisionReason": reason
        },
        "decision": permission,
        "action": permission,
        "reason": reason
    });
    let code = if permission == "deny" { 2 } else { 0 };
    (out.to_string(), code)
}

#[tokio::main]
async fn main() {
    // Hook client must never exit non-zero per spec
    // P1-08: check paused first, instant bypass even daemon-dead (no socket dial).
    if is_paused() {
        println!("{}", paused_allow_json());
        std::process::exit(0);
    }
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
            let (out, code) = to_claude_output(&resp);
            println!("{}", out.trim());
            std::process::exit(code);
        }
        Err(_) => {
            // Try spawn daemon --oneshot once, wait 200ms, retry
            let _ = try_spawn_daemon_oneshot(&socket_path);
            tokio::time::sleep(Duration::from_millis(200)).await;
            match try_call(&socket_path, &payload_str).await {
                Ok(resp) => {
                    let (out, code) = to_claude_output(&resp);
                    println!("{}", out.trim());
                    std::process::exit(code);
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

fn daemon_bin_candidates() -> Vec<PathBuf> {
    // C10: spawn ONLY absolute paths — the installed
    // ~/.algo/bin/algo-daemon, or an explicit ALGO_DAEMON_BIN override that
    // must itself be absolute (canonicalized below). Never ./target/*, never
    // a bare-PATH lookup (PATH/CWD injection).
    let mut bins: Vec<PathBuf> = Vec::new();
    if let Ok(env_bin) = std::env::var("ALGO_DAEMON_BIN") {
        let trimmed = env_bin.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_absolute() {
                // Canonicalize to resolve symlinks; keep the raw absolute path
                // if the binary does not exist yet (spawn will fail closed).
                match std::fs::canonicalize(&p) {
                    Ok(c) => bins.push(c),
                    Err(_) => bins.push(p),
                }
            } else {
                eprintln!("hook-client: ignoring non-absolute ALGO_DAEMON_BIN (C10)");
            }
        }
    }
    let installed = dirs_home().join(".algo").join("bin").join(bin_name());
    bins.push(installed);
    bins
}

#[cfg(windows)]
fn bin_name() -> &'static str {
    "algo-daemon.exe"
}

#[cfg(not(windows))]
fn bin_name() -> &'static str {
    "algo-daemon"
}

fn try_spawn_daemon_oneshot(socket_path: &str) -> Result<(), String> {
    for bin in daemon_bin_candidates() {
        if !bin.is_absolute() {
            continue;
        }
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

    #[test]
    fn paused_allow_is_allow() {
        let s = paused_allow_json();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["decision"], "allow");
        assert_eq!(v["action"], "allow");
        assert_eq!(v["source"], "paused");
        assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "allow");
        assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    }

    #[test]
    fn proves_ask_on_paused_check_error() {
        // Fail-safe: missing file or dir path must NOT count as paused.
        // Caller then proceeds to daemon which fails to ask (never allow-on-error).
        assert!(!is_paused_at(std::path::Path::new(
            "/tmp/definitely-missing-algo-paused-12345"
        )));
        // A directory is not a paused file.
        assert!(!is_paused_at(std::path::Path::new("/tmp")));
    }

    #[test]
    fn paused_file_detects_created_file() {
        // P1-08 AC: `algo pause` touches <home>/.algo/paused — hook must bypass.
        let dir = std::env::temp_dir().join(format!("algo-paused-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("paused");
        assert!(!is_paused_at(&p));
        std::fs::write(&p, b"paused").unwrap();
        assert!(is_paused_at(&p));
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&dir);
    }
}
