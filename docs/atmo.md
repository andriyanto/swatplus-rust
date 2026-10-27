# Atmospheric deposition and Muskingum routing (phase 4)

## Nitrogen deposition (`atmo.cli`)

`cli_read_atmodep` reads the stations of the file `file.cio` names in its `climate` row; each
weather station of `weather-sta.cli` points at one by name (`atmo_dep`). Every day `nut_nrain`
adds to each HRU's top soil layer the nitrate and ammonium of its station: in rain
(`0.01 · mg/L · precipitation` kg/ha) and dry (the yearly kg/ha over 365 days, or a month's over
its days). The port follows SWAT+ bit for bit, with the three time steps of the file (`aa`, one
value; `yr`, a value a year; `mo`, a value a month).

- **The time step moves on at the start of the last day of a month (or year)**: the
  end-of-month flag is set before `cli_atmodep_time_control`, so the last day of each month
  takes the next month's deposition.
- **Past the end of the data nothing is added, and `hru_nb` prints the last day's values
  again** (`hnb_d` isn't zeroed). The code in `codes.bsn` (`atmo_dep`) isn't used: the file
  decides.

**Ps_atmo** (`scripts/make-pessel-variants.py`) is Ps_coast with monthly deposition from
2015 to 2017 at two stations, inland and coastal, given to its weather stations in turn: in rain
0.15 and 0.30 mg/L of ammonium N, 0.25 and 0.30 mg/L of nitrate N; dry 1.5 and 2.5 kg/ha a year
of ammonium N, 1.0 and 1.5 of nitrate N, three times as much in the haze of September–October
2015. The values are assumed for the test (no measured series of Pesisir Selatan was found); at Kototabang,
West Sumatra's global background station, nitrate is the second ion of rain and ammonium among
the least (Budiwati 2007). Every table matches SWAT+; the yearly and average annual forms of the
file were checked the same way.

## Muskingum routing (`rte_cha` 1)

SWAT-DEG channels route by the variable storage method unless `codes.bsn` asks for Muskingum.
The coefficients come from the storage time of the channel at bankfull and a tenth of it and,
when a day is longer than the stable step (`2 K (1 − x)`), from that step: the day is cut into
`Int(24 / step) + 1` parts for the coefficients, but a daily run then routes the day in one step
with them (`ch_rtmusk` resets the steps). **Ps_musk** (Ps_coast with `rte_cha` 1) matches SWAT+
on every table.

## Source

- Budiwati, T. (2007). Wet deposition at urban and rural sites of Indonesia. In *Polusi udara:
  model dan aplikasinya*, LAPAN, Bandung, 27–35. https://karya.brin.go.id/id/eprint/17650/
