"""Provider interface for the Phase-0 eval harness (P0-EVAL-4).

Product-agnostic: a provider maps one redacted tool-use summary to a
Decision with action / reason / confidence / source / latency, mirroring
proto ``decision.proto``. No product core or agent code lives here.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Literal

Action = Literal["ALLOW", "DENY", "ASK"]
SourceLevel = Literal["RULE", "CACHE", "LOCAL_MODEL", "JEV", "FALLBACK"]

ALLOWED_ACTIONS: tuple[str, ...] = ("ALLOW", "DENY", "ASK")


@dataclass(frozen=True)
class ProviderInput:
    """Redacted per-record view handed to a provider (labels stripped)."""

    record_id: str
    tool_kind: str
    payload: str
    obfuscation: str = "NONE"


@dataclass(frozen=True)
class Decision:
    """One allow/deny/ask judgment (mirrors ``Decision`` in decision.proto)."""

    action: Action
    reason: str  # one line, no secrets
    confidence: float  # 0..1
    source: SourceLevel
    latency_ms: float
    policy_version: str = "eval-v0.1"
    trace_id: str = ""

    def __post_init__(self) -> None:
        if self.action not in ALLOWED_ACTIONS:
            raise ValueError(f"invalid action: {self.action!r}")
        if not 0.0 <= self.confidence <= 1.0:
            raise ValueError("confidence must be in [0, 1]")
        if "\n" in self.reason:
            raise ValueError("reason must be a single line")


class Provider(ABC):
    """Abstract judgment provider evaluated by the harness."""

    name: str = "base"
    version: str = "0.0.0"
    cost_usd_per_decision: float = 0.0

    @abstractmethod
    def decide(self, record: ProviderInput) -> Decision:
        """Return a Decision for one redacted record."""
        raise NotImplementedError
