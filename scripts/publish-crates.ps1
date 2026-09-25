# Ordered cargo publish for algo-* crates (local Windows). Usage:
#   .\scripts\publish-crates.ps1 -Mode dry-run     # no network publish
#   .\scripts\publish-crates.ps1 -Mode publish      # real publish (needs cargo login)
param([ValidateSet("dry-run", "publish")][string]$Mode = "dry-run")
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
if ([string]::IsNullOrEmpty($root)) { $root = (Get-Location).Path }

function Manifest-For($pkg) {
  if ($pkg -eq "algo-backend") { return "$root\backend\Cargo.toml" }
  if (@("algo-types", "algo-redact", "algo-fingerprint", "algo-shell-analysis", "algo-policy", "algo-provider") -contains $pkg) { return "$root\core\Cargo.toml" }
  return "$root\agent\Cargo.toml"
}

foreach ($line in (Get-Content "$root\scripts\publish-crates-order.txt")) {
  $pkg = $line.Trim()
  if ([string]::IsNullOrEmpty($pkg) -or $pkg.StartsWith("#")) { continue }
  $manifest = Manifest-For $pkg
  if ($Mode -eq "publish") {
    Write-Output "==> publishing $pkg"
    cargo publish --manifest-path $manifest -p $pkg
    Start-Sleep -Seconds 15
  } else {
    # --no-verify: metadata + packaging check only (fast, no build).
    Write-Output "==> dry-run $pkg"
    cargo publish --dry-run --no-verify --allow-dirty --manifest-path $manifest -p $pkg
  }
}
Write-Output "done ($Mode)"
