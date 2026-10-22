# Phase 2: the HRU water balance

Goal: the daily HRU water balance (`hru_wb_day`) identical to SWAT+, within the K12 tolerance.

## Why in stages

Neither reference scenario isolates the water balance:

- **Osu_1hru** is a rice paddy: its HRU stores water behind a weir (`surf_stor = paddy0001`,
  handled by `wetland_control`), and its management runs through decision tables (puddling, weir
  heights, paddy irrigation).
- **Ames_sub1** grows corn and soybean with irrigation, fertilizer and tillage (`mgt_01`: 305
  operations), with the carbon model on (`cswat = 2`) and Hargreaves PET (`pet = 2`).

In both, evapotranspiration depends on leaf area from plant growth, runoff on curve numbers that
management and residue change, and crops on nitrogen. So the water balance is validated on
scenarios built from Ames_sub1 that remove one dependency at a time
([`scripts/make-variants.sh`](../scripts/make-variants.sh)); SWAT+ Fortran runs them like any other.

| Stage | Scenario | Adds | Exit |
| --- | --- | --- | --- |
| 2a | `Ames_bare`: bare soil, no plants, no management | soil water, snow, soil evaporation, curve-number runoff, percolation, lateral flow, soil temperature | `hru_wb_day` within tolerance, 12 HRUs, 1975–2020 |
| 2b | `Ames_past`: pasture, no management | plant growth (`pl_grow`, ~1 100 lines), canopy, transpiration | `hru_wb_day` and the plant columns of `hru_pw_day` within tolerance |
| later | `Ames_sub1` itself | management, irrigation, nutrients, carbon | with phase 4 |
| later | `Osu_1hru` | paddy and wetland storage, decision tables, Penman-Monteith PET | with phases 4 and 6 |

Bare soil in Ames_sub1 (HRU 1, 1980): 568 mm of precipitation, 64 mm of surface runoff, 531 mm of
evapotranspiration, all from the soil. The columns that move: precip, snofall, snomlt, surq_gen,
wateryld, et, esoil, surq_cont, cn, sw_init, sw_final, sw_ave, sw_300, sno_init, sno_final,
snopack, pet, sw_change, lagsurf.

## Stage 2a: inputs

All read now (`swat-io`): `hru-data.hru`, `topography.hyd`, `hydrology.hyd`, `field.fld`,
`soils.sol`, `soil_plant.ini`, `landuse.lum`, `cntable.lum`, `ovn_table.lum`,
`cons_practice.lum`, `snow.sno`, `codes.bsn`, `parameters.bsn`. Not yet needed: `nutrients.sol`
and the carbon files (the carbon model runs, but doesn't feed the water balance of a bare soil;
to be confirmed in the port).

## Stage 2a: the Fortran to port

Initialisation, called from `proc_hru` (about 1 500 lines without plants):

1. `hru_read`: HRUs and their database pointers (linear lookups by name).
2. `hrudb_init`, `hru_lum_init_all` / `hru_lum_init`: land use, curve number table, overland n,
   conservation practice.
3. `topohyd_init`: topography and hydrology per HRU.
4. `soils_init` → `soil_phys_init`, `layersplit`, `soils_test_adjust`: layer depths, field
   capacity, wilting point, saturation, conductivity, initial water from `sw_frac`.
5. `plant_all_init`: runs, with an empty plant community for bare soil.
6. `cn2_init_all` / `cn2_init` → `curno`: curve numbers and retention parameters.
7. `hydro_init`: hydrology derived values (time of concentration, lag, `ttcoef_wway`).

Daily, in `hru_control` order (about 2 200 lines):

`varinit` → `albedo` → `stmp_solt` → `sq_canopyint` → `sq_snom` → `et_pot` (Hargreaves for Ames)
→ `et_act` → `surface` (`sq_daycn`, `sq_volq`, erosion) → `swr_percmain` (`swr_percmicro`,
`swr_satexcess`, `swr_depstor`) → `swr_latsed` → `stor_surfstor` → `swr_substor` → `sq_surfst` →
`hru_hyds`, and the HRU output sums (`hru_output`).

The routines that run but don't feed water (nutrients, carbon, pesticides, erosion) are skipped
in stage 2a and listed as such, so it is explicit what isn't ported.

## How a stage is checked

The `Validate against SWAT+` workflow builds the variants with `scripts/make-variants.sh`, runs
SWAT+ Fortran and swatplus-rust on them, and compares every column the port writes. Locally:

```sh
scripts/make-variants.sh testdata/Ames_sub1 /tmp/variants
scripts/print-daily.sh /tmp/variants/Ames_bare hru_wb,hru_pw hru_cb,hru_cb_vars
(cd /tmp/variants/Ames_bare && swatplus-fortran)
cargo run -p swatplus-rust -- run /tmp/variants/Ames_bare --out /tmp/rust
cargo run -p swatplus-rust -- compare /tmp/variants/Ames_bare/hru_wb_day.txt /tmp/rust/hru_wb_day.txt
```

## Risks

- **Implicit SAVE.** The water routines are the first with local variables whose values carry
  from one call to the next. Each routine is audited before it is ported; the findings go into
  `docs/swat-quirks.md`.
- **Order of HRU updates.** Some state is global and shared between routines (`sepday`,
  `sw_excess`, `latlyr`): the port keeps it in a per-HRU scratch struct, set and read in the
  same order.
- **Penman-Monteith** (Osu) waits for stage "later"; Ames uses Hargreaves.
