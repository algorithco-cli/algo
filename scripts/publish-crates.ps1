# Ordered cargo publish for algocli-* crates (local Windows). Usage:
#   .\scripts\publish-crates.ps1 -Mode dry-run     # no network publish
#   .\scripts\publish-crates.ps1 -Mode publish      # real publish (needs cargo login)
param([ValidateSet("dry-run", "publish")][string]$Mode = "dry-run")
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
if ([string]::IsNullOrEmpty($root)) { $root = (Get-Location).Path }

function Manifest-For($pkg) {
  if ($pkg -eq "algocli-backend") { return "$root\backend\Cargo.toml" }
  if (@("algocli-types", "algocli-redact", "algocli-fingerprint", "algocli-shell-analysis", "algocli-policy", "algocli-provider") -contains $pkg) { return "$root\core\Cargo.toml" }
  return "$root\agent\Cargo.toml"
}

function PkgManifest-File($pkg) {
  $suffix = $pkg -replace '^algocli-', ''
  if ($pkg -eq "algocli-backend") { return "$root\backend\Cargo.toml" }
  if (@("algocli-types", "algocli-redact", "algocli-fingerprint", "algocli-shell-analysis", "algocli-policy", "algocli-provider") -contains $pkg) { return "$root\core\crates\$suffix\Cargo.toml" }
  if ($pkg -eq "algocli-tui") { return "$root\agent\tui\Cargo.toml" }
  return "$root\agent\crates\$suffix\Cargo.toml"
}

function Get-PkgVersion($manifestFile) {
  foreach ($line in (Get-Content $manifestFile)) {
    if ($line -match '^version = "(.+)"$') { return $Matches[1] }
  }
  throw "no version in $manifestFile"
}

function Test-CratePublished($pkg, $ver) {
  try {
    $r = Invoke-WebRequest -Uri "https://crates.io/api/v1/crates/$pkg/$ver" -UseBasicParsing -Method Head -TimeoutSec 30
    return ($r.StatusCode -eq 200)
  } catch { return $false }
}

foreach ($line in (Get-Content "$root\scripts\publish-crates-order.txt")) {
  $pkg = $line.Trim()
  if ([string]::IsNullOrEmpty($pkg) -or $pkg.StartsWith("#")) { continue }
  $manifest = Manifest-For $pkg
  if ($Mode -eq "publish") {
    $ver = Get-PkgVersion (PkgManifest-File $pkg)
    if (Test-CratePublished $pkg $ver) {
      Write-Output "==> skipping $pkg $ver (already on crates.io)"
      continue
    }
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
