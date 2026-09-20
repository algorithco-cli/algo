"""Dataset schema conformance for eval v0.1 (P0-EVAL-3).

Every ``.jsonl`` record must validate against ``datasets/v0.1/schema.json``:
- required fields + ``canonical`` payload shape (``tool_before`` was renamed),
- non-empty ``rationale`` and ``redaction_cert.redacted is True``,
- label / obfuscation / tool_kind / privacy_mode enums match the taxonomy.

Implemented without third-party validators so the eval tree keeps its
dependency footprint (pandas + scikit-learn only).
"""

from __future__ import annotations

import json
import re
from pathlib import Path

EVAL_ROOT = Path(__file__).resolve().parent.parent
SCHEMA_PATH = EVAL_ROOT / "datasets" / "v0.1" / "schema.json"
DATASET_PATHS = sorted((EVAL_ROOT / "datasets" / "v0.1").glob("*.jsonl"))

LABELS = {"SAFE", "DANGEROUS", "AMBIGUOUS"}
OBFUSCATION = {"NONE", "VAR_EXPANSION", "ENCODING", "SUBSHELL", "PIPE_CHAIN", "OTHER"}
TOOL_KINDS = {"SHELL", "EDIT", "WRITE", "READ", "NET", "OTHER"}
PRIVACY_MODES = {"local-only", "redacted", "full"}
RECORD_ID_RE = re.compile(r"^rec-v01-[0-9]{3}$")


def _load_schema() -> dict[str, object]:
    with SCHEMA_PATH.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    assert isinstance(schema, dict)
    return schema


def _load_records() -> list[dict[str, object]]:
    assert DATASET_PATHS, "no .jsonl datasets found under datasets/v0.1"
    records: list[dict[str, object]] = []
    for path in DATASET_PATHS:
        with path.open(encoding="utf-8") as handle:
            for lineno, line in enumerate(handle, start=1):
                if not line.strip():
                    continue
                record = json.loads(line)
                assert isinstance(record, dict), f"{path.name}:{lineno}: not an object"
                records.append(record)
    assert records, "dataset is empty"
    return records


def test_schema_enums_match_taxonomy() -> None:
    schema = _load_schema()
    props = schema["properties"]
    assert isinstance(props, dict)
    assert set(props["label"]["enum"]) == LABELS
    assert set(props["obfuscation"]["enum"]) == OBFUSCATION
    assert set(props["canonical"]["properties"]["tool_kind"]["enum"]) == TOOL_KINDS
    assert set(props["canonical"]["properties"]["privacy_mode"]["enum"]) == PRIVACY_MODES
    required = schema["required"]
    assert isinstance(required, list)
    assert "canonical" in required
    assert "tool_before" not in props


def test_every_record_validates_against_schema() -> None:
    for record in _load_records():
        assert isinstance(record.get("record_id"), str)
        assert RECORD_ID_RE.match(str(record["record_id"])), record.get("record_id")
        assert "tool_before" not in record, f"{record.get('record_id')}: stale tool_before field"
        canonical = record.get("canonical")
        assert isinstance(canonical, dict), f"{record.get('record_id')}: canonical missing"
        assert isinstance(canonical.get("event_id"), str)
        assert isinstance(canonical.get("agent"), dict)
        assert canonical["agent"].get("agent_type")
        assert canonical["agent"].get("session_id")
        assert canonical.get("tool_kind") in TOOL_KINDS
        payload = canonical.get("redacted_payload")
        assert isinstance(payload, str) and len(payload) >= 1
        assert canonical.get("privacy_mode") in PRIVACY_MODES
        assert record.get("label") in LABELS, record.get("record_id")
        assert record.get("obfuscation") in OBFUSCATION, record.get("record_id")
        assert isinstance(record.get("annotator"), str) and len(str(record["annotator"])) >= 1
        assert record.get("dataset_version") == "v0.1"


def test_rationale_and_redaction_cert() -> None:
    for record in _load_records():
        rationale = record.get("rationale")
        assert isinstance(rationale, str) and len(rationale) >= 10, record.get("record_id")
        cert = record.get("redaction_cert")
        assert isinstance(cert, dict), f"{record.get('record_id')}: redaction_cert missing"
        assert cert.get("redacted") is True, f"{record.get('record_id')}: not certified"
        assert isinstance(cert.get("scanner"), str) and len(str(cert["scanner"])) >= 1
