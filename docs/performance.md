# Where the time goes: a profile of Pessel

Profiled on 29 October 2026, version 1.0.3 and the two changes below. Pessel: 3,966 HRUs, 1,454
SWAT-DEG channels, measured temperature gages, its own `print.prt`.

## How

`callgrind` (valgrind 3.22) counts the instructions of each function, inclusive of what it calls.
A year of Pessel (2015 only: `time.sim`'s `yrc_end` set to 2015) runs about 50 times slower under
it, so the profile is of one year; reading the project and setting up the HRUs happen once, so for
the full 11 years their share is recomputed below. The build keeps line tables for the names:

    CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p swatplus-rust
    valgrind --tool=callgrind --callgrind-out-file=cg.out target/release/swatplus-rust run Pessel-2015 --out out
    callgrind_annotate --inclusive=yes cg.out | less

Instructions are not time (memory, `exp` and division cost more than an add), so the wall time of
the full run is measured as well, on the same machine (a 2-core x86-64 cloud container, slower than
the laptop of the benchmark: 85 s for what took 30 s there).

## Before: version 1.0.3

33.0 billion instructions for the year:

| Part | Instructions | Share of the year | Share of 11 years |
| --- | ---: | ---: | ---: |
| `Hru::day`: the HRUs' day (water, plants, nutrients, erosion) | 16.4 G | 49.6 % | 61 % |
| `Network::day`: routing units, aquifers, channels, reservoirs | 8.0 G | 24.1 % | 30 % |
|   of which the channels (`Channels::control`) | 6.2 G | 18.7 % | 23 % |
|   of which `ch_temp`'s lagged air temperatures | 3.4 G | 10.3 % | 13 % |
| Output, period sums, regions, copies | 2.5 G | 7.4 % | 9 % |
| Reading the project (`load_project`; the weather 3.5 G) | 4.4 G | 13.3 % | 1.5 % |
| Setting up the HRUs (`init_hrus`) | 1.8 G | 5.6 % | 0.6 % |

Two costs did no simulation at all:

1. **`ch_temp` averaged the same air temperatures for every channel.** For each channel each day,
   SWAT+'s stream temperature averages the gage's maximum and minimum air temperature over four
   lags before today: surface (5 days), lateral (10), groundwater (250) and snow (2). Those
   averages depend on the gage and the day only, yet they were summed again for each of the 1,454
   channels: some 280 million gage reads a year. SWAT+ does the same; the port took it over.
2. **A plant's data was copied three times a day.** `partition`, `leaf_gro` and `leaf_senes` began
   with `let db = self.db.clone()` (to read the plant's `plants.plt` row while changing the plant):
   12.3 million copies a year, each with its strings, so `malloc`, `free` and `memcpy`.

## The changes

1. `ch_temp` keeps the day's four lagged temperatures by gage (`Channels::air_lags`): the first
   channel of a gage on a day sums them (`air_lags`, the same loops in the same order), the others
   read them. Same floats, so the same results.
2. `Plant::db` is an `Arc<PlantDb>`: the routines take a reference-counted handle, not a copy. The
   data is read-only during the run.

Every output table of Pessel 2015–2025 is byte-identical to version 1.0.3's, faithful and with
`--fix-swat`; CI checks the 37 scenarios against SWAT+.

## After

27.1 billion instructions for the year (−18 %; −22 % for the daily loop alone):

| Part | Before | After | Change |
| --- | ---: | ---: | ---: |
| `Hru::day` | 16.4 G | 13.8 G | −16 % |
|   of which the plants | 4.9 G | 2.8 G | −44 % |
| `Network::day` | 8.0 G | 4.7 G | −41 % |
|   of which the channels | 6.2 G | 2.9 G | −53 % |
| `malloc` + `free` | 3.9 G | 2.1 G | −46 % |
| Reading and set-up | 6.2 G | 6.2 G | 0 |

Wall time of Pessel 2015–2025, two runs each, alternating: 88.4 and 84.8 s before, 77.4 and 77.9 s
after, about **10 % faster**. The instructions fell more than the time: what remains is heavier per
instruction (`expf` alone is 10 % of the instructions).

Shares of the 11 years now: the HRUs 64 %, the routing 22 %, output and the rest 11 %, reading and
set-up 3 %.

## What it means for running in parallel

Both engines run on one thread. If the HRUs (64 %) ran on 12 cores and the rest stayed sequential,
Amdahl's law bounds the speed-up at 1 / (0.36 + 0.64 / 12) ≈ 2.4. Routing sub-basins that don't
meet in parallel would raise the bound. A faithful run can't do either: SWAT+ carries some values
from one HRU to the next (`docs/runner.md`, *Speed*), so a parallel engine belongs to `--fix-swat`,
once those values are kept per HRU.

What is left to look at, sequentially: `expf` (10 %), the copies in `run_with` (`memcpy`, 6 %), and
the weather files' reading (3.5 G instructions, once per run; it matters for short runs and for
calibration, which runs a project many times).
