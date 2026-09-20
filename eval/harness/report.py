"""Report builder: versioned md+json artifacts with dataset+provider+SHA (P0-EVAL-4)."""

from __future__ import annotations

import hashlib
import json
import subprocess
from collections.abc import Sequence
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .metrics import EvalMetrics
from .runner import RunResult


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_sha(default: str = "unknown") -> str:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        )
    except Exception:  # noqa: BLE001 - best-effort provenance only
        return default
    return out.stdout.strip() or default


def build_report_dict(
    *,
    dataset_path: str,
    dataset_version: str,
    dataset_sha256: str,
    provider_name: str,
    provider_version: str,
    metrics: EvalMetrics,
    results: Sequence[RunResult],
    cost_usd_per_decision: float = 0.0,
) -> dict[str, Any]:
    return {
        "eval": "algorithco-guard-eval-v0",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "git_sha": git_sha(),
        "dataset": {
            "path": dataset_path,
            "version": dataset_version,
            "sha256": dataset_sha256,
        },
        "provider": {
            "name": provider_name,
            "version": provider_version,
            "cost_usd_per_decision": cost_usd_per_decision,
        },
        "metrics": metrics.to_dict(),
        "results": [
            {
                "record_id": r.record_id,
                "expected": r.expected,
                "predicted": r.decision.action,
                "confidence": r.decision.confidence,
                "source": r.decision.source,
                "latency_ms": r.decision.latency_ms,
                "reason": r.decision.reason,
                "timed_out": r.timed_out,
                "attempts": r.attempts,
                "error": r.error,
            }
            for r in results
        ],
    }


def render_markdown(report: dict[str, Any]) -> str:
    metrics = report["metrics"]
    confusion = metrics.get("confusion", {})
    lines = [
        "# Eval report",
        "",
        f"- Generated: `{report['generated_at']}` (git `{report['git_sha']}`)",
        f"- Dataset: `{report['dataset']['path']}` "
        f"version `{report['dataset']['version']}` "
        f"sha256 `{report['dataset']['sha256'][:12]}…`",
        f"- Provider: `{report['provider']['name']}` v`{report['provider']['version']}`",
        "",
        "## Metrics",
        "",
        f"- n = {metrics['total']} (counts {json.dumps(metrics['label_counts'])})",
        f"- accuracy = {metrics['accuracy']:.3f}",
        f"- false_allow = {metrics['false_allow']} "
        f"(rate {metrics['false_allow_rate']:.3f} over DANGEROUS)",
        f"- allow/ask/deny rates = {metrics['allow_rate']:.3f} / "
        f"{metrics['ask_rate']:.3f} / {metrics['deny_rate']:.3f}",
        f"- ambiguous slice: {metrics['ambiguous_correct']}/{metrics['ambiguous_total']} "
        f"({metrics['ambiguous_accuracy']:.3f})",
        f"- brier = {metrics['brier']:.3f}, ece = {metrics['ece']:.3f}",
        f"- latency p50/p95/p99 = {metrics['latency_p50_ms']:.1f} / "
        f"{metrics['latency_p95_ms']:.1f} / {metrics['latency_p99_ms']:.1f} ms",
        f"- cost/1k = ${metrics['cost_usd_per_1k']:.4f}",
        "",
        "## Confusion (actual -> predicted)",
        "",
        "```json",
        json.dumps(confusion, indent=2),
        "```",
        "",
    ]
    return "\n".join(lines)


def write_report(report: dict[str, Any], output_dir: Path) -> tuple[Path, Path]:
    """Write report.json + report.md. Returns both paths."""
    output_dir.mkdir(parents=True, exist_ok=True)
    json_path = output_dir / "report.json"
    md_path = output_dir / "report.md"
    json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(report), encoding="utf-8")
    return json_path, md_path
