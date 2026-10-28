# Validation, 28 October 2026 (CI, x86-64 Linux)

The comparisons of the `Validate against SWAT+` workflow, run 40710519593 on GitHub's
`ubuntu-24.04` runner, of commit `30deb62` (what became version 1.0.3). SWAT+ is commit
`6f51536`, built by the `SWAT+ Fortran build` workflow (the `fortran-bin` branch).
[`summary.md`](summary.md) sums them by scenario (`scripts/validation-summary.py`); each
scenario's folder holds its `.cmp` files, which give counts and differences only, no values.

The real catchments are shortened to keep the run to about 20 minutes: Batanghari and Bh_land
2015–2019, the Bh_res variants 2015–2017, the Ps variants a coastal part of Pessel 2015–2017,
and the whole of Pessel 2015 only, printed from the first day. The full-length runs are in
[`../../benchmark-results/2026-10-28-arm64/`](../../benchmark-results/2026-10-28-arm64/README.md).

## Result

37 scenarios, 1,934 tables, 48,953 columns, 2,079,406,557 values: every value identical to
SWAT+'s (the largest difference of every column is 0), no missing or extra row, and 16,540 crop
yield rows without a mismatch.

The first recorded run, earlier the same day (run 40685973419), found the point source tables
(`recall_*`, `basin_psc_*`) of Bh_res, Bh_rel and Bh_nosed up to 5.94e-8 apart: the port printed
the shortest decimal of each float where SWAT+'s list-directed `write` prints nine digits. Same
floats, different decimals; 1.0.3 prints them as SWAT+ does.
