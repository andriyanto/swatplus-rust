//! SWAT-DEG channels (`sd_hydsed_init`, `sd_rating_curve`, `sd_channel_control3`,
//! `sd_channel_sediment3`, `ch_rtmusk`, `ch_watqual4`, `ch_temp`): rating curves at four depths,
//! storage routing (variable storage coefficient or Muskingum), flood plain deposition, bank and
//! bed erosion, seepage and evaporation, QUAL2E water quality and the stream temperature.

use swat_model::Project;
use swat_model::table::Table;

use crate::fortran::rpow;
use crate::hru::plant::exp_w;
use crate::hyd::Hyd;

/// A point of a rating curve (`channel_rating_curve_parameters`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rc {
    pub flo_rate: f32,
    pub xsec_area: f32,
    pub surf_area: f32,
    pub dep: f32,
    pub top_wid: f32,
    pub vol: f32,
    pub vol_fp: f32,
    pub vol_ch: f32,
    pub wet_perim: f32,
    pub vel: f32,
    pub ttime: f32,
}

impl Rc {
    /// `rc * const` (`chrc_mult`).
    fn scale(self, k: f32) -> Rc {
        Rc {
            xsec_area: self.xsec_area * k,
            surf_area: self.surf_area * k,
            flo_rate: self.flo_rate * k,
            dep: self.dep * k,
            top_wid: self.top_wid * k,
            vol: self.vol * k,
            vol_ch: self.vol_ch * k,
            vol_fp: self.vol_fp * k,
            wet_perim: self.wet_perim * k,
            vel: self.vel * k,
            ttime: self.ttime * k,
        }
    }

    /// `chrc_interp`.
    fn interp(a: Rc, b: Rc, k: f32) -> Rc {
        let i = |x: f32, y: f32| x + k * (y - x);
        Rc {
            xsec_area: i(a.xsec_area, b.xsec_area),
            surf_area: i(a.surf_area, b.surf_area),
            flo_rate: i(a.flo_rate, b.flo_rate),
            dep: i(a.dep, b.dep),
            top_wid: i(a.top_wid, b.top_wid),
            vol: i(a.vol, b.vol),
            vol_ch: i(a.vol_ch, b.vol_ch),
            vol_fp: i(a.vol_fp, b.vol_fp),
            wet_perim: i(a.wet_perim, b.wet_perim),
            vel: i(a.vel, b.vel),
            ttime: i(a.ttime, b.ttime),
        }
    }
}

/// SWAT+ `qman`: Manning's flow rate (`x1` the area) or velocity (`x1` = 1).
fn qman(x1: f32, x2: f32, x3: f32, x4: f32) -> f32 {
    x1 * rpow(x2, 0.6666) * x4.sqrt() / (x3 + 0.001)
}

/// SWAT+ `theta`: a rate at 20 °C adjusted to temperature `tmp`.
fn theta(r20: f32, thk: f32, tmp: f32) -> f32 {
    r20 * rpow(thk, tmp - 20.0)
}

/// SWAT+ `wq_semianalyt`.
fn wq_semianalyt(tres: f32, tdel: f32, term_m: f32, prock: f32, cprev: f32, cint: f32) -> f32 {
    let help1 = 1.0 / tres - prock;
    let help2 = exp_w(-tdel * help1);
    let help3 = cint / tres + term_m;
    let help4 = help3 / help1;
    let term1 = cprev * help2;
    let term2 = help4 * (1.0 - help2);
    term1 + term2
}

/// SWAT+ `wq_k2m`.
fn wq_k2m(t1: f32, t2: f32, tk: f32, c1: f32, c2: f32) -> f32 {
    let h2 = wq_semianalyt(t1, t2, 0.0, tk, c1, c2);
    let help = exp_w(-t2 / t1);
    (h2 - c1 * help) / (t1 * (1.0 - help)) - c2 / t1
}

/// QUAL2E rates of a channel (`channel_nut_data`, after `ch_read_nut`'s defaults).
#[derive(Clone, Debug, Default)]
struct ChNut {
    rs1: f32,
    rs2: f32,
    rs3: f32,
    rs4: f32,
    rs5: f32,
    rk1: f32,
    rk2: f32,
    rk3: f32,
    rk4: f32,
    bc2: f32,
    bc3: f32,
    bc4: f32,
    igropt: i32,
    ai0: f32,
    ai2: f32,
    ai3: f32,
    ai4: f32,
    ai5: f32,
    ai6: f32,
    mumax: f32,
    rhoq: f32,
    tfact: f32,
    k_l: f32,
    k_n: f32,
    k_p: f32,
    lambda0: f32,
    lambda1: f32,
    lambda2: f32,
    p_n: f32,
}

impl ChNut {
    fn new(t: Option<&Table>, row: Option<usize>) -> ChNut {
        let x = |f: &str, d: f32| row.and_then(|r| t.and_then(|t| t.real(r, f))).unwrap_or(d);
        let pos = |v: f32, d: f32| if v <= 0.0 { d } else { v };
        let igropt = row.and_then(|r| t.and_then(|t| t.int(r, "igropt"))).unwrap_or(2);
        ChNut {
            igropt: if igropt <= 0 { 2 } else { igropt },
            ai0: pos(x("ai0", 50.0), 50.0),
            ai2: pos(x("ai2", 0.015), 0.015),
            ai3: pos(x("ai3", 1.60), 1.60),
            ai4: pos(x("ai4", 2.0), 2.0),
            ai5: pos(x("ai5", 3.5), 3.5),
            ai6: pos(x("ai6", 1.07), 1.07),
            mumax: pos(x("mumax", 2.0), 2.0),
            rhoq: pos(x("rhoq", 2.5), 2.5),
            tfact: pos(x("tfact", 0.3), 0.3),
            k_l: pos(x("k_l", 0.75), 0.75) * 1.0e-3 * 60.0,
            k_n: pos(x("k_n", 0.02), 0.02),
            k_p: pos(x("k_p", 0.025), 0.025),
            lambda0: pos(x("lambda0", 1.0), 1.0),
            lambda1: pos(x("lambda1", 0.03), 0.03),
            lambda2: pos(x("lambda2", 0.054), 0.054),
            p_n: pos(x("p_n", 0.5), 0.5),
            rs1: pos(x("rs1", 1.0), 1.0),
            rs2: pos(x("rs2", 0.05), 0.05),
            rs3: pos(x("rs3", 0.5), 0.5),
            rs4: pos(x("rs4", 0.05), 0.05),
            rs5: pos(x("rs5", 0.05), 0.05),
            rk1: pos(x("rk1", 1.71), 1.71),
            rk2: pos(x("rk2", 1.0), 1.0),
            rk3: x("rk3", 2.0),
            rk4: pos(x("rk4", 0.0), 2.0),
            bc2: pos(x("bc2", 1.1), 1.1),
            bc3: pos(x("bc3", 0.21), 0.21),
            bc4: pos(x("bc4", 0.35), 0.35),
        }
    }
}

/// A day of a channel's water body (`ch_wat_d`): its area (ha), the rain on it, evaporation and
/// seepage (m3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChWat {
    pub area_ha: f32,
    pub precip: f32,
    pub evap: f32,
    pub seep: f32,
}

/// A SWAT-DEG channel (`swatdeg_channel_dynamic` and its state).
#[derive(Clone, Debug)]
struct SdCh {
    chw: f32,
    chd: f32,
    chs: f32,
    chl: f32,
    chn: f32,
    chk: f32,
    cov: f32,
    sinu: f32,
    vcr_coef: f32,
    ch_clay: f32,
    ch_bd: f32,
    n_conc: f32,
    p_conc: f32,
    p_bio: f32,
    pk_rto: f32,
    n_sol_part: f32,
    p_sol_part: f32,
    n_dep_enr: f32,
    p_dep_enr: f32,
    wash_bed_fr: f32,
    /// The database row's width and depth (`sd_chd`) at this channel's number, as `ch_watqual4`
    /// and `ch_temp` read them.
    db_chw: f32,
    db_chd: f32,
    nut: ChNut,
    elev: [Rc; 4],
    in1: Rc,
    in2: Rc,
    out1: Rc,
    out2: Rc,
    msk_c: [f32; 3],
    in1_vol: f32,
    out1_vol: f32,
    tot_stor: Hyd,
    ch_stor: Hyd,
    fp_stor: Hyd,
    /// Velocity and travel time of the last day with inflow (`sd_ch_vel`).
    vel: f32,
    rttime: f32,
    wat: ChWat,
    ch_in: Hyd,
    ch_out: Hyd,
    /// The water temperature printed for this channel (corrected mode: its own).
    wtemp: f32,
}

/// What a channel's day needs from outside: its station's weather, the station's generator day
/// length and measured temperature gage, and the HRUs' water for the stream temperature.
pub struct ChanCtx<'a> {
    pub weat: &'a crate::climate::Weat,
    pub daylth: f32,
    pub tgage: Option<&'a crate::climate::Gage>,
    /// `time%yrs` and `time%day`.
    pub yrs: i32,
    pub jday: i32,
    /// Per HRU: snowmelt, surface runoff, lateral flow and water yield (mm, `hwb_d`).
    pub hru_wb: &'a [[f32; 4]],
}

/// A channel's inflows as `ch_temp` reads them: whether each comes from a SWAT-DEG channel, its
/// hydrograph (`hin_d`), and the routing units among them.
pub struct ChanInflows<'a> {
    pub from_chandeg: &'a [bool],
    pub hin_d: &'a [Hyd],
    pub rus: &'a [usize],
}

/// Basin settings the channels use.
#[derive(Clone, Copy, Debug)]
struct Basin {
    rte: i32,
    qual2e: i32,
    evrch: f32,
    scoef: f32,
    fixes: bool,
}

/// The SWAT-DEG channels.
#[derive(Clone, Debug)]
pub struct Channels {
    chs: Vec<SdCh>,
    b: Basin,
    /// `ch_watqual4`'s algal growth rate, kept from the last channel when algae are too dense to
    /// grow (`gra` has the save attribute).
    gra: f32,
    /// `wtemp`: one value for all channels, the last one set.
    pub wtemp: f32,
    /// Routing units' elements (HRU numbers, 0-based) and the global element table's types, as
    /// `ch_temp` reads them.
    ru_hrus: Vec<Vec<(usize, f32)>>,
    ru_elem_is_hru: Vec<bool>,
}

impl Channels {
    /// `sd_channel_read` and `sd_hydsed_init` for the channels in `chandeg.con` order (their data
    /// rows in `props`). `Err` names what isn't ported.
    pub fn new(project: &Project, props: &[i32], fixes: bool) -> Result<Channels, String> {
        let codes = &project.codes;
        if codes.lapse == 1 {
            return Err("channels with lapse rates".into());
        }
        if project.ch_temp_file {
            return Err("temperature.cha".into());
        }
        if project.time.step > 1 {
            return Err("sub-daily channel routing".into());
        }
        let p = |n: &str, d: f32| project.params.get(n).unwrap_or(d);
        let b = Basin { rte: codes.rte, qual2e: codes.qual2e, evrch: p("evrch", 0.6), scoef: p("scoef", 1.0), fixes };
        let hs = project.hyd_sed_lte.as_ref().ok_or("channels without hyd-sed-lte.cha")?;
        let nut_t = project.ch_nut.as_ref();
        let find = |t: Option<&Table>, name: &str| t.and_then(|t| t.find(name));
        let mut chs = Vec::with_capacity(props.len());
        for (k, &idat) in props.iter().enumerate() {
            let ich = k + 1;
            let dat = project.sd_channel_data.get(idat.max(0) as usize).cloned().unwrap_or_default();
            let idb = find(Some(hs), &dat.hyd).unwrap_or(0);
            let r = |f: &str| hs.real(idb, f).unwrap_or(0.0);
            // Data lookups by the channel's own number (`sd_dat(ich)`, `sd_chd(ich)`).
            let own = project.sd_channel_data.get(ich).cloned().unwrap_or_default();
            let nut = ChNut::new(nut_t, Some(find(nut_t, &own.nut).unwrap_or(0)));
            let db_chw = hs.real(ich - 1, "chw").unwrap_or(0.0);
            let db_chd = hs.real(ich - 1, "chd").unwrap_or(0.0);
            // The initial water must be none (`no_init`) so far.
            let init_t = project.ch_init.as_ref();
            let iinit = find(init_t, &dat.init).unwrap_or(0);
            let om_name = init_t.and_then(|t| t.text(iinit, "org_min")).unwrap_or("");
            if let Some(om) = project.om_water.as_ref()
                && let Some(r) = om.find(om_name)
                && om.real(r, "flo").unwrap_or(0.0) > 0.0
            {
                return Err(format!("channel {}: initial water {om_name}", dat.name));
            }
            let mut c = SdCh {
                chw: r("chw"),
                chd: r("chd"),
                chs: r("chs"),
                chl: r("chl"),
                chn: r("chn"),
                chk: r("chk"),
                cov: r("cov"),
                sinu: r("sinu"),
                vcr_coef: r("vcr_coef"),
                ch_clay: r("ch_clay"),
                ch_bd: r("ch_bd"),
                n_conc: r("n_conc"),
                p_conc: r("p_conc"),
                p_bio: r("p_bio"),
                // sed_nut.cha isn't read: `swatdeg_sednut_data`'s defaults.
                pk_rto: 1.0,
                n_sol_part: 0.01,
                p_sol_part: 0.01,
                n_dep_enr: 0.5,
                p_dep_enr: 0.5,
                wash_bed_fr: 0.1,
                db_chw,
                db_chd,
                nut,
                elev: [Rc::default(); 4],
                in1: Rc::default(),
                in2: Rc::default(),
                out1: Rc::default(),
                out2: Rc::default(),
                msk_c: [0.0; 3],
                in1_vol: 0.0,
                out1_vol: 0.0,
                tot_stor: Hyd::default(),
                ch_stor: Hyd::default(),
                fp_stor: Hyd::default(),
                vel: 0.0,
                rttime: 0.0,
                wat: ChWat::default(),
                ch_in: Hyd::default(),
                ch_out: Hyd::default(),
                wtemp: 0.0,
            };
            if c.chs < 1.0e-9 {
                c.chs = 0.000001;
            }
            if c.chn < 0.01 {
                c.chn = 0.01;
            }
            if c.sinu < 1.05 {
                c.sinu = 1.05;
            }
            if c.chl <= 1.0e-3 {
                return Err(format!("channel {}: length {} km (pass-through channels)", dat.name, c.chl));
            }
            let mut fps = r("fps");
            if fps > c.chs {
                fps = c.chs;
            }
            if fps <= 1.0e-6 {
                fps = 0.00001;
            }
            let fpn = r("fpn");
            let mut chss = r("chss");
            // sd_rating_curve
            let mut bw = c.chw - 2.0 * c.chd * chss;
            if bw <= 0.0 {
                bw = 0.5 * c.chw;
                bw = bw.max(0.0);
                chss = (c.chw - bw) / (2.0 * c.chd);
            }
            let (mut p_bf, mut a_bf, mut vol_bf) = (0.0_f32, 0.0_f32, 0.0_f32);
            // SWAT+ takes the side slope as horizontal over vertical for the widths but as
            // vertical over horizontal for the wetted perimeter, and the area as `b d + d / z`;
            // corrected, a trapezoid: `b d + z d²` and `b + 2 d √(1 + z²)`.
            let geom = |dep: f32| -> (f32, f32) {
                if fixes {
                    (bw + 2.0 * dep * (1.0 + chss * chss).sqrt(), bw * dep + chss * dep * dep)
                } else {
                    (bw + 2.0 * (dep * dep * (1.0 + 1.0 / (chss * chss))).sqrt(), bw * dep + dep / chss)
                }
            };
            for i_dep in 0..2 {
                let dep = if i_dep == 0 { 0.1 * c.chd } else { c.chd };
                let (pp, a) = geom(dep);
                let rh = a / pp;
                let e = &mut c.elev[i_dep];
                e.dep = dep;
                e.wet_perim = pp;
                e.xsec_area = a;
                e.top_wid = bw + 2.0 * dep * chss;
                e.surf_area = e.top_wid * c.chl;
                e.vol = a * c.chl * 1000.0;
                e.vol_ch = e.vol;
                e.vol_fp = 0.0;
                e.flo_rate = qman(a, rh, c.chn, c.chs);
                e.vel = qman(1.0, rh, c.chn, c.chs);
                e.ttime = c.chl / (3.6 * e.vel);
                if i_dep == 1 {
                    p_bf = pp;
                    a_bf = a;
                    vol_bf = e.vol_ch;
                }
            }
            for i_dep in 0..2 {
                let frac_abov = if i_dep == 0 { 0.2 } else { 1.0 };
                let dep = frac_abov * c.chd;
                let pp = p_bf + 2.0 * (dep * dep * (1.0 + 1.0 / (fps * fps))).sqrt();
                // Above bankfull the water spans the channel's top width, not its bottom.
                let a = if fixes { a_bf + c.chw * dep + dep * dep / fps } else { a_bf + bw * dep + dep * dep / fps };
                let rh = a / pp;
                let e = &mut c.elev[i_dep + 2];
                e.dep = (1.0 + frac_abov) * c.chd;
                e.wet_perim = pp;
                e.xsec_area = a;
                e.top_wid = c.chw + 2.0 * dep / fps;
                e.surf_area = e.top_wid * c.chl;
                e.vol_ch = vol_bf;
                e.vol_fp = e.top_wid * dep * c.chl * 1000.0;
                e.vol = vol_bf + e.vol_fp;
                e.flo_rate = qman(a, rh, fpn, c.chs);
                e.vel = qman(1.0, rh, fpn, c.chs);
                e.ttime = c.chl / (3.6 * e.vel);
            }
            // Muskingum coefficients (one step a day: `msk%nsteps` 1)
            let mut stor = [0.0_f32; 2];
            for (i_dep, s) in stor.iter_mut().enumerate() {
                let dep = if i_dep == 0 { 0.1 * c.chd } else { c.chd };
                let (pp, a) = geom(dep);
                let rh = a / pp;
                let vel = qman(1.0, rh, c.chn, c.chs);
                let celerity = vel * 5.0 / 3.0;
                *s = c.chl / (3.6 * celerity);
            }
            let (co1, co2, x) = (p("msk_co1", 0.75), p("msk_co2", 0.25), p("msk_x", 0.2));
            let msk1 = co1 / (co1 + co2);
            let msk2 = co2 / (co1 + co2);
            let xkm = stor[1] * msk1 + stor[0] * msk2;
            let det = 24.0_f32;
            let denom = 2.0 * xkm * (1.0 - x) + det;
            let mut c1 = (det - 2.0 * xkm * x) / denom;
            c1 = c1.max(0.0);
            let c2 = (det + 2.0 * xkm * x) / denom;
            let c3 = (2.0 * xkm * (1.0 - x) - det) / denom;
            let sumc = c1 + c2 + c3;
            c.msk_c = [c1 / sumc, c2 / sumc, c3 / sumc];
            // Initial storage of no water: the rating curve at depth 0 (`rcurv_interp_dep`)
            // keeps the shallowest point's travel time.
            let rc0 = Rc { ttime: c.elev[0].ttime, ..c.elev[0].scale(0.0) };
            c.in1 = rc0;
            c.out1 = rc0;
            chs.push(c);
        }
        // ch_temp's view of the routing units.
        let ru_hrus = project
            .ru_defs
            .iter()
            .map(|d| {
                d.elems.iter().filter_map(|&ie| project.ru_elements.get(ie).map(|e| (e.obtypno.max(1) as usize - 1, e.frac))).collect()
            })
            .collect();
        let ru_elem_is_hru = project.ru_elements.iter().map(|e| e.obtyp == "hru").collect();
        Ok(Channels { chs, b, gra: 0.0, wtemp: 0.0, ru_hrus, ru_elem_is_hru })
    }

    /// `rcurv_interp_flo` on channel `ich`'s rating curve.
    fn interp_flo(&self, ich: usize, flo_rate: f32) -> Rc {
        let e = &self.chs[ich].elev;
        for k in 0..4 {
            if flo_rate < e[k].flo_rate {
                if k == 0 {
                    let rto = flo_rate / e[0].flo_rate;
                    let mut r = e[0].scale(rto);
                    r.ttime = e[0].ttime;
                    return r;
                }
                let rto = (flo_rate - e[k - 1].flo_rate) / (e[k].flo_rate - e[k - 1].flo_rate);
                return Rc::interp(e[k - 1], e[k], rto);
            }
            if k == 3 {
                let rto = 1.0 + (flo_rate - e[3].flo_rate) / e[3].flo_rate;
                let mut r = e[3].scale(rto);
                r.ttime /= rto;
                return r;
            }
        }
        Rc::default()
    }

    /// The day's output of channel `ich` (0-based): storage, inflow and outflow (flow in m3/s),
    /// and its water body.
    pub fn output(&self, ich: usize) -> (&Hyd, &Hyd, &Hyd, &ChWat) {
        let c = &self.chs[ich];
        (&c.ch_stor, &c.ch_in, &c.ch_out, &c.wat)
    }

    /// The water temperature printed for channel `ich`: SWAT+ prints one value for all channels,
    /// the last one set (`wtemp`); corrected, the channel's own stream temperature.
    pub fn wtemp_of(&self, ich: usize) -> f32 {
        if self.b.fixes { self.chs[ich].wtemp } else { self.wtemp }
    }

    /// `sd_channel_control3` for channel `ich` (0-based) with inflow `hin`, its drainage area
    /// (ha) and inflows; returns the outflow hydrograph (`ob%hd(1)`).
    pub fn control(&mut self, ich: usize, hin: Hyd, area_ha: f32, ins: &ChanInflows, cx: &ChanCtx) -> Hyd {
        let w = cx.weat;
        let mut ht1 = hin;
        ht1.temp = 5.0 + 0.75 * w.tave;
        self.wtemp = 5.0 + 0.75 * w.tave;
        let mut tsin = ht1.flo;
        self.chs[ich].ch_in = Hyd { flo: ht1.flo / 86400.0, ..ht1 };

        // sd_channel_sediment3
        let mut fp_dep = Hyd::default();
        let mut ch_dep = Hyd::default();
        let mut bank_ero = Hyd::default();
        self.chs[ich].wat.precip = 0.0;
        if ht1.flo > 1.0e-6 {
            let c = &self.chs[ich];
            let (bankfull_flo, fp_inun_days, d50, bed_exp) = (1.5_f32, 1.0_f32, 1.0_f32, 1.0_f32);
            let pk_rto = c.pk_rto * (1.0 + 2.66 * rpow(area_ha / 100.0, -0.3));
            let peakrate = pk_rto * ht1.flo / 86400.0;
            let rc = self.interp_flo(ich, peakrate);
            let vel = rc.vel;
            let c = &mut self.chs[ich];
            let rttime = c.chl / (3.6 * vel);
            c.vel = vel;
            c.rttime = rttime;
            c.wat.area_ha = c.chl * c.chw / 10.0;
            let precip = 10.0 * w.precip * c.wat.area_ha;
            c.wat.precip = precip;
            let rto = precip / ht1.flo;
            tsin *= 1.0 + rto;
            ht1.flo += precip;
            let bf_flow = bankfull_flo * c.elev[1].flo_rate;
            if peakrate > bf_flow {
                let fp_m2 = 5.0 * c.chw * c.chl * 1000.0;
                let exp_co = 0.0001 * fp_m2 / rc.flo_rate;
                let ave_rate = ht1.flo / 86400.0;
                let mut trap_eff = fp_inun_days * (rc.flo_rate / ave_rate) * (1.0 - (-exp_co).exp());
                trap_eff = trap_eff.min(1.0);
                fp_dep.sed = trap_eff * ht1.sed;
                fp_dep.orgn = trap_eff * c.n_dep_enr * ht1.orgn;
                fp_dep.sedp = trap_eff * c.p_dep_enr * ht1.sedp;
                ht1 = ht1.minus(fp_dep);
            }
            // bank erosion
            let clay = c.ch_clay;
            let mut cohesion = -87.1 + (42.82 * clay) - (0.261 * (clay * clay)) + (0.029 * rpow(clay, 3.0));
            cohesion = cohesion.max(1000.0);
            let veg = (-5.0 * c.chd).exp() * 4500.0 * c.cov;
            let bd_fac = (0.03924 * c.ch_bd * 1000.0 - 1000.0).max(0.001);
            let cohes_fac = 0.021 * cohesion + veg;
            let mut vel_cr = (2200.0 * c.chd).log10() * rpow(0.0004 * (bd_fac + cohes_fac), 0.5);
            vel_cr *= c.vcr_coef;
            let rad_curv = ((12.0 * c.chw) * rpow(c.sinu, 1.5)) / (13.0 * rpow(c.sinu - 1.0, 0.5));
            let vel_bend = vel * (1.0 / rad_curv + 1.0);
            let vel_rch = 0.33 * vel_bend + 0.66 * vel;
            let ebank_m = if vel_rch > vel_cr {
                let dur_scale = 0.0001 * rpow(area_ha / 100.0, -0.0858);
                let v_vc = dur_scale * c.chw * (1.0 / (1.0 + exp_w(-4.0 * (vel_rch / vel_cr - 1.0))));
                let m_exhaust = 0.0002 * c.chw;
                1.0 / (1.0 / v_vc + 1.0 / m_exhaust)
            } else {
                0.0
            };
            let arc_len = 0.25 * c.chl;
            let ebank_t = 1000.0 * ebank_m * c.chd * arc_len * c.ch_bd;
            bank_ero.sed = ebank_t;
            bank_ero.orgn = bank_ero.sed * c.n_conc / 1000.0;
            bank_ero.sedp = (1.0 - c.p_bio) * bank_ero.sed * c.p_conc / 1000.0;
            bank_ero.solp = c.p_bio * bank_ero.sed * c.p_conc / 1000.0;
            let rto = bank_ero.flo / ht1.flo;
            tsin *= 1.0 - rto;
            ht1 = ht1.plus(bank_ero);
            ch_dep.sed = c.wash_bed_fr * bank_ero.sed;
            ch_dep.orgn = c.n_dep_enr * c.wash_bed_fr * bank_ero.orgn;
            ch_dep.sedp = c.p_dep_enr * c.wash_bed_fr * bank_ero.sedp;
            ht1 = ht1.minus(ch_dep);
            let _ = (d50, bed_exp);
        }

        // ch_rtmusk (one step a day)
        let mut ht2 = Hyd::default();
        let (mut trans_loss, mut evap) = (0.0_f32, 0.0_f32);
        {
            self.chs[ich].wat.evap = 0.0;
            self.chs[ich].wat.seep = 0.0;
            let mut inflo = 0.0_f32;
            if ht1.flo > 1.0e-6 {
                inflo = tsin;
                let mut rto = inflo / ht1.flo;
                rto = rto.max(0.0);
                rto = rto.min(1.0);
                let c = &mut self.chs[ich];
                c.tot_stor = c.tot_stor.plus(Hyd::scale(rto, ht1));
            }
            let in2 = self.interp_flo(ich, inflo / 86400.0);
            self.chs[ich].in2 = in2;
            if self.chs[ich].tot_stor.flo < 1.0e-6 {
                let c = &mut self.chs[ich];
                c.in1 = Rc::default();
                c.out1 = Rc::default();
                c.in1_vol = 0.0;
                c.out1_vol = 0.0;
            } else {
                let dthr = 24.0_f32;
                let dts = 86400.0_f32;
                let c = &mut self.chs[ich];
                let mut outflo = if self.b.rte == 1 {
                    let o = c.msk_c[0] * inflo + c.msk_c[1] * c.in1_vol + c.msk_c[2] * c.out1_vol;
                    let o = o.min(c.tot_stor.flo).max(0.0);
                    c.in1_vol = inflo;
                    c.out1_vol = o;
                    o
                } else {
                    let mut scoef = self.b.scoef * 2.0 * dthr / (2.0 * c.out1.ttime + dthr);
                    scoef = scoef.min(1.0);
                    scoef * c.tot_stor.flo
                };
                if !outflo.is_finite() {
                    outflo = 0.0;
                }
                let out2 = self.interp_flo(ich, outflo / dts);
                let c = &mut self.chs[ich];
                c.out2 = out2;
                let mut rto = outflo / c.tot_stor.flo;
                rto = rto.min(1.0);
                ht2 = ht2.plus(Hyd::scale(rto, c.tot_stor));
                c.tot_stor = Hyd::scale(1.0 - rto, c.tot_stor);
                c.in1 = c.in2;
                c.out1 = c.out2;
                if c.tot_stor.flo > c.elev[1].vol_ch {
                    let rto = (c.tot_stor.flo - c.elev[1].vol_ch) / c.tot_stor.flo;
                    c.fp_stor = Hyd::scale(rto, c.tot_stor);
                    c.ch_stor = Hyd::scale(1.0 - rto, c.tot_stor);
                } else {
                    c.ch_stor = c.tot_stor;
                    c.fp_stor = Hyd::default();
                }
                c.tot_stor = c.ch_stor.plus(c.fp_stor);
            }
            let c = &mut self.chs[ich];
            if c.ch_stor.flo > 1.0e-6 {
                trans_loss = c.chk * c.chl * c.chw * 24.0;
                trans_loss = trans_loss.min(c.ch_stor.flo);
                let rto = trans_loss / c.ch_stor.flo;
                c.ch_stor = Hyd::scale(1.0 - rto, c.ch_stor);
            }
            c.wat.seep = trans_loss;
            if c.ch_stor.flo > 1.0e-6 {
                let topw = 1000.0 * c.chl * c.chw;
                evap = self.b.evrch * w.pet * topw / 1000.0;
                evap = evap.min(c.ch_stor.flo);
                let rto = evap / c.ch_stor.flo;
                c.ch_stor.flo *= 1.0 - rto;
            }
            c.wat.evap = evap;
            c.tot_stor = c.ch_stor.plus(c.fp_stor);
        }

        if ht1.flo > 1.0e-6 {
            // The maximum flow depth uses `ht3%flo`, never set: the curve at no flow. Corrected,
            // the inflow's rate.
            let rc_ht3 = self.interp_flo(ich, if self.b.fixes { ht1.flo / 86400.0 } else { 0.0 });
            self.ch_watqual4(ich, &ht1, &mut ht2, rc_ht3, cx);
            if self.b.qual2e == 1 {
                let rc = self.interp_flo(ich, ht1.flo / 86400.0);
                let c = &self.chs[ich];
                let conc_chng = 1.0 - (-c.n_sol_part * rc.ttime).exp();
                let mut orgn = conc_chng * ht1.orgn;
                orgn = ht1.no3.min(orgn);
                ht2.orgn = ht1.orgn + orgn;
                ht2.no3 = ht1.no3 - orgn;
                let conc_chng = 1.0 - (-c.p_sol_part * rc.ttime).exp();
                let mut sedp = conc_chng * ht1.sedp;
                sedp = ht1.solp.min(sedp);
                ht2.sedp = ht1.sedp + sedp;
                ht2.solp = ht1.solp - sedp;
            }
        }
        let mut hd1 = ht2;
        let tw = self.ch_temp(ich, hd1, ht2, ins, cx);
        // SWAT+ sets the outflow before computing the stream temperature, so only the storage
        // gets it; corrected, the outflow carries it.
        if self.b.fixes {
            hd1.temp = tw;
        }
        self.chs[ich].ch_out = Hyd { flo: hd1.flo / 86400.0, ..hd1 };
        hd1
    }

    /// SWAT+ `ch_watqual4`: QUAL2E transformations of the inflow's concentrations, giving the
    /// outflow's nutrients, algae, CBOD and oxygen (as masses of the inflow's water).
    fn ch_watqual4(&mut self, ich: usize, ht1: &Hyd, ht2: &mut Hyd, rc_prev: Rc, cx: &ChanCtx) {
        let (thgra, thrho, thrs1, thrs2, thrs3, thrs4, thrs5) =
            (1.047_f32, 1.047_f32, 1.024_f32, 1.074_f32, 1.074_f32, 1.024_f32, 1.024_f32);
        let (_thbc1, thbc2, thbc3, thbc4) = (1.083_f32, 1.047_f32, 1.047_f32, 1.047_f32);
        let (thrk1, thrk2, thrk3, thrk4) = (1.047_f32, 1.024_f32, 1.024_f32, 1.060_f32);
        let w = cx.weat;
        let mut tday = rc_prev.ttime / 24.0;
        tday = tday.min(1.0);
        let rt_delt = 1.0_f32;
        let f = ht1.flo;
        let mut h3 = Hyd {
            orgn: 1000.0 * ht1.orgn / f,
            sedp: 1000.0 * ht1.sedp / f,
            no3: 1000.0 * ht1.no3 / f,
            solp: 1000.0 * ht1.solp / f,
            chla: 1000.0 * ht1.chla / f,
            nh3: 1000.0 * ht1.nh3 / f,
            no2: 1000.0 * ht1.no2 / f,
            cbod: 1000.0 * ht1.cbod / f,
            dox: 1000.0 * ht1.dox / f,
            ..Hyd::default()
        };
        let mut wtmp = 5.0 + 0.75 * w.tave;
        if wtmp <= 0.0 {
            wtmp = 0.1;
        }
        ht2.temp = wtmp;
        let c = &self.chs[ich];
        let n = &c.nut;
        let ben_area = c.chw * c.chl;
        let rs2_s = theta(n.rs2, thrs2, wtmp) * ben_area;
        let rs3_s = theta(n.rs3, thrs3, wtmp) * ben_area;
        let rk4_s = theta(n.rk4, thrk4, wtmp) * ben_area;
        let rc = self.interp_flo(ich, ht1.flo / 86400.0);
        let rchdep = rc.dep;
        let c = &self.chs[ich];
        let n = &c.nut;
        if ht1.flo > 0.0 && rchdep > 0.0 {
            let mut disoxin = h3.dox - rk4_s / (ht1.flo * 1000.0);
            disoxin = disoxin.max(0.0);
            let _ = disoxin;
            let _dispin = h3.solp + rs2_s / (ht1.flo * 1000.0);
            let ammoin = h3.nh3 + rs3_s / (ht1.flo * 1000.0);
            let cinn = c.ch_stor.nh3 + c.ch_stor.no3;
            let tk = wtmp + 273.15;
            let ww = -139.3441_f32 + (1.575701e05 / tk);
            let xx = 6.642308e07 / (tk * tk);
            let yy = 1.2438e10 / (tk * tk * tk);
            let zz = 8.621949e11 / ((tk * tk) * (tk * tk));
            let mut soxy = (ww - xx + yy - zz).exp();
            if soxy < 1.0e-6 {
                soxy = 0.0;
            }
            if h3.dox < 0.001 {
                h3.dox = 0.001;
            }
            if h3.dox > 30.0 {
                h3.dox = 30.0;
            }
            let _cordo = 1.0 - (-0.6 * h3.dox).exp();
            let mut algcon = 1000.0 * h3.chla / n.ai0;
            let algin = 1000.0 * h3.chla / n.ai0;
            let mut lambda = if n.ai0 * algcon > 1.0e-6 {
                n.lambda0 + (n.lambda1 * n.ai0 * algcon) + n.lambda2 * rpow(n.ai0 * algcon, 0.66667)
            } else {
                n.lambda0
            };
            if lambda > n.lambda0 {
                lambda = n.lambda0;
            }
            let fnn = cinn / (cinn + n.k_n);
            let fpp = h3.solp / (h3.solp + n.k_p);
            let algi = if cx.daylth > 0.0 { w.solrad * n.tfact / cx.daylth } else { 0.00001 };
            let fl_1 = (1.0 / (lambda * rchdep)) * ((n.k_l + algi) / (n.k_l + algi * (-lambda * rchdep).exp())).ln();
            let fll = 0.92 * (cx.daylth / 24.0) * fl_1;
            if algcon < 5000.0 {
                self.gra = match n.igropt {
                    1 => n.mumax * fll * fnn * fpp,
                    2 => n.mumax * fll * fnn.min(fpp),
                    3 => {
                        if fnn > 1.0e-6 && fpp > 1.0e-6 {
                            n.mumax * fll * 2.0 / ((1.0 / fnn) + (1.0 / fpp))
                        } else {
                            0.0
                        }
                    }
                    _ => self.gra,
                };
            }
            let gra = self.gra;
            let mut factk = theta(gra, thgra, wtmp) - theta(n.rhoq, thrho, wtmp);
            algcon = 1000.0 * h3.chla / n.ai0;
            let alg_m1 = wq_semianalyt(tday, rt_delt, 0.0, factk, algcon, algin);
            let alg_m = wq_semianalyt(tday, rt_delt, 0.0, factk, algcon, algin);
            let alg_m2 = alg_m - alg_m1;
            let _f1 = n.p_n * h3.nh3 / (n.p_n * h3.nh3 + (1.0 - n.p_n) * h3.no3 + 1.0e-6);
            let mut alg_set = 0.0;
            if rchdep > 0.001 {
                alg_set = theta(n.rs1, thrs1, wtmp) / rchdep;
            }
            let mut algcon_out = wq_semianalyt(tday, rt_delt, alg_m, -alg_set, algcon, algin);
            if algcon_out < 1.0e-6 {
                algcon_out = 0.0;
            }
            if algcon_out > 5000.0 {
                algcon_out = 5000.0;
            }
            h3.chla = algcon_out * n.ai0 / 1000.0;
            let rk1_k = -theta(n.rk1, thrk1, wtmp);
            let rk1_m = wq_k2m(tday, rt_delt, rk1_k, h3.cbod, h3.cbod);
            let mut rk3_k = 0.0;
            if rchdep > 0.001 {
                rk3_k = -theta(n.rk3, thrk3, wtmp) / rchdep;
            }
            let mut factm = rk1_m;
            factk = rk3_k;
            h3.cbod = wq_semianalyt(tday, rt_delt, factm, factk, h3.cbod, h3.cbod);
            let mut bc3_k = theta(n.bc3, thbc3, wtmp);
            bc3_k *= 2.0;
            let mut rs4_k = 0.0;
            if rchdep > 0.001 {
                rs4_k = theta(n.rs4, thrs4, wtmp) / rchdep;
            }
            let bc3_m = wq_k2m(tday, rt_delt, -bc3_k, h3.orgn, h3.orgn);
            factk = -rs4_k;
            factm = bc3_m;
            h3.orgn = wq_semianalyt(tday, rt_delt, factm, factk, h3.orgn, h3.orgn);
            if h3.orgn < 0.0 {
                h3.orgn = 0.0;
            }
            let rk2_m = theta(n.rk2, thrk2, wtmp) * soxy;
            let rk2_k = theta(n.rk2, thrk2, wtmp);
            let _alg_m_o2 = n.ai4 * alg_m2 + n.ai3 * alg_m1;
            factk = -rk2_k;
            let bc2_k = -theta(n.bc2, thbc2, wtmp);
            let bc1_m = wq_k2m(tday, rt_delt, factk, h3.nh3, ammoin);
            let bc2_m = wq_k2m(tday, rt_delt, bc2_k, h3.no2, h3.no2);
            factm = rk1_m + rk2_m - rs4_k + bc1_m * n.ai5 + bc2_m * n.ai6;
            h3.dox = wq_semianalyt(tday, rt_delt, factm, factk, h3.dox, h3.dox);
            if h3.dox < 0.0 {
                h3.dox = 0.0;
            }
            factm = bc1_m - bc3_m;
            h3.nh3 = wq_semianalyt(tday, rt_delt, factm, 0.0, h3.nh3, ammoin);
            if h3.nh3 < 1.0e-6 {
                h3.nh3 = 0.0;
            }
            factm = -bc1_m + bc2_m;
            h3.no2 = wq_semianalyt(tday, rt_delt, factm, 0.0, h3.no2, h3.no2);
            if h3.no2 < 1.0e-6 {
                h3.no2 = 0.0;
            }
            factm = -bc2_m;
            h3.no3 = wq_semianalyt(tday, rt_delt, factm, 0.0, h3.no3, h3.no3);
            if h3.no3 < 1.0e-6 {
                h3.no3 = 0.0;
            }
            let bc4_k = theta(n.bc4, thbc4, wtmp);
            let bc4_m = wq_k2m(tday, rt_delt, -bc4_k, h3.sedp, h3.sedp);
            let mut rs5_k = 0.0;
            if rchdep > 0.001 {
                rs5_k = theta(n.rs5, thrs5, wtmp) / rchdep;
            }
            factk = -rs5_k;
            factm = bc4_m;
            h3.sedp = wq_semianalyt(tday, rt_delt, factm, factk, h3.sedp, h3.sedp);
            if h3.sedp < 1.0e-6 {
                h3.sedp = 0.0;
            }
            let xx = theta(n.bc4, thbc4, wtmp) * h3.sedp;
            let yy = theta(n.rs2, thrs2, wtmp) / c.db_chd;
            let zz = n.ai2 * theta(gra, thgra, wtmp) * algin;
            h3.solp += (xx + yy - zz) * tday;
            if h3.solp < 1.0e-6 {
                h3.solp = 0.0;
            }
            let k = ht1.flo / 1000.0;
            let _ = k;
            ht2.orgn = h3.orgn * ht1.flo / 1000.0;
            ht2.sedp = h3.sedp * ht1.flo / 1000.0;
            ht2.no3 = h3.no3 * ht1.flo / 1000.0;
            ht2.solp = h3.solp * ht1.flo / 1000.0;
            ht2.chla = h3.chla * ht1.flo / 1000.0;
            ht2.nh3 = h3.nh3 * ht1.flo / 1000.0;
            ht2.no2 = h3.no2 * ht1.flo / 1000.0;
            ht2.cbod = h3.cbod * ht1.flo / 1000.0;
            ht2.dox = h3.dox * ht1.flo / 1000.0;
        } else {
            *ht2 = Hyd::default();
            self.chs[ich].ch_stor = Hyd::default();
        }
    }

    /// SWAT+ `ch_temp`: the stream temperature, which ends up only in the channel's storage
    /// (`ch_stor%temp`). `hd1` is the outflow (`ob%hd(1)`), `ht2` the routed outflow.
    fn ch_temp(&mut self, ich: usize, hd1: Hyd, ht2: Hyd, ins: &ChanInflows, cx: &ChanCtx) -> f32 {
        let w = cx.weat;
        let tw_def = 5.0 + 0.75 * w.tave;
        let mut ht1 = hd1;
        if self.b.fixes {
            // Corrected: the temperature of all upstream channels' water mixed by flow (SWAT+
            // looks only at the last inflow and the one before it, whatever that is).
            let (mut tf, mut tt) = (0.0_f32, 0.0_f32);
            for (k, &is_ch) in ins.from_chandeg.iter().enumerate() {
                if is_ch {
                    let f = ins.hin_d[k].flo / 86400.0;
                    tf += f;
                    tt += ins.hin_d[k].temp * f;
                }
            }
            ht1.temp = if tf < 1.0e-6 { 5.0 + 0.75 * w.tave } else { tt / tf };
        }
        for (k, &is_ch) in ins.from_chandeg.iter().enumerate().filter(|_| !self.b.fixes) {
            if is_ch {
                // `hin_d(in-1)`: the inflow before (SWAT+ reads out of bounds for the first).
                let prev = if k > 0 { ins.hin_d[k - 1] } else { Hyd::default() };
                let cur = ins.hin_d[k];
                let (t1f, t2f) = (prev.flo / 86400.0, cur.flo / 86400.0);
                let tf = t1f + t2f;
                ht1.temp = if tf < 1.0e-6 { 5.0 + 0.75 * w.tave } else { (prev.temp * t1f + cur.temp * t2f) / tf };
            } else {
                ht1.temp = 5.0 + 0.75 * w.tave;
            }
        }
        // w_temp(0) defaults (no temperature.cha)
        let (sno_lag0, gw_lag0, lat_lag0, surf_lag0) = (2.0_f32, 250.0_f32, 10.0_f32, 5.0_f32);
        let (surf_lag_coef, gw_lag_coef, lat_lag_coef) = (0.75_f32, 0.75_f32, 0.75_f32);
        let (sno_coef, gw_coef, sur_lat_coef) = (1.0_f32, 1.0_f32, 1.0_f32);
        let (hex_coef1, hex_coef2, ssff) = (0.75_f32, 1.5_f32, 0.5_f32);
        let t_air = w.tave;
        let (mut q_ru_sno, mut q_ru_surf, mut q_ru_lat, mut q_ru_wyld) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        for &iru in ins.rus {
            if let Some(hrus) = self.ru_hrus.get(iru) {
                for (ielem, &(ihru, frac)) in hrus.iter().enumerate() {
                    // `ru_elem(ielem)`: the global element table at the loop counter.
                    if !self.ru_elem_is_hru.get(ielem + 1).copied().unwrap_or(false) {
                        continue;
                    }
                    let v = cx.hru_wb.get(ihru).copied().unwrap_or([0.0; 4]);
                    q_ru_sno += v[0] * frac;
                    q_ru_surf += v[1];
                    q_ru_lat += v[2];
                    q_ru_wyld += v[3];
                }
            }
        }
        if q_ru_wyld == 0.0 {
            q_ru_wyld = 1.0e-6;
        }
        let q_gw = 0.0_f32;
        let tw_local_prev = self.chs[ich].ch_out.temp;
        let tw_local = match cx.tgage {
            None => tw_def,
            Some(g) => {
                let jday = cx.jday as f32;
                let yrs_to_start = cx.yrs - g.yrs_start;
                let avg = |lag0: f32| -> f32 {
                    let lag = if cx.yrs == 1 { jday.min(lag0) } else { lag0 };
                    let (mx, mn) = if jday <= lag && yrs_to_start > 1 {
                        let a = (jday + 365.0 - lag + 1.0) as i32;
                        let s = |f: &dyn Fn(i32, i32) -> f32| {
                            let mut s1 = 0.0_f32;
                            for d in 1..=cx.jday {
                                s1 += f(d, yrs_to_start);
                            }
                            let mut s2 = 0.0_f32;
                            for d in a..=365 {
                                s2 += f(d, yrs_to_start - 1);
                            }
                            (s1 + s2) / lag
                        };
                        (s(&|d, y| g.value(d, y)), s(&|d, y| g.value2(d, y)))
                    } else {
                        let lo = 1.max((jday - lag + 1.0) as i32);
                        let hi = jday as i32;
                        let s = |f: &dyn Fn(i32, i32) -> f32| {
                            let mut s1 = 0.0_f32;
                            for d in lo..=hi {
                                s1 += f(d, yrs_to_start);
                            }
                            s1 / lag
                        };
                        (s(&|d, y| g.value(d, y)), s(&|d, y| g.value2(d, y)))
                    };
                    (mx + mn) / 2.0
                };
                let t_surf = avg(surf_lag0).max(0.01);
                let t_lat = avg(lat_lag0).max(0.01);
                let t_gw = avg(gw_lag0).max(0.01);
                let t_sno = avg(sno_lag0).max(0.01).min(2.5);
                let sno_contr = sno_coef * (surf_lag_coef * t_sno) * q_ru_sno;
                let gw_contr = gw_coef * (gw_lag_coef * t_gw) * q_gw;
                let lat_contr = sur_lat_coef * (lat_lag_coef * t_lat) * q_ru_lat;
                let surf_contr = sur_lat_coef * (surf_lag_coef * t_surf) * q_ru_surf;
                let mut tw = (sno_contr + gw_contr + lat_contr + surf_contr) / q_ru_wyld;
                if (tw - tw_local_prev).abs() > 5.0 {
                    tw = 5.0 + 0.75 * w.tave;
                }
                tw
            }
        };
        let tw_up = ht1.temp;
        let tw_init = if ht1.flo / 86400.0 > 1.0e-6 {
            (tw_up * (ht1.flo / 86400.0) + (tw_local * q_ru_wyld)) / ((ht1.flo / 86400.0) + q_ru_wyld)
        } else {
            tw_local
        };
        let tdx_cal = w.dewpt * hex_coef1;
        let tdx = tdx_cal.min(t_air);
        let t_md = (tw_init + tdx) / 2.0;
        let ke_beta = 0.35 + 0.015 * t_md + 0.0012 * (t_md * t_md);
        let f_wind = 9.2 + 0.46 * (w.windsp * w.windsp);
        let k_e = 4.48 + 0.05 * tw_init + (ke_beta + 0.47) * f_wind;
        let h_sr = 0.97 * (w.solrad * 11.57) * (1.0 - ssff);
        let e_s = 6.1275 * (17.62 * w.tave / (237.3 + w.tave)).exp();
        let e_a = e_s * w.rhum;
        let cloud = if w.solradmx < 1.0e-4 { 0.0 } else { 0.9 * (w.solrad / w.solradmx) + 0.1 };
        let e_atm = 0.74 + 0.0065 * e_a * (1.0 + 0.17 * (cloud * cloud));
        let tk = w.tave + 273.15;
        let h_atm = 0.96 * e_atm * 5.67e-8 * ((tk * tk) * (tk * tk));
        let numerator = h_atm - 305.5 - 4.48 * tdx;
        let t_equil = tdx + h_sr / k_e + numerator / k_e;
        let c = &self.chs[ich];
        let vc = c.vel.max(0.01);
        let mut wid_chan = c.db_chw;
        let dep_chan = c.db_chd;
        let wid_flow = ((((ht2.flo / 86400.0) / vc) * 2.0) / ((dep_chan / hex_coef2) / wid_chan)).sqrt();
        if wid_chan < 0.1 {
            wid_chan = c.db_chw * 0.25;
        }
        let mut dep_flow = 0.1_f32.max(((dep_chan / hex_coef2) / wid_chan) * wid_flow);
        if dep_flow > c.db_chd * 2.0 {
            dep_flow = c.db_chd * 2.0;
        }
        let rttime = c.rttime / 24.0;
        let k_factor = 1000.0 * 4186.0 * dep_flow / 86400.0;
        let t_heat_exch = k_e * (t_equil - tw_init) / k_factor * rttime;
        let mut tw_final = tw_init + t_heat_exch;
        if tw_final < 0.001 {
            tw_final = 0.001;
        }
        self.chs[ich].ch_stor.temp = tw_final;
        self.wtemp = 5.0 + 0.75 * w.tave;
        self.chs[ich].wtemp = if self.b.fixes { tw_final } else { self.wtemp };
        tw_final
    }
}
