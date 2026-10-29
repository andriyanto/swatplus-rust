# Parallel HRUs (`--fix-swat --threads N`)

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
   routing) is done afterwards on the main thread, in the wave's order. The routing, the
   channels and the output stay on the main thread.

Nothing an HRU computes depends on which thread runs it or on what runs beside it, and the sums
are made in a fixed order, so **the results are the same for any number of threads**, bit for
bit: all 37 validation scenarios give the same 627 tables with 1 and with 3 threads (and
Batanghari, Bh_land, Ps_hourly, Ps_paddy, Ps_septic and Ames_dtbl with 2), and
`crates/swatplus-rust/tests/parallel.rs` checks it on Ames_sub1.

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
