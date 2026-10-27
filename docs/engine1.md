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
