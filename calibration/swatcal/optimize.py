"""Three ways to search a parameter space for the run that scores best, compared on the same
number of model runs (M4 in docs/roadmap.md).

- `random_search`: a Latin hypercube over the whole space, the baseline of any comparison.
- `sufi2`: SUFI-2 (Abbaspour et al., 2004, 2007), the usual procedure with SWAT: iterations of Latin
  hypercube sampling, each in ranges narrowed around the behavioural runs of the one before.
- `bayes_opt`: Bayesian optimisation (Shahriari et al., 2016): a Gaussian process learns the score
  from the runs made so far, and each next run goes where the expected improvement is largest.
  Runs are proposed in batches (constant liar) so that a batch can use every core.

`f` takes an array of points (one row per run, parameter values as in the `Space`) and returns
their scores, smaller is better (`inf` for a failed run); it is called once per batch, so the runs of
a batch can go to `swatplus-rust batch` together.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field

import numpy as np

from .params import Space, lhs


@dataclass
class History:
    """Every run of a search, in order, with the batch it belonged to."""

    x: list[np.ndarray] = field(default_factory=list)
    y: list[float] = field(default_factory=list)
    batch: list[int] = field(default_factory=list)
    notes: list[dict] = field(default_factory=list)  # per batch or iteration (SUFI-2's ranges)

    def add(self, xs, ys):
        b = (self.batch[-1] + 1) if self.batch else 0
        for x, y in zip(np.atleast_2d(xs), ys):
            self.x.append(np.asarray(x, dtype=float))
            self.y.append(float(y))
            self.batch.append(b)

    @property
    def n(self) -> int:
        return len(self.y)

    def best(self) -> tuple[np.ndarray, float]:
        i = int(np.argmin(self.y))
        return self.x[i], self.y[i]

    def best_so_far(self) -> np.ndarray:
        return np.minimum.accumulate(np.array(self.y))


def _eval(f, xs, hist: History):
    ys = np.asarray(f(np.atleast_2d(xs)), dtype=float)
    ys = np.where(np.isfinite(ys), ys, np.inf)
    hist.add(xs, ys)
    return ys


def random_search(f, space: Space, budget: int, rng: np.random.Generator, batch: int | None = None) -> History:
    hist = History()
    xs = lhs(budget, space.lo, space.hi, rng)
    step = batch or budget
    for i in range(0, budget, step):
        _eval(f, xs[i : i + step], hist)
    return hist


def sufi2(f, space: Space, budget: int, rng: np.random.Generator, iterations: int = 4,
          behavioural: float = 0.2) -> History:
    """SUFI-2: `iterations` rounds of `budget // iterations` Latin hypercube runs. After each round
    the runs scoring in the best `behavioural` share are kept as behavioural; with b_lo and b_hi the
    2.5 and 97.5 percentiles of each parameter among them, the next range is (Abbaspour et al., 2007)

        b_min' = b_lo - max((b_lo - b_min) / 2, (b_max - b_hi) / 2)
        b_max' = b_hi + max((b_lo - b_min) / 2, (b_max - b_hi) / 2)

    held within the original range. `notes` keeps each round's ranges."""
    hist = History()
    lo, hi = space.lo.copy(), space.hi.copy()
    per = max(2, budget // iterations)
    left = budget
    for it in range(iterations):
        n = per if it < iterations - 1 else left
        if n <= 0:
            break
        xs = lhs(n, lo, hi, rng)
        hist.notes.append({"iteration": it + 1, "lo": lo.tolist(), "hi": hi.tolist(), "runs": n})
        ys = _eval(f, xs, hist)
        left -= n
        ok = np.isfinite(ys)
        if ok.sum() < 2:
            continue
        k = max(2, int(math.ceil(behavioural * ok.sum())))
        good = xs[ok][np.argsort(ys[ok])[:k]]
        b_lo, b_hi = np.percentile(good, 2.5, axis=0), np.percentile(good, 97.5, axis=0)
        half = np.maximum((b_lo - lo) / 2, (hi - b_hi) / 2)
        lo, hi = np.maximum(space.lo, b_lo - half), np.minimum(space.hi, b_hi + half)
        flat = hi - lo < 1e-9 * (space.hi - space.lo)
        hi = np.where(flat, np.minimum(space.hi, lo + 1e-3 * (space.hi - space.lo)), hi)
    return hist


# --- Gaussian process ----------------------------------------------------------------------------


def _matern52(a, b, ls):
    d = np.sqrt(np.maximum(0.0, (((a[:, None, :] - b[None, :, :]) / ls) ** 2).sum(-1)))
    s = math.sqrt(5.0) * d
    return (1.0 + s + s * s / 3.0) * np.exp(-s)


@dataclass
class _Gp:
    x: np.ndarray
    ls: np.ndarray
    amp: float
    noise: float
    mean: float
    scale: float
    chol: np.ndarray
    alpha: np.ndarray

    @classmethod
    def fit(cls, x, y, rng, tries=160):
        mean, scale = float(np.mean(y)), float(np.std(y)) or 1.0
        z = (y - mean) / scale
        n, d = x.shape
        best = None
        for t in range(tries):
            if t == 0:
                ls, amp, noise = np.full(d, 0.3), 1.0, 1e-4
            else:
                ls = np.exp(rng.uniform(math.log(0.05), math.log(3.0), d))
                amp = math.exp(rng.uniform(math.log(0.3), math.log(3.0)))
                noise = math.exp(rng.uniform(math.log(1e-6), math.log(0.1)))
            k = amp * _matern52(x, x, ls) + noise * np.eye(n)
            try:
                c = np.linalg.cholesky(k)
            except np.linalg.LinAlgError:
                continue
            al = np.linalg.solve(c.T, np.linalg.solve(c, z))
            lml = -0.5 * z @ al - np.log(np.diag(c)).sum()
            if best is None or lml > best[0]:
                best = (lml, ls, amp, noise, c, al)
        _, ls, amp, noise, c, al = best
        return cls(x, ls, amp, noise, mean, scale, c, al)

    def predict(self, xq):
        ks = self.amp * _matern52(xq, self.x, self.ls)
        mu = ks @ self.alpha
        v = np.linalg.solve(self.chol, ks.T)
        var = np.maximum(self.amp - (v * v).sum(0), 1e-12)
        return self.mean + self.scale * mu, self.scale * np.sqrt(var)


_erf = np.vectorize(math.erf)


def _expected_improvement(mu, sd, best, xi=0.01):
    imp = best - mu - xi
    z = imp / sd
    cdf = 0.5 * (1.0 + _erf(z / math.sqrt(2.0)))
    pdf = np.exp(-0.5 * z * z) / math.sqrt(2.0 * math.pi)
    return imp * cdf + sd * pdf


def bayes_opt(f, space: Space, budget: int, rng: np.random.Generator, n_init: int | None = None,
              batch: int = 2, log_y: bool = True, trust_region: bool = True) -> History:
    """Bayesian optimisation: `n_init` Latin hypercube runs (default 2 per parameter + 2), then
    batches of `batch` runs where a Gaussian process (Matérn 5/2, one length scale per parameter,
    fitted by marginal likelihood) expects the most improvement. The points of a batch after its
    first are chosen as if the ones before had scored the best score so far (constant liar). The
    process models log(score) when `log_y` (scores are distances, positive and skewed).

    With `trust_region` (TuRBO, Eriksson et al., 2019, for one region), the candidates are drawn in
    a box around the best point so far, its side scaled by each parameter's length scale: the box
    doubles after 3 batches in a row that improve on the best, halves after `ceil(dim / batch)`
    batches that don't, and starts again from a fresh Latin hypercube once it is smaller than
    1/128 of the space. A process fitted over the whole space smooths over a narrow valley; the box
    lets it follow one."""
    hist = History()
    n_init = n_init or min(budget, 2 * space.dim + 2)
    _eval(f, lhs(n_init, space.lo, space.hi, rng), hist)
    length, succ, fail = 0.8, 0, 0
    fail_tol = max(1, math.ceil(space.dim / max(1, batch)))
    restart_from = 0  # the runs before a restart don't count as the region's best
    while hist.n < budget:
        q = min(batch, budget - hist.n)
        u = space.to_unit(np.array(hist.x))
        y = np.array(hist.y)
        ok = np.isfinite(y)
        u, y = u[ok], y[ok]
        y = np.log(np.maximum(y, 1e-12)) if log_y else y
        gp = _Gp.fit(u, y, rng)
        top = u[np.argsort(y)[:5]]
        y_region = np.array(hist.y[restart_from:])
        best_before = float(np.min(y_region[np.isfinite(y_region)])) if np.isfinite(y_region).any() else np.inf
        picks = []
        for _ in range(q):
            if trust_region:
                centre = space.to_unit(hist.x[restart_from + int(np.argmin(y_region))])
                w = gp.ls / np.exp(np.mean(np.log(gp.ls)))
                half = np.clip(length * w / 2.0, 1e-4, 0.5)
                lo_, hi_ = np.clip(centre - half, 0, 1), np.clip(centre + half, 0, 1)
                cand = lo_ + rng.random((4000, space.dim)) * (hi_ - lo_)
                # Perturb only some coordinates of the centre in most candidates (as TuRBO does).
                mask = rng.random(cand.shape) < min(1.0, 20.0 / space.dim)
                mask[np.arange(len(cand)), rng.integers(0, space.dim, len(cand))] = True
                cand = np.where(mask, cand, centre)
            else:
                cand = np.vstack([rng.random((3000, space.dim)),
                                  np.clip(np.repeat(top, 300, 0) + rng.normal(0, 0.08, (len(top) * 300, space.dim)), 0, 1)])
            mu, sd = gp.predict(cand)
            ei = _expected_improvement(mu, sd, float(y.min()))
            pick = cand[int(np.argmax(ei))]
            picks.append(pick)
            # Constant liar: pretend the pick scored the best so far, and refit with the same
            # hyperparameters so the next pick goes elsewhere.
            u = np.vstack([u, pick])
            y = np.append(y, y.min())
            gp = _refit(gp, u, y)
        ys = _eval(f, space.from_unit(np.array(picks)), hist)
        if trust_region:
            if np.min(ys) < best_before - 1e-3 * abs(best_before):
                succ, fail = succ + 1, 0
            else:
                succ, fail = 0, fail + 1
            if succ >= 3:
                length, succ = min(2.0 * length, 1.6), 0
            elif fail >= fail_tol:
                length, fail = length / 2.0, 0
            if length < 1.0 / 128 and hist.n < budget:
                n = min(n_init, budget - hist.n)
                restart_from = hist.n
                _eval(f, lhs(n, space.lo, space.hi, rng), hist)
                length, succ, fail = 0.8, 0, 0
    return hist


def _refit(gp: _Gp, u, y) -> _Gp:
    z = (y - gp.mean) / gp.scale
    k = gp.amp * _matern52(u, u, gp.ls) + gp.noise * np.eye(len(u))
    c = np.linalg.cholesky(k + 1e-10 * np.eye(len(u)))
    al = np.linalg.solve(c.T, np.linalg.solve(c, z))
    return _Gp(u, gp.ls, gp.amp, gp.noise, gp.mean, gp.scale, c, al)
