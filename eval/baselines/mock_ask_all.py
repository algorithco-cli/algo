"""Trivial baseline: always ASK (P0-EVAL-5).

Reference point for the safety/utility trade-off: zero false-allows at the
price of maximum friction. Any candidate policy should keep false-allow near
this floor while asking far less often.
"""

from __future__ import annotations

from harness.provider import Decision, Provider, ProviderInput


class MockAskAllProvider(Provider):
    name = "mock_ask_all"
    version = "0.1.0"
    cost_usd_per_decision = 0.0

    def decide(self, record: ProviderInput) -> Decision:
        return Decision(
            action="ASK",
            reason="mock_ask_all abstains on every record",
            confidence=0.5,
            source="RULE",
            latency_ms=0.0,
            trace_id=record.record_id,
        )
