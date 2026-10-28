# Corrections to SWAT+ (`--fix-swat`)

By default the port reproduces SWAT+ (commit 6f51536) bit for bit, bugs included
(`docs/swat-quirks.md`), because that is how the port is validated. With
`swatplus-rust run <dir> --fix-swat` (in code, `run_with(.., RunOptions { fixes: true })`), the
bugs below are corrected instead. Results then differ from SWAT+'s, so a corrected run should be
calibrated anew rather than compared with a SWAT+ calibration.

Each correction is guarded by the same flag, so a faithful run is unchanged.

## HRUs

| Where (SWAT+) | Bug | Corrected |
|---|---|---|
| `ero_ysed` / snow cover | The branches are in the wrong order: any snow only zeroes yields under 1e-6, so the 100 mm cut-off and the exponential reduction never apply. | No erosion above 100 mm of snow; with less, divided by `exp(sno·3/25.4)`. |
| `psed` | The active and stable mineral P shares are swapped. | `active/attached` goes to the active pool, `stable/attached` to the stable one. |
| `sq_sed` / sediment classes | The sediment taken from the class stores is subtracted twice. | Subtracted once. |
| `pl_nupd`, `pl_pupd` | Only the last plant's N and P demand is zeroed each day, so in a community the other plants keep yesterday's demand. | Every plant's demand starts the day at zero. |
| `hru_hyds` (`subwq`) | The enrichment ratio comes from the previous HRU (a module variable). | The HRU's own ratio for the day. |
| `et_act` (wetlands) | The open-water evaporation coefficient is taken from the HRU numbered as the wetland's hydrology row. | The wetland's own. |
| `et_pot` (Penman-Monteith) | The canopy resistance uses the last plant's stomatal conductance, not the leaf-area-weighted mean summed just before. | The weighted mean. |
| Year end | The yearly irrigation total's reset is commented out, so the annual limit is reached once and for good. | Reset each year. |
| `pl_fert_wet` (paddies) | Fertilizer spread on a flooded paddy goes into the water only with carbon model 2; with the original model it is lost (counted as applied, in no pool). | Into the water with either model. |
| `mgt_newtillmix_wet` (puddling) | The pond's nutrients are added to the mixing sums, which are then zeroed: the pond's nutrients are lost and the water gets the soil's alone. | The pond's nutrients mix with the soil's. |

| Module and `save` variables (`Saved`) | What one HRU leaves in variables shared by all (the nutrient demand arrays `uno3d`, `uapd`, `un2`, `up2`; `delg_p`; the sediment P fractions; the microbial carbon lost with sediment; the enrichment ratio; a septic type's operation; the decision tables' random seed) is read by the next HRU in the command order. | Each HRU has its own copy, its decision tables a random stream of their own (a million draws apart). The HRUs then don't depend on each other's order, and run in parallel (`docs/parallel.md`). |

## Routing units and aquifers

| Where (SWAT+) | Bug | Corrected |
|---|---|---|
| `ru_control` | `ru_d` sums all five hydrographs (total, recharge, surface, lateral, tile), but the total already holds the last three, so they count twice. | Total and recharge only. |
| `aqu_initial` | The initial storage uses `aqu_d(iaqdb)%dep_wt`: the state of the aquifer whose number is this one's database pointer, set only if that aquifer came earlier. | The aquifer's own `dep_wt`. |
| `aqu_hdr` | Columns 9 and 10 are labelled `orgn orgp` but hold carbon and organic N. | Labelled `cbn orgn`. |

## SWAT-DEG channels

| Where (SWAT+) | Bug | Corrected |
|---|---|---|
| `sd_rating_curve` (and the Muskingum `stor_dis`) | The channel area is `b·d + d/z` and the wetted perimeter treats the side slope `z` as vertical over horizontal. | Trapezoid: area `b·d + z·d²`, perimeter `b + 2d·√(1+z²)`. |
| `sd_rating_curve` (flood plain) | Above bankfull the flowing width is the channel's bottom width. | The top width (`chw`). |
| `ch_watqual4` | Travel time uses `ht3%flo`, never set, so it reads the rating curve at zero flow. | The curve at the day's inflow rate. |
| `sd_channel_control3` / `ch_temp` | The outflow hydrograph is copied before the stream temperature is computed, so it leaves with the inflow's temperature. | The outflow carries the computed temperature. |
| `ch_temp` | Upstream water temperature is taken from the last inflow and the one before it, whatever they are. | Flow-weighted mean of all upstream channels' water. |
| `channel_sd_day` `water_temp` | One module variable printed for every channel: the last channel's air-based estimate. | Each channel's own stream temperature. |

## Effect on Bh_land (2015–2016)

- Routing-unit flows halve: the duplicated surface, lateral and tile water is gone.
- Channel storages and seepage rise a few percent with the larger trapezoid areas; outlet flow
  changes by under 0.1%.
- Channel water temperatures now differ by channel (21–28 °C on a mid-year day, where SWAT+
  prints one value for all).
