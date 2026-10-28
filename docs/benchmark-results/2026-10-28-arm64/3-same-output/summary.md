# swatplus-rust vs SWAT+ Fortran

```text
date: 2026-10-28T04:00:21Z
host: Linux 6.18.33.2-microsoft-standard-WSL2 aarch64
arch: aarch64
cores: 12
memory: 15.4 GB
wsl: yes
swatplus-rust: swatplus-rust 1.0.1 (20d21c0)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
fortran: /home/andri/swatplus-fortran/build/swatplus-6f51536-gnu-lin_aarch64-Rel
gfortran: GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
fortran sha256: 7bb263785bad1f49c3485c99201b2dc639e10f0f90917091b364536d0e5f6c2d
reps: 3
end year: as in time.sim
outputs: only the port's tables (SAME_OUTPUT)
```

| Scenario | Engine | Runs | Wall time, median (s) | Min–max (s) | Peak memory (MB) | Files written | Written (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | SWAT+ Fortran | 3 | 0.09 | 0.09–0.10 | 10 | 67 | 3.1 |
| Osu_1hru | swatplus-rust | 3 | 0.04 | 0.04–0.04 | 10 | 16 | 0.0 |
| Ames_sub1 | SWAT+ Fortran | 3 | 3.59 | 3.57–3.61 | 10 | 26 | 289.0 |
| Ames_sub1 | swatplus-rust | 3 | 1.37 | 1.35–1.38 | 19 | 10 | 209.9 |
| Batanghari | SWAT+ Fortran | 3 | 26.87 | 26.07–26.90 | 82 | 66 | 8.1 |
| Batanghari | swatplus-rust | 3 | 10.37 | 10.33–10.90 | 59 | 20 | 3.3 |
| Pessel | SWAT+ Fortran | 3 | 89.60 | 87.93–92.88 | 206 | 66 | 21.9 |
| Pessel | swatplus-rust | 3 | 30.46 | 30.13–30.61 | 101 | 20 | 9.3 |

- **Osu_1hru**: swatplus-rust takes 0.38 of SWAT+ Fortran's time (2.61× as fast).
- **Ames_sub1**: swatplus-rust takes 0.38 of SWAT+ Fortran's time (2.62× as fast).
- **Batanghari**: swatplus-rust takes 0.39 of SWAT+ Fortran's time (2.59× as fast).
- **Pessel**: swatplus-rust takes 0.34 of SWAT+ Fortran's time (2.94× as fast).

## Agreement of the tables

Each table the port wrote, against SWAT+'s from the same run: identical (every value the same number), within decision K12's tolerance, or mismatch; and the largest relative difference of any value.

| Scenario | Tables | Identical | Within tolerance | Mismatch | Largest relative difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | 16 | 9 | 7 | 0 | 1.6 |
| Ames_sub1 | 10 | 4 | 6 | 0 | 1 |
| Batanghari | 20 | 12 | 8 | 0 | 1.8 |
| Pessel | 20 | 12 | 8 | 0 | 2 |

SWAT+ Fortran writes more files than the port (its other outputs and logs); the columns of files written show how much.
