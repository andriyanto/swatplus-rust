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
| 2a ✅ | `Ames_bare`: bare soil, no plants, no management | soil water, snow, soil evaporation, curve-number runoff, percolation, lateral flow, soil temperature, biological mixing | `hru_wb_day` within tolerance, 12 HRUs, 1975–2020: **exact**, 28 columns × 201 624 HRU-days |
| 2b-1 ✅ | `Ames_past0`: pasture, no management, original carbon model (`cswat` 0) | plant growth, canopy, transpiration, soil N and P, residue, erosion | **exact**: `hru_wb_day`, `hru_pw_day`, `hru_nb_day`, `hru_ls_day`, 77 columns × 201 624 HRU-days |
| 2b-2 ✅ | `Ames_past`: the same with the carbon model on (`cswat` 2, as Ames) | `cbn_zhang2` and the Century pools, daily biological mixing of nutrients and texture | **exact**: the same four tables, and every layer's soil carbon (`hru_cbn_lyr`) within print precision |
| M1 ✅ | `Ames_corn`, `Ames_cornf`: corn alone with `mgt_01`'s dated operations; `Ames_cornf` with its fertilizer applied | planting, harvest and kill, tillage with the carbon model, irrigation, fertilizer (mineral and manure), annual plants | **exact**: the four tables, 80 columns × 201 624 HRU-days each |
| M2 | `Ames_sub1` itself | two plants (corn and soybean), irrigation by decision table | next |
| later | `Osu_1hru` | paddy and wetland storage, decision tables, Penman-Monteith PET | with phases 4 and 6 |

Bare soil in Ames_sub1 (HRU 1, 1980): 568 mm of precipitation, 64 mm of surface runoff, 531 mm of
evapotranspiration, all from the soil. The columns that move: precip, snofall, snomlt, surq_gen,
wateryld, et, esoil, surq_cont, cn, sw_init, sw_final, sw_ave, sw_300, sno_init, sno_final,
snopack, pet, sw_change, lagsurf.

## Stage 2a: result

Done (`swat-engine/src/hru.rs`). Every column the port writes is identical to SWAT+'s on every
day, with no tolerance used. What it took beyond the plan:

- **Basin parameter defaults.** `basin_prm_default` replaces values below 1e-6 after reading
  `parameters.bsn`; Ames writes `cn_froz` as 0, so SWAT+ uses 0.000862 (frozen soil runoff).
- **Biological mixing moves soil water.** With carbon on (`cswat = 2`), `mgt_biomix` mixes the
  top layers every day (the `biomix` row of `tillage.til`: 1 % to 50 mm in Ames), soil water
  included. Without it, evaporation drifted by up to 0.003 mm a day once the top layer dried
  out. It needs `tillage.til` and `carbon.bsn` (temperature factor), now read.
- **No `soil_lyr_depths.sol`.** The variants drop it, so soils keep the layers of `soils.sol`
  plus SWAT+'s 10 mm top layer; re-layering to custom depths comes later.

`run` checks a project's HRUs first and, when one needs something not ported (plants,
management, urban land, tiles, septic systems, filter strips, surface storage, other PET methods,
Green and Ampt, cracks, gwflow, sub-daily steps, water routed into HRUs), skips the water balance
and says why.

## Management M1: result

Done (`swat-engine/src/hru/mgt.rs`; readers for `management.sch`, `harv.ops`,
`fertilizer.frt`, `irr.ops`, `chem_app.ops`). `Ames_corn` is Ames with corn alone (`corn_comm`)
and `mgt_01`'s 305 operations without its decision table: tillage, planting, one irrigation,
fertilizer, harvest and kill, and the yearly skips (a 41-year rotation, run over 46 years). Its fertilizer
names are in the wrong column, so SWAT+ applies none (see the quirks); `Ames_cornf` moves them
and broadcasts or injects them, with dairy manure in place of one blend for the organic path.
Ported: `mgt_operatn`, `mgt_sched` (plant, harvest and kill, tillage, irrigation, fertilizer,
skip), `mgt_plantop`, `mgt_harvgrain`, `mgt_killop`, `pl_fert`, `mgt_newtillmix_cswat1`,
`mgt_tillfactor` for tillage, the tillage counters and their effect on biological mixing and
decomposition, the yearly resets, and `plant_init` for annuals. Both match SWAT+ bit for bit.

What it took beyond the plan: a one-bit difference in Hargreaves PET, from `x ** 0.5`, which
Rust computed as a square root (quirks, "Arithmetic"). It showed only in the fertilized run, a
few printed digits over 46 years; found by comparing each layer's soil water from SWAT+'s
`object.prt` output (`sol_water`) and the state SWAT+ writes at each operation (`mgt_out.txt`).

Limits, reported by `run`: decision tables, operations other than these (grazing, pesticide,
manure, burning, harvest only, kill only…), operations by heat units, harvests other than grain,
tillage with the original carbon model.

## Stage 2b-2: result

Done (`swat-engine/src/hru/carbon.rs`): `soil_nutcarb_init` for the carbon model,
`cbn_surfrsd_decomp`, `cbn_rsd_transfer`, `cbn_zhang2` with `nut_np_flow`, `nut_orgnc2`, and
the nutrient, pool and texture part of `mgt_biomix` with `mgt_tillfactor`; `carbon_lyr.bsn` is
read. `Ames_past` matches SWAT+ in all four daily HRU tables, and so does `Ames_bare` in its
nutrient and loss tables, which the port now writes for it too. The carbon pools themselves,
checked once against SWAT+'s `hru_cbn_lyr.txt` (total soil carbon per layer, printed with nine
digits), agree to within the last printed digit on every day of every HRU.

## Stage 2b-1: result

Done (`swat-engine/src/hru/plant.rs`, `hru/nutrient.rs`). The pasture grows on soil nitrogen:
with nutrient stress off, almost every water balance column changes, so plants can't be
validated without the soil nutrients. Stage 2b was split on the carbon model: the original one
(`cswat` 0, SWAT+ Editor's default) first, `Ames_past0`; the Century model Ames uses
(`cswat` 2, `cbn_zhang2`, ~650 lines) next. Ported for 2b-1: `plant_init`,
`plantparm_init`, `pl_community`, `pl_waterup`, `pl_grow` with `pl_nut_demand`, `pl_dormant`,
`pl_biomass_gro`, `pl_tstr`, `pl_nup`, `pl_nfix`, `pl_pup`, `pl_root_gro`, `pl_rootfr`,
`pl_leaf_gro`, `pl_leaf_senes`, `pl_seed_gro`, `pl_partition`, `pl_mortality`; canopy
interception and transpiration in `et_act`; `soil_nutcarb_init`, `rsd_decomp`, `nut_nminrl`,
`nut_nitvol`, `nut_pminrl`, `nut_nlch`, `nut_solp`, `pest_enrsb`, `nut_orgn`, `nut_psed`;
erosion (`ero_pkq`, `ero_eiusle`, `ero_cfactor`, `ero_ysed`) and the sediment lag; the
end-of-year mixing and the yearly resets. All four daily tables are identical to SWAT+'s.

Limits, each reported by `run` when a project needs it: one plant per community, plants with
heat units over the year (`days_mat` 0) or perennials, no plant population, no `co2_yr.dat`, no
atmospheric deposition, soil P model 0.

## Stage 2a: inputs

All read now (`swat-io`): `hru-data.hru`, `topography.hyd`, `hydrology.hyd`, `field.fld`,
`soils.sol`, `soil_plant.ini`, `landuse.lum`, `cntable.lum`, `ovn_table.lum`,
`cons_practice.lum`, `snow.sno`, `codes.bsn`, `parameters.bsn`. Added in the port: `tillage.til`
and `carbon.bsn` (biological mixing, above). Not needed: `nutrients.sol` and the rest of the
carbon model, which doesn't feed the water balance.

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
