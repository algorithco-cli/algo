"""Eval runner with fail-safe semantics (P0-EVAL-4).

Rules:
- Per-record timeout resolves to ``ASK`` (never ``ALLOW``).
- Any provider exception resolves to ``ASK`` (never ``ALLOW``).
- Retries are bounded by a global retry budget shared across the run.
"""

from __future__ import annotations

from collections.abc import Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from time import perf_counter

from .provider import Decision, Provider, ProviderInput


@dataclass(frozen=True)
class EvalItem:
    """One labeled record prepared for a provider (label hidden from provider)."""

    record_id: str
    expected: str  # SAFE | DANGEROUS | AMBIGUOUS
    tool_kind: str
    payload: str
    obfuscation: str = "NONE"

    def to_input(self) -> ProviderInput:
        return ProviderInput(
            record_id=self.record_id,
            tool_kind=self.tool_kind,
            payload=self.payload,
            obfuscation=self.obfuscation,
        )


@dataclass(frozen=True)
class RunResult:
    record_id: str
    expected: str
    decision: Decision
    timed_out: bool = False
    attempts: int = 1
    error: str = ""


def _fallback_ask(record_id: str, reason: str, latency_ms: float) -> Decision:
    return Decision(
        action="ASK",
        reason=reason,
        confidence=0.0,
        source="FALLBACK",
        latency_ms=latency_ms,
        trace_id=record_id,
    )


def _run_once(provider: Provider, item: EvalItem, timeout_s: float) -> tuple[Decision, bool, str]:
    """Run one attempt. Returns (decision, timed_out, error)."""
    start = perf_counter()
    pool = ThreadPoolExecutor(max_workers=1)
    try:
        future = pool.submit(provider.decide, item.to_input())
        try:
            decision = future.result(timeout=timeout_s)
        except Exception as exc:  # noqa: BLE001 - fail-safe must catch everything
            latency_ms = (perf_counter() - start) * 1000.0
            timed_out = isinstance(exc, TimeoutError)
            reason = (
                f"timeout after {timeout_s}s -> ask"
                if timed_out
                else f"provider error -> ask: {type(exc).__name__}"
            )
            error = f"{type(exc).__name__}: {exc}"
            return _fallback_ask(item.record_id, reason, latency_ms), timed_out, error
        latency_ms = (perf_counter() - start) * 1000.0
    finally:
        # Never block the run on a timed-out worker; it dies on its own.
        pool.shutdown(wait=False, cancel_futures=True)
    if decision.latency_ms == 0.0:
        # Provider did not self-report; use wall-clock as the observed latency.
        decision = Decision(
            action=decision.action,
            reason=decision.reason,
            confidence=decision.confidence,
            source=decision.source,
            latency_ms=latency_ms,
            policy_version=decision.policy_version,
            trace_id=decision.trace_id or item.record_id,
        )
    return decision, False, ""


def run_eval(
    provider: Provider,
    items: Sequence[EvalItem],
    timeout_s: float = 5.0,
    max_retries: int = 1,
    retry_budget: int = 10,
) -> list[RunResult]:
    """Evaluate every item, enforcing timeout->ask and a bounded retry budget."""
    results: list[RunResult] = []
    budget_left = retry_budget
    for item in items:
        attempts = 0
        while True:
            attempts += 1
            decision, timed_out, error = _run_once(provider, item, timeout_s)
            retryable = timed_out or error != ""
            if retryable and attempts <= max_retries and budget_left > 0:
                budget_left -= 1
                continue
            results.append(
                RunResult(
                    record_id=item.record_id,
                    expected=item.expected,
                    decision=decision,
                    timed_out=timed_out,
                    attempts=attempts,
                    error=error,
                )
            )
            break
    return results
