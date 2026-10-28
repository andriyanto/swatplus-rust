//! What reservoirs and wetlands share (`res_sediment`, `res_nutrient`): sediment settling to its
//! normal concentration and nutrient settling, of the water body's water and today's outflow.

use crate::fortran::rpow;
use crate::hyd::Hyd;
use swat_model::Project;

/// A water body's `sediment.res` and `nutrients.res` rows (`wbody_prm`), the rates per day.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WbPrm {
    /// Normal sediment (mg/L) and its settling factor.
    pub nsed: f32,
    pub sed_stlr_co: f32,
    /// The share of the soluble N and P that settles with the rates below: 0.2 when the
    /// `sediment.res` row is found, else 0 (SWAT+ sets them only then).
    pub soln_stl_fr: f32,
    pub solp_stl_fr: f32,
    pub ires1: i32,
    pub ires2: i32,
    pub nsetlr1: f32,
    pub nsetlr2: f32,
    pub psetlr1: f32,
    pub psetlr2: f32,
    pub nsolr: f32,
    pub psolr: f32,
    pub theta_n: f32,
    pub theta_p: f32,
    pub conc_nmin: f32,
    pub conc_pmin: f32,
}

fn find<'a>(t: Option<&'a swat_model::table::Table>, name: &str) -> Option<(&'a swat_model::table::Table, usize)> {
    let t = t?;
    (0..t.rows.len()).find(|&r| t.text(r, "name").is_some_and(|n| swat_model::table::fortran_eq(n, name))).map(|r| (t, r))
}

impl WbPrm {
    /// The rows named `sed` and `nut` (the type's defaults when not found).
    ///
    /// `res_read` and `wet_initial` set the settling factor and the soluble shares only when the
    /// `sediment.res` row is found; without it they stay 0: all sediment above the normal
    /// concentration (0) settles, and no soluble N or P.
    pub fn new(project: &Project, sed: &str, nut: &str) -> WbPrm {
        let (nsed, sed_stlr_co, stl_fr) = match find(project.res_sed.as_ref(), sed) {
            Some((t, r)) => (t.real(r, "nsed").unwrap_or(0.0), (-0.184 * t.real(r, "d50").unwrap_or(0.0)).exp(), 0.2),
            None => (0.0, 0.0, 0.0),
        };
        let nt = find(project.res_nut.as_ref(), nut);
        let nr = |f: &str, d: f32| nt.map(|(t, r)| t.real(r, f).unwrap_or(d)).unwrap_or(d);
        let ni = |f: &str| nt.map(|(t, r)| t.int(r, f).unwrap_or(0)).unwrap_or(0);
        // res_read_nut: the rates of the file's rows from per year to per day.
        let rate = |f: &str, d: f32| nt.map(|(t, r)| t.real(r, f).unwrap_or(d) / 365.0).unwrap_or(d);
        WbPrm {
            nsed,
            sed_stlr_co,
            soln_stl_fr: stl_fr,
            solp_stl_fr: stl_fr,
            ires1: ni("ires1"),
            ires2: ni("ires2"),
            nsetlr1: rate("nsetlr1", 0.0),
            nsetlr2: rate("nsetlr2", 0.0),
            psetlr1: rate("psetlr1", 0.0),
            psetlr2: rate("psetlr2", 0.0),
            nsolr: rate("nsolr", 1.0),
            psolr: rate("psolr", 1.0),
            theta_n: nr("theta_n", 1.0),
            theta_p: nr("theta_p", 1.0),
            conc_nmin: nr("conc_nmin", 0.1),
            conc_pmin: nr("conc_pmin", 0.01),
        }
    }
}

fn theta(r20: f32, thk: f32, tmp: f32) -> f32 {
    r20 * rpow(thk, tmp - 20.0)
}

/// SWAT+ `res_sediment`: above its normal concentration the water's sediment settles, the
/// outflow takes the settled concentration and the sediment classes are dropped; at or below it
/// nothing settles and the outflow carries no sediment.
pub fn res_sediment(b: &mut Hyd, out: &mut Hyd, p: &WbPrm) {
    if b.flo < 1.0e-6 {
        *b = Hyd::default();
        out.sed = 0.0;
    } else {
        let mut sed_ppm = (1_000_000.0 * b.sed / b.flo).max(1.0e-6);
        if sed_ppm > p.nsed {
            sed_ppm = (sed_ppm - p.nsed) * p.sed_stlr_co + p.nsed;
            sed_ppm = sed_ppm.max(p.nsed);
            b.sed = sed_ppm * b.flo / 1_000_000.0;
            out.sed = sed_ppm * out.flo / 1_000_000.0;
            b.sed = (b.sed - out.sed).max(0.0);
            b.sil = 0.0;
            b.cla = 0.0;
            b.san = 0.0;
            b.sag = 0.0;
            b.lag = 0.0;
            b.grv = 0.0;
        }
    }
}

/// SWAT+ `res_nutrient` in month `mo` at the station's mean temperature `tave`: organic and
/// soluble N and P settle, and the outflow takes its share of what is left.
pub fn res_nutrient(b: &mut Hyd, o: &mut Hyd, p: &WbPrm, mo: i32, tave: f32) {
    if b.flo < 1.0e-6 {
        *b = Hyd::default();
        return;
    }
    let (nsetlr, psetlr) = if mo >= p.ires1 && mo <= p.ires2 { (p.nsetlr1, p.psetlr1) } else { (p.nsetlr2, p.psetlr2) };
    let conc_n = 1000.0 * b.orgn / b.flo;
    let conc_p = 1000.0 * b.sedp / b.flo;
    let conc_soln = 1000.0 * (b.no3 + b.nh3 + b.no2) / b.flo;
    let conc_solp = 1000.0 * b.solp / b.flo;
    let k = |c: f32, cmin: f32, r: f32, th: f32| ((c - cmin) * theta(r, th, tave)).min(1.0).max(0.0);
    let nitrok = k(conc_n, p.conc_nmin, nsetlr, p.theta_n);
    let phosk = k(conc_p, p.conc_pmin, psetlr, p.theta_p);
    let nitrosolk = k(conc_soln, p.conc_nmin, p.nsolr, p.theta_n);
    let phossolk = k(conc_solp, p.conc_pmin, p.psolr, p.theta_p);
    let (soln_stl_fr, solp_stl_fr) = (p.soln_stl_fr, p.solp_stl_fr);
    b.solp *= 1.0 - phossolk * solp_stl_fr;
    b.sedp *= 1.0 - phosk;
    b.orgn *= 1.0 - nitrok;
    b.no3 *= 1.0 - nitrosolk * soln_stl_fr;
    b.nh3 *= 1.0 - nitrosolk * soln_stl_fr;
    b.no2 *= 1.0 - nitrosolk * soln_stl_fr;
    b.chla = 0.0;
    let tpco = 1.0e6 * (b.solp + b.sedp) / (b.flo + o.flo);
    if tpco > 1.0e-4 {
        b.chla = (b.flo + o.flo) * 1.0e-6;
    }
    for v in [&mut b.no3, &mut b.orgn, &mut b.sedp, &mut b.solp, &mut b.chla, &mut b.nh3, &mut b.no2] {
        *v = v.max(0.0);
    }
    let tot = b.flo + o.flo;
    o.no3 = b.no3 * o.flo / tot;
    o.orgn = b.orgn * o.flo / tot;
    o.sedp = b.sedp * o.flo / tot;
    o.solp = b.solp * o.flo / tot;
    o.chla = b.chla * o.flo / tot;
    o.nh3 = b.nh3 * o.flo / tot;
    o.no2 = b.no2 * o.flo / tot;
    b.no3 = (b.no3 - o.no3).max(0.0);
    b.orgn = (b.orgn - o.orgn).max(0.0);
    b.sedp = (b.sedp - o.sedp).max(0.0);
    b.solp = (b.solp - o.solp).max(0.0);
    b.chla = (b.chla - o.chla).max(0.0);
    b.nh3 = (b.nh3 - o.nh3).max(0.0);
    b.no2 = (b.no2 - o.no2).max(0.0);
}
