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
        [
            ("flo", self.flo),
            ("dep_wt", self.dep_wt),
            ("stor", self.stor),
            ("rchrg", self.rchrg),
            ("seep", self.seep),
            ("revap", self.revap),
            ("no3_st", self.no3_st),
            ("minp", self.minp),
            ("orgn", self.cbn),
            ("orgp", self.orgn),
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
    /// Objects in command order, without the HRUs (they run first).
    order: Vec<usize>,
    rus: Vec<Ru>,
    /// Routing unit and aquifer by object index.
    ru_of: Vec<Option<usize>>,
    aqu_of: Vec<Option<usize>>,
    aqus: Vec<Aqu>,
    /// Today's routing unit outputs (`ru_d`), by routing unit.
    pub ru_d: Vec<Hyd>,
}

impl Network {
    /// `hyd_connect`, `ru_read_elements`, `time_conc_init`, `dr_ru` and `aqu_initial`. The HRUs'
    /// times of concentration come in `tconc` (h) with their Manning's n and area (km2) in
    /// `ovn_km`. `Ok(None)` when the project has nothing past its HRUs; `Err` names what isn't
    /// ported.
    pub fn new(project: &Project, stations: &[String], tconc: &[f32], ovn_km: &[(f32, f32)]) -> Result<Option<Network>, String> {
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
                ObjectKind::Hru | ObjectKind::RoutingUnit | ObjectKind::Aquifer | ObjectKind::ChannelDeg | ObjectKind::Outlet
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
                    if let Some(t) = to
                        && matches!(obs[t].kind, ObjectKind::Hru | ObjectKind::RoutingUnit)
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
            rus.push(Ru { ob, da_km2, elems });
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
            let dep_wt_db = if iaqdb >= 1 && iaqdb - 1 < iaq {
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

        // The command order (`cmd_next`): objects fire, in passes over the array, once all
        // their inflows and (for routing units) all their elements have.
        let n = obs.len();
        let rcv_tot: Vec<usize> = obs.iter().map(|o| o.ins.len()).collect();
        let mut rcv_sum = vec![0usize; n];
        let mut dfn_sum = vec![0usize; n];
        let mut fired = vec![false; n];
        let mut order = Vec::with_capacity(n);
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
                }
            }
            iord += 1;
        }
        let n_ru = rus.len();
        Ok(Some(Network { obs, order, rus, ru_of, aqu_of, aqus, ru_d: vec![Hyd::default(); n_ru] }))
    }

    /// Sets HRU `j`'s hydrographs for today (0-based).
    pub fn set_hru(&mut self, j: usize, hd: &[Hyd; 5]) {
        if let Some(o) = self.obs.get_mut(j) {
            o.hd.copy_from_slice(hd);
        }
    }

    /// The command loop past the HRUs, with today's weather station PET (mm) for revap.
    pub fn day(&mut self, station_pet: &[f32]) {
        for oi in 0..self.order.len() {
            let icmd = self.order[oi];
            let mut hin = Hyd::default();
            for k in 0..self.obs[icmd].ins.len() {
                let In { from, ihtyp, frac } = self.obs[icmd].ins[k];
                let ht1 = Hyd::scale(frac, self.obs[from].hd[ihtyp - 1]);
                hin = hin.plus(ht1);
            }
            self.obs[icmd].hin = hin;
            match self.obs[icmd].kind {
                ObjectKind::RoutingUnit => {
                    if let Some(iru) = self.ru_of[icmd] {
                        self.ru_control(iru);
                    }
                }
                ObjectKind::Aquifer => {
                    if let Some(iaq) = self.aqu_of[icmd] {
                        let pet = station_pet.get(self.obs[icmd].wst.wrapping_sub(1)).copied().unwrap_or(0.0);
                        self.aqu_1d_control(iaq, pet);
                    }
                }
                _ => {
                    // Outlets, and channels not routed yet.
                    self.obs[icmd].hd[0] = hin;
                }
            }
        }
    }

    /// SWAT+ `ru_control`: the elements' hydrographs, with their delivery ratios (but not on the
    /// recharge), scaled to the share of each in the unit and summed; `ru_d` sums all five.
    fn ru_control(&mut self, iru: usize) {
        let ob = self.rus[iru].ob;
        let mut hd = [Hyd::default(); 5];
        let mut ru_d = Hyd::default();
        for e in &self.rus[iru].elems {
            let src = &self.obs[e.ob];
            let mut ef = e.frac;
            if e.hru && ef < 0.99999 {
                ef = ef * self.rus[iru].da_km2 / (src.area_ha / 100.0);
            }
            for (k, h) in src.hd.iter().enumerate() {
                let ht1 = if k != 1 { h.times(e.dr) } else { *h };
                let ht1 = Hyd::scale(ef, ht1);
                hd[k] = hd[k].plus(ht1);
                ru_d = ru_d.plus(ht1);
            }
        }
        self.obs[ob].hd.copy_from_slice(&hd);
        self.ru_d[iru] = ru_d;
    }

    /// SWAT+ `aqu_1d_control`: recharge, return flow by recession, seepage, revap, and nitrate.
    fn aqu_1d_control(&mut self, iaq: usize, pet: f32) {
        let ob = self.aqus[iaq].ob;
        let hin = self.obs[ob].hin;
        let area = self.obs[ob].area_ha;
        let a = &mut self.aqus[iaq];
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
        for o in &self.obs[ob].outs {
            if o.ihtyp != 1 {
                continue;
            }
            match o.code.as_str() {
                "cha" | "sdc" => cha += flo * o.frac,
                "res" => res += flo * o.frac,
                "aqu" => ls += flo * o.frac,
                _ => {}
            }
        }
        let a = &mut self.aqus[iaq];
        a.d.flo_cha = cha;
        a.d.flo_res = res;
        a.d.flo_ls = ls;
        self.obs[ob].hd[0] = hd1;
        self.obs[ob].hd[1] = hd2;
    }

    /// Today's aquifers, in order: number (1-based), GIS id, name and state.
    pub fn aquifers(&self) -> impl Iterator<Item = (usize, i32, &str, &AquDay)> {
        self.aqus.iter().enumerate().map(|(i, a)| (i + 1, self.obs[a.ob].gis_id, self.obs[a.ob].name.as_str(), &a.d))
    }

    /// Today's routing units: name and `ru_d`.
    pub fn routing_units(&self) -> impl Iterator<Item = (&str, &Hyd)> {
        self.rus.iter().zip(&self.ru_d).map(|(r, h)| (self.obs[r.ob].name.as_str(), h))
    }
}
