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
- **A row stops at a value that doesn't convert.** When a value of a row can't be read as its
  variable's type, gfortran's read ends with an error there, the variables after it keep their
  defaults, and the next read starts on the next line. Batanghari's `plants.plt` (SWAT+ Editor
  4.1.0) has 53 values and a description per row where `plant_db` has 56 variables: the
  description fails as the 54th, so the three lignin fractions keep their defaults and each row
  is one line. Reading on into the next line instead shifts every plant after the first.
  (`list::Records::read_checked`)
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
  left over; element 1 for an HRU without plants). These arrays belong to the module, not to the HRU: a plant still dormant when its
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

## Carbon model (`cswat` 2)

- **Mineralized N never reaches the mineral pools.** `cbn_zhang2` moves N between litter,
  microbes and humus (`nut_np_flow`) and subtracts the mineralized part from the source pools,
  but adds it to neither nitrate nor ammonium. Mineral N changes only by denitrification and by
  a transfer of nitrate to ammonium when the day's N balance has a surplus.
- **`nut_np_flow` drops the N released with CO2:** it computes it (`EfromCO2`) and adds a
  different variable, always 0 (`efco2`).
- **The top layer's microbial N:C ratio ignores residue N above 2 %**: the test for it is
  followed by an `if/else` whose `else` overrides it with 0.05.
- **The top layer's decomposition uses the second layer's depths for its oxygen factor**, and
  layers below the first set the shared allocation `org_allo(2)%abco2` from their own sand.
- **Organic N and carbon only leave on days with sediment.** `nut_orgnc2`, which also moves
  microbial carbon down the profile with percolation, runs only when there is runoff, peak flow
  and precipitation. Its microbial carbon lost with sediment (`c_microb_sed`) is kept from the
  last time it was computed, for any HRU, and taken again every time it runs. The residue N it
  removes is that of plant 1, and the basin residue total isn't updated.
- **Biological mixing mixes texture and pools every day** (`mgt_biomix`): clay, silt and sand,
  mineral N and P, humus, microbes and litter of the thawed top layers, but not the structural
  litter's non-lignin part (`nonlig`), which keeps its own value. The surface residue mixed in
  is counted once per mixed layer. The mixing's effect on decomposition (`mgt_tillfactor`,
  `idc_till` 3) uses an efficiency that, once reduced for the layer the mixing depth ends in,
  stays 0 for the layers below.

## Plant communities

From `pl_community`, `pl_grow` and the routines they call, with a community of several plants
(Ames's `cosy_comm`, corn and soybean); the port is `swat-engine/src/hru/plant.rs`.

- **Water goes to the plants in community order**, each taking its share of the maximum
  transpiration (by leaf area) from what the plants before it left.
- **Nitrogen and P taken up are a running total for the HRU** (`nplnt`, `pplnt`, and the fixed N
  `fixn`): a second plant's uptake by layer starts from the first's total, and adds it again to
  its own N, as does a non-legume after a legume with the fixed N.
- **Leaf growth is shared by canopy height** (`pl_leaf_gro`), but the sum uses the growing
  plant's leaf area with every plant's height, so each plant's share is its height over the sum
  of heights.
- **The salt stress printed is summed** over the growing plants, the other stresses averaged.
- **Sediment takes plant 1's surface residue N once per plant** (`nut_orgnc2`), and no other
  plant's.

## Decision tables

From `conditions`, `cond_real` and `actions` as `hru_control` calls them (Ames's
`irr_year_irr`); the port is in `swat-engine/src/hru/mgt.rs`.

- **They run first thing in the day**, before the stresses are reset: water stress is
  yesterday's, as are leaf area and heat units.
- **The yearly irrigation is never reset** (`hru%irr_yr`; its reset is commented out), so
  "irrigation this year under 500 mm" holds until 500 mm have been applied over the whole run.
  Ames reaches it in its second year (1976); after that only `mgt_01`'s one dated irrigation
  waters, when the 41-year rotation comes round to it again in 2018.
- **Heat units are those of the first plant that has any**, else plant 1's; the condition's
  limit variable (`phu_mat`) isn't used.
- **The action count starts at 1** and restarts at 1 every year, so `const2` 20 allows 20
  applications a year (`const2` is at least 1). Of two tables with the same name the last is used.
- **Generic plant-and-harvest tables** (`pl_hv_summer1`, `pl_hv_winter1`, `pl_hv_summer2`, as
  the Editor writes them for Batanghari's crops): `management.sch` gives the crop after the
  table's name, and `plant` and `harvest_kill` act on that crop instead of the table's option
  (on the rotation year's for `pl_hv_summer2`).
- **Planting by a table doesn't reset the plant** (no `mgt_plantop`): it only starts growing.
  `plant` has no yearly limit; `harvest_kill` harvests and kills the named plants whether they
  are growing or not, and, unlike a dated `hvkl`, leaves the base-zero heat units alone.
- **`phu_base0` is the weather station's base-zero heat units**, reset each year; `soil_water`
  compares the profile's water with field capacity, porosity or 0 combined with the limit
  (`lim_op` `*`, `+`, `-`, `/`); `jday` and `year_rot` compare integers. The rotation year
  starts at the community's `rot_yr_ini` and moves on at the end of each year in every HRU with
  decision tables.

## Custom soil layers (`soil_lyr_depths.sol`)

From `soils_init`; the port is `custom_layers` in `swat-engine/src/hru/mod.rs`.

- **The profile is cut into 1 mm slices**, by whole millimetres (the depths are truncated to
  integers), each with its `soils.sol` layer's properties, and each new layer averages its
  slices one by one in single precision (so a uniform layer's bulk density of 1.26 comes out a
  bit off).
- **Depths of 10 mm or less are skipped**; a 10 mm top layer is always first. The first depth at
  or past the profile's bottom becomes the bottom; if the list ends before the bottom, the
  profile ends there. The file is read whatever `file.cio` says, by that name.

## Management (dated operations)

From `mgt_operatn`, `mgt_sched`, `plant_init`, `time_control` and the operation routines; the
port is `swat-engine/src/hru/mgt.rs`, with tillage in `hru/carbon.rs`.

- **Operations run on their month and day,** in schedule order, after evapotranspiration and
  before runoff, and only until the year is skipped. A `skip` (month 0) never matches a date: the
  schedule waits on it until the year ends, when `time_control` steps past it. At the end of the
  list it starts over.
- **The first operation is one before the one wanted.** `plant_init` looks for the first
  operation of the starting rotation year on or after the start day and keeps its index minus one
  (at least 1). For a schedule starting on day 1 that is operation 1 anyway. An HRU with a
  schedule but no plants keeps the default, 1.
- **A fertilizer operation names the fertilizer first, then the application method**
  (`chem_app.ops`). Ames's `mgt_01` gives them the other way round (`null elem_n`), so its
  fertilizer is element 0 of `fertilizer.frt`, all zeros: nothing is applied. A method that isn't
  found (Ames has no `chem_app.ops`) has no surface share, so everything goes into the second
  layer. With the original carbon model, organic P sets the first two layers' active humus P to
  the stable humus P plus the fertilizer's.
- **Harvest and kill** (`hvkl`, grain): the seed is the yield times the harvest efficiency; what
  the harvest misses goes to the top layer's active humus (`hact`), which the carbon model never
  reads. The kill puts the rest above ground into the surface residue and the roots into the
  layers' residue by the root distribution, and takes the plant out of the community totals,
  which, for the rest of the day (runoff and erosion), still count the harvested seed. Stresses,
  heat units and the HRU's base heat units are reset for every growing plant, whichever one the
  operation names.
- **Planting caps the soil's rooting depth for good** (`mgt_plantop`): `zmx` becomes the
  smaller of itself and the plant's maximum root depth.
- **Annual plants' heat units** (`plant_init`, `days_mat` > 0): planting is taken as the day
  base-0 heat units reach 15 % of the year's; maturity sums `days_mat` days from there, the days
  past 31 December taking December's temperature (`xmon` leaves the month as it was).
- **Tillage with the carbon model** (`mgt_newtillmix_cswat1`): the layers above the tillage depth
  give a share of their soil (clay, silt, sand, rock), water, mineral N and P, and pools to a mix
  spread back by thickness. Every layer's mass is taken with the first layer's bulk density. The
  surface residue joins the mix once per mixed layer, but leaves the surface layer by layer, each
  time a share of what is left: less leaves than goes in.
- **Tillage's effect on decomposition** (`mgt_tillfactor`, `idc_till` 3) is weighted in the
  layer the depth ends in by the share of the layer *below* the depth. For `till_eff_days`
  (100 in Ames) the factor decays every time `mgt_tillfactor` runs, which after a tillage is
  every day the biological mixing runs. That mixing stops on the day of a tillage and comes
  back by `bio_consf` of its full value a day.

## Real projects: the tropics and the SWAT+ Editor (Batanghari, Pessel)

Found while building the two West Sumatra test cases (`testdata/Batanghari`, `testdata/Pessel`).
They are SWAT+ behaviour, so the port reproduces them; the test cases carry the work-arounds in
their inputs, listed in each `README.md`.

- **Perennials never go dormant below 20° latitude.** `cli_initwgn` sets the day-length
  threshold for dormancy (`wgn_pms%daylth`) to -1 hour when `|lat| < 20` and `dorm_hr` is 0, so
  `pl_dormant` never starts dormancy, and dormancy is the only place a `temp_gro` perennial's
  `phuacc` goes back to 0. Once `phuacc` passes `dlai` (`hu_lai_decl` in `plants.plt`, 0.99 for
  forest and oil palm), `pl_leaf_senes` brings leaf area down to `lai_min` within 15 days and it
  stays there for the rest of the run. ET falls to about half of PET and runoff rises with the
  wetter soil. The SWAT+ Editor caps `hu_lai_decl` at 1, so the test cases set it to 20 in
  `plants.plt` after the Editor writes the inputs (about one heat unit accrues a year).
- **`perco` is a steep function.** `topohyd_init` turns `hydrology.hyd`'s `perco` into the
  limit on drainage out of the profile bottom: `perco_lim = exp(-(1.0052 ln(-ln(perco - 1e-6))
  + 5.6862))`, at most 1. perco 0.05 gives about 0.001, 0.5 gives 0.005, 0.8 gives 0.015, 0.95
  gives 0.067. A tile-drained HRU has perco forced to 0.1 and `cn3_swf` to 0.95.
- **The Editor writes `perco` by hydrologic group**: 0.5 for group C and 0.05 for group D in
  both projects, and `cn3_swf` 0.95. With 0.5 almost all percolation stays in the profile,
  the soil is near saturation, and curve numbers stay high.
- **`dp_tot` in `soils.sol` is not the root depth.** `hydro_init` sets `soil%zmx` to the depth
  of the bottom layer, whatever `dp_tot` says.
- **HWSD Histosols come with an empty subsoil.** The Editor's `global_usersoil` gives Od19's
  second layer `awc` 0 and `soil_k` 0, an impermeable layer at 30 cm. Pessel replaces both.
- **`calibration.cal` object lists.** The number before the list (`obj_tot` in the header) is
  the number of list entries, not of objects: `cal_parmchg_read` reads it as `nspu` and then that
  many integers, where a negative entry closes a range (`1 -111 113 115 -1123` is 1-111, 113,
  115-1123). The Editor lists only HRUs that have a land use.
- **QSWAT+ turns `WATR` into `WETW`.** Open water cells become wetland HRUs (`wetland.wet`,
  surface storage). Cells without a land use the Editor knows get `lu_mgt` `null` in
  `hru-data.hru`.
- **Wetland HRUs evaporate little.** In Pessel the wetland HRUs (`wetl`, `wetf`, `wetw`) return
  about 375 mm a year to the air against a PET of about 1,300 mm; almost all rain leaves as
  `wet_oflo`. Reproduced as is.

## Hard calibration (`calibration.cal`)

- **Applied once, after the HRUs are set up** (`proc_cal` → `cal_conditions`), so a new `perco`
  recomputes `perco_lim` with its own threshold (1e-6, where `hydro_init` uses 1e-9).
- **The object list** is the count of its entries, then the entries, where a negative number
  closes a range (`2  1 -1858` is HRUs 1 to 1858; `define_unit_elements`). A count of 0 means
  every object of the parameter's kind. Changes are held to `cal_parms.cal`'s range.

## Arithmetic

- **gfortran folds `Sin(Atan(x))` into `x / Sqrt(x*x + 1)`.** The LS factor's `sin_sl`
  (`plant_init`) is computed that way in the SWAT+ build, which rounds differently from
  `sinf(atanf(x))` for about two slopes in five (checked on all 7,562 slopes of Batanghari and
  Pessel). One bit of the USLE factor moves sediment yield by a bit, the organic N it carries off
  the top layer, and, in a nitrogen-limited crop, growth: two of Batanghari's 1,858 HRUs drifted
  by up to 2 % in biomass until the port did the same.

- **`x ** 0.5` is the math library's `powf`**, not a square root: gfortran calls `powf`, while
  Rust's optimizer turns `x.powf(0.5)` into `sqrt`, which is correctly rounded where `powf` isn't
  always (they differ in the last bit for about one value in 1600). Hargreaves PET has it; one
  bit of PET was enough to move a soil layer's water by one bit a few days later and a printed
  digit weeks after. The port calls `powf` with the exponent hidden from the optimizer
  (`fortran::rpow`).
