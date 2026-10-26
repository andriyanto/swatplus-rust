# swatplus-rust

SWAT+ (Soil and Water Assessment Tool Plus) translated from Fortran into Rust, to run natively
and in the browser (WebAssembly), and to be run by [RustGIS](https://github.com/andriyanto/rustgis).

Status: **phase 2 done (stages 2a, 2b-1, 2b-2, management M1 and M2).** The daily weather matches SWAT+ exactly: precipitation,
maximum, minimum and mean temperature, solar radiation, wind speed and relative humidity, measured
and generated, are identical to the Fortran engine's on every day of both reference scenarios
(Osu_1hru 2013–2023, Ames_sub1 1975–2020, 12 HRUs). The HRU water balance of bare soil (no plants,
no management) matches too: all 28 columns the port writes to `hru_wb_day` (snow, soil
temperature, soil evaporation, curve-number runoff and its lag, percolation, lateral flow, soil
water) are identical on all 201 624 HRU-days of `Ames_bare`. With a pasture growing and the
original carbon model (`Ames_past0`) and with the Century carbon model Ames uses
(`Ames_past`), all four daily HRU tables — water balance, plants, nutrient balance and losses,
77 columns — are identical too, and so are they with corn under Ames's scheduled management
(planting, harvest, tillage, irrigation, fertilizer: `Ames_corn`, `Ames_cornf`), and with
**Ames_sub1 itself**, unchanged: corn and soybean in one community, its 305 scheduled
operations, irrigation by decision table and custom soil layers. CI checks this on every push.

Phase 3 (routing units, aquifers, SWAT-DEG channels) works on two real projects from West Sumatra, Indonesia, in `testdata/`: `Batanghari` (upland, 284 channels, 1,858 HRUs) and `Pessel` (coastal, 1,454 channels, 3,966 HRUs, peat and paddy), each with its Fortran reference; the stages are in [`docs/phase3.md`](docs/phase3.md). Stage 3a-0 is done: Batanghari's 1,858 HRUs (`Bh_land`: Hargreaves PET, urban and wetland HRUs as forest) match SWAT+ on all four daily HRU tables over nine years.

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
| `swat-engine` | The simulation. So far the clock, the daily weather (measured and generated), the print schedule, and HRUs: water balance, one plant's growth, soil nutrients (original and Century carbon models), erosion, and dated management. |
| `swatplus-rust` | The command line program: `inspect`, `run`, `compare`. |

No external dependencies, so the core builds anywhere, WebAssembly included.

```sh
cargo test
cargo run -p swatplus-rust -- inspect testdata/Osu_1hru
cargo run -p swatplus-rust -- run testdata/Osu_1hru --out out
cargo run -p swatplus-rust -- compare <SWAT+ hru_pw_day.txt> out/hru_pw_day.txt
```

The SWAT+ Fortran reference, built from the commit this port follows, is on the `fortran-bin`
branch (`SWAT+ Fortran build` workflow).

## Faithful to SWAT+, quirks included

The aim is the same results as SWAT+, so its behaviour is reproduced even where it surprises.
What has been found so far is in [`docs/swat-quirks.md`](docs/swat-quirks.md).

Next: routing (phase 3a, the command loop with routing units and aquifers) — see [`docs/phase3.md`](docs/phase3.md). For a project the port can't
simulate yet (other decision table conditions and actions, other operations, other PET
methods…), `run` says why and writes the weather and precipitation only.

## License

GNU Lesser General Public License, version 2.1 only (LGPL-2.1-only), the license of SWAT+, as this
is a translation of it. See [NOTICE](NOTICE) for attribution: SWAT+ is developed by USDA-ARS and
Texas A&M AgriLife Research. This project is not affiliated with or endorsed by them.
