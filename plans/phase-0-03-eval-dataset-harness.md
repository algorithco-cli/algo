# P0-03 Eval Dataset + Harness

Stack: Python, pandas, scikit-learn/PyTorch as needed, ruff/mypy/pytest.

## 1. Skeleton (`P0-EVAL-1`)

`pyproject.toml`, ruff/mypy/pytest, `AGENTS.md`, `tests/test_no_secrets.py` (AWS/PEM/sk-/entropy) + redaction tests.
Acceptance: linters green; planted-secret fixture fails scan, clean tree passes.

## 2. Label taxonomy + guide (`P0-EVAL-2`)

`labeling-guide.md`: SAFE / DANGEROUS / AMBIGUOUS + obfuscation appendix + double-label + adjudication + redact-before-label (no real secrets/user code without consent).
Acceptance: 20-record pilot double-labeled, Cohen's κ ≥0.7 or guide revised.

## 3. Collection 200–300 (`P0-EVAL-3`)

`datasets/v0.1/*.jsonl` schema-validated in CI; ~40/30/30 safe/dangerous/ambiguous; ≥20% obfuscated; shell-heavy + small edit/write sample; `DATASET.md` (source, version, redaction cert, license).
Acceptance: 200–300 valid, distribution published, secret-scan green, tag `eval-data-v0.1`. **Risk:** High — quality determines gate.

## 4. Harness v0 (`P0-EVAL-4`)

`harness/{provider.py (ABC→Decision), runner.py (timeout→ask, retry budget), metrics.py (false-allow/ask/deny, confusion, ECE/Brier, p50/p95/p99, cost/1k), report.py (md+json with dataset+provider+SHA)}` + threshold-sweep stub.
Acceptance: runs on mocks; JSON has all metrics + versions; timeout-injection proves →`ask`.

## 5. Baselines + CI (`P0-EVAL-5`)

`baselines/rules_only.py` (deliberately weak) + `mock_ask_all.py`; CI publishes report artifact; gate plumbing parses report.
Acceptance: weak baseline worse on ambiguous slice (proves headroom); artifact downloadable.

## 6. Question variants (`P0-EVAL-6`, joint with P0-JEV)

`questions/` 3–5 phrasings per judgment (Boolean/Choice/Score) + batching test, versioned prompts; A/B on same slice.
Acceptance: Δfalse-allow at fixed false-ask reported; winner pinned `questions-v0.1`.
