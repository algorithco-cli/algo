# apply-branch-protection.ps1 — enforce docs/branch-protection.json on main.
# HUMAN ACT: needs repo admin. Preflight 2026-09-23: GET protection → 404
# Branch not protected; GET rulesets → empty. GitHub docs gate protected
# branches to Pro/Team/Enterprise for private repos, so a 403 "Upgrade to
# GitHub Pro" at apply time is the expected plan-gate (flip to Pro, or public
# after license ADR-0001 + legal sign-off, then re-run).
# Usage: ./scripts/apply-branch-protection.ps1 [-CheckOnly]

param([switch]$CheckOnly)

$ErrorActionPreference = "Stop"
$OWNER = if ($env:OWNER) { $env:OWNER } else { "algorithcoguard" }
$REPO = if ($env:REPO) { $env:REPO } else { "algorithco-guard" }
$BRANCH = if ($env:BRANCH) { $env:BRANCH } else { "main" }
$BODY = Join-Path (Join-Path (Join-Path $PSScriptRoot "..") "docs") "branch-protection.json" | Resolve-Path | Select-Object -ExpandProperty Path

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "needs gh CLI (authenticated, admin on $OWNER/$REPO)" }
Get-Content $BODY -Raw | ConvertFrom-Json | Out-Null  # valid JSON or throw

Write-Host "[protect] preflight: GET branches/$BRANCH/protection"
$err = $null
try {
  $current = gh api "repos/$OWNER/$REPO/branches/$BRANCH/protection" --jq '.required_status_checks.contexts' 2>&1
  Write-Host "[protect] protection already present. Current required contexts:"
  Write-Host $current
} catch {
  $err = $_.Exception.Message + ($_ | Out-String)
  if ($err -match "Upgrade to GitHub Pro") {
    Write-Host "[protect] EXPECTED-BLOCKED (exit 2): plan-gated — private Free-plan repos cannot use branch protection." -ForegroundColor Yellow
    Write-Host "[protect] Flip to Pro (or public after license ADR-0001 + legal sign-off), then re-run."
    exit 2
  }
  Write-Host "[protect] no protection yet (or other error): $err" -ForegroundColor Yellow
}

if ($CheckOnly) { Write-Host "[protect] -CheckOnly: not applying."; exit 0 }

Write-Host "[protect] applying $BODY → $OWNER/$REPO@$BRANCH"
gh api --method PUT "repos/$OWNER/$REPO/branches/$BRANCH/protection" --input $BODY --jq '{strict: .required_status_checks.strict, contexts: .required_status_checks.contexts, code_owners: .required_pull_request_reviews.require_code_owner_reviews, approvals: .required_pull_request_reviews.required_approving_review_count, enforce_admins: .enforce_admins.enabled}'
Write-Host "[protect] applied. Verify: gh api repos/$OWNER/$REPO/branches/$BRANCH/protection"
