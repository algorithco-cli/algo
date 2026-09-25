"""Binomial confidence bounds for gate statistics (exit-gate-P0.md §1.1).

Exact Clopper-Pearson upper bounds and Wilson upper bounds for observed
false_allow counts, plus the minimum dangerous-slice n needed to claim a
ceiling with 0 observed events at 95% confidence
(``n = ceil(log(alpha)/log(1-p))``).

Pure math, no providers, no network. Used to size dataset expansion
(DEFERRED 2.1) and to report bounds alongside measured rates.
"""

from __future__ import annotations

import math

Z_95 = 1.959963984540054


def _log_binom_pmf(n: int, k: int, p: float) -> float:
    if p <= 0.0:
        return 0.0 if k == 0 else float("-inf")
    if p >= 1.0:
        return 0.0 if k == n else float("-inf")
    return (
        math.lgamma(n + 1)
        - math.lgamma(k + 1)
        - math.lgamma(n - k + 1)
        + k * math.log(p)
        + (n - k) * math.log(1.0 - p)
    )


def _logaddexp(a: float, b: float) -> float:
    """log(exp(a) + exp(b)) without overflow (math has no logaddexp)."""
    if a == float("-inf"):
        return b
    if b == float("-inf"):
        return a
    return max(a, b) + math.log1p(math.exp(-abs(a - b)))


def _lower_tail(n: int, k: int, p: float) -> float:
    """P(X <= k) for X ~ Binomial(n, p), log-domain sum (stable to n ~ 1e6)."""
    if k < 0:
        return 0.0
    if k >= n:
        return 1.0
    total = float("-inf")
    for j in range(0, k + 1):
        total = _logaddexp(total, _log_binom_pmf(n, j, p))
    return math.exp(total)


def clopper_pearson_upper(k: int, n: int, alpha: float = 0.05) -> float:
    """Exact one-sided 1-alpha upper bound for a binomial rate.

    k=0 has the closed form ``1 - alpha**(1/n)`` (rule of three, exact);
    otherwise bisection on the upper tail to 1e-12.
    """
    if n <= 0:
        return 1.0
    if k < 0:
        raise ValueError("k must be >= 0")
    if k >= n:
        return 1.0
    if k == 0:
        return 1.0 - float(alpha ** (1.0 / n))
    # Upper bound p_u solves P(X <= k; p_u) = alpha. The lower tail falls from
    # ~0.5 at p = k/n toward 0 at p = 1, so bisection converges from below.
    lo, hi = k / n, 1.0
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        if _lower_tail(n, k, mid) > alpha:
            lo = mid
        else:
            hi = mid
        if hi - lo < 1e-12:
            break
    return hi


def wilson_upper(k: int, n: int, z: float = Z_95) -> float:
    """Wilson score one-sided upper bound (95% default)."""
    if n <= 0:
        return 1.0
    if k >= n:
        return 1.0
    if k <= 0:
        return (z * z) / (n + z * z)
    p = k / n
    denom = 1.0 + z * z / n
    center = p + z * z / (2.0 * n)
    margin = z * math.sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n))
    return min(1.0, (center + margin) / denom)


def min_n_for_ceiling(p: float, alpha: float = 0.05) -> int:
    """Minimum n so that 0 observed events claims rate <= p at 1-alpha."""
    if not 0.0 < p < 1.0:
        raise ValueError("ceiling p must be in (0, 1)")
    return math.ceil(math.log(alpha) / math.log(1.0 - p))


__all__ = [
    "clopper_pearson_upper",
    "min_n_for_ceiling",
    "wilson_upper",
]
