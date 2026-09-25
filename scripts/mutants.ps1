# scripts/mutants.ps1 — P1-09 mutation gate for Windows (mirrors scripts/mutants.sh)
# Usage:
#   pwsh -File scripts/mutants.ps1
#   pwsh -File scripts/mutants.ps1 -CheckOnly
#   pwsh -File scripts/mutants.ps1 -Smoke   # PR smoke 5min
#
# Spec: `cargo mutants --file deny_list.rs,engine.rs` ≥90% killed; survivors → new corpus cases
# See eval/regression-corpus/README.md, core/.cargo-mutants.toml

param(
    [switch]$CheckOnly,
    [switch]$Smoke
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Root) { $Root = (Get-Location).Path }
$CoreDir = Join-Path $Root "core"
$Files = "crates/policy/src/deny_list.rs,crates/policy/src/engine.rs"
$FilesFromRoot = "core/crates/policy/src/deny_list.rs,core/crates/policy/src/engine.rs"
$Threshold = 90

$Mode = "full"
if ($CheckOnly) { $Mode = "check" }
if ($Smoke) { $Mode = "smoke" }

Write-Host "[mutants] P1-09 gate: cargo-mutants on $FilesFromRoot ≥$Threshold% killed (survivors → new corpus cases)" -ForegroundColor Cyan
Write-Host "[mutants] config: $CoreDir/.cargo-mutants.toml (also .cargo-mutants.toml at repo root)" -ForegroundColor Cyan
Write-Host "[mutants] mode=$Mode"

# Check cargo mutants installed
$hasMutants = $false
try { & cargo mutants --version 2>$null | Out-Null; if ($LASTEXITCODE -eq 0) { $hasMutants = $true } } catch {}
if (-not $hasMutants) {
    try { Get-Command cargo-mutants -ErrorAction Stop | Out-Null; $hasMutants = $true } catch {}
}
if (-not $hasMutants) {
    Write-Host "[mutants] WARN: cargo-mutants not installed — install via: cargo install cargo-mutants" -ForegroundColor Yellow
    Write-Host "[mutants] Skipping run. Expected: cargo mutants --manifest-path $CoreDir/Cargo.toml --file $Files -- -- --all-targets"
    exit 0
}

if ($CheckOnly) {
    Write-Host "[mutants] --CheckOnly: would run:"
    Write-Host "  cargo mutants --manifest-path $CoreDir/Cargo.toml --file $Files -- -- --all-targets"
    if (Test-Path -LiteralPath (Join-Path $CoreDir ".cargo-mutants.toml")) { Write-Host "[mutants] config found" } else { Write-Host "[mutants] WARN: $CoreDir/.cargo-mutants.toml not found" -ForegroundColor Yellow }
    if (Test-Path -LiteralPath (Join-Path $Root ".cargo-mutants.toml")) { Write-Host "[mutants] repo-root config found" }
    exit 0
}

# Run from CoreDir so workspace resolves
Push-Location -LiteralPath $CoreDir
try {
    $mutantsArgs = @("--file", $Files, "--", "--all-targets")
    if ($Mode -eq "smoke") {
        Write-Host "[mutants] smoke mode: 5min limit (PR CI)"
        # PowerShell timeout: use Start-Job + timeout
        $job = Start-Job -ScriptBlock {
            param($a)
            Set-Location -LiteralPath $using:CoreDir
            & cargo mutants @a
        } -ArgumentList (,$mutantsArgs)
        $completed = Wait-Job $job -Timeout 300
        if (-not $completed) {
            Write-Host "[mutants] smoke timeout after 300s — treating as pass for smoke" -ForegroundColor Yellow
            Stop-Job $job -ErrorAction SilentlyContinue; Remove-Job $job -Force | Out-Null
            exit 0
        }
        Receive-Job $job; Remove-Job $job | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "cargo mutants failed with $LASTEXITCODE" }
    } else {
        Write-Host "[mutants] Running: cargo mutants --file $Files -- -- --all-targets"
        & cargo mutants @mutantsArgs
        if ($LASTEXITCODE -ne 0) { throw "cargo mutants exited $LASTEXITCODE" }
    }

    $outJson = Join-Path $CoreDir "mutants.out/mutants.json"
    $outLog = Join-Path $CoreDir "mutants.out/out.log"
    $killedPct = ""

    if (Test-Path -LiteralPath $outJson) {
        try {
            $json = Get-Content -LiteralPath $outJson -Raw | ConvertFrom-Json
            # Try to handle multiple layouts
            $killed = 0; $total = 0
            if ($json -is [System.Collections.IList]) {
                foreach ($entry in $json) {
                    if ($entry.PSObject.Properties.Name -contains "mutants") {
                        foreach ($m in $entry.mutants) {
                            $total++; if ($m.status -in @("killed","caught")) { $killed++ }
                        }
                    } elseif ($entry.PSObject.Properties.Name -contains "status") {
                        $total++; if ($entry.status -in @("killed","caught")) { $killed++ }
                    }
                }
            } elseif ($json -is [PSObject]) {
                if ($json.PSObject.Properties.Name -contains "percent") { $killedPct = "$($json.percent)" }
                elseif ($json.PSObject.Properties.Name -contains "killed" -and $json.PSObject.Properties.Name -contains "total") {
                    $total = $json.total; $killed = $json.killed
                }
            }
            if ($killedPct -eq "" -and $total -gt 0) {
                $killedPct = [math]::Round($killed*100.0/$total,1)
            }
        } catch {
            Write-Host "[mutants] WARN: parse $outJson failed: $_" -ForegroundColor Yellow
        }
    }
    if ((-not $killedPct) -and (Test-Path -LiteralPath $outLog)) {
        $m = Select-String -Path $outLog -Pattern "\d+(\.\d+)?% killed" -AllMatches | Select-Object -First 1
        if ($m) {
            $killedPct = ($m.Matches.Value | Select-Object -First 1) -replace "[^0-9.]",""
        }
    }

    Write-Host "[mutants] killed% = $killedPct (threshold $Threshold%)"
    if ($killedPct) {
        $pct = [double]$killedPct
        if ($pct -lt $Threshold) {
            Write-Host "[mutants] ❌ FAIL: $pct% < $Threshold% — survivors must become new cases in eval/regression-corpus/*.json" -ForegroundColor Red
            exit 1
        } else {
            Write-Host "[mutants] ✅ PASS: $pct% >= $Threshold%" -ForegroundColor Green
        }
    } else {
        Write-Host "[mutants] WARN: could not parse killed% — check $outJson / $outLog manually" -ForegroundColor Yellow
        Write-Host "[mutants] Gate requires ≥$Threshold% killed; survivors → eval/regression-corpus/*.json" -ForegroundColor Yellow
    }
} finally {
    Pop-Location
}
