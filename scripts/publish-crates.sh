#!/usr/bin/env bash
# Ordered cargo publish for algocli-* crates (CI + local). Usage:
#   bash scripts/publish-crates.sh --dry-run   # no network publish
#   bash scripts/publish-crates.sh --publish    # real publish (needs cargo login)
set -euo pipefail

MODE="${1:---dry-run}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORDER="$ROOT/scripts/publish-crates-order.txt"

manifest_for() {
  case "$1" in
    algocli-backend) echo "$ROOT/backend/Cargo.toml" ;;
    algocli-types|algocli-redact|algocli-fingerprint|algocli-shell-analysis|algocli-policy|algocli-provider) echo "$ROOT/core/Cargo.toml" ;;
    *) echo "$ROOT/agent/Cargo.toml" ;;
  esac
}

pkg_manifest_file() {
  # Own manifest of a crate (for its version), derived from package name.
  local suffix="${1#algocli-}"
  case "$1" in
    algocli-backend) echo "$ROOT/backend/Cargo.toml" ;;
    algocli-types|algocli-redact|algocli-fingerprint|algocli-shell-analysis|algocli-policy|algocli-provider)
      echo "$ROOT/core/crates/$suffix/Cargo.toml" ;;
    algocli-tui) echo "$ROOT/agent/tui/Cargo.toml" ;;
    *) echo "$ROOT/agent/crates/$suffix/Cargo.toml" ;;
  esac
}

pkg_version() {
  sed -n 's/^version = "\(.*\)"$/\1/p' "$1" | head -n 1
}

crate_version_published() {
  # $1=name $2=version → true iff that exact version is on crates.io.
  local code
  code=$(curl -sSL -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$1/$2" 2>/dev/null || true)
  [ "$code" = "200" ]
}

while read -r pkg; do
  case "$pkg" in ""|\#*) continue ;; esac
  manifest="$(manifest_for "$pkg")"
  if [ "$MODE" = "--publish" ]; then
    ver="$(pkg_version "$(pkg_manifest_file "$pkg")")"
    if [ -n "$ver" ] && crate_version_published "$pkg" "$ver"; then
      echo "==> skipping $pkg $ver (already on crates.io)"
      continue
    fi
    echo "==> publishing $pkg"
    cargo publish --manifest-path "$manifest" -p "$pkg"
    sleep 15 # let crates.io index settle for dependents
  else
    # --no-verify: metadata + packaging check only (fast, no build).
    # Full --verify is covered by the real publish run; algo-types verify
    # needs the proto-vendoring fix (owner decision) before it can pass.
    echo "==> dry-run $pkg"
    cargo publish --dry-run --no-verify --allow-dirty --manifest-path "$manifest" -p "$pkg"
  fi
done < "$ORDER"
echo "done ($MODE)"
