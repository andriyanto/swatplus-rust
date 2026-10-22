//! The carbon model (`cswat` 2, SWAT-C after Zhang): its soil pools (`soil_nutcarb_init`),
//! surface residue decomposition (`cbn_surfrsd_decomp`), soil residue transfer
//! (`cbn_rsd_transfer`), the pool transformations (`cbn_zhang2`, `nut_np_flow`), organic N and
//! carbon leaving with water and sediment (`nut_orgnc2`), and the nutrient part of the daily
//! biological mixing (`mgt_biomix`, `mgt_tillfactor`).

use swat_model::Project;

use super::plant::{OrgMass, Saved};
use super::{Biomix, Hru};

/// A layer's carbon model pools (`soil1(j)`): total, slow and passive humus, microbial biomass,
/// structural litter with its lignin and non-lignin parts, metabolic litter, sequestered.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pools {
    pub tot: OrgMass,
    pub hs: OrgMass,
    pub hp: OrgMass,
    pub microb: OrgMass,
    pub str: OrgMass,
    pub lig: OrgMass,
    pub nonlig: OrgMass,
    pub meta: OrgMass,
    pub seq: OrgMass,
}

/// Rates of `carbon_lyr.bsn` for one kind of layer (`carbdb`, `org_allo`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayerRates {
    pub hp_rate: f32,
    pub hs_rate: f32,
    pub microb_rate: f32,
    pub meta_rate: f32,
    pub str_rate: f32,
    pub microb_top_rate: f32,
    pub hs_hp: f32,
    pub a1co2: f32,
    pub asco2: f32,
    pub apco2: f32,
    pub abco2: f32,
}

/// `carbon.bsn` and `carbon_lyr.bsn`, and the codes and parameters the carbon model reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarbonBasin {
    pub frac_seq: f32,
    pub frac_hum_microb: f32,
    pub frac_hum_slow: f32,
    pub frac_hum_passive: f32,
    pub prmt_21: f32,
    pub prmt_44: f32,
    pub watf: i32,
    pub bmix_a: f32,
    pub bmix_b: f32,
    pub bmix_c: f32,
    pub photo_degrade_factor: f32,
    pub cnr_cap: f32,
    pub cnr_ref: f32,
    pub cpr_cap: f32,
    pub cpr_ref: f32,
    pub mathers: bool,
    pub rates: [LayerRates; 2],
    pub idc_till: i32,
    pub sdnco: f32,
    pub cdn: f32,
    pub decr_min: f32,
}

impl CarbonBasin {
    pub fn new(project: &Project) -> Self {
        let c = project.carbon.as_ref();
        let r = |n: &str, d: f32| c.and_then(|t| t.real(0, n)).unwrap_or(d);
        let i = |n: &str, d: i32| c.and_then(|t| t.int(0, n)).unwrap_or(d);
        let mut rates = [LayerRates::default(); 2];
        if let Some(t) = project.carbon_lyr.as_ref() {
            for row in 0..t.rows.len() {
                let id = t.int(row, "layer").unwrap_or(0);
                if !(1..=2).contains(&id) {
                    continue;
                }
                let x = |n: &str| t.real(row, n).unwrap_or(0.0);
                rates[(id - 1) as usize] = LayerRates {
                    hp_rate: x("hp_rate"),
                    hs_rate: x("hs_rate"),
                    microb_rate: x("microb_rate"),
                    meta_rate: x("meta_rate"),
                    str_rate: x("str_rate"),
                    microb_top_rate: x("microb_top_rate"),
                    hs_hp: x("hs_hp"),
                    a1co2: x("a1co2"),
                    asco2: x("asco2"),
                    apco2: x("apco2"),
                    abco2: x("abco2"),
                };
            }
        }
        let p = |n: &str| project.params.get(n).unwrap_or(0.0);
        Self {
            frac_seq: r("frac_seq", 0.95),
            frac_hum_microb: r("frac_hum_microb", 0.02),
            frac_hum_slow: r("frac_hum_slow", 0.54),
            frac_hum_passive: r("frac_hum_passive", 0.44),
            prmt_21: r("prmt_21", 1000.0),
            prmt_44: r("prmt_44", 0.5),
            watf: i("watf", 1),
            bmix_a: r("bmix_a", 3.0),
            bmix_b: r("bmix_b", 0.0),
            bmix_c: r("bmix_c", 0.0),
            photo_degrade_factor: r("photo_degrade_factor", 0.01),
            cnr_cap: r("cnr_cap", 500.0),
            cnr_ref: r("cnr_ref", 25.0),
            cpr_cap: r("cpr_cap", 5000.0),
            cpr_ref: r("cpr_ref", 200.0),
            mathers: i("mathers_int", 0) == 1,
            rates,
            idc_till: project.codes.idc_till,
            sdnco: p("sdnco"),
            cdn: p("cdn"),
            decr_min: p("decr_min"),
        }
    }
}

/// SWAT+ `nut_np_flow`: the N (or P) that goes with a carbon flow from pool A to pool B, and the
/// immobilization or mineralization needed for B's C:N ratio. Returns (flow, immobilized,
/// mineralized). The N released with the CO2 is computed, then a different variable (always 0) is
/// added to the mineralization.
fn np_flow(c_a: f32, e_a: f32, ce_to_b: f32, c_a_to_b: f32) -> (f32, f32, f32) {
    let mut e_a_to_b = 0.0_f32;
    let mut mnr = 0.0_f32;
    let mut imm = 0.0_f32;
    if e_a < 1.0e-6 || c_a_to_b < 1.0e-6 || c_a < 1.0e-6 {
        return (e_a_to_b, imm, mnr);
    }
    e_a_to_b = e_a * (c_a_to_b / c_a);
    if c_a_to_b / e_a_to_b > ce_to_b {
        imm = c_a_to_b / ce_to_b - e_a_to_b;
    } else {
        mnr = e_a_to_b - c_a_to_b / ce_to_b;
        e_a_to_b -= mnr;
    }
    mnr += 0.0;
    (e_a_to_b, imm, mnr)
}

impl Hru {
    /// `soil_nutcarb_init` for `cswat` 2: the carbon model's pools from each layer's organic
    /// carbon.
    pub(super) fn carbon_init(&mut self, cb: &CarbonBasin) {
        for l in self.layers.iter_mut() {
            let tot_mass = 10000.0 * l.thick * l.bd;
            let mut tot = OrgMass { m: tot_mass * (l.cbn / 100.0) / 0.58, c: tot_mass * (l.cbn / 100.0), ..OrgMass::default() };
            tot.n = tot.c / 10.0;
            tot.p = tot.c / 100.0;
            let frac_not_seq = 1.0 - cb.frac_seq;
            let c = &mut l.cp;
            c.hp.m = cb.frac_seq * cb.frac_hum_passive * tot.m;
            c.hp.c = cb.frac_seq * cb.frac_hum_passive * tot.c;
            c.hp.n = c.hp.c / 10.0;
            c.hp.p = c.hp.c / 80.0;
            if !cb.mathers {
                c.hs.m = cb.frac_seq * cb.frac_hum_slow * tot.m;
                c.hs.c = cb.frac_seq * cb.frac_hum_slow * tot.c;
                c.hs.n = c.hs.c / 10.0;
                c.hs.p = c.hs.c / 80.0;
            } else {
                // Clay and silt are percentages here, compared with 0.35 and scaled by 100 again.
                let cs = l.clay + l.silt;
                let mathers_frac = if cs >= 0.35 { (0.41 + 0.0053 * 100.0 * cs) / 100.0 } else { (0.069 + 0.015 * 100.0 * cs) / 100.0 };
                c.hs.c = cb.frac_seq * mathers_frac * tot.c;
                c.hs.m = c.hs.c / 0.58;
                c.hs.n = c.hs.c / 10.0;
                c.hs.p = c.hs.c / 80.0;
            }
            c.microb.m = cb.frac_seq * cb.frac_hum_microb * tot.m;
            c.microb.c = cb.frac_seq * cb.frac_hum_microb * tot.c;
            c.microb.n = c.microb.c / 8.0;
            c.microb.p = c.microb.c / 80.0;
            c.meta.m = frac_not_seq * 0.85 * tot.m;
            c.meta.c = frac_not_seq * 0.85 * tot.c;
            c.meta.n = c.meta.c / 10.0;
            c.meta.p = c.meta.c / 100.0;
            c.str.m = frac_not_seq * 0.15 * tot.m;
            c.str.c = frac_not_seq * 0.15 * tot.c;
            c.str.n = c.str.c / 150.0;
            c.str.p = c.str.c / 1500.0;
            c.lig.m = 0.8 * c.str.m;
            c.lig.c = 0.8 * c.str.c;
            c.lig.n = 0.2 * c.str.n;
            c.lig.p = 0.02 * c.str.p;
            c.nonlig = c.str - c.lig;
            c.tot = c.str + c.meta + c.hs + c.hp + c.microb;
            c.seq = c.hs + c.hp + c.microb;
        }
    }

    /// SWAT+ `cbn_surfrsd_decomp`: surface residue loses a share to sunlight, and decomposes
    /// into the top layer's metabolic, structural and lignin litter.
    pub(super) fn cbn_surfrsd_decomp(&mut self, cb: &CarbonBasin) {
        let Some(pl) = self.plant.as_mut() else { return };
        let photo = OrgMass::scale(cb.photo_degrade_factor, pl.rsd);
        pl.rsd = pl.rsd - photo;
        self.rsd_tot = self.rsd_tot - photo;
        let l = &mut self.layers[0];
        if l.tmp <= 0.0 {
            return;
        }
        let mut sut = 0.1 + 0.9 * (l.st / l.fc).sqrt();
        sut = sut.max(0.05);
        let xx = l.tmp;
        let mut cdg = 0.9 * xx / (xx + (9.93_f32 - 0.312 * xx).exp()) + 0.1;
        cdg = cdg.max(0.1);
        let mut xx = cdg * sut;
        if xx < 0.0 {
            xx = 0.0;
        }
        if xx > 1.0e6 {
            xx = 1.0e6;
        }
        let csf = xx.sqrt();
        let r = pl.rsd;
        let cnrf = if r.n > 1.0e-4 {
            let mut cnr = r.c / r.n;
            if cnr > cb.cnr_cap {
                cnr = cb.cnr_cap;
            }
            (-0.693_f32 * (cnr - cb.cnr_ref) / cb.cnr_ref).exp()
        } else {
            1.0
        };
        let cprf = if r.p > 1.0e-4 {
            let mut cpr = r.c / r.p;
            if cpr > cb.cpr_cap {
                cpr = cb.cpr_cap;
            }
            (-0.693_f32 * (cpr - cb.cpr_ref) / cb.cpr_ref).exp()
        } else {
            1.0
        };
        let ca = cnrf.min(cprf).min(1.0);
        let mut decr = pl.db.rsdco_pl * ca * csf;
        decr = decr.max(cb.decr_min);
        decr = decr.min(1.0);
        let decomp = OrgMass::scale(decr, pl.rsd);
        pl.rsd = pl.rsd - decomp;
        self.rsd_tot = self.rsd_tot - decomp;
        if pl.rsd.m < 1.0e-10 {
            pl.rsd.m = 0.0;
        }
        if pl.rsd.c < 1.0e-10 {
            pl.rsd.c = 0.0;
        }
        if pl.rsd.n < 1.0e-10 {
            pl.rsd.n = 0.0;
        }
        if pl.rsd.p < 1.0e-10 {
            pl.rsd.p = 0.0;
        }
        let f = pl.db.fracs;
        l.cp.meta = l.cp.meta + OrgMass::scale(f.meta_abg, decomp);
        l.cp.str = l.cp.str + OrgMass::scale(f.str_abg, decomp);
        l.cp.lig = l.cp.lig + OrgMass::scale(f.lig_abg, decomp);
    }

    /// SWAT+ `cbn_rsd_transfer`: all of a thawed layer's plant residue goes to its litter pools.
    pub(super) fn cbn_rsd_transfer(&mut self) {
        let Some(f) = self.plant.as_ref().map(|p| p.db.fracs) else { return };
        for l in self.layers.iter_mut() {
            if l.tmp > 0.0 {
                let transfer = OrgMass::scale(1.0, l.rsd);
                l.rsd = l.rsd - transfer;
                if l.rsd.m < 1.0e-10 {
                    l.rsd.m = 0.0;
                }
                if l.rsd.c < 1.0e-10 {
                    l.rsd.c = 0.0;
                }
                if l.rsd.n < 1.0e-10 {
                    l.rsd.n = 0.0;
                }
                if l.rsd.p < 1.0e-10 {
                    l.rsd.p = 0.0;
                }
                l.cp.meta = l.cp.meta + OrgMass::scale(f.meta_blg, transfer);
                l.cp.str = l.cp.str + OrgMass::scale(f.str_blg, transfer);
                l.cp.lig = l.cp.lig + OrgMass::scale(f.lig_blg, transfer);
            }
        }
    }

    /// SWAT+ `cbn_zhang2`: decomposition and the flows between litter, microbes and humus, layer
    /// by layer, with denitrification. Returns the denitrified N.
    pub(super) fn cbn_zhang2(&mut self, cb: &CarbonBasin, bm: &Biomix) -> f32 {
        let mut denit = 0.0_f32;
        let nly = self.layers.len();
        let rsd_tot = self.rsd_tot;
        // org_allo(2)%abco2 is a module variable: layers below the first set it from their sand.
        let mut allo = cb.rates;
        for k in 0..nly {
            let kk = if k == 0 { 1.min(nly - 1) } else { k };
            let cf = if k == 0 { 0 } else { 1 };
            let dmid = (self.layers[kk].d + self.layers[kk.saturating_sub(1)].d) / 2.0;
            let st_kk = self.layers[kk].st;
            let l = &mut self.layers[k];
            let sol_mass = 10000.0 * l.thick * l.bd * (1.0 - l.rock / 100.0);
            let stemp = l.tmp;
            if !(stemp > 0.0 && l.st > 0.0) {
                continue;
            }
            let fc = l.fc + l.wpmm;
            let wc = l.st + l.wpmm;
            let mut sut = 0.0_f32;
            if cb.watf == 1 {
                sut = if wc - l.wpmm < 0.0 {
                    let x = st_kk / l.wpmm;
                    0.1 * (x * x)
                } else {
                    0.1 + 0.9 * (l.st / l.fc).sqrt()
                };
                sut = sut.min(1.0);
                sut = sut.max(0.05);
            }
            if cb.watf == 2 {
                let svoid = l.por * (1.0 - (wc / (l.ul + l.wpmm)));
                let xx1 = if wc <= l.wpmm {
                    0.4 * (wc / l.wpmm)
                } else if wc <= fc {
                    0.4 + 0.6 * ((wc - l.wpmm) / (fc - l.wpmm))
                } else {
                    1.0
                };
                let xx2 = if svoid >= 0.1 { 0.2 + 0.8 * ((svoid - 0.1) / (l.por - 0.1)) } else { 0.2 * (svoid / 0.1) };
                let w1 = (1.0 + 4.0 * (1.0 - xx1)) * (xx1 * xx1 * xx1 * xx1);
                let w2 = 0.5 + 0.5 * (xx2 / (xx2 + (-20.0 * xx2).exp()));
                sut = w1 * w2;
            }
            let till_eff = if cb.idc_till == 3 { 1.0 + l.tillagef } else { 1.0 };
            let cdg = match bm.tmpf {
                1 => {
                    if stemp <= 35.0 {
                        stemp / (stemp + (5.058_f32 - 0.2504 * stemp).exp())
                    } else if stemp <= 60.0 {
                        1.0 - 0.04 * (stemp - 35.0)
                    } else {
                        0.0
                    }
                }
                2 => bm.fcgd(stemp),
                3 => 0.9 * (stemp / (stemp + (9.93_f32 - 0.312 * stemp).exp())) + 0.1,
                _ => 0.0,
            };
            let ox = 1.0 - 0.8 * dmid / (dmid + (18.40961_f32 - 0.023683632 * dmid).exp());
            let cs = (15.0_f32).min((cdg * sut).sqrt() * 0.9 * ox * till_eff);
            let wdn = if sut >= cb.sdnco { l.no3 * (1.0 - (-cb.cdn * cdg * l.cbn / 100.0).exp()) } else { 0.0 };
            l.no3 = (l.no3 - wdn).max(0.0001);
            denit += wdn;
            let sol_min_n = l.no3 + l.nh4;
            let c = &mut l.cp;
            let rlr = (0.8_f32).min(c.lig.m / (c.str.m + 1.0e-5));
            let (ncbm, nchs, nchp, xbm) = if k == 0 {
                let rsdn_pct = 0.1 * (rsd_tot.n + c.meta.n) / (rsd_tot.c / 1000.0 + 1.0e-5);
                // The first test is overridden: its `else` belongs to the second.
                let ncbm = if rsdn_pct > 0.01 && rsdn_pct <= 2.0 { 1.0 / (20.05 - 5.0251 * rsdn_pct) } else { 0.05 };
                (ncbm, ncbm / (5.0 * ncbm + 1.0), 0.1_f32, 1.0_f32)
            } else {
                allo[1].abco2 = 0.17 + 0.0068 * l.sand;
                let xbm = 0.25 + 0.0075 * l.sand;
                let min_n_ppm = 1000.0 * sol_min_n / (sol_mass / 1000.0);
                if min_n_ppm > 7.15 {
                    (0.33, 0.083, 0.143, xbm)
                } else {
                    (1.0 / (15.0 - 1.678 * min_n_ppm), 1.0 / (20.0 - 1.119 * min_n_ppm), 1.0 / (10.0 - 0.42 * min_n_ppm), xbm)
                }
            };
            let a = allo[cf];
            let rates = cb.rates[cf];
            let asp = (0.001_f32).max(rates.hs_hp - 0.00009 * l.clay);
            let x1 = rates.str_rate * cs * (-3.0 * rlr).exp();
            let lsctp = x1 * c.str.c;
            let lslctp = lsctp * rlr;
            let lslnctp = lsctp * (1.0 - rlr);
            let lsntp = x1 * c.str.n;
            let x1 = rates.meta_rate * cs;
            let mut lmctp = c.meta.c * x1;
            if lmctp <= 1.0e-6 {
                lmctp = 0.0;
            }
            let lmntp = c.meta.n * x1;
            let x1 = if k == 0 { rates.microb_top_rate * cs * xbm } else { rates.microb_rate * cs * xbm };
            let bmctp = c.microb.c * x1;
            let bmntp = c.microb.n * x1;
            let x1 = rates.hs_rate * cs;
            let hsctp = c.hs.c * x1;
            let hsntp = c.hs.n * x1;
            let x1 = cs * rates.hp_rate;
            let hpctp = c.hp.c * x1;
            let hpntp = c.hp.n * x1;
            let a1 = 1.0 - a.a1co2;
            let asx = 1.0 - a.asco2 - asp;
            let apx = 1.0 - a.apco2;
            let abp = 0.003 + 0.00032 * l.clay;

            // Demand and supply of N by flow, then the same with the flows reduced to the N there is.
            let balance = |ls_c: f32,
                           lsl_c: f32,
                           lsln_c: f32,
                           ls_n: f32,
                           lm_c: f32,
                           lm_n: f32,
                           bm_c: f32,
                           bm_n: f32,
                           hs_c: f32,
                           hs_n: f32,
                           hp_c: f32,
                           hp_n: f32| {
                let pn1 = lsln_c * a1 * ncbm;
                let pn2 = 0.7 * lsl_c * nchs;
                let pn3 = lm_c * a1 * ncbm;
                let pn5 = bm_c * abp * nchp;
                let pn6 = bm_c * (1.0 - abp - a.abco2) * nchs;
                let pn7 = hs_c * asx * ncbm;
                let pn8 = hs_c * asp * nchp;
                let pn9 = hp_c * apx * ncbm;
                let _ = ls_c;
                let (mut s1, mut s2, mut s3, mut s4, mut s5) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
                let (mut c1, mut c2, mut c3, mut c4, mut c5) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
                let x = pn1 + pn2;
                if ls_n < x {
                    c1 = x - ls_n;
                } else {
                    s1 = s1 + ls_n - x;
                }
                if lm_n < pn3 {
                    c2 = pn3 - lm_n;
                } else {
                    s2 = s2 + lm_n - pn3;
                }
                let x = pn5 + pn6;
                if bm_n < x {
                    c3 = x - bm_n;
                } else {
                    s3 = s3 + bm_n - x;
                }
                let x = pn7 + pn8;
                if hs_n < x {
                    c4 = x - hs_n;
                } else {
                    s4 = s4 + hs_n - x;
                }
                if hp_n < pn9 {
                    c5 = pn9 - hp_n;
                } else {
                    s5 = s5 + hp_n - pn9;
                }
                let sum = s1 + s2 + s3 + s4 + s5;
                let trnn = c1 + c2 + c3 + c4 + c5;
                (sum, trnn, [c1, c2, c3, c4, c5])
            };
            let (sum, trnn, cpn) = balance(lsctp, lslctp, lslnctp, lsntp, lmctp, lmntp, bmctp, bmntp, hsctp, hsntp, hpctp, hpntp);
            let wmin = (1.0e-5_f32).max(l.no3 + l.nh4 + sum);
            let reduc = if wmin < trnn { wmin / trnn } else { 1.0 };
            let (mut lscta, lsnta, mut lslcta, lslncta) = if cpn[0] > 0.0 {
                (lsctp * reduc, lsntp * reduc, lslctp * reduc, lslnctp * reduc)
            } else {
                (lsctp, lsntp, lslctp, lslnctp)
            };
            let (mut lmcta, lmnta) = if cpn[1] > 0.0 { (lmctp * reduc, lmntp * reduc) } else { (lmctp, lmntp) };
            let (bmcta, bmnta) = if cpn[2] > 0.0 { (bmctp * reduc, bmntp * reduc) } else { (bmctp, bmntp) };
            let (hscta, hsnta) = if cpn[3] > 0.0 { (hsctp * reduc, hsntp * reduc) } else { (hsctp, hsntp) };
            let (hpcta, hpnta) = if cpn[4] > 0.0 { (hpctp * reduc, hpntp * reduc) } else { (hpctp, hpntp) };
            let (sum, trnn, _) = balance(lscta, lslcta, lslncta, lsnta, lmcta, lmnta, bmcta, bmnta, hscta, hsnta, hpcta, hpnta);
            let rnmn = sum - trnn;
            if rnmn > 0.0 {
                let min_n = (l.no3 - rnmn).max(0.0);
                l.nh4 += min_n;
                l.no3 -= min_n;
            }
            let c = &mut l.cp;
            let hmp_rate = 1.4 * (hsnta + hpnta) / (c.hs.n + c.hp.n + 1.0e-6);
            let mut hmp = hmp_rate * c.hp.p;
            hmp = hmp.min(c.hp.p);
            c.hp.p -= hmp;
            l.lab += hmp;
            let c = &mut l.cp;
            let mut decr = (lscta + lmcta) / (c.str.c + c.meta.c + 1.0e-6);
            decr = decr.min(1.0);
            let rmp = decr * c.tot.p;
            c.tot.p -= rmp;
            l.lab += 0.8 * rmp;
            let c = &mut l.cp;
            c.hp.p += 0.2 * rmp;
            lscta = c.str.c.min(lscta);
            lslcta = c.lig.c.min(lslcta);
            let mut co2fstr = 0.3 * lslcta;
            co2fstr += a.a1co2 * lslncta;
            lmcta = c.meta.c.min(lmcta);
            let _ = co2fstr;
            let cfs3s1 = apx * hpcta;
            let (efs3s1, imms3s1, mnrs3s1) = np_flow(c.hp.c, c.hp.n, 1.0 / ncbm, cfs3s1);
            let cfs2s1 = asx * hscta;
            let (efs2s1, imms2s1, mnrs2s1) = np_flow(c.hs.c, c.hs.n, 1.0 / ncbm, cfs2s1);
            let cfmets1 = a1 * lmcta;
            let (efmets1, immmets1, mnrmets1) = np_flow(c.meta.c, c.meta.n, 1.0 / ncbm, cfmets1);
            let cfstrs1 = a1 * lslncta;
            let (efstrs1, immstrs1, mnrstrs1) = np_flow(c.str.c, c.str.n, 1.0 / ncbm, cfstrs1);
            let cfstrs2 = 0.7 * lslcta;
            let (efstrs2, immstrs2, mnrstrs2) = np_flow(c.str.c, c.str.n, 1.0 / nchs, cfstrs2);
            let cfs1s2 = bmcta * (1.0 - abp - a.abco2);
            let (efs1s2, imms1s2, mnrs1s2) = np_flow(c.microb.c, c.microb.n, 1.0 / nchs, cfs1s2);
            let cfs1s3 = bmcta * abp;
            let (efs1s3, imms1s3, mnrs1s3) = np_flow(c.microb.c, c.microb.n, 1.0 / nchs, cfs1s3);
            let cfs2s3 = hscta * asp;
            let (efs2s3, imms2s3, mnrs2s3) = np_flow(c.hs.c, c.hs.n, 1.0 / nchp, cfs2s3);
            let x3 = apx * hpcta + asx * hscta + a1 * (lmcta + lslncta);
            c.microb.c = c.microb.c - bmcta + x3;
            let x1 = 0.7 * lslcta + bmcta * (1.0 - abp - a.abco2);
            c.hs.c = c.hs.c - hscta + x1;
            let x1 = hscta * asp + bmcta * abp;
            c.hp.c = c.hp.c - hpcta + x1;
            c.lig.c = (1.0e-10_f32).max(c.lig.c - lslcta);
            c.nonlig.c = (1.0e-10_f32).max(c.nonlig.c - lslncta);
            c.str.c = c.nonlig.c + c.lig.c;
            c.lig.m = (1.0e-10_f32).max(c.lig.m - lslcta / 0.42);
            c.nonlig.m = (1.0e-10_f32).max(c.nonlig.m - lslncta / 0.42);
            c.str.m = (1.0e-10_f32).max(c.str.m - lscta / 0.42);
            if c.meta.m > 0.0 {
                let rto = (0.42_f32).max(c.meta.c / c.meta.m);
                c.meta.m -= lmcta / rto;
                c.meta.c -= lmcta;
            }
            if l.no3 < 0.0 {
                l.no3 = 0.0;
            }
            if l.nh4 < 0.0 {
                l.nh4 = 0.0;
            }
            let c = &mut l.cp;
            c.meta.n = (0.001_f32).max(c.meta.n - efmets1 - mnrmets1);
            c.str.n = (0.001_f32).max(c.str.n - efstrs1 - efstrs2 - mnrstrs1 - mnrstrs2);
            c.microb.n = c.microb.n + efmets1 + efstrs1 - efs1s2 - efs1s3 + efs2s1 + efs3s1 - mnrs1s2 - mnrs1s3
                + immmets1
                + immstrs1
                + imms2s1
                + imms3s1;
            c.hs.n = c.hs.n + efstrs2 + efs1s2 - efs2s1 - efs2s3 - mnrs2s1 - mnrs2s3 + immstrs2 + imms1s2;
            c.hp.n = c.hp.n + efs1s3 + efs2s3 - efs3s1 - mnrs3s1 + imms1s3 + imms2s3;
            c.tot.c = c.str.c + c.meta.c + c.hp.c + c.hs.c + c.microb.c;
            c.seq.c = c.hp.c + c.hs.c + c.microb.c;
        }
        denit
    }

    /// SWAT+ `nut_orgnc2`, on days with runoff and sediment: organic N leaves the top layer with
    /// the sediment, carbon with the sediment, and microbial carbon with the water down the
    /// profile. Returns the organic N in the sediment.
    pub(super) fn nut_orgnc2(&mut self, surfq: f32, sedyld: f32, enratio: f32, cb: &CarbonBasin, saved: &mut Saved) -> f32 {
        let area = self.area_ha;
        let erorgn = self.erorgn;
        let rsd_tot_n = self.rsd_tot.n;
        let nly = self.layers.len();
        let l = &mut self.layers[0];
        let c_ly1 = l.cp.hp.n + l.cp.hs.n + rsd_tot_n;
        let wt1 = l.bd * l.d / 100.0;
        let er = if erorgn > 0.001 { erorgn } else { enratio };
        let conc = c_ly1 * er / wt1;
        let sedorgn = 0.001 * conc * sedyld / area;
        if c_ly1 > 1.0e-6 {
            let n_left_rto = 1.0 - sedorgn / c_ly1;
            let c = &mut l.cp;
            c.tot.n *= n_left_rto;
            c.hs.n *= n_left_rto;
            c.hp.n *= n_left_rto;
            if let Some(pl) = self.plant.as_mut() {
                pl.rsd.n *= n_left_rto;
            }
            let l = &mut self.layers[0];
            let c = &mut l.cp;
            c.meta.n *= n_left_rto;
            c.str.n *= n_left_rto;
            c.lig.n *= n_left_rto;
        }
        let l = &mut self.layers[0];
        let sol_mass = (l.d / 1000.0) * 10000.0 * l.bd * 1000.0 * (1.0 - l.rock / 100.0);
        let mut c_vert = 0.0_f32;
        let ero_fr;
        {
            let c = &mut l.cp;
            c.tot.c = c.hp.c + c.hs.c + c.meta.c + c.str.c;
            ero_fr = ((sedyld / area) / (sol_mass / 1000.0)).min(0.9);
            c.tot.c *= 1.0 - ero_fr;
            c.hs.c *= 1.0 - ero_fr;
            c.hp.c *= 1.0 - ero_fr;
        }
        if let Some(pl) = self.plant.as_mut() {
            pl.rsd.c *= 1.0 - ero_fr;
        }
        let l = &mut self.layers[0];
        if l.cp.microb.c > 0.01 {
            let koc = cb.prmt_21;
            let c = &mut l.cp;
            c.tot.c = c.str.c + c.meta.c + c.hp.c + c.hs.c + c.microb.c;
            let c_microb_fac = 0.0001 * koc * c.tot.c;
            let flo_loss_co = l.ul + c_microb_fac;
            let flo_tot = surfq + l.prk + l.flat;
            if flo_tot > 1.0e-10 {
                let c_microb_loss = l.cp.microb.c * (1.0 - (-flo_tot / flo_loss_co).exp());
                let vert_conc = c_microb_loss / (l.prk + cb.prmt_44 * (surfq + l.flat));
                c_vert = vert_conc * l.prk;
                l.cp.microb.c -= c_microb_loss;
                if ero_fr > 0.0 {
                    let c_horiz = c_microb_fac * l.cp.microb.c / flo_loss_co;
                    saved.c_microb_sed = ero_fr * c_horiz;
                }
            }
        }
        // c_microb_sed keeps its value from the last time it was computed, for any HRU.
        let c = &mut l.cp;
        c.microb.c -= saved.c_microb_sed;
        c.tot.c = c.str.c + c.meta.c + c.hp.c + c.hs.c + c.microb.c;
        for ly in 1..nly {
            let l = &mut self.layers[ly];
            let c_microb_perc = l.cp.microb.c + c_vert;
            c_vert = 0.0;
            if c_microb_perc >= 0.01 {
                let flo_tot = l.prk + l.flat;
                if flo_tot > 0.0 {
                    c_vert = c_microb_perc * (1.0 - (-flo_tot / (l.por * l.thick - l.wpmm)).exp());
                }
            }
            let c = &mut l.cp;
            c.microb.c = c_microb_perc - c_vert;
            c.tot.c = c.str.c + c.meta.c + c.hp.c + c.hs.c + c.microb.c;
            c.seq.c = c.hp.c + c.hs.c + c.microb.c;
        }
        sedorgn
    }

    /// The nutrient and pool part of `mgt_biomix`, with the mixed shares `frac_mixed` (of each
    /// layer's mass taken into the mix) and `frac_dep` (of the mix given back), for the layers
    /// before `lyr_exit`; and `mgt_tillfactor` after it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn biomix_pools(
        &mut self,
        frac_mixed: &[f32],
        frac_non_mixed: &[f32],
        frac_dep: &[f32],
        sol_mass: &[f32],
        avg_emix: f32,
        lyr_exit: usize,
        dtil: f32,
        cb: &CarbonBasin,
    ) {
        let mut mix_no3 = 0.0_f32;
        let mut mix_nh4 = 0.0_f32;
        let (mut wsol, mut lab, mut act, mut sta) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        let mut m = Pools::default();
        let (mut mix_clay, mut mix_silt, mut mix_sand) = (0.0_f32, 0.0_f32, 0.0_f32);
        let mut mix_rsd = OrgMass::default();
        let surf = self.plant.as_ref().map(|p| p.rsd);
        for l in 0..lyr_exit.min(frac_mixed.len()) {
            let fm = frac_mixed[l];
            let ly = &self.layers[l];
            mix_sand += fm * ly.sand * sol_mass[l] / 100.0;
            mix_silt += fm * ly.silt * sol_mass[l] / 100.0;
            mix_clay += fm * ly.clay * sol_mass[l] / 100.0;
            mix_no3 += fm * ly.no3;
            mix_nh4 += fm * ly.nh4;
            wsol += fm * ly.wsol;
            lab += fm * ly.lab;
            act += fm * ly.act;
            sta += fm * ly.sta;
            m.tot = m.tot + OrgMass::scale(fm, ly.cp.tot);
            m.hs = m.hs + OrgMass::scale(fm, ly.cp.hs);
            m.hp = m.hp + OrgMass::scale(fm, ly.cp.hp);
            m.microb = m.microb + OrgMass::scale(fm, ly.cp.microb);
            m.str = m.str + OrgMass::scale(fm, ly.cp.str);
            m.lig = m.lig + OrgMass::scale(fm, ly.cp.lig);
            m.meta = m.meta + OrgMass::scale(fm, ly.cp.meta);
            if let Some(s) = surf {
                mix_rsd = mix_rsd + OrgMass::scale(frac_dep[l] * avg_emix, ly.rsd);
                let surf_rsd = OrgMass::scale(frac_dep[l] * avg_emix, s);
                mix_rsd = mix_rsd + surf_rsd;
            }
        }
        // Humus pools of the old model (hact, hsta) are mixed too, but are empty with cswat 2.
        for l in 0..lyr_exit {
            let fnm = frac_non_mixed[l];
            let fd = frac_dep[l];
            let ly = &mut self.layers[l];
            ly.no3 = fnm * ly.no3 + fd * mix_no3;
            ly.nh4 = fnm * ly.nh4 + fd * mix_nh4;
            ly.wsol = fnm * ly.wsol + fd * wsol;
            ly.lab = fnm * ly.lab + fd * lab;
            ly.act = fnm * ly.act + fd * act;
            ly.sta = fnm * ly.sta + fd * sta;
            let mix = |a: OrgMass, b: OrgMass| OrgMass::scale(fnm, a) + OrgMass::scale(fd, b);
            ly.cp.tot = mix(ly.cp.tot, m.tot);
            if let Some(pl) = self.plant.as_mut() {
                ly.rsd = mix(ly.rsd, mix_rsd);
                let surf_rsd = OrgMass::scale(fd * avg_emix, pl.rsd);
                pl.rsd = pl.rsd - surf_rsd;
                self.rsd_tot = self.rsd_tot - surf_rsd;
            }
            let c = &mut ly.cp;
            c.hs = mix(c.hs, m.hs);
            c.hp = mix(c.hp, m.hp);
            c.microb = mix(c.microb, m.microb);
            c.str = mix(c.str, m.str);
            c.lig = mix(c.lig, m.lig);
            c.meta = mix(c.meta, m.meta);
            let mass = sol_mass[l];
            ly.clay = 100.0 / mass * (fnm * ly.clay * mass / 100.0 + fd * mix_clay);
            ly.silt = 100.0 / mass * (fnm * ly.silt * mass / 100.0 + fd * mix_silt);
            ly.sand = 100.0 / mass * (fnm * ly.sand * mass / 100.0 + fd * mix_sand);
        }
        self.tillfactor(avg_emix, dtil, cb);
    }

    /// SWAT+ `mgt_tillfactor` for a biological mixing event (no tillage): each thawed layer's
    /// mixing effect on decomposition. `emix` is reduced for the layer the mixing depth ends in
    /// and is 0 below it, for good.
    fn tillfactor(&mut self, mut emix: f32, dtil: f32, cb: &CarbonBasin) {
        for l in 0..self.layers.len() {
            let prev = if l == 0 { 0.0 } else { self.layers[l - 1].d };
            let ly = &mut self.layers[l];
            if ly.d <= dtil {
            } else if ly.d > dtil && prev < dtil {
                emix = emix * (dtil - prev) / ly.thick;
            } else {
                emix = 0.0;
            }
            let zz = cb.bmix_a + cb.bmix_b * (cb.bmix_c * ly.clay / 100.0).exp();
            let csdr = 0.0 + emix;
            if ly.tmp <= 0.0 {
                ly.tillagef_biomix = 0.0;
            } else {
                ly.tillagef_biomix = zz * (csdr / (csdr + (1.0 - 2.0 * csdr).exp()));
                ly.tillagef = 0.0 + ly.tillagef_biomix;
            }
        }
    }
}
