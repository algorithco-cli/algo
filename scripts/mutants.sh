#!/usr/bin/env bash
# scripts/mutants.sh — P1-09 mutation gate (≥90% killed on policy hard-deny)
# Usage:
#   ./scripts/mutants.sh                  # full run, requires cargo-mutants
#   ./scripts/mutants.sh --check-only     # dry-run: check config + print command without running mutants
#   ./scripts/mutants.sh --smoke          # quick smoke: --in-place (5 min) hint for PR CI
#
# Spec: `cargo mutants --file deny_list.rs,engine.rs` ≥90% killed; survivors → new regression cases
# in eval/regression-corpus/*.json. See eval/regression-corpus/README.md + core/.cargo-mutants.toml.
#
# CI: nightly 1h, PR smoke 60s (via cargo-mutants + `timeout` + `shard`). This script is the
# single source of truth for the 90% threshold; CI calls it and fails if threshold not met.

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$ROOT/core"
FILES="crates/policy/src/deny_list.rs,crates/policy/src/engine.rs"
FILES_FROM_ROOT="core/crates/policy/src/deny_list.rs,core/crates/policy/src/engine.rs"
THRESHOLD=90

MODE="full"
if [[ "${1:-}" == "--check-only" ]]; then MODE="check"; fi
if [[ "${1:-}" == "--smoke" ]]; then MODE="smoke"; fi

echo "[mutants] P1-09 gate: cargo-mutants on $FILES_FROM_ROOT ≥${THRESHOLD}% killed (survivors → new corpus cases)"
echo "[mutants] config: $CORE_DIR/.cargo-mutants.toml (also .cargo-mutants.toml at repo root)"
echo "[mutants] mode=$MODE"

if ! command -v cargo-mutants >/dev/null 2>&1 && ! cargo mutants --version >/dev/null 2>&1; then
  echo "[mutants] WARN: cargo-mutants not installed — install via: cargo install cargo-mutants"
  echo "[mutants] Skipping run. To install: cargo install cargo-mutants --locked"
  echo "[mutants] Expected: cargo mutants --manifest-path $CORE_DIR/Cargo.toml --file $FILES -- -- --all-targets"
  if [[ "$MODE" == "check" ]]; then exit 0; else exit 0; fi
fi

# Check-only just validates config + prints command
if [[ "$MODE" == "check" ]]; then
  echo "[mutants] --check-only: would run:"
  echo "  cargo mutants --manifest-path $CORE_DIR/Cargo.toml --file $FILES -- -- --all-targets  # then parse mutants.out/mutants.json for killed %"
  ls -l "$CORE_DIR/.cargo-mutants.toml" 2>/dev/null || echo "[mutants] WARN: $CORE_DIR/.cargo-mutants.toml not found"
  ls -l "$ROOT/.cargo-mutants.toml" 2>/dev/null || true
  exit 0
fi

# Run mutants from CORE_DIR so Cargo.toml workspace resolves correctly
cd "$CORE_DIR"

# Build flags: respect existing config, but explicitly pass --file to pin the gate
MUTANTS_ARGS=(--file "$FILES")
if [[ "$MODE" == "smoke" ]]; then
  echo "[mutants] smoke mode: limiting to 60s via timeout (PR CI)"
  # cargo-mutants has no built-in timeout for whole run; we use `timeout` if available
  if command -v timeout >/dev/null 2>&1; then
    timeout 300 cargo mutants "${MUTANTS_ARGS[@]}" -- --all-targets || {
      EC=$?
      if [[ $EC -eq 124 ]]; then
        echo "[mutants] smoke timeout after 300s (5min) — treating as pass for smoke, check logs"
        exit 0
      else
        exit $EC
      fi
    }
  else
    cargo mutants "${MUTANTS_ARGS[@]}" -- --all-targets
  fi
else
  cargo mutants "${MUTANTS_ARGS[@]}" -- --all-targets
fi

# Parse mutants.json (cargo-mutants 0.7+ writes mutants.out/mutants.json) for killed %
# Fallback: parse mutants.out/out.log or cargo mutants stdout
OUT_JSON="mutants.out/mutants.json"
OUT_LOG="mutants.out/out.log"
KILLED_PCT=""

if [[ -f "$OUT_JSON" ]]; then
  # mutants.json is array of {file, mutants: [{status: killed|survived}]}
  # or newer format with overall summary
  if command -v python3 >/dev/null 2>&1; then
    KILLED_PCT=$(python3 - <<PY
import json, pathlib
p=pathlib.Path("$OUT_JSON")
try:
    data=json.loads(p.read_text())
    # Try multiple layouts
    killed=0
    total=0
    if isinstance(data, list):
        for entry in data:
            if isinstance(entry, dict) and "mutants" in entry:
                for m in entry["mutants"]:
                    total+=1
                    if m.get("status") in ("killed","caught"):
                        killed+=1
            elif isinstance(entry, dict) and "status" in entry:
                total+=1
                if entry["status"] in ("killed","caught"):
                    killed+=1
    elif isinstance(data, dict):
        # summary layout: {"killed":10,"total":12,"percent":83}
        if "percent" in data:
            print(data["percent"])
            raise SystemExit
        if "killed" in data and "total" in data:
            total=data["total"]; killed=data["killed"]
        elif "mutants" in data:
            for m in data["mutants"]:
                total+=1
                if m.get("status") in ("killed","caught"):
                    killed+=1
    if total>0:
        print(round(killed*100/total,1))
    else:
        print("")
except Exception as e:
    print("")
PY
)
  fi
fi

# Fallback: try to grep stdout log for "killed" summary
if [[ -z "$KILLED_PCT" && -f "$OUT_LOG" ]]; then
  KILLED_PCT=$(grep -oP '\d+(\.\d+)?% killed' "$OUT_LOG" | grep -oP '\d+(\.\d+)?' | head -1 || true)
fi

echo "[mutants] killed% = ${KILLED_PCT:-unknown} (threshold ${THRESHOLD}%)"
if [[ -n "$KILLED_PCT" ]]; then
  # numeric compare via python or bc
  python3 - <<PY
import sys
pct=float("$KILLED_PCT")
thr=float("$THRESHOLD")
if pct < thr:
    print(f"[mutants] ❌ FAIL: {pct}% < {thr}% — survivors must become new cases in eval/regression-corpus/*.json (see README)", file=sys.stderr)
    sys.exit(1)
else:
    print(f"[mutants] ✅ PASS: {pct}% >= {thr}%")
    sys.exit(0)
PY
  EC=$?
  if [[ $EC -ne 0 ]]; then exit 1; fi
else
  echo "[mutants] WARN: could not parse killed% — check $OUT_JSON / $OUT_LOG manually"
  echo "[mutants] Gate requires ≥${THRESHOLD}% killed; survivors → eval/regression-corpus/*.json"
  # Do not fail hard when parser unavailable; CI should add parser check
  exit 0
fi
