//! Many runs of one project (M2 in `docs/roadmap.md`): the runs a calibration makes, each with its
//! own parameter set ([`swat_model::cal::Calibration`]), side by side on the computer's cores,
//! each keeping in memory only the series a calibration compares with observations.
//!
//! A calibration's runs don't depend on each other, so running one per core speeds the whole
//! calibration up by about the number of cores, whatever share of a run is parallel inside it.
//! Each run is the run [`crate::run::run_with_calibration`] makes on its own: the results don't
//! depend on how many run at once or in which order.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use swat_model::cal::Calibration;
use swat_model::{ObjectKind, Project};

use crate::hru::HruDay;
use crate::run::{Day, HruWeather, NoOutput, OutputSink, RunOptions, RunReport, Simulation};

/// What a calibration run keeps: for each printed day, the outflow of chosen channels and,
/// optionally, their depth, and the basin's precipitation and evapotranspiration (the HRUs'
/// area-weighted mean). Everything else the run computes is dropped as it goes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series {
    /// The printed days, in order.
    pub days: Vec<Day>,
    /// The chosen channels (1-based numbers in `channel_sd`), in the order asked.
    pub channels: Vec<usize>,
    /// Per channel, the day's mean outflow (m3/s; `channel_sd_day`'s `flo_out`).
    pub flo: Vec<Vec<f32>>,
    /// Per channel, the depth of that outflow (m): SWAT+'s rating curve (`channel_hyd_day`'s
    /// `dep_out`) and Manning's formula on the channel as a trapezoid (`dep_mann`). Empty unless
    /// asked for.
    pub dep_out: Vec<Vec<f32>>,
    pub dep_mann: Vec<Vec<f32>>,
    /// The basin's day: precipitation and evapotranspiration (mm), HRUs weighted by area.
    pub precip: Vec<f32>,
    pub et: Vec<f32>,
}

/// An [`OutputSink`] that fills a [`Series`].
#[derive(Clone, Debug)]
pub struct SeriesSink {
    pub series: Series,
    depth: bool,
    /// Each HRU's share of the HRUs' area.
    weight: Vec<f64>,
    precip: f64,
    et: f64,
    /// Position of each channel in `series.channels` (by 1-based number).
    slot: Vec<Option<usize>>,
}

impl SeriesSink {
    /// A sink for `channels` (1-based numbers in `channel_sd`), with their depths when `depth`.
    pub fn new(project: &Project, channels: &[usize], depth: bool) -> SeriesSink {
        let areas: Vec<f64> = project.objects_of(ObjectKind::Hru).map(|o| f64::from(o.area_ha)).collect();
        let total: f64 = areas.iter().sum();
        let weight = areas.iter().map(|a| if total > 0.0 { a / total } else { 0.0 }).collect();
        let n = channels.iter().copied().max().unwrap_or(0);
        let mut slot = vec![None; n + 1];
        for (k, &c) in channels.iter().enumerate() {
            if let Some(s) = slot.get_mut(c) {
                s.get_or_insert(k);
            }
        }
        let empty = || vec![Vec::new(); channels.len()];
        SeriesSink {
            series: Series {
                channels: channels.to_vec(),
                flo: empty(),
                dep_out: if depth { empty() } else { Vec::new() },
                dep_mann: if depth { empty() } else { Vec::new() },
                ..Series::default()
            },
            depth,
            weight,
            precip: 0.0,
            et: 0.0,
            slot,
        }
    }

    fn close_day(&mut self) {
        if self.series.days.len() > self.series.precip.len() {
            self.series.precip.push(self.precip as f32);
            self.series.et.push(self.et as f32);
            self.precip = 0.0;
            self.et = 0.0;
        }
    }

    /// The series, once the run has ended.
    pub fn finish(mut self) -> Series {
        self.close_day();
        self.series
    }
}

impl OutputSink for SeriesSink {
    fn hru_day(&mut self, day: Day, hru: usize, _gis_id: i32, _name: &str, w: &HruWeather, d: Option<&HruDay>) {
        if hru == 1 {
            self.close_day();
            self.series.days.push(day);
        }
        let wt = self.weight.get(hru.wrapping_sub(1)).copied().unwrap_or(0.0);
        match d {
            Some(d) => {
                self.precip += wt * f64::from(d.wb.precip);
                self.et += wt * f64::from(d.wb.et);
            }
            None => self.precip += wt * f64::from(w.precip),
        }
    }

    fn cha_day(
        &mut self,
        _day: Day,
        ich: usize,
        _gis_id: i32,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        hout: &crate::hyd::Hyd,
        _wtemp: f32,
    ) {
        if let Some(Some(k)) = self.slot.get(ich) {
            self.series.flo[*k].push(hout.flo);
        }
    }

    fn cha_hyd(&mut self, _day: Day, ich: usize, _gis_id: i32, _name: &str, h: &crate::channel::ChHyd) {
        if let Some(Some(k)) = self.slot.get(ich) {
            self.series.dep_out[*k].push(h.out.dep);
            self.series.dep_mann[*k].push(h.manning_out.0);
        }
    }

    fn wants_cha_hyd(&self) -> bool {
        self.depth
    }
}

/// One run of a batch: its series and what the run did, or why it failed.
pub type BatchResult = Result<(Series, RunReport), String>;

/// Runs `project` once per calibration in `cals`, `threads` runs at a time (0: as many as the
/// computer has), each keeping the [`Series`] of `channels`. The results come back in the order of
/// `cals`. `progress` is called after each run with the number done.
///
/// Each run is single-threaded inside (`opts.threads` is set to 1): the cores are shared out
/// between runs, which is where a calibration gains the most.
pub fn run_batch(
    project: &Project,
    opts: RunOptions,
    cals: &[Calibration],
    channels: &[usize],
    depth: bool,
    threads: usize,
    progress: &(dyn Fn(usize) + Sync),
) -> Vec<BatchResult> {
    let workers = match threads {
        0 => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        n => n,
    }
    .min(cals.len().max(1));
    let opts = RunOptions { threads: 1, ..opts };
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<BatchResult>>> = Mutex::new(vec![None; cals.len()]);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(cal) = cals.get(i) else { break };
                    let r = run_one(project, opts, cal, channels, depth);
                    if let Ok(mut all) = results.lock() {
                        all[i] = Some(r);
                    }
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });
    results.into_inner().unwrap_or_default().into_iter().map(|r| r.unwrap_or_else(|| Err("the run did not finish".into()))).collect()
}

/// [`run_batch`] from one shared spin-up (`docs/spinup.md`): the project with its own calibration
/// `base` is run once through the last day of `year` (in practice the warm-up years, which print
/// nothing), and each run then starts from a copy of that state with its own changes `sets`
/// applied ([`Simulation::calibrate`]), after `base`'s. A run's spin-up then had the project's
/// parameters, not its own: whether that matters is what `docs/spinup.md` measures.
#[allow(clippy::too_many_arguments)]
pub fn run_batch_shared(
    project: &Project,
    opts: RunOptions,
    base: &Calibration,
    sets: &[Vec<swat_model::cal::CalUpdate>],
    year: i32,
    channels: &[usize],
    depth: bool,
    threads: usize,
    progress: &(dyn Fn(usize) + Sync),
) -> Result<Vec<BatchResult>, String> {
    let opts = RunOptions { threads: 1, ..opts };
    let mut spun = Simulation::with_calibration(project, opts, base)?;
    let last = if swat_model::time::is_leap(year) { 366 } else { 365 };
    if !spun.run_until(&mut NoOutput, year, last)? {
        return Err(format!("the shared spin-up: day {last} of {year} is not in the simulated period"));
    }
    let workers = match threads {
        0 => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        n => n,
    }
    .min(sets.len().max(1));
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<BatchResult>>> = Mutex::new(vec![None; sets.len()]);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(set) = sets.get(i) else { break };
                    let r = (|| {
                        let mut sim = spun.checkpoint();
                        sim.calibrate(&Calibration { parms: base.parms.clone(), updates: set.clone() })?;
                        let mut sink = SeriesSink::new(project, channels, depth);
                        while sim.step(&mut sink)? {}
                        let report = sim.finish(&mut sink);
                        Ok((sink.finish(), report))
                    })();
                    if let Ok(mut all) = results.lock() {
                        all[i] = Some(r);
                    }
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });
    Ok(results.into_inner().unwrap_or_default().into_iter().map(|r| r.unwrap_or_else(|| Err("the run did not finish".into()))).collect())
}

/// One run of a batch.
pub fn run_one(project: &Project, opts: RunOptions, cal: &Calibration, channels: &[usize], depth: bool) -> BatchResult {
    let mut sink = SeriesSink::new(project, channels, depth);
    let mut sim = Simulation::with_calibration(project, opts, cal)?;
    while sim.step(&mut sink)? {}
    let report = sim.finish(&mut sink);
    Ok((sink.finish(), report))
}
