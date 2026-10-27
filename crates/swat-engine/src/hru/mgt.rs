//! Scheduled management (`mgt_operatn`, `mgt_sched`): planting, harvest and kill, tillage,
//! irrigation and fertilizer on their dates, and the skip to the next year.
//!
//! So far for dated operations of these kinds, and decision tables (`conditions`, `actions`)
//! that irrigate on water stress, heat units, leaf area and the irrigation so far, or plant,
//! harvest and kill on heat units, soil water, the day and the rotation year (the generic
//! `pl_hv_summer1`, `pl_hv_winter1` and `pl_hv_summer2` tables among them).

use swat_model::table::{Table, fortran_eq};
use swat_model::{MgtSchedule, Project};

use super::HruBasin;
use super::plant::{OrgMass, Plant};
use crate::fortran::aunif;

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
    /// `manu`: manure of `manure_om.frt` (the row numbered as the `manure_db.frt` one named).
    Manure(Manure),
    Skip,
}

/// A manure application: the `manure_om.frt` fractions, the surface share and the amount
/// (kg/ha, dry).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Manure {
    pub frac_water: f32,
    pub fminn: f32,
    pub fminp: f32,
    pub forgn: f32,
    pub forgp: f32,
    pub fnh3n: f32,
    pub surf_frac: f32,
    pub kg: f32,
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
    /// The schedule's decision tables, checked every day (`sched%num_autos`).
    pub dtbls: Vec<Dtbl>,
    /// The crops of a generic plant-and-harvest table (`sched%auto_crop`).
    pub auto_crops: Vec<String>,
    /// The rotation year (`pcom%rot_yr`), which decision tables test and reset.
    pub rot_yr: i32,
}

/// A condition variable of a decision table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CondVar {
    /// Mean water stress of the growing plants (1 without any).
    WStress,
    /// Irrigation applied so far (`hru%irr_yr`: never reset, so over the whole run).
    IrrYear,
    /// Heat units of the first plant with any, else of plant 1.
    PhuPlant,
    /// The community's leaf area.
    LeafArea,
    /// Base-zero heat units of the HRU's station this year.
    PhuBase0,
    /// Soil water against a share of the wilting point (0), field capacity or porosity
    /// (`lim_var`), the share applied with `lim_op`.
    SoilWater {
        targ: SwTarget,
        op: u8,
    },
    /// The day of the year (an integer test).
    Jday,
    /// The rotation year (an integer test).
    YearRot,
    /// The month (an integer test).
    Month,
    /// Days since the last planting, harvest, kill, irrigation (integer tests).
    DaysPlant,
    DaysHarv,
    DaysKill,
    DaysIrr,
    /// The station's precipitation today.
    PrecipCur,
    /// A paddy's weir height and ponded water depth (m; the limit is in mm).
    WeirH,
    WetDepth,
    /// `prob_unif`: one day drawn at random in a window of days (from `first`, to `last`), on
    /// the window's first day, with the probability `frac_app` of the table. Only its `=`
    /// alternatives are tested: they fail on any other day.
    ProbUnif {
        first: i32,
    },
}

/// The soil water a `soil_water` condition is measured against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwTarget {
    Wp,
    Fc,
    Ul,
}

/// An action of a decision table.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `irrigate` from an unlimited source: `amt` mm with the `irr.ops` efficiency and runoff
    /// share, at most `max_yr` times a year. Named `ponding`, a paddy's: up to `amt` mm of
    /// ponded water (and `max_yr`, as mm, is the paddy's threshold depth).
    Irrigate { amt: f32, eff: f32, surq: f32, max_yr: i32, ponding: bool, const2: f32 },
    /// `till` with a `tillage.til` row.
    Till { effmix: f32, deptil: f32, max_yr: i32 },
    /// `fertilize` with a `fertilizer.frt` row and `chem_app.ops` surface share.
    Fertilize { fert: Fert, max_yr: i32 },
    /// `weir_height` of a paddy (`wet`): mm.
    WeirHeight { h_mm: f32 },
    /// `puddle`: a `puddle.ops` row (the top layer's conductivity, the water's sediment, ppm)
    /// and the tillage named as the action.
    Puddle { wet_hc: f32, sed: f32, effmix: f32, deptil: f32, max_yr: i32 },
    /// `plant` the community's plant named `crop` if it isn't growing (no `mgt_plantop`).
    Plant { crop: String },
    /// `harvest_kill` the plants named `crop` (or `all`), growing or not, at most `max_yr` times
    /// a year: a grain harvest above `bm_min` of biomass, with the `harv.ops` efficiency.
    HarvKill { crop: String, grain: bool, eff: f32, bm_min: f32, max_yr: i32 },
    /// `rot_reset`: the rotation year becomes `yr`.
    RotReset { yr: i32 },
}

/// A decision table as an HRU uses it: conditions with each alternative's operator ("-" for
/// none), actions with their outcome by alternative, and the count of each action this year
/// (`pcom%dtbl%num_actions`, starting at 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Dtbl {
    /// The crops come from the schedule: 1 for the first crop, 2 for the rotation year's
    /// (`pl_hv_summer1` / `pl_hv_winter1`, `pl_hv_summer2`), 0 for the table's own option.
    pub generic: u8,
    pub alts: usize,
    pub conds: Vec<(CondVar, f32, Vec<String>)>,
    pub acts: Vec<(Action, Vec<bool>)>,
    pub num_actions: Vec<i32>,
    /// `prob_unif`: the window's last day and the probability (`frac_app`), and the day drawn
    /// (`pcom%dtbl%apply_day`).
    pub frac_app: f32,
    pub apply_day: i32,
}

impl Dtbl {
    fn new(project: &Project, t: &swat_model::DecisionTable, auto_name: &str) -> Result<Dtbl, String> {
        let generic = match auto_name {
            "pl_hv_summer1" | "pl_hv_winter1" => 1,
            "pl_hv_summer2" => 2,
            _ => 0,
        };
        let mut conds = Vec::new();
        let mut frac_app = 0.0_f32;
        for (c, alt) in t.conds.iter().zip(&t.alt) {
            let v = match c.var.as_str() {
                "month" => CondVar::Month,
                "days_plant" => CondVar::DaysPlant,
                "days_harv" => CondVar::DaysHarv,
                "days_kill" => CondVar::DaysKill,
                "days_irr" => CondVar::DaysIrr,
                "precip_cur" => {
                    if c.ob != "hru" {
                        return Err(format!("decision table {}: precip_cur of {}", t.name, c.ob));
                    }
                    CondVar::PrecipCur
                }
                "weirh" => CondVar::WeirH,
                "wet_depth" => CondVar::WetDepth,
                "prob_unif" => {
                    // dtbl_lum_read reads the object column again, as the probability.
                    frac_app =
                        c.ob.trim().parse::<f32>().map_err(|_| format!("decision table {}: prob_unif probability {}", t.name, c.ob))?;
                    CondVar::ProbUnif { first: c.ob_num }
                }
                "w_stress" => CondVar::WStress,
                "irr_year" => CondVar::IrrYear,
                "phu_plant" => CondVar::PhuPlant,
                "leaf_area" => CondVar::LeafArea,
                "phu_base0" => {
                    if c.ob != "hru" {
                        return Err(format!("decision table {}: phu_base0 of {}", t.name, c.ob));
                    }
                    CondVar::PhuBase0
                }
                "soil_water" => {
                    let targ = match c.lim_var.as_str() {
                        "wp" => SwTarget::Wp,
                        "fc" => SwTarget::Fc,
                        "ul" => SwTarget::Ul,
                        other => return Err(format!("decision table {}: soil water against {other}", t.name)),
                    };
                    let op = match c.lim_op.as_str() {
                        "*" | "+" | "-" | "/" => c.lim_op.as_bytes()[0],
                        other => return Err(format!("decision table {}: soil water target operator {other}", t.name)),
                    };
                    CondVar::SoilWater { targ, op }
                }
                "jday" => CondVar::Jday,
                "year_rot" => CondVar::YearRot,
                other => return Err(format!("decision table {}: condition {other}", t.name)),
            };
            if c.ob_num != 0 && !matches!(v, CondVar::ProbUnif { .. }) {
                return Err(format!("decision table {}: condition on object {}", t.name, c.ob_num));
            }
            for a in alt {
                if !matches!(a.as_str(), "-" | "<" | ">" | "<=" | ">=" | "=" | "/=") {
                    return Err(format!("decision table {}: alternative {a}", t.name));
                }
            }
            conds.push((v, c.lim_const, alt.clone()));
        }
        let mut acts = Vec::new();
        for (a, out) in t.acts.iter().zip(&t.outcomes) {
            let act = match a.typ.as_str() {
                "irrigate" => {
                    if matches!(a.ob.as_str(), "aqu" | "cha" | "sdc" | "res") {
                        return Err(format!("decision table {}: irrigation from {} ({})", t.name, a.ob, a.name));
                    }
                    let ir = project.irr_ops.as_ref();
                    Action::Irrigate {
                        amt: a.const_,
                        eff: row_real(ir, &a.option, "eff", 0.0),
                        surq: row_real(ir, &a.option, "surq", 0.0),
                        max_yr: a.const2 as i32,
                        ponding: a.name == "ponding",
                        const2: a.const2,
                    }
                }
                "plant" | "harvest_kill" | "rot_reset" | "till" | "fertilize" | "weir_height" | "puddle" if a.ob_num != 0 => {
                    return Err(format!("decision table {}: {} on object {}", t.name, a.typ, a.ob_num));
                }
                "plant" => {
                    // A transplant (a `transplant.ops` row) isn't ported yet.
                    if project.transplant_ops.as_ref().is_some_and(|t| t.find(&a.file_pointer).is_some()) {
                        return Err(format!("decision table {}: transplant {}", t.name, a.file_pointer));
                    }
                    Action::Plant { crop: a.option.clone() }
                }
                "till" => {
                    let tl = project.tillage.as_ref();
                    Action::Till {
                        effmix: row_real(tl, &a.option, "effmix", 0.0),
                        deptil: row_real(tl, &a.option, "deptil", 0.0),
                        max_yr: a.const2 as i32,
                    }
                }
                "fertilize" => {
                    let ft = project.fertilizer.as_ref();
                    let x = |f: &str| row_real(ft, &a.option, f, 0.0);
                    Action::Fertilize {
                        fert: Fert {
                            fminn: x("fminn"),
                            fminp: x("fminp"),
                            forgn: x("forgn"),
                            forgp: x("forgp"),
                            fnh3n: x("fnh3n"),
                            surf_frac: row_real(project.chem_app.as_ref(), &a.file_pointer, "surf_frac", 0.0),
                            kg: a.const_,
                        },
                        max_yr: a.const2 as i32,
                    }
                }
                "weir_height" => {
                    if a.option != "wet" {
                        return Err(format!("decision table {}: weir height of a reservoir", t.name));
                    }
                    Action::WeirHeight { h_mm: a.const_ }
                }
                "puddle" => {
                    // The last row of that name (no exit in the loop); none would leave the
                    // previous puddling's.
                    let pt = project.puddle_ops.as_ref();
                    let row =
                        pt.and_then(|pt| (0..pt.rows.len()).rev().find(|&r| pt.text(r, "name").is_some_and(|n| fortran_eq(n, &a.option))));
                    let (Some(pt), Some(r)) = (pt, row) else {
                        return Err(format!("decision table {}: puddling {} not in puddle.ops", t.name, a.option));
                    };
                    let tl = project.tillage.as_ref();
                    if tl.and_then(|tl| tl.find(&a.name)).is_none() {
                        return Err(format!("decision table {}: puddling tillage {} not in tillage.til", t.name, a.name));
                    }
                    Action::Puddle {
                        wet_hc: pt.real(r, "wet_hc").unwrap_or(0.0),
                        sed: pt.real(r, "sed").unwrap_or(0.0),
                        effmix: row_real(tl, &a.name, "effmix", 0.0),
                        deptil: row_real(tl, &a.name, "deptil", 0.0),
                        max_yr: a.const2 as i32,
                    }
                }
                "harvest_kill" => {
                    let h = project.harv_ops.as_ref();
                    let typ = h.and_then(|h| h.find(&a.file_pointer).and_then(|r| h.text(r, "typ"))).unwrap_or("");
                    if matches!(typ, "biomass" | "residue" | "tuber" | "peanuts") {
                        return Err(format!("decision table {}: {typ} harvest", t.name));
                    }
                    Action::HarvKill {
                        crop: a.option.clone(),
                        grain: matches!(typ, "grain" | "stripper" | "picker"),
                        eff: row_real(h, &a.file_pointer, "eff", 0.0),
                        bm_min: row_real(h, &a.file_pointer, "bm_min", 0.0),
                        max_yr: a.const2 as i32,
                    }
                }
                "rot_reset" => Action::RotReset { yr: a.const_.max(1.0) as i32 },
                other => return Err(format!("decision table {}: action {other}", t.name)),
            };
            acts.push((act, out.iter().map(|o| o == "y").collect()));
        }
        let n = acts.len();
        Ok(Dtbl { generic, alts: t.alts, conds, acts, num_actions: vec![1; n], frac_app, apply_day: 0 })
    }
}

fn row_real(t: Option<&Table>, name: &str, field: &str, default: f32) -> f32 {
    match t.and_then(|t| t.find(name).map(|r| (t, r))) {
        Some((t, r)) => t.real(r, field).unwrap_or(default),
        None => default,
    }
}

/// `cond_real` and `cond_integer`: alternatives still hit whose operator fails for `cur`
/// against `tbl` are no longer hit.
pub(super) fn cond_test<T: PartialOrd>(hit: &mut [bool], alt: &[String], cur: T, tbl: T) {
    for (h, op) in hit.iter_mut().zip(alt) {
        if op == "-" || !*h {
            continue;
        }
        let fails = match op.as_str() {
            "<" => cur >= tbl,
            ">" => cur <= tbl,
            "<=" => cur > tbl,
            ">=" => cur < tbl,
            "=" => cur != tbl,
            "/=" => cur == tbl,
            _ => false,
        };
        if fails {
            *h = false;
        }
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
        // proc_cond: each decision table by name, the last of that name.
        let mut dtbls = Vec::new();
        for a in &s.autos {
            let t = project
                .lum_dtl
                .iter()
                .rfind(|t| fortran_eq(&t.name, a))
                .ok_or_else(|| format!("schedule {}: decision table {a} not in lum.dtl", s.name))?;
            dtbls.push(Dtbl::new(project, t, a)?);
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
                "manu" => {
                    if project.codes.cswat == 2 {
                        return Err(format!("schedule {}: manure with carbon model 2", s.name));
                    }
                    // read_mgtops finds the manure_db.frt row, pl_manure takes the manure_om.frt
                    // row of that number (row 0, all zeros, when it isn't found).
                    let idb = project.manure_db.as_ref().and_then(|t| t.find(&o.op_char)).map_or(0, |r| r + 1);
                    let om = project.manure_om.as_ref();
                    let x = |f: &str| match om {
                        Some(t) if idb >= 1 && idb <= t.rows.len() => t.real(idb - 1, f).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    OpKind::Manure(Manure {
                        frac_water: x("frac_water"),
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
        if dtbls.iter().any(|t| t.generic > 0 && s.auto_crops.len() < t.generic as usize) {
            return Err(format!("schedule {}: generic decision table without its crops", s.name));
        }
        Ok(Mgt { ops, cur_op: cur_op - 1, yr_skip: false, dtbls, auto_crops: s.auto_crops.clone(), rot_yr: rot_yr_ini })
    }
}

impl super::Hru {
    /// The decision tables of the HRU's schedule (`hru_control`: `conditions`, then `actions`),
    /// before the day's stresses are reset, with the station's base-zero heat units `phubase0`.
    /// At the end of the year the action counts restart and the rotation moves on a year.
    pub(super) fn dtbl_day(&mut self, c: &super::DayCtx, phubase0: f32, b: &HruBasin, saved: &mut super::plant::Saved) {
        let Some(mut m) = self.mgt.take() else { return };
        for t in m.dtbls.iter_mut() {
            let mut hit = vec![true; t.alts];
            for (var, lim, alt) in &t.conds {
                if *var == CondVar::PhuPlant && self.plants.is_empty() {
                    hit.iter_mut().for_each(|h| *h = false);
                    continue;
                }
                let mut targ = *lim;
                let cur = match var {
                    CondVar::WStress => {
                        let (mut n, mut sum) = (0, 0.0_f32);
                        for pl in &self.plants {
                            if pl.gro {
                                n += 1;
                                sum += pl.strsw;
                            }
                        }
                        if n > 0 { sum / n as f32 } else { 1.0 }
                    }
                    CondVar::IrrYear => self.irr_yr,
                    CondVar::PhuPlant => self.plants.iter().find(|p| p.phuacc > 1.0e-6).unwrap_or(&self.plants[0]).phuacc,
                    CondVar::LeafArea => self.lai_sum,
                    CondVar::PhuBase0 => phubase0,
                    CondVar::SoilWater { targ: tv, op } => {
                        let v = match tv {
                            SwTarget::Wp => 0.0,
                            SwTarget::Fc => self.sumfc,
                            SwTarget::Ul => self.sumul,
                        };
                        targ = match op {
                            b'*' => v * *lim,
                            b'+' => v + *lim,
                            b'-' => v - *lim,
                            _ => v / *lim,
                        };
                        self.sw
                    }
                    CondVar::PrecipCur => c.station_precip,
                    CondVar::WeirH | CondVar::WetDepth => {
                        targ = *lim / 1000.0;
                        let wt = self.wet.as_ref();
                        if *var == CondVar::WeirH { wt.map_or(0.0, |w| w.weir_hgt) } else { wt.map_or(0.0, |w| w.depth) }
                    }
                    // cond_integer
                    CondVar::Jday
                    | CondVar::YearRot
                    | CondVar::Month
                    | CondVar::DaysPlant
                    | CondVar::DaysHarv
                    | CondVar::DaysKill
                    | CondVar::DaysIrr => {
                        let v = match var {
                            CondVar::Jday => c.day,
                            CondVar::YearRot => m.rot_yr,
                            CondVar::Month => c.mo,
                            CondVar::DaysPlant => self.days_plant,
                            CondVar::DaysHarv => self.days_harv,
                            CondVar::DaysKill => self.days_kill,
                            _ => self.days_irr,
                        };
                        cond_test(&mut hit, alt, v, *lim as i32);
                        continue;
                    }
                    CondVar::ProbUnif { first } => {
                        let first = *first;
                        if c.day == first {
                            let seed = &mut saved.rndseed_cond;
                            if *lim > first as f32 {
                                // The window is within the year.
                                let ran_num = aunif(seed);
                                if ran_num < t.frac_app {
                                    let days_tot = (*lim - first as f32 + 1.0) as i32;
                                    let ran_num = aunif(seed);
                                    t.apply_day = (ran_num * days_tot as f32) as i32 + first + 1;
                                } else {
                                    t.apply_day = 0;
                                }
                            } else {
                                // It runs over the year's end.
                                let ran_num = aunif(seed);
                                if ran_num < t.frac_app {
                                    if c.day as f32 >= *lim {
                                        let days_tot = c.day_end_yr - first + 1;
                                        let ran_num = aunif(seed);
                                        t.apply_day = (ran_num * days_tot as f32) as i32 + first + 1;
                                    } else {
                                        let days_tot = *lim as i32;
                                        let ran_num = aunif(seed);
                                        t.apply_day = (ran_num * days_tot as f32) as i32 + 1;
                                    }
                                } else {
                                    t.apply_day = 0;
                                }
                            }
                        }
                        if c.day != t.apply_day {
                            for (h, op) in hit.iter_mut().zip(alt) {
                                if op == "=" {
                                    *h = false;
                                }
                            }
                        }
                        continue;
                    }
                };
                // cond_real
                cond_test(&mut hit, alt, cur, targ);
            }
            for (iac, (act, out)) in t.acts.iter().enumerate() {
                if !hit.iter().zip(out).any(|(h, o)| *h && *o) {
                    continue;
                }
                // A generic table's crop comes from the schedule.
                let crop = |own: &str| -> String {
                    match t.generic {
                        1 => m.auto_crops[0].clone(),
                        2 => m.auto_crops[(m.rot_yr.clamp(1, 2) - 1) as usize].clone(),
                        _ => own.to_string(),
                    }
                };
                match act {
                    Action::Irrigate { amt, eff, surq, max_yr, ponding, const2 } => {
                        if t.num_actions[iac] <= *max_yr {
                            if *ponding {
                                self.irr_hmax = *amt;
                                self.irr_hmin = *const2;
                                let depth = self.wet.as_ref().map_or(0.0, |w| w.depth);
                                self.irr_applied = (amt - depth * 1000.0).max(0.0) * eff * (1.0 - surq);
                                self.irr_runoff = (amt - depth * 1000.0).max(0.0) * surq;
                            } else {
                                self.irr_applied = amt * eff * (1.0 - surq);
                                self.irr_runoff = amt * surq;
                            }
                            self.irr_yr += self.irr_applied;
                            t.num_actions[iac] += 1;
                            self.days_irr = 1;
                        }
                    }
                    Action::Fertilize { fert, max_yr } => {
                        if t.num_actions[iac] <= *max_yr {
                            if self.wet.as_ref().is_some_and(|w| w.water.flo > 0.0) && fert.surf_frac == 1.0 {
                                self.pl_fert_wet(fert, b.cswat, b.fixes);
                            } else {
                                let rtof = b.carbon.as_ref().map(|c| c.rtof).unwrap_or(0.5);
                                self.pl_fert(fert, b.cswat, rtof);
                            }
                            t.num_actions[iac] += 1;
                        }
                    }
                    Action::Till { effmix, deptil, max_yr } => {
                        if t.num_actions[iac] <= *max_yr {
                            match b.carbon.as_ref() {
                                Some(cb) if b.cswat == 2 => self.tillmix_cswat1(*effmix, *deptil, cb),
                                _ => self.tillmix_cswat0(0.0, *effmix, *deptil),
                            }
                            t.num_actions[iac] += 1;
                        }
                    }
                    Action::WeirHeight { h_mm } => {
                        let area = self.area_ha;
                        if let Some(w) = self.wet.as_mut() {
                            w.set_weir(*h_mm, area);
                        }
                    }
                    Action::Puddle { wet_hc, sed, effmix, deptil, max_yr } => {
                        if t.num_actions[iac] <= *max_yr {
                            let mut wet_depth = 0.0;
                            if let Some(w) = self.wet.as_mut() {
                                if *wet_hc > 0.0 {
                                    w.wet_hc = *wet_hc;
                                }
                                w.water.sed = sed * w.water.flo / 1000000.0;
                                wet_depth = w.depth;
                            }
                            if wet_depth > 0.001 {
                                self.tillmix_wet(*effmix, *deptil, b.cswat, b.fixes);
                            } else {
                                match b.carbon.as_ref() {
                                    Some(cb) if b.cswat == 2 => self.tillmix_cswat1(*effmix, *deptil, cb),
                                    _ => self.tillmix_cswat0(0.0, *effmix, *deptil),
                                }
                            }
                            t.num_actions[iac] += 1;
                        }
                    }
                    Action::Plant { crop: own } => {
                        self.days_plant = 1;
                        let crop = crop(own);
                        for pl in self.plants.iter_mut() {
                            if fortran_eq(&crop, &pl.cpnm) && !pl.gro {
                                pl.gro = true;
                                pl.idorm = false;
                            }
                        }
                        t.num_actions[iac] += 1;
                    }
                    Action::HarvKill { crop: own, grain, eff, bm_min, max_yr } => {
                        if t.num_actions[iac] <= *max_yr {
                            self.days_harv = 1;
                            let crop = crop(own);
                            for i in 0..self.plants.len() {
                                let mut pl = std::mem::take(&mut self.plants[i]);
                                if fortran_eq(&crop, &pl.cpnm) || fortran_eq(&crop, "all") {
                                    if pl.tot.m > *bm_min && *grain {
                                        self.harvgrain(&mut pl, *eff);
                                    }
                                    self.killop(&mut pl, i);
                                    // Summed even when the harvest didn't happen (the day's
                                    // yield so far, usually none).
                                    pl.yield_tot = pl.yield_tot + self.pl_yield;
                                    pl.harv_num += 1;
                                    self.harvests.push((i, self.pl_yield.m));
                                }
                                self.plants[i] = pl;
                            }
                            t.num_actions[iac] += 1;
                        }
                    }
                    Action::RotReset { yr } => m.rot_yr = *yr,
                }
            }
            if c.end_yr {
                t.num_actions.iter_mut().for_each(|n| *n = 1);
            }
        }
        if c.end_yr && !m.dtbls.is_empty() {
            m.rot_yr += 1;
        }
        self.mgt = Some(m);
    }

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
                self.days_plant = 1;
                for i in 0..self.plants.len() {
                    let zmx = self.zmx;
                    let pl = &mut self.plants[i];
                    if fortran_eq(&op.op_char, &pl.cpnm) && !pl.gro {
                        pl.gro = true;
                        pl.idorm = false;
                        self.zmx = plantop(pl, zmx);
                    }
                }
            }
            OpKind::HarvKill { grain, eff, bm_min } => {
                for i in 0..self.plants.len() {
                    let mut pl = std::mem::take(&mut self.plants[i]);
                    if pl.gro {
                        let biomass = pl.tot.m;
                        if fortran_eq(&op.op_char, &pl.cpnm) || fortran_eq(&op.op_char, "all") {
                            self.days_harv = 1;
                            self.days_kill = 1;
                            if biomass > bm_min && grain {
                                self.harvgrain(&mut pl, eff);
                            }
                            self.killop(&mut pl, i);
                            pl.yield_tot = pl.yield_tot + self.pl_yield;
                            pl.harv_num += 1;
                            self.harvests.push((i, self.pl_yield.m));
                        }
                        pl.reset_stress();
                        pl.phuacc = 0.0;
                        self.phubase = 0.0;
                    }
                    self.plants[i] = pl;
                }
            }
            OpKind::Till { effmix, deptil } => {
                if let Some(cb) = b.carbon.as_ref() {
                    self.tillmix_cswat1(effmix, deptil, cb);
                }
            }
            OpKind::Irr { amt_mm, eff, surq } => {
                self.days_irr = 1;
                self.irr_applied = amt_mm * eff * (1.0 - surq);
                self.irr_runoff = amt_mm * surq;
                self.irr_yr += self.irr_applied;
            }
            OpKind::Fert(f) => {
                let rtof = b.carbon.as_ref().map(|c| c.rtof).unwrap_or(0.5);
                self.pl_fert(&f, b.cswat, rtof);
            }
            OpKind::Manure(mn) => {
                let rtof = b.carbon.as_ref().map(|c| c.rtof).unwrap_or(0.5);
                self.pl_manure(&mn, rtof);
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
        self.pl_yield = OrgMass::scale(eff, pl.seed);
        let harveff1 = 1.0 - eff;
        if let Some(l) = self.layers.first_mut() {
            l.hact = OrgMass::scale(harveff1, pl.seed) + l.hact;
        }
        pl.seed = OrgMass::default();
    }

    /// SWAT+ `mgt_killop`: the plant above ground goes to the surface residue, its roots to the
    /// layers' residue, and it stops growing. The community totals lose it for the rest of the day.
    /// `pl` is plant `me` of the community, taken out of it.
    fn killop(&mut self, pl: &mut Plant, me: usize) {
        let layers: Vec<(f32, f32)> = self.layers.iter().map(|l| (l.d, l.thick)).collect();
        pl.rootfr(&layers);
        pl.rsd = pl.rsd + pl.ab_gr;
        self.rsd_tot = OrgMass::default();
        for j in 0..self.plants.len() {
            self.rsd_tot = self.rsd_tot + if j == me { pl.rsd } else { self.plants[j].rsd };
        }
        for ly in 0..pl.soil_rsd.len() {
            pl.soil_rsd[ly] = pl.soil_rsd[ly] + OrgMass::scale(pl.rtfr[ly], pl.root);
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
                // Plant 1's residue (SWAT+ would fail without plants).
                if let Some(pl) = self.plants.first_mut() {
                    pl.soil_rsd[l].n += rtof * fr_ly * kg * f.forgn;
                    pl.soil_rsd[l].p += rtof * fr_ly * kg * f.forgp;
                }
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

impl super::Hru {
    /// SWAT+ `pl_fert_wet`: fertilizer into a paddy's water. With the original carbon model it
    /// goes nowhere (SWAT+ adds it to the water only with carbon model 2): only the totals
    /// count it. Corrected (`fixes`), it goes into the water with either model.
    fn pl_fert_wet(&mut self, f: &Fert, cswat: i32, fixes: bool) {
        let kg = f.kg;
        if (cswat == 2 || fixes)
            && let Some(w) = self.wet.as_mut()
        {
            let b = &mut w.water;
            b.no3 += kg * (1.0 - f.fnh3n) * f.fminn;
            b.nh3 += kg * f.fnh3n * f.fminn;
            b.solp += kg * f.fminp;
            b.orgn += kg * f.forgn;
            b.sedp += kg * f.forgp;
        }
        self.fertn += kg * (f.fminn + f.forgn);
        self.fertp += kg * (f.fminp + f.forgp);
    }

    /// SWAT+ `pl_manure` with the original carbon model: the manure's water into the top two
    /// layers' soil water, mineral N and P into them, organic N and P to their totals and humus.
    fn pl_manure(&mut self, mn: &Manure, rtof: f32) {
        let kg = mn.kg;
        let frac_solids = 1.0 - mn.frac_water;
        let liq_manure_kg = kg / frac_solids - kg;
        let liq_manure_mm = liq_manure_kg * 0.0001;
        for l in 0..2.min(self.layers.len()) {
            let fr_ly = if l == 0 { mn.surf_frac } else { 1.0 - mn.surf_frac };
            let fr_mass = fr_ly * kg;
            let ly = &mut self.layers[l];
            ly.st += fr_ly * liq_manure_mm;
            ly.no3 += fr_mass * (1.0 - mn.fnh3n) * mn.fminn;
            ly.cp.tot.n += rtof * fr_mass * mn.forgn;
            ly.hact.n += (1.0 - rtof) * fr_mass * mn.forgn;
            ly.cp.tot.p += rtof * fr_mass * mn.forgp;
            ly.hsta.p += (1.0 - rtof) * fr_mass * mn.forgp;
            ly.nh4 += fr_mass * mn.fnh3n * mn.fminn;
            ly.lab += fr_mass * mn.fminp;
        }
        self.fertn += kg * (mn.fminn + mn.forgn);
        self.fertp += kg * (mn.fminp + mn.forgp);
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
