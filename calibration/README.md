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
| `synthetic` | The synthetic experiment |

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
