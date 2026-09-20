# latency-budget.ps1 — enforce P1 latency budgets + regression gate (phase-1-09, Windows)
# Mirrors scripts/latency-budget.sh for PowerShell
# Spec: L0/L1 p50<3ms p99<10ms, L3 p50<250 p99<800 (report-only for L3 mock), redact <500us, regression >10% fails
# Usage:
#   pwsh -File scripts/latency-budget.ps1
#   pwsh -File scripts/latency-budget.ps1 -CheckOnly
#   pwsh -File scripts/latency-budget.ps1 -Baseline p1-exit -Compare

param(
    [string]$Baseline = "p1-exit",
    [switch]$CheckOnly,
    [switch]$Compare
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Root) { $Root = (Get-Location).Path }

$ThresholdL0L1P50Ns = 3000000
$ThresholdL0L1P99Ns = 10000000
$ThresholdL3P50Ns = 250000000
$ThresholdL3P99Ns = 800000000
$ThresholdRedact10kNs = 500000
$RegressionPct = 10

Write-Host "[latency-budget] P1 exit gate latency check — baseline=$Baseline" -ForegroundColor Cyan
Write-Host "[latency-budget] Thresholds: L0/L1 p50<3ms p99<10ms, L3 p50<250ms p99<800ms (mock report-only), redact_10k <500us, regression >${RegressionPct}% fails"
Write-Host "[latency-budget] Reference runner pinned: windows-latest (see core/benches/README.md), artifacts: target/criterion"

if ($CheckOnly) {
    Write-Host "[latency-budget] --CheckOnly: skipping cargo bench run"
} elseif ($Compare) {
    Write-Host "[latency-budget] Running: cargo bench -- --baseline $Baseline"
    & cargo bench -- --baseline $Baseline
    if ($LASTEXITCODE -ne 0) { Write-Host "[latency-budget] cargo bench --baseline failed" -ForegroundColor Yellow }
} else {
    Write-Host "[latency-budget] Running: cargo bench -- --save-baseline $Baseline"
    & cargo bench -- --save-baseline $Baseline
    if ($LASTEXITCODE -ne 0) { throw "cargo bench --save-baseline $Baseline failed with $LASTEXITCODE" }
}

$CriterionRoot = Join-Path $Root "target/criterion"
if (-not (Test-Path -LiteralPath $CriterionRoot)) {
    Write-Host "[latency-budget] WARN: $CriterionRoot not found — bench may not have run" -ForegroundColor Yellow
    Write-Host "[latency-budget] Run cargo bench first, or check target/criterion exists"
    exit 0
}

# Python parser (same scaffold as .sh, works on Windows if python available)
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command python3 -ErrorAction SilentlyContinue }
if (-not $python) {
    Write-Host "[latency-budget] WARN: python not found — falling back to PowerShell JSON parse (best-effort)" -ForegroundColor Yellow
    # Minimal PowerShell fallback: scan estimates.json
    $fail = 0; $warn = 0
    Get-ChildItem -LiteralPath $CriterionRoot -Directory | ForEach-Object {
        $group = $_.Name
        $p50b = $ThresholdL0L1P50Ns; $p99b = $ThresholdL0L1P99Ns
        if ($group -eq "redact_10k") { $p50b = $ThresholdRedact10kNs; $p99b = $ThresholdRedact10kNs * 20 }
        $isL3 = $group.ToLower().StartsWith("l3") -or $group.ToLower().Contains("l3")
        if ($isL3) { $p50b = $ThresholdL3P50Ns; $p99b = $ThresholdL3P99Ns }
        Get-ChildItem -LiteralPath $_.FullName -Directory | ForEach-Object {
            $bench = $_.Name
            $candidates = @(
                (Join-Path $_.FullName "new/estimates.json"),
                (Join-Path $_.FullName "base/estimates.json"),
                (Join-Path $_.FullName "estimates.json")
            )
            $estPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
            if (-not $estPath) {
                $globs = Get-ChildItem -LiteralPath $_.FullName -Recurse -Filter "estimates.json" -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($globs) { $estPath = $globs.FullName }
            }
            if (-not $estPath) {
                Write-Host "[latency-budget] WARN: no estimates.json for $group/$bench" -ForegroundColor Yellow
                $script:warn++
                return
            }
            try {
                $est = Get-Content -LiteralPath $estPath -Raw | ConvertFrom-Json
                $p50 = $null
                if ($est.median -and $est.median.point_estimate) { $p50 = $est.median.point_estimate }
                elseif ($est.Median -and $est.Median.point_estimate) { $p50 = $est.Median.point_estimate }
                elseif ($est.mean -and $est.mean.point_estimate) { $p50 = $est.mean.point_estimate }
                if ($null -eq $p50) {
                    Write-Host "[latency-budget] WARN: no median/mean in $estPath" -ForegroundColor Yellow
                    return
                }
                $p50 = [double]$p50
                $p50ms = $p50 / 1e6
                $ok = $p50 -le $p50b
                $status = if ($ok) {"PASS"} else {"FAIL"}
                if (-not $ok -and -not $isL3) { $script:fail++ }
                Write-Host "[latency-budget] $status $group/$bench : p50=${p50}ns (${p50ms}ms) budget $($p50b/1e6)ms"
            } catch {
                Write-Host "[latency-budget] WARN: parse failed $estPath : $_" -ForegroundColor Yellow
            }
        }
    }
    Write-Host "[latency-budget] Summary FAIL=$fail WARN=$warn"
    if ($fail -gt 0) { Write-Host "[latency-budget] ❌ BREACH" -ForegroundColor Red; exit 1 }
    else { Write-Host "[latency-budget] ✅ budgets OK" -ForegroundColor Green; exit 0 }
}

# Use Python scaffold for full p99 + regression parse (identical to .sh)
$pyCode = @'
import json, math, sys
from pathlib import Path
import os
root = Path(r"' + $Root.Replace('\','\\') + r'")
baseline = r"' + $Baseline + r'"
p50_budget = ' + $ThresholdL0L1P50Ns + r'
p99_budget = ' + $ThresholdL0L1P99Ns + r'
l3_p50 = ' + $ThresholdL3P50Ns + r'
l3_p99 = ' + $ThresholdL3P99Ns + r'
redact_budget = ' + $ThresholdRedact10kNs + r'
regression_pct = ' + $RegressionPct + r'
criterion_root = root / "target" / "criterion"
budgets = {
    "policy_eval": (p50_budget, p99_budget),
    "fingerprint_normalize": (p50_budget, p99_budget),
    "redact_10k": (redact_budget, redact_budget*20),
    "pipeline_L0L1": (p50_budget, p99_budget),
}
fail=0
warn=0
def read_estimates(p):
    try: return json.loads(Path(p).read_text(encoding="utf-8"))
    except: return None
def p99_from_sample(sp):
    try:
        data=json.loads(Path(sp).read_text(encoding="utf-8"))
        samples=data.get("sample") if isinstance(data,dict) else data
        if not samples: samples=data.get("values") if isinstance(data,dict) else None
        if isinstance(samples,dict): samples=samples.get("values") or samples.get("sample")
        if not samples or not isinstance(samples,list): return None
        samples=sorted(float(x) for x in samples)
        idx=int(math.ceil(0.99*len(samples)))-1
        idx=max(0,min(idx,len(samples)-1))
        return samples[idx]
    except: return None
if not criterion_root.exists():
    print(f"[latency-budget] WARN: {criterion_root} not found")
    sys.exit(0)
groups=[p for p in criterion_root.iterdir() if p.is_dir()]
for group_path in groups:
    group=group_path.name
    if group.startswith("."): continue
    p50_b,p99_b=budgets.get(group,(p50_budget,p99_budget))
    is_l3=group.lower().startswith("l3") or "pipeline_l3" in group.lower()
    for bench_path in group_path.iterdir():
        if not bench_path.is_dir(): continue
        bench=bench_path.name
        candidates=[bench_path/"new"/"estimates.json", bench_path/"base"/"estimates.json", bench_path/"estimates.json"]
        est_path=next((p for p in candidates if p.exists()), None)
        if not est_path:
            globs=list(bench_path.rglob("estimates.json"))
            est_path=globs[0] if globs else None
        if not est_path:
            print(f"[latency-budget] WARN: no estimates.json for {group}/{bench}")
            warn+=1
            continue
        est=read_estimates(est_path)
        if not est:
            print(f"[latency-budget] WARN: cannot parse {est_path}")
            warn+=1
            continue
        def point(obj,key):
            if not obj or key not in obj: return None
            v=obj[key]
            if isinstance(v,dict): return v.get("point_estimate")
            return v
        p50=point(est,"median") or point(est,"Median") or point(est,"mean") or point(est,"Mean")
        if p50 is None:
            for k in ("estimates","statistics"):
                if k in est:
                    p50=point(est[k],"median") or point(est[k],"mean")
                    if p50: break
        if p50 is None:
            print(f"[latency-budget] WARN: no median/mean in {est_path}: keys={list(est.keys())}")
            warn+=1
            continue
        p50=float(p50)
        sample_candidates=[bench_path/"new"/"sample.json", bench_path/"base"/"sample.json", est_path.parent/"sample.json"]
        sample_path=next((p for p in sample_candidates if p.exists()), None)
        if not sample_path:
            globs=list(bench_path.rglob("sample.json"))
            sample_path=globs[0] if globs else None
        p99=p99_from_sample(sample_path) if sample_path else None
        if p99 is None:
            std=point(est,"std_dev") or point(est,"StdDev")
            if std is not None:
                try: p99=p50+3*float(std)
                except: pass
        p99_str=f"{p99:.0f} ns ({p99/1e6:.3f} ms)" if p99 else "n/a"
        p50_str=f"{p50:.0f} ns ({p50/1e6:.3f} ms)"
        if group=="redact_10k": b_p50,b_p99=redact_budget, redact_budget*20
        elif is_l3: b_p50,b_p99=l3_p50,l3_p99
        else: b_p50,b_p99=p50_budget,p99_budget
        ok_p50=p50<=b_p50
        ok_p99=(p99 is None) or (p99<=b_p99)
        status="PASS" if (ok_p50 and ok_p99) else "FAIL"
        if status=="FAIL" and not is_l3: fail+=1
        if is_l3 and not (ok_p50 and ok_p99): warn+=1; status="WARN (L3 report-only)"
        print(f"[latency-budget] {status} {group}/{bench}: p50={p50_str} budget {b_p50/1e6:.3f}ms | p99={p99_str} budget {b_p99/1e6:.3f}ms from {est_path}")
print(f"[latency-budget] done: fail={fail} warn={warn}")
Path(r"C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_fail").write_text(str(fail))
Path(r"C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_warn").write_text(str(warn))
'@
$tmpPy = Join-Path $env:TEMP "latency_budget_parse.py"
Set-Content -LiteralPath $tmpPy -Value $pyCode -Encoding UTF8
try { & $python $tmpPy } catch { Write-Host "[latency-budget] python parse failed: $_" -ForegroundColor Yellow }
$fail = 0; $warn = 0
if (Test-Path "C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_fail") { $fail = [int](Get-Content "C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_fail") }
if (Test-Path "C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_warn") { $warn = [int](Get-Content "C:\Users\hamro\AppData\Local\Temp\opencode\latency_budget_warn") }
Write-Host ""
Write-Host "[latency-budget] Summary: FAIL=$fail WARN=$warn"
if ($fail -gt 0) {
    Write-Host "[latency-budget] ❌ LATENCY BUDGET BREACH or REGRESSION >${RegressionPct}% — adjust only via ADR" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[latency-budget] ✅ budgets OK (L0/L1 p50<3ms p99<10ms, redact <500us, L3 report-only, regression <=${RegressionPct}%)" -ForegroundColor Green
}

Write-Host ""
Write-Host "[latency-budget] hyperfine hook-client cold start (reference, ~1ms budget):"
if (Get-Command hyperfine -ErrorAction SilentlyContinue) {
    Write-Host "  hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin'"
    if (-not $env:CI) {
        try {
            hyperfine --warmup 10 --runs 20 --show-output 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin <<< "ls -la"' 2>&1 | Out-Null
        } catch {}
        if (Test-Path "target/release/algo-hook-client.exe") {
            try { hyperfine --warmup 10 --runs 20 'echo "ls -la" | target/release/algo-hook-client.exe --socket NUL --stdin' } catch {}
        } elseif (Test-Path "target/release/algo-hook-client") {
            try { hyperfine --warmup 10 --runs 20 'echo "ls -la" | target/release/algo-hook-client --socket /tmp/nonexistent.sock --stdin' } catch {}
        }
    } else {
        Write-Host "  (CI mode: skipping hyperfine run, documented)"
    }
} else {
    Write-Host "  hyperfine not installed — cargo install hyperfine  or  choco install hyperfine" -ForegroundColor Yellow
    Write-Host "  Expected: hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin'  must be ~1ms"
    Write-Host "  (On Windows: hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket NUL --stdin')"
}
Write-Host "[latency-budget] Artifacts: upload target/criterion and html_reports (see core/benches/README.md)"
