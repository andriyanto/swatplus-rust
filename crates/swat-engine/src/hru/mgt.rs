//! Scheduled management (`mgt_operatn`, `mgt_sched`): planting, harvest and kill, tillage,
//! irrigation and fertilizer on their dates, and the skip to the next year.
//!
//! So far for dated operations of these kinds, without decision tables.

use swat_model::table::{Table, fortran_eq};
use swat_model::{MgtSchedule, Project};

use super::HruBasin;
use super::plant::{OrgMass, Plant};

/// What an operation does, with the database row it points to (element 0, all defaults, when the
/// name isn't found).
#[derive(Clone, Debug, PartialEq)]
pub enum OpKind {
    Plant,
    /// Harvest (grain, or none for another type) and kill.
    HarvKill {
        grain: bool,
        eff: f32,
        bm_min: f32,
    },
    Till {
        effmix: f32,
        deptil: f32,
    },
    Irr {
        amt_mm: f32,
        eff: f32,
        surq: f32,
    },
    Fert(Fert),
    Skip,
}

/// A fertilizer application: the `fertilizer.frt` row, the share left at the surface
/// (`chem_app.ops`) and the amount (kg/ha).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fert {
    pub fminn: f32,
    pub fminp: f32,
    pub forgn: f32,
    pub forgp: f32,
    pub fnh3n: f32,
    pub surf_frac: f32,
    pub kg: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub kind: OpKind,
    pub mon: i32,
    pub day: i32,
    /// The plant it applies to (`op_char`).
    pub op_char: String,
}

/// An HRU's schedule and where it stands (`hru%cur_op`, 0-based here; `yr_skip`).
#[derive(Clone, Debug, PartialEq)]
pub struct Mgt {
    pub ops: Vec<Op>,
    pub cur_op: usize,
    pub yr_skip: bool,
}

fn row_real(t: Option<&Table>, name: &str, field: &str, default: f32) -> f32 {
    match t.and_then(|t| t.find(name).map(|r| (t, r))) {
        Some((t, r)) => t.real(r, field).unwrap_or(default),
        None => default,
    }
}

/// SWAT+ `jdt`: the day of the year of a month and day (0 without a month).
fn jdt(ndays: &[i32; 13], day: i32, mon: i32) -> i32 {
    if mon == 0 {
        0
    } else if mon <= 2 {
        ndays[(mon - 1) as usize] + day
    } else {
        ndays[(mon - 1) as usize] - 1 + day
    }
}

impl Mgt {
    /// `read_mgtops` for schedule `s`, with `plant_init`'s first operation for a community whose
    /// rotation starts in year `rot_yr_ini`, the simulation starting on day `day_start`.
    /// `Err` names what isn't ported yet.
    pub fn new(project: &Project, s: &MgtSchedule, ndays: &[i32; 13], rot_yr_ini: i32, day_start: i32) -> Result<Mgt, String> {
        if !s.autos.is_empty() {
            return Err(format!("schedule {}: decision tables ({})", s.name, s.autos.join(", ")));
        }
        let mut ops = Vec::with_capacity(s.ops.len());
        let mut jdays = Vec::with_capacity(s.ops.len());
        for o in &s.ops {
            if o.husc > 0.0 {
                return Err(format!("schedule {}: {} by heat units", s.name, o.op));
            }
            let kind = match o.op.as_str() {
                "plnt" => OpKind::Plant,
                "hvkl" => {
                    let t = project.harv_ops.as_ref();
                    let typ = t.and_then(|t| t.find(&o.op_plant).and_then(|r| t.text(r, "typ"))).unwrap_or("");
                    let grain = matches!(typ, "grain" | "stripper" | "picker");
                    if matches!(typ, "biomass" | "residue" | "tuber" | "peanuts") {
                        return Err(format!("schedule {}: {typ} harvest", s.name));
                    }
                    OpKind::HarvKill { grain, eff: row_real(t, &o.op_plant, "eff", 0.0), bm_min: row_real(t, &o.op_plant, "bm_min", 0.0) }
                }
                "till" => {
                    if project.codes.cswat != 2 {
                        return Err(format!("schedule {}: tillage with carbon model {}", s.name, project.codes.cswat));
                    }
                    let t = project.tillage.as_ref();
                    OpKind::Till { effmix: row_real(t, &o.op_char, "effmix", 0.0), deptil: row_real(t, &o.op_char, "deptil", 0.0) }
                }
                "irrm" => {
                    let t = project.irr_ops.as_ref();
                    OpKind::Irr {
                        amt_mm: row_real(t, &o.op_char, "amt_mm", 25.4),
                        eff: row_real(t, &o.op_char, "eff", 0.0),
                        surq: row_real(t, &o.op_char, "surq", 0.0),
                    }
                }
                "fert" => {
                    let t = project.fertilizer.as_ref();
                    let x = |f: &str| row_real(t, &o.op_char, f, 0.0);
                    OpKind::Fert(Fert {
                        fminn: x("fminn"),
                        fminp: x("fminp"),
                        forgn: x("forgn"),
                        forgp: x("forgp"),
                        fnh3n: x("fnh3n"),
                        surf_frac: row_real(project.chem_app.as_ref(), &o.op_plant, "surf_frac", 0.0),
                        kg: o.op3,
                    })
                }
                "skip" => OpKind::Skip,
                other => return Err(format!("schedule {}: operation {other}", s.name)),
            };
            jdays.push(jdt(ndays, o.day, o.mon));
            ops.push(Op { kind, mon: o.mon, day: o.day, op_char: o.op_char.clone() });
        }
        // plant_init: the first operation is the one before the first of the starting rotation
        // year on or after the start day (the loop leaves its index one past the one wanted).
        let mut cur_op = 1usize;
        if let Some(&first) = jdays.first()
            && first > 0
        {
            let n = ops.len();
            let mut irot = 1;
            let mut jday_prev = first;
            let mut iop = n + 1;
            for (k, op) in ops.iter().enumerate() {
                if irot == rot_yr_ini && day_start <= jdays[k] {
                    iop = k + 1;
                    break;
                }
                if op.kind == OpKind::Skip || jdays[k] < jday_prev {
                    irot += 1;
                }
                jday_prev = jdays[k];
            }
            cur_op = (iop - 1).min(n).max(1);
        }
        Ok(Mgt { ops, cur_op: cur_op - 1, yr_skip: false })
    }
}

impl super::Hru {
    /// SWAT+ `mgt_operatn`: the operations dated today, in order.
    pub(super) fn mgt_operatn(&mut self, mo: i32, day_mo: i32, b: &HruBasin) {
        let Some(m) = self.mgt.as_ref() else { return };
        let n = m.ops.len();
        if n < 1 {
            return;
        }
        let mut iter_guard = 0;
        loop {
            let Some(m) = self.mgt.as_ref() else { return };
            let op = &m.ops[m.cur_op];
            if !(op.mon == mo && op.day == day_mo) {
                break;
            }
            self.mgt_sched(b);
            if n == 1 {
                break;
            }
            if self.mgt.as_ref().is_some_and(|m| m.yr_skip) {
                break;
            }
            iter_guard += 1;
            if iter_guard >= n {
                break;
            }
        }
    }

    /// SWAT+ `mgt_sched`: the current operation, then the next one becomes current (a skip stays
    /// current until the year ends).
    fn mgt_sched(&mut self, b: &HruBasin) {
        let Some(m) = self.mgt.as_ref() else { return };
        let op = m.ops[m.cur_op].clone();
        match op.kind {
            OpKind::Plant => {
                let zmx = self.zmx;
                if let Some(pl) = self.plant.as_mut()
                    && fortran_eq(&op.op_char, &pl.cpnm)
                    && !pl.gro
                {
                    pl.gro = true;
                    pl.idorm = false;
                    self.zmx = plantop(pl, zmx);
                }
            }
            OpKind::HarvKill { grain, eff, bm_min } => {
                if let Some(mut pl) = self.plant.take() {
                    if pl.gro {
                        let biomass = pl.tot.m;
                        if fortran_eq(&op.op_char, &pl.cpnm) || fortran_eq(&op.op_char, "all") {
                            if biomass > bm_min && grain {
                                self.harvgrain(&mut pl, eff);
                            }
                            self.killop(&mut pl);
                        }
                        pl.reset_stress();
                        pl.phuacc = 0.0;
                        self.phubase = 0.0;
                    }
                    self.plant = Some(pl);
                }
            }
            OpKind::Till { effmix, deptil } => {
                if let Some(cb) = b.carbon.as_ref() {
                    self.tillmix_cswat1(effmix, deptil, cb);
                }
            }
            OpKind::Irr { amt_mm, eff, surq } => {
                self.irr_applied = amt_mm * eff * (1.0 - surq);
                self.irr_runoff = amt_mm * surq;
            }
            OpKind::Fert(f) => {
                let rtof = b.carbon.as_ref().map(|c| c.rtof).unwrap_or(0.5);
                self.pl_fert(&f, b.cswat, rtof);
            }
            OpKind::Skip => {
                if let Some(m) = self.mgt.as_mut() {
                    m.yr_skip = true;
                }
            }
        }
        if let Some(m) = self.mgt.as_mut() {
            if op.kind != OpKind::Skip {
                m.cur_op += 1;
            }
            if m.cur_op >= m.ops.len() {
                m.cur_op = 0;
            }
        }
    }

    /// The yearly reset (`time_control`): operations resume, past a skip they stopped at.
    pub(super) fn mgt_new_year(&mut self) {
        if let Some(m) = self.mgt.as_mut() {
            m.yr_skip = false;
            if m.ops.is_empty() {
                return;
            }
            if m.ops[m.cur_op].kind == OpKind::Skip {
                m.cur_op += 1;
            }
            if m.cur_op >= m.ops.len() {
                m.cur_op = 0;
            }
        }
    }

    /// SWAT+ `mgt_harvgrain`: the seed leaves (the share `eff` of it is the yield); what the harvest misses goes to the active humus of
    /// the top layer.
    fn harvgrain(&mut self, pl: &mut Plant, eff: f32) {
        pl.harv_idx = pl.harv_idx.max(pl.db.wsyf);
        pl.tot = pl.tot - pl.seed;
        pl.ab_gr = pl.ab_gr - pl.seed;
        self.pl_yield = eff * pl.seed.m;
        let harveff1 = 1.0 - eff;
        if let Some(l) = self.layers.first_mut() {
            l.hact = OrgMass::scale(harveff1, pl.seed) + l.hact;
        }
        pl.seed = OrgMass::default();
    }

    /// SWAT+ `mgt_killop`: the plant above ground goes to the surface residue, its roots to the
    /// layers' residue, and it stops growing. The community totals lose it for the rest of the day.
    fn killop(&mut self, pl: &mut Plant) {
        let layers: Vec<(f32, f32)> = self.layers.iter().map(|l| (l.d, l.thick)).collect();
        pl.rootfr(&layers);
        pl.rsd = pl.rsd + pl.ab_gr;
        self.rsd_tot = OrgMass::default() + pl.rsd;
        for (ly, l) in self.layers.iter_mut().enumerate() {
            l.rsd = l.rsd + OrgMass::scale(pl.rtfr[ly], pl.root);
        }
        self.tot_com = self.tot_com - pl.tot;
        self.ab_gr_com = self.ab_gr_com - pl.ab_gr;
        pl.tot = OrgMass::default();
        pl.ab_gr = OrgMass::default();
        pl.leaf = OrgMass::default();
        pl.stem = OrgMass::default();
        pl.seed = OrgMass::default();
        pl.root = OrgMass::default();
        // plg_zero
        pl.cht = 0.0;
        pl.lai = 0.0;
        pl.plet = 0.0;
        pl.plpet = 0.0;
        pl.laimxfr = 0.0;
        pl.hi_adj = 0.0;
        pl.hi_prev = 0.0;
        pl.olai = 0.0;
        pl.dphu = 0.0;
        pl.d_senes = 0.0;
        pl.root_dep = 0.0;
        pl.root_frac = 0.0;
        for v in pl.rtfr.iter_mut() {
            *v = 0.0;
        }
        pl.n_fr = 0.0;
        pl.p_fr = 0.0;
        pl.reset_stress();
        pl.gro = false;
        pl.idorm = false;
        pl.phuacc = 0.0;
        pl.curyr_mat = 1;
    }

    /// SWAT+ `pl_fert`: mineral N and P into the top two layers (the surface share in the first),
    /// organic N and P to the residue and humus, with the carbon model as organic matter.
    fn pl_fert(&mut self, f: &Fert, cswat: i32, rtof: f32) {
        let kg = f.kg;
        let organic = cswat == 2 && (f.forgn > 0.0 || f.forgp > 0.0);
        let mut org = OrgMass::default();
        let mut meta_fr = 0.0_f32;
        if organic {
            org = OrgMass { m: kg, c: f.forgn * kg * 10.0, n: f.forgn * kg, p: f.forgp * kg };
            let c_n_rto = 0.175 * org.c / (f.fminn + f.forgn + 1.0e-5);
            meta_fr = 0.85 - 0.018 * c_n_rto;
        }
        if meta_fr < 0.01 {
            meta_fr = 0.01;
        } else if meta_fr > 0.7 {
            meta_fr = 0.7;
        }
        for l in 0..2.min(self.layers.len()) {
            let fr_ly = if l == 0 { f.surf_frac } else { 1.0 - f.surf_frac };
            let ly = &mut self.layers[l];
            ly.no3 += fr_ly * kg * (1.0 - f.fnh3n) * f.fminn;
            ly.nh4 += fr_ly * kg * f.fnh3n * f.fminn;
            ly.lab += fr_ly * kg * f.fminp;
            ly.cp.tot.n += rtof * fr_ly * kg * f.forgn;
            ly.cp.tot.p += rtof * fr_ly * kg * f.forgp;
            if cswat == 0 {
                ly.rsd.n += rtof * fr_ly * kg * f.forgn;
                ly.rsd.p += rtof * fr_ly * kg * f.forgp;
                ly.hact.n += (1.0 - rtof) * fr_ly * kg * f.forgn;
                // The active humus P is set from the stable humus's.
                ly.hact.p = ly.hsta.p + (1.0 - rtof) * fr_ly * kg * f.forgp;
            }
            if organic {
                let pool_fr = (1.0 - rtof) * fr_ly;
                ly.cp.tot = ly.cp.tot + OrgMass::scale(pool_fr, org);
                ly.cp.hs = ly.cp.hs + OrgMass::scale(pool_fr, org);
                let pool_fr = (1.0 - rtof) * meta_fr * fr_ly;
                ly.cp.meta = ly.cp.meta + OrgMass::scale(pool_fr, org);
                let pool_fr = (1.0 - rtof) * (1.0 - meta_fr) * fr_ly;
                ly.cp.str = ly.cp.str + OrgMass::scale(pool_fr, org);
                ly.cp.lig = ly.cp.lig + OrgMass::scale(0.175 * pool_fr, org);
            }
        }
        self.fertn += kg * (f.fminn + f.forgn);
        self.fertp += kg * (f.fminp + f.forgp);
    }
}

/// SWAT+ `mgt_plantop`: the plant starts from nothing. Returns the soil's rooting depth, capped
/// by the plant's.
fn plantop(pl: &mut Plant, zmx: f32) -> f32 {
    pl.phuacc = 0.0;
    pl.tot = OrgMass::default();
    pl.ab_gr = OrgMass::default();
    pl.leaf = OrgMass::default();
    pl.stem = OrgMass::default();
    pl.seed = OrgMass::default();
    pl.root = OrgMass::default();
    pl.plet = 0.0;
    pl.plpet = 0.0;
    pl.laimxfr = 0.0;
    pl.hi_adj = 0.0;
    pl.olai = 0.0;
    pl.dphu = 0.0;
    pl.reset_stress();
    let plt_zmx = 1000.0 * pl.db.rdmx;
    zmx.min(plt_zmx)
}
