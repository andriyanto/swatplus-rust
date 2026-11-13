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
| `compare` | The optimisers on a synthetic world with the same run budget, under three states of knowledge (M4) |

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
    --out cmp --years 2015-2019 --budget 80
```

The truth is the project run with known values of four parameters (CN2 +8 %, baseflow alpha −20 %,
EPCO 0.6, deep seepage 0.15). Each optimiser gets the same number of runs, under three states of
knowledge: the stage record alone, with satellite ET, and with six discharge measurements. It reports
the best score, how far its parameters are from the truth, and how good its discharge really is
(`results.csv`, `convergence.json`, `convergence.png`).

First run, Batanghari's outlet, 2015–2019 (two years of warm-up), 80 runs each, **one seed**:

| knowledge | method | best score (truth: 0.0016 / 0.0016 / 0.070) | parameter error | true flow KGE | true flow PBIAS |
| --- | --- | ---: | ---: | ---: | ---: |
| stage | random | 0.050 | 0.21 | 0.943 | −4.3 % |
| stage | SUFI-2 | 0.023 | 0.15 | 0.974 | −1.7 % |
| stage | Bayesian | 0.046 | 0.29 | 0.968 | −1.3 % |
| stage + ET | random | 0.050 | 0.21 | 0.943 | −4.3 % |
| stage + ET | SUFI-2 | 0.023 | 0.15 | 0.974 | −1.7 % |
| stage + ET | Bayesian | 0.051 | 0.22 | 0.939 | −4.8 % |
| stage + ET + gaugings | random | 0.081 | 0.30 | 0.964 | −1.2 % |
| stage + ET + gaugings | SUFI-2 | 0.068 | 0.17 | 0.988 | −0.0 % |
| stage + ET + gaugings | Bayesian | 0.083 | 0.38 | 0.969 | −0.9 % |

What it suggests, to be checked over several seeds before anything is concluded:

- **Six gaugings matter most for the volume**: with them, every method's flow bias is within 1.2 %,
  and SUFI-2 finds the deep seepage (0.152 against 0.15).
- **Satellite ET adds nothing here**, as the parameters searched hardly change ET.
- **80 runs is far from enough**: no method comes near the truth's own score.
- **Bayesian optimisation stalls** after about 30 runs, where it beat random search clearly on a smooth
  test function. The score of a real run is not smooth (a rating curve fitted on a grid, reading noise):
  the Gaussian process' noise model, the exploration and the scale of the score are what to examine.
