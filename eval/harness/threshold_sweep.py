"""Threshold-sweep stub (P0-EVAL-4).

Full confidence-threshold sweeping (per-profile allow/ask/deny cutoffs with
false-allow at fixed false-ask) lands with P1-QUAL. This stub exists so the
harness CLI and CI plumbing can reference a stable entrypoint now.

TODO(P1-QUAL): grid-search thresholds on the harness JSON, plot
false-allow vs false-ask, and pin per-profile cutoffs for the exit gate.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

from .metrics import EvalMetrics  # noqa: F401  (re-exported for the future sweep API)
from .runner import RunResult


def sweep_thresholds(
    results: Sequence[RunResult],
    thresholds: Sequence[float] | None = None,
) -> list[dict[str, Any]]:
    """Placeholder sweep; returns one row per threshold with no filtering yet."""
    grid = list(thresholds) if thresholds else [0.5, 0.7, 0.9]
    return [
        {
            "threshold": t,
            "note": "stub: full sweep in P1-QUAL",
            "n": len(results),
        }
        for t in grid
    ]
