//! The simulation loop (`time_control`) and what it hands to output.

use swat_model::{ObjectKind, Project};

use crate::climate::{Source, Weather};
use crate::clock::Clock;

/// The weather an HRU sees on a day (`w` in `hru_control`): its station's, with the lapse-rate
/// corrections for elevation when `codes.bsn` turns them on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruWeather {
    pub precip: f32,
    pub tmax: f32,
    pub tmin: f32,
    pub tave: f32,
    pub solrad: f32,
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
    /// One HRU's day: its 1-based number, the connection-file object and its weather.
    fn hru_day(&mut self, day: Day, hru: usize, gis_id: i32, name: &str, w: &HruWeather);
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

/// Runs the simulation over `time.sim`, handing each printed day to `out`.
pub fn run(project: &Project, out: &mut dyn OutputSink) -> Result<(), String> {
    let mut clock = Clock::new(&project.time);
    let mut weather = Weather::new(project, &mut clock);

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

    clock.before_loop();
    weather.precip_control(0, &clock)?;

    let mut gate = PrintGate::new(&project.print, &clock);
    let nbyr = clock.nbyr;
    for curyr in 1..=nbyr {
        clock.start_year(curyr);
        for jday in clock.day_start..=clock.day_end_yr {
            clock.set_day(jday);
            gate.update(&clock);
            weather.climate_control(&clock)?;

            let printing = clock.yrs > project.print.nyskip && gate.prints();
            if printing {
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
                    out.hru_day(day, j + 1, o.gis_id, &o.name, &w);
                }
            }
        }
        clock.yrc += 1;
    }
    Ok(())
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
