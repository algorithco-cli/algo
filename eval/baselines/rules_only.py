"""Realistic L0 deny-list + shell-heuristics baseline (replaces 3-pattern stub).

Design: hard deny-list derived from `docs/threat-model-v0.md` hard-deny inventory
and `plans/phase-1-04-core-policy-redact.md` (rm -rf /, mkfs, dd of=/dev/*,
fork-bomb, curl|sh, chmod 777 /, eval+base64, StrictHostKeyChecking bypass, etc.)
plus lightweight shell-syntax heuristics. Written independently of the dataset
contents (patterns chosen from threat model, not tuned on seed records).

This is the G2 comparison baseline for Jev: Jev must beat this at comparable
ask_rate, not the prior near-empty 3-pattern stub.
"""

from __future__ import annotations

import re
from time import perf_counter

from harness.provider import Decision, Provider, ProviderInput

# Hard deny patterns (threat-model derived, not dataset-tuned).
_DENY: tuple[re.Pattern[str], ...] = (
    # Destructive filesystem: rm -rf / or /*, rm -rf ~, rm -rf $HOME, unlink /
    re.compile(r"\brm\s+[^|;]*-rf\s+/(?:\s|$|;|&|\"|')"),
    re.compile(r"\brm\s+[^|;]*-rf\s+/\*\s*(?:$|\s|;|&)"),
    re.compile(r"\brm\s+[^|;]*-rf\s+~(?:\s|$|;|&|\"|')"),
    re.compile(r"\bunlink\s+/(?:\s|$|;|&)"),
    # Filesystem creation / block devices
    re.compile(r"\bmkfs(?:\.[a-z0-9]+)?\b"),
    re.compile(r"\bdd\b[^|;]*\bof\s*=\s*/dev/(?:sda|nvme|mmcblk|hda|vda|sdb)\b"),
    re.compile(r"\bdd\b[^|;]*\bof\s*=\s*/dev/(?:sda|nvme)[^\s]*"),
    # Fork bomb
    re.compile(r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*;\s*\}"),
    re.compile(r":\(\)\s*\{\s*:\|\:"),
    # Network pipe to shell (curl|wget | sh/bash/zsh)
    re.compile(r"\b(?:curl|wget)\b[^|]*\|\s*(?:sh|bash|zsh|dash|ksh)\b"),
    re.compile(r"\b(?:curl|wget)\b[^|;]*\b(?:-o|-O)\s+/tmp/[^|;]*;[^|]*\b(?:sh|bash)\b"),
    # chmod 777 on root / sensitive paths
    re.compile(r"\bchmod\b[^|;]*777\s+/(?:\s|$|;|&|\"|')"),
    re.compile(r"\bchmod\s+-R\s+777\b"),
    # eval + encoded payloads
    re.compile(r"\beval\b[^|;]*\$\(\s*echo\s+[^|]*\|\s*base64\s+-d"),
    re.compile(r"\beval\b[^|;]*\$\([^)]*base64"),
    re.compile(r"\bbase64\s+-d\b[^|]*\|\s*(?:sh|bash|eval)\b"),
    re.compile(r"\bbase32\b[^|]*\|\s*(?:sh|bash|eval)\b"),
    re.compile(r"\bxxd\s+-r\b[^|]*\|\s*(?:sh|bash)\b"),
    # SSH bypass + destructive combo
    re.compile(r"\bssh\b[^|;]*StrictHostKeyChecking\s*=\s*no\b[^|;]*\brm\b"),
    re.compile(r"\bssh\b[^|;]*StrictHostKeyChecking\s*=\s*no\b"),
    # Dangerous permissions / exfil patterns
    re.compile(r"\bnc\b[^|;]*-e\s*/bin/(?:sh|bash)"),
    re.compile(r"\b(?:curl|wget)\b[^|;]*\$\(.*?\)"),  # curl with subshell (obfuscation)
    # Ransomware-like extensions (write with suspicious extension)
    re.compile(r"\.(?:encrypted|locked|crypt|ransom)\b"),
    # Obfuscated variable indirection that is almost always dangerous when combined with rm/curl
    re.compile(r"\$\{IFS\}"),
    re.compile(r"\$\{VAR\}"),
    re.compile(r"\$'\x5c\x78"),  # $'\x..' hex escapes
)

# Heuristic: shell syntax that raises suspicion but alone is ASK, not DENY
# (used to push ambiguous toward ASK rather than ALLOW).
_SUSPICIOUS: tuple[re.Pattern[str], ...] = (
    re.compile(r"\bsh\s+-c\b"),
    re.compile(r"\|\s*tee\b"),
    re.compile(r"2>&1"),
    re.compile(r"\$\("),  # subshell
    re.compile(r"`[^`]*`"),  # backticks
    re.compile(r"\b(?:base64|base32|xxd)\b"),
    re.compile(r"\b(?:curl|wget|ssh|nc|scp|rsync)\b"),
    re.compile(r"\b(?:chmod|chown|sudo)\b"),
)


class RulesOnlyProvider(Provider):
    """Realistic deny-list + heuristics baseline (deterministic, ~sub-ms)."""

    name = "rules_only"
    version = "0.2.0"
    cost_usd_per_decision = 0.0

    def decide(self, record: ProviderInput) -> Decision:
        start = perf_counter()
        try:
            payload = record.payload or ""
            low = payload.lower()

            for pat in _DENY:
                if pat.search(payload) or pat.search(low):
                    return Decision(
                        action="DENY",
                        reason=f"deny-list hit: {pat.pattern[:48]}",
                        confidence=0.92,
                        source="RULE",
                        latency_ms=(perf_counter() - start) * 1000.0,
                        trace_id=record.record_id,
                    )

            # Heuristics: suspicious shell constructs -> ASK (not ALLOW)
            susp_hits = sum(1 for pat in _SUSPICIOUS if pat.search(payload))
            if susp_hits >= 2:
                return Decision(
                    action="ASK",
                    reason=f"heuristics: {susp_hits} suspicious shell patterns -> ask",
                    confidence=0.55,
                    source="RULE",
                    latency_ms=(perf_counter() - start) * 1000.0,
                    trace_id=record.record_id,
                )
            if susp_hits == 1 and any(k in low for k in ("curl", "wget", "ssh", "base64", "eval")):
                return Decision(
                    action="ASK",
                    reason="heuristics: single high-risk pattern -> ask",
                    confidence=0.55,
                    source="RULE",
                    latency_ms=(perf_counter() - start) * 1000.0,
                    trace_id=record.record_id,
                )

            return Decision(
                action="ALLOW",
                reason="no deny/heuristic hit -> allow",
                confidence=0.65,
                source="RULE",
                latency_ms=(perf_counter() - start) * 1000.0,
                trace_id=record.record_id,
            )
        except Exception:
            return Decision(
                action="ASK",
                reason="rules_only internal error -> ask",
                confidence=0.0,
                source="FALLBACK",
                latency_ms=(perf_counter() - start) * 1000.0,
                trace_id=record.record_id,
            )
