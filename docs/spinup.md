# How long a spin-up, and can calibration runs share one

A model starts from a guessed state: soil water, the aquifers' water table, the other stores. The
first years of a run (the warm-up, `print.prt`'s `nyskip`) let that state adjust to the climate
and are not printed. A calibration runs the model hundreds of times, so two questions matter:
how many warm-up years are enough, and whether one warm-up can serve every parameter set.
`docs/roadmap.md` had the second as "to be tested, not assumed" (M2). Studied on 24 November 2026
with `python -m swatcal.spinup` (`calibration/swatcal/spinup.py`).

## The tools

- **`Simulation::calibrate`** (`swat-engine/src/run.rs`) applies calibration changes to a
  simulation as it stands, between two days: the HRUs' and aquifers' parameters change, their
  water, plants and nutrients stay. It uses the same functions as the set-up (`cal_conditions`,
  `calibrate_aquifers`). It is not a SWAT+ step (SWAT+ applies `calibration.cal` once, before the
  first day), so it serves calibration and studies, not runs compared with SWAT+. It refuses a
  change that would set the state up again: a soil layer's depth, bulk density or available
  water (`soil_awc_init` recomputes the soil water), an aquifer's specific yield or initial
  values, the basin's `surlag`. Tests (`tests/calibration.rs`): changes applied before the first
  day give the tables of a run set up with them; changes applied mid-run leave the days before
  unchanged; `awc` mid-run is refused.
- **`run_batch_shared` / `swatplus-rust batch --shared-spinup`** (`swat-engine/src/batch.rs`)
  run the project with its own calibration once through the warm-up years, then each parameter
  set from a copy of that state with its changes applied. Tests (`tests/batch.rs`): a set without
  changes is the project's own run, day for day; a set with changes is close to, not the same as,
  its run with its own warm-up.

## Part A: how long until the start no longer matters

Three copies of each project start from a dry, the usual and a wet state and print from the
first day; years 2015–2025, outlet flow daily (the channel with the largest mean flow).

| State | Soil water at the start (`parameters.bsn` `sw_init`, share of field capacity) | Aquifer water table (`aquifer.aqu` `dep_wt`) |
| --- | --- | --- |
| Dry | 0.05 | 95 % of the aquifer's depth (nearly empty) |
| Usual | the project's (0: from annual rainfall) | the project's |
| Wet | 1.0 | 5 % of the aquifer's depth (nearly full) |

Wet minus dry, as a share of the usual run, year by year (low flow: the flow exceeded on 90 % of
the year's days; NSE: the dry run's daily flow against the wet run's):

| Year | Batanghari annual flow | low flow | daily NSE | Pessel annual flow | low flow | daily NSE |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2015 | 40.5 % | 99.8 % | 0.17 | 55.5 % | 96.7 % | 0.06 |
| 2016 | 3.9 % | 19.4 % | 0.95 | 3.9 % | 2.1 % | 0.60 |
| 2017 | 1.3 % | −6.3 % | 0.95 | 0.6 % | 1.5 % | 0.59 |
| 2018 | 0.4 % | 7.7 % | 0.98 | 0.5 % | −1.4 % | 0.73 |
| 2019 | −1.1 % | −3.7 % | 0.96 | −0.2 % | 8.9 % | 0.48 |
| 2020 | 0.2 % | −4.2 % | 0.99 | −0.1 % | −4.1 % | 0.57 |
| 2021 | −0.1 % | 1.8 % | 0.98 | 0.0 % | −0.2 % | 0.96 |
| 2022 | 0.3 % | 0.6 % | 0.96 | −0.1 % | −9.0 % | 0.75 |
| 2023 | −0.7 % | −5.6 % | 0.99 | 0.0 % | −1.5 % | 0.99 |
| 2024 | 0.1 % | −8.7 % | 0.99 | −0.1 % | −7.9 % | 0.82 |
| 2025 | 0.0 % | −0.4 % | 0.93 | 0.1 % | 0.5 % | 0.83 |

- **Soil water forgets its start within the first year** in both basins: the three runs' basin
  soil water at the end of 2015 differs by 1 mm or less.
- **The annual volume needs two years**: the spread falls from 40–56 % in the first year to 4 %
  in the second, and stays within about ±1 % from the third. The projects' two warm-up years are
  enough for volumes and for calibrating on them.
- **The aquifers never forget.** Their storage (`basin_aqu_yr` `stor`) keeps the gap it started
  with: in Batanghari the dry run's rises by about 50 a year, to 1,062 at the end of 2025, the
  usual run's to 3,298, the wet run's stays near 3,390; Pessel is alike (1,105, 3,352, 3,395).
  The usual run's storage still grows each year: under these parameters the aquifers are not in
  balance with the climate after 11 years.
- **So the start keeps showing in daily and low flows**: low flows still differ by up to 9 %
  between starts after ten years, and in Pessel the daily flows of the dry and wet runs agree
  only to NSE 0.48–0.99. A calibration on low flows (Q80, Q90) or daily shape inherits the guessed
  aquifer state, however long the warm-up the period allows.

## Part B: one shared warm-up for all parameter sets

Twenty parameter sets (a Latin hypercube over M4's space: CN2 −25…+25 %, baseflow alpha
−60…+60 %, EPCO 0.1–1, deep seepage 0.001–0.4) and M4's truth (CN2 +8 %, alpha −20 %, EPCO
0.6, deep seepage 0.15), run on the whole projects (2015–2025, two warm-up years) twice: each
with its own warm-up, and all from one shared warm-up (`--shared-spinup`). Two runs at a time on
two cores.

| | Batanghari | Pessel |
| --- | ---: | ---: |
| Daily KGE, shared against own (lowest, median) | 0.970, 0.980 | 0.866, 0.921 |
| Bias over the 9 printed years | −0.2 to +0.2 % | −0.3 to +0.4 % |
| Bias in the first printed year | −2.8 to +2.1 % | −3.1 to +3.7 % |
| Change of each set's KGE against the truth (range; median of the size) | −0.011 to +0.007; 0.002 | −0.030 to +0.020; 0.010 |
| Rank correlation of the sets' KGE against the truth, shared vs own | 0.99 | 0.77 |
| Best set, own / shared | s004 / s004 | s006 / s016 |
| Time for 21 runs, own / shared | 163 s / 139 s (−14 %) | 432 s / 364 s (−16 %) |

- **Batanghari would tolerate a shared warm-up**: volumes within 0.2 %, the same best set, ranks
  nearly unchanged.
- **Pessel would not**: the ranking of the sets changes (0.77) and another set comes out best.
  Its flows carry more of the aquifers' state (part A), and a warm-up with other parameters
  leaves the aquifers elsewhere.
- **The gain is small**: two of eleven years, 14–16 % of the time.

## What this means

1. Keep each run's own warm-up in a calibration (the default of `batch`); `--shared-spinup` is
   there for quick exploration and must be checked against own warm-ups before it is trusted.
2. Two warm-up years are enough for volumes and soil water in these basins.
3. The aquifers' initial state is not forgotten in 11 years. For low flows, start the aquifers
   from the state at the end of a long run (or from storages measured or estimated
   independently), and treat the initial water table as uncertain; a run that starts the
   aquifers from a guess carries that guess into the low flows it is calibrated on.
4. The usual runs' aquifers still fill year after year: the aquifer parameters of these projects
   (recharge against outflow, revap and deep seepage) deserve a look before calibrating low flows.
