# SWAT+ behaviour the port reproduces

SWAT+ is the reference: where it does something surprising, the port does the same, so results
match. Each entry says where it comes from in the Fortran (commit `6f51536`) and where the port
handles it.

## Reading files

- **List-directed input.** Every reader uses `read (unit, *)`. A read that needs more values than
  a line holds goes on into the next lines; values left on the last line are skipped; blank lines
  are skipped; a `/` ends the read; `,,` is a null value that leaves the variable unchanged;
  unquoted text ends at a blank, comma or slash. (`swat-io/src/list.rs`)
- **Integers reject decimals.** gfortran rejects `1.0` where an integer is read. (`list::parse_int`)
- **`file.cio` is read by position.** The name at the start of each row is ignored; a row with
  more names than slots has the extra ones skipped (Ames_sub1's `init` row has 12 for 11); rows
  past the end of the file keep their default file names; `null` turns a slot off.
  (`readcio_read`; `swat-io/src/cio.rs`)
- **`time.sim`.** A `step` of 0 or less is daily; a `day_start` of 0 or less is 1 January
  (`time_read`). A `day_end` of 0 means 31 December of the last year, and a `day_end` past the
  year's length is capped (`time_control`). (`swat-model/src/time.rs`)
- **`print.prt`.** The count of average annual intervals is read, then its record is read again
  with the years (`backspace`). Afterwards the print window is completed: a first day of 0 is 1, a
  last day of 0 is 366, a first year of 0 is the first simulated, a last year of 0 is one past the
  last, and the interval counter starts at the interval. (`basin_print_codes_read`;
  `swat-io/src/print.rs`, `swat-engine/src/run.rs`)
- **A value that doesn't convert ends the read silently.** SWAT+ checks only for end of file
  (`iostat < 0`). Osu_1hru's `parameters.bsn` (written by SWAT+ Editor) has `day_lag_mx` as
  `0.00000`, which gfortran won't read as an integer: the read stops there and `igen` keeps its
  default 5, which changes the random seeds. Ames_sub1's file has `0 0`, so its `igen` is 0.
  (`swat-io/src/basin.rs`)
- **Connection files** are read twice per row, the second time with as many outflows as
  `out_tot` says. (`hyd_read_connect`; `swat-io/src/con.rs`)

## Climate

- **An empty or missing station file stops its variable.** The measured readers (`cli_pmeas`,
  `cli_tmeas`…) leave their station loop at the first file that ends inside its header, so the
  files after it in the list are not read either. A missing file is opened anyway (gfortran
  creates it empty) and behaves the same. Ames_sub1's `tmp.cli` lists `ames.tmp`, which is empty:
  no measured temperature is read. (`swat-io/src/project.rs`, `LoadReport::unread`)
- **Station files are found by binary search.** `cli_staread` looks up each station's gage in the
  station list with `search`, a binary search, so it finds names reliably only in a sorted list.
  In Ames_sub1 the station names `ames.tem` as its temperature gage, which isn't in the list
  `["ames.tmp"]`, so its gage stays 0: an empty gage, always out of bounds, so temperature is
  generated every day. (`swat-engine/src/fortran.rs::search`)
- **Names not found fall back to the first.** A station whose generator name isn't found uses
  generator 1, and an object whose station name isn't found uses station 1 (Ames_sub1's HRUs name
  `wgn_01`, the station is `wgn_sta1`).
- **A gage named `null` is not the same as `sim`.** Only `sim` takes the generator's path; any
  other name that isn't found goes through the measured path with an empty gage, which also ends
  in the generator but draws more random numbers on the way (temperature calls `cli_weatgn` twice).
- **Headers can be wrong.** Osu_1hru's precipitation header says 42 years, but the file holds 43
  (1981–2023). SWAT+ sizes its array from the header but starts filling it at the simulation's
  first year, so this does no harm there. The readers keep the records as written.
- **Short series.** Osu_1hru's solar radiation and wind series end on 2022-304, before the
  simulation ends (2023). SWAT+ checks each day against the file's bounds (`cli_bounds_check`) and
  marks days outside them -98, for the generator. (`swat-engine/src/climate.rs`)
- **Generated solar radiation is the monthly mean.** `cli_slrgen` computes a generated value, then
  its last line replaces it with the month's average from the generator parameters.
- **Precipitation is decided a day ahead.** `cli_precip_control` reads or generates the next day's
  precipitation before `cli_clgen` updates the wet/dry state from today's, so a generated day's
  chance of rain follows the wet/dry state of two days before it.
- **The month table is global.** `cli_initwgn` takes month lengths from SWAT+'s global `ndays`,
  which the temperature reader last set from the year of its last record (leap if divisible by 4),
  or the leap-year table it starts as when no temperature file was read. The simulation loop then
  sets it per year with the full Gregorian rule.
- **Seeds depend on `igen`.** With `igen` not 0, `gcycl` moves the seeds (of the first generator
  only) and shuffles which seed each draw uses (`idg`). `cli_initwgn` uses a copy of the
  precipitation seed for its 1000-value correction factors, so they don't move the seed itself.
- **Missing values** are written -99 in the files.
