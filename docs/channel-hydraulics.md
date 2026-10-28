# Channel hydraulics (`channel_hyd_day.txt`)

H1 in [`roadmap.md`](roadmap.md). SWAT-DEG already computes a rating curve for every channel
(`sd_rating_curve`): depth, area, wetted perimeter, top width, velocity, volume and travel time at
four depths. SWAT+ uses it inside the routing but prints none of it. `swatplus-rust run
--channel-hyd` writes it out as one more table, `channel_hyd_day.txt`, for each SWAT-DEG channel on
every day of `print.prt`'s print window, whether or not `channel_sd` prints daily.

It is not a SWAT+ table. It is only recorded: the routing doesn't read it, and every SWAT+ table is
byte-identical with and without the option (`crates/swatplus-rust/tests/channel_hyd.rs`; all 37
validation scenarios, faithful and `--fix-swat`). With `--format parquet` it is written as Parquet
too.

## Columns

| Column | Unit | What |
| --- | --- | --- |
| `flo_out` | m³/s | The day's mean outflow rate (`channel_sd`'s `flo_out`) |
| `dep_out`, `wid_out`, `area_out`, `perim_out`, `vel_out`, `ttime_out` | m, m, m², m, m/s, hr | SWAT+'s rating curve at that rate (`rcurv_interp_flo`) |
| `flo_in`, `dep_in`, `vel_in` | m³/s, m, m/s | The same at the day's mean inflow rate |
| `flo_peak`, `dep_peak`, `vel_peak` | m³/s, m, m/s | The peak inflow SWAT-DEG estimates for its sediment (`pk_rto`, from the drainage area) and the curve there |
| `vol_ch`, `vol_fp` | m³ | Storage at the day's end in the channel and over the flood plain |
| `dep_bf`, `wid_bf` | m | Bankfull depth and top width (`hyd-sed-lte.cha`) |
| `overbank` | 0/1 | 1 when `dep_out` is above bankfull |
| `dep_mann`, `vel_mann`, `wid_mann` | m, m/s, m | Depth, velocity and top width of the mean outflow by Manning's formula on the channel's own geometry (below) |

## SWAT+'s curve and Manning's depth

SWAT+'s curve is four points (0.1 and 1 times bankfull depth in the channel, 1.2 and 2 times over
the flood plain) with straight lines between them. Three things follow, and the `*_out` columns
keep them, since they are what SWAT+ routes with:

- **Below the first point every quantity scales with the flow**, the width and the depth too: at
  a hundredth of the first point's flow the channel is a hundredth as wide. Low flows, most days
  in most channels, read too shallow and too narrow.
- **Above twice bankfull the curve goes on in proportion** to its last point.
- **Above bankfull the whole section takes the flood plain's n.** A channel whose flood plain is
  much rougher can carry less at 1.2 times bankfull depth than at bankfull, and the curve is then
  not one-to-one. None of the channels of Batanghari (284) or Pessel (1,454) is like that, since
  their flood plain slope `fps` is 0.00001 and the flood plain very wide, but a project can be.

Without `--fix-swat` the points are on SWAT+'s geometry, whose area is `b d + d / z` and whose
wetted perimeter takes the side slope the other way up (`docs/swat-fixes.md`); with it, on a true
trapezoid.

`dep_mann`, `vel_mann` and `wid_mann` solve Manning's formula (as SWAT+ writes it: `R^0.6666` and
`n + 0.001`) for the day's mean outflow on the channel as a true trapezoid, whatever the mode,
with the bottom width and side slope `sd_rating_curve` used. Above bankfull the section is divided
(Chow, 1959): the main channel with its own n, and the two flood plain wedges with theirs, their
flows added, so the depth always rises with the flow. Up to bankfull these meet the corrected curve
at its points (unit tests in `crates/swat-engine/src/channel.rs`). They are this port's, not
SWAT+'s, and they are the better estimate of a water level to set beside a gauge (AWLR) reading.

## Limits

- **A daily mean**, not the day's peak stage. A sub-daily run (`time.sim` step above 1) still
  prints one row a day, at the day's mean flow.
- **The geometry is SWAT-DEG's**: a trapezoid from the bankfull width, depth and side slope in
  `hyd-sed-lte.cha`, and a flood plain of one slope. A measured cross-section gives a different
  depth for the same flow; that is H1's next step (a rating curve from surveyed sections).
- **The depth is above the channel bed**, not a gauge reading: comparing it with a staff gauge
  needs the gauge's zero relative to the bed.
