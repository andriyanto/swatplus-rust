"""The parameters a calibration searches, and how points of the search become parameter sets."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass(frozen=True)
class Param:
    """One calibrated change: `key` is `parameter:change` (`cn2:pctchg`, `epco:absval`), searched
    between `lo` and `hi`."""

    key: str
    lo: float
    hi: float


@dataclass(frozen=True)
class Space:
    params: tuple[Param, ...]

    @property
    def dim(self) -> int:
        return len(self.params)

    @property
    def lo(self) -> np.ndarray:
        return np.array([p.lo for p in self.params])

    @property
    def hi(self) -> np.ndarray:
        return np.array([p.hi for p in self.params])

    def to_unit(self, x) -> np.ndarray:
        return (np.asarray(x, dtype=float) - self.lo) / (self.hi - self.lo)

    def from_unit(self, u) -> np.ndarray:
        return self.lo + np.asarray(u, dtype=float) * (self.hi - self.lo)

    def sets(self, xs) -> list[dict[str, float]]:
        """Parameter sets for `swatplus-rust batch`, one per row of `xs`."""
        return [{p.key: float(v) for p, v in zip(self.params, x)} for x in np.atleast_2d(xs)]


def lhs(n: int, lo, hi, rng: np.random.Generator) -> np.ndarray:
    """A Latin hypercube of `n` points between `lo` and `hi`: each parameter's range cut into n
    strata, one point in each, the strata paired at random across parameters."""
    lo, hi = np.asarray(lo, dtype=float), np.asarray(hi, dtype=float)
    d = len(lo)
    u = (np.argsort(rng.random((n, d)), axis=0) + rng.random((n, d))) / n
    return lo + u * (hi - lo)
