# CI eval (P0-EVAL-5): artifact publish + gate plumbing

## Publish (per PR touching `eval/` or a pinned dataset tag)

```yaml
- name: Run eval baselines
  run: |
    python -m pip install -e "eval/.[dev]"
    python -m ruff check eval/
    python -m mypy  # runs from eval/
    python -m pytest eval/tests
    python eval/harness/cli.py --dataset eval/datasets/v0.1/seed.jsonl \
      --provider rules_only --output eval/reports/rules_only
    python eval/harness/cli.py --dataset eval/datasets/v0.1/seed.jsonl \
      --provider mock_ask_all --output eval/reports/mock_ask_all
- name: Upload eval reports
  uses: actions/upload-artifact@v4
  with:
    name: eval-reports-${{ github.sha }}
    path: eval/reports/
```

## Gate plumbing

A follow-up job parses `eval/reports/<provider>/report.json` and compares
against per-profile thresholds (final numbers at the Phase-0 exit gate,
`plans/phase-0-05-exit-gate.md`):

```powershell
# sketch: fail the build on false-allow regression
$report = Get-Content eval/reports/rules_only/report.json | ConvertFrom-Json
if ($report.metrics.false_allow_rate -gt $env:EVAL_MAX_FALSE_ALLOW) { exit 1 }
```

Rules:

- `report.json` must carry `dataset.{version,sha256}` + `provider.{name,version}`
  or the gate rejects the artifact as unpinned.
- `rules_only` is the floor: a candidate policy must beat its ambiguous-slice
  accuracy at equal-or-lower false-allow, else the PR is blocked.
- Headroom check: if `rules_only` ever stops being worse on the ambiguous
  slice, the dataset (not the policy) needs harder records — file a
  `P0-EVAL-3` issue instead of weakening the gate.
