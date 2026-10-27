//! Edge-of-field and in-field practices that take pollutants out of the surface runoff after it
//! has been lagged: filter strips (`smp_filter`), grassed waterways (`smp_grass_wway`) and
//! user-defined removal shares (`smp_bmpfixed`). None of them changes the water.

use swat_model::Project;
use swat_model::table::Table;

/// A vegetative filter strip (`filterstrip.str`, `structure_set_parms`): the ratio of field to
/// strip area, the share of the field draining to it, and the share of that in its most
/// concentrated tenth that is fully channelized.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Filter {
    pub ratio: f32,
    pub con: f32,
    pub ch: f32,
}

/// A grassed waterway (`grassedww.str`, `structure_set_parms` and `ttcoef_wway`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrassWw {
    pub n: f32,
    pub spcon: f32,
    pub d: f32,
    pub w: f32,
    pub l: f32,
    pub s: f32,
    /// Its time of concentration (h, `tc_gwat`).
    pub tc: f32,
    /// `grwway_vel`: bottom width, bankfull area, flow and celerity.
    pub wid_btm: f32,
    pub area: f32,
    pub vel_bf: f32,
    pub celerity_bf: f32,
}

/// The shares removed by a user-defined practice (`bmpuser.str`): sediment, particulate P,
/// soluble P, particulate N, soluble N.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BmpUser {
    pub sed: f32,
    pub pp: f32,
    pub sp: f32,
    pub pn: f32,
    pub sn: f32,
}

/// The practices of one HRU.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bmps {
    pub filter: Option<Filter>,
    pub grassww: Option<GrassWw>,
    pub user: Option<BmpUser>,
}

/// The surface constituents after `stor_surfstor`, as `hru_control` passes them on: sediment
/// (t), organic N, organic P, nitrate, soluble P, active and stable mineral P (kg/ha), and the
/// sediment classes (sand, silt, clay, small and large aggregates, t).
pub struct Surface<'a> {
    pub lagged: &'a mut [f32; 7],
    pub cls: &'a mut [f32; 5],
}

fn qman(a: f32, rh: f32, n: f32, s: f32) -> f32 {
    a * rh.powf(0.6666) * s.sqrt() / (n + 0.001)
}

fn find<'a>(t: Option<&'a Table>, name: &str) -> Option<(&'a Table, usize)> {
    t.and_then(|t| t.find(name).map(|r| (t, r)))
}

/// `structure_init`: the practices a land use names. A name not in its file gives none (the row
/// 0 of SWAT+'s tables is empty), as does a filter strip whose flag is 0.
pub fn init(project: &Project, names: [&str; 3], km: f32, slope: f32) -> Bmps {
    let [fstrip, grassww, bmpuser] = names;
    let mut b = Bmps::default();
    if let Some((t, r)) = find(project.fstrip.as_ref(), fstrip) {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        if t.int(r, "flag").unwrap_or(0) > 0 {
            let mut ratio = x("fld_vfs");
            if ratio <= 0.0 {
                ratio = 0.0;
            }
            if ratio > 300.0 {
                ratio = 300.0;
            }
            let mut con = x("con_vfs");
            if con <= 0.0 {
                con = 0.5;
            }
            if con <= 0.1 {
                con = 0.1;
            }
            if con > 0.95 {
                con = 0.95;
            }
            let mut ch = x("cha_q");
            if ch <= 0.0 {
                ch = 0.0;
            }
            if ch > 0.95 {
                ch = 0.95;
            }
            b.filter = Some(Filter { ratio, con, ch });
        }
    }
    if let Some((t, r)) = find(project.grassww.as_ref(), grassww) {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        let mut g = GrassWw { n: x("mann"), spcon: x("sed_co"), d: x("dp"), w: x("wd"), l: x("len"), s: x("slp"), ..GrassWw::default() };
        if g.n <= 0.0 {
            g.n = 0.35;
        }
        if g.l <= 0.0 {
            g.l = km.powf(0.5);
        }
        if g.d <= 0.0 {
            g.d = 3.0 / 64.0 * g.w;
        }
        if g.s <= 0.0 {
            g.s = slope * 0.75;
        }
        if g.spcon <= 0.0 {
            g.spcon = 0.005;
        }
        // The overland time (`t_ov`) is added before `time_conc_init` has computed it: 0.
        g.tc = 0.62 * g.l * g.n.powf(0.6) / (km.powf(0.125) * g.s.powf(0.375));
        if g.w - 2.0 * g.d * 8.0 <= 0.0 {
            g.d = 3.0 / 64.0 * g.w;
        }
        // ttcoef_wway
        let mut chsslope = 8.0_f32;
        let d = g.d;
        let mut bw = g.w - 2.0 * d * chsslope;
        if bw <= 0.0 {
            bw = 0.5 * g.w;
            chsslope = (g.w - bw) / (2.0 * d);
        }
        g.wid_btm = bw;
        let p = bw + 2.0 * d * (chsslope * chsslope + 1.0).sqrt();
        let a = bw * d + chsslope * d * d;
        let rh = a / p;
        g.area = a;
        g.vel_bf = qman(a, rh, g.n, g.s);
        g.celerity_bf = qman(1.0, rh, g.n, g.s) * 5.0 / 3.0;
        b.grassww = Some(g);
    }
    if let Some((t, r)) = find(project.bmpuser.as_ref(), bmpuser) {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        b.user = Some(BmpUser { sed: x("sed_eff"), pp: x("ptlp_eff"), sp: x("solp_eff"), pn: x("ptln_eff"), sn: x("soln_eff") });
    }
    b
}

/// Takes `trap` t out of the sediment classes: large aggregates first, then sand, small
/// aggregates, silt and clay.
fn trap_classes(cls: &mut [f32; 5], trap: f32) {
    let mut xrem = trap;
    for k in [4, 0, 3, 1, 2] {
        if xrem <= cls[k] {
            cls[k] -= xrem;
            break;
        }
        xrem -= cls[k];
        cls[k] = 0.0;
    }
    for v in cls.iter_mut() {
        *v = v.max(0.0);
    }
}

fn pct(v: f32, hi: f32) -> f32 {
    let mut v = v;
    if v > hi {
        v = hi;
    }
    if v < 0.0 {
        v = 0.0;
    }
    v
}

/// `smp_filter`, with the surface runoff of the day (mm) and the top layer's saturated
/// conductivity (mm/h).
pub fn filter(f: &Filter, area_ha: f32, surfq: f32, k1: f32, s: &mut Surface) {
    if surfq <= 0.0001 {
        return;
    }
    let l = &mut *s.lagged;
    let drain_vfs1 = (1.0 - f.con) * area_ha;
    let drain_vfs2 = ((1.0 - f.ch) * f.con) * area_ha;
    let area_vfs1 = area_ha * 0.9 / f.ratio;
    let area_vfs2 = area_ha * 0.1 / f.ratio;
    let vfs_ratio1 = drain_vfs1 / area_vfs1;
    let vfs_ratio2 = drain_vfs2 / area_vfs2;
    let vfs_depth1 = vfs_ratio1 * surfq;
    let vfs_depth2 = vfs_ratio2 * surfq;
    let vfs_sed1 = (l[0] / area_ha * 1000.0 * drain_vfs1) / (area_vfs1 * 10000.0);
    let vfs_sed2 = (l[0] / area_ha * 1000.0 * drain_vfs2) / (area_vfs2 * 10000.0);
    let surq_remove1 = pct(75.8 - 10.8 * vfs_depth1.ln() + 25.9 * k1.ln(), 100.0);
    let surq_remove2 = pct(75.8 - 10.8 * vfs_depth2.ln() + 25.9 * k1.ln(), 100.0);
    let wavg = |r1: f32, r2: f32| (r1 * drain_vfs1 + r2 * drain_vfs2) / area_ha;
    let sed_remove1 = pct(79.0 - 1.04 * vfs_sed1 + 0.213 * surq_remove1, 100.0);
    let sed_remove2 = pct(79.0 - 1.04 * vfs_sed2 + 0.213 * surq_remove2, 100.0);
    let sed_remove = wavg(sed_remove1, sed_remove2);
    l[0] = (l[0] * (1.0 - sed_remove / 100.0)).max(0.0);
    // The trapped sediment is taken from the classes after the yield is reduced.
    let sedtrap = l[0] * sed_remove / 100.0;
    trap_classes(s.cls, sedtrap);
    let orgn_remove = wavg(pct(0.036 * sed_remove1.powf(1.69), 100.0), pct(0.036 * sed_remove2.powf(1.69), 100.0));
    l[1] *= 1.0 - orgn_remove / 100.0;
    let no3_remove = wavg(pct(39.4 + 0.584 * surq_remove1, 100.0), pct(39.4 + 0.584 * surq_remove2, 100.0));
    l[3] *= 1.0 - no3_remove / 100.0;
    let pp_remove2 = pct(0.903 * sed_remove2, 100.0);
    let part_p_remove = wavg(pct(0.903 * sed_remove1, 100.0), pp_remove2);
    l[5] *= 1.0 - part_p_remove / 100.0;
    l[6] *= 1.0 - part_p_remove / 100.0;
    l[2] *= 1.0 - part_p_remove / 100.0;
    // Soluble P's second share is computed into `remove21`, so the particulate P's is used.
    let solp_remove = wavg(pct(29.3 + 0.51 * surq_remove1, 100.0), pp_remove2);
    l[4] *= 1.0 - solp_remove / 100.0;
}

/// `smp_grass_wway`, with the surface runoff of the day (mm).
pub fn grass_wway(g: &GrassWw, area_ha: f32, km: f32, surfq: f32, s: &mut Surface) {
    if surfq <= 0.001 {
        return;
    }
    let l = &mut *s.lagged;
    let chflow_day = 1000.0 * surfq * km;
    let chflow_m3 = chflow_day / 10800.0;
    let qp_cms = 2.0 * chflow_m3 / (1.5 * g.tc);
    let (rcharea, rchdep) = if qp_cms > g.vel_bf {
        (g.area, g.d)
    } else {
        let (mut sdti, mut rchdep, mut rcharea) = (0.0_f32, 0.0_f32, 0.0_f32);
        while sdti < qp_cms {
            rchdep += 0.01;
            rcharea = (g.wid_btm + 8.0 * rchdep) * rchdep;
            let p = g.wid_btm + 2.0 * rchdep * 65.0_f32.sqrt();
            let rh = rcharea / p;
            sdti = qman(rcharea, rh, g.n, g.s);
        }
        (rcharea, rchdep)
    };
    let sedin = l[0];
    let mut sf_area = (g.d - rchdep) * 8.06 * g.l * 1000.0;
    sf_area = sf_area.min(0.1 * area_ha * 10000.0);
    let (mut sf_depth, mut sf_sed) = (0.0_f32, 0.0_f32);
    if sf_area > 1.0e-6 {
        sf_area *= 0.20;
        sf_depth = surfq * area_ha * 10000.0 / sf_area;
        sf_sed = sedin * 1000.0 / sf_area;
    }
    let (sed_remove, surq_remove) = if sf_area > 0.0 {
        let surq_remove = (95.6 - 10.79 * sf_depth.ln()).clamp(0.0, 95.0);
        let mut sed_remove = 79.0 - 1.04 * sf_sed + 0.213 * surq_remove;
        if sed_remove > 95.0 {
            sed_remove = 95.0;
        }
        if sed_remove < 0.0 {
            sed_remove = 0.0;
        }
        (sed_remove, surq_remove)
    } else {
        (0.0, 0.0)
    };
    let sedint = sedin * (1.0 - sed_remove / 100.0);
    let mut vc = 0.001_f32;
    if rcharea > 1.0e-4 {
        vc = qp_cms / rcharea;
        if vc > g.celerity_bf {
            vc = g.celerity_bf;
        }
    }
    let mut depnet = 0.0_f32;
    if chflow_m3 > 1.0e-4 {
        let cyin = sedint / chflow_day;
        let cych = g.spcon * vc.powf(1.5);
        depnet = chflow_day * (cyin - cych);
        if depnet < 0.0 {
            depnet = 0.0;
        }
        if depnet > sedint {
            depnet = sedint;
        }
    }
    let sedout = sedint - depnet;
    if l[0] < 0.0001 {
        l[0] = 0.0001;
    }
    let mut sed_frac = sedout / l[0];
    if sed_frac > 75.0 {
        sed_frac = 75.0;
    }
    if sed_frac < 0.0 {
        sed_frac = 5.0;
    }
    let surq_frac = 1.0 - surq_remove / 100.0;
    if sed_frac > 30.0 {
        sed_frac = 30.0;
    }
    let sedtrap = l[0] * (1.0 - sed_frac);
    l[0] *= sed_frac;
    l[5] *= sed_frac;
    l[6] *= sed_frac;
    l[2] *= sed_frac;
    l[4] *= surq_frac;
    l[1] *= sed_frac;
    l[3] *= surq_frac;
    trap_classes(s.cls, sedtrap);
}

/// `smp_bmpfixed`; lateral nitrate goes down by the soluble N share too.
pub fn user(u: &BmpUser, s: &mut Surface, latno3: &mut f32) {
    let l = &mut *s.lagged;
    l[0] *= 1.0 - u.sed;
    l[5] *= 1.0 - u.pp;
    l[6] *= 1.0 - u.pp;
    l[2] *= 1.0 - u.pp;
    l[4] *= 1.0 - u.sp;
    l[1] *= 1.0 - u.pn;
    l[3] *= 1.0 - u.sn;
    *latno3 *= 1.0 - u.sn;
}
