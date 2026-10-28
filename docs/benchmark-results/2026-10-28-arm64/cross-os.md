# Same program, two operating systems (ARM64)

28 October 2026, on the same Snapdragon laptop as the benchmark: Windows 11 on ARM and
Ubuntu 24.04 aarch64 in WSL2.

| | Program | Built by |
|---|---|---|
| Linux | `swatplus-rust-v1.0.2-aarch64-unknown-linux-gnu` | release workflow, GitHub Actions |
| Windows | `swatplus-rust-v1.0.2-aarch64-pc-windows-msvc` | release workflow, GitHub Actions |

Both programs come from the same tag (v1.0.2) and ran the same project (Batanghari):

    swatplus-rust run Batanghari --out out-linux      # in WSL
    swatplus-rust.exe run Batanghari --out out-windows # in PowerShell

## Result

19 of the 20 output tables are identical byte for byte (`cmp`).

The other table, `hru_ls_aa.txt` (1858 rows, 13 value columns), differs on 7 rows. On
each of them only one value differs, in one column, `usle`, by one in the last printed
digit:

| HRU | Linux | Windows | Relative difference |
|---|---|---|---|
| hru0068 | 2891.452 | 2891.451 | 3.5e-7 |
| hru0128 | 237.063 | 237.064 | 4.2e-6 |
| hru0171 | 5356.289 | 5356.288 | 1.9e-7 |
| hru0183 | 1198.175 | 1198.174 | 8.3e-7 |
| hru0277 | 873.799 | 873.798 | 1.1e-6 |

(The table lists the 5 rows shown on screen; the other 2 differ the same way.)
`swatplus-rust compare` reports no mismatch on this table.

## Why only `usle`

`usle` is the soil loss predicted by the USLE equation, a diagnostic output. Its rainfall
erosivity term takes a `log10` and an `ln` of the day's rain:

    usle_ei = precip * (12.1 + 8.9 * (log10(pkrf) - 0.4343)) * pkrf30 / 1000

Rust's `log10` and `ln` call the system math library: glibc on Linux, the Universal C
Runtime on Windows. The two can return results one unit in the last place apart for the
same input. The 7 values lie near a rounding boundary of the printed `f10.3`-style field,
so that last bit shows.

Nothing else reads `usle` or `usle_ei`. The sediment yield (`sedyld`, MUSLE) and every
other column are identical, so the difference does not spread through the simulation:
all water, sediment, nutrient, crop and aquifer tables agree byte for byte.

## Conclusion

The engine gives the same results on Linux and Windows. The only difference comes from
the operating system's math library, on one diagnostic column, at the level of printing
precision (at most 4.2e-6 relative). The same holds against SWAT+ Fortran: see the
agreement tables in `README.md`.
