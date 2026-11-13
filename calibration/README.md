# swatcal

Stage-based calibration of SWAT+ with swatplus-rust (the M track of [`docs/roadmap.md`](../docs/roadmap.md)):
calibrating a basin where the gauges record water level (stage) but not discharge. Python with numpy;
the model runs are made by `swatplus-rust batch` ([`docs/calibration.md`](../docs/calibration.md)).

| Module | What |
| --- | --- |
| `stage` | Rating curves `Q = a (H − h0)^b` both ways; a synthetic gauge record from a model's discharge (reading error, rounding, missing days); `fit_rating`, the curve that best turns one run's discharge into the observed stage |
| `metrics` | NSE, KGE (with r, alpha, beta), PBIAS, RSR, Spearman's rank correlation |
| `objective` | The score of one run: KGE of the stage through the fitted curve, Spearman of discharge against stage, the bias of mean annual ET against satellite ET, and the error of the curve at measured discharges |
| `runner` | Parameter sets through `swatplus-rust batch`, the series read back |
| `synthetic` | The synthetic experiment (M3) |
| `params` | The searched space (`parameter:change` between bounds), Latin hypercube sampling |
| `optimize` | Random search; SUFI-2 (Abbaspour et al., 2007: Latin hypercube rounds, ranges narrowed around the behavioural runs); Bayesian optimisation (a numpy Gaussian process, Matérn 5/2, expected improvement, batches by constant liar) |
| `compare` | The optimisers on a synthetic world with the same run budget, under three states of knowledge, over several seeds (M4) |

```bash
pip install numpy pytest
cargo build --release -p swatplus-rust
cd calibration
python -m pytest -q tests
python -m swatcal.synthetic --exe ../target/release/swatplus-rust --project <TxtInOut> --channel 1 --out synth
```

## The synthetic experiment

The project as it is plays the truth. Its discharge at the chosen channel becomes a gauge record
through a known rating curve (b = 1.8, h0 = 0.5 m, a set so the median flow reads 1.5 m above h0),
with 2 cm reading errors, rounding to the centimetre and 5 % of days missing. Each parameter is then
moved alone (`--sweep`), each run is scored against the synthetic record, and the score is set beside
how far the run's discharge really is from the truth's (`summary.md`, `results.csv`).

What it shows from the start is the problem the research is about: **stage fixes the timing and the
shape of the hydrograph, not its volume.** A run that loses water to the deep aquifer still fits the
stage record well, because the fitted curve's `a` shrinks to match; and evapotranspiration doesn't
see it either, since that water never evaporates. Only measured discharges do. How much each part of
the score should weigh, and how few gaugings are enough, are what M4 examines.

Two practical points the experiment brought out:

- **"No change" is not "pctchg 0".** SWAT+ holds every changed value to `cal_parms.cal`'s range, so a
  curve number of 98 "changed" by 0 % becomes 95. `run_sets` therefore runs sets that change
  different parameters separately, and never sends a parameter a set doesn't name.
- **Curve number, ESCO and baseflow alpha move the timing, hardly the volume**, in a humid basin; the
  volume moves with what removes water from the basin (deep seepage) or adds evaporation.

## Comparing the optimisers (M4)

```bash
python -m swatcal.compare --exe ../target/release/swatplus-rust --project <TxtInOut> --channel 1 \
    --out cmp --years 2015-2019 --budget 80 --seeds 1,2,3
```

The truth is the project run with known values of four parameters (CN2 +8 %, baseflow alpha −20 %,
EPCO 0.6, deep seepage 0.15). Each optimiser gets the same number of runs, under three states of
knowledge: the stage record alone, with satellite ET, and with six discharge measurements. It reports
the best score, how far its parameters are from the truth, and how good its discharge really is
(`results.csv`, `convergence.json`, `convergence.png`).

Batanghari's outlet, 2015–2019 (two years of warm-up), 80 runs per search, **three seeds**
(`--seeds 1,2,3`), medians over the seeds with the range of the best score. "Bayesian, global" is
the Gaussian process over the whole space; "Bayesian, trust region" keeps its next runs in a box
around the best run so far, grown after successes and shrunk after failures (Eriksson et al., 2019).
Every rating curve is fitted smoothly in the flows (no grid jumps).

| knowledge | method | best score (truth: 0.0016 / 0.070) | parameter error | true flow KGE | true flow PBIAS |
| --- | --- | ---: | ---: | ---: | ---: |
| stage | random | 0.022 (0.012–0.034) | 0.26 | 0.976 | −1.6 % |
| stage | SUFI-2 | 0.042 (0.017–0.049) | 0.22 | 0.963 | +2.1 % |
| stage | Bayesian, global | 0.026 (0.016–0.031) | 0.30 | 0.964 | +2.6 % |
| stage | Bayesian, trust region | 0.025 (0.015–0.041) | 0.34 | 0.948 | +4.7 % |
| stage + ET + gaugings | random | 0.068 (0.067–0.082) | 0.27 | 0.976 | −0.9 % |
| stage + ET + gaugings | SUFI-2 | 0.073 (0.069–0.077) | 0.25 | 0.971 | +1.5 % |
| stage + ET + gaugings | Bayesian, global | 0.074 (0.074–0.077) | 0.22 | 0.964 | −2.3 % |
| stage + ET + gaugings | Bayesian, trust region | 0.082 (0.074–0.082) | 0.31 | 0.966 | +0.1 % |

What the seeds show:

- **No method wins at 80 runs.** The spread between seeds of one method (random: 0.012 to 0.034) is as
  wide as the differences between methods, and each method is best on some seed. The first, one-seed
  run (SUFI-2 ahead) was one draw of this spread.
- **The trust region does not help on the real model**, though it follows a narrow valley better on a
  test function (median 0.26 against 0.44 over six seeds, `tests/test_optimize.py`). With four
  parameters and 80 runs, a Latin hypercube already covers the space about as densely as the
  Gaussian process can exploit.
- **A good score is not a good parameter set.** With gaugings, random search scores 0.068, below the
  truth's own 0.070, while its parameters are 0.27 away: the gauge's reading noise is being fitted.
  The flow is still good (KGE ≥ 0.96, bias within 2.5 % for every method), as different parameter
  sets give nearly the same discharge (equifinality).
- **What to change is the problem, not the optimiser**: more parameters (where a model of the score
  pays off), more runs, or a score less sensitive to reading noise. Random search, being simplest and
  using every core at once, is the reasonable default for a few parameters.
