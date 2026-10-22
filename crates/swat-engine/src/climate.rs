//! Daily weather: measured station data placed into SWAT+'s daily arrays (`cli_pmeas`,
//! `cli_tmeas`, `cli_smeas`, `cli_hmeas`, `cli_wmeas`), the weather generator (`cli_wgnread`,
//! `gcycl`, `cli_initwgn`, `cli_pgen`, `cli_weatgn`, `cli_tgen`, `cli_slrgen`, `cli_rhgen`,
//! `cli_wndgen`, `cli_clgen`), and the daily sequence that combines them (`cli_precip_control`,
//! `climate_control`).
//!
//! The order of every random draw matches SWAT+, because generated values depend on it.

use swat_model::{ClimateVar, Project, StationSeries, Wgn, WgnMonth};

use crate::clock::Clock;
use crate::fortran::{atri, aunif, dstn1, search};

/// A measured station file placed into daily arrays, as the `cli_*meas` readers leave it.
#[derive(Clone, Debug, Default)]
pub struct Gage {
    pub start_day: i32,
    pub start_yr: i32,
    pub end_day: i32,
    pub end_yr: i32,
    /// Years between the simulation's first year and the file's first, when the file starts later.
    pub yrs_start: i32,
    pub tstep: i32,
    pub elev: f32,
    /// `ts(day, year)`: index `[year - 1][day - 1]`, year counted from the first placed record.
    ts: Vec<[f32; 366]>,
    /// Minimum temperature (`ts2`).
    ts2: Vec<[f32; 366]>,
}

impl Gage {
    /// Places `series` the way `cli_pmeas` and its siblings do: skip to the first record of the
    /// simulation's first year at or after its first day, then fill one column per year, moving to
    /// the next column when a day 365 or 366 is followed by a different year. `ndays` is the global
    /// month table the temperature reader changes (`Some` only for temperature). `None` when the
    /// file has no data records: the reader stops the variable there.
    pub fn place(series: &StationSeries, yrc: i32, day_start: i32, mut ndays: Option<&mut Clock>) -> Option<Gage> {
        let first = series.records.first()?;
        let last = series.records.last()?;
        let mut g = Gage {
            start_day: first.jday,
            start_yr: first.year,
            end_day: last.jday,
            end_yr: last.year,
            yrs_start: if first.year > yrc { first.year - yrc } else { 0 },
            tstep: series.tstep,
            elev: series.elev,
            ts: Vec::new(),
            ts2: Vec::new(),
        };
        let records = &series.records;
        let start = records.iter().position(|r| r.year >= yrc && r.jday >= day_start);
        // The skip loop leaves `iyr` at the record it stopped on (the last, if none qualified).
        let iyr_found = start.and_then(|k| records.get(k)).unwrap_or(last).year;
        if let Some(clock) = ndays.as_deref_mut() {
            clock.set_ndays_mod4(iyr_found);
        }
        let Some(start) = start else {
            return Some(g);
        };
        let mut iyr_prev = iyr_found;
        let mut iyrs: usize = 1;
        let mut k = start;
        while let Some(r) = records.get(k) {
            if (1..=366).contains(&r.jday) {
                while g.ts.len() < iyrs {
                    g.ts.push([0.0; 366]);
                    g.ts2.push([0.0; 366]);
                }
                let d = (r.jday - 1) as usize;
                if let (Some(col), Some(col2)) = (g.ts.get_mut(iyrs - 1), g.ts2.get_mut(iyrs - 1)) {
                    col[d] = r.values[0];
                    col2[d] = r.values[1];
                }
            }
            if r.jday == 365 || r.jday == 366 {
                let Some(next) = records.get(k + 1) else { break };
                if next.year != iyr_prev {
                    iyr_prev = next.year;
                    iyrs += 1;
                    if let Some(clock) = ndays.as_deref_mut() {
                        clock.set_ndays_mod4(next.year);
                    }
                }
            }
            k += 1;
        }
        Some(g)
    }

    /// `ts(day, year)`; 0 outside what was placed (SWAT+'s arrays start zeroed).
    pub fn value(&self, day: i32, year: i32) -> f32 {
        Self::at(&self.ts, day, year)
    }

    /// `ts2(day, year)`: minimum temperature.
    pub fn value2(&self, day: i32, year: i32) -> f32 {
        Self::at(&self.ts2, day, year)
    }

    fn at(ts: &[[f32; 366]], day: i32, year: i32) -> f32 {
        if day < 1 || year < 1 {
            return 0.0;
        }
        ts.get((year - 1) as usize).and_then(|c| c.get((day - 1) as usize)).copied().unwrap_or(0.0)
    }

    /// SWAT+ `cli_bounds_check`: whether the current day is outside the file's records.
    pub fn out_of_bounds(&self, clock: &Clock) -> bool {
        let mut out = false;
        if self.start_yr > clock.yrc || (self.start_yr == clock.yrc && self.start_day > clock.day) {
            out = true;
        }
        if self.end_yr < clock.yrc || (self.end_yr == clock.yrc && self.end_day < clock.day) {
            out = true;
        }
        out
    }
}

/// How a weather station gets one variable: generated (`sim`), or from gage `n` of the variable's
/// list (0 when the name wasn't found: an empty gage, always out of bounds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Sim,
    Gage(usize),
}

/// The day's weather at a station (`weather_daily`).
#[derive(Clone, Debug, Default)]
pub struct Weat {
    pub precip: f32,
    pub precip_next: f32,
    pub tmax: f32,
    pub tmin: f32,
    pub tave: f32,
    pub solrad: f32,
    pub solradmx: f32,
    pub rhum: f32,
    pub dewpt: f32,
    pub windsp: f32,
    pub pet: f32,
    pub phubase0: f32,
    pub ppet: f32,
    pub daylength: f32,
    pub precip_half_hr: f32,
    /// `precip_prior_day == "wet"`.
    pub prior_wet: bool,
}

/// A weather station: its generator, its sources and its weather.
#[derive(Clone, Debug)]
pub struct Station {
    pub name: String,
    /// 1-based weather generator (SWAT+ keeps 1 when the name isn't found).
    pub wgn: usize,
    pub pcp: Source,
    pub tmp: Source,
    pub slr: Source,
    pub hmd: Source,
    pub wnd: Source,
    /// PET gage, 0 for none.
    pub petgage: usize,
    pub weat: Weat,
    pub tlag: [f32; 6],
    pub tlag_mne: usize,
    pub airlag_temp: f32,
}

/// Weather generator parameters as `cli_initwgn` leaves them, and the generator's state.
#[derive(Clone, Debug)]
pub struct WgnState {
    pub name: String,
    pub lat: f32,
    pub elev: f32,
    /// The monthly parameters, as adjusted by `cli_initwgn`.
    pub m: [WgnMonth; 12],
    pub rain_yrs: f32,
    pub latsin: f32,
    pub latcos: f32,
    pub daylmn: f32,
    pub daylth: f32,
    pub phutot: f32,
    pub pr_wdays: [f32; 12],
    pub pcpmean: [f32; 12],
    pub pcf: [f32; 12],
    pub amp_r: [f32; 12],
    pub pet: [f32; 12],
    /// Dew point given as relative humidity (all months within 0–1).
    pub dew_is_rhum: bool,
    pub precip_mce: [f32; 30],
    pub pet_mce: [f32; 30],
    pub precip_sum: f32,
    pub pet_sum: f32,
    pub p_pet_rto: f32,
    /// Annual precipitation, the sum of the monthly means (`pcp_an`).
    pub pcp_an: f32,
    /// Average annual air temperature (`tmp_an`), from the monthly mean maxima and minima.
    pub tmp_an: f32,
    /// `rndseed(1:10, iwgn)`.
    pub seed: [i32; 10],
    pub rnd2: f32,
    pub rnd3: f32,
    pub rnd8: f32,
    pub rnd9: f32,
    pub wgncur: [f32; 3],
    pub wgnold: [f32; 3],
}

/// Everything the daily weather needs: gages, generators and stations.
#[derive(Clone, Debug)]
pub struct Weather {
    pub pcp: Vec<Gage>,
    pub tmp: Vec<Gage>,
    pub slr: Vec<Gage>,
    pub hmd: Vec<Gage>,
    pub wnd: Vec<Gage>,
    pub wgn: Vec<WgnState>,
    pub stations: Vec<Station>,
    /// `idg`: which seed each draw uses (1-based).
    pub idg: [usize; 9],
    /// Moving 30-day window position (`ppet_mce`).
    pub ppet_mce: usize,
    /// The seed left for conditional management (`rndseed_cond`).
    pub rndseed_cond: i32,
}

/// Default seeds of `gcycl`.
const SEEDS: [i32; 9] = [748932582, 1985072130, 1631331038, 67377721, 366304404, 1094585182, 1767585417, 1980520317, 392682216];

impl Weather {
    /// Sets up the weather as SWAT+ does while reading: the measured files in reading order
    /// (precipitation, temperature, solar radiation, humidity, wind), then the generator, then the
    /// stations.
    pub fn new(project: &Project, clock: &mut Clock) -> Self {
        let yrc = clock.yrc;
        let day_start = clock.day_start;
        let gages = |var: ClimateVar, clock: Option<&mut Clock>| build_gages(project, var, yrc, day_start, clock);
        let pcp = gages(ClimateVar::Pcp, None);
        let tmp = gages(ClimateVar::Tmp, Some(clock));
        let slr = gages(ClimateVar::Slr, None);
        let hmd = gages(ClimateVar::Hmd, None);
        let wnd = gages(ClimateVar::Wnd, None);

        let mut w = Weather {
            pcp,
            tmp,
            slr,
            hmd,
            wnd,
            wgn: Vec::new(),
            stations: Vec::new(),
            idg: [1, 2, 3, 4, 5, 6, 7, 8, 9],
            ppet_mce: 0,
            rndseed_cond: 0,
        };
        w.read_wgn(project, clock);
        w.read_stations(project);
        w
    }

    fn seed(&mut self, iwgn: usize, which: usize) -> Option<&mut i32> {
        let slot = *self.idg.get(which - 1)?;
        self.wgn.get_mut(iwgn - 1)?.seed.get_mut(slot - 1)
    }

    /// `cli_wgnread`: the generator stations, `gcycl`, then `cli_initwgn` for each.
    fn read_wgn(&mut self, project: &Project, clock: &Clock) {
        self.wgn = project.wgn.iter().map(new_wgn_state).collect();
        self.gcycl(project.params.igen);
        let slot = self.idg[2];
        for w in &mut self.wgn {
            init_wgn(w, clock, project.params.adj_pkr(), project.params.dorm_hr(), slot);
        }
    }

    /// SWAT+ `gcycl`: the seeds, moved by `igen` when it isn't 0, and the first draws.
    fn gcycl(&mut self, igen: i32) {
        self.idg = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        for w in &mut self.wgn {
            w.seed[..9].copy_from_slice(&SEEDS);
        }
        let mut rndseed10: i32 = 64_298_628;
        if igen != 0 {
            for j in 1..=9usize {
                let rn = aunif(&mut rndseed10);
                let ii = ((100 * igen) as f32 * rn) as i32;
                for _ in 0..ii.max(0) {
                    aunif(&mut rndseed10);
                }
                if let Some(w) = self.wgn.first_mut() {
                    w.seed[j - 1] = rndseed10;
                }
            }
            self.rndseed_cond = rndseed10;
            for j in (1..=9usize).rev() {
                let ii = self.idg[j - 1];
                let rn = aunif(&mut rndseed10);
                let k = (j as f32 * rn + 1.0) as usize;
                let k = k.clamp(1, 9);
                self.idg[j - 1] = self.idg[k - 1];
                self.idg[k - 1] = ii;
            }
        }
        let idg = self.idg;
        for w in &mut self.wgn {
            w.seed[9] = w.seed[idg[5] - 1];
            w.rnd2 = aunif(&mut w.seed[idg[1] - 1]);
            w.rnd3 = aunif(&mut w.seed[idg[2] - 1]);
            w.rnd8 = aunif(&mut w.seed[idg[7] - 1]);
            w.rnd9 = aunif(&mut w.seed[idg[8] - 1]);
        }
    }

    /// `cli_staread`: stations and the lookup of their generator and gages by name.
    fn read_stations(&mut self, project: &Project) {
        let wgn_names: Vec<String> = self.wgn.iter().map(|w| w.name.clone()).collect();
        let list = |var: ClimateVar| -> Vec<String> {
            project.station_lists.iter().find(|l| l.var == var).map(|l| l.files.clone()).unwrap_or_default()
        };
        let lists = [list(ClimateVar::Pcp), list(ClimateVar::Tmp), list(ClimateVar::Slr), list(ClimateVar::Hmd), list(ClimateVar::Wnd)];
        let pet_list = list(ClimateVar::Pet);
        let source = |name: &Option<String>, list: &[String]| -> Source {
            let name = name.as_deref().unwrap_or("null");
            if name == "sim" {
                return Source::Sim;
            }
            Source::Gage(search(list, name).unwrap_or(0))
        };
        self.stations = project
            .stations
            .iter()
            .map(|s| {
                let wgn = s.wgn.as_deref().and_then(|n| search(&wgn_names, n)).unwrap_or(1);
                let tlag0 = self.wgn.get(wgn - 1).map(|w| (w.m[0].tmp_min_ave + w.m[0].tmp_max_sd) / 2.0).unwrap_or(0.0);
                Station {
                    name: s.name.clone(),
                    wgn,
                    pcp: source(&s.pcp, &lists[0]),
                    tmp: source(&s.tmp, &lists[1]),
                    slr: source(&s.slr, &lists[2]),
                    hmd: source(&s.hmd, &lists[3]),
                    wnd: source(&s.wnd, &lists[4]),
                    petgage: s.pet.as_deref().and_then(|n| search(&pet_list, n)).unwrap_or(0),
                    weat: Weat::default(),
                    tlag: [tlag0; 6],
                    tlag_mne: 1,
                    airlag_temp: 0.0,
                }
            })
            .collect();
    }

    fn gage(list: &[Gage], i: usize) -> Option<&Gage> {
        list.get(i)
    }

    /// SWAT+ `cli_pgen`: generated precipitation for the next day at station `ist`.
    fn pgen(&mut self, ist: usize, mo: usize) {
        let Some(iwgn) = self.stations.get(ist).map(|s| s.wgn) else { return };
        let prior_wet = self.stations.get(ist).is_some_and(|s| s.weat.prior_wet);
        let vv = self.seed(iwgn, 1).map(aunif).unwrap_or(0.0);
        let Some(g) = self.wgn.get(iwgn - 1) else { return };
        let xx = if prior_wet { g.m[mo].wet_wet } else { g.m[mo].wet_dry };
        let pcpgen = if vv > xx {
            0.0
        } else {
            let v8 = self.seed(iwgn, 3).map(aunif).unwrap_or(0.0);
            let Some(g) = self.wgn.get_mut(iwgn - 1) else { return };
            let skw = g.m[mo].pcp_skew;
            let r6 = skw / 6.0;
            let mut xlv = (dstn1(g.rnd3, v8) - r6) * r6 + 1.0;
            xlv = (xlv * xlv * xlv - 1.0) * 2.0 / skw;
            g.rnd3 = v8;
            let mut p = xlv * g.m[mo].pcp_sd + g.pcpmean[mo];
            p *= g.pcf[mo];
            if p < 0.1 {
                p = 0.1;
            }
            p
        };
        if let Some(s) = self.stations.get_mut(ist) {
            s.weat.precip_next = pcpgen;
        }
    }

    /// SWAT+ `cli_precip_control`: today's precipitation is yesterday's "next", and the next day's
    /// is read or generated now. `istart` is 0 before the first day, 1 after.
    pub fn precip_control(&mut self, istart: i32, clock: &Clock) -> Result<(), String> {
        let mo = clock.month_index();
        for ist in 0..self.stations.len() {
            let Some(st) = self.stations.get_mut(ist) else { continue };
            st.weat.precip = st.weat.precip_next;
            st.weat.precip_next = 0.0;
            match st.pcp {
                Source::Sim => self.pgen(ist, mo),
                Source::Gage(ipg) => {
                    let default = Gage::default();
                    let g = Self::gage(&self.pcp, ipg).unwrap_or(&default);
                    if g.tstep > 0 {
                        return Err("sub-daily precipitation is not supported yet".into());
                    }
                    let mut cur_day = clock.day + istart;
                    let mut yrs_to_start = clock.yrs - g.yrs_start;
                    if cur_day > clock.day_end_yr {
                        cur_day = 1;
                        yrs_to_start += 1;
                    }
                    let mut out = g.out_of_bounds(clock);
                    if yrs_to_start > g.end_yr - g.start_yr + 1 {
                        out = true;
                    }
                    let next = if out { -98.0 } else { g.value(cur_day, yrs_to_start) };
                    if let Some(st) = self.stations.get_mut(ist) {
                        st.weat.precip_next = next;
                    }
                    if next <= -97.0 {
                        self.pgen(ist, mo);
                    }
                }
            }
        }
        Ok(())
    }

    /// SWAT+ `cli_weatgn`: the next values of the generator's correlated normal deviates.
    fn weatgn(&mut self, iwgn: usize) {
        const A: [[f32; 3]; 3] = [[0.567, 0.086, -0.002], [0.253, 0.504, -0.050], [-0.006, -0.039, 0.244]];
        const B: [[f32; 3]; 3] = [[0.781, 0.0, 0.0], [0.328, 0.637, 0.0], [0.238, -0.341, 0.873]];
        let v2 = self.seed(iwgn, 8).map(aunif).unwrap_or(0.0);
        let Some(g) = self.wgn.get_mut(iwgn - 1) else { return };
        let mut e = [0.0_f32; 3];
        e[0] = dstn1(g.rnd8, v2);
        g.rnd8 = v2;
        let v2 = self.seed(iwgn, 9).map(aunif).unwrap_or(0.0);
        let Some(g) = self.wgn.get_mut(iwgn - 1) else { return };
        e[1] = dstn1(g.rnd9, v2);
        g.rnd9 = v2;
        let v2 = self.seed(iwgn, 2).map(aunif).unwrap_or(0.0);
        let Some(g) = self.wgn.get_mut(iwgn - 1) else { return };
        e[2] = dstn1(g.rnd2, v2);
        g.rnd2 = v2;
        let mut xx = [0.0_f32; 3];
        for n in 0..3 {
            g.wgncur[n] = 0.0;
            for l in 0..3 {
                g.wgncur[n] += B[n][l] * e[l];
                xx[n] += A[n][l] * g.wgnold[l];
            }
        }
        for n in 0..3 {
            g.wgncur[n] += xx[n];
            g.wgncur[n] = g.wgncur[n].clamp(-1.0, 1.0);
            g.wgnold[n] = g.wgncur[n];
        }
    }

    /// SWAT+ `cli_tgen`.
    fn tgen(&mut self, ist: usize, mo: usize) {
        let Some(st) = self.stations.get(ist) else { return };
        let Some(g) = self.wgn.get(st.wgn - 1) else { return };
        let m = &g.m[mo];
        let tamp = 0.5 * (m.tmp_max_ave - m.tmp_min_ave);
        let mut txxm = m.tmp_max_ave + tamp * g.pr_wdays[mo];
        if st.weat.precip > 0.0 {
            txxm -= tamp;
        }
        let tmxg = txxm + m.tmp_max_sd * g.wgncur[0];
        let mut tmng = m.tmp_min_ave + m.tmp_min_sd * g.wgncur[1];
        if tmng > tmxg {
            tmng = tmxg - 0.2 * tmxg.abs();
        }
        if let Some(st) = self.stations.get_mut(ist) {
            st.weat.tmax = tmxg;
            st.weat.tmin = tmng;
        }
    }

    /// SWAT+ `cli_clgen`: yesterday-or-today wetness, day length and maximum solar radiation.
    fn clgen(&mut self, ist: usize, day: i32) {
        let Some(st) = self.stations.get(ist) else { return };
        let Some(g) = self.wgn.get(st.wgn - 1) else { return };
        let (latsin, latcos) = (g.latsin, g.latcos);
        let Some(st) = self.stations.get_mut(ist) else { return };
        st.weat.prior_wet = st.weat.precip >= 0.1;
        let rday = day as f32;
        let sd = (0.4_f32 * ((rday - 82.0) / 58.09).sin()).asin();
        let dd = 1.0_f32 + 0.033 * (rday / 58.09).cos();
        let sdlat = -latsin * sd.tan() / latcos;
        let h = if sdlat > 1.0 {
            0.0
        } else if sdlat >= -1.0 {
            sdlat.acos()
        } else {
            3.1416
        };
        st.weat.daylength = 7.6394 * h;
        let ys = latsin * sd.sin();
        let yc = latcos * sd.cos();
        st.weat.solradmx = 30.0 * dd * (h * ys + yc * h.sin());
    }

    /// SWAT+ `cli_slrgen`. It computes a generated value and then replaces it with the month's
    /// average (the last line of the Fortran), so generated solar radiation is the monthly mean.
    fn slrgen(&mut self, ist: usize, mo: usize) {
        let Some(st) = self.stations.get(ist) else { return };
        let Some(g) = self.wgn.get(st.wgn - 1) else { return };
        let solarav = g.m[mo].slr_ave;
        if let Some(st) = self.stations.get_mut(ist) {
            st.weat.solrad = solarav;
        }
    }

    /// SWAT+ `cli_rhgen`.
    fn rhgen(&mut self, ist: usize, mo: usize) {
        let Some(st) = self.stations.get(ist) else { return };
        let iwgn = st.wgn;
        let precip = st.weat.precip;
        let Some(g) = self.wgn.get(iwgn - 1) else { return };
        let m = &g.m[mo];
        let rhmo = if !g.dew_is_rhum {
            let tmpmean = (m.tmp_max_ave + m.tmp_min_ave) / 2.0;
            ee(m.dew_ave) / ee(tmpmean)
        } else {
            m.dew_ave
        };
        let yy = 0.9 * g.pr_wdays[mo];
        let mut rhm = (rhmo - yy) / (1.0 - yy);
        if rhm < 0.05 {
            rhm = 0.5 * rhmo;
        }
        if precip > 0.0 {
            rhm = rhm * 0.1 + 0.9;
        }
        let vv = rhm - 1.0;
        let uplm = rhm - vv * vv.exp();
        let blm = rhm * (1.0 - (-rhm).exp());
        let slot = self.idg[6];
        let Some(g) = self.wgn.get_mut(iwgn - 1) else { return };
        let r = atri(blm, rhm, uplm, &mut g.seed[slot - 1]);
        if let Some(st) = self.stations.get_mut(ist) {
            st.weat.rhum = r;
        }
    }

    /// SWAT+ `cli_wndgen` (wind direction is off in SWAT+: `iwndir = 0`).
    fn wndgen(&mut self, ist: usize, mo: usize) {
        let Some(iwgn) = self.stations.get(ist).map(|s| s.wgn) else { return };
        let v6 = self.seed(iwgn, 5).map(aunif).unwrap_or(0.0);
        let Some(g) = self.wgn.get(iwgn - 1) else { return };
        let windav = g.m[mo].wnd_ave;
        if let Some(st) = self.stations.get_mut(ist) {
            st.weat.windsp = windav * (-v6.ln()).powf(0.3);
        }
    }

    /// SWAT+ `climate_control`: one day of weather at every station.
    pub fn climate_control(&mut self, clock: &Clock) -> Result<(), String> {
        self.precip_control(1, clock)?;
        let mo = clock.month_index();
        let n = self.stations.len();

        // Temperature.
        for ist in 0..n {
            let Some(st) = self.stations.get(ist) else { continue };
            let (iwgn, src) = (st.wgn, st.tmp);
            self.weatgn(iwgn);
            match src {
                Source::Sim => self.tgen(ist, mo),
                Source::Gage(ig) => {
                    let default = Gage::default();
                    let g = Self::gage(&self.tmp, ig).unwrap_or(&default);
                    let (tmax, tmin) = if g.out_of_bounds(clock) {
                        (-98.0, -98.0)
                    } else {
                        let y = clock.yrs - g.yrs_start;
                        (g.value(clock.day, y), g.value2(clock.day, y))
                    };
                    if let Some(st) = self.stations.get_mut(ist) {
                        st.weat.tmax = tmax;
                        st.weat.tmin = tmin;
                    }
                    if tmax <= -97.0 || tmin <= -97.0 {
                        self.weatgn(iwgn);
                        self.tgen(ist, mo);
                    }
                }
            }
            if let Some(st) = self.stations.get_mut(ist) {
                st.weat.tave = (st.weat.tmax + st.weat.tmin) / 2.0;
            }
        }

        // Solar radiation.
        for ist in 0..n {
            self.clgen(ist, clock.day);
            let Some(src) = self.stations.get(ist).map(|s| s.slr) else { continue };
            match src {
                Source::Sim => self.slrgen(ist, mo),
                Source::Gage(ig) => {
                    let v = self.measured(&self.slr, ig, clock);
                    if let Some(st) = self.stations.get_mut(ist) {
                        st.weat.solrad = v;
                    }
                    if v <= -97.0 {
                        self.slrgen(ist, mo);
                    }
                }
            }
        }

        // Relative humidity and dew point.
        for ist in 0..n {
            let Some(src) = self.stations.get(ist).map(|s| s.hmd) else { continue };
            match src {
                Source::Sim => self.rhgen(ist, mo),
                Source::Gage(ig) => {
                    let v = self.measured(&self.hmd, ig, clock);
                    if let Some(st) = self.stations.get_mut(ist) {
                        st.weat.rhum = v;
                    }
                    if v <= -97.0 {
                        self.rhgen(ist, mo);
                    }
                }
            }
            if let Some(st) = self.stations.get_mut(ist) {
                st.weat.dewpt = st.weat.tave - (1.0 - st.weat.rhum) / 5.0;
            }
        }

        // Wind speed.
        for ist in 0..n {
            let Some(src) = self.stations.get(ist).map(|s| s.wnd) else { continue };
            match src {
                Source::Sim => self.wndgen(ist, mo),
                Source::Gage(ig) => {
                    let v = self.measured(&self.wnd, ig, clock);
                    if let Some(st) = self.stations.get_mut(ist) {
                        st.weat.windsp = v;
                    }
                    if v <= -97.0 {
                        self.wndgen(ist, mo);
                    }
                }
            }
        }

        if self.stations.iter().any(|s| s.petgage > 0) {
            return Err("measured PET (pet.cli) is not supported yet".into());
        }

        // Hargreaves PET, the moving 30-day precipitation and PET sums, and the air temperature lag.
        self.ppet_mce += 1;
        if self.ppet_mce > 30 {
            self.ppet_mce = 1;
        }
        let k = self.ppet_mce - 1;
        for ist in 0..n {
            let Some(st) = self.stations.get_mut(ist) else { continue };
            let w = &mut st.weat;
            let ramm = w.solradmx * 37.59 / 30.0;
            if w.tmax > w.tmin {
                let xl = 2.501 - 2.361e-3 * w.tave;
                w.pet = 0.0023 * (ramm / xl) * (w.tave + 17.8) * crate::fortran::rpow(w.tmax - w.tmin, 0.5);
                w.pet = w.pet.max(0.01);
            } else {
                w.pet = 0.01;
            }
            if w.pet > 0.1 {
                w.ppet += w.precip / w.pet;
            }
            let (precip, pet, tave) = (w.precip, w.pet, w.tave);
            let iwgn = st.wgn;
            if let Some(g) = self.wgn.get_mut(iwgn - 1) {
                g.precip_sum = g.precip_sum + precip - g.precip_mce[k];
                g.pet_sum = g.pet_sum + pet - g.pet_mce[k];
                g.p_pet_rto = if g.pet_sum > 1.0e-6 { g.precip_sum / g.pet_sum } else { g.precip_sum / 1.0e-6 };
                g.precip_mce[k] = precip;
                g.pet_mce[k] = pet;
            }
            let Some(st) = self.stations.get_mut(ist) else { continue };
            st.tlag[st.tlag_mne - 1] = tave;
            st.tlag_mne += 1;
            if st.tlag_mne > 6 {
                st.tlag_mne = 1;
            }
            st.airlag_temp = st.tlag[st.tlag_mne - 1];
        }

        // Fraction of the day's rain in the wettest half hour: a draw from seed 10 every day.
        for ist in 0..n {
            let Some(st) = self.stations.get(ist) else { continue };
            let (iwgn, precip) = (st.wgn, st.weat.precip);
            let half_hr_mn = 0.02083_f32;
            let xx = -125.0_f32 / (precip + 5.0);
            let half_hr_mx = 1.0 - xx.exp();
            let Some(g) = self.wgn.get_mut(iwgn - 1) else { continue };
            let amp = g.amp_r[mo];
            let v = atri(half_hr_mn, amp, half_hr_mx, &mut g.seed[9]);
            if let Some(st) = self.stations.get_mut(ist) {
                st.weat.precip_half_hr = v;
            }
        }

        // Base-zero heat units.
        for ist in 0..n {
            let Some(st) = self.stations.get(ist) else { continue };
            let phutot = self.wgn.get(st.wgn - 1).map(|g| g.phutot).unwrap_or(0.0);
            let Some(st) = self.stations.get_mut(ist) else { continue };
            if phutot > 0.0 {
                if st.weat.tave > 0.0 {
                    st.weat.phubase0 += st.weat.tave / phutot;
                }
            } else {
                st.weat.phubase0 = 0.0;
            }
            if clock.end_yr {
                st.weat.phubase0 = 0.0;
            }
        }

        // Monthly adjustments (all zero without calibration) and the humidity limits.
        for st in &mut self.stations {
            let w = &mut st.weat;
            w.precip *= 1.0 + 0.0 / 100.0;
            if w.precip < 0.0 {
                w.precip = 0.0;
            }
            w.tmax += 0.0;
            w.tmin += 0.0;
            w.solrad += 0.0;
            w.solrad = w.solrad.max(0.0);
            w.rhum += 0.0;
            w.rhum = w.rhum.max(0.01).min(0.99);
        }
        Ok(())
    }

    /// A measured value for today, or -98 outside the gage's records.
    fn measured(&self, list: &[Gage], ig: usize, clock: &Clock) -> f32 {
        let default = Gage::default();
        let g = Self::gage(list, ig).unwrap_or(&default);
        if g.out_of_bounds(clock) { -98.0 } else { g.value(clock.day, clock.yrs - g.yrs_start) }
    }
}

/// SWAT+ `ee`: saturation vapour pressure (kPa) at `tk` °C.
pub fn ee(tk: f32) -> f32 {
    if tk + 237.3 != 0.0 { ((16.78 * tk - 116.9) / (tk + 237.3)).exp() } else { 0.0 }
}

/// Builds the gages of one variable in list order, index 0 being the empty gage. As the readers,
/// the first file without data stops the variable: it and the files after it stay empty.
fn build_gages(project: &Project, var: ClimateVar, yrc: i32, day_start: i32, mut clock: Option<&mut Clock>) -> Vec<Gage> {
    let files = project.station_lists.iter().find(|l| l.var == var).map(|l| l.files.as_slice()).unwrap_or(&[]);
    let mut series = project.series.iter().filter(|s| s.var == var);
    let mut out = vec![Gage::default(); files.len() + 1];
    for (i, file) in files.iter().enumerate() {
        let Some(s) = series.next() else { break };
        if &s.file != file {
            break;
        }
        match Gage::place(s, yrc, day_start, clock.as_deref_mut()) {
            Some(g) => {
                if let Some(slot) = out.get_mut(i + 1) {
                    *slot = g;
                }
            }
            None => break,
        }
    }
    out
}

fn new_wgn_state(w: &Wgn) -> WgnState {
    WgnState {
        name: w.name.clone(),
        lat: w.lat,
        elev: w.elev,
        m: w.months,
        rain_yrs: w.rain_yrs,
        latsin: 0.0,
        latcos: 0.0,
        daylmn: 0.0,
        daylth: 0.0,
        phutot: 0.0,
        pr_wdays: [0.0; 12],
        pcpmean: [0.0; 12],
        pcf: [0.0; 12],
        amp_r: [0.0; 12],
        pet: [0.0; 12],
        dew_is_rhum: false,
        precip_mce: [0.0; 30],
        pet_mce: [0.0; 30],
        precip_sum: 0.0,
        pet_sum: 0.0,
        p_pet_rto: 0.0,
        pcp_an: 0.0,
        tmp_an: 0.0,
        seed: [0; 10],
        rnd2: 0.0,
        rnd3: 0.0,
        rnd8: 0.0,
        rnd9: 0.0,
        wgncur: [0.0; 3],
        wgnold: [0.0; 3],
    }
}

/// SWAT+ `cli_initwgn`. Month lengths come from the global `ndays` as the readers left it, not
/// from the simulation's years; `time%mo` is the first day's month as `time_read` found it.
fn init_wgn(w: &mut WgnState, clock: &Clock, adj_pkr: f32, dorm_hr: f32, seed3_slot: usize) {
    let ndays = clock.ndays;
    let mdays = |mon: usize| (ndays[mon + 1] - ndays[mon]) as f32;

    let xx = w.lat / 57.296;
    w.latsin = xx.sin();
    w.latcos = xx.cos();
    let lattan = xx.tan();
    let x1 = 0.4348 * lattan.abs();
    let x2 = if x1 < 1.0 { x1.acos() } else { 0.0 };
    w.daylmn = 7.6394 * x2;
    w.daylth = if dorm_hr < 1.0e-6 {
        if w.lat.abs() > 40.0 {
            1.0
        } else if w.lat.abs() < 20.0 {
            -1.0
        } else {
            (w.lat.abs() - 20.0) / 20.0
        }
    } else {
        dorm_hr
    };

    let rhmx: [f32; 12] = std::array::from_fn(|i| w.m[i].pcp_hhr);
    let mut rain_hhsm = [0.0_f32; 12];
    rain_hhsm[0] = (rhmx[11] + rhmx[0] + rhmx[1]) / 3.0;
    for mon in 1..11 {
        rain_hhsm[mon] = (rhmx[mon - 1] + rhmx[mon] + rhmx[mon + 1]) / 3.0;
    }
    rain_hhsm[11] = (rhmx[10] + rhmx[11] + rhmx[0]) / 3.0;

    let mut summm_p = 0.0_f32;
    let mut summx_t = 0.0_f32;
    let mut summn_t = 0.0_f32;
    for mon in 0..12 {
        let md = ndays[mon + 1] - ndays[mon];
        let m = &mut w.m[mon];
        summx_t += m.tmp_max_ave;
        summn_t += m.tmp_min_ave;
        let tav = (m.tmp_max_ave + m.tmp_min_ave) / 2.0;
        if tav > 0.0 {
            w.phutot += tav * md as f32;
        }
        if m.wet_wet <= m.wet_dry || m.wet_dry <= 0.0 {
            if m.pcp_days < 0.1 {
                m.pcp_days = 0.1;
            }
            m.wet_dry = 0.75 * m.pcp_days / md as f32;
            m.wet_wet = 0.25 + m.wet_dry;
        } else {
            m.pcp_days = md as f32 * m.wet_dry / (1.0 - m.wet_wet + m.wet_dry);
        }
        if m.pcp_days <= 0.0 {
            m.pcp_days = 0.001;
        }
        w.pr_wdays[mon] = m.pcp_days / md as f32;
        w.pcpmean[mon] = m.pcp_ave / m.pcp_days;
        if m.pcp_skew < 0.2 {
            m.pcp_skew = 0.2;
        }
        summm_p += m.pcp_ave;
        let tav = (m.tmp_max_ave + m.tmp_min_ave) / 2.0;
        let tk = tav + 273.0;
        let alb = 0.15_f32;
        let d = (21.255_f32 - 5304.0 / tk).exp() * 5304.0 / (tk * tk);
        let gma = d / (d + 0.68);
        let ho = 23.9 * m.slr_ave * (1.0 - alb) / 58.3;
        let aph = 1.28_f32;
        w.pet[mon] = aph * ho * gma * md as f32;
    }
    w.pcp_an = summm_p;
    w.tmp_an = (summx_t + summn_t) / 24.0;

    w.dew_is_rhum = !w.m.iter().any(|m| m.dew_ave > 1.0 || m.dew_ave < 0.0);

    let mo_ppet = if clock.mo == 1 { 12 } else { clock.mo - 1 };
    let mp = (mo_ppet.clamp(1, 12) - 1) as usize;
    w.precip_sum = 0.0;
    w.pet_sum = 0.0;
    for k in 0..30 {
        w.precip_mce[k] = w.m[mp].pcp_ave / mdays(mp);
        w.pet_mce[k] = w.pet[mp] / mdays(mp);
        w.precip_sum += w.precip_mce[k];
        w.pet_sum += w.pet_mce[k];
    }

    // Precipitation correction factors from 1000 generated values per month, drawn from a copy
    // of the precipitation amount seed (`rndseed(idg(3))`; the generator's own seed doesn't move).
    let mut xrnd = w.seed.get(seed3_slot.wrapping_sub(1)).copied().unwrap_or(0);
    let mut rndm1 = aunif(&mut xrnd);
    for mon in 0..12 {
        let r6 = w.m[mon].pcp_skew / 6.0;
        let mut sum = 0.0_f32;
        for _ in 0..1000 {
            let rnm2 = aunif(&mut xrnd);
            let mut xlv = (dstn1(rndm1, rnm2) - r6) * r6 + 1.0;
            rndm1 = rnm2;
            xlv = (xlv * xlv * xlv - 1.0) * 2.0 / w.m[mon].pcp_skew;
            let mut pcp_gen = xlv * w.m[mon].pcp_sd + w.pcpmean[mon];
            if pcp_gen < 0.01 {
                pcp_gen = 0.01;
            }
            sum += pcp_gen;
        }
        w.pcf[mon] = if sum > 0.0 { 1000.0 * w.pcpmean[mon] / sum } else { 1.0 };
        if w.rain_yrs < 1.0 {
            w.rain_yrs = 10.0;
        }
        let x1 = 0.5 / w.rain_yrs;
        let x2 = x1 / w.m[mon].pcp_days;
        let x3 = rain_hhsm[mon] / x2.ln();
        w.amp_r[mon] = if w.pcpmean[mon] > 1.0e-4 { adj_pkr * (1.0 - (x3 / w.pcpmean[mon]).exp()) } else { 0.95 };
        w.amp_r[mon] = w.amp_r[mon].clamp(0.1, 0.95);
    }
}
