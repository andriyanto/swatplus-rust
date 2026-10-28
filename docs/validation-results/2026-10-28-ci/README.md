# Validation, 28 October 2026 (CI, x86-64 Linux)

The comparisons of the `Validate against SWAT+` workflow, run 40685973419 on GitHub's
`ubuntu-24.04` runner, of commit `bd74945`: pull request #34 merged into `main`, whose engine is
that of version 1.0.2. SWAT+ is commit `6f51536`, built by the `SWAT+ Fortran build` workflow
(the `fortran-bin` branch). [`summary.md`](summary.md) sums them by scenario
(`scripts/validation-summary.py`); each scenario's folder holds its `.cmp` files, which give
counts and differences only, no values.

The real catchments are shortened to keep the run to about 20 minutes: Batanghari and Bh_land
2015–2019, the Bh_res variants 2015–2017, the Ps variants a coastal part of Pessel 2015–2017,
and the whole of Pessel 2015 only, printed from the first day. The full-length runs are in
[`../../benchmark-results/2026-10-28-arm64/`](../../benchmark-results/2026-10-28-arm64/README.md).

## Result

37 scenarios, 1,934 tables, 48,953 columns, 2,079,406,557 values: no mismatch, no missing or
extra row, and 16,540 crop yield rows without a mismatch.

Every value is identical to SWAT+'s except in the point source tables (`recall_*` and their
basin sums `basin_psc_*`) of the three scenarios with recall objects (Bh_res, Bh_rel,
Bh_nosed), where values differ by at most 5.94e-8 relative: half the spacing of 32-bit floats
(2^-24 = 5.96e-8), that is, one rounding of a `real`. Where that rounding happens hasn't been
traced; nothing downstream differs (the channels, the reservoir and the basin tables are
identical).
