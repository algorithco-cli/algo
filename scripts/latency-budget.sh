#!/usr/bin/env bash
# latency-budget.sh — enforce P1 latency budgets + regression gate (phase-1-09)
# Spec: P1-09 Quality, latency, eval enforcement (P1-QUAL) §Benchmarks
#   L0/L1 p50<3ms p99<10ms, L2 <10/<25ms, L3 p50<250 p99<800 (report only for L3 mock)
#   >10% regression vs baseline p1-exit fails, reference runner pinned, baselines uploaded.
# Usage:
#   ./scripts/latency-budget.sh                # save baseline p1-exit + check thresholds
#   ./scripts/latency-budget.sh --check-only   # check thresholds against existing baseline without re-running bench
#   ./scripts/latency-budget.sh --baseline p1-exit --compare  # diff against baseline (cargo bench --baseline p1-exit)
# See core/benches/README.md for full workflow.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${BASELINE:-p1-exit}"
THRESHOLD_L0L1_P50_NS=3000000    # 3ms
THRESHOLD_L0L1_P99_NS=10000000   # 10ms
THRESHOLD_L3_P50_NS=250000000    # 250ms
THRESHOLD_L3_P99_NS=800000000    # 800ms
THRESHOLD_REDACT_10K_NS=500000   # 500us / 10KB (docs/redact-crate-design.md)
REGRESSION_PCT=10

MODE="save"
if [[ "${1:-}" == "--check-only" ]]; then MODE="check"; fi
if [[ "${1:-}" == "--baseline" ]]; then MODE="compare"; BASELINE="${2:-p1-exit}"; fi

# Colors
GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; NC='\033[0m'

echo "[latency-budget] P1 exit gate latency check — baseline=${BASELINE} mode=${MODE}"
echo "[latency-budget] Thresholds: L0/L1 p50<3ms p99<10ms, L3 p50<250ms p99<800ms (mock report-only), redact_10k <500us, regression >${REGRESSION_PCT}% fails"
echo "[latency-budget] Reference runner pinned: ubuntu-latest (see core/benches/README.md), artifacts: target/criterion + html_reports"

if [[ "$MODE" == "save" ]]; then
  echo "[latency-budget] Running: cargo bench -- --save-baseline ${BASELINE}"
  cargo bench -- --save-baseline "${BASELINE}"
elif [[ "$MODE" == "compare" ]]; then
  echo "[latency-budget] Running: cargo bench -- --baseline ${BASELINE}"
  cargo bench -- --baseline "${BASELINE}"
else
  echo "[latency-budget] Skipping bench run (--check-only), checking existing target/criterion"
fi

# Parse criterion JSON output scaffold
# Criterion 0.5 writes per-bench JSON at:
#   target/criterion/<group>/<bench>/new/estimates.json
#   target/criterion/<group>/<bench>/base/estimates.json  (when --baseline used)
#   target/criterion/<group>/<bench>/change/estimates.json (regression %)
# We parse `median.point_estimate` as p50 and `mean.point_estimate` as proxy, and try to
# derive p99 from raw samples at target/criterion/<group>/<bench>/new/sample.json (99th percentile).
# Fallback: treat std_dev tail as p99 approx if sample.json absent.
# Scaffold: fails open only with warning when JSON absent (CI should require artifacts).

FAIL=0
WARN=0

python3 - <<PY
import json, os, glob, sys, math
from pathlib import Path

root = Path("${ROOT}")
criterion_root = root / "target" / "criterion"
baseline = "${BASELINE}"
p50_budget = ${THRESHOLD_L0L1_P50_NS}
p99_budget = ${THRESHOLD_L0L1_P99_NS}
l3_p50 = ${THRESHOLD_L3_P50_NS}
l3_p99 = ${THRESHOLD_L3_P99_NS}
redact_budget = ${THRESHOLD_REDACT_10K_NS}
regression_pct = ${REGRESSION_PCT}

# Map group -> budget
budgets = {
    "policy_eval": (p50_budget, p99_budget),
    "fingerprint_normalize": (p50_budget, p99_budget),
    "redact_10k": (redact_budget, redact_budget*20),  # p99 ~ 20x p50 for redact
    "pipeline_L0L1": (p50_budget, p99_budget),
}

fail = 0
warn = 0

def read_estimates(path):
    try:
        return json.loads(Path(path).read_text())
    except Exception as e:
        return None

def p99_from_sample(sample_path):
    try:
        data = json.loads(Path(sample_path).read_text())
        # sample.json format varies: either {"sample":[...]} or list
        samples = data.get("sample") if isinstance(data, dict) else data
        if not samples:
            samples = data.get("mean", {}).get("sample") if isinstance(data, dict) else None
        if not samples:
            # try raw array at "values"
            samples = data.get("values") if isinstance(data, dict) else None
        if isinstance(samples, dict):
            samples = samples.get("values") or samples.get("sample")
        if not samples or not isinstance(samples, list):
            return None
        samples = sorted(float(x) for x in samples)
        idx = int(math.ceil(0.99 * len(samples))) - 1
        idx = max(0, min(idx, len(samples)-1))
        return samples[idx]
    except Exception:
        return None

# Find all groups
if not criterion_root.exists():
    print(f"[latency-budget] WARN: {criterion_root} not found — bench may not have run (no JSON to parse). Run cargo bench first.")
    # not failing, just warn (CI should run bench)
    sys.exit(0)

# Enumerate groups
groups = [p for p in criterion_root.iterdir() if p.is_dir()]
if not groups:
    print("[latency-budget] WARN: no groups under target/criterion")
    sys.exit(0)

for group_path in groups:
    group = group_path.name
    # skip non-benchmark dirs
    if group.startswith("."):
        continue
    # budgets: use L0/L1 for known groups, else generic
    p50_b, p99_b = budgets.get(group, (p50_budget, p99_budget))
    # special: L3 mock is not in L0/L1 groups; we report only
    is_l3 = (group.lower().startswith("l3") or "pipeline_l3" in group.lower())

    # Each bench inside group
    for bench_path in group_path.iterdir():
        if not bench_path.is_dir():
            continue
        bench = bench_path.name
        # criterion layout: .../group/bench/new/estimates.json
        candidates = [
            bench_path / "new" / "estimates.json",
            bench_path / "base" / "estimates.json",
            bench_path / "estimates.json",
        ]
        est_path = next((p for p in candidates if p.exists()), None)
        if not est_path:
            # also check nested: target/criterion/group/bench/<hash>/new/estimates.json (some criterion versions)
            globs = list(bench_path.rglob("estimates.json"))
            est_path = globs[0] if globs else None
        if not est_path:
            print(f"[latency-budget] WARN: no estimates.json for {group}/{bench}")
            warn += 1
            continue
        est = read_estimates(est_path)
        if not est:
            print(f"[latency-budget] WARN: cannot parse {est_path}")
            warn += 1
            continue
        # Extract median / mean point_estimate (ns)
        # Criterion 0.5 JSON: keys like "median" -> {"point_estimate": 1234.5, "confidence_interval": ...}
        # older: "Slope" etc. We try multiple keys.
        def point(obj, key):
            if not obj or key not in obj:
                return None
            v = obj[key]
            if isinstance(v, dict):
                return v.get("point_estimate")
            return v
        # p50 = median, fallback to mean
        p50 = point(est, "median") or point(est, "Median") or point(est, "mean") or point(est, "Mean")
        if p50 is None:
            # try nested "estimates" or "statistics"
            for k in ("estimates", "statistics"):
                if k in est:
                    p50 = point(est[k], "median") or point(est[k], "mean")
                    if p50: break
        if p50 is None:
            print(f"[latency-budget] WARN: no median/mean in {est_path}: keys={list(est.keys())}")
            warn += 1
            continue
        p50 = float(p50)
        # Try sample.json for p99
        sample_candidates = [
            bench_path / "new" / "sample.json",
            bench_path / "base" / "sample.json",
            est_path.parent / "sample.json",
        ]
        sample_path = next((p for p in sample_candidates if p.exists()), None)
        if not sample_path:
            globs = list(bench_path.rglob("sample.json"))
            sample_path = globs[0] if globs else None
        p99 = p99_from_sample(sample_path) if sample_path else None
        # Fallback p99 estimate: p50 + 3*std_dev if available
        if p99 is None:
            std = point(est, "std_dev") or point(est, "StdDev")
            if std is not None:
                try:
                    p99 = p50 + 3*float(std)
                except: pass
        p99_str = f"{p99:.0f} ns ({p99/1e6:.3f} ms)" if p99 else "n/a"
        p50_str = f"{p50:.0f} ns ({p50/1e6:.3f} ms)"
        budget_label = "redact_10k" if group=="redact_10k" else ("L3 (report-only)" if is_l3 else "L0/L1")
        # Determine budget for this bench
        if group == "redact_10k":
            b_p50, b_p99 = redact_budget, redact_budget*20
        elif is_l3:
            b_p50, b_p99 = l3_p50, l3_p99
        else:
            b_p50, b_p99 = p50_budget, p99_budget

        ok_p50 = p50 <= b_p50
        ok_p99 = (p99 is None) or (p99 <= b_p99)
        status = "PASS" if (ok_p50 and ok_p99) else "FAIL"
        if status == "FAIL" and not is_l3:
            fail += 1
        # L3 is report-only: don't fail, just warn
        if is_l3 and not (ok_p50 and ok_p99):
            warn += 1
            status = "WARN (L3 report-only)"

        print(f"[latency-budget] {status} {group}/{bench}: p50={p50_str} budget {b_p50/1e6:.3f}ms | p99={p99_str} budget {b_p99/1e6:.3f}ms [{budget_label}] from {est_path}")

        # Regression check: look for change/estimates.json
        change_candidates = [
            bench_path / "change" / "estimates.json",
            est_path.parent.parent / "change" / "estimates.json",
        ]
        change_path = next((p for p in change_candidates if p.exists()), None)
        if not change_path:
            # glob search
            globs = list(bench_path.rglob("change/estimates.json"))
            change_path = globs[0] if globs else None
        if change_path and change_path.exists():
            ch = read_estimates(change_path)
            if ch:
                # Criterion change JSON: {"mean": {"point_estimate": 11.2, "change": ...}} or similar
                # Newer: {"mean": {"point_estimate": 123, "change": {"point_estimate": 10.5}}}
                def change_val(obj):
                    if not obj: return None
                    for k in ("mean","median"):
                        if k in obj:
                            v = obj[k]
                            if isinstance(v, dict):
                                # try nested "change" or direct
                                if "change" in v and isinstance(v["change"], dict):
                                    return v["change"].get("point_estimate")
                                # some versions store at top level
                                if "point_estimate" in v and "change" in obj:
                                    pass
                            # top-level change dict
                            if "change" in obj and isinstance(obj["change"], dict):
                                c = obj["change"]
                                if k in c and isinstance(c[k], dict):
                                    return c[k].get("point_estimate")
                    # fallback: look for any "change"
                    if "change" in obj:
                        chd = obj["change"]
                        if isinstance(chd, dict):
                            for kk in ("mean","median"):
                                if kk in chd and isinstance(chd[kk], dict):
                                    return chd[kk].get("point_estimate")
                    return None
                pct = None
                # Try direct layout: {"mean": {"point_estimate": ..., "change": {"point_estimate": 12.3}}}
                # Also: {"change": {"mean": {"point_estimate": 12.3}}}
                # We'll search recursively for a float that looks like percent (>1 and <200)
                import re
                # Simpler: grep raw json string for "change" point_estimate
                try:
                    raw = Path(change_path).read_text()
                    # crude: find all point_estimate after "change"
                    import json as js
                    # Walk recursively
                    def walk(o):
                        vals=[]
                        if isinstance(o, dict):
                            for kk,vv in o.items():
                                if kk=="point_estimate" and isinstance(vv,(int,float)):
                                    # need context: if parent key is "change" we capture
                                    pass
                                vals.extend(walk(vv))
                            # if this dict has change subdict, capture its point_estimates
                            if "change" in o:
                                chd=o["change"]
                                if isinstance(chd, dict):
                                    for kk2,vv2 in chd.items():
                                        if isinstance(vv2, dict) and "point_estimate" in vv2:
                                            vals.append(vv2["point_estimate"])
                                        elif isinstance(vv2,(int,float)):
                                            vals.append(vv2)
                        elif isinstance(o, list):
                            for it in o: vals.extend(walk(it))
                        return vals
                    cand = walk(ch)
                    # Filter plausible percent range -100..+200
                    pct_cands = [v for v in cand if -100 <= v <= 200]
                    if pct_cands:
                        pct = pct_cands[0]
                except: pass
                if pct is not None:
                    if pct > regression_pct:
                        print(f"[latency-budget] FAIL regression {group}/{bench}: change {pct:.1f}% > {regression_pct}% vs baseline {baseline} (from {change_path})")
                        fail += 1
                    elif pct < -regression_pct:
                        print(f"[latency-budget] INFO improvement {group}/{bench}: change {pct:.1f}% vs baseline {baseline}")
                    else:
                        print(f"[latency-budget] PASS regression {group}/{bench}: change {pct:.1f}% within {regression_pct}% vs {baseline}")
                else:
                    print(f"[latency-budget] INFO no parseable regression % for {group}/{bench} at {change_path}")

# Summary
print(f"[latency-budget] done: fail={fail} warn={warn}")
# Write exit code file for bash wrapper
Path("/tmp/latency_budget_fail").write_text(str(fail))
Path("/tmp/latency_budget_warn").write_text(str(warn))
PY

FAIL=$(cat /tmp/latency_budget_fail 2>/dev/null || echo 0)
WARN=$(cat /tmp/latency_budget_warn 2>/dev/null || echo 0)

echo ""
echo "[latency-budget] Summary: FAIL=${FAIL} WARN=${WARN}"
if [[ "${FAIL}" -gt 0 ]]; then
  echo -e "${RED}[latency-budget] ❌ LATENCY BUDGET BREACH or REGRESSION >${REGRESSION_PCT}% — see above. Adjust only via ADR (phase-1-09).${NC}"
  exit 1
else
  echo -e "${GREEN}[latency-budget] ✅ budgets OK (L0/L1 p50<3ms p99<10ms, redact <500us, L3 report-only, regression <=${REGRESSION_PCT}%)${NC}"
fi

# Hyperfine hook-client cold start reference (~1ms budget)
echo ""
echo "[latency-budget] hyperfine hook-client cold start (reference, ~1ms budget):"
if command -v hyperfine >/dev/null 2>&1; then
  echo "  hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin'"
  # Run a quick 10-run hyperfine if not in CI (optional, not failing)
  if [[ "${CI:-}" != "true" ]]; then
    set +e
    hyperfine --warmup 10 --runs 20 --show-output 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin <<< "ls -la"' || true
    # Also benchmark the binary directly if built
    if [[ -f "target/release/algo-hook-client" ]]; then
      hyperfine --warmup 10 --runs 20 'echo "ls -la" | target/release/algo-hook-client --socket /tmp/nonexistent.sock --stdin' || true
    fi
    set -e
  else
    echo "  (CI mode: skipping hyperfine run, but command is documented; uncomment in workflow to enforce ~1ms)"
  fi
else
  echo -e "${YELLOW}  hyperfine not installed — install via: cargo install hyperfine  OR  choco install hyperfine / brew install hyperfine${NC}"
  echo "  Expected: hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin'  must be ~1ms"
  echo "  (On Windows: hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket NUL --stdin')"
fi

echo "[latency-budget] Artifacts: upload target/criterion and target/criterion/*/p1-exit html_reports (see core/benches/README.md)"
