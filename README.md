# swatplus-rust

SWAT+ (Soil and Water Assessment Tool Plus) translated from Fortran into Rust, to run natively
and in the browser (WebAssembly), and to be run by [RustGIS](https://github.com/andriyanto/rustgis).

Status: **phase 1.** The daily weather matches SWAT+ exactly: precipitation, maximum, minimum and
mean temperature, solar radiation, wind speed and relative humidity, measured and generated, are
identical to the Fortran engine's on every day of both reference scenarios (Osu_1hru 2013–2023,
Ames_sub1 1975–2020, 12 HRUs). CI checks this on every push (`Validate against SWAT+`). The design, decisions and
phases are in RustGIS: [`docs/swat/design.md`](https://github.com/andriyanto/rustgis/blob/swat-plus-design/docs/swat/design.md).

## Goal

Every daily output matches SWAT+ Fortran (commit `6f51536`) within tolerance on the reference
scenarios. The Fortran baseline is built and run by RustGIS's `SWAT+ baseline` workflow; outputs
are compared value by value with `tools/swat/compare_outputs.py` there.

## Layout

| Crate | Job |
| --- | --- |
| `swat-model` | The data of a project: `file.cio`, simulation period, print settings, spatial objects and their outflows, weather stations, weather generator, measured climate series. No I/O. |
| `swat-io` | Readers for TxtInOut files. Each follows the SWAT+ routine that reads the file, with Fortran list-directed input rules, from a folder or from files in memory (the browser). |
| `swat-engine` | The simulation. So far the clock, the daily weather (measured and generated) and the print schedule. |
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

## License

GNU Lesser General Public License, version 2.1 only (LGPL-2.1-only), the license of SWAT+, as this
is a translation of it. See [NOTICE](NOTICE) for attribution: SWAT+ is developed by USDA-ARS and
Texas A&M AgriLife Research. This project is not affiliated with or endorsed by them.
