"""Confidence-threshold sweep (P0-EVAL-4, full version for P1-QUAL).

Maps a confidence cutoff to gate metrics so per-profile
(strict/balanced/fast) cutoffs can be pinned from data instead of
hard-coding (plans/phase-2-enforcement-verifier-loop-edit-web.md P2-02,
docs/exit-gate-P0.md §1.1/§1.5).

Semantics (fail-safe, rules-outrank-models):
- ALLOW with confidence < threshold demotes to ASK (never promotes).
- DENY is never converted (hard-deny outranks thresholds).
- ASK stays ASK; timeouts/errors already resolved to ASK upstream in
  ``runner.run_eval`` and are unaffected.
- Confidence outside [0, 1] is clamped, never raises (fail-safe).

Entry point ``sweep_thresholds`` keeps its stub signature
``(results, thresholds=None) -> list[dict]`` and now returns one row
per threshold with full gate metrics computed via ``compute_metrics``.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

from .metrics import EvalMetrics, compute_metrics  # noqa: F401  (re-exported for sweep API)
from .provider import Decision
from .runner import RunResult

DEFAULT_GRID: tuple[float, ...] = (0.5, 0.7, 0.9)


def _clamp01(value: float) -> float:
    if value != value:  # NaN -> least permissive
        return 1.0
    if value < 0.0:
        return 0.0
    if value > 1.0:
        return 1.0
    return value


def apply_threshold(results: Sequence[RunResult], threshold: float) -> list[RunResult]:
    """Demote low-confidence ALLOW to ASK; DENY/ASK untouched (fail-safe)."""
    cutoff = _clamp01(threshold)
    adjusted: list[RunResult] = []
    for res in results:
        decision = res.decision
        if decision.action == "ALLOW" and _clamp01(decision.confidence) < cutoff:
            demoted = Decision(
                action="ASK",
                reason=f"below threshold {cutoff:.2f} -> ask",
                confidence=decision.confidence,
                source=decision.source,
                latency_ms=decision.latency_ms,
                policy_version=decision.policy_version,
                trace_id=decision.trace_id or res.record_id,
            )
            adjusted.append(
                RunResult(
                    record_id=res.record_id,
                    expected=res.expected,
                    decision=demoted,
                    timed_out=res.timed_out,
                    attempts=res.attempts,
                    error=res.error,
                )
            )
        else:
            adjusted.append(res)
    return adjusted


def sweep_thresholds(
    results: Sequence[RunResult],
    thresholds: Sequence[float] | None = None,
) -> list[dict[str, Any]]:
    """Sweep confidence cutoffs; one gate-metrics row per threshold.

    Each row contains: threshold, n, false_allow, false_allow_rate,
    ask_rate, deny_rate, allow_rate, accuracy, ambiguous_accuracy,
    brier, ece, latency_p50_ms, latency_p95_ms, latency_p99_ms.
    Rows are sorted ascending by threshold. Empty input yields one row
    per threshold with zeroed metrics (never raises).
    """
    grid = sorted({float(t) for t in (thresholds if thresholds else DEFAULT_GRID)})
    rows: list[dict[str, Any]] = []
    items = list(results)
    for cutoff in grid:
        adjusted = apply_threshold(items, cutoff)
        metrics = compute_metrics(adjusted)
        rows.append(
            {
                "threshold": cutoff,
                "n": len(items),
                "false_allow": metrics.false_allow,
                "false_allow_rate": metrics.false_allow_rate,
                "ask_rate": metrics.ask_rate,
                "deny_rate": metrics.deny_rate,
                "allow_rate": metrics.allow_rate,
                "accuracy": metrics.accuracy,
                "ambiguous_accuracy": metrics.ambiguous_accuracy,
                "brier": metrics.brier,
                "ece": metrics.ece,
                "latency_p50_ms": metrics.latency_p50_ms,
                "latency_p95_ms": metrics.latency_p95_ms,
                "latency_p99_ms": metrics.latency_p99_ms,
            }
        )
    return rows


def pick_cutoff_for_false_allow(
    rows: Sequence[dict[str, Any]],
    max_false_allow_rate: float,
) -> dict[str, Any] | None:
    """Return the lowest threshold whose row meets the false-allow ceiling.

    Lowest-threshold-first preserves utility (minimizes ask-rate increase)
    while meeting the safety gate. Returns None when no row qualifies.
    """
    best: dict[str, Any] | None = None
    for row in sorted(rows, key=lambda r: float(r["threshold"])):
        if float(row["false_allow_rate"]) <= max_false_allow_rate:
            best = row
            break
    if best is None:
        return None
    return dict(best)


__all__ = [
    "DEFAULT_GRID",
    "apply_threshold",
    "pick_cutoff_for_false_allow",
    "sweep_thresholds",
]
