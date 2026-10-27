# Running the engine from another program

How a program (RustGIS's runner, a script, a web page) runs the engine and reads what it gives
back. The engine is a separate program, or a WebAssembly module in the browser, never a library
linked into the caller: it is LGPL-2.1 and its callers needn't be (RustGIS's design, decision
K11). The project goes in as SWAT+'s TxtInOut files; the tables come out as SWAT+'s text tables
or as Parquet; progress and events come out as JSON lines.

## The command line

```text
swatplus-rust run <TxtInOut folder> [--out <folder>] [--fix-swat]
                  [--format text|parquet|both] [--progress json] [--strict]
swatplus-rust inspect <TxtInOut folder>
swatplus-rust compare <SWAT+ table> <our table> [--abs 0.001] [--rel 1e-4]
swatplus-rust --version
```

- `--out`: where the tables go (the TxtInOut folder by default, as SWAT+ does).
- `--format`: SWAT+'s text tables (`text`, the default), Parquet (`parquet`), or both.
- `--progress json`: progress and events as JSON lines on standard output (below); messages
  otherwise go to standard error.
- `--fix-swat`: correct SWAT+'s bugs (`docs/swat-fixes.md`); the results then differ from
  SWAT+'s.
- `--strict`: exit with code 3 when part of the project isn't simulated (a feature the port
  refuses, which otherwise only gives a note and passes that part's water on). The tables are
  still written in full (Parquet too), and the error comes after them.

Exit codes: 0 done, 1 an error (the project can't be read or run, a table can't be written),
2 wrong arguments, 3 part of the project not simulated (`--strict`).

### Events

With `--progress json`, each line on standard output is one JSON object:

```text
{"event":"start","version":"1.0.0","days":1096,"first":"2015-01-01","last":"2017-12-31"}
{"event":"progress","done":548,"days":1096,"date":"2016-07-02"}
{"event":"note","text":"channels not routed (...); they pass their inflow on"}
{"event":"table","file":"hru_wb_day.parquet","rows":126040}
{"event":"table","file":"hru_wb_day.txt"}
{"event":"done","days":1096,"seconds":6.917}
{"event":"error","text":"..."}
```

`progress` comes at each whole percent of the simulated days (`done` days finished, `date` the
day starting); `table` for each file written, with its rows for Parquet; `done` or `error` last.

## The Parquet tables

Each text table SWAT+ writes (`<table>_<day|mon|yr|aa>.txt`, as `print.prt` asks) becomes
`<table>_<period>.parquet` with the same columns, in the same order, under the same names (a
repeated name gets `_2`):

- the keys (`jday`, `mon`, `day`, `yr`, `unit`, `gis_id`, `tstep`, `year`, `plant_no`) as
  32-bit integers;
- the names (`name`, `type`, `plantnm`, `plant_name`) as UTF-8 strings;
- every value as a 32-bit float: the number the text table shows (SWAT+'s own precision), NaN
  where SWAT+ prints NaN or asterisks.

The files are plain encoded, uncompressed, one row group per million rows. DuckDB, Arrow,
Polars and DuckDB-Wasm read them; CI checks every table of Osu_1hru with DuckDB and pyarrow
against its text (`scripts/check-parquet.py`).

## The WebAssembly module

`crates/swat-wasm` builds `swat_wasm.wasm` (`cargo build --release --target
wasm32-unknown-unknown -p swat-wasm`), a module with no JavaScript glue:

| export | |
|---|---|
| `memory` | the module's memory |
| `swat_alloc(len) -> ptr` | a buffer of `len` bytes to fill |
| `swat_free(ptr, len)` | frees a buffer from `swat_alloc` or `swat_run` |
| `swat_run(ptr, len, flags) -> ptr` | runs the scenario in the tar archive at `ptr` |

The scenario is a tar archive of its TxtInOut files (in one folder or none). `flags`: 1 Parquet,
2 text (both when neither), 4 `--fix-swat`. `swat_run` returns a buffer of 4 bytes of
little-endian length and then a tar archive of the tables and `run.json`:

```json
{"version":"1.0.0","ok":true,"error":null,"notes":[],"tables":["hru_wb_day.txt", ...]}
```

The module imports `env.swat_progress(done, days)`, called at each whole percent. Run it in a
Web Worker; `scripts/wasm-run.mjs` shows the calls (Node or a browser). CI runs Osu_1hru through
it and compares every table with the native run's.

**The module's results can differ from the native program's in the last bits**: in WebAssembly
the math functions (`exp`, `ln`, `powf`, the trigonometric ones) are Rust's own (a port of
musl's), not glibc's, and a few of them round the last bit of some single-precision results
differently. On Osu_1hru over 14 years, 17 of its 18 tables are identical; the routing unit's
average annual table, printed with 9 significant digits, differs in two of its values by 1 and 2
units in the last place of a 32-bit float. Every table matches within the tolerance the port is
held to (decision K12). The native program on Linux uses glibc, as the Fortran reference does;
on macOS and Windows the system's math library can differ the same way.

## Speed

The engine runs on one thread. The whole of Pessel (3,966 HRUs, 1,454 channels) takes 6.4 s a
simulated year on one core, against 16.7 s for a Release build of SWAT+ with gfortran (2.6 times
faster).
Running HRUs in parallel can't give SWAT+'s results: several of SWAT+'s variables carry one
HRU's values into the next one's day (a decision table's random numbers, the plants' nutrient
demand, the enrichment ratio, the phosphorus fractions of sediment, a septic system's state),
so the order matters. A parallel run belongs to the corrected mode (`--fix-swat`), once those
are made per HRU.
