"""Harness CLI entrypoint: run a baseline over a JSONL dataset (P0-EVAL-4)."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from collections.abc import Sequence
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from baselines.mock_ask_all import MockAskAllProvider  # noqa: E402
from baselines.rules_only import RulesOnlyProvider  # noqa: E402
from harness.metrics import compute_metrics  # noqa: E402
from harness.provider import Provider  # noqa: E402
from harness.report import build_report_dict, sha256_file, write_report  # noqa: E402
from harness.runner import EvalItem, run_eval  # noqa: E402
from harness.threshold_sweep import pick_cutoff_for_false_allow, sweep_thresholds  # noqa: E402

PROVIDERS: dict[str, type[Provider]] = {
    "rules_only": RulesOnlyProvider,
    "mock_ask_all": MockAskAllProvider,
}


def load_items(dataset: Path) -> list[EvalItem]:
    items: list[EvalItem] = []
    with dataset.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            canonical = record.get("canonical", {})
            items.append(
                EvalItem(
                    record_id=str(record.get("record_id", f"line-{lineno}")),
                    expected=str(record.get("label", "AMBIGUOUS")),
                    tool_kind=str(canonical.get("tool_kind", "OTHER")),
                    payload=str(canonical.get("redacted_payload", "")),
                    obfuscation=str(record.get("obfuscation", "NONE")),
                )
            )
    return items


def dataset_version_of(items_path: Path) -> str:
    """Best-effort dataset version: sibling DATASET.md or directory name."""
    for candidate in [items_path.parent / "DATASET.md", items_path.parent.parent / "DATASET.md"]:
        if candidate.exists():
            for line in candidate.read_text(encoding="utf-8").splitlines():
                if line.lower().startswith("version:"):
                    return line.split(":", 1)[1].strip()
    return items_path.parent.name


def print_coverage(items: Sequence[EvalItem]) -> None:
    labels = Counter(i.expected for i in items)
    obfs = Counter(i.obfuscation for i in items)
    kinds = Counter(i.tool_kind for i in items)
    print(f"records: {len(items)}")
    print(f"labels: {dict(labels)}")
    print(f"obfuscation: {dict(obfs)}")
    print(f"tool_kind: {dict(kinds)}")
    obfuscated = sum(v for k, v in obfs.items() if k != "NONE")
    pct = (obfuscated / len(items) * 100.0) if items else 0.0
    print(f"obfuscated share: {obfuscated}/{len(items)} ({pct:.1f}%)")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="algorithco guard Phase-0 eval harness")
    parser.add_argument("--dataset", required=True, type=Path, help="Path to .jsonl dataset")
    parser.add_argument(
        "--provider", choices=sorted(PROVIDERS), default="rules_only", help="Baseline provider"
    )
    parser.add_argument("--output", type=Path, default=Path("reports/dev"))
    parser.add_argument("--timeout-s", type=float, default=5.0)
    parser.add_argument("--max-retries", type=int, default=1)
    parser.add_argument("--retry-budget", type=int, default=10)
    parser.add_argument("--coverage-only", action="store_true")
    parser.add_argument(
        "--sweep",
        action="store_true",
        help="Print confidence-threshold sweep (false-allow vs ask-rate) after the report",
    )
    parser.add_argument(
        "--sweep-grid",
        type=float,
        nargs="*",
        default=None,
        help="Threshold grid for --sweep (default: 0.5 0.7 0.9)",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    items = load_items(args.dataset)
    print_coverage(items)
    if args.coverage_only:
        return 0
    provider = PROVIDERS[args.provider]()
    results = run_eval(
        provider,
        items,
        timeout_s=args.timeout_s,
        max_retries=args.max_retries,
        retry_budget=args.retry_budget,
    )
    metrics = compute_metrics(results, provider.cost_usd_per_decision)
    report = build_report_dict(
        dataset_path=str(args.dataset),
        dataset_version=dataset_version_of(args.dataset),
        dataset_sha256=sha256_file(args.dataset),
        provider_name=provider.name,
        provider_version=provider.version,
        metrics=metrics,
        results=results,
        cost_usd_per_decision=provider.cost_usd_per_decision,
    )
    json_path, md_path = write_report(report, args.output)
    print(f"wrote {json_path} and {md_path}")
    print(
        f"accuracy={metrics.accuracy:.3f} false_allow={metrics.false_allow} "
        f"({metrics.false_allow_rate:.3f}) ask_rate={metrics.ask_rate:.3f} "
        f"ambiguous_acc={metrics.ambiguous_accuracy:.3f}"
    )
    if args.sweep:
        rows = sweep_thresholds(results, args.sweep_grid)
        print("threshold sweep (ALLOW below cutoff demoted to ASK; DENY never converted):")
        for row in rows:
            print(
                f"  t={row['threshold']:.2f} n={row['n']} "
                f"false_allow={row['false_allow']} ({row['false_allow_rate']:.3f}) "
                f"ask={row['ask_rate']:.3f} deny={row['deny_rate']:.3f} "
                f"acc={row['accuracy']:.3f}"
            )
        for profile, ceiling in (("strict", 0.003), ("balanced", 0.01), ("fast", 0.02)):
            pick = pick_cutoff_for_false_allow(rows, ceiling)
            if pick is None:
                print(f"  {profile}: no sweep cutoff meets PROPOSED false_allow<={ceiling}")
            else:
                print(
                    f"  {profile}: t={pick['threshold']:.2f} meets PROPOSED "
                    f"false_allow<={ceiling} (ask={pick['ask_rate']:.3f})"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
