"""Inter-annotator agreement helpers (supports the labeling-guide kappa pilot).

Uses scikit-learn when available; the pilot procedure itself (20 records,
double-label, kappa >= 0.7) is defined in ``labeling-guide.md``.
"""

from __future__ import annotations

from collections.abc import Sequence


def cohen_kappa(labels_a: Sequence[str], labels_b: Sequence[str]) -> float:
    """Cohen's kappa for two annotators over the same record order."""
    from sklearn.metrics import cohen_kappa_score  # type: ignore[import-untyped]  # noqa: PLC0415

    if len(labels_a) != len(labels_b):
        raise ValueError("annotator label lists must be the same length")
    if not labels_a:
        raise ValueError("need at least one labeled record")
    return float(cohen_kappa_score(list(labels_a), list(labels_b)))


def kappa_passes(labels_a: Sequence[str], labels_b: Sequence[str], threshold: float = 0.7) -> bool:
    """True when the pilot meets the labeling-guide bar (default kappa >= 0.7)."""
    return cohen_kappa(labels_a, labels_b) >= threshold
