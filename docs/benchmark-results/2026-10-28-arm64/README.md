# Benchmark, 28 October 2026: a Snapdragon laptop (ARM64)

`scripts/bench.sh` on one laptop: Windows on ARM, a Snapdragon processor (12 cores; the model
wasn't recorded), 15.4 GB of memory, WSL 2 with Ubuntu 24.04 (aarch64). swatplus-rust 1.0.1
(rustc 1.99.1, `--release`); SWAT+ `6f51536` built there with gfortran 13.3.0 as its CMake
Release does (`-O`). Every run in a fresh copy of the scenario, the engines alternating.

| Run | Folder | SWAT+ build | `print.prt` | Runs per engine |
| --- | --- | --- | --- | ---: |
| 1 | [`1-default`](1-default/summary.md) | as SWAT+ builds it | the scenario's | 5 |
| 2 | [`2-no-fma`](2-no-fma/summary.md) | `-ffp-contract=off` (no fused multiply-add) | the scenario's | 3 |
| 3 | [`3-same-output`](3-same-output/summary.md) | as SWAT+ builds it | only the port's tables (`SAME_OUTPUT=1`) | 3 |

## Time and memory

Median wall time (s), and swatplus-rust's speed against SWAT+'s:

| Scenario | HRUs, years | Run 1: SWAT+ / port | Run 2 | Run 3 (same output) | Peak memory, SWAT+ / port (MB) |
| --- | --- | --- | --- | --- | --- |
| Osu_1hru | 1, 14 | 0.14 / 0.06 (2.6×) | 0.10 / 0.04 (2.6×) | 0.09 / 0.04 (2.6×) | 10 / 10 |
| Ames_sub1 | 12, 46 | 118.9 / 1.9 (62×) | 92.5 / 1.4 (66×) | **3.6 / 1.4 (2.6×)** | 10 / 19 |
| Batanghari | 1,858, 11 | 34.5 / 13.9 (2.5×) | 25.9 / 10.7 (2.4×) | **26.9 / 10.4 (2.6×)** | 82 / 59 |
| Pessel | 3,966, 11 | 88.5 / 29.8 (3.0×) | 89.2 / 29.8 (3.0×) | **89.6 / 30.5 (2.9×)** | 207 / 101 |

- **With the same output, swatplus-rust runs in 0.34 to 0.39 of SWAT+'s time (2.6 to 2.9 times
  as fast)** on every scenario, and uses half of SWAT+'s memory on the large ones.
- **Ames_sub1's 62× is SWAT+'s writing, not its simulation**: its `print.prt` asks for daily
  carbon and management outputs the port doesn't write, 9.7 GB of them (the port writes 0.2 GB).
  Asked for the same tables, SWAT+ takes 3.6 s instead of 119.
- **The ratio is steadier than the times**: run 1's Batanghari took a third longer than runs 2
  and 3 for both engines (the laptop's state), the ratio stayed 2.4 to 2.6. The runs within one
  session vary by under 3 % (`summary.md`, min–max).
- Turning SWAT+'s fused multiply-add off changed neither its time (Pessel 88.5 against 89.2 s)
  nor its results.

## The results agree

`swatplus-rust compare`, by column name: in every run, all 1,461 columns of the daily-keyed tables
(1,384 with the same output) have a largest difference of 0, on ARM64 as on x86-64 in CI, with
or without fused multiply-add. The crop yield tables, printed with 9 decimals, differ by at most
1.2e-8 relative (`agreement.csv`), less than a 32-bit float's last bit, in the printing.

`agreement.csv`'s *identical* and *within tolerance* columns of these runs came from an earlier
check that paired the columns by position; the water and nutrient balance tables, whose columns
SWAT+ writes more of, showed as *within tolerance* and with relative differences up to 2. The
`compare/` files, which pair them by name, show no difference; `scripts/bench-agree.py` now pairs
by name.

## Linux and Windows

The same v1.0.2 program, built for Linux and for Windows on ARM64, was run on the same
laptop: 19 of 20 tables are identical byte for byte, and the last differs on 7 values of the
diagnostic `usle` column, in the last printed digit. See [cross-os.md](cross-os.md).
