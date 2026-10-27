# Changelog

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
