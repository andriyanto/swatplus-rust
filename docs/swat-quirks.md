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
  with the years (`backspace`). (`basin_print_codes_read`; `swat-io/src/print.rs`)
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
  `["ames.tmp"]`, so its gage stays 0. The list order is kept in `Project::station_lists` for the
  engine to reproduce this. *(Engine, to do.)*
- **Headers can be wrong.** Osu_1hru's precipitation header says 42 years, but the file holds 43
  (1981–2023). SWAT+ sizes its array from the header but starts filling it at the simulation's
  first year, so this does no harm there. The readers keep the records as written.
- **Short series.** Osu_1hru's solar radiation and wind series end on 2022-304, before the
  simulation ends (2023). SWAT+ checks each day against the file's bounds (`cli_bounds_check`) and
  marks days outside them -98, for the generator. *(Engine, to do.)*
- **Missing values** are written -99 in the files.
