# Working on swatplus-rust

- **Document every step in the design note.** Each piece of work that changes the engine, the
  calibration tools or what a result means gets its section in `docs/nota-desain` (Indonesian,
  function by function, as the chapters there) and a row in the work log of its appendix
  (Lampiran B, `B.8 Catatan langkah kerja`), in the same pull request. Then rebuild the single
  files with `scripts/build-nota-desain.sh`. The English docs (`docs/*.md`, `CHANGELOG.md`,
  `docs/roadmap.md`) are updated as before. Work in RustGIS-SwatPlus (`andriyanto/rustgis-swatplus`)
  is recorded the same way in Lampiran C (`22-lampiran-c-rustgis-swatplus.md`, work log C.7).
- **The engine is run, never linked, by RustGIS-SwatPlus** (decision K11): the program
  `swatplus-rust` or the module `swat_wasm.wasm`, because the engine is LGPL-2.1 and RustGIS
  Apache-2.0.
- **Faithful mode stays SWAT+.** A change of results in a faithful run is a bug; corrections and
  extensions belong to `--fix-swat` or to new functions, recorded in `docs/swat-fixes.md`. Every
  change is validated against SWAT+ Fortran in CI (`Validate against SWAT+`).
- **Private data.** The Batanghari and Pessel projects (`testdata/Batanghari`, `testdata/Pessel`,
  read by CI from a private repository) belong to Dinas SDA-BK Provinsi Sumatera Barat. Only
  summary figures from them go into this repository or a publication; never their files, maps or
  daily series. JAXA GSMaP data are not committed either.
- **No personal contact details** (e-mail, telephone) in the repository or the Zenodo record.
