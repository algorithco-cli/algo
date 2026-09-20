"""No-secrets enforcement for the eval tree (P0-EVAL-1).

Scans code, docs, fixtures, and datasets for high-signal secret shapes
(AWS key IDs, PEM blocks, vendor secret-key prefixes, high-entropy tokens)
and asserts every dataset record carries a ``redaction_cert``.

Acceptance (per plan):
- a planted-secret fixture FAILS the scan,
- the clean tree PASSES.

Planted secrets below are assembled from fragments at runtime so this file
itself never contains a scannable literal.
"""

from __future__ import annotations

import json
import math
import re
from collections import Counter
from pathlib import Path

EVAL_ROOT = Path(__file__).resolve().parent.parent

# High-signal patterns only: specific prefixes/structures, not generic words
# like "token" or "password", so redacted fixtures using [REDACTED_*] pass.
PATTERNS: dict[str, re.Pattern[str]] = {
    "aws_access_key": re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    "aws_secret_key": re.compile(
        r"\baws_secret_access_key\b\s*[:=]\s*['\"]?[A-Za-z0-9/+=]{40}['\"]?"
    ),
    "pem_block": re.compile(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----"),
    "vendor_sk": re.compile(r"\bsk-(live|test)-[A-Za-z0-9]{16,}\b"),
    "github_pat": re.compile(r"\bghp_[A-Za-z0-9]{20,}\b"),
    "slack_token": re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
}

SCAN_SUFFIXES = {".py", ".json", ".jsonl", ".yaml", ".yml", ".md", ".toml", ".txt"}
SKIP_DIRS = {".git", "__pycache__", ".mypy_cache", ".ruff_cache", "reports", ".venv", "venv"}


def shannon_entropy(text: str) -> float:
    if not text:
        return 0.0
    counts = Counter(text)
    length = len(text)
    return -sum((n / length) * math.log2(n / length) for n in counts.values())


def _char_classes(token: str) -> int:
    classes = 0
    if re.search(r"[A-Z]", token):
        classes += 1
    if re.search(r"[a-z]", token):
        classes += 1
    if re.search(r"[0-9]", token):
        classes += 1
    if re.search(r"[^A-Za-z0-9]", token):
        classes += 1
    return classes


def high_entropy_tokens(text: str) -> list[str]:
    """Long dense mixed-class tokens (len>=24, entropy>4.5) typical of leaks.

    The 3-of-4 character-class requirement keeps URLs, paths, and prose out
    while catching credential-shaped strings (upper+lower+digit+symbol).
    """
    hits: list[str] = []
    for token in re.findall(r"[A-Za-z0-9][A-Za-z0-9+/=_.\-#@!$%^&*?]{23,}", text):
        stripped = token.strip("=_.-")
        if len(stripped) >= 24 and _char_classes(stripped) >= 3 and shannon_entropy(stripped) > 4.5:
            hits.append(token)
    return hits


def scan_text(text: str) -> list[str]:
    findings: list[str] = []
    for name, pattern in PATTERNS.items():
        if pattern.search(text):
            findings.append(name)
    if high_entropy_tokens(text):
        findings.append("high_entropy_token")
    return findings


def scan_tree(root: Path = EVAL_ROOT) -> dict[str, list[str]]:
    """Map relative path -> findings for every scannable file under root."""
    results: dict[str, list[str]] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        if path.suffix.lower() not in SCAN_SUFFIXES:
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        findings = scan_text(text)
        if findings:
            results[str(path.relative_to(root))] = findings
    return results


def _planted_aws_key() -> str:
    return "AKI" + "A" + "IOSFODNN7EXAMPLE"


def _planted_pem() -> str:
    return "-----BEG" + "IN RSA PRIVATE KEY-----"


def _planted_vendor_sk() -> str:
    return "sk-live-" + "X7q9Zm2kL4vN8pQ1wE5rT"


def _planted_entropy_token() -> str:
    # Assembled from short fragments; no scannable literal in source.
    parts = ["K7#mQ9!vZ2", "@xP4$wL8&nB", "5*tR1^yH6!dF3"]
    return "".join(parts) + "0%gJ9"


def test_planted_aws_key_fails_scan() -> None:
    assert "aws_access_key" in scan_text(f"key={_planted_aws_key()}")


def test_planted_pem_fails_scan() -> None:
    assert "pem_block" in scan_text(_planted_pem())


def test_planted_vendor_sk_fails_scan() -> None:
    assert "vendor_sk" in scan_text(f"api_key={_planted_vendor_sk()}")


def test_planted_high_entropy_token_fails_scan() -> None:
    assert "high_entropy_token" in scan_text(f"secret={_planted_entropy_token()}")


def test_clean_tree_passes_scan() -> None:
    results = scan_tree()
    assert results == {}, f"secret scan findings: {results}"


def test_redaction_cert_on_every_record() -> None:
    seed = EVAL_ROOT / "datasets" / "v0.1" / "seed.jsonl"
    assert seed.exists(), "seed.jsonl missing"
    with seed.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            record = json.loads(line)
            cert = record.get("redaction_cert")
            assert isinstance(cert, dict), f"line {lineno}: redaction_cert missing"
            assert cert.get("redacted") is True, f"line {lineno}: not certified redacted"
            assert cert.get("scanner"), f"line {lineno}: scanner version missing"
            payload = record.get("canonical", {}).get("redacted_payload", "")
            assert scan_text(str(payload)) == [], f"line {lineno}: payload trips scan"
