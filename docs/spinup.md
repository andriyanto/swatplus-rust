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
- **The shallow aquifers need two to three years; the deep ones never forget.** The basin's
  aquifer storage (`basin_aqu_yr` `stor`) keeps the gap it started with to 2025 (Batanghari: dry
  1,062, usual 3,298, wet about 3,390; Pessel: 1,105, 3,352, 3,395), but that sum is dominated by
  the deep aquifers (below). Aquifer by aquifer in Batanghari, the shallow aquifers' storage of
  the wet and dry runs differs by 226 mm (median) at the end of 2015, 55 mm in 2016, 8 mm in 2017
  and at most 10 mm from 2020; the two deep aquifers keep their gap (656 and 886 mm for the dry
  run in 2025 against 3,014 and 3,021 for the wet one).
- **Some of the start still shows in daily and low flows**: low flows differ by up to 9 % between
  starts in single years after the third, without a trend, and in Pessel the daily flows of the
  dry and wet runs agree only to NSE 0.48–0.99. The deep aquifers send no water to the rivers
  (below), so this comes from the shallow ones; a likely cause is that their return flow switches
  on and off at a water table threshold (`flo_min`), where differences of a few millimetres
  matter. Not proven.

## The aquifers

`python -m swatcal.aquifers` prints every aquifer each year and sums their water balance. Both
projects have the SWAT+ editor's aquifers: shallow ones (Batanghari 42, Pessel 230) with the
editor's defaults (baseflow alpha 0.05, revap 0.02, deep seepage 0.05 of the recharge, specific
yield 0.05, return flow when the water table is within 3 m, `dep_bot` 10 m), and deep ones
(`aqu_deep…`; Batanghari 2, Pessel 24; `dep_bot` 100 m, return flow only when full) that take the
shallow aquifers' deep seepage. Means over the aquifers, mm a year, 2015–2025:

| | Batanghari shallow | Batanghari deep | Pessel shallow | Pessel deep |
| --- | ---: | ---: | ---: | ---: |
| Recharge | 1,023 | 59 | 1,102 | 63 |
| Return flow | 943 | 7 | 1,020 | 10 |
| Deep seepage | 51 | 0 | 55 | 0 |
| Revap | 26 | 0 | 25 | 0 |
| Storage change 2015–2025, mm (min, median, max) | −12, 3, 17 | 490, 546, 601 | −88, 13, 25 | 0, 602, 607 |

- **The shallow aquifers are in balance**: recharge equals return flow, seepage and revap within
  a few millimetres a year, and their storage barely moves over 11 years.
- **The deep aquifers only fill.** They receive the deep seepage (5 % of the shallow recharge),
  give nothing back until full, and their `aquifer.con` sends their flow nowhere (`out_tot` 0):
  in Batanghari one of the two is full by 2025 and its return flow (77 mm that year) leaves the
  model. That water (about 5 % of the recharge) is lost to the rivers by design, which a
  calibration on volume must keep in mind.
- **SWAT+'s return flow above the threshold does not depend on storage** (`aqu_1d_control`: the
  day's return flow is the recharge smoothed by `alpha_bf`), so nothing pulls a shallow
  aquifer's storage back to a level: a difference in storage decays only through the threshold.
  That is why the start lingers, and why calibrating the shallow aquifers (alpha, `flo_min`,
  deep seepage) needs low flows observed, not only volumes.

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
  Its daily flows also kept more of their start in part A, and a warm-up with other parameters
  leaves the aquifers elsewhere.
- **The gain is small**: two of eleven years, 14–16 % of the time.

## What this means

1. Keep each run's own warm-up in a calibration (the default of `batch`); `--shared-spinup` is
   there for quick exploration and must be checked against own warm-ups before it is trusted.
2. Two warm-up years are enough for volumes and soil water in these basins.
3. The shallow aquifers need two to three years; a calibration on low flows (Q80, Q90) or
   daily shape should use three warm-up years, or start the aquifers from the state at the end of
   a long run, and treat the initial water table as uncertain.
4. The deep aquifers fill without end and lose their water to nowhere: summed basin aquifer
   storage is not a sign of drift in these projects, but about 5 % of the recharge leaves the
   rivers through them. The aquifer parameters are the editor's defaults everywhere; alpha,
   `flo_min` and deep seepage are the ones to calibrate once observed low flows (stage records)
   are in hand.
