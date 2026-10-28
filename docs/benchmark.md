# Benchmark: swatplus-rust vs SWAT+ Fortran

How fast the port runs against the SWAT+ Fortran program it follows, on the same machine and the
same scenarios. `scripts/bench.sh` runs both engines on each scenario of `testdata/`: one warm-up
run each, then `REPS` runs alternating the engines, each in a fresh copy of the scenario, with
the scenario's own `time.sim` and `print.prt`. It records per run the wall time, the CPU time,
the peak memory (`scripts/bench-run.py`) and the files written, the machine and both programs'
versions, and writes `bench-results/<date>/` (`runs.csv`, `machine.txt`, `summary.md`).

```sh
FORTRAN=$(scripts/build-fortran.sh) scripts/bench.sh                  # all four scenarios, 5 runs
FORTRAN=... REPS=3 scripts/bench.sh Batanghari Pessel                 # some of them
FORTRAN=... REPS=3 END_YEAR=2017 scripts/bench.sh Pessel             # a shorter period
```

`scripts/build-fortran.sh` builds SWAT+ at the commit the port follows as CI does (CMake Release,
gfortran, Ninja). On Windows, run everything in WSL 2 (Ubuntu), in the Linux file system (`~`,
not `/mnt/c`), on mains power, with other programs closed.

After the runs it compares every table the port wrote with SWAT+'s from the same machine
(`agreement.csv`, and a table in `summary.md`): identical, within decision K12's tolerance, or
not.

**On ARM64 (aarch64: Snapdragon, Apple silicon under Linux, Graviton)**, gcc fuses `a*b + c` into
one rounding (FMA) by default; the x86-64 reference and the port don't. SWAT+ built there can then
differ from the port in the last bits, and differences grow through the days. Build it a second
time without the fusion and time both:

```sh
FORTRAN=$(scripts/build-fortran.sh) scripts/bench.sh                                     # as SWAT+ builds by default
FORTRAN=$(FORTRAN_FLAGS=-ffp-contract=off scripts/build-fortran.sh) REPS=3 scripts/bench.sh  # without FMA
```

The second shows how much of the agreement and of the speed comes from the fusion.

SWAT+'s own Release build uses `-O` (gcc's `-O1`), as its `CMakeLists.txt` sets it; the port is
built with `opt-level` 3 and thin LTO. Each is timed as its project builds it.

`SAME_OUTPUT=1` turns off in `print.prt` everything but the tables the port writes (the CSV,
management, carbon, hydrograph and flow duration outputs too), so both write about the same and
the time is mostly the simulation's. `scripts/bench-agree.py <results folder>` checks the
agreement again on the last runs (in `$WORK`), without timing anything.

What to keep in mind when reading the numbers:

- **SWAT+ writes more than the port**: besides the tables `print.prt` asks for, its logs and
  other outputs; the files and megabytes written are in the summary. The difference in time is
  the whole program's, reading, simulating and writing.
- **With `END_YEAR`, the years skipped by `print.prt` (`nyskip`) still apply**: a period shorter
  than them prints nothing.
- Both engines run on one core. The time on a laptop varies with its temperature and power plan;
  the median and the range of the runs show how much.
