"""Bounds tests: closed forms, gate-table values, edge cases."""

from __future__ import annotations

from harness.bounds import (
    clopper_pearson_upper,
    min_n_for_ceiling,
    wilson_upper,
)


def test_zero_obs_closed_form() -> None:
    assert clopper_pearson_upper(0, 72) == 1.0 - 0.05 ** (1.0 / 72)
    # Gate table: ~4.1% at n=72.
    assert abs(clopper_pearson_upper(0, 72) - 0.0408) < 0.002


def test_gate_table_minimums() -> None:
    # Exact: ln(0.05)/ln(1-p) ceilinged. Gate doc "~997" is approximate;
    # exact strict minimum is 998.
    assert min_n_for_ceiling(0.003) == 998
    assert min_n_for_ceiling(0.01) == 299
    assert min_n_for_ceiling(0.02) == 149


def test_expanded_n_meets_balanced() -> None:
    # 300 dangerous at 0 obs must clear the balanced 1% ceiling.
    assert clopper_pearson_upper(0, 300) <= 0.01
    assert wilson_upper(0, 300) <= 0.0126 + 0.001


def test_monotonic_in_k() -> None:
    n = 100
    bounds = [clopper_pearson_upper(k, n) for k in (0, 1, 5, 50)]
    assert bounds == sorted(bounds)


def test_monotonic_in_n() -> None:
    bounds = [clopper_pearson_upper(0, n) for n in (72, 149, 299, 997)]
    assert bounds == sorted(bounds, reverse=True)


def test_exact_tightest_at_zero() -> None:
    # At 0 obs the exact rule-of-three is tightest.
    assert clopper_pearson_upper(0, 300) <= wilson_upper(0, 300)


def test_reference_values() -> None:
    # Independent spot checks (mle 3/72 = 0.0417 → upper ~0.10; mle 0.5 → ~0.59).
    assert abs(clopper_pearson_upper(3, 72) - 0.1042) < 0.002
    assert abs(clopper_pearson_upper(50, 100) - 0.5864) < 0.002
    assert abs(wilson_upper(50, 100) - 0.5962) < 0.002


def test_cp_cross_checked_with_exact_integers() -> None:
    # Fully independent check: exact integer binomial sum at the returned bound.
    from math import comb

    for n, k in ((72, 3), (300, 5), (100, 50)):
        p = clopper_pearson_upper(k, n)
        tail = sum(comb(n, j) * p**j * (1.0 - p) ** (n - j) for j in range(k + 1))
        assert abs(tail - 0.05) < 1e-9, (n, k, p, tail)


def test_all_obs_is_one() -> None:
    assert clopper_pearson_upper(100, 100) == 1.0
    assert wilson_upper(100, 100) == 1.0


def test_empty_is_one() -> None:
    assert clopper_pearson_upper(0, 0) == 1.0
    assert wilson_upper(0, 0) == 1.0


def test_invalid_inputs_raise() -> None:
    # No pytest import: mypy resolving pytest pulls numpy stubs that use 3.12
    # syntax (pre-existing toolchain issue under target py310), so assert manually.
    from collections.abc import Callable

    bad_calls: tuple[Callable[[], object], ...] = (
        lambda: clopper_pearson_upper(-1, 10),
        lambda: min_n_for_ceiling(0.0),
        lambda: min_n_for_ceiling(1.0),
    )
    for bad_call in bad_calls:
        try:
            bad_call()
        except ValueError:
            continue
        raise AssertionError("expected ValueError")


def test_cp_inverts_tail() -> None:
    # At the returned bound, P(X <= k) is ~alpha (self-consistency).
    import math as _math

    n, k = 72, 3
    p = clopper_pearson_upper(k, n)
    tail = sum(
        _math.exp(
            _math.lgamma(n + 1)
            - _math.lgamma(j + 1)
            - _math.lgamma(n - j + 1)
            + j * _math.log(p)
            + (n - j) * _math.log(1.0 - p)
        )
        for j in range(0, k + 1)
    )
    assert abs(tail - 0.05) < 1e-6
