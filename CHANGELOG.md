# Changelog

## 1.0.2

- **The source checks out on Windows**: `crates/swat-io/src/con.rs` (CON is a reserved name
  there) is now `connect.rs`, the module still `con`. Release 1.0.1 built nothing for that
  reason. CI checks the file names; the release builds each platform even when another fails.
- **Programs for ARM64**: Windows on ARM (Snapdragon laptops) and Linux on ARM (and WSL there),
  besides Linux and Windows on x86-64 and macOS (Apple silicon and Intel).

## 1.0.1

- **Reservoirs and wetlands without their `sediment.res` row** keep settling factors of 0, as
  in SWAT+: all their sediment above the normal concentration settles, and no soluble N or P
  (the port used 1 and 0.2). Validated by the new variants Bh_nosed and Ps_edge.
- **`surlag` calibrated more than once** compounds, each change from the last one's value.
- **`run --strict`** writes the tables in full (Parquet too) before exiting with code 3;
  `--out` no longer takes the next flag as its folder.
- **`compare`** counts NaN, infinities and asterisks against a number as a mismatch (they were
  skipped); NaN equals NaN.
- **Text tables** write NaN and the infinities as gfortran does in the `E15.4` columns
  (`0.E+01` before).
- Docs: `usgs_reg` and the newer reservoir releases are ported (the docs said not); Bh_urb is no
  longer run by CI.

## 1.0.0 — engine 1.0

The first release of the engine RustGIS runs. Its output tables are SWAT+'s (commit `6f51536`)
bit for bit on Osu_1hru and Ames_sub1 (SWAT+'s examples), Batanghari and Pessel (West Sumatra)
and 31 variants that exercise each feature; CI checks every table against SWAT+ on every push.

- **Land**: weather (measured, generated, lapse rates), HRU water balance, plant growth,
  nutrients with the original and Century carbon models, erosion; management by date, heat
  units and decision tables, every harvest type; rice paddies, wetlands, tile drains, filter
  strips, grassed waterways, user practices, septic systems, urban areas (build-up and
  wash-off, USGS regressions); hard calibration with conditions.
- **Routing**: landscape routing between HRUs, routing units, aquifers, SWAT-DEG channels
  (variable storage or Muskingum, QUAL2E, temperature), reservoirs (decision tables, natural
  lakes, `dyrt1`, `inflo_targ`…), point sources, nitrogen deposition.
- **Sub-daily steps**: Green-Ampt, or the curve number with unit hydrographs; urban HRUs by
  step; generated rain by step; hourly channel flows.
- **Running it** (`docs/runner.md`): `--format text|parquet|both`, `--progress json`,
  `--strict`; a WebAssembly module for the browser (tar in, tar out); `--fix-swat` to correct
  SWAT+'s bugs instead of reproducing them.

What isn't ported is refused by name; the quirks reproduced are in `docs/swat-quirks.md`,
`docs/engine1.md` and the other documents of `docs/`.
