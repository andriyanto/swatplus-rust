//! Reservoirs (`res_initial`, `res_control`, `res_hydro`) and recall objects (`recall_read`,
//! the `recall` command): water held behind a dam and released by a decision table of
//! `res_rel.dtl`; measured or modelled loads put into the network.

use swat_model::Project;
use swat_model::table::{Table, fortran_eq};

use crate::channel::ChWat;
use crate::fortran::rpow;
use crate::hyd::Hyd;
use crate::waterbody::{WbPrm, res_nutrient, res_sediment};

/// The volume a `vol` condition tests against.
#[derive(Clone, Copy, Debug, PartialEq)]
enum VolTarg {
    Pvol,
    Evol,
    /// `e-pv`: the principal volume plus a share of the flood storage.
    EmPv,
}

/// A condition of a release table.
#[derive(Clone, Debug, PartialEq)]
enum Cond {
    Vol { targ: VolTarg, op: char, lim: f32, alt: Vec<String> },
    Month { lim: i32, alt: Vec<String> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Base {
    Zero,
    Pvol,
    Evol,
}

/// How an action releases water (`res_hydro`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Release {
    Rate(f32),
    RatePct(f32),
    InfloRate { min: f32, plus: f32 },
    InfloFrac(f32),
    AbEmer,
    Days { days: f32, base: Base, frac: f32 },
    Dyrt { days: f32, base: Base, pct: f32 },
}

/// A reservoir's data and state.
#[derive(Clone, Debug)]
pub struct Reservoir {
    /// Object index in the network.
    pub ob: usize,
    /// Operational from this year and month (`iyres`, `mores`).
    iyres: i32,
    mores: i32,
    /// Volumes (m3) and areas (ha) at the principal and emergency spillways.
    pvol: f32,
    evol: f32,
    /// The volume-area curve (`br1`, `br2`), the release lags (`lag_up`, `lag_down`) and the
    /// release of yesterday.
    br1: f32,
    br2: f32,
    lag_up: f32,
    lag_down: f32,
    prev_flo: f32,
    /// Bottom conductivity (mm/h), evaporation coefficient.
    k: f32,
    evrsv: f32,
    alts: usize,
    conds: Vec<Cond>,
    acts: Vec<(Release, Vec<String>)>,
    prm: WbPrm,
    /// The water and what it holds (`res`), its water body (`res_wat_d`), and today's inflow
    /// and outflow as printed (`res_in_d`, `res_out_d`).
    pub water: Hyd,
    pub wat: ChWat,
    pub in_d: Hyd,
    pub out_d: Hyd,
    /// Time steps a day (`res_hydro`'s `nstep`).
    nstep: usize,
}

fn row<'a>(t: Option<&'a Table>, name: &str) -> Option<(&'a Table, usize)> {
    let t = t?;
    (0..t.rows.len()).find(|&r| t.text(r, "name").is_some_and(|n| fortran_eq(n, name))).map(|r| (t, r))
}

impl Reservoir {
    /// `res_read` and `res_initial` for reservoir `ires` (0-based), object `ob`, whose
    /// `reservoir.con` points at `reservoir.res` row `props`, the simulation starting in
    /// `yrc` (month 1). SWAT+ takes the hydrology, sediment and nutrient rows of the
    /// reservoir's own number and the release table and initial water of `props`'s.
    pub fn new(project: &Project, ires: usize, ob: usize, props: i32, yrc: i32) -> Result<Reservoir, String> {
        let dat = project.res_dat.as_ref().ok_or("reservoirs without reservoir.res")?;
        let by_id = |id: usize| (0..dat.rows.len()).find(|&r| dat.int(r, "id").unwrap_or(0) as usize == id);
        let own = by_id(ires + 1).ok_or_else(|| format!("reservoir {} not in reservoir.res", ires + 1))?;
        let pr = by_id(props.max(0) as usize).ok_or_else(|| format!("reservoir row {props} not in reservoir.res"))?;
        let s = |r: usize, f: &str| dat.text(r, f).unwrap_or("").to_string();
        let (hy, ih) = row(project.res_hyd.as_ref(), &s(own, "hyd"))
            .ok_or_else(|| format!("reservoir hydrology {} not in hydrology.res", s(own, "hyd")))?;
        let x = |f: &str| hy.real(ih, f).unwrap_or(0.0);
        let xi = |f: &str| hy.int(ih, f).unwrap_or(0);
        let evol = x("evol") * 10000.0;
        let pvol = x("pvol") * 10000.0;
        let (esa, psa) = (x("esa"), x("psa"));
        // The volume-area curve; `lnvol` is an integer.
        let resdif = x("evol") - x("pvol");
        let (br1, br2);
        if esa - psa > 0.0 && resdif > 0.0 {
            let lnvol = (evol.log10() - pvol.log10()) as i32;
            let mut b2 =
                if lnvol as f32 > 1.0e-4 { (esa.log10() - psa.log10()) / lnvol as f32 } else { (esa.log10() - psa.log10()) / 0.001 };
            if b2 > 0.9 {
                b2 = 0.9;
                br1 = rpow(psa / pvol, 0.9);
            } else {
                br1 = rpow(esa / evol, b2);
            }
            br2 = b2;
        } else {
            br2 = 0.9;
            br1 = if pvol > 1.0e-6 { rpow(psa / pvol, 0.9) } else { 0.1 };
        }

        // The release table.
        let rel = s(pr, "rel");
        if rel.starts_with("ctbl_") {
            return Err(format!("reservoir release by condition table {rel}"));
        }
        let dt = project
            .res_dtl
            .iter()
            .find(|d| fortran_eq(&d.name, &rel))
            .ok_or_else(|| format!("reservoir release table {rel} not in res_rel.dtl"))?;
        let mut conds = Vec::new();
        for (c, alt) in dt.conds.iter().zip(&dt.alt) {
            if c.ob_num != 0 {
                return Err(format!("reservoir release condition on object {}", c.ob_num));
            }
            let cond = match c.var.as_str() {
                "vol" => {
                    let targ = match c.lim_var.as_str() {
                        "pvol" => VolTarg::Pvol,
                        "evol" => VolTarg::Evol,
                        "e-pv" => VolTarg::EmPv,
                        v => return Err(format!("reservoir release condition on {v}")),
                    };
                    let op = match c.lim_op.as_str() {
                        "*" => '*',
                        "+" => '+',
                        "-" => '-',
                        "/" => '/',
                        o if targ == VolTarg::EmPv => o.chars().next().unwrap_or('*'),
                        o => return Err(format!("reservoir release condition operator {o}")),
                    };
                    Cond::Vol { targ, op, lim: c.lim_const, alt: alt.clone() }
                }
                "month" => Cond::Month { lim: c.lim_const as i32, alt: alt.clone() },
                v => return Err(format!("reservoir release condition {v}")),
            };
            conds.push(cond);
        }
        let mut acts = Vec::new();
        for (a, out) in dt.acts.iter().zip(&dt.outcomes) {
            let base = || match a.file_pointer.as_str() {
                "null" => Ok(Base::Zero),
                "pvol" => Ok(Base::Pvol),
                "evol" => Ok(Base::Evol),
                fp => Err(format!("reservoir release base {fp}")),
            };
            let release = match a.option.as_str() {
                "rate" => Release::Rate(a.const_),
                "rate_pct" => Release::RatePct(a.const_),
                "inflo_rate" => Release::InfloRate { min: a.const_, plus: a.const2 },
                "inflo_frac" => Release::InfloFrac(a.const_),
                "ab_emer" => Release::AbEmer,
                "days" => Release::Days { days: a.const_, base: base()?, frac: a.const2 },
                "dyrt" => Release::Dyrt { days: a.const_, base: base()?, pct: a.const2 },
                o => return Err(format!("reservoir release option {o}")),
            };
            acts.push((release, out.clone()));
        }

        let prm = WbPrm::new(project, &s(own, "sed"), &s(own, "nut"));
        let mut r = Reservoir {
            nstep: project.time.step.max(1) as usize,
            ob,
            iyres: xi("iyres"),
            mores: xi("mores"),
            pvol,
            evol,
            br1,
            br2,
            lag_up: x("br1"),
            lag_down: x("br2"),
            prev_flo: 0.0,
            k: hy.real(ih, "k").unwrap_or(0.01),
            evrsv: hy.real(ih, "evrsv").unwrap_or(0.7),
            alts: dt.alts,
            conds,
            acts,
            prm,
            water: Hyd::default(),
            wat: ChWat::default(),
            in_d: Hyd::default(),
            out_d: Hyd::default(),
        };
        // The initial water: om_water.ini's row for initial.res's org_min, as shares of the
        // principal volume and concentrations (res_convert_mass), if operational at the start.
        if r.operational(yrc, 1) {
            let init = s(pr, "init");
            if let Some((ti, ir)) = row(project.res_init.as_ref(), &init) {
                let om = ti.text(ir, "org_min").unwrap_or("").to_string();
                if let Some((t, k)) = row(project.om_water.as_ref(), &om) {
                    let v = |f: &str| t.real(k, f).unwrap_or(0.0);
                    let flo = v("flo") * pvol;
                    let ppm = |f: &str| v(f) * flo / 1_000_000.0;
                    let kg = |f: &str| v(f) * flo / 1000.0;
                    r.water = Hyd {
                        flo,
                        sed: ppm("sed"),
                        orgn: kg("orgn"),
                        sedp: kg("sedp"),
                        no3: kg("no3"),
                        solp: kg("solp"),
                        chla: kg("chla"),
                        nh3: kg("nh3"),
                        no2: kg("no2"),
                        cbod: kg("cbod"),
                        dox: kg("dox"),
                        san: ppm("san"),
                        sil: ppm("sil"),
                        cla: ppm("cla"),
                        sag: ppm("sag"),
                        lag: ppm("lag"),
                        grv: ppm("grv"),
                        temp: v("temp"),
                    };
                }
            }
            r.wat.area_ha = r.br1 * rpow(r.water.flo, r.br2);
        }
        Ok(r)
    }

    fn operational(&self, yrc: i32, mo: i32) -> bool {
        yrc > self.iyres || (mo >= self.mores && yrc == self.iyres)
    }

    /// `conditions` and `res_hydro`: the release (m3), the actions taken once for each of the
    /// day's time steps.
    fn release(&self, mo: i32, hin: &Hyd) -> f32 {
        let nstep = self.nstep;
        let ns = nstep as f32;
        let mut hit = vec![true; self.alts];
        let cur = self.water.flo;
        for c in &self.conds {
            match c {
                Cond::Vol { targ, op, lim, alt } => {
                    let tbl = match targ {
                        VolTarg::EmPv => self.pvol + (self.evol - self.pvol) * lim,
                        _ => {
                            let v = if *targ == VolTarg::Evol { self.evol } else { self.pvol };
                            match op {
                                '*' => v * lim,
                                '+' => v + 10000.0 * lim,
                                '-' => v - 10000.0 * lim,
                                _ => v / lim,
                            }
                        }
                    };
                    crate::hru::mgt::cond_test(&mut hit, alt, cur, tbl);
                }
                Cond::Month { lim, alt } => crate::hru::mgt::cond_test(&mut hit, alt, mo, *lim),
            }
        }
        let mut flo = 0.0_f32;
        for (rel, out) in self.acts.iter().cycle().take(self.acts.len() * nstep) {
            let action = self.alts == 0 || (0..self.alts).any(|k| hit[k] && out.get(k).is_some_and(|o| o == "y"));
            if !action {
                continue;
            }
            let base = |b: Base| match b {
                Base::Zero => 0.0,
                Base::Pvol => self.pvol,
                Base::Evol => self.evol,
            };
            match *rel {
                Release::Rate(r) => flo += r * 86400.0,
                Release::RatePct(p) => flo += p * self.pvol / 100.0,
                Release::InfloRate { min, plus } => flo += (hin.flo + plus * 86400.0).max(min * 86400.0),
                Release::InfloFrac(f) => flo += hin.flo * f,
                Release::AbEmer => {
                    if cur > self.evol {
                        flo += cur - self.evol;
                    }
                }
                Release::Days { days, base: b, frac } => {
                    let b_lo = if b == Base::Zero { 0.0 } else { base(b) * frac };
                    flo += (cur - b_lo) / days / ns;
                    flo = flo.max(0.0);
                }
                Release::Dyrt { days, base: b, pct } => {
                    let b_lo = base(b).max(0.0);
                    flo += (cur - b_lo) / days + pct * self.pvol / 100.0 / ns;
                    flo = flo.max(0.0);
                }
            }
        }
        flo
    }

    /// SWAT+ `res_control`: the day's inflow `hin`, the station's precipitation (mm), PET (mm)
    /// and mean temperature. Returns the outflow. `print` keeps the day's inflow and outflow for
    /// the tables (`time%yrs > nyskip`).
    #[allow(clippy::too_many_arguments)]
    pub fn control(&mut self, hin: Hyd, yrc: i32, mo: i32, precip: f32, pet: f32, tave: f32, print: bool) -> Hyd {
        if !self.operational(yrc, mo) {
            return hin;
        }
        let mut ht2 = Hyd::default();
        self.water = self.water.plus(hin);
        ht2.flo = self.release(mo, &hin);
        let alpha_up = (-self.lag_up).exp();
        let alpha_down = (-self.lag_down).exp();
        if self.prev_flo < ht2.flo {
            ht2.flo = ht2.flo * alpha_up + self.prev_flo * (1.0 - alpha_up);
        } else {
            ht2.flo = ht2.flo * alpha_down + self.prev_flo * (1.0 - alpha_down);
        }
        self.prev_flo = ht2.flo;
        res_sediment(&mut self.water, &mut ht2, &self.prm);
        let w = &mut self.wat;
        w.evap = 10.0 * self.evrsv * pet * w.area_ha;
        w.precip = 10.0 * precip * w.area_ha;
        w.seep = 240.0 * self.k * w.area_ha;
        let b = &mut self.water;
        b.flo += w.precip;
        b.flo -= ht2.flo;
        if b.flo < 0.0 {
            ht2.flo += b.flo;
            b.flo = 0.0;
        }
        b.flo -= w.evap;
        if b.flo < 0.0 {
            w.evap += b.flo;
            b.flo = 0.0;
        }
        b.flo -= w.seep;
        if b.flo < 0.0 {
            w.seep += b.flo;
            b.flo = 0.0;
        }
        w.area_ha = if b.flo > 0.0 { self.br1 * rpow(b.flo, self.br2) } else { 0.0 };
        res_nutrient(&mut self.water, &mut ht2, &self.prm, mo, tave);
        if print {
            self.in_d = hin;
            self.out_d = ht2;
        }
        ht2
    }
}

/// A recall object's loads by time step and simulation year (`recall%hd`), as `recall_read`
/// stores them, and the years they cover.
#[derive(Clone, Debug)]
pub struct Recall {
    pub ob: usize,
    typ: i32,
    start_yr: i32,
    end_yr: i32,
    /// Steps a year (366 days, 12 months, 1), then the data by `(step - 1) + steps · (iyrs - 1)`.
    steps: usize,
    hd: Vec<Hyd>,
    /// Today's load (`rec_d`).
    pub d: Hyd,
}

impl Recall {
    /// `recall_read` for the recall object at `ob` pointing at `recall.rec` row `props`, the
    /// simulation from `yrc` to `yrc_end` (`nbyr` years).
    pub fn new(project: &Project, ob: usize, props: i32, yrc: i32, yrc_end: i32, nbyr: usize) -> Result<Recall, String> {
        let db =
            project.recalls.iter().find(|r| r.id == props.max(0) as usize).ok_or_else(|| format!("recall {props} not in recall.rec"))?;
        let steps = match db.typ {
            1 => 366,
            2 => 12,
            3 => 1,
            0 => return Err("subdaily recall objects".into()),
            _ => return Err(format!("recall objects of type {}", db.typ)),
        };
        let mut hd = vec![Hyd::default(); steps * nbyr.max(1)];
        let rows = &db.rows;
        let start_yr = rows.first().map(|r| r.iyr).unwrap_or(yrc);
        // Skip to the first record of the simulation's first year.
        let (mut k, mut iyrs) = if start_yr <= yrc {
            let k = rows.iter().position(|r| r.iyr == yrc).ok_or_else(|| format!("recall {}: no record of {yrc}", db.name))?;
            (k, 1usize)
        } else {
            (0, (start_yr - yrc + 1) as usize)
        };
        let mut iyr = rows.get(k).map(|r| r.iyr).unwrap_or(start_yr);
        while let Some(r) = rows.get(k) {
            let iyr1 = iyr;
            iyr = r.iyr;
            if iyr > yrc_end {
                break;
            }
            if iyr1 != iyr {
                iyrs += 1;
            }
            let step = match db.typ {
                1 => r.jday,
                2 => r.mo,
                _ => 1,
            };
            if r.vals[0] < 0.0 {
                return Err(format!("recall {}: a diversion (negative flow)", db.name));
            }
            let v = &r.vals;
            let h = Hyd {
                flo: v[0],
                sed: v[1],
                orgn: v[2],
                sedp: v[3],
                no3: v[4],
                solp: v[5],
                chla: v[6],
                nh3: v[7],
                no2: v[8],
                cbod: v[9],
                dox: v[10],
                san: v[11],
                sil: v[12],
                cla: v[13],
                sag: v[14],
                lag: v[15],
                grv: v[16],
                temp: v[17],
            };
            if step >= 1 && (step as usize) <= steps && iyrs >= 1 && iyrs <= nbyr {
                hd[(step as usize - 1) + steps * (iyrs - 1)] = h;
            } else {
                return Err(format!("recall {}: record out of range (step {step}, year {iyr})", db.name));
            }
            k += 1;
        }
        Ok(Recall { ob, typ: db.typ, start_yr, end_yr: iyr, steps, hd, d: Hyd::default() })
    }

    /// The `recall` command on day `day` of month `mo` in calendar year `yrc`, simulation year
    /// `yrs`: the day's load (none outside the years of the file; a yearly one always).
    pub fn day(&mut self, day: i32, mo: i32, yrc: i32, yrs: i32) -> Hyd {
        let at = |step: i32| {
            let i = (step.max(1) as usize - 1) + self.steps * (yrs.max(1) as usize - 1);
            self.hd.get(i).copied().unwrap_or_default()
        };
        let within = yrc >= self.start_yr && yrc <= self.end_yr;
        self.d = match self.typ {
            1 if within => at(day),
            2 if within => at(mo),
            // `.or.`: always.
            3 => at(1),
            _ => Hyd::default(),
        };
        self.d
    }
}
