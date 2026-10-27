//! Septic systems (`septic.str`, `sep_biozone`): a biozone layer cut into the soil
//! (`soils_init`, `layersplit`) whose field capacity, saturation, biomass and plaque change
//! every day, and where ammonium, nitrate and soluble P move and transform.
//!
//! SWAT+ never sets the septic tank effluent (`qstemm` is only zeroed), so no water or load
//! reaches the biozone; the port follows it bit for bit (docs/structures.md).

use super::Layer;
use swat_model::SepticSys as S;

/// One HRU's septic system: its type (`iseptic`, the row of `septic.str`), the parameters of
/// that row and of its effluent (`septic.sep` row `typ`), the biozone layer (0-based `i_sep`),
/// and its state (`biom`, `plqm`, `bio_bod`, `fcoli`, `rbiom`, `bz_perc`, `sep_tsincefail`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Septic {
    pub isep: usize,
    pub yr: i32,
    pub tfail: i32,
    pub r: [f32; 23],
    /// q_rate, bod, tss, nh4, no3, no2, orgn, minp, orgp, fcoli.
    pub eff: [f32; 10],
    pub bz: usize,
    pub biom: f32,
    pub plqm: f32,
    pub bio_bod: f32,
    pub fcoli: f32,
    pub rbiom: f32,
    pub bz_perc: f32,
    pub tsincefail: i32,
}

/// `layersplit`: cuts the first layer below the top one that ends deeper than `dep_new`. Both
/// parts keep the layer's properties, its water and capacities in mm included; only depth and
/// thickness change. With no such layer SWAT+ is left without layers.
pub fn layersplit(layers: &mut Vec<Layer>, dep_new: f32) -> Result<(), String> {
    for ly in 1..layers.len() {
        if layers[ly].d > dep_new {
            let orig = layers[ly];
            let mut lower = orig;
            lower.thick = orig.d - dep_new;
            layers[ly].d = dep_new;
            layers[ly].thick = dep_new - layers[ly - 1].d;
            layers.insert(ly + 1, lower);
            return Ok(());
        }
    }
    Err(format!("septic biozone at {dep_new} mm below the soil"))
}

/// `soils_init`: the biozone layer of an operated system. Returns its index (0-based), the
/// layer SWAT+ calls `i_sep`: when the biozone starts within an inch of a layer's top, that is
/// the layer below the biozone.
pub fn biozone(layers: &mut Vec<Layer>, sep: &S) -> Result<usize, String> {
    let (z, thk) = (sep.r[S::Z], sep.r[S::THK]);
    let nly = layers.len();
    let (mut dep_new1, mut dep_new2) = (0.0_f32, 0.0_f32);
    let i_sep;
    if z + thk > layers[nly - 1].d {
        i_sep = nly + 1;
        dep_new1 = z - thk;
    } else {
        let mut found = 0;
        for ly in 1..nly {
            if z < layers[ly].d {
                found = ly + 2;
                if (z - layers[ly].d).abs() > 25.4 {
                    dep_new1 = z;
                    dep_new2 = z + thk;
                } else {
                    dep_new1 = 0.0;
                    layers[ly].d = z;
                }
                if (z - layers[ly - 1].d).abs() < 25.4 {
                    dep_new1 = z + thk;
                    dep_new2 = 0.0;
                }
                break;
            }
        }
        i_sep = found;
    }
    if dep_new1 > 1.0e-6 {
        layersplit(layers, dep_new1)?;
    }
    if dep_new2 > 1.0e-6 {
        layersplit(layers, dep_new2)?;
    }
    // SWAT+ would index outside the soil (or layer 0): the biozone must have a layer below it.
    if i_sep < 3 || i_sep >= layers.len() {
        return Err(format!("septic biozone layer {i_sep} of {} (none below it)", layers.len()));
    }
    Ok(i_sep - 1)
}

/// `sep_biozone`, with the shared operation flag of the system's type (`sep(isep)%opt`: one
/// HRU's failure fails every HRU of that type).
pub fn sep_biozone(sp: &mut Septic, layers: &mut [Layer], area_ha: f32, opt: &mut i32) {
    let qstemm = 0.0_f32;
    let r = sp.r;
    let bz = sp.bz;
    let nly = layers.len();
    let bza = area_ha as f64;
    let thk = r[S::THK];
    let bz_vol = thk as f64 * bza * 10.0;
    let qi = (layers[bz].st + layers[bz - 1].prk + qstemm) as f64 * bza * 10.0;
    let qin = qstemm as f64 * bza * 10.0;
    let qout = sp.bz_perc as f64 * bza * 10.0;
    let rtof = 0.5_f64;
    let ctmp = 1.0_f64;
    if *opt == 2 {
        if sp.tsincefail > 0 {
            sp.tsincefail += 1;
        }
        if sp.tsincefail >= sp.tfail {
            *opt = 1;
            let l = &mut layers[bz];
            l.ul = thk * (l.por - l.wp);
            l.fc = thk * (l.up - l.wp);
            l.nh4 = 0.0;
            l.no3 = 0.0;
            l.hsta.n = 0.0;
            l.hsta.p = 0.0;
            l.lab = 0.0;
            l.act = 0.0;
            sp.biom = 0.0;
            sp.plqm = 0.0;
            sp.bio_bod = 0.0;
            sp.fcoli = 0.0;
            sp.tsincefail = 0;
        }
        return;
    }
    let e = sp.eff;
    let xx = qin / bza / 1000.0;
    {
        let l = &mut layers[bz];
        l.no3 = (l.no3 as f64 + xx * (e[4] + e[5]) as f64) as f32;
        l.nh4 = (l.nh4 as f64 + xx * e[3] as f64) as f32;
        l.hsta.n = (l.hsta.n as f64 + xx * e[6] as f64 * rtof) as f32;
        l.hsta.p = (l.hsta.p as f64 + xx * e[8] as f64 * rtof) as f32;
        l.lab = (l.lab as f64 + xx * e[7] as f64) as f32;
    }
    sp.bio_bod = (sp.bio_bod as f64 + xx * e[1] as f64) as f32;
    let bodi = sp.bio_bod as f64 * bza / qi * 1000.0;
    let bd = r[S::BD];
    {
        let l = &mut layers[bz];
        l.fc += r[S::FC1] * (l.ul - l.fc).powf(r[S::FC2]) * sp.rbiom / (bd * 10.0);
        l.ul = l.por * thk - sp.plqm / (bd * 10.0);
        if l.ul <= l.fc {
            l.ul = l.fc;
            *opt = 2;
        }
    }
    let rrsp = ctmp * r[S::RSP] as f64 * sp.biom as f64;
    let rmort = ctmp * r[S::MRT] as f64 * sp.biom as f64;
    let rslg = (r[S::SLG1] * sp.bz_perc.powf(r[S::SLG2]) * sp.biom) as f64;
    let mut rplqm = (rmort - rslg) + r[S::PLQ] as f64 * qin * e[2] as f64 / (1000.0 * bza);
    rplqm = rplqm.max(0.0);
    sp.plqm = (sp.plqm as f64 + rplqm) as f32;
    let st = layers[bz].st;
    let por = layers[bz].por;
    let xx = (10.0 * st) as f64 * bza / (qin + (10.0 * st) as f64 * bza);
    sp.fcoli = (sp.fcoli as f64 * xx + e[9] as f64 * (1.0 - xx)) as f32;
    let rtrate = sp.biom as f64 * bza / (bz_vol * por as f64);
    let mut bod_rt = (r[S::BOD_DC] as f64 * rtrate).max(0.0);
    if bod_rt > 4.0 {
        bod_rt = 4.0;
    }
    let rbod = bodi * (1.0 - (-bod_rt).exp());
    let bode = bodi - rbod;
    sp.bio_bod = (bode * (st * 10.0) as f64 / 1000.0) as f32;
    let fcoli_rt = (r[S::FECAL] as f64 * rtrate).max(0.0);
    let rfcoli = sp.fcoli as f64 * (1.0 - (-fcoli_rt).exp());
    sp.fcoli = (sp.fcoli as f64 - rfcoli) as f32;
    let ntr_rt = (r[S::NITR] as f64 * rtrate).max(0.0);
    let l = &mut layers[bz];
    let rnit = l.nh4 as f64 * (1.0 - (-ntr_rt).exp());
    l.nh4 = (l.nh4 as f64 - rnit) as f32;
    l.no3 = (l.no3 as f64 + rnit) as f32;
    let mut nperc = 0.2_f32 as f64 * qout / qi * l.nh4 as f64;
    nperc = nperc.min((0.5 * l.nh4) as f64);
    l.nh4 = (l.nh4 as f64 - nperc) as f32;
    layers[bz + 1].nh4 = (layers[bz + 1].nh4 as f64 + nperc) as f32;
    let l = &mut layers[bz];
    let dentr_rt = (r[S::DENITR] as f64 * rtrate).max(0.0);
    let rdenit = l.no3 as f64 * (1.0 - (-dentr_rt).exp());
    l.no3 = (l.no3 as f64 - rdenit) as f32;
    let dn = layers[nly - 1].d;
    let l = &mut layers[bz];
    let svolp = (dn - r[S::Z]) as f64 * bza * 10.0;
    let solpconc = l.lab as f64 * bza / qi * 1000.0;
    let mut solpsorb = (r[S::PDISTRB] as f64 * solpconc).min(r[S::PSORPMAX] as f64);
    solpsorb = (1.6_f32 * 1.0e-3_f32) as f64 * solpsorb * svolp * (1.0 - l.por) as f64;
    if (l.lab as f64 * bza) < solpsorb {
        let totalp = (l.lab + l.act) as f64;
        let mut solp_end = r[S::SOLPSLP] as f64 * totalp + r[S::SOLPINTC] as f64;
        if solp_end > l.lab as f64 {
            solp_end = l.lab as f64;
        }
        l.act = ((l.act + l.lab) as f64 - solp_end) as f32;
        l.lab = solp_end as f32;
    }
    let solpconc = l.lab as f64 * bza / qi * 1000.0;
    let percp = 0.01_f32 as f64 * solpconc * qout / bza * 1.0e-3_f32 as f64;
    l.lab = (l.lab as f64 - percp) as f32;
    layers[bz + 1].lab = (layers[bz + 1].lab as f64 + percp) as f32;
    let mut rbiom = ctmp * r[S::BOD_CONV] as f64 * (qin * e[1] as f64 - qout * bode) / (1000.0 * bza) - (rrsp + rmort + rslg);
    rbiom = rbiom.max(1.0e-6_f32 as f64);
    sp.rbiom = rbiom as f32;
    sp.biom += sp.rbiom;
}
