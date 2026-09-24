# algorithco-guard-eval

Phase-0 eval dataset + harness for algorithco guard (CLI `algo`). No product code.

Measures false-allow / ask-rate on `datasets/v0.1` with `rules_only` and
`mock_ask_all` baselines. Dataset schema mirrors `proto/dataset.proto`
(`DatasetRecord`).

## Install

```powershell
python -m pip install -e ".[dev]"
# optional Jev probe client (throwaway, Phase 0 only)
python -m pip install -e ".[jev]"
```

## Run

```powershell
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider rules_only --output reports/rules_only
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/mock_ask_all
# or via console script
eval-harness --dataset datasets/v0.1/seed.jsonl --provider rules_only --output reports/rules_only
```

See `AGENTS.md` for labeling rules, fail-safe (`ask`) policy, and CI gates.
`jev_client/` is an explicit non-distributed throwaway probe (see its README).
