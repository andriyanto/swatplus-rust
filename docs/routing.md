# Landscape routing between HRUs (engine 1.0)

An HRU whose `hru.con` row sends its water to another HRU (`hru` with `tot`, `sur`, `lat` or
`til`) routes it across the land: the receiving HRU gets it the same day, before its own
evapotranspiration (`command`, `hru_control`, `rls_routesurf`, `rls_routesoil`). The port
follows SWAT+ 6f51536 bit for bit:

- **HRUs are computed in the order of the commands** (an upslope HRU before the one it drains
  into), and their tables are written in the order of their numbers, as SWAT+ does.
- **The inflow is put per hectare of the receiving HRU** (amounts divided by its area, the water
  then by 10 for mm). `tot` from an HRU is split into its surface, lateral and tile parts; `rhg`
  sent to an HRU is ignored.
- **Surface runon into a dry HRU is rain**: added to the day's effective precipitation (and to
  each step's, at sub-daily steps), so it infiltrates or runs off with the HRU's own. The
  sediment it carries passes through up to the transport capacity
  `dep_co · usle_cfac · q · slope^1.4 · field width^1.4`, and is added to the HRU's yield.
- **Lateral runon goes into the soil**, into each layer in proportion to its thickness.

## Quirks

- **Tile runon is divided by the area twice**: `rls_routesurf` adds
  `hin_til%flo / (10 · area)` to the runon, though `command` already turned it into mm.
- **A wetland HRU takes the runon into its pond as it comes, per hectare**: the per-hectare
  amounts (mm, kg/ha) are added to a pond kept in m3 and kg, so a peat swamp downslope of oil
  palm barely notices it: in **Ps_land** the swamp, a fifth the size of the oil palm above it,
  gets some 1,600 mm a year of runoff from it and releases 2 mm more.
- **The same runon also seeps into the wetland's soil**: `ht1`, still holding it, is added to
  `swr_percmain`'s infiltration (divided again by 10 times the area), and its soluble P to the
  top layer's labile P (`nut_solp`, as kg/ha).
- **Water sent from other objects into an HRU, or into a routing unit, is refused** (SWAT+ adds
  it to the HRU's surface runon, or loses it in the routing unit).

## Validation: Ps_land

**Ps_land** (`scripts/make-pessel-variants.py`) is Ps_coast with three upland oil palm HRUs
taken out of their routing units and drained across the land into the HRU below them (with
their recharge still to the routing unit's aquifer): one into a peat swamp forest (wetland,
`tot`), one into dry fields (`tot`), and one into dry fields by surface and lateral flow
separately (`sur` and `lat`). Every table matches SWAT+. The dry fields' surface runoff rises
from 274 mm a year to 339 mm with 84 mm of runon from an upland HRU about their size, and to
784 mm with 593 mm of runon from one six and a half times their size.
