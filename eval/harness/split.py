"""Dev/held-out stratified split (EVAL-6 §3).

Stratified 70/30 on label × obfuscation, seed 42, preserves 40/30/30 and 30% obfuscated.
All tuning uses dev only; held-out is scored once. Never tune on held-out.

Usage:
  python -m harness.split --dataset datasets/v0.1/seed.jsonl \
    --dev datasets/v0.1/dev.jsonl --held-out datasets/v0.1/held-out.jsonl \
    --seed 42 --ratio 0.7
"""

from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict
from pathlib import Path


def load_rows(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def split_rows(rows: list[dict], ratio: float, seed: int) -> tuple[list[dict], list[dict]]:
    # Stratify by (label, obfuscation)
    buckets: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for r in rows:
        key = (str(r.get("label", "AMBIGUOUS")), str(r.get("obfuscation", "NONE")))
        buckets[key].append(r)
    rnd = random.Random(seed)
    dev, held = [], []
    for _key, bucket in sorted(buckets.items()):
        rnd.shuffle(bucket)
        # Round to nearest to hit the global ratio more accurately (e.g., 240*0.7=168)
        cut = int(len(bucket) * ratio + 0.5)
        # Ensure at least 1 in dev if bucket has 1, and at least 1 in held if bucket >1
        if len(bucket) == 1:
            dev.extend(bucket)
        elif cut >= len(bucket):
            # Keep at least 1 in held-out for buckets >1
            dev.extend(bucket[:-1])
            held.extend(bucket[-1:])
        elif cut == 0:
            dev.extend(bucket[:1])
            held.extend(bucket[1:])
        else:
            dev.extend(bucket[:cut])
            held.extend(bucket[cut:])
    rnd.shuffle(dev)
    rnd.shuffle(held)
    return dev, held


def main() -> int:
    ap = argparse.ArgumentParser(description="Stratified dev/held-out split for EVAL-6")
    ap.add_argument("--dataset", required=True, type=Path)
    ap.add_argument("--dev", required=True, type=Path)
    ap.add_argument("--held-out", required=True, type=Path)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--ratio", type=float, default=0.7, help="dev fraction")
    args = ap.parse_args()

    rows = load_rows(args.dataset)
    dev, held = split_rows(rows, args.ratio, args.seed)

    for out, data in [(args.dev, dev), (args.held_out, held)]:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            "\n".join(json.dumps(r, ensure_ascii=False) for r in data) + "\n", encoding="utf-8"
        )
        print(f"wrote {out} {len(data)} records (seed {args.seed}, ratio {args.ratio})")

    # Summary
    from collections import Counter

    def summary(name: str, data: list[dict]) -> None:
        labels = Counter(r["label"] for r in data)
        obfs = Counter(r["obfuscation"] for r in data)
        total_obf = sum(v for k, v in obfs.items() if k != "NONE")
        print(
            f"{name}: {len(data)}  labels {dict(labels)}  "
            f"obfuscated {total_obf}/{len(data)} ({total_obf / len(data) * 100:.1f}%)"
        )

    summary("dev", dev)
    summary("held-out", held)
    summary("total", rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
