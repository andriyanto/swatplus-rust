# SWAT+ behaviour the port reproduces

SWAT+ is the reference: where it does something surprising, the port does the same, so results
match. Each entry says where it comes from in the Fortran (commit `6f51536`) and where the port
handles it.

## Reading files

- **List-directed input.** Every reader uses `read (unit, *)`. A read that needs more values than
  a line holds goes on into the next lines; values left on the last line are skipped; blank lines
  are skipped; a `/` ends the read; `,,` is a null value that leaves the variable unchanged;
  unquoted text ends at a blank, comma or slash. (`swat-io/src/list.rs`)
- **Integers reject decimals.** gfortran rejects `1.0` where an integer is read. (`list::parse_int`)
- **`file.cio` is read by position.** The name at the start of each row is ignored; a row with
  more names than slots has the extra ones skipped (Ames_sub1's `init` row has 12 for 11); rows
  past the end of the file keep their default file names; `null` turns a slot off.
  (`readcio_read`; `swat-io/src/cio.rs`)
- **`time.sim`.** A `step` of 0 or less is daily; a `day_start` of 0 or less is 1 January
  (`time_read`). A `day_end` of 0 means 31 December of the last year, and a `day_end` past the
  year's length is capped (`time_control`). (`swat-model/src/time.rs`)
- **`print.prt`.** The count of average annual intervals is read, then its record is read again
  with the years (`backspace`). Afterwards the print window is completed: a first day of 0 is 1, a
  last day of 0 is 366, a first year of 0 is the first simulated, a last year of 0 is one past the
  last, and the interval counter starts at the interval. (`basin_print_codes_read`;
  `swat-io/src/print.rs`, `swat-engine/src/run.rs`)
- **A value that doesn't convert ends the read silently.** SWAT+ checks only for end of file
  (`iostat < 0`). Osu_1hru's `parameters.bsn` (written by SWAT+ Editor) has `day_lag_mx` as
  `0.00000`, which gfortran won't read as an integer: the read stops there and `igen` keeps its
  default 5, which changes the random seeds. Ames_sub1's file has `0 0`, so its `igen` is 0.
  (`swat-io/src/basin.rs`)
- **Tables are counted, then read.** Database tables (`topography.hyd`, `landuse.lum`…) are
  counted one value per record (blank lines don't count), then read that many times as whole
  derived types; a short row takes values from the next line. `hru-data.hru` is sized by its
  largest HRU number and each row placed at its own number. (`swat-io/src/table.rs`, `hru.rs`)
- **Lookups by name are linear and take the first match**, compared as Fortran strings (trailing
  blanks ignored); a name not found leaves the pointer 0. (`swat-model/src/table.rs`)
- **Soil texture is one word.** `soils.sol` texture strings with blanks (`L- L- L- L`) are read
  up to the first blank. A first layer thinner than 20 mm becomes 20 mm when the soil has one
  layer or its second is deeper than 20 mm. (`soil_db_read`; `swat-io/src/hru.rs`)
- **Basin parameters below 1e-6 take defaults.** After `parameters.bsn` is read,
  `basin_prm_default` replaces any value below 1e-6 (zero, or not read) with a default, which for
  some isn't the variable's own initial value (`prf` 1 instead of 484, `cmn` 0.0003 instead of
  0.003). Ames_sub1 writes `cn_froz` as 0; SWAT+ runs with 0.000862.
  (`swat-model/src/basin.rs::BasinParams::complete`)
- **Connection files** are read twice per row, the second time with as many outflows as
  `out_tot` says. (`hyd_read_connect`; `swat-io/src/con.rs`)

## Climate

- **An empty or missing station file stops its variable.** The measured readers (`cli_pmeas`,
  `cli_tmeas`…) leave their station loop at the first file that ends inside its header, so the
  files after it in the list are not read either. A missing file is opened anyway (gfortran
  creates it empty) and behaves the same. Ames_sub1's `tmp.cli` lists `ames.tmp`, which is empty:
  no measured temperature is read. (`swat-io/src/project.rs`, `LoadReport::unread`)
- **Station files are found by binary search.** `cli_staread` looks up each station's gage in the
  station list with `search`, a binary search, so it finds names reliably only in a sorted list.
  In Ames_sub1 the station names `ames.tem` as its temperature gage, which isn't in the list
  `["ames.tmp"]`, so its gage stays 0: an empty gage, always out of bounds, so temperature is
  generated every day. (`swat-engine/src/fortran.rs::search`)
- **Names not found fall back to the first.** A station whose generator name isn't found uses
  generator 1, and an object whose station name isn't found uses station 1 (Ames_sub1's HRUs name
  `wgn_01`, the station is `wgn_sta1`).
- **A gage named `null` is not the same as `sim`.** Only `sim` takes the generator's path; any
  other name that isn't found goes through the measured path with an empty gage, which also ends
  in the generator but draws more random numbers on the way (temperature calls `cli_weatgn` twice).
- **Headers can be wrong.** Osu_1hru's precipitation header says 42 years, but the file holds 43
  (1981–2023). SWAT+ sizes its array from the header but starts filling it at the simulation's
  first year, so this does no harm there. The readers keep the records as written.
- **Short series.** Osu_1hru's solar radiation and wind series end on 2022-304, before the
  simulation ends (2023). SWAT+ checks each day against the file's bounds (`cli_bounds_check`) and
  marks days outside them -98, for the generator. (`swat-engine/src/climate.rs`)
- **Generated solar radiation is the monthly mean.** `cli_slrgen` computes a generated value, then
  its last line replaces it with the month's average from the generator parameters.
- **Precipitation is decided a day ahead.** `cli_precip_control` reads or generates the next day's
  precipitation before `cli_clgen` updates the wet/dry state from today's, so a generated day's
  chance of rain follows the wet/dry state of two days before it.
- **The month table is global.** `cli_initwgn` takes month lengths from SWAT+'s global `ndays`,
  which the temperature reader last set from the year of its last record (leap if divisible by 4),
  or the leap-year table it starts as when no temperature file was read. The simulation loop then
  sets it per year with the full Gregorian rule.
- **Seeds depend on `igen`.** With `igen` not 0, `gcycl` moves the seeds (of the first generator
  only) and shuffles which seed each draw uses (`idg`). `cli_initwgn` uses a copy of the
  precipitation seed for its 1000-value correction factors, so they don't move the seed itself.
- **Missing values** are written -99 in the files.

## HRU water balance

- **A 10 mm top layer is added** to every soil whose first layer is deeper than 19.5 mm, with the
  first layer's properties; layer j then takes the file's layer j − 1 (`soils_init`, when there is
  no `soil_lyr_depths.sol`). (`swat-engine/src/hru.rs::soil_layers`)
- **Initial soil temperature is December's maximum** averaged with itself:
  `(tmpmx(12) + tmpmx(12)) / 2` (`hydro_init`). Every layer starts there.
- **Initial soil water comes from `parameters.bsn`'s `sw_init`** (`ffcb`, a fraction of field
  capacity), or from annual precipitation when it is 0. `soil_plant.ini`'s `sw_frac` is read but
  not used.
- **A curve number table name not found** points at element 0, which keeps the type's defaults
  (30, 55, 70, 77). A hydrologic group other than A–D gives a curve number of 0.
- **Overland n defaults to 0.05** when the land use's `ovn` name isn't in `ovn_table.lum`
  (`plant_init`); the table's own default is 0.5.
- **The snow temperature has no memory.** `sq_snom` resets its snowpack temperature to 0 on each
  call before lagging it, so it is always `tave * timp`.
- **Percolation runs every day**, even without rain: layers above field capacity drain whether or
  not water arrives. A frozen layer (temperature ≤ 0) holds its water. Water above saturation
  moves up layer by layer, and what the top layer can't hold joins surface runoff.
- **Soil evaporation where the cap bites:** each layer gives at most 80 % of its water a day
  (`etco`); a layer left with less than 1e-6 mm keeps 1e-6.
- **Biological mixing moves water.** With carbon on (`cswat` 2), `mgt_biomix` runs every day: the
  thawed layers above the mixing depth (`biomix` row of `tillage.til`; 0.2 and 50 mm when the file
  has none, nothing without the file) exchange a share of their soil water, weighted by a
  temperature factor from `carbon.bsn`. Two oddities: every layer's mass uses the first layer's
  bulk density, and a layer only partly above the mixing depth uses the temperature of the layer
  above it. The soil water total isn't summed again after mixing.
- **`sw_init` moves only on printed days.** `hru_output` starts the next period from today's soil
  water only when it prints the day, so with a print interval over 1 `sw_init` and `sw_ave` span
  the whole interval. Skipped years (`nyskip`) don't print and start the balance in the first
  printed year (`basin_sw_init`).
- **Two values for one name.** `es_max` is changed before `ep_max` is computed from it when PET
  limits both (`et_act`).

## Plants (stage 2b)

- **A plant name not in `plants.plt` takes the first plant** (`readpcom` leaves the pointer at its
  default, 1). A land use whose plant community isn't in `plant.ini` has no plants at all.
- **Harvest index above 0.7 becomes 0.02** (`plantparm_init`); Ames' pasture has 0.9.
- **Roots start at the depth of `soils.sol`'s `zmx`.** `plant_init` caps the first rooting depth
  with the soil profile's maximum from the file; the HRU's own (the bottom of its last layer)
  is set later by `hydro_init`.
- **Dormancy is first decided for day 0.** `hydro_init` runs before the clock is set, so the day
  length it tests is that of day 0.
- **The demand of a plant that wakes up is stale.** `pl_nut_demand` should zero each plant's N
  and P demand (`uno3d`, `uapd`) but zeroes the element after the last plant (the loop counter
  left over). These arrays belong to the module, not to the HRU: a plant still dormant when its
  demand is computed, and woken by `pl_dormant` the same day, grows with the demand last
  computed for any HRU, and its stress is measured against that HRU's optimum (`un2`, `up2`).
  (`swat-engine/src/hru/plant.rs::Saved`)
- **Temperature stress is always a sine.** `pl_tstr` computes the classic SWAT stress, then
  replaces it whenever `rto > 0 .or. rto < 2`, which is always true:
  `strst = sin(1.5707 * (tave - tbase) / (topt - tbase))`, kept within 0–1.
- **Water uptake skips `epco` in the second layer** (`pl_waterup` tests `k == 2`, not 1).
- **A perennial growing on temperature drops leaf area but no leaves.** Its leaf turnover
  (`leaf_tov`) is only set for plants growing on moisture, so `pl_leaf_senes` moves no mass to
  the residue. Leaves dropped at dormancy carry no carbon (`leaf_drop%c` is never set).
- **The heat index uses the weather of object j** (`ob(j)%wst` with j the HRU number); with
  HRUs first in the object list, that is the HRU's own station.
- **Base heat units reset at the end of the year in the northern hemisphere and on day 181 in
  the southern** (`time_control`) — Indonesian projects reset in July.

## Nutrients and erosion (original carbon model, `cswat` 0)

- **The top layer mineralizes at the second layer's temperature and water** (`nut_nminrl`
  takes `kk = 2` for `k = 1`).
- **`rsd_nitorg_n` and `rsd_laborg_p` print 0.** `rsd_decomp` sums them, then `nut_nminrl`,
  called right after, zeroes them.
- **Soil mixing at the end of each year.** With `cswat` 0, `time_control` calls
  `mgt_newtillmix_cswat0` with the hydrology's `biomix` (0.2 in Ames) on the last day: the
  layers above 50 mm exchange that share of their water, nitrogen, phosphorus and humus, and
  part of the surface residue goes into them. (`hru/nutrient.rs::year_end_mixing`)
- **The C factor method is always 1.** `ero_cfactor` sets `bsn_cc%cfac = 1` before testing it,
  so the C factor comes from residue and biomass, whatever `codes.bsn` says.
- **The snow test on sediment is inverted** (`ero_ysed`): with snow only yields under 1e-6 are
  zeroed; without snow the yield is divided by `exp(0)`.
- **The sediment store is emptied twice.** `stor_surfstor` takes the released sediment from its
  lag store right after computing it and again with the other constituents, so the store goes
  negative after a day with sediment and the next day's release is smaller.
- **Mineral P on sediment is swapped** (`nut_psed`): the "active" share is computed from the
  stable pool's size and the "stable" share from the active pool's.
- **Missing `nutrients.sol`** (Ames names it but doesn't have it) means SWAT+'s defaults:
  7 ppm nitrate and 5 ppm labile P at the surface, decreasing with depth.
