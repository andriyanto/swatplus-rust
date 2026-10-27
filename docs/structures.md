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

## Filter strips, grassed waterways and user-defined practices

`filterstrip.str` (`smp_filter`), `grassedww.str` (`smp_grass_wway`) and `bmpuser.str`
(`smp_bmpfixed`) take sediment and nutrients out of the runoff after `swr_substor`, in that
order; none of them changes the water. The port follows SWAT+ bit for bit, with these quirks:

- **A filter strip only works when its `flag` is 1**, but a grassed waterway and a user-defined
  practice work whatever theirs says.
- **The filter strip's trapped sediment is taken from the classes after the yield is reduced**
  (`sedtrap = sedyld · sed_remove / 100` with the new yield), so the classes lose less than the
  total.
- **The filter strip's soluble P uses the particulate P's second share**: the line meant for it
  writes `remove21`, so `remove2` is still the particulate one.
- **A grassed waterway's time of concentration lacks the overland time**: `structure_init` runs
  before `time_conc_init` has computed `t_ov`, so only the channel time counts.
- **`bmpuser.str`'s efficiencies are shares (0–1), not the percent of SWAT2012** (`1 − bmp_sed`),
  and the soluble N share also reduces lateral nitrate.

**Ps_bmp** (`scripts/make-pessel-variants.py`) is Ps_coast with three practices. Its oil palm
has riparian filter strips with SWAT's documented defaults: a field 40 times the strip (30–60
being most common), half the runoff into the strip's most concentrated tenth (White and Arnold
2009 found that a tenth of a strip got half of a field's runoff), none of it channelized. Its
row crops have the editor's `grwway_low` waterway (n 0.05, 0.5 m deep, 2 m wide, 1 km long, 1 %
slope), and its generic farmland the editor's `bmpusr1` (20 % of each). Every table matches
SWAT+; the basin's sediment yield falls from 0.52 to 0.18 t/ha a year, its organic N from 69 to
39 kg/ha.

## Sources

- Arnold, J. G., et al. (2012). *SWAT input/output file documentation, version 2012*, chapter
  33 (management: filter strips, grassed waterways, user-defined BMPs). Texas A&M.
  https://swat.tamu.edu/media/69398/ch33_input_ops.pdf
- White, M. J., & Arnold, J. G. (2009). Development of a simplistic vegetative filter strip
  model for sediment and nutrient retention at the field scale. *Hydrological Processes*, 23,
  1602–1616. https://doi.org/10.1002/hyp.7291
