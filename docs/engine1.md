# Engine 1.0: the remaining features

Features ported for engine 1.0 that need no document of their own. Landscape routing is in
[`routing.md`](routing.md), urban HRUs by step in [`subdaily.md`](subdaily.md).

## PET "read in" (`codes.bsn` pet 3)

With `pet` 3, each HRU takes its weather station's PET (`et_pot`) instead of computing one, and
`climate_control` reads the station's measured PET from the files of `pet.cli` (`cli_petmeas`;
a file a station, named in `weather-sta.cli`'s `pet` column, with a year, a day and a value a
line), falling back on Hargreaves' outside the file's years.

- **The measured PET is never used**: `climate_control` then computes every station's
  Hargreaves PET for the climatic moisture index, into the same variable, so `pet` 3 is
  Hargreaves' equation on the station's (not lapse-adjusted) temperatures, at least 0.01 mm a
  day.

**Ps_pet** (`scripts/make-pessel-variants.py`) is Ps_coast (Penman-Monteith) with `pet` 3 and a
measured PET of a constant 4 mm a day (1,461 mm a year) for every station. Every table matches
SWAT+; the basin's PET is Hargreaves' 1,252 mm a year, against Penman-Monteith's 1,318.

## Hard calibration with conditions (`calibration.cal`)

`cal_conditions` changes a parameter for each object of its list (or of its kind) that meets
the change's conditions, all of them: the HRU's soil hydrologic group (`hsg`) or texture, its
land use (`landuse`) or the land use's calibration group (`cal_group`), or its plant (`plant`).
A condition line is `variable operator number text` (`hsg = 0 D`). The port changes these
parameters, with what SWAT+ recomputes after each:

- **HRU**: `cn2` and `cn3_swf` (the curve numbers again), `perco` (its limit), `esco`, `epco`
  (and each plant's), `canmx`, `petco`, `latq_co`, `lat_len`, `lat_ttime`, `erorgn`, `erorgp`,
  `biomix`, `slope`, `elev`, `field_wid`, the snow parameters (`snofall_tmp`, `snomelt_tmp`,
  `snomelt_max`, `snomelt_min`, `snomelt_lag`) and `usle_p`;
- **soil**, layer by layer from `soil_lyr1` to `soil_lyr2` (0 for the first and the last):
  `z`, `bd` and `awc` (the soil's water contents again, and the curve numbers), `k` (the
  percolation time), `rock` and `usle_k` (the USLE factor), `alb`, `cal`, `anion_excl`;
- **basin**: `surlag`;
- **aquifer**: `alpha`, `deep_seep`, `revap_co`, `revap_min`, `flo_min`, `sp_yld` (the storage
  again), `hlife_n`, `no3_init` (the nitrate stored), `dep_bot`, `minp_init`.

A parameter SWAT+ only uses while setting up (`ovn`, `slope_len`, `field_len`, `ec`, `ph`, an
aquifer's `flo_init_mm`, `dep_wt_init`, `cbn_init`, `flo_dist`, and `bf_max` without 2-D
aquifers) is accepted and changes nothing; any other parameter, conditions on reservoirs
(`res_pvol`, `res_typ`), on a plant's class (`pl_class`) or on an aquifer's change are refused.

- **Conditions are tested on the HRU numbered as the object**, whatever the parameter's kind.
- **`plant` looks at the community's first plant only**, and an HRU without plants meets it.
- **`landuse` stops at the first condition that fails**; the others are all tested.
- **A soil parameter changes once a layer, but `anion_excl` belongs to the soil**: a `pctchg`
  or `relchg` of it is compounded as many times as there are layers changed.
- **`lat_ttime` is changed after it has become `1 − exp(−1 / t)`**, and then made so again.
- **Changes come after the set-up**: the slope's time of concentration and USLE LS keep the old
  slope, and only what is computed later sees the change. The soil's nutrients are set up after
  calibration (`soil_nutcarb_init` follows `proc_cal`), from the calibrated soil.

**Ps_cal** (`scripts/make-pessel-variants.py`) is Ps_coast with 24 more changes (`PS_CAL`):
curve numbers lowered on groups C and D, `esco`, `epco` on oil palm, `canmx` where oil palm is
the plant, `awc` on clay loams' top two layers, `k`, `bd`, `anion_excl`, `usle_k`, `alb`, the
lateral flow, the slope, `elev` on rice, `erorgn`, `petco` on group B oil palm, `surlag`, and six
aquifer parameters. Every table matches SWAT+: surface runoff falls from 598 to 541 mm a year
and percolation rises from 1,210 to 1,297 mm.
