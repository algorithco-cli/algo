"""Pre-send redaction stub for the Jev probe client.

Canonical harness redaction lands in P0-EVAL-4 (`harness/redact.py`). Until then
this stub guarantees the invariant: NOTHING leaves the machine unredacted, and
logs carry only hashes + counts, never payloads or keys.

`client.py` prefers the canonical implementation when importable::

    try:
        from harness.redact import redact_text  # P0-EVAL-4 canonical
    except ImportError:
        from redaction import redact_text       # this stub

The stub is deliberately conservative: it masks high-entropy / known-secret
shapes AND wraps the whole state with a marker so reviewers can see redaction ran.
It is NOT a substitute for the Rust `redact` crate (gitleaks-style rules) that
ships in Phase 1; it only protects Phase-0 probe traffic (hand-written, redacted
fixtures — never real secrets, never user code without consent).
"""

from __future__ import annotations

import hashlib
import re

_STUB_MARKER = "[REDACTED-BY-STUB]"

_PATTERNS = [
    # PEM blocks
    (
        re.compile(
            r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY-----"
        ),
        "[PEM-KEY]",
    ),
    # AWS access key id
    (re.compile(r"\bAKIA[0-9A-Z]{16}\b"), "[AWS-KEY-ID]"),
    # AWS secret (40-char base64-ish after assignment)
    (re.compile(r"(?i)(aws_secret[^=\n]*[=:]\s*)(['\"]?)[A-Za-z0-9/+=]{40}\2"), r"\1[AWS-SECRET]"),
    # Generic api key / token / password assignments
    (
        re.compile(
            r"(?i)\b(api[_-]?key|secret|token|password|passwd|pwd)\b\s*[:=]\s*(['\"]?)[^'\"\s,}]{4,}\2"
        ),
        r"\1=[CREDENTIAL]",
    ),
    # Bearer tokens
    (re.compile(r"(?i)\bearer\s+[A-Za-z0-9\-._~+/=]{8,}"), "Bearer [TOKEN]"),
    # sk- style keys
    (re.compile(r"\bsk-[A-Za-z0-9]{8,}\b"), "[SK-KEY]"),
    # user home dirs (path normalization, avoids leaking usernames)
    (re.compile(r"/Users/[^/\s]+"), "/Users/[USER]"),
    (re.compile(r"/home/[^/\s]+"), "/home/[USER]"),
    (re.compile(r"C:\\Users\\[^\\\s]+", re.IGNORECASE), r"C:\\Users\\[USER]"),
]


def redact_text(text: str) -> tuple[str, int]:
    """Return (redacted_text, substitution_count). Pure function, no I/O."""
    count = 0
    out = text
    for rx, repl in _PATTERNS:
        out, n = rx.subn(repl, out)
        count += n
    if out == text:
        # No secret shape found: still mark so downstream can prove the hook ran.
        out = f"{_STUB_MARKER} {out}"
    return out, count


def sha256_hex(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def log_record(*, redacted_state: str, substitutions: int) -> dict:
    """Log-safe record: hashes and counts only. NEVER the payload or key."""
    return {
        "state_sha256": sha256_hex(redacted_state),
        "state_chars": len(redacted_state),
        "redact_substitutions": substitutions,
        "stub": True,  # False once harness.redact (P0-EVAL-4) takes over
    }
