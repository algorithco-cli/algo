# AGENTS.md — algorithco guard eval skeleton (Phase 0, `P0-EVAL-*`)

Python-only scaffold. No product core/agent code lives under `eval/`.
Proto source of truth for record shape: `proto/dataset.proto`
(`DatasetRecord` — this repo mirrors it in `datasets/v0.1/schema.json`).

## Setup

```powershell
python -m pip install -e ".[dev]"
```

## Run

```powershell
# lint + types + tests (all must be green)
python -m ruff check .
python -m ruff format --check .
python -m mypy
python -m pytest

# harness on mocks (30-record seed slice)
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider rules_only --output reports/rules_only
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/mock_ask_all

# dataset coverage stats
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/coverage --coverage-only
```

## Layout

- `labeling-guide.md` — SAFE/DANGEROUS/AMBIGUOUS + obfuscation appendix + kappa pilot.
- `datasets/v0.1/` — `schema.json`, `seed.jsonl` (30 seeds), `DATASET.md`.
- `harness/` — `provider.py`, `runner.py`, `metrics.py`, `report.py`,
  `threshold_sweep.py` (stub), `agreement.py` (Cohen kappa), `cli.py`.
- `baselines/` — `rules_only.py` (deliberately weak), `mock_ask_all.py`.
- `questions/` — versioned judgment phrasings, joint with Jev work.
- `ci-eval.md` — CI artifact + gate plumbing.
- `tests/` — secret scan, fail-safe timeout->ask, metrics/report smoke tests.

## Rules

- Redact before label: no real secrets or user code in datasets/fixtures without
  written consent. `tests/test_no_secrets.py` enforces this in CI.
- Fail-safe: every timeout/error/parse path must resolve to `ask`, with a
  `proves_ask_on_*` test (see `tests/test_runner_failsafe.py`).
- Keep `ruff` + `mypy` clean; small PRs; one concern per change.
