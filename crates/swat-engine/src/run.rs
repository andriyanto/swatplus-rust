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
    /// Atmospheric deposition today (`nut_nrain`): nitrate in rain (mg/L) and dry (kg/ha a day),
    /// ammonium in rain and dry; none without `atmo.cli`.
    pub atmo: Option<[f32; 4]>,
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

    /// One SWAT-DEG channel's day (`channel_sd_day.txt`): its 1-based number, GIS id and name,
    /// water body, storage, inflow and outflow, and the day's water temperature.
    #[allow(clippy::too_many_arguments)]
    fn cha_day(
        &mut self,
        _day: Day,
        _ich: usize,
        _gis_id: i32,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        _hout: &crate::hyd::Hyd,
        _wtemp: f32,
    ) {
    }

    /// One aquifer's day (`aquifer_day.txt`): its 1-based number, GIS id, name and state.
    fn aqu_day(&mut self, _day: Day, _iaq: usize, _gis_id: i32, _name: &str, _d: &crate::route::AquDay) {}

    /// An HRU's month, year or average annual (`hru_*_mon`, `_yr`, `_aa`), on the period's last
    /// day; `nutrients` as for the daily tables.
    #[allow(clippy::too_many_arguments)]
    fn hru_period(&mut self, _p: Period, _day: Day, _hru: usize, _gis_id: i32, _name: &str, _rows: &HruRows, _nutrients: bool) {}

    /// A routing unit's period (`ru_mon`, …).
    fn ru_period(&mut self, _p: Period, _day: Day, _name: &str, _hd: &crate::hyd::Hyd) {}

    /// A SWAT-DEG channel's period (`channel_sd_mon`, …): storage and water temperature are the
    /// day's.
    #[allow(clippy::too_many_arguments)]
    fn cha_period(
        &mut self,
        _p: Period,
        _day: Day,
        _ich: usize,
        _gis_id: i32,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        _hout: &crate::hyd::Hyd,
        _wtemp: f32,
    ) {
    }

    /// An aquifer's period (`aquifer_mon`, …).
    fn aqu_period(&mut self, _p: Period, _day: Day, _iaq: usize, _gis_id: i32, _name: &str, _d: &crate::route::AquDay) {}

    /// The basin's or a landscape unit's day (`basin_wb_day`, `lsunit_wb_day`, …; `p` None) or
    /// period.
    #[allow(clippy::too_many_arguments)]
    fn region(&mut self, _r: Region, _p: Option<Period>, _day: Day, _unit: usize, _gis_id: i32, _name: &str, _rows: &HruRows) {}

    /// An HRU plant's average yield per harvest over the simulation (`crop_yld_aa.txt`: mass,
    /// carbon, N, P in kg/ha), on the last day.
    fn crop_yld_aa(&mut self, _day: Day, _hru: usize, _plant: &str, _y: [f32; 4]) {}

    /// The basin's crops (`basin_crop_yld_yr`, `_aa`): harvested area (ha), yield (t) and t/ha.
    #[allow(clippy::too_many_arguments)]
    fn basin_crop_yld(&mut self, _p: Period, _yrc: i32, _iplt: usize, _plant: &str, _area: f32, _yld: f32, _t_ha: f32) {}

    /// The basin's aquifers (`basin_aqu_*`), weighted by area.
    fn basin_aqu(&mut self, _p: Option<Period>, _day: Day, _name: &str, _d: &crate::route::AquDay) {}

    /// The basin's SWAT-DEG channels (`basin_sd_cha_*`), added.
    #[allow(clippy::too_many_arguments)]
    fn basin_cha(
        &mut self,
        _p: Option<Period>,
        _day: Day,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        _hout: &crate::hyd::Hyd,
    ) {
    }

    /// A reservoir's day (`p` None) or period (`reservoir_*`): its 1-based number, GIS id, name,
    /// water body, storage, inflow and outflow.
    #[allow(clippy::too_many_arguments)]
    fn res(
        &mut self,
        _p: Option<Period>,
        _day: Day,
        _j: usize,
        _gis_id: i32,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        _hout: &crate::hyd::Hyd,
    ) {
    }

    /// The basin's reservoirs (`basin_res_*`), added; `unit` is one past the last reservoir.
    #[allow(clippy::too_many_arguments)]
    fn basin_res(
        &mut self,
        _p: Option<Period>,
        _day: Day,
        _unit: usize,
        _name: &str,
        _w: &crate::channel::ChWat,
        _stor: &crate::hyd::Hyd,
        _hin: &crate::hyd::Hyd,
        _hout: &crate::hyd::Hyd,
    ) {
    }

    /// A recall object's day or period (`recall_*`): its name and load.
    fn recall(&mut self, _p: Option<Period>, _day: Day, _name: &str, _h: &crate::hyd::Hyd) {}

    /// The basin's recall objects (`basin_psc_*`), added.
    fn basin_psc(&mut self, _p: Option<Period>, _day: Day, _name: &str, _h: &crate::hyd::Hyd) {}
}

/// A region of the basin-level tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Basin,
    Lsu,
}

pub use crate::periods::{HruRows, Period};

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
    /// Why the channels weren't routed (they pass their inflow on), when the project has them.
    pub channel_skipped: Option<String>,
}

/// How to run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RunOptions {
    /// Correct the bugs found in SWAT+ (see `docs/swat-fixes.md`) instead of reproducing them.
    /// The results then differ from SWAT+'s; off, they match it.
    pub fixes: bool,
}

/// Runs the simulation over `time.sim`, handing each printed day to `out`.
pub fn run(project: &Project, out: &mut dyn OutputSink) -> Result<RunReport, String> {
    run_with(project, out, RunOptions::default())
}

/// [`run`] with options.
pub fn run_with(project: &Project, out: &mut dyn OutputSink, opts: RunOptions) -> Result<RunReport, String> {
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
            match crate::route::Network::new(project, &station_names, &tconc, &ovn_km, opts.fixes) {
                Ok(n) => n,
                Err(e) => {
                    report.routing_skipped = Some(e);
                    None
                }
            }
        }
        None => None,
    };
    if let Some(why) = network.as_ref().and_then(|n| n.channel_skipped.clone()) {
        report.channel_skipped = Some(why);
    }
    let mut hru_wb = vec![[0.0_f32; 4]; hrus.len()];
    let basin = HruBasin::new(project).with_fixes(opts.fixes);
    let mut balance_started = false;
    let mut saved = Saved { rndseed_cond: weather.rndseed_cond, ..Saved::default() };
    // cli_read_atmodep: the deposition station of each weather station, and the time step of
    // the first day (`atmodep_cont%ts`, `first`).
    let atmo = project.atmodep.as_ref();
    let atmo_names: Vec<String> = atmo.map(|a| a.stations.iter().map(|s| s.name.clone()).collect()).unwrap_or_default();
    let atmo_of: Vec<usize> =
        project.stations.iter().map(|s| s.atmo_dep.as_deref().and_then(|n| crate::fortran::search(&atmo_names, n)).unwrap_or(0)).collect();
    let (mut atmo_ts, mut atmo_first) = (0_i32, true);
    if let Some(a) = atmo {
        let (yrc0, mo0) = (project.time.yrc_start, clock.mo);
        if a.timestep == "yr" {
            if let Some(iyr) = (1..=a.num as i32).find(|&iyr| a.yr_init + iyr - 1 == yrc0) {
                atmo_ts = iyr;
                atmo_first = false;
            }
        }
        if a.timestep == "mo" {
            let (mut iyrc, mut imo) = (a.yr_init, a.mo_init);
            for _ in 0..a.num {
                if iyrc == yrc0 && imo == mo0 {
                    atmo_ts = 12 * (yrc0 - a.yr_init) + imo;
                    atmo_first = false;
                    break;
                }
                imo += 1;
                if imo > 12 {
                    imo = 1;
                    iyrc += 1;
                }
            }
        }
    }

    clock.before_loop();
    weather.precip_control(0, &clock)?;

    let mut gate = PrintGate::new(&project.print, &clock);
    let nyskip = project.print.nyskip;
    let nbyr = clock.nbyr;
    let mut prt = crate::periods::PrintYears::default();
    let mut hru_acc = vec![crate::periods::HruAcc::default(); hrus.len()];
    let mut ru_acc: Vec<crate::periods::RuAcc> = Vec::new();
    let mut cha_acc: Vec<crate::periods::ChaAcc> = Vec::new();
    let mut aqu_acc: Vec<crate::periods::AquAcc> = Vec::new();
    // basin_output needs ls_unit.ele, lsu_output ls_unit.def, basin_aquifer_output aqu_catunit.ele.
    let basin_on = project.lsu_elems.len() > 1;
    let mut basin_acc = crate::periods::RegionAcc::new(false);
    let mut lsu_acc: Vec<crate::periods::RegionAcc> = project.lsu_defs.iter().map(|_| crate::periods::RegionAcc::new(true)).collect();
    let mut day_rows: Vec<Option<HruRows>> = vec![None; hrus.len()];
    let mut basin_aqu_acc = crate::periods::AquAcc::default();
    let mut basin_cha_acc = crate::periods::BasinChaAcc::default();
    let mut res_acc: Vec<crate::periods::ResAcc> = Vec::new();
    let mut basin_res_acc = crate::periods::ResAcc::default();
    let mut rec_acc: Vec<crate::periods::RecAcc> = Vec::new();
    let mut basin_rec_acc = crate::periods::RecAcc::default();
    let aqu_area: Vec<f32> = project.objects_of(swat_model::ObjectKind::Aquifer).map(|o| o.area_ha).collect();
    let ru_gis: Vec<i32> = project.objects_of(swat_model::ObjectKind::RoutingUnit).map(|o| o.gis_id).collect();
    let bsn_name = project.basin_name.clone();
    // plant_all_init: the basin's plants, in the order they first appear in the HRUs'
    // communities, and each HRU plant's place among them.
    let mut plts_bsn: Vec<String> = Vec::new();
    let bsn_num: Vec<Vec<usize>> = land
        .as_ref()
        .map(|l| {
            l.iter()
                .map(|h| {
                    h.plants
                        .iter()
                        .map(|p| match plts_bsn.iter().position(|n| *n == p.cpnm) {
                            Some(k) => k,
                            None => {
                                plts_bsn.push(p.cpnm.clone());
                                plts_bsn.len() - 1
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    let mut bsn_yld = vec![(0.0_f32, 0.0_f32); plts_bsn.len()];
    let mut bsn_yld_aa = vec![(0.0_f32, 0.0_f32); plts_bsn.len()];
    for curyr in 1..=nbyr {
        clock.start_year(curyr);
        prt.start_year(clock.yrs, nyskip, nbyr, swat_model::time::is_leap(clock.yrc), clock.day_start, clock.day_end_yr);
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
            let end_sim = clock.end_yr && clock.yrs == nbyr;
            if end_sim {
                prt.end_sim(nbyr);
            }
            let mo = clock.mo.clamp(1, 12) as usize;
            let pday = crate::periods::PeriodDay {
                end_mo: clock.day == clock.ndays[mo],
                end_yr: clock.end_yr,
                end_sim,
                mon_days: (clock.ndays[mo] - clock.ndays[mo - 1]) as f32,
                day_end_yr: clock.day_end_yr as f32,
                yrs_prt: prt.yrs_prt,
                days_prt: prt.days_prt,
            };
            // cli_atmodep_time_control (the day's end-of-month and -year flags are already set,
            // so the last day of a month or year takes the next one's deposition).
            let end_mo_today = clock.day == clock.ndays[mo];
            if let Some(a) = atmo
                && !a.stations.is_empty()
            {
                if atmo_first {
                    if (a.timestep == "yr" && a.yr_init == clock.yrc)
                        || (a.timestep == "mo" && a.yr_init == clock.yrc && a.mo_init == clock.mo)
                    {
                        atmo_ts = 1;
                        atmo_first = false;
                    }
                } else if (a.timestep == "yr" && clock.end_yr) || (a.timestep == "mo" && end_mo_today) {
                    atmo_ts += 1;
                }
            }
            let atmo_day = |ist: usize| -> Option<[f32; 4]> {
                let a = atmo?;
                let st = a.stations.get(atmo_of.get(ist.wrapping_sub(1)).copied().unwrap_or(0).wrapping_sub(1));
                let v = |x: Option<&Vec<f32>>, k: usize| x.and_then(|x| x.get(k)).copied().unwrap_or(0.0);
                match a.timestep.as_str() {
                    "aa" => {
                        let s = st;
                        Some([
                            v(s.map(|s| &s.no3_rf), 0),
                            v(s.map(|s| &s.no3_dry), 0) / 365.0,
                            v(s.map(|s| &s.nh4_rf), 0),
                            v(s.map(|s| &s.nh4_dry), 0) / 365.0,
                        ])
                    }
                    "mo" | "yr" if atmo_ts > 0 && atmo_ts as usize <= a.num => {
                        let k = atmo_ts as usize - 1;
                        let c = if a.timestep == "mo" { (clock.ndays[mo] - clock.ndays[mo - 1]) as f32 } else { 365.0 };
                        Some([
                            v(st.map(|s| &s.no3_rf), k),
                            v(st.map(|s| &s.no3_dry), k) / c,
                            v(st.map(|s| &s.nh4_rf), k),
                            v(st.map(|s| &s.nh4_dry), k) / c,
                        ])
                    }
                    _ => None,
                }
            };
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
                    atmo: atmo_day(*ist),
                };
                if lapse {
                    w.precip = (w.precip + plaps).max(0.0);
                    w.tmax += tlaps;
                    w.tmin += tlaps;
                    w.tave += tlaps;
                }
                let mut hd = None;
                if let Some(h) = land.as_mut().and_then(|l| l.get_mut(j)) {
                    // et_pot (Penman-Monteith) raises the HRU's wind to 0.01 m/s, and the HRU
                    // prints it.
                    if basin.pet == 1 && w.windsp < 0.01 {
                        w.windsp = 0.01;
                    }
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
                        station_precip: s.precip,
                        day_end_yr: clock.day_end_yr,
                        end_yr: clock.day == clock.day_end_yr,
                    };
                    let mut d = h.day(&w, &ctx, &basin, &mut saved);
                    for &(ipl, y) in &d.harvests {
                        if let Some(&k) = bsn_num.get(j).and_then(|b| b.get(ipl)) {
                            bsn_yld[k].0 += h.area_ha;
                            bsn_yld[k].1 += y * h.area_ha / 1000.0;
                        }
                    }
                    if output {
                        h.close_day(&mut d.wb, printing);
                    }
                    hd = Some(d);
                }
                if let (Some(n), Some(d)) = (network.as_mut(), hd.as_ref()) {
                    n.set_hru(j, &d.hd);
                    hru_wb[j] = [d.wb.snomlt, d.wb.surq_gen, d.wb.latq, d.wb.wateryld];
                }
                if printing {
                    out.hru_day(day, j + 1, o.gis_id, &o.name, &w, hd.as_ref());
                }
                if output && let Some(d) = hd.as_ref() {
                    let weather = [w.tmax, w.tmin, w.tave, w.solrad, w.windsp, w.rhum];
                    hru_acc[j].day(d, weather, &pday, |p, rows| out.hru_period(p, day, j + 1, o.gis_id, &o.name, rows, d.nutrients));
                    day_rows[j] = Some(crate::periods::region_values(d, weather));
                }
                if end_sim
                    && output
                    && let Some(h) = land.as_ref().and_then(|l| l.get(j))
                {
                    for pl in &h.plants {
                        let mut y = pl.yield_tot;
                        if pl.harv_num > 0 {
                            let k = pl.harv_num as f32;
                            y = crate::hru::plant::OrgMass { m: y.m / k, c: y.c / k, n: y.n / k, p: y.p / k };
                        }
                        out.crop_yld_aa(day, j + 1, &pl.db.name, [y.m, y.c, y.n, y.p]);
                    }
                }
            }
            if output && land.is_some() {
                regions(project, &day_rows, basin_on, &mut basin_acc, &mut lsu_acc, &ru_gis, &bsn_name, &pday, printing, day, out);
            }
            if let Some(n) = network.as_mut() {
                let nd = crate::route::NetDay {
                    weat: weather.stations.iter().map(|s| &s.weat).collect(),
                    daylth: weather
                        .stations
                        .iter()
                        .map(|s| weather.wgn.get(s.wgn.wrapping_sub(1)).map(|g| g.daylth).unwrap_or(0.0))
                        .collect(),
                    tgage: weather
                        .stations
                        .iter()
                        .map(|s| match s.tmp {
                            Source::Gage(i) => weather.tmp.get(i),
                            Source::Sim => None,
                        })
                        .collect(),
                    hru_wb: &hru_wb,
                    yrs: clock.yrs,
                    jday: clock.day,
                    yrc: clock.yrc,
                    mo: clock.mo,
                    print: clock.yrs > nyskip,
                };
                n.day(&nd);
                if output {
                    let mut bw = crate::channel::ChWat::default();
                    let (mut bs, mut bi, mut bo) = (crate::hyd::Hyd::default(), crate::hyd::Hyd::default(), crate::hyd::Hyd::default());
                    let mut any_cha = false;
                    let mut ba = crate::route::AquDay::default();
                    let mut any_aqu = false;
                    for (_, _, _, w, st, i, o, _) in n.channels() {
                        any_cha = true;
                        bs = bs.plus(st);
                        bi = bi.plus(i);
                        bo = bo.plus(o);
                        crate::periods::wat_sum(&mut bw, &w);
                    }
                    if any_cha {
                        if printing {
                            out.basin_cha(None, day, &bsn_name, &bw, &bs, &bi, &bo);
                        }
                        basin_cha_acc.day(&bw, &bi, &bo, &pday, |p, w, i, o| out.basin_cha(Some(p), day, &bsn_name, w, &bs, i, o));
                    }
                    if project.aqu_elems {
                        for (k, (_, _, _, d)) in n.aquifers().enumerate() {
                            any_aqu = true;
                            let c = aqu_area.get(k).copied().unwrap_or(0.0) / project.area_tot_ha;
                            crate::periods::aqu_sum(&mut ba, d, c);
                        }
                    }
                    if any_aqu {
                        if printing {
                            out.basin_aqu(None, day, &bsn_name, &ba);
                        }
                        basin_aqu_acc.day(&ba, &pday, |p, d| out.basin_aqu(Some(p), day, &bsn_name, d));
                    }
                    for (k, (ich, gis_id, name, w, st, i, o, wtemp)) in n.channels().into_iter().enumerate() {
                        if printing {
                            out.cha_day(day, ich, gis_id, name, &w, &st, &i, &o, wtemp);
                        }
                        if cha_acc.len() <= k {
                            cha_acc.resize(k + 1, Default::default());
                        }
                        cha_acc[k].day(&w, &i, &o, &pday, |p, w, i, o| out.cha_period(p, day, ich, gis_id, name, w, &st, i, o, wtemp));
                    }
                    for (k, (iaq, gis_id, name, d)) in n.aquifers().enumerate() {
                        if printing {
                            out.aqu_day(day, iaq, gis_id, name, d);
                        }
                        if aqu_acc.len() <= k {
                            aqu_acc.resize(k + 1, Default::default());
                        }
                        aqu_acc[k].day(d, &pday, |p, d| out.aqu_period(p, day, iaq, gis_id, name, d));
                    }
                    for (k, (j, gis_id, name, r)) in n.reservoir_days().enumerate() {
                        if printing {
                            out.res(None, day, j, gis_id, name, &r.wat, &r.water, &r.in_d, &r.out_d);
                        }
                        if res_acc.len() <= k {
                            res_acc.resize(k + 1, Default::default());
                        }
                        res_acc[k]
                            .day(&r.wat, &r.in_d, &r.out_d, &pday, |p, w, i, o| out.res(Some(p), day, j, gis_id, name, w, &r.water, i, o));
                    }
                    if !n.reservoirs.is_empty() {
                        let mut bw = crate::channel::ChWat::default();
                        let (mut bs, mut bi, mut bo) = (crate::hyd::Hyd::default(), crate::hyd::Hyd::default(), crate::hyd::Hyd::default());
                        for r in &n.reservoirs {
                            bs = bs.plus(r.water);
                            bi = bi.plus(r.in_d);
                            bo = bo.plus(r.out_d);
                            crate::periods::wat_sum(&mut bw, &r.wat);
                        }
                        let unit = n.reservoirs.len() + 1;
                        if printing {
                            out.basin_res(None, day, unit, &bsn_name, &bw, &bs, &bi, &bo);
                        }
                        basin_res_acc.day(&bw, &bi, &bo, &pday, |p, w, i, o| out.basin_res(Some(p), day, unit, &bsn_name, w, &bs, i, o));
                        // basin_reservoir_output zeroes the day's inflow and outflow.
                        for r in n.reservoirs.iter_mut() {
                            r.in_d = crate::hyd::Hyd::default();
                            r.out_d = crate::hyd::Hyd::default();
                        }
                    }
                    for (k, (_, name, h)) in n.recall_days().enumerate() {
                        if printing {
                            out.recall(None, day, name, h);
                        }
                        if rec_acc.len() <= k {
                            rec_acc.resize(k + 1, Default::default());
                        }
                        rec_acc[k].day(h, &pday, |p, h| out.recall(Some(p), day, name, h));
                    }
                    if !n.recalls.is_empty() {
                        let mut b = crate::hyd::Hyd::default();
                        for r in &n.recalls {
                            b = b.plus(r.d);
                        }
                        if printing {
                            out.basin_psc(None, day, &bsn_name, &b);
                        }
                        basin_rec_acc.day(&b, &pday, |p, h| out.basin_psc(Some(p), day, &bsn_name, h));
                    }
                    for (k, (name, h)) in n.routing_units().enumerate() {
                        if printing {
                            out.ru_day(day, name, h);
                        }
                        if ru_acc.len() <= k {
                            ru_acc.resize(k + 1, Default::default());
                        }
                        ru_acc[k].day(h, &pday, |p, h| out.ru_period(p, day, name, h));
                    }
                }
            }
            if clock.day == 181 {
                for h in land.iter_mut().flatten() {
                    h.midyear();
                }
            }
        }
        // time_control at the year's end: the basin's crop yields.
        for (k, name) in plts_bsn.iter().enumerate() {
            if clock.yrs > nyskip {
                let (area, yld) = bsn_yld[k];
                out.basin_crop_yld(Period::Yr, clock.yrc, k + 1, name, area, yld, yld / (area + 1.0e-6));
                bsn_yld_aa[k].0 += area;
                bsn_yld_aa[k].1 += yld;
            }
            bsn_yld[k] = (0.0, 0.0);
            if clock.yrs == nbyr {
                let (area, yld) = bsn_yld_aa[k];
                let t_ha = yld / (area + 1.0e-6);
                out.basin_crop_yld(Period::Aa, clock.yrc, k + 1, name, area / prt.yrs_prt, yld / prt.yrs_prt, t_ha);
            }
        }
        for h in land.iter_mut().flatten() {
            h.end_year(basin.cswat, opts.fixes);
        }
        clock.yrc += 1;
    }
    Ok(report)
}

/// `basin_output` and `lsu_output` for a printed day: the HRUs' day weighted by their share of
/// the basin (`bsn_frac`) and of their landscape unit (`ru_frac`), in SWAT+'s order.
#[allow(clippy::too_many_arguments)]
fn regions(
    project: &Project,
    rows: &[Option<HruRows>],
    basin_on: bool,
    basin: &mut crate::periods::RegionAcc,
    lsus: &mut [crate::periods::RegionAcc],
    ru_gis: &[i32],
    bsn_name: &str,
    pday: &crate::periods::PeriodDay,
    printing: bool,
    day: Day,
    out: &mut dyn OutputSink,
) {
    use crate::periods::{region_add, region_start};
    let elems = &project.lsu_elems;
    let empty = HruRows { wb: [0.0; 32], nb: [0.0; 15], ls: [0.0; 13], pw: [0.0; 25] };
    if basin_on {
        let mut r = region_start(&empty);
        let mut init = [0.0_f32; 2];
        for ihru in 1..=rows.len() {
            // lsu_elem(ihru)%obtypno, then that element's share.
            let iihru = elems.get(ihru).map_or(0, |e| e.obtypno.max(0) as usize);
            let Some(e) = elems.get(iihru) else { continue };
            if e.bsn_frac > 1.0e-12
                && e.obtyp == "hru"
                && let Some(Some(h)) = rows.get(iihru.wrapping_sub(1))
            {
                region_add(&mut r, h, e.bsn_frac);
                init[0] += h.wb[13] * e.bsn_frac;
                init[1] += h.wb[17] * e.bsn_frac;
            }
        }
        basin.start(init);
        if printing {
            let d = basin.daily_row(&r);
            out.region(Region::Basin, None, day, 1, 1, bsn_name, &d);
        }
        basin.day(&r, pday, |p, rows| out.region(Region::Basin, Some(p), day, 1, 1, bsn_name, rows));
    }
    for (ilsu, (def, acc)) in project.lsu_defs.iter().zip(lsus.iter_mut()).enumerate() {
        let mut r = region_start(&empty);
        let mut init = [0.0_f32; 2];
        for &ihru in &def.elems {
            let Some(e) = elems.get(ihru) else { continue };
            if e.ru_frac > 1.0e-9
                && e.obtyp == "hru"
                && let Some(Some(h)) = rows.get(ihru.wrapping_sub(1))
            {
                region_add(&mut r, h, e.ru_frac);
                init[0] += h.wb[13] * e.ru_frac;
                init[1] += h.wb[17] * e.ru_frac;
            }
        }
        acc.start(init);
        let gis = ru_gis.get(ilsu).copied().unwrap_or(0);
        if printing {
            let d = acc.daily_row(&r);
            out.region(Region::Lsu, None, day, ilsu + 1, gis, &def.name, &d);
        }
        acc.day(&r, pday, |p, rows| out.region(Region::Lsu, Some(p), day, ilsu + 1, gis, &def.name, rows));
    }
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
