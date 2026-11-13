"""swatcal: stage-based calibration of SWAT+ with swatplus-rust (M3 and M4 in docs/roadmap.md).

- `stage`: rating curves, synthetic gauge records, a rating curve fitted to each run;
- `metrics`: NSE, KGE, PBIAS, RSR, Spearman;
- `objective`: the score of one run against a stage record and the water balance;
- `runner`: parameter sets through `swatplus-rust batch`, series read back;
- `synthetic`: the synthetic experiment (python -m swatcal.synthetic).
"""

from .metrics import Kge, kge, nse, pbias, rsr, spearman
from .objective import Observations, Score, evaluate
from .stage import Rating, RatingFit, fit_rating, synthetic_stage

__all__ = [
    "Kge",
    "Observations",
    "Rating",
    "RatingFit",
    "Score",
    "evaluate",
    "fit_rating",
    "kge",
    "nse",
    "pbias",
    "rsr",
    "spearman",
    "synthetic_stage",
]
