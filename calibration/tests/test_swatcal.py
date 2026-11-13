"""Tests of swatcal (M3): rating curves, goodness of fit, the objective, and runs through
`swatplus-rust batch` (skipped when the program hasn't been built)."""

import math
import os
import shutil

import numpy as np
import pytest

from swatcal import Observations, Rating, evaluate, fit_rating, kge, nse, pbias, rsr, spearman, synthetic_stage
from swatcal.runner import read_series, run_sets

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def flows(n=1500, seed=3):
    """A plausible daily hydrograph: baseflow with storm peaks and recessions."""
    rng = np.random.default_rng(seed)
    q = np.empty(n)
    v = 20.0
    for i in range(n):
        v = 8.0 + (v - 8.0) * 0.9 + (rng.gamma(0.4, 60.0) if rng.random() < 0.25 else 0.0)
        q[i] = v
    return q


def test_rating_round_trip_and_zero_flow():
    r = Rating(a=30.0, b=1.7, h0=0.4)
    q = np.array([0.0, 1.0, 10.0, 250.0])
    h = r.stage(q)
    assert h[0] == pytest.approx(0.4)
    assert np.allclose(r.flow(h), q)
    assert r.flow(np.array([0.2]))[0] == 0.0  # below h0: no flow
    assert math.isnan(r.stage(np.array([np.nan]))[0])


def test_synthetic_stage_is_reproducible_and_rounded():
    r = Rating(30.0, 1.7, 0.4)
    q = flows()
    h1 = synthetic_stage(q, r, noise_sd=0.02, resolution=0.01, missing=0.1, seed=7)
    h2 = synthetic_stage(q, r, noise_sd=0.02, resolution=0.01, missing=0.1, seed=7)
    assert np.array_equal(h1, h2, equal_nan=True)
    ok = np.isfinite(h1)
    assert 0.05 < 1 - ok.mean() < 0.15
    assert np.allclose(h1[ok] * 100, np.round(h1[ok] * 100))


def test_fit_rating_finds_the_curve_back():
    r = Rating(30.0, 1.7, 0.4)
    q = flows()
    fit = fit_rating(q, r.stage(q))
    assert fit.rmse < 2e-3
    assert fit.rating.b == pytest.approx(1.7, abs=0.02)
    assert fit.rating.h0 == pytest.approx(0.4, abs=0.02)
    assert fit.rating.a == pytest.approx(30.0, rel=0.05)


def test_stage_cannot_see_a_volume_error():
    # A model whose flow is 1.5 times too large matches the same stage through a curve with a
    # 1.5 times larger: the stage score is perfect, only the curve's a betrays it.
    r = Rating(30.0, 1.7, 0.4)
    q = flows()
    h = r.stage(q)
    fit = fit_rating(1.5 * q, h)
    assert fit.rmse < 2e-3
    assert fit.rating.a == pytest.approx(45.0, rel=0.05)


def test_metrics_known_values():
    obs = np.array([1.0, 2.0, 3.0, 4.0, 5.0])
    assert nse(obs, obs) == 1.0
    assert nse(np.full(5, 3.0), obs) == pytest.approx(0.0)
    k = kge(obs, obs)
    assert (k.kge, k.r, k.alpha, k.beta) == pytest.approx((1.0, 1.0, 1.0, 1.0))
    assert kge(2 * obs, obs).beta == pytest.approx(2.0)
    assert pbias(1.1 * obs, obs) == pytest.approx(10.0)
    assert rsr(obs, obs) == 0.0
    # Spearman ignores any monotonic transform, and shares the ranks of ties.
    assert spearman(np.exp(obs), obs) == pytest.approx(1.0)
    assert spearman(-obs, obs) == pytest.approx(-1.0)
    assert spearman(np.array([1.0, 1.0, 2.0, 3.0]), np.array([1.0, 2.0, 3.0, 4.0])) == pytest.approx(0.9486833, abs=1e-6)
    # Missing days are left out.
    assert nse(np.array([1.0, np.nan, 3.0]), np.array([1.0, 2.0, 3.0])) == 1.0


def test_objective_sees_the_volume_through_et_and_gaugings():
    r = Rating(30.0, 1.7, 0.4)
    q = flows()
    et = np.full(q.shape, 3.0)  # mm/day
    gaugings = [(float(r.stage(v)), float(v)) for v in np.quantile(q, [0.1, 0.5, 0.9])]
    obs = Observations(r.stage(q), et_annual_mm=3.0 * 365.25, gaugings=gaugings)
    right = evaluate(q, et, obs)
    assert right.distance < 0.01
    # Too much flow (and too little ET to make room for it): the stage still fits.
    wrong = evaluate(1.5 * q, et * 0.8, obs)
    assert wrong.kge_stage.kge > 0.99
    assert wrong.et_bias == pytest.approx(-0.2)
    assert wrong.gauging_error > 0.3
    assert wrong.distance > 10 * right.distance


def test_read_series(tmp_path):
    p = tmp_path / "run_a.csv"
    p.write_text("date,precip_mm,et_mm,flo_3,flo_12,dep_3,dep_mann_3,dep_12,dep_mann_12\n"
                 "2017-01-01,8.5,3.0,10.0,2.0,0.5,0.6,0.1,0.2\n2017-01-02,0,2.5,9.0,1.5,0.45,0.55,0.09,0.18\n")
    s = read_series(str(p))
    assert s.dates[1] == np.datetime64("2017-01-02")
    assert list(s.flo) == [3, 12] and s.flo[12][1] == 1.5
    assert s.dep[3][0] == 0.5 and s.dep_mann[12][1] == 0.18


def exe():
    for path in ("target/release/swatplus-rust", "target/debug/swatplus-rust"):
        full = os.path.join(ROOT, path)
        if os.path.exists(full) or os.path.exists(full + ".exe"):
            return full
    return shutil.which("swatplus-rust")


@pytest.mark.skipif(exe() is None, reason="swatplus-rust is not built")
def test_runs_through_batch(tmp_path):
    project = os.path.join(ROOT, "testdata", "Osu_1hru")
    runs = run_sets(exe(), project, [{}, {"esco:absval": 0.5}, {"esco:absval": 1.0}, {"no_such:pctchg": 1.0}], [1],
                    names=["base", "lo", "hi", "bad"], workdir=str(tmp_path))
    assert not isinstance(runs["base"], str)
    assert len(runs["base"].flo[1]) == len(runs["base"].dates) > 0
    assert not np.array_equal(runs["lo"].flo[1], runs["hi"].flo[1])
    assert isinstance(runs["bad"], str) and "no_such" in runs["bad"]
