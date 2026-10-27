# Reservoirs and point sources (phase 4)

The port runs SWAT+'s reservoirs (`reservoir.con`, `reservoir.res`, `hydrology.res`, a release
table of `res_rel.dtl`) and recall objects (`recall.con`, `recall.rec` and its files: point
sources, measured inflows), bit for bit with SWAT+ commit 6f51536, and writes their tables
(`reservoir_*`, `basin_res_*`, `recall_*`, `basin_psc_*`, daily to average annual).

## Validation: Bh_res

None of the reference projects has a reservoir or a recall object, so
`scripts/make-reservoir-variants.py` builds one from Batanghari (2015–2017, printed from the
first day). The reservoir and the point sources are hypothetical, sized as typical ones:

| Object | What | Values |
|---|---|---|
| Reservoir `res001` | at the head of channel cha037 (47,000 ha upstream): the two channels and the routing unit that flowed into cha037 flow into it, and it releases into cha037 | 30 million m³ over 300 ha at the principal spillway (10 m deep), 50 million m³ over 400 ha at the emergency spillway; bottom conductivity 0.05 mm/h; evaporation coefficient 0.6; starts 80 % full; settles to 50 mg/L of sediment (d50 10 µm); Batanghari's `flood_season` release table |
| Recall `mill`, daily | the treated effluent of a palm oil mill into cha060 | 45 t of fresh fruit bunches an hour for 20 h a day, 0.6 m³ of effluent a tonne: 540 m³ a day, less in the low season (February–April); after the ponds 100 mg/L BOD, 50 mg/L N (half ammonium), 10 mg/L P, 200 mg/L suspended solids, 30 °C |
| Recall `town`, yearly | a town's wastewater into cha100 | 20,000 people at 120 L a day through septic tanks: 876,000 m³ a year at 30 mg/L N, 5 mg/L P, 150 mg/L BOD, 100 mg/L suspended solids |

Every table matches: the reservoirs', the recalls', their basin sums, the 284 channels and all
the others. The reservoir keeps 47–53 million m³ and traps about 99 % of the sediment that
reaches it.

## Quirks

- **`hydrology.res` and the sediment and nutrient rows are taken for the reservoir's own
  number**, the release table and the initial water for the row its `reservoir.con` points at
  (`res_read` loops over the reservoirs with the row index). The SWAT+ Editor writes them in the
  same order, so they agree.
- **The volume-area curve's `lnvol` is an integer**: `log10(evol) − log10(pvol)` truncated, so
  for an emergency volume under ten times the principal one the exponent comes out huge and is
  held to 0.9, and the area is `(psa / pvol)^0.9 · V^0.9`.
- **`hydrology.res`'s `br1` and `br2` are the release lags** (`lag_up`, `lag_down`, as
  `exp(−lag)`), not the curve's coefficients, which are always computed.
- **A `vol` condition's `+` and `−` are in ha-m** (times 10,000 m³); `*` and `/` are shares.
- **Sediment only settles above its normal concentration**: at or below it the outflow carries
  none, and the classes (sand, silt, clay…) stay in the water.
- **The outflow's temperature is 0 °C** (it starts from an empty hydrograph and nothing sets it),
  which the channel below mixes in.
- **A reservoir with no inflow isn't run**, and before its first operational month it passes its
  inflow on.
- **`basin_reservoir_output` zeroes each reservoir's daily inflow and outflow** after adding them,
  so a reservoir that isn't run prints zeros.
- **A yearly recall puts the whole year's load in every day** (`recall%hd(1, yrs)` each day; its
  test of the years is `.or.`, so always true). A daily or monthly one gives nothing outside the
  years of its file. The files' flow is in m³ (the tables head it `m^3/s`).
- **`recall.rec` is read whether or not there are recall objects**, and stops at the first file
  that is missing or ends before its header (Osu_1hru's `RECYR.`).

Not ported yet (`run` says so): reservoirs released by condition tables (`ctbl_`), the weir,
measured (`meas`), lake and hydropower release options, reservoirs with lapse rates, subdaily
recalls and diversions (negative recall flows).
