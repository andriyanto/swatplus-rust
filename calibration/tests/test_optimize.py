"""Tests of the parameter space and the three searches (M4), on functions whose minimum is known."""

import numpy as np
import pytest

from swatcal.optimize import History, bayes_opt, random_search, sufi2
from swatcal.params import Param, Space, lhs

SPACE = Space((Param("cn2:pctchg", -25, 25), Param("alpha:pctchg", -60, 60), Param("epco:absval", 0.1, 1.0)))
TRUE = np.array([8.0, -20.0, 0.6])


def bowl(xs):
    """A smooth bowl with its minimum (0.01) at TRUE, scaled so each parameter counts alike."""
    u, t = SPACE.to_unit(xs), SPACE.to_unit(TRUE)
    return np.sqrt(((u - t) ** 2).sum(-1)) + 0.01


def test_space_maps_points_and_sets():
    assert np.allclose(SPACE.from_unit(SPACE.to_unit(TRUE)), TRUE)
    assert SPACE.sets(np.array([TRUE]))[0] == {"cn2:pctchg": 8.0, "alpha:pctchg": -20.0, "epco:absval": 0.6}


def test_lhs_has_one_point_per_stratum():
    x = lhs(10, [0, 0], [1, 10], np.random.default_rng(0))
    assert x.shape == (10, 2)
    for j, top in enumerate([1, 10]):
        strata = np.floor(x[:, j] / top * 10).astype(int)
        assert sorted(strata) == list(range(10))


def test_history_tracks_the_best():
    h = History()
    h.add(np.array([[0.0], [1.0]]), [3.0, 2.0])
    h.add(np.array([[2.0]]), [np.inf])
    assert h.n == 3 and h.best()[1] == 2.0
    assert list(h.best_so_far()) == [3.0, 2.0, 2.0]
    assert h.batch == [0, 0, 1]


@pytest.mark.parametrize("search", ["random", "sufi2", "bayes"])
def test_each_search_uses_its_budget_and_finds_the_bowl(search):
    rng = np.random.default_rng(1)
    if search == "random":
        h = random_search(bowl, SPACE, 40, rng)
    elif search == "sufi2":
        h = sufi2(bowl, SPACE, 40, rng)
    else:
        h = bayes_opt(bowl, SPACE, 40, rng, batch=2)
    assert h.n == 40
    x, y = h.best()
    assert np.all(x >= SPACE.lo) and np.all(x <= SPACE.hi)
    assert y < 0.25


def test_sufi2_narrows_its_ranges_around_the_best():
    h = sufi2(bowl, SPACE, 80, np.random.default_rng(2), iterations=4)
    widths = [np.array(n["hi"]) - np.array(n["lo"]) for n in h.notes]
    assert len(widths) == 4
    assert np.all(widths[-1] <= widths[0] + 1e-12)
    last = h.notes[-1]
    assert np.all(np.array(last["lo"]) <= TRUE + 1e-9) and np.all(TRUE <= np.array(last["hi"]) + 1e-9)


def test_bayes_beats_random_on_a_smooth_function():
    # Over a few seeds, on the same budget: what Bayesian optimisation is for.
    rb, rr = [], []
    for seed in range(3):
        rb.append(bayes_opt(bowl, SPACE, 30, np.random.default_rng(seed)).best()[1])
        rr.append(random_search(bowl, SPACE, 30, np.random.default_rng(seed)).best()[1])
    assert np.mean(rb) < np.mean(rr)


def test_failed_runs_are_ignored():
    def sometimes(xs):
        y = bowl(xs)
        y[::3] = np.inf
        return y

    h = bayes_opt(sometimes, SPACE, 20, np.random.default_rng(0))
    assert h.n == 20 and np.isfinite(h.best()[1])
