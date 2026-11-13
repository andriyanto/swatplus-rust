//! The objects and the command loop (`hyd_connect`, `command`): routing units (`ru_control`)
//! and aquifers (`aqu_1d_control`) take what the HRUs pass on. Channels aren't routed yet: a
//! SWAT-DEG channel passes its inflow straight on (as one shorter than 1 m does in SWAT+).

use swat_model::{ObjectKind, Project};

use crate::fortran::rpow;
use crate::hyd::Hyd;

/// The order of the kinds in SWAT+'s object array (`sp_ob1`), with each kind's number of
/// hydrographs (`hd_tot`).
const KINDS: [(ObjectKind, usize); 12] = [
    (ObjectKind::Hru, 5),
    (ObjectKind::HruLte, 5),
    (ObjectKind::RoutingUnit, 5),
    (ObjectKind::Gwflow, 1),
    (ObjectKind::Aquifer, 2),
    (ObjectKind::Channel, 3),
    (ObjectKind::Reservoir, 2),
    (ObjectKind::Recall, 1),
    (ObjectKind::ExportCoefficient, 2),
    (ObjectKind::DeliveryRatio, 2),
    (ObjectKind::Outlet, 1),
    (ObjectKind::ChannelDeg, 3),
];

/// Where an object sends a share of one of its hydrographs.
#[derive(Clone, Debug)]
struct Out {
    /// Receiving object (index into the object array), or `None` for an object number of 0.
    to: Option<usize>,
    /// Its type code (`obtyp_out`) and the hydrograph and share sent.
    code: String,
    ihtyp: usize,
    frac: f32,
}

/// What an object receives: from which object, which hydrograph, and the share.
#[derive(Clone, Debug)]
struct In {
    from: usize,
    ihtyp: usize,
    frac: f32,
}

#[derive(Clone, Debug)]
struct Ob {
    kind: ObjectKind,
    name: String,
    gis_id: i32,
    area_ha: f32,
    /// Weather station (1-based).
    wst: usize,
    props: i32,
    outs: Vec<Out>,
    ins: Vec<In>,
    hd: Vec<Hyd>,
    hin: Hyd,
    /// The routing units it is an element of (`obj_subs`), and the number of elements of a
    /// routing unit (`dfn_tot`).
    subs: Vec<usize>,
    dfn_tot: usize,
    /// Sub-daily: the flow hydrograph by step of the days kept (`hyd_flo(day_max, step)`, flat),
    /// the current day (1-based, `day_cur`; 0 before the first) and the inflow by step (`tsin`).
    hyd_flo: Vec<f32>,
    /// Sub-daily with the curve number: the unit hydrograph of an HRU or routing unit
    /// (`uh(day_lag_mx, step)`, flat), from its time of concentration.
    uh: Vec<f32>,
    day_cur: usize,
    day_max: usize,
    tsin: Vec<f32>,
}

/// A routing unit's element: its object, share and delivery ratio.
#[derive(Clone, Debug)]
struct RuElem {
    ob: usize,
    hru: bool,
    frac: f32,
    dr: Hyd,
}

#[derive(Clone, Debug)]
struct Ru {
    ob: usize,
    da_km2: f32,
    elems: Vec<RuElem>,
    /// Time of concentration (h, `ru_tc`).
    tc: f32,
}

/// An aquifer's state (`aquifer_dynamic`), as `aquifer_day.txt` prints it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AquDay {
    pub flo: f32,
    pub dep_wt: f32,
    pub stor: f32,
    pub rchrg: f32,
    pub seep: f32,
    pub revap: f32,
    pub no3_st: f32,
    pub minp: f32,
    pub cbn: f32,
    pub orgn: f32,
    pub no3_rchg: f32,
    pub no3_loss: f32,
    pub no3_lat: f32,
    pub no3_seep: f32,
    pub flo_cha: f32,
    pub flo_res: f32,
    pub flo_ls: f32,
}

impl AquDay {
    /// The columns of `aquifer_day.txt`. The header has `orgn` and `orgp` where the values are
    /// the carbon and organic N (`aqu_hdr` against `aquifer_dynamic`).
    pub fn columns(&self) -> [(&'static str, f32); 17] {
        self.columns_as(false)
    }

    /// The columns, with the header labels corrected (`cbn`, `orgn`) when `fixes` is set.
    pub fn columns_as(&self, fixes: bool) -> [(&'static str, f32); 17] {
        let (l1, l2) = if fixes { ("cbn", "orgn") } else { ("orgn", "orgp") };
        [
            ("flo", self.flo),
            ("dep_wt", self.dep_wt),
            ("stor", self.stor),
            ("rchrg", self.rchrg),
            ("seep", self.seep),
            ("revap", self.revap),
            ("no3_st", self.no3_st),
            ("minp", self.minp),
            (l1, self.cbn),
            (l2, self.orgn),
            ("no3_rchg", self.no3_rchg),
            ("no3_loss", self.no3_loss),
            ("no3_lat", self.no3_lat),
            ("no3_seep", self.no3_seep),
            ("flo_cha", self.flo_cha),
            ("flo_res", self.flo_res),
            ("flo_ls", self.flo_ls),
        ]
    }
}

#[derive(Clone, Debug)]
struct Aqu {
    ob: usize,
    db: swat_model::AquiferDb,
    alpha_e: f32,
    nloss: f32,
    d: AquDay,
}

/// The objects past the HRUs, in SWAT+'s command order.
#[derive(Clone, Debug)]
pub struct Network {
    obs: Vec<Ob>,
    /// Objects in command order, without the HRUs (they run first, in their own command order:
    /// an HRU that receives another's water after it).
    order: Vec<usize>,
    hru_order: Vec<usize>,
    rus: Vec<Ru>,
    /// Routing unit and aquifer by object index.
    ru_of: Vec<Option<usize>>,
    aqu_of: Vec<Option<usize>>,
    aqus: Vec<Aqu>,
    /// Today's routing unit outputs (`ru_d`), by routing unit.
    pub ru_d: Vec<Hyd>,
    /// Correct SWAT+'s bugs instead of reproducing them.
    fixes: bool,
    /// SWAT-DEG channels (by object index), with their inflows' origins for the stream
    /// temperature, unless they aren't ported for this project (then they pass water on).
    channels: Option<crate::channel::Channels>,
    chan_of: Vec<Option<usize>>,
    chan_in: Vec<(Vec<bool>, Vec<usize>)>,
    /// Why the channels aren't routed, when they aren't.
    pub channel_skipped: Option<String>,
    /// Reservoirs and recall objects, by object index.
    pub reservoirs: Vec<crate::res::Reservoir>,
    pub recalls: Vec<crate::res::Recall>,
    /// Time steps a day and their length (minutes); the HRUs' times of concentration and their
    /// runoff by step leaving today (Green and Ampt).
    step: usize,
    /// Green and Ampt (`gampt` 1): the runoff by step; otherwise unit hydrographs.
    ga: bool,
    dtm: f32,
    tconc: Vec<f32>,
    hhsurfq: Vec<Vec<f32>>,
    /// The HRUs are the first objects.
    n_hru: usize,
    /// How the objects are routed on the threads asked for last.
    plan: Option<Plan>,
}

/// A channel's printed day: number (1-based), GIS id, name, water body, storage, inflow,
/// outflow and the water temperature.
pub type ChannelDay<'a> = (usize, i32, &'a str, crate::channel::ChWat, Hyd, Hyd, Hyd, f32);

/// What the command loop needs from the day: the stations' weather, their generators' day
/// lengths and measured temperature gages, the HRUs' water balance (snowmelt, surface runoff,
/// lateral flow, water yield; mm) and the date (`time%yrs`, `time%day`).
pub struct NetDay<'a> {
    pub weat: Vec<&'a crate::climate::Weat>,
    pub daylth: Vec<f32>,
    pub tgage: Vec<Option<&'a crate::climate::Gage>>,
    pub hru_wb: &'a [[f32; 4]],
    pub yrs: i32,
    pub jday: i32,
    /// Calendar year, month, and whether the day is printed (`time%yrs > nyskip`).
    pub yrc: i32,
    pub mo: i32,
    pub day_mo: i32,
    pub print: bool,
}

impl Network {
    /// `hyd_connect`, `ru_read_elements`, `time_conc_init`, `dr_ru` and `aqu_initial`. The HRUs'
    /// times of concentration come in `tconc` (h) with their Manning's n and area (km2) in
    /// `ovn_km`. `Ok(None)` when the project has nothing past its HRUs; `Err` names what isn't
    /// ported.
    pub fn new(
        project: &Project,
        cal: &swat_model::cal::Calibration,
        stations: &[String],
        tconc: &[f32],
        ovn_km: &[(f32, f32)],
        fixes: bool,
    ) -> Result<Option<Network>, String> {
        let mut obs: Vec<Ob> = Vec::new();
        let mut first = std::collections::HashMap::new();
        for (kind, nhyds) in KINDS {
            first.insert(kind, obs.len());
            for o in project.objects_of(kind) {
                obs.push(Ob {
                    kind,
                    name: o.name.clone(),
                    gis_id: o.gis_id,
                    area_ha: o.area_ha,
                    wst: crate::fortran::search(stations, &o.weather_station).unwrap_or(1),
                    props: o.props,
                    outs: Vec::new(),
                    ins: Vec::new(),
                    hd: vec![Hyd::default(); nhyds],
                    hin: Hyd::default(),
                    subs: Vec::new(),
                    dfn_tot: 0,
                    hyd_flo: Vec::new(),
                    uh: Vec::new(),
                    day_cur: 0,
                    day_max: 1,
                    tsin: Vec::new(),
                });
            }
        }
        let n_hru = project.objects_of(ObjectKind::Hru).count();
        if obs.len() == n_hru {
            return Ok(None);
        }
        for (kind, _) in KINDS {
            let n = project.objects_of(kind).count();
            let ok = matches!(
                kind,
                ObjectKind::Hru
                    | ObjectKind::RoutingUnit
                    | ObjectKind::Aquifer
                    | ObjectKind::ChannelDeg
                    | ObjectKind::Outlet
                    | ObjectKind::Reservoir
                    | ObjectKind::Recall
            );
            if n > 0 && !ok {
                return Err(format!("{kind:?} objects"));
            }
        }
        if project.objects_of(ObjectKind::Aquifer2d).count() > 0 {
            return Err("2-D aquifers".into());
        }
        let kind_of_code = |code: &str| KINDS.iter().map(|(k, _)| *k).find(|k| k.code() == Some(code));

        // Outflows (`obj_out`, `ihtyp_out`).
        let mut idx = 0usize;
        for (kind, _) in KINDS {
            for o in project.objects_of(kind) {
                for f in &o.outflows {
                    let to_kind = kind_of_code(&f.obj_type).ok_or_else(|| format!("{}: outflow to {}", o.name, f.obj_type))?;
                    let to = if f.obj_id > 0 { Some(first[&to_kind] + f.obj_id as usize - 1) } else { None };
                    if to.is_some_and(|t| t >= obs.len()) {
                        return Err(format!("{}: outflow to {} {}", o.name, f.obj_type, f.obj_id));
                    }
                    let ihtyp = match f.hyd_type.as_str() {
                        "tot" => 1,
                        "rhg" => 2,
                        "sur" => 3,
                        "lat" => 4,
                        "til" => 5,
                        other => return Err(format!("{}: outflow of {other}", o.name)),
                    };
                    // Landscape routing: an HRU may send its water to another HRU. Water sent into a
                    // routing unit, or from other objects into an HRU, isn't ported.
                    if let Some(t) = to
                        && (obs[t].kind == ObjectKind::RoutingUnit || (obs[t].kind == ObjectKind::Hru && kind != ObjectKind::Hru))
                    {
                        return Err(format!("{}: water routed into {}", o.name, obs[t].name));
                    }
                    obs[idx].outs.push(Out { to, code: f.obj_type.clone(), ihtyp, frac: f.frac });
                    if let Some(t) = to {
                        let nh = obs[idx].hd.len();
                        if ihtyp > nh {
                            return Err(format!("{}: hydrograph {ihtyp} of {nh}", o.name));
                        }
                        let frac = f.frac;
                        obs[t].ins.push(In { from: idx, ihtyp, frac });
                    }
                }
                idx += 1;
            }
        }

        // Routing units: their elements, times of concentration and delivery ratios.
        let ru0 = first[&ObjectKind::RoutingUnit];
        let n_ru = project.objects_of(ObjectKind::RoutingUnit).count();
        let mut rus = Vec::with_capacity(n_ru);
        let mut ru_of = vec![None; obs.len()];
        for iru in 0..n_ru {
            let ob = ru0 + iru;
            let def = project.ru_defs.get(iru).cloned().unwrap_or_default();
            let da_km2 = obs[ob].area_ha / 100.0;
            let mut elems = Vec::with_capacity(def.elems.len());
            let mut ru_n = 0.0_f32;
            for &ie in &def.elems {
                let e = project.ru_elements.get(ie).ok_or_else(|| format!("routing unit {}: element {ie}", obs[ob].name))?;
                let hru = e.obtyp == "hru";
                if !hru {
                    return Err(format!("routing unit {}: element of type {}", obs[ob].name, e.obtyp));
                }
                let k = e.obtypno.max(1) as usize - 1;
                let (ovn, km) = ovn_km.get(k).copied().ok_or_else(|| format!("routing unit {}: HRU {}", obs[ob].name, e.obtypno))?;
                ru_n += ovn * km;
                elems.push(RuElem { ob: k, hru, frac: e.frac, dr: Hyd::default() });
                obs[k].subs.push(ob);
            }
            obs[ob].dfn_tot = elems.len();
            // time_conc_init
            ru_n /= da_km2;
            let data = project.ru_data.get(iru).cloned().unwrap_or_default();
            let topo = project.topography.as_ref();
            let ith = topo.and_then(|t| t.find(&data.topo)).unwrap_or(0);
            let tr = |f: &str, d: f32| topo.and_then(|t| t.real(ith, f)).unwrap_or(d);
            let (slope, slope_len) = (tr("slope", 0.02), tr("slope_len", 50.0));
            let fld = project.field.as_ref();
            let length = fld.and_then(|t| t.find(&data.field).and_then(|r| t.real(r, "length"))).unwrap_or(500.0);
            let tov = 0.0556 * (slope_len * ru_n).powf(0.6) / (slope + 0.001).powf(0.3);
            let ch_slope = 0.5 * (slope + 0.001);
            let ch_n = ru_n;
            let ch_l = length / 1000.0;
            let t_ch = 0.62 * ch_l * ch_n.powf(0.75) / (da_km2.powf(0.125) * ch_slope.powf(0.375));
            let ru_tc = tov + t_ch;
            // dr_ru
            for (e, &ie) in elems.iter_mut().zip(&def.elems) {
                let name = project.ru_elements[ie].dr_name.as_str();
                match name {
                    "calc" | "0" => {
                        let rto = tconc.get(e.ob).copied().unwrap_or(0.0) / ru_tc;
                        let rto = rpow(rto, 0.5).min(1.0);
                        e.dr = Hyd::add_const(rto, Hyd::default());
                        e.dr.flo = 1.0;
                    }
                    "full" | "1" => e.dr = Hyd::add_const(1.0, Hyd::default()),
                    other => return Err(format!("routing unit {}: delivery ratio {other}", obs[ob].name)),
                }
            }
            ru_of[ob] = Some(iru);
            rus.push(Ru { ob, da_km2, elems, tc: ru_tc });
        }

        // aqu_initial
        let aq0 = first[&ObjectKind::Aquifer];
        let n_aq = project.objects_of(ObjectKind::Aquifer).count();
        let mut aqus: Vec<Aqu> = Vec::with_capacity(n_aq);
        let mut aqu_of = vec![None; obs.len()];
        for iaq in 0..n_aq {
            let ob = aq0 + iaq;
            let iaqdb = obs[ob].props.max(0) as usize;
            let db = project.aquifer_db.get(iaqdb).cloned().unwrap_or_default();
            let area = obs[ob].area_ha;
            let mut d = AquDay { flo: db.flo, dep_wt: db.dep_wt, cbn: db.cbn, ..AquDay::default() };
            // The storage uses the water table of the aquifer numbered as the database row
            // (`aqu_d(iaqdb)`), set only once that aquifer has been initialized.
            let dep_wt_db = if fixes {
                d.dep_wt
            } else if iaqdb >= 1 && iaqdb - 1 < iaq {
                aqus[iaqdb - 1].d.dep_wt
            } else if iaqdb >= 1 && iaqdb - 1 == iaq {
                d.dep_wt
            } else {
                0.0
            };
            d.stor = 1000.0 * (db.dep_bot - dep_wt_db) * db.spyld;
            d.no3_st = (10.0 * d.flo * area) * db.no3 / 1000.0;
            d.minp = (10.0 * d.flo * area) * db.minp / 1000.0;
            let alpha_e = (-db.alpha).exp();
            let nloss = (-0.693 / (db.hlife_n + 0.1)).exp();
            aqu_of[ob] = Some(iaq);
            aqus.push(Aqu { ob, db, alpha_e, nloss, d });
        }
        // cal_conditions: the aquifers' calibration (after aqu_initial). A condition would be
        // tested on the HRU numbered as the aquifer.
        for u in &cal.updates {
            let p = crate::hru::cal_parm(cal, u);
            if p.ob_typ != "aqu" {
                continue;
            }
            if !u.conds.is_empty() {
                return Err(format!("calibration of {} with conditions (aquifer)", u.name));
            }
            for e in crate::hru::cal_elems(u, aqus.len()) {
                let a = aqus.get_mut(e.wrapping_sub(1)).ok_or_else(|| format!("calibration of {}: aquifer {e}", u.name))?;
                let chg = |v: f32| crate::hru::chg_par(u, &p, v);
                let area = obs[a.ob].area_ha;
                match u.name.as_str() {
                    "alpha" => {
                        a.db.alpha = chg(a.db.alpha);
                        a.alpha_e = (-a.db.alpha).exp();
                    }
                    "revap_co" => a.db.revap_co = chg(a.db.revap_co),
                    "deep_seep" => a.db.seep = chg(a.db.seep),
                    "flo_min" => a.db.flo_min = chg(a.db.flo_min),
                    "revap_min" => a.db.revap_min = chg(a.db.revap_min),
                    "dep_bot" => a.db.dep_bot = chg(a.db.dep_bot),
                    "minp_init" => a.db.minp = chg(a.db.minp),
                    "sp_yld" => {
                        a.db.spyld = chg(a.db.spyld);
                        a.d.stor = 1000.0 * (a.db.dep_bot - a.d.dep_wt) * a.db.spyld;
                    }
                    "hlife_n" => {
                        a.db.hlife_n = chg(a.db.hlife_n);
                        a.nloss = (-0.693 / (a.db.hlife_n + 0.1)).exp();
                    }
                    "no3_init" => {
                        a.db.no3 = chg(a.db.no3);
                        a.d.no3_st = (10.0 * a.d.flo * area) * a.db.no3 / 1000.0;
                    }
                    // Used only while setting up, or by 2-D aquifers.
                    "flo_init_mm" => a.db.flo = chg(a.db.flo),
                    "dep_wt_init" => a.db.dep_wt = chg(a.db.dep_wt),
                    "cbn_init" => a.db.cbn = chg(a.db.cbn),
                    "flo_dist" => a.db.flo_dist = chg(a.db.flo_dist),
                    "bf_max" => a.db.bf_max = chg(a.db.bf_max),
                    other => return Err(format!("calibration of {other}")),
                }
            }
        }

        // The command order (`cmd_next`): objects fire, in passes over the array, once all
        // their inflows and (for routing units) all their elements have.
        let n = obs.len();
        let rcv_tot: Vec<usize> = obs.iter().map(|o| o.ins.len()).collect();
        let mut rcv_sum = vec![0usize; n];
        let mut dfn_sum = vec![0usize; n];
        let mut fired = vec![false; n];
        let mut order = Vec::with_capacity(n);
        let mut hru_order = Vec::new();
        let mut done = 0usize;
        let mut iord = 1;
        while done < n {
            if iord > 5000 {
                return Err("an infinite loop in the connection files".into());
            }
            for i in 0..n {
                if fired[i] || rcv_sum[i] != rcv_tot[i] || dfn_sum[i] != obs[i].dfn_tot {
                    continue;
                }
                fired[i] = true;
                done += 1;
                for &k in &obs[i].subs {
                    dfn_sum[k] += 1;
                }
                for o in &obs[i].outs {
                    if let Some(k) = o.to {
                        rcv_sum[k] += 1;
                    }
                }
                if obs[i].kind != ObjectKind::Hru {
                    order.push(i);
                } else {
                    hru_order.push(i);
                }
            }
            iord += 1;
        }
        // sd_channel_read, sd_hydsed_init
        let ch0 = first[&ObjectKind::ChannelDeg];
        let n_ch = project.objects_of(ObjectKind::ChannelDeg).count();
        let props: Vec<i32> = (0..n_ch).map(|k| obs[ch0 + k].props).collect();
        let (channels, channel_skipped) = match crate::channel::Channels::new(project, &props, fixes) {
            Ok(c) if n_ch > 0 => (Some(c), None),
            Ok(_) => (None, None),
            Err(e) => (None, Some(e)),
        };
        let mut chan_of = vec![None; obs.len()];
        let mut chan_in = Vec::with_capacity(n_ch);
        for k in 0..n_ch {
            chan_of[ch0 + k] = Some(k);
            let ins = &obs[ch0 + k].ins;
            let from_ch: Vec<bool> = ins.iter().map(|i| obs[i.from].kind == ObjectKind::ChannelDeg).collect();
            let rus_in: Vec<usize> = ins.iter().filter_map(|i| ru_of[i.from]).collect();
            chan_in.push((from_ch, rus_in));
        }
        // res_read, res_initial; recall_read.
        let (yrc, yrc_end) = (project.time.yrc_start, project.time.yrc_end);
        let nbyr = (yrc_end - yrc + 1).max(1) as usize;
        let mut reservoirs = Vec::new();
        let r0 = first[&ObjectKind::Reservoir];
        let n_res = project.objects_of(ObjectKind::Reservoir).count();
        if n_res > 0 && project.codes.lapse == 1 {
            return Err("reservoirs with lapse rates".into());
        }
        for k in 0..n_res {
            reservoirs.push(crate::res::Reservoir::new(project, k, r0 + k, obs[r0 + k].props, yrc)?);
        }
        let mut recalls = Vec::new();
        let c0 = first[&ObjectKind::Recall];
        for k in 0..project.objects_of(ObjectKind::Recall).count() {
            recalls.push(crate::res::Recall::new(project, c0 + k, obs[c0 + k].props, yrc, yrc_end, nbyr)?);
        }
        let n_ru = rus.len();
        // hyd_read_connect: two days of hydrographs for HRUs, routing units, channels and
        // outlets (`day_lag_mx`), one for the others.
        let step = project.time.step.max(1) as usize;
        for o in obs.iter_mut() {
            o.day_max = match o.kind {
                ObjectKind::Hru
                | ObjectKind::HruLte
                | ObjectKind::RoutingUnit
                | ObjectKind::Channel
                | ObjectKind::Outlet
                | ObjectKind::ChannelDeg => 2,
                _ => 1,
            };
            o.hyd_flo = vec![0.0; o.day_max * step];
            o.tsin = vec![0.0; step];
        }
        // unit_hyd_ru_hru: with the curve number, each HRU's and routing unit's unit hydrograph.
        let ga = project.codes.gampt == 1;
        if step > 1 && !ga {
            let uhp = UhParams {
                step,
                dtm: 1440.0 / step as f32,
                tb_adj: project.params.get("tb_adj").unwrap_or(0.0),
                uhyd: project.codes.uhyd,
                uhalpha: project.params.get("uhalpha").unwrap_or(1.0),
            };
            for (k, o) in obs.iter_mut().enumerate() {
                if o.kind == ObjectKind::Hru {
                    o.uh = unit_hyd(tconc.get(k).copied().unwrap_or(0.0), &uhp);
                }
            }
            for r in &rus {
                obs[r.ob].uh = unit_hyd(r.tc, &uhp);
            }
        }
        Ok(Some(Network {
            obs,
            order,
            hru_order,
            rus,
            ru_of,
            aqu_of,
            aqus,
            ru_d: vec![Hyd::default(); n_ru],
            fixes,
            channels,
            chan_of,
            chan_in,
            channel_skipped,
            reservoirs,
            recalls,
            step,
            ga,
            dtm: 1440.0 / step as f32,
            tconc: tconc.to_vec(),
            hhsurfq: vec![Vec::new(); n_hru],
            n_hru,
            plan: None,
        }))
    }

    /// The flow of today's hydrograph of channel object `ob` by step (`hyd_flo(1, :)`).
    pub fn hyd_flo_day1(&self, ob: usize) -> &[f32] {
        &self.obs[ob].hyd_flo[..self.step]
    }

    /// Advances an object's current hydrograph day (`command`).
    fn next_day(o: &mut Ob) {
        if matches!(o.kind, ObjectKind::Hru | ObjectKind::RoutingUnit) {
            o.day_cur += 1;
        } else {
            o.day_cur = 2;
        }
        if o.day_cur > o.day_max {
            o.day_cur = 1;
        }
    }

    /// Moves the hydrograph of day `cur` on by `tinc` steps, the last into day `next`
    /// (`hru_hyds`, `ru_control`).
    /// SWAT+ `flow_hyd_ru_hru`: `surfq` (m3) spread over today's and tomorrow's steps by the
    /// object's unit hydrograph, tomorrow's first cleared. (The lateral and tile flow it is given
    /// are left out.)
    fn flow_hyd(o: &mut Ob, step: usize, surfq: f32) {
        let day_max = o.day_max;
        let mut iday_cur = o.day_cur;
        let mut iday_prev = if o.day_cur <= 1 { day_max } else { o.day_cur - 1 };
        for v in &mut o.hyd_flo[(iday_prev - 1) * step..iday_prev * step] {
            *v = 0.0;
        }
        for iday in 1..=day_max {
            for istep in 0..step {
                let sq = o.uh.get((iday - 1) * step + istep).copied().unwrap_or(0.0) * surfq;
                o.hyd_flo[(iday_cur - 1) * step + istep] += sq;
            }
            iday_cur += 1;
            if iday_cur > day_max {
                iday_cur = 1;
            }
            iday_prev += 1;
            if iday_prev > day_max {
                iday_prev = 1;
            }
        }
    }

    fn shift(hyd: &mut [f32], step: usize, cur: usize, next: usize, tinc: usize) {
        let c = (cur - 1) * step;
        let n = (next - 1) * step;
        for istep in 1..=tinc {
            let inext = step - tinc + istep;
            hyd[n + istep - 1] = hyd[c + inext - 1];
        }
        for istep in 1..=step {
            let bak = step - istep + 1;
            hyd[c + bak - 1] = if bak <= tinc { 0.0 } else { hyd[c + bak - tinc - 1] };
        }
    }

    /// The HRUs (0-based) in the order SWAT+ runs them.
    pub fn hru_order(&self) -> &[usize] {
        &self.hru_order
    }

    /// The HRUs in waves that can run at once (`--fix-swat`'s parallel HRUs): an HRU is in a
    /// later wave than every HRU it receives water from earlier in the command order, so it reads
    /// their water of today, and in no earlier wave than one it gives water to that runs before it
    /// in that order, so that one still reads its water of yesterday. Each wave keeps the command
    /// order.
    pub fn hru_waves(&self) -> Vec<Vec<usize>> {
        let n = self.obs.len();
        let pos: Vec<usize> = {
            let mut p = vec![usize::MAX; n];
            for (k, &j) in self.hru_order.iter().enumerate() {
                p[j] = k;
            }
            p
        };
        let mut level = vec![0usize; n];
        let mut floor = vec![0usize; n];
        for &j in &self.hru_order {
            let mut l = floor[j];
            for i in &self.obs[j].ins {
                let s = i.from;
                if self.obs[s].kind != ObjectKind::Hru || pos[s] == usize::MAX {
                    continue;
                }
                if pos[s] < pos[j] {
                    l = l.max(level[s] + 1);
                }
            }
            level[j] = l;
            for i in &self.obs[j].ins {
                let s = i.from;
                if self.obs[s].kind == ObjectKind::Hru && pos[s] != usize::MAX && pos[s] > pos[j] {
                    floor[s] = floor[s].max(l);
                }
            }
        }
        let mut waves: Vec<Vec<usize>> = Vec::new();
        for &j in &self.hru_order {
            if waves.len() <= level[j] {
                waves.resize(level[j] + 1, Vec::new());
            }
            waves[level[j]].push(j);
        }
        waves
    }

    /// `command` for a receiving HRU `j`: the surface, lateral and tile flow it gets from other
    /// HRUs, per hectare (flow in mm). A total hydrograph brings the three; `None` when nothing
    /// is routed into it.
    pub fn hru_runon(&self, j: usize) -> Option<crate::hru::Runon> {
        let o = self.obs.get(j)?;
        if o.ins.is_empty() {
            return None;
        }
        let (mut sur, mut lat, mut til) = (Hyd::default(), Hyd::default(), Hyd::default());
        for i in &o.ins {
            let src = &self.obs[i.from];
            match i.ihtyp {
                1 => {
                    sur = sur.plus(Hyd::scale(i.frac, src.hd[2]));
                    til = til.plus(Hyd::scale(i.frac, src.hd[4]));
                    lat = lat.plus(Hyd::scale(i.frac, src.hd[3]));
                }
                3 => sur = sur.plus(Hyd::scale(i.frac, src.hd[2])),
                4 => lat = lat.plus(Hyd::scale(i.frac, src.hd[3])),
                5 => til = til.plus(Hyd::scale(i.frac, src.hd[4])),
                // Recharge isn't one of the HRU's inflows.
                _ => {}
            }
        }
        let conv = o.area_ha;
        let per_ha = |h: Hyd| {
            let mut h = h.over(conv);
            h.flo /= 10.0;
            h
        };
        Some(crate::hru::Runon { sur: per_ha(sur), lat: per_ha(lat), til: per_ha(til) })
    }

    /// Sets HRU `j`'s hydrographs for today (0-based).
    pub fn set_hru(&mut self, j: usize, hd: &[Hyd; 5], hhsurfq: &[f32]) {
        let (step, dtm, ga) = (self.step, self.dtm, self.ga);
        let tconc = self.tconc.get(j).copied().unwrap_or(0.0);
        if let Some(o) = self.obs.get_mut(j) {
            o.hd.copy_from_slice(hd);
            Self::next_day(o);
            if step > 1 && !ga {
                // hru_hyds: the day's surface runoff spread by the unit hydrograph.
                let surfq = o.hd[2].flo;
                Self::flow_hyd(o, step, surfq);
            } else if step > 1 {
                // hru_hyds (Green and Ampt): the surface runoff by step, moved on by the time of
                // concentration.
                let cur = o.day_cur;
                let next = if cur + 1 > o.day_max { 1 } else { cur + 1 };
                let cnv_m3 = o.area_ha * 10.0;
                for ii in 0..step {
                    o.hyd_flo[(cur - 1) * step + ii] += hhsurfq.get(ii).copied().unwrap_or(0.0) * cnv_m3 / dtm * 60.0;
                }
                for ii in 0..step {
                    o.hyd_flo[(next - 1) * step + ii] = 0.0;
                }
                if tconc * 60.0 > dtm {
                    let tinc = ((tconc * 60.0 / dtm) as i32).min(step as i32).max(0) as usize;
                    Self::shift(&mut o.hyd_flo, step, cur, next, tinc);
                }
            }
        }
        if let Some(h) = self.hhsurfq.get_mut(j) {
            h.clear();
            h.extend_from_slice(hhsurfq);
        }
    }

    /// The command loop past the HRUs. With `fixes` and more than one thread, the network is
    /// cut into parts (see [`Plan`]) routed at once, then what is left in command order; each
    /// object's day is the same whatever the part it is in, so the results are the same for any
    /// number of threads.
    pub fn day(&mut self, nd: &NetDay, threads: usize) {
        let threads = if self.fixes { threads.max(1) } else { 1 };
        if self.plan.as_ref().is_none_or(|p| p.threads != threads) {
            self.plan = Some(self.plan(threads));
        }
        let Network {
            obs,
            rus,
            ru_of,
            aqu_of,
            aqus,
            ru_d,
            fixes,
            channels,
            chan_of,
            chan_in,
            reservoirs,
            recalls,
            step,
            ga,
            dtm,
            hhsurfq,
            plan,
            n_hru,
            ..
        } = self;
        let Some(plan) = plan.as_ref() else { return };
        let n = obs.len();
        let (chs, ch_cfg, carry, mut air_lags) = match channels.as_mut() {
            Some(c) => (Some(&mut c.chs), Some(&c.cfg), Some(&mut c.carry), Some(&mut c.air_lags)),
            None => (None, None, None, None),
        };
        let cfg = Cfg { rus, ru_of, aqu_of, chan_of, chan_in, fixes: *fixes, step: *step, ga: *ga, dtm: *dtm, hhsurfq, ch: ch_cfg };
        // Each object with what it owns of the kinds' state.
        let mut st: Vec<St> = (0..n).map(|_| St::None).collect();
        for (k, h) in ru_d.iter_mut().enumerate() {
            st[rus[k].ob] = St::Ru(h);
        }
        for a in aqus.iter_mut() {
            let ob = a.ob;
            st[ob] = St::Aqu(a);
        }
        if let Some(chs) = chs {
            for (ob, c) in chan_of.iter().enumerate().filter_map(|(ob, k)| k.map(|_| ob)).zip(chs.iter_mut()) {
                st[ob] = St::Ch(c);
            }
        }
        for r in reservoirs.iter_mut() {
            let ob = r.ob;
            st[ob] = St::Res(r);
        }
        for r in recalls.iter_mut() {
            let ob = r.ob;
            st[ob] = St::Rec(r);
        }
        let (hrus, rest) = obs.split_at_mut(*n_hru);
        let mut seen: Vec<Option<&Ob>> = hrus.iter().map(Some).collect();
        seen.resize(n, None);
        let mut slots: Vec<Option<(&mut Ob, St)>> = rest.iter_mut().zip(st.drain(*n_hru..)).map(Some).collect();
        let mut take = |ids: &[usize]| -> Vec<(&mut Ob, St)> { ids.iter().filter_map(|&i| slots[i - *n_hru].take()).collect() };
        let trunk_ids = &plan.parts[plan.parts.len() - 1];
        let shared = if cfg.fixes { None } else { carry };
        if plan.parts.len() > 1 {
            let mut parts: Vec<Part> = plan.parts[..plan.parts.len() - 1]
                .iter()
                .enumerate()
                .map(|(p, ids)| Part { ids, own: take(ids), me: p as u32, plan, carry: None, lags: AirLags::new() })
                .collect();
            let trunk_own = take(trunk_ids);
            {
                let (cfg, seen) = (&cfg, &seen);
                std::thread::scope(|sc| {
                    for part in parts.iter_mut() {
                        sc.spawn(move || run_part(cfg, part, seen, nd));
                    }
                });
            }
            for part in parts {
                for ((ob, _), &i) in part.own.into_iter().zip(part.ids) {
                    seen[i] = Some(ob);
                }
            }
            let me = (plan.parts.len() - 1) as u32;
            let lags = air_lags.as_deref_mut().map(std::mem::take).unwrap_or_default();
            let mut trunk = Part { ids: trunk_ids, own: trunk_own, me, plan, carry: shared, lags };
            run_part(&cfg, &mut trunk, &seen, nd);
            if let Some(l) = air_lags {
                *l = trunk.lags;
            }
        } else {
            let own = take(trunk_ids);
            let lags = air_lags.as_deref_mut().map(std::mem::take).unwrap_or_default();
            let mut all = Part { ids: trunk_ids, own, me: 0, plan, carry: shared, lags };
            run_part(&cfg, &mut all, &seen, nd);
            if let Some(l) = air_lags {
                *l = all.lags;
            }
        }
    }

    /// How to cut the objects past the HRUs into parts for `threads` (see [`Plan`]).
    fn plan(&self, threads: usize) -> Plan {
        let ins: Vec<Vec<usize>> = self.obs.iter().map(|o| o.ins.iter().map(|i| i.from).collect()).collect();
        make_plan(&self.order, &ins, threads)
    }

    /// The parts the objects past the HRUs are routed in on `threads` threads: their numbers of
    /// objects, the last part routed after the others.
    pub fn parts(&self, threads: usize) -> Vec<usize> {
        self.plan(threads.max(1)).parts.iter().map(Vec::len).collect()
    }
}

/// [`Plan`] for the objects of `order` (indices into `ins`, each object's sources; objects not in
/// `order`, the HRUs, run before) on `threads` threads.
fn make_plan(order: &[usize], ins: &[Vec<usize>], threads: usize) -> Plan {
    let n = ins.len();
    let m = order.len();
    let mut pos = vec![usize::MAX; n];
    for (k, &i) in order.iter().enumerate() {
        pos[i] = k;
    }
    // What must run before what (by position in the command order): an object after the
    // objects whose water of today it takes, and before those whose water of yesterday it
    // takes (they come later in the order).
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); m];
    for (k, &j) in order.iter().enumerate() {
        for &from in &ins[j] {
            let s = pos[from];
            if s == usize::MAX {
                continue;
            }
            if s < k {
                deps[k].push(s);
            } else if s > k {
                deps[s].push(k);
            }
        }
    }
    let mut parts: Vec<Vec<usize>> = Vec::new();
    let mut trunk: Vec<usize> = (0..m).collect();
    if threads > 1 && m > 1 {
        // Every object's ancestors (all that must run before it), as bit sets: each edge
        // goes from an earlier position to a later one.
        let words = m.div_ceil(64);
        let mut anc = vec![0u64; m * words];
        for k in 0..m {
            for &d in &deps[k] {
                let (lo, hi) = anc.split_at_mut(k * words);
                let mine = &mut hi[..words];
                for (w, v) in mine.iter_mut().zip(&lo[d * words..(d + 1) * words]) {
                    *w |= v;
                }
                mine[d / 64] |= 1 << (d % 64);
            }
        }
        let size = |k: usize| anc[k * words..(k + 1) * words].iter().map(|w| w.count_ones() as usize).sum::<usize>() + 1;
        // Groups: from the outlets up, the first object small enough with all its ancestors;
        // groups that share an ancestor are joined. A group then needs nothing outside it.
        let target = m.div_ceil(2 * threads).max(1);
        let mut group = vec![usize::MAX; m];
        let mut parent: Vec<usize> = (0..m).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for k in (0..m).rev() {
            if group[k] != usize::MAX || size(k) > target {
                continue;
            }
            group[k] = k;
            for wi in 0..words {
                let mut w = anc[k * words + wi];
                while w != 0 {
                    let d = wi * 64 + w.trailing_zeros() as usize;
                    w &= w - 1;
                    if group[d] == usize::MAX {
                        group[d] = k;
                    } else {
                        let (a, b) = (find(&mut parent, group[d]), find(&mut parent, k));
                        if a != b {
                            parent[a.max(b)] = a.min(b);
                        }
                    }
                }
            }
        }
        let mut members: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
        trunk.clear();
        for k in 0..m {
            if group[k] == usize::MAX {
                trunk.push(k);
            } else {
                let g = find(&mut parent, group[k]);
                members.entry(g).or_default().push(k);
            }
        }
        // The groups onto the threads, largest first, each to the least loaded.
        let mut groups: Vec<Vec<usize>> = members.into_values().collect();
        groups.sort_by_key(|g| std::cmp::Reverse(g.len()));
        let mut bins: Vec<Vec<usize>> = vec![Vec::new(); threads];
        for g in groups {
            let b = (0..threads).min_by_key(|&b| (bins[b].len(), b)).unwrap_or(0);
            bins[b].extend(g);
        }
        for mut b in bins {
            if !b.is_empty() {
                b.sort_unstable();
                parts.push(b);
            }
        }
    }
    parts.push(trunk);
    let parts: Vec<Vec<usize>> = parts.into_iter().map(|p| p.into_iter().map(|k| order[k]).collect()).collect();
    let mut part_of = vec![u32::MAX; n];
    let mut at = vec![0u32; n];
    for (p, ids) in parts.iter().enumerate() {
        for (k, &i) in ids.iter().enumerate() {
            part_of[i] = p as u32;
            at[i] = k as u32;
        }
    }
    Plan { threads, parts, part_of, at }
}

impl Network {
    /// Today's aquifers, in order: number (1-based), GIS id, name and state.
    pub fn aquifers(&self) -> impl Iterator<Item = (usize, i32, &str, &AquDay)> {
        self.aqus.iter().enumerate().map(|(i, a)| (i + 1, self.obs[a.ob].gis_id, self.obs[a.ob].name.as_str(), &a.d))
    }

    /// Today's channels: number (1-based), GIS id, name, water body, storage, inflow and outflow
    /// (flows in m3/s), and the one water temperature printed for all.
    pub fn channels(&self) -> Vec<ChannelDay<'_>> {
        let Some(chs) = self.channels.as_ref() else { return Vec::new() };
        let mut out = Vec::new();
        for (ob, k) in self.chan_of.iter().enumerate() {
            if let Some(k) = *k {
                let (st, i, o, w) = chs.output(k);
                out.push((k + 1, self.obs[ob].gis_id, self.obs[ob].name.as_str(), *w, *st, *i, *o, chs.wtemp_of(k)));
            }
        }
        out
    }

    /// The channels' hydraulics on the day (`ChHyd`), in the order of [`Network::channels`].
    pub fn channel_hyds(&self) -> Vec<crate::channel::ChHyd> {
        let Some(chs) = self.channels.as_ref() else { return Vec::new() };
        self.chan_of.iter().flatten().map(|&k| chs.hyd_of(k)).collect()
    }

    /// The reservoirs: number (1-based), GIS id, name and the reservoir.
    pub fn reservoir_days(&self) -> impl Iterator<Item = (usize, i32, &str, &crate::res::Reservoir)> {
        self.reservoirs.iter().enumerate().map(|(i, r)| (i + 1, self.obs[r.ob].gis_id, self.obs[r.ob].name.as_str(), r))
    }

    /// The recall objects: number (1-based), name and today's load.
    pub fn recall_days(&self) -> impl Iterator<Item = (usize, &str, &Hyd)> {
        self.recalls.iter().enumerate().map(|(i, r)| (i + 1, self.obs[r.ob].name.as_str(), &r.d))
    }

    /// Sub-daily runs: today's outflow of each channel by time step (`hyd_flo(1, :)`, m3 a step),
    /// with its number (1-based), GIS id and name.
    pub fn channel_steps(&self) -> Vec<(usize, i32, &str, &[f32])> {
        if self.step <= 1 || self.channels.is_none() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (ob, k) in self.chan_of.iter().enumerate() {
            if let Some(k) = *k {
                out.push((k + 1, self.obs[ob].gis_id, self.obs[ob].name.as_str(), &self.obs[ob].hyd_flo[..self.step]));
            }
        }
        out
    }

    /// Today's routing units: name and `ru_d`.
    pub fn routing_units(&self) -> impl Iterator<Item = (&str, &Hyd)> {
        self.rus.iter().zip(&self.ru_d).map(|(r, h)| (self.obs[r.ob].name.as_str(), h))
    }
}

/// What `unit_hyd` takes from the basin: the steps, the base time's adjustment (`tb_adj`, h),
/// the hydrograph's shape (`uhyd` 0 triangular, 1 gamma) and the gamma's alpha.
struct UhParams {
    step: usize,
    dtm: f32,
    tb_adj: f32,
    uhyd: i32,
    uhalpha: f32,
}

/// SWAT+ `unit_hyd`: the dimensionless unit hydrograph over two days of steps for a time of
/// concentration `tc` (h): a base time of `0.5 + 0.6 tc + tb_adj` h (at most 48), a peak at 3/8
/// of it, integrated over a few points a step; a day's steps stop once the rate falls under
/// 1e-4. Scaled to sum to 1 (flat, day by day).
fn unit_hyd(tc: f32, p: &UhParams) -> Vec<f32> {
    let step = p.step;
    let mut uh = vec![0.0_f32; 2 * step];
    let mut ql = 0.0_f32;
    let mut sumq = 0.0_f32;
    let mut tb = 0.5 + 0.6 * tc + p.tb_adj;
    if tb > 48.0 {
        tb = 48.0;
    }
    let tp = 0.375 * tb;
    let mut t_tot = 0.0_f32;
    let ts_base = ((tb / (p.dtm / 60.0)) as i32).max(1);
    let t_inc = 20 / ts_base + 2;
    let t_inc_hr = p.dtm / t_inc as f32 / 60.0;
    let mut q = 0.0_f32;
    for iday in 0..2 {
        for istep in 0..step {
            for _ in 0..t_inc {
                t_tot += t_inc_hr;
                if p.uhyd == 0 {
                    q = if t_tot < tp { t_tot / tp } else { (tb - t_tot) / (tb - tp) };
                }
                if p.uhyd == 1 {
                    q = crate::fortran::rpow(t_tot / tp, p.uhalpha) * ((1.0 - t_tot / tp) * p.uhalpha).exp();
                }
                q = q.max(0.0);
                uh[iday * step + istep] += (q + ql) / 2.0;
                sumq += (q + ql) / 2.0;
                ql = q;
            }
            if q < 1.0e-4 {
                break;
            }
        }
    }
    for v in uh.iter_mut() {
        *v /= sumq;
    }
    uh
}

/// The objects past the HRUs cut into parts for `threads` threads (`--fix-swat`): parts that
/// need nothing from one another, routed at once, then the last part (the rest, mostly the
/// main channels) in command order. A part keeps the command order. One part holds them all
/// when routed on one thread or reproducing SWAT+.
#[derive(Clone, Debug)]
struct Plan {
    threads: usize,
    parts: Vec<Vec<usize>>,
    /// Each object's part (`u32::MAX` for the HRUs) and its place in it.
    part_of: Vec<u32>,
    at: Vec<u32>,
}

/// What every object's day reads of the network.
struct Cfg<'a> {
    rus: &'a [Ru],
    ru_of: &'a [Option<usize>],
    aqu_of: &'a [Option<usize>],
    chan_of: &'a [Option<usize>],
    chan_in: &'a [(Vec<bool>, Vec<usize>)],
    fixes: bool,
    step: usize,
    ga: bool,
    dtm: f32,
    hhsurfq: &'a [Vec<f32>],
    ch: Option<&'a crate::channel::ChanCfg>,
}

/// The state of its kind an object owns.
enum St<'a> {
    None,
    Ru(&'a mut Hyd),
    Aqu(&'a mut Aqu),
    Ch(&'a mut crate::channel::SdCh),
    Res(&'a mut crate::res::Reservoir),
    Rec(&'a mut crate::res::Recall),
}

/// A part being routed: its objects (in command order) with their state; the objects of the
/// other parts it reads come read-only.
struct Part<'a> {
    ids: &'a [usize],
    own: Vec<(&'a mut Ob, St<'a>)>,
    me: u32,
    plan: &'a Plan,
    /// The channels' carried values SWAT+ shares (reproducing it), or none.
    carry: Option<&'a mut crate::channel::Carry>,
    lags: AirLags,
}

use crate::channel::AirLags;

/// Object `i` as read by a part: its own, or read-only (`None` would be an object of a part
/// routed at the same time, which the plan rules out).
fn src<'b>(part: &'b Part, seen: &'b [Option<&Ob>], i: usize) -> Option<&'b Ob> {
    if part.plan.part_of[i] == part.me { part.own.get(part.plan.at[i] as usize).map(|o| &*o.0) } else { seen.get(i).copied().flatten() }
}

/// The command loop over one part.
fn run_part(cfg: &Cfg, part: &mut Part, seen: &[Option<&Ob>], nd: &NetDay) {
    let mut hin_d: Vec<Hyd> = Vec::new();
    for k in 0..part.own.len() {
        let icmd = part.ids[k];
        let mut hin = Hyd::default();
        hin_d.clear();
        for q in 0..part.own[k].0.ins.len() {
            let In { from, ihtyp, frac } = part.own[k].0.ins[q];
            let Some(s) = src(part, seen, from) else { continue };
            let ht1 = Hyd::scale(frac, s.hd[ihtyp - 1]);
            hin = hin.plus(ht1);
            hin_d.push(ht1);
        }
        part.own[k].0.hin = hin;
        Network::next_day(part.own[k].0);
        if cfg.step > 1 {
            let tsin = sum_tsin(cfg, part, seen, k);
            part.own[k].0.tsin = tsin;
        }
        let wst = part.own[k].0.wst.wrapping_sub(1);
        match part.own[k].0.kind {
            ObjectKind::RoutingUnit => {
                if let Some(iru) = cfg.ru_of[icmd] {
                    ru_control(cfg, part, seen, k, iru);
                }
            }
            ObjectKind::Aquifer => {
                if cfg.aqu_of[icmd].is_some() {
                    let pet = nd.weat.get(wst).map(|w| w.pet).unwrap_or(0.0);
                    let (ob, st) = &mut part.own[k];
                    if let St::Aqu(a) = st {
                        aqu_1d_control(ob, a, pet);
                    }
                }
            }
            ObjectKind::ChannelDeg if cfg.ch.is_some() => {
                let ich = cfg.chan_of[icmd].unwrap_or(0);
                let (Some(w), Some(ch)) = (nd.weat.get(wst), cfg.ch) else { continue };
                let cx = crate::channel::ChanCtx {
                    weat: w,
                    daylth: nd.daylth.get(wst).copied().unwrap_or(0.0),
                    tgage: nd.tgage.get(wst).copied().flatten(),
                    yrs: nd.yrs,
                    jday: nd.jday,
                    hru_wb: nd.hru_wb,
                };
                let (from_ch, rus) = &cfg.chan_in[ich];
                let ins = crate::channel::ChanInflows { from_chandeg: from_ch, hin_d: &hin_d, rus };
                let Part { own, carry, lags, .. } = part;
                let (ob, st) = &mut own[k];
                let St::Ch(c) = st else { continue };
                let area = ob.area_ha;
                let tsin = std::mem::take(&mut ob.tsin);
                let mut hyd = std::mem::take(&mut ob.hyd_flo);
                let mut flo = Vec::new();
                ob.hd[0] = ch.control(c, carry.as_deref_mut(), lags, hin, area, &ins, &cx, &tsin, &mut flo);
                // ch_rtmusk zeroes the whole hydrograph and fills the first day.
                for v in hyd.iter_mut() {
                    *v = 0.0;
                }
                for (v, f) in hyd.iter_mut().zip(&flo) {
                    *v = *f;
                }
                ob.hyd_flo = hyd;
                ob.tsin = tsin;
            }
            ObjectKind::Reservoir => {
                // A reservoir with no inflow isn't run.
                let (ob, st) = &mut part.own[k];
                if let St::Res(r) = st
                    && !ob.ins.is_empty()
                {
                    let w = nd.weat.get(wst);
                    let (precip, pet, tave) = w.map_or((0.0, 0.0, 0.0), |w| (w.precip, w.pet, w.tave));
                    ob.hd[0] = r.control(hin, nd.yrc, nd.mo, nd.jday, nd.day_mo, precip, pet, tave, nd.print);
                }
            }
            ObjectKind::Recall => {
                let (ob, st) = &mut part.own[k];
                if let St::Rec(r) = st {
                    ob.hd[0] = r.day(nd.jday, nd.mo, nd.yrc, nd.yrs);
                }
            }
            _ => {
                // Outlets, and channels not routed yet.
                part.own[k].0.hd[0] = hin;
            }
        }
    }
}

/// `command`: the inflow by step of the part's object `k` (`tsin`), the sum of its inputs'. An
/// input of a kind not listed passes on the hydrograph of the input before it (`hyd_flo` isn't
/// reset between inputs).
fn sum_tsin(cfg: &Cfg, part: &Part, seen: &[Option<&Ob>], k: usize) -> Vec<f32> {
    let step = cfg.step;
    let mut tsin = vec![0.0_f32; step];
    let mut hyd = vec![0.0_f32; step];
    for i in &part.own[k].0.ins {
        let In { from, ihtyp, frac } = *i;
        let Some(src) = src(part, seen, from) else { continue };
        let even = |flo: f32| vec![flo / step as f32; step];
        match src.kind {
            ObjectKind::Hru | ObjectKind::RoutingUnit => {
                let day = &src.hyd_flo[(src.day_cur.max(1) - 1) * step..src.day_cur.max(1) * step];
                match ihtyp {
                    1 => {
                        let add = (src.hd[3].flo + src.hd[4].flo) / step as f32;
                        for ii in 0..step {
                            hyd[ii] = day[ii] + add;
                        }
                    }
                    3 => hyd.copy_from_slice(day),
                    2 | 4 | 5 => hyd = even(src.hd[ihtyp - 1].flo),
                    _ => {}
                }
            }
            ObjectKind::Aquifer | ObjectKind::Reservoir | ObjectKind::Outlet => hyd = even(src.hd[ihtyp - 1].flo),
            ObjectKind::ChannelDeg => hyd.copy_from_slice(&src.hyd_flo[..step]),
            ObjectKind::Recall => hyd = even(src.hd[0].flo),
            _ => {}
        }
        for ii in 0..step {
            hyd[ii] *= frac;
            tsin[ii] += hyd[ii];
        }
    }
    tsin
}

/// SWAT+ `ru_control` for the part's object `k`, routing unit `iru`: the elements' hydrographs,
/// with their delivery ratios (but not on the recharge), scaled to the share of each in the unit
/// and summed; `ru_d` sums all five.
fn ru_control(cfg: &Cfg, part: &mut Part, seen: &[Option<&Ob>], k: usize, iru: usize) {
    let ru = &cfg.rus[iru];
    let mut hd = [Hyd::default(); 5];
    let mut ru_d = Hyd::default();
    for e in &ru.elems {
        let Some(src) = src(part, seen, e.ob) else { continue };
        let mut ef = e.frac;
        if e.hru && ef < 0.99999 {
            ef = ef * ru.da_km2 / (src.area_ha / 100.0);
        }
        for (k, h) in src.hd.iter().enumerate() {
            let ht1 = if k != 1 { h.times(e.dr) } else { *h };
            let ht1 = Hyd::scale(ef, ht1);
            hd[k] = hd[k].plus(ht1);
            // SWAT+ sums all five hydrographs, counting the total twice; corrected, the
            // total and the recharge.
            if !cfg.fixes || k <= 1 {
                ru_d = ru_d.plus(ht1);
            }
        }
        // Green and Ampt: the unit's hydrograph is the element HRU's plus its runoff by step
        // (as m3/s), replaced by each element in turn, then moved on by a time of
        // concentration of 2 h.
        if cfg.step > 1 && cfg.ga && e.hru {
            let (step, dtm) = (cfg.step, cfg.dtm);
            let (cur, day_max, ru_area) = {
                let o = &part.own[k].0;
                (o.day_cur, o.day_max, o.area_ha)
            };
            let next = if cur + 1 > day_max { 1 } else { cur + 1 };
            let vals: Vec<f32> = (0..step)
                .map(|ii| {
                    let cnv = e.frac * ru_area * e.dr.flo * 10.0 / (dtm * 60.0);
                    let ts_flo = cnv * cfg.hhsurfq.get(e.ob).and_then(|h| h.get(ii)).copied().unwrap_or(0.0);
                    src.hyd_flo[(cur - 1) * step + ii] + ts_flo
                })
                .collect();
            let o = &mut part.own[k].0;
            for (ii, v) in vals.into_iter().enumerate() {
                o.hyd_flo[(cur - 1) * step + ii] = v;
            }
            for ii in 0..step {
                o.hyd_flo[(next - 1) * step + ii] = 0.0;
            }
            let ru_tc = 2.0_f32;
            if ru_tc * 60.0 > dtm {
                let tinc = (ru_tc * 60.0 / dtm) as usize;
                Network::shift(&mut o.hyd_flo, step, cur, next, tinc);
            }
        }
    }
    let (o, st) = &mut part.own[k];
    o.hd.copy_from_slice(&hd);
    // With the curve number: the unit's surface runoff spread by its unit hydrograph.
    if cfg.step > 1 && !cfg.ga {
        Network::flow_hyd(o, cfg.step, hd[2].flo);
    }
    if let St::Ru(d) = st {
        **d = ru_d;
    }
}

/// SWAT+ `aqu_1d_control`: recharge, return flow by recession, seepage, revap, and nitrate.
fn aqu_1d_control(o: &mut Ob, a: &mut Aqu, pet: f32) {
    let hin = o.hin;
    let area = o.area_ha;
    let (db, d) = (&a.db, &mut a.d);
    let mut hd1 = Hyd::default();
    let mut hd2 = Hyd::default();
    d.rchrg = hin.flo / (10.0 * area);
    d.stor += d.rchrg;
    d.dep_wt = db.dep_bot - (d.stor / (1000.0 * db.spyld));
    d.dep_wt = d.dep_wt.max(0.0);
    if d.dep_wt <= db.flo_min {
        d.flo = d.flo * a.alpha_e + d.rchrg * (1.0 - a.alpha_e);
        d.flo = d.flo.max(0.0);
        d.flo = d.stor.min(d.flo);
        d.stor -= d.flo;
    } else {
        d.flo = 0.0;
    }
    hd1.flo = 10.0 * d.flo * area;
    d.seep = d.rchrg * db.seep;
    d.seep = d.seep.min(d.stor);
    hd2.flo = 10.0 * d.seep * area;
    d.stor -= d.seep;
    if d.dep_wt < db.revap_min {
        d.revap = pet * db.revap_co;
        d.revap = d.revap.min(d.stor);
        d.stor -= d.revap;
    } else {
        d.revap = 0.0;
    }
    d.no3_rchg = hin.no3 / area;
    d.no3_st += d.no3_rchg;
    let conc_no3 = if d.stor > 1.0e-6 { d.no3_st / d.stor } else { 0.0 };
    hd1.no3 = (conc_no3 * d.flo) * area;
    hd1.no3 = hd1.no3.min(d.no3_st * area);
    d.no3_lat = hd1.no3 / area;
    d.no3_st -= d.no3_lat;
    let no3_init = d.no3_st;
    d.no3_st *= a.nloss;
    d.no3_loss = no3_init - d.no3_st;
    d.no3_seep = conc_no3 * d.seep;
    d.no3_seep = d.no3_seep.min(d.no3_st);
    d.no3_st -= d.no3_seep;
    hd2.no3 = d.no3_seep * area;
    d.minp = hin.flo * db.minp / 1000.0;
    hd1.solp = d.minp;
    // Where the return flow goes.
    d.flo_cha = 0.0;
    d.flo_res = 0.0;
    d.flo_ls = 0.0;
    let flo = d.flo;
    let (mut cha, mut res, mut ls) = (0.0_f32, 0.0_f32, 0.0_f32);
    for out in &o.outs {
        if out.ihtyp != 1 {
            continue;
        }
        match out.code.as_str() {
            "cha" | "sdc" => cha += flo * out.frac,
            "res" => res += flo * out.frac,
            "aqu" => ls += flo * out.frac,
            _ => {}
        }
    }
    a.d.flo_cha = cha;
    a.d.flo_res = res;
    a.d.flo_ls = ls;
    o.hd[0] = hd1;
    o.hd[1] = hd2;
}

#[cfg(test)]
mod tests {
    use super::make_plan;

    /// A network of `2^depth - 1` channels in a binary tree, each leaf fed by a routing unit,
    /// with an aquifer taking water from two far-apart routing units and giving it to two
    /// far-apart channels, and one object reading another's water of yesterday. Objects 0 and 1
    /// are HRUs. Returns the command order and each object's sources.
    fn network(depth: u32) -> (Vec<usize>, Vec<Vec<usize>>) {
        let nch = (1usize << depth) - 1;
        let first_leaf = nch / 2;
        let nleaf = nch - first_leaf;
        // 0, 1: HRUs; then the routing units (one per leaf), the aquifer, the channels.
        let ru0 = 2;
        let aq = ru0 + nleaf;
        let ch0 = aq + 1;
        let n = ch0 + nch;
        let mut ins = vec![Vec::new(); n];
        for c in 0..nch {
            if c >= first_leaf {
                ins[ch0 + c].push(ru0 + c - first_leaf);
            } else {
                ins[ch0 + c].push(ch0 + 2 * c + 1);
                ins[ch0 + c].push(ch0 + 2 * c + 2);
            }
        }
        ins[aq].push(ru0);
        ins[aq].push(ru0 + nleaf - 1);
        ins[ch0 + first_leaf].push(aq);
        ins[ch0 + nch - 1].push(aq);
        // The first channel in the order below reads the root's water of yesterday.
        let mut order: Vec<usize> = (ru0..aq).collect();
        order.push(aq);
        for c in (0..nch).rev() {
            order.push(ch0 + c);
        }
        let back = order[nleaf + 1];
        ins[back].push(ch0);
        (order, ins)
    }

    #[test]
    fn a_plan_keeps_every_object_after_what_it_needs() {
        let (order, ins) = network(6);
        let pos = |i: usize| order.iter().position(|&o| o == i);
        for threads in 1..=7 {
            let plan = make_plan(&order, &ins, threads);
            let mut all: Vec<usize> = plan.parts.concat();
            all.sort_unstable();
            let mut want = order.clone();
            want.sort_unstable();
            assert_eq!(all, want, "{threads} threads: each object in one part");
            for p in &plan.parts {
                assert!(p.windows(2).all(|w| pos(w[0]) < pos(w[1])), "a part keeps the command order");
            }
            let last = plan.parts.len() as u32 - 1;
            for &j in &order {
                for &s in &ins[j] {
                    let Some(ps) = pos(s) else { continue };
                    let pj = pos(j).unwrap_or(0);
                    // `before` must run before `after`.
                    let (before, after) = if ps < pj { (s, j) } else { (j, s) };
                    let (pb, pa) = (plan.part_of[before], plan.part_of[after]);
                    assert!(pb == pa || pa == last, "{threads} threads: {after} (part {pa}) needs {before} (part {pb})");
                }
            }
            if threads == 1 {
                assert_eq!(plan.parts, vec![order.clone()]);
            } else {
                assert!(plan.parts.len() > 2, "{threads} threads: parts {:?}", plan.parts.iter().map(Vec::len).collect::<Vec<_>>());
            }
        }
    }
}
