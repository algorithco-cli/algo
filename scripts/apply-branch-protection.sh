#!/usr/bin/env bash
# apply-branch-protection.sh — enforce docs/branch-protection.json on main.
# HUMAN ACT: needs repo admin. Preflight 2026-09-23: GET protection → 404
# Branch not protected; GET rulesets → empty. GitHub docs gate protected
# branches to Pro/Team/Enterprise for private repos, so a 403 "Upgrade to
# GitHub Pro" at apply time is the expected plan-gate (flip to Pro, or public
# after license ADR-0001 + legal sign-off, then re-run).
# Usage: ./scripts/apply-branch-protection.sh [--check-only]
set -euo pipefail

OWNER="${OWNER:-algorithcoguard}"
REPO="${REPO:-algorithco-guard}"
BRANCH="${BRANCH:-main}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BODY="$ROOT/docs/branch-protection.json"

if ! command -v gh >/dev/null 2>&1; then
  echo "needs gh CLI (authenticated, admin on $OWNER/$REPO)" >&2
  exit 1
fi
if [[ ! -f "$BODY" ]]; then
  echo "missing $BODY" >&2
  exit 1
fi
python3 -c "import json; json.load(open('$BODY'))" || { echo "branch-protection.json is not valid JSON" >&2; exit 1; }

echo "[protect] preflight: GET branches/$BRANCH/protection"
if CODE=$(gh api "repos/$OWNER/$REPO/branches/$BRANCH/protection" --jq '.required_status_checks.contexts // empty' 2>/tmp/protect_err.txt); then
  echo "[protect] protection already present. Current required contexts:"
  echo "$CODE"
else
  if grep -q "Upgrade to GitHub Pro" /tmp/protect_err.txt 2>/dev/null; then
    echo "[protect] EXPECTED-BLOCKED (exit 2): plan-gated — private Free-plan repos cannot use branch protection." >&2
    echo "[protect] Flip to Pro (or public after license ADR-0001 + legal sign-off), then re-run." >&2
    exit 2
  fi
  echo "[protect] no protection yet (or other error):" >&2
  cat /tmp/protect_err.txt >&2 || true
fi

if [[ "${1:-}" == "--check-only" ]]; then
  echo "[protect] --check-only: not applying."
  exit 0
fi

echo "[protect] applying $BODY → $OWNER/$REPO@$BRANCH"
gh api --method PUT "repos/$OWNER/$REPO/branches/$BRANCH/protection" --input "$BODY" --jq '{strict: .required_status_checks.strict, contexts: .required_status_checks.contexts, code_owners: .required_pull_request_reviews.require_code_owner_reviews, approvals: .required_pull_request_reviews.required_approving_review_count, enforce_admins: .enforce_admins.enabled}'
echo "[protect] applied. Verify: gh api repos/$OWNER/$REPO/branches/$BRANCH/protection"
