# Phase 3: objects and routing

Goal: SWAT+'s command loop with routing units, aquifers and SWAT-DEG channels, so that daily
`channel_sd` and `aquifer` output match SWAT+ within the K12 tolerance.

## Test projects

Neither reference scenario has a real channel network (Ames_sub1 has none, Osu_1hru one channel).
Phase 3 uses the two West Sumatra projects in `testdata/`:

| Project | Area | Objects | Features beyond phase 2 |
| --- | --- | --- | --- |
| `Batanghari` | 850,722 ha, upland | 42 subbasins, 284 routing units and SWAT-DEG channels, 44 aquifers, 1,858 HRUs | Penman-Monteith PET, urban HRUs, wetland HRUs, decision-table planting and harvest, QUAL2E channel water quality |
| `Pessel` | 272,817 ha, coastal | 230 subbasins, 1,454 routing units and channels, 254 aquifers, 3,966 HRUs, 26 outlets | the same, plus peat, paddy rice and many wetland HRUs |

Both run in SWAT+ Fortran (`6f51536`) exactly as the owner's own SWAT+ rev. 62.0.1 runs, so their
outputs are the reference as they are.

## Stages

As in phase 2, variants built from the real projects remove one dependency at a time, and each
stage is exact before the next starts.

| Stage | Scenario | Adds | Exit |
| --- | --- | --- | --- |
| 3a-0 | `Bh_land`: Batanghari's HRUs with Hargreaves PET, urban and wetland HRUs given an ordinary land use | the land phase on tropical data: evergreen perennials, oil palm, decision-table crops, 1,858 HRUs | `hru_wb`, `hru_pw` daily within tolerance |
| 3a | `Bh_land` (routing units and aquifers depend only on the HRUs, so no subset is needed) | the command loop, HRU hydrographs, routing units (`ru_control`), the 1-D aquifer (`aqu_1d_control`) | `ru` and `aquifer` daily tables within tolerance |
| 3b | `Bh_sub`: one headwater subbasin of `Bh_land` with its SWAT-DEG channels | `sd_channel_control3`: storage routing, rating curves, evaporation and seepage, floodplain link | `channel_sd` daily flow columns within tolerance |
| 3c | `Bh_land`, the whole network | many subbasins, sediment and nutrients in channels and aquifers, QUAL2E | `channel_sd`, `aquifer` all columns |
| 3d | `Batanghari` itself | Penman-Monteith PET, urban HRUs, wetland HRUs | all daily tables |
| 3e | `Pessel` | scale (1,454 channels, 3,966 HRUs), peat, paddy, outlets on the coast | all daily tables |

`scripts/make-phase3-variants.py` builds the variants; SWAT+ Fortran runs them like any other
project.

## Progress

- **3a-0 done.** `Bh_land` matches SWAT+ on every value of the four daily HRU tables (water
  balance, plants, nutrient balance, losses: 80 columns, 6,107,246 HRU-days, 2017–2025 after two
  warm-up years). It took the generic plant-and-harvest decision tables, hard calibration of
  `perco`, gfortran's handling of a short `plants.plt` row, and gfortran's folding of
  `Sin(Atan(x))` (see `docs/swat-quirks.md`). CI runs it on 2015–2019.
- **3a done.** The command loop, HRU hydrographs, 284 routing units and 44 aquifers (42
  shallow, 2 deep) of `Bh_land` match SWAT+ bit for bit on every column of `ru_day` and
  `aquifer_day`, 2015–2025.
- **3b and 3c done.** `Bh_land`'s 284 SWAT-DEG channels, the whole network, match SWAT+ bit
  for bit on all 59 columns of `channel_sd_day` (storage, inflow and outflow hydrographs with
  sediment, nutrients, algae, CBOD and oxygen; seepage, evaporation, water temperature),
  2015–2025: rating curves, variable storage routing, flood plain deposition, bank erosion,
  QUAL2E and the stream temperature model. Not exercised yet: Muskingum routing (`rte_cha` 1),
  channels shorter than 1 m, initial channel water, flood plain wetlands. `Bh_sub` wasn't
  needed.
- **3d: Penman-Monteith PET and urban HRUs done.** `Bh_urb` (Batanghari with only its 79
  wetland HRUs replaced) matches SWAT+ bit for bit on all seven daily tables, 2015–2016, with
  Penman-Monteith PET (and the plants' maximum transpiration under it) and the 167 urban HRUs
  (impervious runoff at curve number 98, build-up and wash-off of solids). CI runs it on
  2015–2019. Priestley-Taylor (`pet` 0) is ported too, not yet validated.
- **3d done.** `Batanghari` itself, all 1,858 HRUs with its 79 wetlands (ponding, seepage into
  the soil and back, open-water evaporation, release by `res_rel.dtl`, settling of sediment and
  nutrients), matches SWAT+ bit for bit on all seven daily tables, 2015–2016; every wetland
  fills and releases. CI runs it on 2015–2019. Not ported: paddies, weirs (`weir.res`),
  initial wetland water, the wetland output tables, release options other than `rate`,
  `rate_pct`, `ab_emer`, `days` and `dyrt`.
- **3e, coastal lowland done.** `Ps_coast` (`scripts/make-pessel-variants.py`): the 20 small
  catchments of Pessel whose land lies under 20 m and drains straight to the sea, cut out with
  everything renumbered — 74 channels, 74 routing units, 115 HRUs on peat (oil palm, peat swamp
  and mangrove wetlands, rice, six HRUs without a land use), 23 shallow and 20 deep aquifers,
  2015–2017 printed from the first day. It matches SWAT+ bit for bit on all seven daily tables.
  It took two quirks: an HRU whose land use (or topography, hydrology, snow, soil-plant data)
  isn't found, the Editor's `null` among them, gets the table's first row; and `varinit` zeroes
  the peak rate for each HRU, so a wetland never runs `nut_orgn`/`nut_psed`. CI runs it. Next:
  the whole of Pessel (3,966 HRUs, 1,454 channels), which needs a shorter print to fit.
