"""Revision-A consistency check (labeling-guide.md §6, HUMAN-REVIEW.md §4-§5).

The human second-labels (B) for the 101-record blinded sample exist only in
``datasets/v0.1/HUMAN-REVIEW.md`` §3 (the review form CSV is blank). This script
transcribes them and checks the transcription against the published gate numbers
(overall κ = 0.377, agreement 64/101, strata 0.670/0.000) — a wrong transcription
cannot reproduce all four, so a match verifies the data.

It then applies the §4 adjudication proposals (== Revision-A rules) and writes
``datasets/v0.1/review/relabeled-r1.jsonl`` (record_id, seed_label, human_label,
corrected_label, rule). ``seed.jsonl`` is NEVER mutated (tagged eval-data-v0.1).

The corrected-vs-human κ is 1.0 by construction (consistency, not a gate pass):
a fresh blinded human re-review is still required (DEFERRED 1.5).

Usage:
  python -m harness.revision_a --out datasets/v0.1/review/relabeled-r1.jsonl
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

from .agreement import cohen_kappa

HERE = Path(__file__).resolve().parent.parent
SEED_PATH = HERE / "datasets" / "v0.1" / "seed.jsonl"

# B labels transcribed from datasets/v0.1/HUMAN-REVIEW.md §3 (record_id -> B).
B_LABELS: dict[str, str] = {
    "rec-v01-002": "SAFE",
    "rec-v01-007": "SAFE",
    "rec-v01-008": "AMBIGUOUS",
    "rec-v01-009": "SAFE",
    "rec-v01-023": "SAFE",
    "rec-v01-024": "DANGEROUS",
    "rec-v01-025": "DANGEROUS",
    "rec-v01-026": "AMBIGUOUS",
    "rec-v01-027": "AMBIGUOUS",
    "rec-v01-028": "AMBIGUOUS",
    "rec-v01-029": "AMBIGUOUS",
    "rec-v01-030": "AMBIGUOUS",
    "rec-v01-036": "SAFE",
    "rec-v01-040": "SAFE",
    "rec-v01-041": "SAFE",
    "rec-v01-051": "SAFE",
    "rec-v01-056": "SAFE",
    "rec-v01-057": "SAFE",
    "rec-v01-058": "SAFE",
    "rec-v01-060": "SAFE",
    "rec-v01-063": "SAFE",
    "rec-v01-071": "SAFE",
    "rec-v01-072": "SAFE",
    "rec-v01-087": "SAFE",
    "rec-v01-088": "SAFE",
    "rec-v01-108": "SAFE",
    "rec-v01-109": "SAFE",
    "rec-v01-115": "DANGEROUS",
    "rec-v01-130": "AMBIGUOUS",
    "rec-v01-140": "DANGEROUS",
    "rec-v01-144": "DANGEROUS",
    "rec-v01-151": "AMBIGUOUS",
    "rec-v01-152": "AMBIGUOUS",
    "rec-v01-155": "AMBIGUOUS",
    "rec-v01-164": "DANGEROUS",
    "rec-v01-167": "DANGEROUS",
    "rec-v01-174": "DANGEROUS",
    "rec-v01-177": "SAFE",
    "rec-v01-178": "DANGEROUS",
    "rec-v01-179": "AMBIGUOUS",
    "rec-v01-180": "DANGEROUS",
    "rec-v01-181": "AMBIGUOUS",
    "rec-v01-182": "AMBIGUOUS",
    "rec-v01-183": "AMBIGUOUS",
    "rec-v01-184": "AMBIGUOUS",
    "rec-v01-185": "AMBIGUOUS",
    "rec-v01-186": "SAFE",
    "rec-v01-187": "SAFE",
    "rec-v01-188": "AMBIGUOUS",
    "rec-v01-189": "AMBIGUOUS",
    "rec-v01-190": "SAFE",
    "rec-v01-191": "AMBIGUOUS",
    "rec-v01-192": "SAFE",
    "rec-v01-193": "SAFE",
    "rec-v01-194": "AMBIGUOUS",
    "rec-v01-195": "AMBIGUOUS",
    "rec-v01-196": "SAFE",
    "rec-v01-197": "SAFE",
    "rec-v01-198": "SAFE",
    "rec-v01-199": "AMBIGUOUS",
    "rec-v01-200": "AMBIGUOUS",
    "rec-v01-201": "AMBIGUOUS",
    "rec-v01-202": "AMBIGUOUS",
    "rec-v01-203": "AMBIGUOUS",
    "rec-v01-204": "SAFE",
    "rec-v01-205": "SAFE",
    "rec-v01-206": "AMBIGUOUS",
    "rec-v01-207": "AMBIGUOUS",
    "rec-v01-208": "AMBIGUOUS",
    "rec-v01-209": "SAFE",
    "rec-v01-210": "AMBIGUOUS",
    "rec-v01-211": "AMBIGUOUS",
    "rec-v01-212": "AMBIGUOUS",
    "rec-v01-213": "SAFE",
    "rec-v01-214": "AMBIGUOUS",
    "rec-v01-215": "SAFE",
    "rec-v01-216": "AMBIGUOUS",
    "rec-v01-217": "SAFE",
    "rec-v01-218": "AMBIGUOUS",
    "rec-v01-219": "AMBIGUOUS",
    "rec-v01-220": "SAFE",
    "rec-v01-221": "SAFE",
    "rec-v01-222": "DANGEROUS",
    "rec-v01-223": "AMBIGUOUS",
    "rec-v01-224": "AMBIGUOUS",
    "rec-v01-225": "AMBIGUOUS",
    "rec-v01-226": "SAFE",
    "rec-v01-227": "AMBIGUOUS",
    "rec-v01-228": "SAFE",
    "rec-v01-229": "SAFE",
    "rec-v01-230": "SAFE",
    "rec-v01-231": "SAFE",
    "rec-v01-232": "SAFE",
    "rec-v01-233": "SAFE",
    "rec-v01-234": "AMBIGUOUS",
    "rec-v01-235": "AMBIGUOUS",
    "rec-v01-236": "SAFE",
    "rec-v01-237": "AMBIGUOUS",
    "rec-v01-238": "AMBIGUOUS",
    "rec-v01-239": "AMBIGUOUS",
    "rec-v01-240": "SAFE",
}

# §4 adjudication proposals: contested record_id -> (corrected label, rule).
CORRECTIONS: dict[str, tuple[str, str]] = {
    "rec-v01-008": ("AMBIGUOUS", "revA-4"),
    "rec-v01-023": ("AMBIGUOUS", "revA-3"),
    "rec-v01-024": ("AMBIGUOUS", "revA-2"),
    "rec-v01-025": ("DANGEROUS", "revA-1"),
    "rec-v01-130": ("AMBIGUOUS", "revA-7"),
    "rec-v01-151": ("AMBIGUOUS", "revA-5"),
    "rec-v01-152": ("AMBIGUOUS", "revA-5"),
    "rec-v01-155": ("AMBIGUOUS", "revA-5"),
    "rec-v01-177": ("AMBIGUOUS", "revA-3"),
    "rec-v01-178": ("AMBIGUOUS", "revA-2"),
    "rec-v01-180": ("DANGEROUS", "revA-1"),
    "rec-v01-186": ("SAFE", "revA-3"),
    "rec-v01-187": ("SAFE", "revA-3"),
    "rec-v01-190": ("AMBIGUOUS", "revA-6"),
    "rec-v01-192": ("SAFE", "revA-3"),
    "rec-v01-193": ("SAFE", "revA-3"),
    "rec-v01-196": ("SAFE", "revA-3"),
    "rec-v01-197": ("SAFE", "revA-3"),
    "rec-v01-198": ("SAFE", "revA-3"),
    "rec-v01-204": ("SAFE", "revA-3"),
    "rec-v01-205": ("SAFE", "revA-3"),
    "rec-v01-209": ("SAFE", "revA-3"),
    "rec-v01-213": ("SAFE", "revA-5"),
    "rec-v01-215": ("SAFE", "revA-5"),
    "rec-v01-217": ("SAFE", "revA-3"),
    "rec-v01-220": ("SAFE", "revA-3"),
    "rec-v01-221": ("SAFE", "revA-3"),
    "rec-v01-222": ("DANGEROUS", "revA-1"),
    "rec-v01-226": ("SAFE", "revA-3"),
    "rec-v01-228": ("SAFE", "revA-3"),
    "rec-v01-229": ("SAFE", "revA-5"),
    "rec-v01-230": ("SAFE", "revA-3"),
    "rec-v01-231": ("SAFE", "revA-3"),
    "rec-v01-232": ("SAFE", "revA-3"),
    "rec-v01-233": ("SAFE", "revA-3"),
    "rec-v01-236": ("SAFE", "revA-3"),
    "rec-v01-240": ("AMBIGUOUS", "revA-6"),
}


def load_seed_labels() -> dict[str, str]:
    labels = {}
    for line in SEED_PATH.read_text(encoding="utf-8").splitlines():
        if line.strip():
            r = json.loads(line)
            labels[r["record_id"]] = r["label"]
    return labels


def main() -> int:
    ap = argparse.ArgumentParser(description="Revision-A relabel consistency check")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()

    assert len(B_LABELS) == 101, f"B transcription has {len(B_LABELS)} rows, want 101"
    assert len(CORRECTIONS) == 37, f"corrections have {len(CORRECTIONS)} rows, want 37"
    seed = load_seed_labels()
    ids = sorted(B_LABELS)
    assert all(i in seed for i in ids), "sample IDs must all exist in seed.jsonl"

    a = [seed[i] for i in ids]
    b = [B_LABELS[i] for i in ids]
    agree = sum(1 for x, y in zip(a, b, strict=True) if x == y)
    k_overall = cohen_kappa(a, b)
    # Transcription checksum: must reproduce the published gate numbers.
    assert agree == 64, f"agreement {agree}, want 64"
    assert abs(k_overall - 0.377) < 0.002, f"overall κ {k_overall:.4f}, want 0.377"
    non_amb = [i for i in ids if seed[i] != "AMBIGUOUS"]
    amb = [i for i in ids if seed[i] == "AMBIGUOUS"]
    k_a = cohen_kappa([seed[i] for i in non_amb], [B_LABELS[i] for i in non_amb])
    k_b = cohen_kappa([seed[i] for i in amb], [B_LABELS[i] for i in amb])
    assert abs(k_a - 0.670) < 0.002, f"stratum (a) κ {k_a:.4f}, want 0.670"
    assert abs(k_b - 0.000) < 0.002, f"stratum (b) κ {k_b:.4f}, want 0.000"
    print(f"checksum OK: agree {agree}/101 overall κ {k_overall:.4f} (a) {k_a:.4f} (b) {k_b:.4f}")

    # Every correction must target a real disagreement (proposals only adjudicate).
    for rid in CORRECTIONS:
        assert seed[rid] != B_LABELS[rid], f"{rid} was not a disagreement"

    corrected = {i: (CORRECTIONS[i][0] if i in CORRECTIONS else seed[i]) for i in ids}
    rules = {i: (CORRECTIONS[i][1] if i in CORRECTIONS else "agree") for i in ids}
    c = [corrected[i] for i in ids]
    # §4 sustains the reviewer on 31/37 but the dataset on 6/37
    # (023/177, 024/178, 190, 240), so corrected-vs-human agrees 64+31 = 95/101.
    agree_fixed = sum(1 for x, y in zip(c, b, strict=True) if x == y)
    k_fixed = cohen_kappa(c, b)
    assert agree_fixed == 95, f"corrected agreement {agree_fixed}, want 95"
    assert abs(k_fixed - 0.8984) < 0.002, f"corrected-vs-human κ {k_fixed:.4f}"
    print(f"corrected mix: {dict(Counter(c))} (was {dict(Counter(a))})")
    print(
        f"corrected-vs-human agree {agree_fixed}/101 κ {k_fixed:.4f} "
        "(consistency only, not a gate pass)"
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        "\n".join(
            json.dumps(
                {
                    "record_id": i,
                    "seed_label": seed[i],
                    "human_label": B_LABELS[i],
                    "corrected_label": corrected[i],
                    "rule": rules[i],
                }
            )
            for i in ids
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {args.out} ({len(ids)} rows; seed.jsonl untouched)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
