"""Threshold-sweep tests: fail-safe demotion, monotonic safety, cutoffs."""

from __future__ import annotations

from harness.provider import Decision
from harness.runner import RunResult
from harness.threshold_sweep import (
    apply_threshold,
    pick_cutoff_for_false_allow,
    sweep_thresholds,
)


def _res(record_id: str, expected: str, action: str, confidence: float) -> RunResult:
    return RunResult(
        record_id=record_id,
        expected=expected,
        decision=Decision(
            action=action,  # type: ignore[arg-type]
            reason="test",
            confidence=confidence,
            source="RULE",
            latency_ms=1.0,
            trace_id=record_id,
        ),
    )


def test_low_confidence_allow_demotes_to_ask() -> None:
    rows = [_res("r1", "SAFE", "ALLOW", 0.4)]
    out = apply_threshold(rows, 0.5)
    assert out[0].decision.action == "ASK"
    assert "below threshold" in out[0].decision.reason


def test_high_confidence_allow_kept() -> None:
    rows = [_res("r1", "SAFE", "ALLOW", 0.95)]
    out = apply_threshold(rows, 0.5)
    assert out[0].decision.action == "ALLOW"


def test_deny_never_converted_proves_fail_safe() -> None:
    rows = [_res("r1", "DANGEROUS", "DENY", 0.01)]
    out = apply_threshold(rows, 0.99)
    assert out[0].decision.action == "DENY"


def test_ask_untouched() -> None:
    rows = [_res("r1", "AMBIGUOUS", "ASK", 0.0)]
    out = apply_threshold(rows, 0.9)
    assert out[0].decision.action == "ASK"


def test_sweep_false_allow_monotonic_nonincreasing() -> None:
    rows = [
        _res("d1", "DANGEROUS", "ALLOW", 0.6),
        _res("d2", "DANGEROUS", "ALLOW", 0.85),
        _res("s1", "SAFE", "ALLOW", 0.9),
        _res("a1", "AMBIGUOUS", "ASK", 0.5),
    ]
    swept = sweep_thresholds(rows, [0.0, 0.5, 0.7, 0.95])
    rates = [r["false_allow_rate"] for r in swept]
    assert rates == sorted(rates, reverse=True)
    assert swept[0]["false_allow"] == 2
    assert swept[-1]["false_allow"] == 0
    assert swept[-1]["ask_rate"] >= swept[0]["ask_rate"]


def test_sweep_empty_results_never_raises() -> None:
    swept = sweep_thresholds([], [0.5])
    assert swept[0]["n"] == 0
    assert swept[0]["false_allow"] == 0


def test_pick_cutoff_prefers_lowest_qualifying() -> None:
    rows = [
        {"threshold": 0.5, "false_allow_rate": 0.05, "ask_rate": 0.1},
        {"threshold": 0.7, "false_allow_rate": 0.01, "ask_rate": 0.2},
        {"threshold": 0.9, "false_allow_rate": 0.0, "ask_rate": 0.4},
    ]
    pick = pick_cutoff_for_false_allow(rows, 0.01)
    assert pick is not None
    assert pick["threshold"] == 0.7


def test_pick_cutoff_none_when_no_row_qualifies() -> None:
    rows = [{"threshold": 0.5, "false_allow_rate": 0.5, "ask_rate": 0.0}]
    assert pick_cutoff_for_false_allow(rows, 0.01) is None
