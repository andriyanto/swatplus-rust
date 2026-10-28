# swatplus-rust vs SWAT+ Fortran

```text
date: 2026-10-28T00:46:12Z
host: Linux 6.18.33.2-microsoft-standard-WSL2 aarch64
arch: aarch64
cores: 12
memory: 15.4 GB
wsl: yes
swatplus-rust: swatplus-rust 1.0.1 (20d21c0)
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
fortran: /home/andri/swatplus-fortran/build--ffp-contract=off/swatplus-6f51536-gnu-lin_aarch64-Rel
gfortran: GNU Fortran (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
fortran sha256: c4d18956207cd5fb2471e3cc34ddd36a112a4fb8ddb0bc909f884282b028db4e
reps: 3
end year: as in time.sim
outputs: as in print.prt
```

| Scenario | Engine | Runs | Wall time, median (s) | Min–max (s) | Peak memory (MB) | Files written | Written (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | SWAT+ Fortran | 3 | 0.10 | 0.09–0.10 | 10 | 78 | 3.1 |
| Osu_1hru | swatplus-rust | 3 | 0.04 | 0.04–0.04 | 10 | 18 | 0.0 |
| Ames_sub1 | SWAT+ Fortran | 3 | 92.46 | 86.76–92.71 | 10 | 41 | 9719.5 |
| Ames_sub1 | swatplus-rust | 3 | 1.40 | 1.36–1.52 | 19 | 10 | 209.9 |
| Batanghari | SWAT+ Fortran | 3 | 25.94 | 25.74–26.07 | 82 | 73 | 8.9 |
| Batanghari | swatplus-rust | 3 | 10.70 | 10.54–10.83 | 59 | 20 | 3.3 |
| Pessel | SWAT+ Fortran | 3 | 89.24 | 88.69–90.60 | 206 | 73 | 26.0 |
| Pessel | swatplus-rust | 3 | 29.84 | 29.64–40.11 | 101 | 20 | 9.3 |

- **Osu_1hru**: swatplus-rust takes 0.38 of SWAT+ Fortran's time (2.64× as fast).
- **Ames_sub1**: swatplus-rust takes 0.02 of SWAT+ Fortran's time (65.95× as fast).
- **Batanghari**: swatplus-rust takes 0.41 of SWAT+ Fortran's time (2.42× as fast).
- **Pessel**: swatplus-rust takes 0.33 of SWAT+ Fortran's time (2.99× as fast).

## Agreement of the tables

Each table the port wrote, against SWAT+'s from the same run: identical (every value the same number), within decision K12's tolerance, or mismatch; and the largest relative difference of any value.

| Scenario | Tables | Identical | Within tolerance | Mismatch | Largest relative difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| Osu_1hru | 18 | 11 | 7 | 0 | 1.6 |
| Ames_sub1 | 10 | 4 | 6 | 0 | 1 |
| Batanghari | 20 | 12 | 8 | 0 | 1.8 |
| Pessel | 20 | 12 | 8 | 0 | 2 |

SWAT+ Fortran writes more files than the port (its other outputs and logs); the columns of files written show how much.
