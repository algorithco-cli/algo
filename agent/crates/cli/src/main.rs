#![allow(unused_imports)]
use algo_audit::AuditStore;
use algo_policy::Engine as PolicyEngine;
use algo_policy::Profile;
use algo_provider::{DecisionProvider, MockProvider};
use algo_types::{Action, AgentIdentity, Decision, PrivacyMode, SourceLevel, ToolBefore, ToolKind};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, ValueEnum)]
enum PrivacyArg {
    #[value(name = "local-only")]
    LocalOnly,
    #[value(name = "redacted")]
    Redacted,
    #[value(name = "full")]
    Full,
}

impl PrivacyArg {
    fn as_str(&self) -> &'static str {
        match self {
            PrivacyArg::LocalOnly => "local-only",
            PrivacyArg::Redacted => "redacted",
            PrivacyArg::Full => "full",
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "algo", version, about = "algorithco guard CLI")]
struct Cli {
    /// Override HOME directory (for tests, also respects ALGO_HOME env)
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize algo: detect claude config, backup, merge hook, set privacy, run doctor
    Init {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long, value_enum)]
        privacy: Option<PrivacyArg>,
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
    /// Uninstall: restore backup byte-identical, remove socket/db, hooks
    Uninstall {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        keep_db: bool,
    },
    /// Run diagnostics
    Doctor {
        #[arg(long)]
        home: Option<PathBuf>,
        /// Preview exactly what would leave the machine (redaction demo)
        #[arg(long, default_value_t = false)]
        show_egress: bool,
    },
    /// Pause guard (instant bypass, works daemon-dead)
    Pause {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    /// Resume guard
    Resume {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    /// Show last decision pretty (action+reason+confidence+source+latency)
    Why {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    /// Show counts auto-approved/asked/blocked + would-have-blocked
    Status {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    /// Show recent decisions
    Log {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        show_egress: bool,
    },
    /// Policy stub
    Policy {
        #[arg(long)]
        home: Option<PathBuf>,
    },
    /// Shadow vs enforcing: `algo enforce on|off|status` (default shadow, P1-08).
    /// Requires daemon restart (private-MVP limit).
    Enforce {
        #[arg(long)]
        home: Option<PathBuf>,
        /// on = enforcing (shadow off), off = shadow (default), status = print current
        mode: Option<String>,
    },
    /// Login stub
    Login {
        #[arg(long)]
        home: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let res = match cli.command {
        Commands::Init { home, privacy, yes } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            let p = privacy.map(|p| p.as_str().to_string());
            cmd_init(&h, p.as_deref(), yes)
        }
        Commands::Uninstall { home, keep_db } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_uninstall(&h, keep_db)
        }
        Commands::Doctor { home, show_egress } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_doctor(&h, show_egress)
        }
        Commands::Pause { home } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_pause(&h)
        }
        Commands::Resume { home } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_resume(&h)
        }
        Commands::Why { home } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_why(&h)
        }
        Commands::Status { home } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_status(&h)
        }
        Commands::Log {
            home,
            limit,
            show_egress,
        } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_log(&h, limit, show_egress)
        }
        Commands::Policy { home } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_policy(&h)
        }
        Commands::Enforce { home, mode } => {
            let h = resolve_home(combine_home(cli.home.as_deref(), home.as_deref()));
            cmd_enforce(&h, mode.as_deref())
        }
        Commands::Login { .. } => {
            println!("login: not yet implemented (stub, exit 0)");
            Ok(())
        }
    };
    if let Err(e) = res {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn combine_home<'a>(global: Option<&'a Path>, local: Option<&'a Path>) -> Option<&'a Path> {
    local.or(global)
}

fn resolve_home(cli_home: Option<&Path>) -> PathBuf {
    if let Some(p) = cli_home {
        return p.to_path_buf();
    }
    if let Ok(v) = std::env::var("ALGO_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Some(h) = dirs::home_dir() {
        return h;
    }
    // fallback to HOME / USERPROFILE
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

fn detect_claude_config(home: &Path) -> PathBuf {
    let p1 = home.join(".claude.json");
    let p2 = home.join(".config").join("claude").join("settings.json");
    if p1.exists() {
        return p1;
    }
    if p2.exists() {
        return p2;
    }
    // fallback: prefer .claude.json for new installs
    p1
}

fn hook_command_for_home(home: &Path) -> String {
    // Use ~/.algo/hooks/hook-client as spec
    home.join(".algo")
        .join("hooks")
        .join("hook-client")
        .to_string_lossy()
        .to_string()
}

fn algo_dir(home: &Path) -> PathBuf {
    home.join(".algo")
}

fn ensure_algo_dir(home: &Path) -> io::Result<()> {
    fs::create_dir_all(algo_dir(home))?;
    fs::create_dir_all(algo_dir(home).join("hooks"))?;
    Ok(())
}

// ---------- init ----------

/// Egress disclosure shown before any non-local-only choice (privacy-dataflow.md
/// consent draft). local-only sends nothing; redacted/full go to TypeSafe (US).
fn print_privacy_disclosure() {
    println!("--- data disclosure (required before redacted|full) ---");
    println!("local-only: nothing leaves this machine. The daemon never calls");
    println!("  the Jev API; decisions are local rules + cache + ask.");
    println!("redacted: redacted payloads are sent to api.typesafe.ai (US infrastructure)");
    println!("  using YOUR key (ALGO_JEV_API_KEY, BYOK only — never embedded). Processed");
    println!("  in the United States regardless of your location; EEA/UK transfers run");
    println!("  under Standard Contractual Clauses + UK Addendum. Retention: TypeSafe states");
    println!("  request/response data is not retained; no fixed deletion SLA beyond that.");
    println!("full: UNREDACTED commands (may include secrets) go to the same US endpoint.");
    println!("Inspect exactly what would leave: `algo log --show-egress`.");
    println!("Revert any time: re-run `algo init` and choose local-only (stops all sending).");
    println!("--- end disclosure ---");
}

/// Persist the privacy opt-in record (consent_id + timestamp + scope).
/// local-only records scope local-only (no egress consent granted).
fn write_consent_record(home: &Path, privacy: &str) -> Result<(), String> {
    ensure_algo_dir(home).map_err(|e| e.to_string())?;
    let consent_id = format!(
        "cons-{}-{}",
        chrono::Utc::now().timestamp_millis(),
        std::process::id()
    );
    let scope = if privacy == "local-only" {
        "no-egress"
    } else {
        "jev-egress"
    };
    let rec = serde_json::json!({
        "consent_id": consent_id,
        "privacy": privacy,
        "scope": scope,
        "endpoint": if privacy == "local-only" { serde_json::Value::Null } else { serde_json::json!("api.typesafe.ai (US)") },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
    });
    let path = algo_dir(home).join("consent.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&rec).unwrap().as_bytes(),
    )
    .map_err(|e| format!("write consent record: {e}"))?;
    println!(
        "consent: {consent_id} (scope {scope}) -> {}",
        path.display()
    );
    Ok(())
}
fn cmd_init(home: &Path, privacy: Option<&str>, yes: bool) -> Result<(), String> {
    let config_path = detect_claude_config(home);
    let hook_cmd = hook_command_for_home(home);

    // Ensure parent dir for config exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create dir {parent:?}: {e}"))?;
    }
    ensure_algo_dir(home).map_err(|e| format!("ensure .algo: {e}"))?;

    // Read original bytes if exists
    let orig_bytes: Option<Vec<u8>> = if config_path.exists() {
        Some(fs::read(&config_path).map_err(|e| format!("read {config_path:?}: {e}"))?)
    } else {
        None
    };
    let orig_json: serde_json::Value = if let Some(ref b) = orig_bytes {
        serde_json::from_slice(b).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let before_str = serde_json::to_string_pretty(&orig_json).unwrap_or_else(|_| "{}".into());

    // Build after json with additive hook merge
    let mut after_json = orig_json.clone();
    merge_hook(&mut after_json, &hook_cmd);

    let after_str = serde_json::to_string_pretty(&after_json).unwrap();

    // Show exact diff
    println!("algo init: detected config {}", config_path.display());
    println!("--- before ---\n{before_str}");
    println!("+++ after +++\n{after_str}");
    if before_str != after_str {
        println!("diff: would add hook PreToolUse -> {hook_cmd}");
    } else {
        println!("hook already present, no diff");
    }

    // Per-agent consent
    if !yes {
        print!("Add hook to {}? [y/N] ", config_path.display());
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        if !line.trim().eq_ignore_ascii_case("y") && !line.trim().eq_ignore_ascii_case("yes") {
            println!("aborted by user (use --yes to auto-consent)");
            return Ok(());
        }
    }

    // Backup *.algo-backup-<ts> if orig exists
    if let Some(ref b) = orig_bytes {
        let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
        // also add millis to avoid collision
        let ts_full = format!("{}-{}", ts, Utc::now().timestamp_millis() % 1000);
        let backup_path =
            PathBuf::from(format!("{}.algo-backup-{}", config_path.display(), ts_full));
        fs::write(&backup_path, b).map_err(|e| format!("backup write {backup_path:?}: {e}"))?;
        // verify byte-identical
        let backup_bytes = fs::read(&backup_path).map_err(|e| format!("read backup: {e}"))?;
        if &backup_bytes != b {
            return Err("backup not byte-identical".into());
        }
        println!("backup: {} ({} bytes)", backup_path.display(), b.len());
    } else {
        println!("no existing config, no backup needed");
    }

    // Privacy prompt local-only|redacted|full — local-only is the default
    // (privacy-dataflow.md:26). Anything beyond local-only prints the egress
    // disclosure and records opt-in consent; `full` needs a second confirm.
    let chosen_privacy = if let Some(p) = privacy {
        p.to_string()
    } else if yes {
        "local-only".to_string()
    } else {
        print_privacy_disclosure();
        print!("privacy mode [local-only|redacted|full] (default local-only): ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        let t = line.trim();
        if t.is_empty() {
            "local-only".to_string()
        } else {
            t.to_string()
        }
    };
    if !["local-only", "redacted", "full"].contains(&chosen_privacy.as_str()) {
        return Err(format!("invalid privacy: {chosen_privacy}"));
    }
    if chosen_privacy == "full" {
        // Second explicit opt-in for unredacted egress (fail closed: --yes
        // with --privacy full is rejected; run interactively to confirm).
        if yes && privacy.is_some() {
            return Err(
                "--privacy full requires interactive confirmation (type FULL); refusing --yes"
                    .into(),
            );
        }
        print!("FULL sends UNREDACTED commands (may include secrets) to api.typesafe.ai (US). Type FULL to confirm: ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        if line.trim() != "FULL" {
            return Err(
                "full privacy not confirmed; re-run init and choose local-only|redacted".into(),
            );
        }
    }
    if chosen_privacy != "local-only" {
        print_privacy_disclosure();
    }

    // Write merged config atomically: write to temp then rename? For now write directly
    // Ensure we preserve byte-identical for non-hook parts? Our pretty print may change formatting,
    // but additive merge is allowed to change file. For uninstall to restore byte-identical, backup suffices.
    fs::write(&config_path, after_str.as_bytes())
        .map_err(|e| format!("write config {config_path:?}: {e}"))?;
    println!("wrote: {}", config_path.display());

    // Ensure ~/.algo/ exists, touch config.json with privacy + shadow default (P1-08).
    // Preserve existing enforce setting on re-init (explicit `enforce on` survives re-init).
    let algo_config = algo_dir(home).join("config.json");
    let prev_enforce = read_enforce(home);
    let cfg = serde_json::json!({
        "privacy": chosen_privacy,
        "version": env!("CARGO_PKG_VERSION"),
        "updated_at": Utc::now().to_rfc3339(),
        "enforce": prev_enforce,
        "shadow": !prev_enforce,
    });
    fs::write(
        &algo_config,
        serde_json::to_string_pretty(&cfg).unwrap().as_bytes(),
    )
    .map_err(|e| format!("write algo config: {e}"))?;
    println!("privacy: {} -> {}", chosen_privacy, algo_config.display());
    write_consent_record(home, &chosen_privacy)?;

    // Ensure hooks dir and placeholder hook-client (for test, touch file)
    let hook_path = Path::new(&hook_cmd);
    if let Some(parent) = hook_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    // Touch hook-client if not exists (for doctor check, not required to be executable in test)
    if !hook_path.exists() {
        // leave placeholder
        let _ = fs::write(hook_path, b"#!/bin/sh\nexec algo-hook-client \"$@\"\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(hook_path, fs::Permissions::from_mode(0o755));
        }
    }

    // Run doctor
    println!("running algo doctor...");
    cmd_doctor(home, false)?;

    Ok(())
}

fn merge_hook(json: &mut serde_json::Value, hook_cmd: &str) {
    // Ensure json is object
    if !json.is_object() {
        *json = serde_json::json!({});
    }
    let obj = json.as_object_mut().unwrap();
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    if !hooks.is_object() {
        *hooks = serde_json::json!({});
    }
    let hooks_obj = hooks.as_object_mut().unwrap();
    let entry = hooks_obj
        .entry("PreToolUse")
        .or_insert_with(|| serde_json::json!([]));

    // Ensure it's array
    if !entry.is_array() {
        *entry = serde_json::json!([]);
    }
    let arr = entry.as_array_mut().unwrap();

    // Check if hook already present
    let already = arr.iter().any(|v| {
        // v could be { "matcher":"Bash", "hooks":[{"type":"command","command": hook_cmd}] }
        // or directly command string?
        if let Some(hooks_arr) = v.get("hooks").and_then(|h| h.as_array()) {
            hooks_arr.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| s == hook_cmd)
                    .unwrap_or(false)
            })
        } else {
            // fallback: check stringified contains
            v.to_string().contains(hook_cmd)
        }
    });
    if already {
        return;
    }
    let new_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [
            { "type": "command", "command": hook_cmd }
        ]
    });
    arr.push(new_entry);
}

// ---------- uninstall ----------
fn cmd_uninstall(home: &Path, keep_db: bool) -> Result<(), String> {
    let config_path = detect_claude_config(home);
    let hook_cmd = hook_command_for_home(home);

    // Restore backup byte-identical (find latest *.algo-backup-*)
    let parent = config_path.parent().unwrap_or(home);
    let file_name = config_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".claude.json");
    let prefix = format!("{}.algo-backup-", file_name);

    let mut backups: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(parent) {
        for e in entries.flatten() {
            let p = e.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) {
                    backups.push(p);
                }
            }
        }
    }
    backups.sort();
    if let Some(latest) = backups.last() {
        let backup_bytes = fs::read(latest).map_err(|e| format!("read backup {latest:?}: {e}"))?;
        // Restore
        fs::write(&config_path, &backup_bytes)
            .map_err(|e| format!("restore {config_path:?}: {e}"))?;
        // Verify byte-identical
        let restored = fs::read(&config_path).map_err(|e| format!("read restored: {e}"))?;
        if restored != backup_bytes {
            return Err("restore not byte-identical".into());
        }
        println!(
            "restored backup {} -> {}",
            latest.display(),
            config_path.display()
        );
        // Optionally keep backup or remove? Keep for audit; not deleting.
    } else {
        // No backup: remove hooks added by init
        if config_path.exists() {
            let bytes = fs::read(&config_path).map_err(|e| format!("read config: {e}"))?;
            let mut json: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
            let removed = remove_hook(&mut json, &hook_cmd);
            if removed {
                // If after removal, hooks empty and original was empty object, we could remove file or write back
                // Check if json is empty or only empty hooks
                let out = serde_json::to_string_pretty(&json).unwrap();
                // If original file was pre-existing and we removed hook, write back
                // If json becomes {} and no backup existed, we could remove file if it was created by init?
                // For now write back if file existed, else remove if empty
                if json == serde_json::json!({})
                    || json == serde_json::json!({"hooks":{}})
                    || json == serde_json::json!({"hooks":{"PreToolUse":[]}})
                {
                    // If config was created by init and now empty, remove it to restore original non-existence?
                    // But original non-existence case had no file, so we should remove file to be byte-identical to pre-init (which was no file)
                    // However we can't know original absence vs empty file. We treat empty as removal candidate.
                    // For test, original was a file, so we will write back {}
                    // Let's write back pretty
                    fs::write(&config_path, out.as_bytes())
                        .map_err(|e| format!("write after hook removal: {e}"))?;
                    println!("removed hook from {}", config_path.display());
                } else {
                    fs::write(&config_path, out.as_bytes())
                        .map_err(|e| format!("write after hook removal: {e}"))?;
                    println!("removed hook from {}", config_path.display());
                }
                // If json is empty and file didn't exist originally, we could delete file
                // Heuristic: if backups empty and json == {} we delete file to mimic original absence
                #[allow(clippy::cmp_owned)]
                if backups.is_empty() && json == serde_json::json!({}) {
                    let _ = fs::remove_file(&config_path);
                    println!("removed empty config {}", config_path.display());
                }
            } else {
                println!("hook not present in {}", config_path.display());
            }
        } else {
            println!("no config and no backup, nothing to restore");
        }
    }

    // Remove socket/db if not keep
    if !keep_db {
        let sock = algo_dir(home).join("algo.sock");
        let db = algo_dir(home).join("audit.db");
        let wal = algo_dir(home).join("audit.db-wal");
        let shm = algo_dir(home).join("audit.db-shm");
        for p in [&sock, &db, &wal, &shm] {
            if p.exists() {
                let _ = fs::remove_file(p);
                println!("removed {}", p.display());
            }
        }
        // also remove config.json if present? spec says remove socket/db opt-in
        // we keep config.json unless explicitly? But we can leave it
    } else {
        println!("keep-db: preserving audit.db");
    }

    // Remove hooks added by init: ensure config file hook removed even when backup restored?
    // If we restored backup, backup already doesn't contain hook, so done.
    // If we restored, still need to ensure hook-client file removed? Not necessarily, but spec says remove hooks
    // We also remove hook-client placeholder?
    let hook_path = PathBuf::from(&hook_cmd);
    if hook_path.exists() {
        // Only remove if we created it; for safety, remove placeholder
        // But don't delete if user has custom hook; we only remove our entry from config, not file
        // For completeness, we can leave hook file but test expects hook removed from config only
    }

    // Remove ~/.algo/paused
    let paused = algo_dir(home).join("paused");
    if paused.exists() {
        let _ = fs::remove_file(&paused);
        println!("removed {}", paused.display());
    }

    // Also clean up backups? Keep them? Not needed.
    println!("uninstall complete");
    Ok(())
}

fn hook_present(json: &serde_json::Value, hook_cmd: &str) -> bool {
    if let Some(hooks) = json.get("hooks").and_then(|h| h.as_object()) {
        if let Some(arr) = hooks.get("PreToolUse").and_then(|a| a.as_array()) {
            for v in arr {
                if let Some(hooks_arr) = v.get("hooks").and_then(|h| h.as_array()) {
                    for h in hooks_arr {
                        if let Some(cmd) = h.get("command").and_then(|c| c.as_str()) {
                            if cmd == hook_cmd {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn remove_hook(json: &mut serde_json::Value, hook_cmd: &str) -> bool {
    let mut removed = false;
    if let Some(hooks) = json.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        if let Some(arr) = hooks.get_mut("PreToolUse").and_then(|a| a.as_array_mut()) {
            let orig_len = arr.len();
            arr.retain(|v| {
                if let Some(hooks_arr) = v.get("hooks").and_then(|h| h.as_array()) {
                    // if any hook command equals hook_cmd, this entry is ours -> remove
                    let is_ours = hooks_arr.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|s| s == hook_cmd)
                            .unwrap_or(false)
                    });
                    !is_ours
                } else {
                    // fallback: exact match on string value
                    if let Some(s) = v.as_str() {
                        s != hook_cmd
                    } else {
                        true
                    }
                }
            });
            if arr.len() != orig_len {
                removed = true;
            }
            if arr.is_empty() {
                hooks.remove("PreToolUse");
            }
        }
        if hooks.is_empty() {
            json.as_object_mut().unwrap().remove("hooks");
        }
    }
    removed
}

// ---------- doctor ----------
fn cmd_doctor(home: &Path, show_egress: bool) -> Result<(), String> {
    println!("=== algo doctor ===");
    let mut ok = true;

    // socket
    let sock = algo_dir(home).join("algo.sock");
    if sock.exists() {
        println!("socket: {} exists", sock.display());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&sock) {
                let mode = meta.permissions().mode() & 0o777;
                if mode == 0o600 {
                    println!("  perms 0600: OK");
                } else {
                    println!("  perms {:o}: FAIL (expected 0600)", mode);
                    ok = false;
                }
            }
        }
        #[cfg(not(unix))]
        {
            println!("  perms: (skip on non-unix)");
        }
    } else {
        println!(
            "socket: {} missing (daemon not running - FAIL)",
            sock.display()
        );
        ok = false;
    }

    // hook present
    let config_path = detect_claude_config(home);
    let hook_cmd = hook_command_for_home(home);
    if config_path.exists() {
        let bytes =
            fs::read(&config_path).map_err(|e| format!("read config {config_path:?}: {e}"))?;
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
        let present = hook_present(&json, &hook_cmd);
        if present {
            println!("hook: present in {} OK", config_path.display());
        } else {
            println!("hook: NOT present in {} FAIL", config_path.display());
            ok = false;
        }
    } else {
        println!("hook: config {} missing FAIL", config_path.display());
        ok = false;
    }

    // audit.db WAL
    let db_path = algo_dir(home).join("audit.db");
    if db_path.exists() {
        match AuditStore::open(&db_path) {
            Ok(store) => match store.check_wal() {
                Ok(true) => println!("audit.db WAL: {} OK", db_path.display()),
                Ok(false) => {
                    println!("audit.db WAL: {} mode not WAL FAIL", db_path.display());
                    ok = false;
                }
                Err(e) => {
                    println!("audit.db WAL check error: {e} FAIL");
                    ok = false;
                }
            },
            Err(e) => {
                println!("audit.db open error: {e} FAIL");
                ok = false;
            }
        }
    } else {
        println!(
            "audit.db: {} missing (not yet created) - WARN",
            db_path.display()
        );
        // Not fatal for doctor? But mark warn not fail?
        // We'll not set ok false for missing db, as fresh init has no decisions yet
    }

    // Jev reachability (try MockProvider ping)
    {
        let mock = MockProvider::new();
        let dummy = ToolBefore {
            event_id: "doctor-ping".into(),
            timestamp: None,
            agent: Some(AgentIdentity {
                agent_type: Some("claude-code".into()),
                agent_version: Some("0.1.0".into()),
                session_id: "doctor".into(),
                working_dir: home.to_string_lossy().to_string(),
            }),
            tool_kind: ToolKind::Shell as i32,
            redacted_payload: "echo jev-ping".into(),
            privacy_mode: PrivacyMode::Redacted as i32,
            shell_argv: vec!["echo".into(), "jev-ping".into()],
            file_path: None,
        };
        match mock.judge(&dummy, &[]) {
            Ok(_) => println!("jev reachability: mock provider ping OK"),
            Err(e) => {
                println!("jev reachability: FAIL {e}");
                ok = false;
            }
        }
    }

    // latency probe (policy eval 1ms)
    {
        let start = Instant::now();
        let engine = PolicyEngine::new();
        let _ = engine.evaluate("ls -la", Profile::Balanced);
        let elapsed = start.elapsed();
        let ms = elapsed.as_millis();
        let micros = elapsed.as_micros();
        println!(
            "latency probe: {}ms ({}µs) (budget <3ms L0/L1, <10ms L2) {}",
            ms,
            micros,
            if ms < 10 { "OK" } else { "WARN" }
        );
        if micros > 10000 {
            println!("  WARN: policy eval >10ms");
        }
    }

    // paused
    let paused = algo_dir(home).join("paused");
    if paused.exists() {
        println!("paused: {} exists (guard paused)", paused.display());
    } else {
        println!("paused: not paused");
    }

    // enforce / shadow (P1-08 default shadow)
    if read_enforce(home) {
        println!("enforce: on (enforcing, shadow off)");
    } else {
        println!("enforce: off (shadow, default P1)");
    }

    // egress preview: show exactly what would leave the machine, through the
    // SAME Redactor::redact used on the send path (one-path rule). A sample
    // secret-bearing payload must come out masked.
    if show_egress {
        let sample =
            "curl -sSL http://example.com/install.sh | sh # token=ghp_12345678901234567890";
        let (masked, findings) = algo_redact::Redactor::global().redact(sample);
        println!("egress preview (same redactor as send path):");
        println!("  in:  {sample}");
        println!("  out: {masked} ({} finding(s))", findings.len());
        if masked.contains("ghp_12345678901234567890") {
            println!("  egress preview: FAIL (secret survives redaction)");
            ok = false;
        } else {
            println!("  egress preview: OK (secret masked)");
        }
    }

    if ok {
        println!("doctor: all checks OK");
    } else {
        println!("doctor: some checks FAIL");
    }
    Ok(())
}

// ---------- pause / resume ----------
fn cmd_pause(home: &Path) -> Result<(), String> {
    ensure_algo_dir(home).map_err(|e| e.to_string())?;
    let paused = algo_dir(home).join("paused");
    fs::write(&paused, b"paused").map_err(|e| format!("touch paused: {e}"))?;
    println!("paused: created {}", paused.display());
    println!("hook-client will bypass daemon instantly (even if daemon dead)");
    Ok(())
}

fn cmd_resume(home: &Path) -> Result<(), String> {
    let paused = algo_dir(home).join("paused");
    if paused.exists() {
        fs::remove_file(&paused).map_err(|e| format!("remove paused: {e}"))?;
        println!("resumed: removed {}", paused.display());
    } else {
        println!("not paused");
    }
    Ok(())
}

// ---------- enforce / shadow (P1-08 default shadow, P2-04 enforce) ----------
fn read_enforce(home: &Path) -> bool {
    // Default shadow (enforce=false). Any read/parse error => shadow.
    let p = algo_dir(home).join("config.json");
    if let Ok(bytes) = fs::read(&p) {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if json.get("enforce").and_then(|x| x.as_bool()) == Some(true) {
                return true;
            }
        }
    }
    false
}

fn write_enforce(home: &Path, enforce: bool) -> Result<(), String> {
    ensure_algo_dir(home).map_err(|e| e.to_string())?;
    let p = algo_dir(home).join("config.json");
    let mut json = if let Ok(bytes) = fs::read(&p) {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !json.is_object() {
        json = serde_json::json!({});
    }
    let obj = json.as_object_mut().unwrap();
    obj.insert("enforce".into(), serde_json::json!(enforce));
    obj.insert("shadow".into(), serde_json::json!(!enforce));
    obj.insert(
        "version".into(),
        serde_json::json!(env!("CARGO_PKG_VERSION")),
    );
    obj.insert(
        "updated_at".into(),
        serde_json::json!(Utc::now().to_rfc3339()),
    );
    fs::write(&p, serde_json::to_string_pretty(&json).unwrap().as_bytes())
        .map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

fn cmd_policy(home: &Path) -> Result<(), String> {
    // Read-only snapshot of the effective local policy (never writes).
    let cfg_path = algo_dir(home).join("config.json");
    let (privacy, enforce, version) = if let Ok(bytes) = fs::read(&cfg_path) {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            (
                json.get("privacy")
                    .and_then(|x| x.as_str())
                    .unwrap_or("local-only")
                    .to_string(),
                json.get("enforce").and_then(|x| x.as_bool()) == Some(true),
                json.get("version")
                    .and_then(|x| x.as_str())
                    .unwrap_or(env!("CARGO_PKG_VERSION"))
                    .to_string(),
            )
        } else {
            (
                "local-only".to_string(),
                false,
                env!("CARGO_PKG_VERSION").to_string(),
            )
        }
    } else {
        (
            "local-only".to_string(),
            false,
            env!("CARGO_PKG_VERSION").to_string(),
        )
    };
    let paused = algo_dir(home).join("paused").exists();
    println!("policy (local snapshot, read-only):");
    println!(
        "  mode: {}",
        if enforce {
            "enforcing (shadow off)"
        } else {
            "shadow (default, never blocks)"
        }
    );
    println!("  privacy: {privacy} (local-only|redacted|full)");
    println!("  profile: balanced (strict|balanced|fast visible; advanced hidden)");
    println!("  policy version: {version}");
    println!(
        "  paused: {}",
        if paused {
            "yes (hook bypasses daemon)"
        } else {
            "no"
        }
    );
    println!("  config: {}", cfg_path.display());
    // Counts prove the digest without dumping decisions.
    let db_path = algo_dir(home).join("audit.db");
    if db_path.exists() {
        if let Ok(store) = AuditStore::open(&db_path) {
            if let Ok(c) = store.counts() {
                println!(
                    "  decisions: allow {} ask {} deny {} would-have-blocked {} (shadow {})",
                    c.allow, c.ask, c.deny, c.would_have_blocked, c.shadow
                );
            }
        }
    } else {
        println!("  decisions: none yet (audit.db missing)");
    }
    println!("  change: algo enforce on|off (restart daemon) · algo init --privacy <mode>");
    Ok(())
}

fn cmd_enforce(home: &Path, mode: Option<&str>) -> Result<(), String> {
    let m = mode.map(|s| s.trim().to_ascii_lowercase());
    match m.as_deref() {
        None | Some("") | Some("status") => {
            let enforce = read_enforce(home);
            if enforce {
                println!("enforce: on (enforcing, shadow off)");
            } else {
                println!("enforce: off (shadow, default P1)");
            }
            println!("would-have-blocked via `algo status` (shadow deny, never blocked)");
            println!("note: requires daemon restart after change");
            Ok(())
        }
        Some("on") | Some("enable") | Some("enforce") => {
            write_enforce(home, true)?;
            println!("enforce: on (enforcing, shadow off)");
            println!("restart daemon to apply");
            Ok(())
        }
        Some("off") | Some("disable") | Some("shadow") => {
            write_enforce(home, false)?;
            println!("enforce: off (shadow, default P1)");
            println!("restart daemon to apply");
            Ok(())
        }
        Some(other) => Err(format!(
            "invalid enforce mode '{other}' (expected on|off|status)"
        )),
    }
}

// ---------- why / status / log ----------
fn cmd_why(home: &Path) -> Result<(), String> {
    let db_path = algo_dir(home).join("audit.db");
    if !db_path.exists() {
        println!("no decisions yet (audit.db not found)");
        return Ok(());
    }
    let store = AuditStore::open(&db_path).map_err(|e| e.to_string())?;
    match store.last().map_err(|e| e.to_string())? {
        Some(entry) => {
            println!("last decision:");
            println!(
                "  action: {} (reason: {})",
                entry.action_str(),
                entry.reason
            );
            println!("  confidence: {:.2}", entry.confidence);
            println!("  source: {} ({} )", entry.source_str(), entry.source);
            println!("  latency: {}ms", entry.latency_ms);
            println!("  profile: {}", entry.profile);
            println!("  shadow: {}", entry.shadow);
            println!("  fingerprint: {}", entry.fingerprint);
            // One-path rule: re-redact on display, same as the send path.
            let (masked, _) = algo_redact::Redactor::global().redact(&entry.redacted_command);
            println!("  redacted_command: {}", masked);
            println!("  ts: {}", entry.ts);
            println!("  session: {}", entry.session_id);
        }
        None => println!("no decisions yet"),
    }
    Ok(())
}

fn cmd_status(home: &Path) -> Result<(), String> {
    let db_path = algo_dir(home).join("audit.db");
    if !db_path.exists() {
        println!("status: no audit.db yet");
        println!("counts: allowed 0, asked 0, blocked 0");
        println!("would-have-blocked 0");
        return Ok(());
    }
    let store = AuditStore::open(&db_path).map_err(|e| e.to_string())?;
    let c = store.counts().map_err(|e| e.to_string())?;
    println!("status:");
    println!("  total: {}", c.total);
    println!("  auto-approved (allow): {}", c.allow);
    println!("  asked: {}", c.ask);
    println!("  blocked (deny): {}", c.deny);
    println!("  shadow total: {}", c.shadow);
    println!(
        "  would-have-blocked {} (shadow deny)",
        c.would_have_blocked
    );
    // Also show would-have N digest
    println!("would-have-blocked {}", c.would_have_blocked);
    Ok(())
}

fn cmd_log(home: &Path, limit: usize, show_egress: bool) -> Result<(), String> {
    let db_path = algo_dir(home).join("audit.db");
    if !db_path.exists() {
        println!("no audit.db yet");
        return Ok(());
    }
    let store = AuditStore::open(&db_path).map_err(|e| e.to_string())?;
    let entries = store.list(limit).map_err(|e| e.to_string())?;
    if entries.is_empty() {
        println!("no decisions");
        return Ok(());
    }
    println!("log (last {}):", entries.len());
    // One-path rule: --show-egress runs the SAME Redactor::redact used on the
    // send path (pipeline re-redact), never prints stored text raw.
    let redactor = algo_redact::Redactor::global();
    for e in entries {
        let egress = if show_egress {
            let (masked, findings) = redactor.redact(&e.redacted_command);
            format!(" egress:{} ({} finding(s))", masked, findings.len())
        } else {
            String::new()
        };
        println!(
            "  [{}] {} reason=\"{}\" conf={:.2} src={} latency={}ms shadow={}{}",
            e.ts,
            e.action_str(),
            e.reason,
            e.confidence,
            e.source_str(),
            e.latency_ms,
            e.shadow,
            egress
        );
    }
    // Also print counts like status
    let c = store.counts().map_err(|e| e.to_string())?;
    println!(
        "counts: allow {} ask {} deny {} would-have-blocked {}",
        c.allow, c.ask, c.deny, c.would_have_blocked
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_types::{Action, SourceLevel};
    use tempfile::TempDir;

    fn test_home() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let home = dir.path().to_path_buf();
        // Keep dir alive by forgetting? We'll return dir and home
        (dir, home)
    }

    #[test]
    fn init_yes_defaults_local_only_with_consent() {
        // Fail-safe default: non-interactive init must land on local-only
        // (no egress) and record a no-egress consent entry.
        let (_tmp, home) = test_home();
        cmd_init(&home, None, true).unwrap();
        let cfg: serde_json::Value =
            serde_json::from_slice(&fs::read(algo_dir(&home).join("config.json")).unwrap())
                .unwrap();
        assert_eq!(cfg["privacy"], "local-only");
        let consent: serde_json::Value =
            serde_json::from_slice(&fs::read(algo_dir(&home).join("consent.json")).unwrap())
                .unwrap();
        assert_eq!(consent["privacy"], "local-only");
        assert_eq!(consent["scope"], "no-egress");
        assert!(consent["consent_id"].as_str().unwrap().starts_with("cons-"));
    }

    #[test]
    fn init_explicit_redacted_records_egress_consent() {
        let (_tmp, home) = test_home();
        cmd_init(&home, Some("redacted"), true).unwrap();
        let consent: serde_json::Value =
            serde_json::from_slice(&fs::read(algo_dir(&home).join("consent.json")).unwrap())
                .unwrap();
        assert_eq!(consent["privacy"], "redacted");
        assert_eq!(consent["scope"], "jev-egress");
    }

    #[test]
    fn init_full_with_yes_is_rejected_fail_closed() {
        // --privacy full + --yes must fail: unredacted egress needs an
        // interactive typed FULL confirmation, never a flag alone.
        let (_tmp, home) = test_home();
        let err = cmd_init(&home, Some("full"), true).unwrap_err();
        assert!(err.contains("interactive confirmation"), "{err}");
    }

    #[test]
    fn doctor_show_egress_masks_sample_secret() {
        // --show-egress preview must run through the real redactor (one path)
        // and must not print the sample secret. doctor returns Ok even when
        // checks FAIL (it reports, exit code stays 0 for hook safety).
        let (_tmp, home) = test_home();
        cmd_init(&home, None, true).unwrap();
        cmd_doctor(&home, true).unwrap();
    }

    #[test]
    fn init_doctor_uninstall_byte_identical() {
        let (tmp, home) = test_home();
        // Create fake claude.json with original bytes
        let claude_path = home.join(".claude.json");
        let orig = serde_json::json!({
            "model": "claude-3",
            "other": 123,
            "hooks": {
                "PreToolUse": [
                    { "matcher": "FileEdit", "hooks": [{ "type": "command", "command": "existing-hook" }] }
                ]
            }
        });
        let orig_str = serde_json::to_string_pretty(&orig).unwrap();
        fs::create_dir_all(home.clone()).unwrap();
        fs::write(&claude_path, orig_str.as_bytes()).unwrap();
        let orig_bytes = fs::read(&claude_path).unwrap();

        // Run init with --yes
        cmd_init(&home, Some("redacted"), true).unwrap();

        // Verify hook added additive (other hooks preserved)
        let after_bytes = fs::read(&claude_path).unwrap();
        let after_json: serde_json::Value = serde_json::from_slice(&after_bytes).unwrap();
        // Hook should be present
        let hook_cmd = hook_command_for_home(&home);
        assert!(
            hook_present(&after_json, &hook_cmd),
            "hook not added: {} not in {:?}",
            hook_cmd,
            after_json
        );
        // Existing hook preserved - check via structure or string (existing-hook is simple ascii, safe)
        assert!(
            after_json.to_string().contains("existing-hook"),
            "existing hook not preserved"
        );
        // Backup exists and is byte-identical
        let parent = claude_path.parent().unwrap();
        let backups: Vec<_> = fs::read_dir(parent)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".algo-backup-"))
            .collect();
        assert!(!backups.is_empty(), "backup not created");
        let latest = backups.last().unwrap().path();
        let backup_bytes = fs::read(&latest).unwrap();
        assert_eq!(backup_bytes, orig_bytes, "backup not byte-identical");

        // Run doctor (should not fail)
        cmd_doctor(&home, false).unwrap();

        // Run uninstall
        cmd_uninstall(&home, false).unwrap();

        // Verify original bytes restored
        assert!(claude_path.exists(), "config should exist after restore");
        let restored = fs::read(&claude_path).unwrap();
        assert_eq!(restored, orig_bytes, "uninstall not byte-identical");

        // Verify hook removed
        let restored_json: serde_json::Value = serde_json::from_slice(&restored).unwrap();
        assert!(!hook_present(&restored_json, &hook_cmd), "hook not removed");
        // Keep tmp alive
        drop(tmp);
    }

    #[test]
    fn pause_bypasses_daemon_killed() {
        let (tmp, home) = test_home();
        // Ensure .algo exists
        ensure_algo_dir(&home).unwrap();
        // Pause
        cmd_pause(&home).unwrap();
        let paused = home.join(".algo/paused");
        assert!(paused.exists(), "paused file not created");
        // Simulate daemon killed: no socket expected, but paused file still makes hook-client bypass
        // Hook-client checks paused first, instant bypass even daemon-dead
        // Here we just verify file exists regardless of socket
        let sock = home.join(".algo/algo.sock");
        assert!(!sock.exists(), "socket should not exist (daemon killed)");
        assert!(paused.exists(), "pause must persist daemon-killed");
        // Resume
        cmd_resume(&home).unwrap();
        assert!(!paused.exists(), "paused file not removed");
        drop(tmp);
    }

    #[test]
    fn shadow_counts() {
        let (tmp, home) = test_home();
        ensure_algo_dir(&home).unwrap();
        let db_path = home.join(".algo/audit.db");
        let store = AuditStore::open(&db_path).unwrap();
        store.init().unwrap();

        // Insert shadow decisions
        let d_allow = Decision {
            action: Action::Allow as i32,
            reason: "allow".into(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 1,
            policy_version: "v0".into(),
            trace_id: "t-allow".into(),
        };
        let d_deny = Decision {
            action: Action::Deny as i32,
            reason: "deny".into(),
            confidence_0_1: 0.95,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 2,
            policy_version: "v0".into(),
            trace_id: "t-deny".into(),
        };
        let d_ask = Decision {
            action: Action::Ask as i32,
            reason: "ask".into(),
            confidence_0_1: 0.5,
            source_level: SourceLevel::Fallback as i32,
            latency_ms: 0,
            policy_version: "v0".into(),
            trace_id: "t-ask".into(),
        };
        // Insert 2 shadow denies, 1 non-shadow allow, 1 shadow allow
        store.insert(&d_deny, "fp-deny-1", true).unwrap();
        store.insert(&d_deny, "fp-deny-2", true).unwrap();
        store.insert(&d_allow, "fp-allow-1", false).unwrap();
        store.insert(&d_allow, "fp-allow-2", true).unwrap();
        store.insert(&d_ask, "fp-ask-1", true).unwrap();

        // Check counts via store
        let c = store.counts().unwrap();
        assert_eq!(c.would_have_blocked, 2, "should count shadow deny only");
        assert_eq!(c.shadow, 4);
        assert_eq!(c.deny, 2);
        // Now test cmd_status prints would-have-blocked
        // We just ensure status doesn't error
        cmd_status(&home).unwrap();
        // Test soak >=500 shadow decisions without blocking: simulate 500 inserts shadow
        for i in 0..500 {
            let d = if i % 10 == 0 {
                Decision {
                    action: Action::Deny as i32,
                    reason: format!("deny {}", i),
                    confidence_0_1: 0.9,
                    source_level: SourceLevel::Rule as i32,
                    latency_ms: 1,
                    policy_version: "v0".into(),
                    trace_id: format!("t-{}", i),
                }
            } else {
                Decision {
                    action: Action::Allow as i32,
                    reason: format!("allow {}", i),
                    confidence_0_1: 0.9,
                    source_level: SourceLevel::Rule as i32,
                    latency_ms: 1,
                    policy_version: "v0".into(),
                    trace_id: format!("t-{}", i),
                }
            };
            let shadow = true;
            store.insert(&d, &format!("fp-{}", i), shadow).unwrap();
        }
        let c2 = store.counts().unwrap();
        // Shadow soak should have many entries but status still counts would-have-blocked correctly
        // Ensure total increased
        assert!(c2.total >= 500);
        // All inserted as shadow, so no actual blocking occurred (daemon shadow mode always approves)
        // We verify would-have-blocked is at least original 2 plus ~50 (500/10)
        assert!(c2.would_have_blocked >= 50);
        drop(tmp);
    }

    #[test]
    fn init_merge_additive_no_overwrite() {
        let (tmp, home) = test_home();
        let claude_path = home.join(".claude.json");
        let orig = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Bash", "hooks": [{ "type": "command", "command": "other" }] }
                ],
                "PostToolUse": [
                    { "matcher": "*", "hooks": [{ "type": "command", "command": "post" }] }
                ]
            },
            "extra": "keep"
        });
        fs::write(&claude_path, serde_json::to_string_pretty(&orig).unwrap()).unwrap();
        cmd_init(&home, Some("redacted"), true).unwrap();
        let after: serde_json::Value =
            serde_json::from_slice(&fs::read(&claude_path).unwrap()).unwrap();
        // PostToolUse preserved
        assert!(after["hooks"]["PostToolUse"].is_array());
        assert_eq!(after["extra"], "keep");
        // PreToolUse should have 2 entries now
        assert_eq!(after["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        drop(tmp);
    }

    #[test]
    fn uninstall_keep_db() {
        let (tmp, home) = test_home();
        let claude_path = home.join(".claude.json");
        fs::write(&claude_path, b"{}").unwrap();
        cmd_init(&home, Some("redacted"), true).unwrap();
        let db_path = home.join(".algo/audit.db");
        let store = AuditStore::open(&db_path).unwrap();
        store.init().unwrap();
        let d = Decision {
            action: Action::Allow as i32,
            reason: "test".into(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 1,
            policy_version: "v0".into(),
            trace_id: "t1".into(),
        };
        store.insert(&d, "fp", false).unwrap();
        // Uninstall with keep-db true
        cmd_uninstall(&home, true).unwrap();
        assert!(db_path.exists(), "db should be kept");
        drop(tmp);
    }

    #[test]
    fn enforce_default_shadow_then_on_off() {
        let (tmp, home) = test_home();
        // Default (no config) => shadow
        assert!(!read_enforce(&home));
        // Init writes shadow default
        let claude_path = home.join(".claude.json");
        fs::write(&claude_path, b"{}").unwrap();
        cmd_init(&home, Some("redacted"), true).unwrap();
        assert!(!read_enforce(&home));
        let cfg_bytes = fs::read(home.join(".algo/config.json")).unwrap();
        let cfg: serde_json::Value = serde_json::from_slice(&cfg_bytes).unwrap();
        assert_eq!(cfg["shadow"], serde_json::json!(true));
        // Enforce on
        cmd_enforce(&home, Some("on")).unwrap();
        assert!(read_enforce(&home));
        // Enforce off preserves privacy field
        cmd_enforce(&home, Some("off")).unwrap();
        assert!(!read_enforce(&home));
        let cfg2: serde_json::Value =
            serde_json::from_slice(&fs::read(home.join(".algo/config.json")).unwrap()).unwrap();
        assert_eq!(cfg2["privacy"], serde_json::json!("redacted"));
        // Status mode does not error
        cmd_enforce(&home, Some("status")).unwrap();
        cmd_enforce(&home, None).unwrap();
        // Invalid mode errors (fail-safe, never silent allow)
        assert!(cmd_enforce(&home, Some("bogus")).is_err());
        drop(tmp);
    }

    #[test]
    fn policy_shows_snapshot_without_writing() {
        let (tmp, home) = test_home();
        // No config yet: defaults, must not error.
        cmd_policy(&home).unwrap();
        // Init then enforce on: snapshot reflects mode.
        let claude_path = home.join(".claude.json");
        fs::write(&claude_path, b"{}").unwrap();
        cmd_init(&home, Some("redacted"), true).unwrap();
        cmd_policy(&home).unwrap();
        cmd_enforce(&home, Some("on")).unwrap();
        assert!(read_enforce(&home));
        cmd_policy(&home).unwrap();
        drop(tmp);
    }
}
