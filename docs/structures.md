# Structural practices (phase 4)

## Tile drainage (`tiledrain.str`)

An HRU is drained when its `landuse.lum` row names a `tiledrain.str` row (`tile`). The port follows
SWAT+ bit for bit with `codes.bsn` `tdrn` 0 and `wtdn` 0 (the original SWAT tile equation and
the water table from soil water); the DRAINMOD forms (`tdrn` 1, `wtdn` 1) are refused.

- **`topohyd_init` sets a drained HRU's `cn3_swf` to 0.95 and `perco` to 0.1**, and calibration
  of either is then skipped for it.
- **The drain lies in the first layer whose bottom is deeper than the drain** (`ldrain`).
- **The water table is computed at the end of `swr_percmain`** from the soil water above field
  capacity, and the tile flow `(wt_shall − d) / wt_shall · (sw − fc) · (1 − exp(−24 / time))`,
  held to the drainage coefficient, is taken from the layers above field capacity.
- **Tile nitrate leaches with `nperco_lchtile`**, and the drain's share overrides the lateral
  and percolation shares; tile soluble P (`tilelabp`) is added to the surface runoff's.
- **Tile flow and its nitrate are lagged in `swr_substor`** by `1 − exp(−24 / lag)` before they
  reach the routing unit (with lateral flow, in the hydrograph's flow and nitrate).

Two variants test it:

- **Ames_tile** (`scripts/make-variants.sh`): Ames_sub1's corn and soybean on tile drains 1 m deep
  (`mw24_1000`), as much of Iowa's cropland is. It matches, but its soil water stays below field
  capacity over the drains, so no tile flow comes out.
- **Ps_tile** (`scripts/make-pessel-variants.py`): Ps_coast's oil palm on drains 0.6 m deep
  (`peat_canal`, 48 h to drain to field capacity, a 96 h lag, 20 mm a day), the depth at which
  the water table of drained peat under oil palm is kept (40–60 cm, Indonesia's peat regulation
  PP 57/2016 asks for at most 0.4 m). Every table matches SWAT+.
