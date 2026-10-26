# Output tables

`swatplus-rust run` writes SWAT+'s text tables for the objects and periods `print.prt` asks for
(`y` in the daily, monthly, yearly or average annual column), with the columns the port computes,
in SWAT+'s layout and numbers (validated bit for bit, see `docs/phase3.md`):

| `print.prt` row | Files |
|---|---|
| `hru_wb`, `hru_nb`, `hru_ls`, `hru_pw` | `hru_*_day`, `_mon`, `_yr`, `_aa` |
| `ru` | `ru_*` |
| `aquifer` | `aquifer_*` |
| `channel_sd` | `channel_sd_*` |
| `basin_wb`, `basin_nb`, `basin_ls`, `basin_pw` | `basin_*` (needs `ls_unit.ele`) |
| `lsunit_wb`, `lsunit_nb`, `lsunit_ls`, `lsunit_pw` | `lsunit_*` (needs `ls_unit.def`) |
| `basin_aqu` | `basin_aqu_*` (needs `aqu_catunit.ele`) |
| `basin_sd_cha` | `basin_sd_cha_*` |
| `crop_yld` flag (`y`, `a`, `b`) | `basin_crop_yld_yr`, `_aa` (`y`, `b`); `crop_yld_aa` (`a`, `b`) |

Daily rows follow the print window and interval of `print.prt`; periods are printed after the
warm-up years (`nyskip`).

How SWAT+ sums the periods, and the port with it (`swat-engine/src/periods.rs`):

- **Fluxes are summed, states averaged.** A month's soil water, curve number, snow, wetland
  storage, leaf area, biomass, residue, temperatures and weather are divided by the month's days
  (all of them, even in a month partly simulated); a year's by its last day (`day_end_yr`); the
  average annual's by all printed days. Fluxes are divided by the years printed for the average
  annual only.
- **The years printed (`yrs_prt`) are days divided by 365** (plus the leap years over the years,
  in integers, so 0 unless all are leap years); during warm-up years `yrs_prt` is set to the
  number of years simulated, so with `nyskip` > 0 it is that number plus the printed days.
- **An HRU's monthly soil water and snow are averages of the end of each day**; its daily table
  prints the average of start and end. The basin and landscape units sum the daily averages.
- **Plant N and P (`nplt`, `pplnt`) in an HRU's periods are the last day's**; the basin's are
  averaged, and its `pplnt` is always 0 (`hruout_plantweather_add` leaves it out), as are a
  landscape unit's.
- **The maximum biomass (`bm_max`)** is the period's largest; the average annual's leaves out
  the last day of each year (it is restored to the previous day's value).
- **Aquifers** average storage, water table and nitrate stored over a month's days, then over 12
  months; the carbon is the last day's.
- **Channels** average only the flows (not the loads) over a period's days; storage and water
  temperature are the last day's; the average annual water area is divided by the years twice.
  The basin's channels add the daily values without averaging the flows.
- **Landscape units and the basin** weight each HRU by its `ls_unit.ele` shares (`sub_frac`,
  `bsn_frac`); the basin reads HRU `obtypno` of element `i` for HRU `i`, and the landscape units
  read element numbers as HRU numbers.
- **Crop yields**: an HRU plant's average annual yield is its yields summed over the whole run,
  warm-up included, divided by its harvests; the basin's per year by area. A decision table's
  harvest-only action doesn't count a harvest (it adds to `harv_num_yr` twice).
