# Dynamic soil phosphorus and urban regressions (phase 4)

## Soil phosphorus model 1 (`codes.bsn` `soil_p` 1)

With `soil_p` 1, SWAT+ moves phosphorus between the labile, active and stable pools with the
routines of Vadas and White (2010) (`nut_pminrl2`) instead of the fixed sorption ratio of
`parameters.bsn` (`nut_pminrl`), and sets the pools at the start from the soil (`soil_nutcarb_init`):

- **The sorption ratio (PSP) comes from clay, organic carbon and labile P**
  (`−0.045 ln(clay) + 0.001 P_lab − 0.035 C + 0.43`, held to 0.1–0.7) and is smoothed day by day
  (a 30-day running weight); the stable-to-active ratio comes from active P
  (`25.044 (P_act (1 + PSP/(1 − PSP)))^−0.3833`, held to 0.7–10 and smoothed over 100 days; at
  the start 1–7, from active and labile P).
- **The flow from labile to active slows with the days since the surplus began**, and both
  flows shrink in dry soil (down to a quarter of the rate below a quarter of field capacity).
- **The active-to-stable rate grows with calcium carbonate** (`0.0023 CaCO₃ + 0.005`, 0.002–0.05).
  `soil_phys_init` gives a layer without CaCO₃ 2.8 % (the average of 20,000 SSURGO soils), so a
  `soils.sol` with zeros there, as the editor writes, gets that.
- **A layer without clay keeps the last PSP computed at the start** (`psp` is a saved local of
  `soil_nutcarb_init`: the previous layer's, or the previous HRU's), and takes 0.4 every day.

**Ps_solp** (`scripts/make-pessel-variants.py`) is Ps_coast with `soil_p` 1. Its soils are rich in
organic carbon (7 % in the top layer of the rice soils), which holds PSP at its floor, 0.1. Every
table matches SWAT+.

## Urban loads by the USGS regressions (`urb_ro` usgs_reg)

An urban HRU whose land use asks for `usgs_reg` takes, on a day with more than 0.1 mm of
precipitation and of runoff, the suspended solids, total N and total P of a storm from the
regressions of Driver and Tasker (1990) (`regres`): a power function of the precipitation, the
impervious area and its share, with coefficients for three bands of annual precipitation (of the
weather generator: under 508 mm, under 1,016 mm, more). Those loads replace the impervious share
of the HRU's sediment (as `0.001` times the solids in kg), of its organic N and nitrate (70 % and
30 % of the N) and of its organic and soluble P (75 % and 25 % of the P).

**Ps_urban** is Ps_coast with its generic farmland (two HRUs) a medium-density town (`urmd`, 38 %
impervious) by the regressions; Pesisir Selatan gets well over 1,016 mm a year. Every table
matches SWAT+; on those HRUs surface nitrate rises from 0.16 to 3.0–3.5 kg/ha a year.

## Sources

- Driver, N. E., & Tasker, G. D. (1990). *Techniques for estimation of storm-runoff loads,
  volumes, and selected constituent concentrations in urban watersheds in the United States*.
  U.S. Geological Survey Water-Supply Paper 2363. https://doi.org/10.3133/wsp2363
- Vadas, P. A., & White, M. J. (2010). Validating soil phosphorus routines in the SWAT model.
  *Transactions of the ASABE*, 53(5), 1469–1476.
