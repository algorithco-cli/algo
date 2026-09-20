# Phase 4 — TUI + GitHub App + Windows

## TUI (`agent/tui`, ratatui)

Live decision feed, stats (auto-approved/asked/blocked), policy editor (local rules, dry-run vs history). Offline-capable; read-only when daemon down. TUI crash never blocks hooks (no decision-path coupling). Keyboard-only flow tested.

## GitHub App (`backend`, octocrab)

PR risk scoring + CI-failure triage (minimal: score + comment + dashboard link); SSO/SCIM via Zitadel; rate-limited authed webhooks with secret verification; background queue. AC: signed-webhook rejection test; fixture-repo E2E (PR scored, CI triaged); SCIM provision/deprovision test. Human review (auth).

## Windows hardening ([DECISION] D-09 gates scope)

Default P1-P3 macOS+Linux; harden in P4 unless ADR pulls forward. Named-pipe transport (vs Unix socket), install paths + ACLs, init/uninstall/pause on PowerShell, winget/scoop (+ musl/Homebrew/apt-rpm), signed releases + SBOM + verified self-update, CI matrix (Win 11 + Server + macOS/Linux), latency re-baselined on Windows (ADR if unreachable).

AC: Windows CI install/uninstall/pause E2E; pipe-failure → ask proven; signature verification test.
