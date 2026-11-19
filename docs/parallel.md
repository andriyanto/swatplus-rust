# Parallel HRUs and routing (`--fix-swat --threads N`)

F1b in [`roadmap.md`](roadmap.md). SWAT+ runs its HRUs one after another, and some of what one
HRU computes is left in variables every HRU shares (module arrays and Fortran `save` variables:
`docs/swat-fixes.md`, the `Saved` row). The next HRU reads them, so an HRU's results depend on the
HRU before it. A faithful run keeps that, so its HRUs run in order on one thread.

With `--fix-swat` each HRU gets its own copy of that state, and its decision tables their own
random-number stream (the Park–Miller seed advanced a million draws per HRU, `aunif_skip`). The
HRUs then depend only on the HRUs that route water to them, and run on `--threads N` threads (by
default as many as the computer has; `--threads 1` runs them on the main thread).

## How

1. **Waves** (`Network::hru_waves`): an HRU goes in a later wave than every HRU it receives water
   from (HRU to HRU routing, `docs/routing.md`), so it reads their water of the same day, as in
   SWAT+'s command order. Without HRU-to-HRU routing all HRUs are one wave (Batanghari: 1,858 HRUs
   in one).
2. **A wave's HRUs** are split into as many contiguous parts as threads (`std::thread::scope`, no
   dependency), each HRU's day computed with its own state.
3. **What follows each HRU's day** (crop yields summed for the basin, its water handed to the
   routing) is done afterwards on the main thread, in the wave's order.
4. **The routing by sub-basin** (M5, below) follows on the same threads; the output stays on the
   main thread.

Nothing an HRU computes depends on which thread runs it or on what runs beside it, and the sums
are made in a fixed order, so **the results are the same for any number of threads**, bit for
bit: all 37 validation scenarios give the same 627 tables with 1 and with 3 threads (and
Batanghari, Bh_land, Ps_hourly, Ps_paddy, Ps_septic and Ames_dtbl with 2), and
`crates/swatplus-rust/tests/parallel.rs` checks it on Ames_sub1.

## Routing by sub-basin (M5)

After the HRUs, SWAT+ runs the other objects (routing units, aquifers, channels, reservoirs,
recall points, outlets) one after another in its command order. Two things tie a channel to the
one before it in that order: two `save` variables of the channel routines, `ch_rtmusk`'s `inflo`
and `ch_watqual4`'s `gra` (`docs/swat-fixes.md`). With `--fix-swat` each channel keeps its own,
so an object's day then depends only on the objects it takes water from. The network is cut once
per run into parts (`route::make_plan`):

1. **What must come first.** Each object needs the objects whose water of today it takes
   (earlier in the command order) to have run, and the objects whose water of *yesterday* it
   takes (later in the order) not to have run yet.
2. **Groups.** From the outlets up, the first object that has, with everything it needs,
   at most 1/(2 × threads) of the objects starts a group holding all of them; groups that share an
   object (an aquifer fed from two sub-basins, say) are joined. A group then needs nothing from
   outside it.
3. **Parts.** The groups go to the threads, largest first, each to the least loaded; each part
   keeps the command order. They are routed at once (`std::thread::scope`), then the objects in no
   group (the main channels near the outlets) on the main thread, in the command order.

| Project | objects past the HRUs | parts on 4 threads | on 12 threads | left for the main thread (12) |
|---|---:|---|---|---:|
| Batanghari | 612 | 149, 148, 148, 146 | 45–47 each | 55 (9 %) |
| Pessel | 3,162 | 773–776 each | 249–252 each | 156 (5 %) |

An object computes the same day whichever part it is in, and its inflows are summed in a fixed
order, so the tables stay the same for any number of threads. A faithful run keeps one part, in
SWAT+'s order, with the shared values as SWAT+ has them.

Checked: all 37 validation scenarios give the same 1,254 tables as before this change, faithful
and corrected (the corrected `inflo` and `gra` change none of them), with the default threads and
with `--threads 1`; Batanghari, Pessel and eight of their variants (channels, reservoirs, recall
points, hourly steps, unit hydrographs) give the same tables with 5 threads as with 1.
`route::tests` checks on a made-up network that every part has all it needs.

Measuring the speed-up needs a computer with many cores (the one used here has two, shared with
other work): `scripts/bench-threads.ps1` runs one project on 1, 2, … 12 threads a few times
each, writes the times to a CSV file, and checks that every thread count gives the same tables.

## What changes

- **A faithful run doesn't**: the 37 validation scenarios give the same tables as before, every
  byte, and `--threads` has no effect without `--fix-swat`.
- **A `--fix-swat` run changes where the shared state mattered**: in 4 of the 37 scenarios
  (Ames_cornf, Ps_land, Ps_paddy, Ps_septic), by up to about 1 % in the printed tables (the
  largest: a wetland's overflow in Ps_paddy). In the other 33 no table changes.

## Speed

On the 2-core machine used here, Bh_land (2015–2019, 1,858 HRUs) runs in 12.9 s on one thread
and 11.9 s on two; a faithful run takes 12.2 s. The gain is small there because the HRUs are
about a third of that run's work, and the rest (routing, channels, writing the tables) is still
on one thread. `docs/performance.md` estimated the bound at about 2.4 times on 12 cores for
Pessel, whose HRUs are about two thirds of the work; a run that prints little (as in
calibration) gains the most. Parallel routing by sub-basin is the next step.

In the browser (WebAssembly) the HRUs run on one thread.
