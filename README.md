# swatplus-rust

SWAT+ (Soil and Water Assessment Tool Plus) translated from Fortran into Rust, to run natively
and in the browser (WebAssembly), and to be run by [RustGIS](https://github.com/andriyanto/rustgis).

Status: **engine 1.0.** The port runs SWAT+ projects from the land to the outlet — HRUs (weather,
water balance, plants, nutrients, carbon, erosion, management by date, heat units and decision
tables, paddies, wetlands, tile drains and other structures, septic systems, urban areas),
landscape routing between HRUs, routing units, aquifers, SWAT-DEG channels, reservoirs and point
sources — daily or by sub-daily steps (Green-Ampt or the curve number with unit hydrographs),
and its output tables are those of SWAT+ commit `6f51536`, bit for bit, on two SWAT+ examples
and two real projects from West Sumatra (`Batanghari`, 1,858 HRUs; `Pessel`, 3,966 HRUs) and 31
variants that exercise each feature. CI runs both engines on every push and compares every
table. What isn't ported is refused by name (`run` says why), never guessed.

The design, decisions and phases are in RustGIS: [`docs/swat/design.md`](https://github.com/andriyanto/rustgis/blob/swat-plus-design/docs/swat/design.md).

## Goal

Every daily output matches SWAT+ Fortran (commit `6f51536`) on the reference scenarios, within a
relative difference of 1e-4 or one unit in the last printed decimal (decision K12). The
`Validate against SWAT+` workflow runs both engines and compares them with `swatplus-rust compare`.

## Layout

| Crate | Job |
| --- | --- |
| `swat-model` | The data of a project: `file.cio`, basin codes and parameters, simulation period, print settings, spatial objects and their outflows, weather, HRUs, soils, and the database tables (topography, hydrology, land use, curve numbers, snow…). No I/O. |
| `swat-io` | Readers for TxtInOut files. Each follows the SWAT+ routine that reads the file, with Fortran list-directed input rules, from a folder or from files in memory (the browser). |
| `swat-engine` | The simulation. So far the clock, the daily weather (measured and generated), the print schedule, HRUs (water balance, plant growth, soil nutrients with the original and Century carbon models, erosion, dated and decision-table management), and the objects they feed in SWAT+'s command order: routing units, aquifers and SWAT-DEG channels. |
| `swat-output` | The output: SWAT+'s text tables, the same tables as Parquet, tar archives, progress as JSON lines. |
| `swat-wasm` | The engine as a WebAssembly module for the browser: a scenario's tar archive in, its tables out. |
| `swatplus-rust` | The command line program: `inspect`, `run`, `compare`. |

No external dependencies, so the core builds anywhere, WebAssembly included.

```sh
cargo test
cargo run --release -p swatplus-rust -- inspect testdata/Osu_1hru
cargo run --release -p swatplus-rust -- run testdata/Osu_1hru --out out --format both --progress json
cargo run --release -p swatplus-rust -- compare <SWAT+ hru_pw_day.txt> out/hru_pw_day.txt
```

How another program runs it (the command line, Parquet tables, JSON progress, the WebAssembly
module) is in [`docs/runner.md`](docs/runner.md).

The SWAT+ Fortran reference, built from the commit this port follows, is on the `fortran-bin`
branch (`SWAT+ Fortran build` workflow).

## Faithful to SWAT+, quirks included

The aim is the same results as SWAT+, so its behaviour is reproduced even where it surprises.
What has been found so far is in [`docs/swat-quirks.md`](docs/swat-quirks.md).

Rice paddies (weir, ponding, puddling) and their validation are in [`docs/paddy.md`](docs/paddy.md); reservoirs and point sources (recall objects) in [`docs/reservoirs.md`](docs/reservoirs.md); nitrogen deposition and Muskingum routing in [`docs/atmo.md`](docs/atmo.md); tile drains and other structural practices in [`docs/structures.md`](docs/structures.md); the dynamic soil P model and urban loads by the USGS regressions in [`docs/nutrients.md`](docs/nutrients.md); hourly (sub-daily) steps with Green-Ampt infiltration and hourly channel flows in [`docs/subdaily.md`](docs/subdaily.md); water routed from HRU to HRU across the land in [`docs/routing.md`](docs/routing.md); the other features of engine 1.0 in [`docs/engine1.md`](docs/engine1.md).

Which output tables it writes, and how SWAT+ sums them into months, years and average annual values, is in [`docs/outputs.md`](docs/outputs.md).

The quirks that are plainly bugs can be corrected instead with `run <folder> --fix-swat`; the
results then differ from SWAT+'s. They are listed in [`docs/swat-fixes.md`](docs/swat-fixes.md).

Batanghari and Pessel match SWAT+ bit for bit (phases 3d and 3e, [`docs/phase3.md`](docs/phase3.md)), and so does Osu_1hru, SWAT+'s rice paddy example: weir, ponding irrigation, puddling, manure and calibration, on every table over 14 years (phase 4). For a project the port can't
simulate (a feature it refuses), `run` says why and passes that part's water on, or fails with
`--strict`.

## License

GNU Lesser General Public License, version 2.1 only (LGPL-2.1-only), the license of SWAT+, as this
is a translation of it. See [NOTICE](NOTICE) for attribution: SWAT+ is developed by USDA-ARS and
Texas A&M AgriLife Research. This project is not affiliated with or endorsed by them.
