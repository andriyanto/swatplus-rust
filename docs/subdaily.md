# Sub-daily steps and Green-Ampt infiltration

With a `time.sim` step above 1 and `codes.bsn` `gampt` 1, SWAT+ runs the land and the channels by
time step (`time%step` a day, `time%dtm` = 1440 / step minutes). The port follows SWAT+
6f51536 bit for bit for steps up to 96 a day (15 minutes):

- **Precipitation by step** (`cli_pmeas`, `cli_precip_control`): a precipitation file whose
  `tstep` is above 0 gives a value a line (year, day, month, day of month, hour, value); the
  day's total is their sum.
- **Green and Ampt infiltration** (`sq_greenampt`) replaces the curve number: the infiltration
  rate from the top layer's conductivity, adjusted by the curve number, and the wetting front's
  suction (`wfsh`, from the top layer's porosity, clay and sand, Rawls and Brakensiek). Its
  runoff by step is lagged step by step (`sq_surfst`), and the curve number is updated each step
  for the printed `cn`.
- **Splash and overland erosion by step** (`ero_ovrsed`): from the step's rain intensity and
  kinetic energy, the canopy, the runoff depth and the bed shear, with `parameters.bsn`'s
  `splash`, `rill`, `surq_exp` and `cov_mgt`, lagged by step (`stor_surfstor`).
- **Hydrographs by step**: each HRU's surface runoff (moved on by its time of concentration),
  each routing unit's, the inflows by step of every object (`command`), and the SWAT-DEG
  channels routed step by step (`ch_rtmusk`, each step cut into its stable sub-steps).
- **`channel_sd_subday.txt`**: each channel's outflow in each step, written with the daily
  channel table. The values are volumes per step (m3), though the table heads them `m^3/s`:
  divide by `dtm · 60` (3,600 for hourly steps) for a rate.

- **Urban HRUs by step** (`sq_greenampt`, `hru_urbanhr`): the directly connected impervious
  share's runoff by step is the station's rain less what is left of the initial abstraction
  (`parameters.bsn` `urb_init_abst`, taken by rain above 0.017 mm/h and refilled by the PET
  between storms); each step whose impervious runoff is above 0.025 mm/h washes the built-up
  solids off, and their nutrients replace that share of the HRU's.

- **The curve number with sub-daily routing** (`gampt` 0, `unit_hyd`, `flow_hyd_ru_hru`): the
  HRUs keep the day's curve number runoff, and each HRU's and routing unit's surface runoff is
  spread over today's and tomorrow's steps by a unit hydrograph of its time of concentration
  (`codes.bsn` `uhyd` 0 triangular, 1 gamma with `parameters.bsn`'s `uhyd_alpha`; a base time of
  `0.5 + 0.6 tc + adj_uhyd` hours, at most 48, peaking at 3/8 of it); the channels are routed by
  step as with Green-Ampt.
- **Generated rain by step** (`cli_pgenhr`), for a station without a precipitation file
  (`sim`) or a gap (-99) in a sub-daily file: the day's rain as a storm rising and falling
  exponentially around its peak, the peak rate twice the half-hour fraction of the generator,
  lasting up to 24 hours.

## Quirks

- **The steps of a day come from the year being simulated**, whatever the day: on the last day
  of a year the next day's steps are read from this year's column (`tss(:, 1, time%yrs)`), so
  1 January of the second and later years gets the hours of 1 January of the year before. The
  steps fill a day in the file's order whatever its hour column says.
- **The canopy takes its water from the steps twice, and not from the day's rain**: the steps
  are reduced as the canopy fills, then again by all it took (against the station's rain), while
  the infiltration (`inflpcp = precip_eff − surfq`) still counts the day's whole rain.
- **Runoff added after the surface step (saturation excess) never reaches the hydrograph**: the
  day's runoff (`qday`) is the steps' lagged runoff, set before percolation.
- **A wetland's or paddy's release is spread evenly over the steps**, and `res_hydro` takes each
  decision-table action once a step: a constant rate, a share of the principal volume, of the
  inflow or of the volume above the emergency spillway is released `step` times a day, while a
  drawdown over `n` days is taken in `step` parts (and `dyrt`'s first term `step` times). The
  same holds for reservoirs.
- **The sediment of the day by MUSLE still sets the nutrients**, but what leaves is the steps'
  erosion; the day's sediment store takes the MUSLE yield once a step and gives back the steps'
  total once.
- **A routing unit's hydrograph is its last HRU's** plus that HRU's runoff by step converted to
  m3/s (each HRU element replaces it), moved on by a time of concentration fixed at 2 h; its
  lateral and tile flow, and aquifers', reservoirs' and outlets' flow, are spread evenly over the
  steps.
- **A channel's inflow rate for its rating curve is the step's volume over a day** (`/ 86400`),
  while the outflow's is over the step; `inflo` is kept from the last channel computed when a
  channel has no inflow.
- **The QUAL2E rates are divided by the steps a day** (`ch_read_nut`), but the channel water
  quality still runs once a day, so algae, nitrogen and oxygen change `step` times more slowly.
- **By step, solids are washed off whatever the land use's `urb_ro`** (`usgs_reg` too), and
  each step's wash-off replaces the impervious share of the day's nutrients again, on top of
  the earlier steps'. The sediment is not replaced, and the nitrate (`tno3`) is an integer:
  only whole kilograms per hectare count, so in practice the impervious share has none and its
  organic N is the whole of its total N.

### Generated rain

- **A generated station has no rain at all**: `cli_pgenhr` zeroes today's steps before it
  spreads today's rain into tomorrow's, and tomorrow's rain is taken as today's steps' sum.
- **A gap generates the day before it**: the file is read a day ahead, so at a gap today's
  steps are zeroed (today keeps its daily total) and today's rain fills tomorrow's steps from
  the first, the file's hours after the storm staying as read. Tomorrow's daily total is still
  the file's sum, gaps and all: negative, so no rain that day (the climate adjustment floors it
  at 0, and the gaps in the steps too).
- **Every storm has the same shape**: its peak's position comes from a random number whose seed
  is set to 8 each time.
- **The storm starts at midnight** and uses yesterday's half-hour fraction (today's is
  generated afterwards).

### With the curve number

- **The rain of a daily file never reaches the steps**: `cli_pgenhr` only spreads generated
  rain, so a station's steps stay dry. The canopy, which takes its water from the steps in a
  sub-daily run, then intercepts nothing (and evaporates nothing from the canopy).
- **No sediment leaves the HRUs**: by step, the sediment lag (`stor_surfstor`) releases the
  steps' erosion, which only Green-Ampt computes (`ero_ovrsed`); the day's MUSLE sediment goes
  into a store that is never released (and its nutrients still leave by the day).
- **The unit hydrograph carries only the surface runoff**: `flow_hyd_ru_hru` is given the
  lateral and tile flow but doesn't add them; they reach the channels evenly over the steps, with
  the total hydrograph.
- **A day's steps of the unit hydrograph stop once its rate falls under 1e-4**, but the next
  day's are still computed.

## Validation: Ps_hourly

None of the projects has sub-daily rain, so **Ps_hourly** (`scripts/make-pessel-variants.py`)
is Ps_coast run hourly with Green-Ampt, its 37 CHIRPS precipitation files spread into hours:
each day's rain falls in a four-hour storm from 15:00, by Mononobe's distribution (the depth by
hour `t` is `R (t / 4)^(1/3)`: 63 %, 16 %, 12 % and 9 % of the day). Four hours is the dominant
duration of heavy rain that Mononobe's distribution matches best in Java (Citanduy), and over
Sumatra's land convective rain peaks between 15:00 and 20:00 (Mori et al. 2004). Every table
matches SWAT+, the 1.9 million hourly channel flows of `channel_sd_subday` among them. At the
largest outlet (cha0315) the wettest hour of 23 November 2017 carries 10,430 m3, 1.45 times
the day's mean hour; upstream channels peak up to 12 times their day's mean.

**Ps_urbhr** is Ps_hourly with its generic farmland (two HRUs) a medium-density town
(`urmd`, 38 % impervious, 30 % directly connected) by build-up and wash-off. Every table matches
SWAT+; those HRUs' surface runoff rises from 15 to 939 mm a year, their organic N from 5 to
31 kg/ha.

**Ps_gap** is Ps_hourly with the hours from 17:00 of three rainy days of 2016 missing from one
station's file, and another station generated. Every table and hourly flow matches SWAT+; the
generated station's HRUs get no rain in three years, and the gap station loses the daily rain of
the gap days and the hours of the days before them.

**Ps_uh** is Ps_coast run hourly with the curve number (its daily precipitation, `gampt` 0,
the gamma unit hydrograph). Every table matches SWAT+, and so do its 1.9 million hourly channel
flows. Without interception the basin's ET falls from 1,253 to 1,170 mm a year and its surface
runoff rises from 598 to 622 mm; the HRUs' sediment yield is nil. A channel's wettest hour
carries up to 13.6 times its day's mean (5 for the median channel; 4.2 at the largest outlet).

## Sources

- Mori, S., Hamada, J., Tauhid, Y. I., Yamanaka, M. D., Okamoto, N., Murata, F., Sakurai, N.,
  Hashiguchi, H., & Sribimawati, T. (2004). Diurnal land–sea rainfall peak migration over
  Sumatera Island, Indonesian Maritime Continent, observed by TRMM satellite and intensive
  rawinsonde soundings. *Monthly Weather Review*, 132(8), 2021–2039.
  https://doi.org/10.1175/1520-0493(2004)132<2021:DLRPMO>2.0.CO;2
- Analisis pola distribusi curah hujan lebat dominan di DAS Citanduy Hulu (2024). *Jurnal
  Sumber Daya Air*. https://doaj.org/article/aa3fe4224261494382269316b1eb7266
