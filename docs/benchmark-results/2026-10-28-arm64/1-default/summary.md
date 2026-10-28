# swatplus-rust vs SWAT+ Fortran

```text
date: 2026-10-28T00:02:29Z
host: Linux 6.18.33.2-microsoft-standard-WSL2 aarch64
arch: aarch64
cores: 12
memory: 15.4 GB
wsl: yes
swatplus-rust: swatplus-rust 1.0.1 (9e558b1)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
fortran: /home/andri/swatplus-fortran/build/swatplus-6f51536-gnu-lin_aarch64-Rel
gfortran: GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
fortran sha256: 7bb263785bad1f49c3485c99201b2dc639e10f0f90917091b364536d0e5f6c2d
reps: 5
end year: as in time.sim
```

| Scenario | Engine | Runs | Wall time, median (s) | Min–max (s) | Peak memory (MB) | Files written | Written (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | SWAT+ Fortran | 5 | 0.14 | 0.12–0.16 | 10 | 78 | 3.1 |
| Osu_1hru | swatplus-rust | 5 | 0.06 | 0.05–0.07 | 10 | 18 | 0.0 |
| Ames_sub1 | SWAT+ Fortran | 5 | 118.89 | 116.16–122.28 | 10 | 41 | 9719.5 |
| Ames_sub1 | swatplus-rust | 5 | 1.92 | 1.85–2.07 | 19 | 10 | 209.9 |
| Batanghari | SWAT+ Fortran | 5 | 34.49 | 33.98–34.51 | 82 | 73 | 8.9 |
| Batanghari | swatplus-rust | 5 | 13.87 | 13.74–14.10 | 59 | 20 | 3.3 |
| Pessel | SWAT+ Fortran | 5 | 88.47 | 88.07–88.81 | 207 | 73 | 26.0 |
| Pessel | swatplus-rust | 5 | 29.84 | 29.63–30.57 | 101 | 20 | 9.3 |

- **Osu_1hru**: swatplus-rust takes 0.39 of SWAT+ Fortran's time (2.55× as fast).
- **Ames_sub1**: swatplus-rust takes 0.02 of SWAT+ Fortran's time (62.05× as fast).
- **Batanghari**: swatplus-rust takes 0.40 of SWAT+ Fortran's time (2.49× as fast).
- **Pessel**: swatplus-rust takes 0.34 of SWAT+ Fortran's time (2.96× as fast).

## Agreement of the tables

Each table the port wrote, against SWAT+'s from the same run: identical (every value the same number), within decision K12's tolerance, or mismatch; and the largest relative difference of any value.

| Scenario | Tables | Identical | Within tolerance | Mismatch | Largest relative difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | 18 | 11 | 7 | 0 | 1.6 |
| Ames_sub1 | 10 | 4 | 6 | 0 | 1 |
| Batanghari | 20 | 12 | 8 | 0 | 1.8 |
| Pessel | 20 | 12 | 8 | 0 | 2 |

SWAT+ Fortran writes more files than the port (its other outputs and logs); the columns of files written show how much.
