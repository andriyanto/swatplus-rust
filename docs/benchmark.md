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

What to keep in mind when reading the numbers:

- **SWAT+ writes more than the port**: besides the tables `print.prt` asks for, its logs and
  other outputs; the files and megabytes written are in the summary. The difference in time is
  the whole program's, reading, simulating and writing.
- **With `END_YEAR`, the years skipped by `print.prt` (`nyskip`) still apply**: a period shorter
  than them prints nothing.
- Both engines run on one core. The time on a laptop varies with its temperature and power plan;
  the median and the range of the runs show how much.
