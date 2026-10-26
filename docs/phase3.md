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
| 3a | `Bh_sub`: one headwater subbasin of `Bh_land`, its routing units, shallow and deep aquifer, channels as zero-length pass-throughs | the command loop, HRU hydrographs, routing units (`ru_control`), the 1-D aquifer (`aqu_1d_control`) | `ru` and `aquifer` daily flows within tolerance |
| 3b | `Bh_sub` with its SWAT-DEG channels | `sd_channel_control3`: storage routing, rating curves, evaporation and seepage, floodplain link | `channel_sd` daily flow columns within tolerance |
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
