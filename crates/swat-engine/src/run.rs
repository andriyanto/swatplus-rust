//! The simulation loop (`time_control`) and what it hands to output.

use swat_model::{ObjectKind, Project};

use crate::climate::{Source, Weather};
use crate::clock::Clock;
use crate::hru::plant::Saved;
use crate::hru::{DayCtx, Hru, HruBasin, HruDay, init_hrus};

/// The weather an HRU sees on a day (`w` in `hru_control`): its station's, with the lapse-rate
/// corrections for elevation when `codes.bsn` turns them on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruWeather {
    pub precip: f32,
    pub tmax: f32,
    pub tmin: f32,
    pub tave: f32,
    pub solrad: f32,
    pub solradmx: f32,
    pub windsp: f32,
    pub rhum: f32,
    pub phubase0: f32,
}

/// The date and position of a printed day.
#[derive(Clone, Copy, Debug)]
pub struct Day {
    pub jday: i32,
    pub mon: i32,
    pub day: i32,
    pub yr: i32,
}

/// Receives daily output, for the days SWAT+ prints.
pub trait OutputSink {
    /// One HRU's day: its 1-based number, the connection-file object, its weather and, when the
    /// project's HRUs are within what the port simulates, its water balance.
    fn hru_day(&mut self, day: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather, d: Option<&HruDay>);

    /// One routing unit's day (`ru_day.txt`): its name and what it passed on (`ru_d`).
    fn ru_day(&mut self, _day: Day, _name: &str, _hd: &crate::hyd::Hyd) {}

    /// One aquifer's day (`aquifer_day.txt`): its 1-based number, GIS id, name and state.
    fn aqu_day(&mut self, _day: Day, _iaq: usize, _gis_id: i32, _name: &str, _d: &crate::route::AquDay) {}
}

/// SWAT+'s daily print switches (`pco%day_print`, `day_print_over`, `int_day_cur`) and the print
/// window as `basin_print_codes_read` completes it: a first day of 0 is 1, a last day of 0 is 366,
/// a first year of 0 is the simulation's first, a last year of 0 is one past its last
/// (`yrc + nbyr`), an interval of 0 or less is 1, and the interval counter starts at the interval.
struct PrintGate {
    day_print: bool,
    over: bool,
    int_day_cur: i32,
    p: swat_model::PrintSettings,
}

impl PrintGate {
    fn new(print: &swat_model::PrintSettings, clock: &Clock) -> Self {
        let mut p = print.clone();
        if p.day_start == 0 {
            p.day_start = 1;
        }
        if p.day_end == 0 {
            p.day_end = 366;
        }
        if p.yrc_start == 0 {
            p.yrc_start = clock.yrc;
        }
        if p.yrc_end == 0 {
            p.yrc_end = clock.yrc + clock.nbyr;
        }
        if p.interval <= 0 {
            p.interval = 1;
        }
        Self { day_print: false, over: false, int_day_cur: p.interval, p }
    }

    fn update(&mut self, clock: &Clock) {
        let p = &self.p;
        if self.over {
            return;
        }
        if !self.day_print {
            if clock.day >= p.day_start && clock.yrc >= p.yrc_start {
                self.day_print = true;
            }
        } else if clock.day > p.day_end && clock.yrc == p.yrc_end {
            self.day_print = false;
            self.over = true;
        } else {
            self.int_day_cur += 1;
            if self.int_day_cur > p.interval {
                self.int_day_cur = 1;
            }
        }
    }

    fn prints(&self) -> bool {
        self.day_print && self.int_day_cur == self.p.interval
    }
}

/// What a run did besides its output.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunReport {
    /// Why the HRU water balance wasn't simulated, when it wasn't (weather only).
    pub hru_skipped: Option<String>,
    /// Why routing units and aquifers weren't simulated, when the project has them.
    pub routing_skipped: Option<String>,
}

/// Runs the simulation over `time.sim`, handing each printed day to `out`.
pub fn run(project: &Project, out: &mut dyn OutputSink) -> Result<RunReport, String> {
    let mut clock = Clock::new(&project.time);
    let mut weather = Weather::new(project, &mut clock);
    let mut report = RunReport::default();

    // HRUs and their stations (`ob%wst`: 1 when the name isn't found), with lapse corrections
    // (`cli_lapse`).
    let station_names: Vec<String> = weather.stations.iter().map(|s| s.name.clone()).collect();
    let lapse = project.codes.lapse == 1;
    let hrus: Vec<(usize, &swat_model::SpatialObject, f32, f32)> = project
        .objects_of(ObjectKind::Hru)
        .map(|o| {
            let ist = crate::fortran::search(&station_names, &o.weather_station).unwrap_or(1);
            let (plaps, tlaps) = lapse_rates(project, &weather, ist, o.elev);
            (ist, o, plaps, tlaps)
        })
        .collect();

    // The HRUs' land phase, when everything they need is ported.
    let wgn_of: Vec<usize> =
        hrus.iter().map(|(ist, ..)| weather.stations.get(ist - 1).map(|s| s.wgn.saturating_sub(1)).unwrap_or(0)).collect();
    let mut land: Option<Vec<Hru>> = match init_hrus(project, &wgn_of, &weather.wgn, clock.step, &clock.ndays) {
        Ok(h) if h.len() == hrus.len() => Some(h),
        Ok(h) => {
            report.hru_skipped = Some(format!("{} HRUs in hru-data.hru, {} in hru.con", h.len(), hrus.len()));
            None
        }
        Err(e) => {
            report.hru_skipped = Some(e);
            None
        }
    };
    // The objects past the HRUs (routing units, aquifers), when the HRUs are simulated.
    let mut network = match land.as_ref() {
        Some(l) => {
            let tconc: Vec<f32> = l.iter().map(|h| h.tconc).collect();
            let ovn_km: Vec<(f32, f32)> = l.iter().map(|h| (h.ovn, h.km)).collect();
            match crate::route::Network::new(project, &station_names, &tconc, &ovn_km) {
                Ok(n) => n,
                Err(e) => {
                    report.routing_skipped = Some(e);
                    None
                }
            }
        }
        None => None,
    };
    let basin = HruBasin::new(project);
    let mut balance_started = false;
    let mut saved = Saved::default();

    clock.before_loop();
    weather.precip_control(0, &clock)?;

    let mut gate = PrintGate::new(&project.print, &clock);
    let nyskip = project.print.nyskip;
    let nbyr = clock.nbyr;
    for curyr in 1..=nbyr {
        clock.start_year(curyr);
        if !balance_started && clock.yrs > nyskip {
            for h in land.iter_mut().flatten() {
                h.start_balance();
            }
            balance_started = true;
        }
        for jday in clock.day_start..=clock.day_end_yr {
            clock.set_day(jday);
            gate.update(&clock);
            weather.climate_control(&clock)?;

            let output = clock.yrs > nyskip;
            let printing = output && gate.prints();
            let day = Day { jday: clock.day, mon: clock.mo, day: clock.day_mo, yr: clock.yrc };
            for (j, (ist, o, plaps, tlaps)) in hrus.iter().enumerate() {
                let Some(st) = weather.stations.get(ist - 1) else { continue };
                let s = &st.weat;
                let mut w = HruWeather {
                    precip: s.precip,
                    tmax: s.tmax,
                    tmin: s.tmin,
                    tave: s.tave,
                    solrad: s.solrad,
                    solradmx: s.solradmx,
                    windsp: s.windsp,
                    rhum: s.rhum,
                    phubase0: s.phubase0,
                };
                if lapse {
                    w.precip = (w.precip + plaps).max(0.0);
                    w.tmax += tlaps;
                    w.tmin += tlaps;
                    w.tave += tlaps;
                }
                let mut hd = None;
                if let Some(h) = land.as_mut().and_then(|l| l.get_mut(j)) {
                    let g = weather.wgn.get(h.wgn);
                    let ctx = DayCtx {
                        day: clock.day,
                        mo: clock.mo,
                        day_mo: clock.day_mo,
                        tmp_an: g.map(|g| g.tmp_an).unwrap_or(0.0),
                        phutot: g.map(|g| g.phutot).unwrap_or(0.0),
                        daylmn: g.map(|g| g.daylmn).unwrap_or(0.0),
                        dormhr: g.map(|g| g.daylth).unwrap_or(0.0),
                        station_tave: s.tave,
                        daylength: s.daylength,
                        precip_half_hr: s.precip_half_hr,
                        end_yr: clock.day == clock.day_end_yr,
                    };
                    let mut d = h.day(&w, &ctx, &basin, &mut saved);
                    if output {
                        h.close_day(&mut d.wb, printing);
                    }
                    hd = Some(d);
                }
                if let (Some(n), Some(d)) = (network.as_mut(), hd.as_ref()) {
                    n.set_hru(j, &d.hd);
                }
                if printing {
                    out.hru_day(day, j + 1, o.gis_id, &o.name, &w, hd.as_ref());
                }
            }
            if let Some(n) = network.as_mut() {
                let pet: Vec<f32> = weather.stations.iter().map(|s| s.weat.pet).collect();
                n.day(&pet);
                if printing {
                    for (iaq, gis_id, name, d) in n.aquifers() {
                        out.aqu_day(day, iaq, gis_id, name, d);
                    }
                    for (name, h) in n.routing_units() {
                        out.ru_day(day, name, h);
                    }
                }
            }
            if clock.day == 181 {
                for h in land.iter_mut().flatten() {
                    h.midyear();
                }
            }
        }
        for h in land.iter_mut().flatten() {
            h.end_year(basin.cswat);
        }
        clock.yrc += 1;
    }
    Ok(report)
}

/// SWAT+ `cli_lapse` for one object at elevation `elev` using station `ist`.
fn lapse_rates(project: &Project, weather: &Weather, ist: usize, elev: f32) -> (f32, f32) {
    let Some(st) = weather.stations.get(ist.wrapping_sub(1)) else { return (0.0, 0.0) };
    let wgn_elev = weather.wgn.get(st.wgn.wrapping_sub(1)).map(|g| g.elev).unwrap_or(0.0);
    let gage_elev = |list: &[crate::climate::Gage], i: usize| list.get(i).map(|g| g.elev).unwrap_or(0.0);
    let plaps = project.params.plaps();
    let tlaps = project.params.tlaps();
    let p = match st.pcp {
        Source::Sim => plaps * (elev - wgn_elev) / 1000.0,
        Source::Gage(i) => plaps * (elev - gage_elev(&weather.pcp, i)) / 1000.0,
    };
    let t = match st.tmp {
        Source::Sim => tlaps * (wgn_elev - elev) / 1000.0,
        Source::Gage(i) => tlaps * (gage_elev(&weather.tmp, i) - elev) / 1000.0,
    };
    (p, t)
}
