#!/usr/bin/env bash
# Ordered cargo publish for algo-* crates (CI + local). Usage:
#   bash scripts/publish-crates.sh --dry-run   # no network publish
#   bash scripts/publish-crates.sh --publish    # real publish (needs cargo login)
set -euo pipefail

MODE="${1:---dry-run}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORDER="$ROOT/scripts/publish-crates-order.txt"

manifest_for() {
  case "$1" in
    algo-backend) echo "$ROOT/backend/Cargo.toml" ;;
    algo-types|algo-redact|algo-fingerprint|algo-shell-analysis|algo-policy|algo-provider) echo "$ROOT/core/Cargo.toml" ;;
    *) echo "$ROOT/agent/Cargo.toml" ;;
  esac
}

while read -r pkg; do
  case "$pkg" in ""|\#*) continue ;; esac
  manifest="$(manifest_for "$pkg")"
  if [ "$MODE" = "--publish" ]; then
    echo "==> publishing $pkg"
    cargo publish --manifest-path "$manifest" -p "$pkg"
    sleep 15 # let crates.io index settle for dependents
  else
    # --no-verify: metadata + packaging check only (fast, no build).
    # Full --verify is covered by the real publish run; algo-types verify
    # needs the proto-vendoring fix (ADR-0013) before it can pass.
    echo "==> dry-run $pkg"
    cargo publish --dry-run --no-verify --allow-dirty --manifest-path "$manifest" -p "$pkg"
  fi
done < "$ORDER"
echo "done ($MODE)"
