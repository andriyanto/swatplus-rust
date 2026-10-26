//! Soil nutrients with the original carbon model (`cswat` 0), residue decomposition, and the
//! erosion that carries organic nitrogen and phosphorus away: `soil_nutcarb_init`, `rsd_decomp`,
//! `nut_nminrl`, `nut_nitvol`, `nut_pminrl`, `nut_nlch`, `nut_solp`, `ero_pkq`, `ero_eiusle`,
//! `ero_cfactor`, `ero_ysed`, `pest_enrsb`, `nut_orgn`, `nut_psed`, and the year-end mixing
//! (`mgt_newtillmix_cswat0`).

use std::ops::{Add, Sub};

use super::Hru;
use super::plant::{OrgMass, Saved, exp_w};

/// Basin values for the nutrient and erosion routines (`bsn_prm`, `hru(j)%nut`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutBasin {
    pub cmn: f32,
    pub cdn: f32,
    pub sdnco: f32,
    pub psp: f32,
    pub phoskd: f32,
    pub pperco: f32,
    pub nperco: f32,
    pub decr_min: f32,
    pub prf: f32,
    pub rsd_covco: f32,
    pub sed_det: i32,
    /// `n_act_frac` (carbon module, 0.02 unless `carbon.bsn` sets it).
    pub n_act_frac: f32,
    /// Correct SWAT+'s bugs instead of reproducing them (`RunOptions::fixes`).
    pub fixes: bool,
}

impl NutBasin {
    pub fn new(project: &swat_model::Project) -> Self {
        let p = |n: &str| project.params.get(n).unwrap_or(0.0);
        let n_act_frac = project.carbon.as_ref().and_then(|t| t.real(0, "n_act_frac")).filter(|_| project.codes.cswat == 2).unwrap_or(0.02);
        Self {
            cmn: p("cmn"),
            cdn: p("cdn"),
            sdnco: p("sdnco"),
            psp: p("psp"),
            phoskd: p("phoskd"),
            pperco: p("pperco"),
            nperco: p("nperco"),
            decr_min: p("decr_min"),
            prf: p("prf"),
            rsd_covco: p("rsd_covco"),
            sed_det: project.codes.sed_det,
            n_act_frac,
            fixes: false,
        }
    }
}

/// A row of `nutrients.sol` (`soiltest_db`); the defaults when the soil-plant entry's name isn't
/// found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoilTest {
    pub exp_co: f32,
    pub lab_p: f32,
    pub nitrate: f32,
    pub fr_hum_act: f32,
    pub hum_c_n: f32,
    pub hum_c_p: f32,
}

impl Default for SoilTest {
    fn default() -> Self {
        Self { exp_co: 0.001, lab_p: 5.0, nitrate: 7.0, fr_hum_act: 0.02, hum_c_n: 10.0, hum_c_p: 80.0 }
    }
}

/// The day's nutrient flows the outputs report (`hnb_d`, `hls_d` before the lag).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NutDay {
    pub act_nit_n: f32,
    pub org_lab_p: f32,
    pub act_sta_n: f32,
    pub denit: f32,
    pub lab_min_p: f32,
    pub act_sta_p: f32,
    pub qp_cms: f32,
    pub usle_ei: f32,
    pub usle: f32,
    pub sedyld: f32,
    pub enratio: f32,
    pub sedorgn: f32,
    pub sedorgp: f32,
    pub sedminpa: f32,
    pub sedminps: f32,
    pub surqno3: f32,
    pub latno3: f32,
    pub percn: f32,
    pub surqsolp: f32,
    pub lchlabp: f32,
}

fn sut_cdg(st: f32, fc: f32, tmp: f32) -> (f32, f32, f32) {
    let mut sut = 0.1 + 0.9 * (st / fc).sqrt();
    sut = sut.max(0.05);
    let xx = tmp;
    let mut cdg = 0.9 * xx / (xx + (9.93_f32 - 0.312 * xx).exp()) + 0.1;
    cdg = cdg.max(0.1);
    let mut xx = cdg * sut;
    if xx < 0.0 {
        xx = 0.0;
    }
    if xx > 1.0e6 {
        xx = 1.0e6;
    }
    (sut, cdg, xx.sqrt())
}

/// The decomposition rate factor from a residue's C:N and C:P ratios (`cnrf`, `cprf`).
fn residue_ca(r: &OrgMass) -> f32 {
    let cnrf = if r.n > 1.0e-4 {
        let mut cnr = r.c / r.n;
        if cnr > 500.0 {
            cnr = 500.0;
        }
        (-0.693_f32 * (cnr - 25.0) / 25.0).exp()
    } else {
        1.0
    };
    let cprf = if r.p > 1.0e-4 {
        let mut cpr = r.c / r.p;
        if cpr > 5000.0 {
            cpr = 5000.0;
        }
        (-0.693_f32 * (cpr - 200.0) / 200.0).exp()
    } else {
        1.0
    };
    cnrf.min(cprf).min(1.0)
}

impl Hru {
    /// `soil_nutcarb_init` for `cswat` 0: mineral N and P by depth, and humus pools.
    pub(super) fn nutcarb_init(&mut self, t: &SoilTest, psp: f32, cswat: i32) {
        for l in self.layers.iter_mut() {
            l.cbn = l.cbn.max(0.001);
        }
        for l in self.layers.iter_mut() {
            let conv_wt = l.bd * l.thick / 100.0;
            let wt1 = conv_wt;
            let dep_frac = (-t.exp_co * l.d).exp();
            l.no3 = if t.nitrate > 1.0e-9 { t.nitrate * dep_frac } else { 7.0 * dep_frac };
            l.no3 *= wt1;
            l.lab = if t.lab_p > 1.0e-9 { t.lab_p * dep_frac } else { 5.0 * dep_frac };
            l.lab *= wt1;
            l.act = l.lab * (1.0 - psp) / psp;
            l.sta = 4.0 * l.act;
        }
        if cswat != 0 {
            return;
        }
        let fr = if t.fr_hum_act < 1.0e-9 { 0.02 } else { t.fr_hum_act };
        for l in self.layers.iter_mut() {
            let tot_mass = 10000.0 * l.thick * l.bd;
            let tot = OrgMass { m: tot_mass * (l.cbn / 100.0) / 0.58, c: tot_mass * (l.cbn / 100.0), ..OrgMass::default() };
            let tot = OrgMass { n: tot.c / 10.0, p: tot.c / 100.0, ..tot };
            l.hact.m = fr * tot.m;
            l.hact.c = fr * tot.c;
            l.hact.n = l.hact.c / t.hum_c_n;
            l.hact.p = l.hact.c / t.hum_c_p;
            l.hsta.m = (1.0 - fr) * tot.m;
            l.hsta.c = (1.0 - fr) * tot.c;
            l.hsta.n = l.hsta.c / t.hum_c_n;
            l.hsta.p = l.hsta.c / t.hum_c_p;
        }
    }

    /// SWAT+ `rsd_decomp`: surface residue decomposes into the top layer.
    pub(super) fn rsd_decomp(&mut self, b: &NutBasin) {
        let (st, fc, tmp) = match self.layers.first() {
            Some(l) => (l.st, l.fc, l.tmp),
            None => return,
        };
        for pl in self.plants.iter_mut() {
            if tmp <= 0.0 {
                continue;
            }
            let ca = residue_ca(&pl.rsd);
            let st = if st < 0.0 { 0.0000001 } else { st };
            let (_, _, csf) = sut_cdg(st, fc, tmp);
            let mut decr = pl.db.rsdco_pl * ca * csf;
            decr = decr.max(b.decr_min);
            decr = decr.min(1.0);
            let decomp = OrgMass::scale(decr, pl.rsd);
            pl.rsd = pl.rsd.sub(decomp);
            let l = &mut self.layers[0];
            l.no3 += 0.8 * decomp.n;
            l.hact.n += 0.2 * decomp.n;
            l.lab += 0.8 * decomp.p;
            l.hact.p += 0.2 * decomp.p;
        }
        self.rsd_tot = OrgMass::default();
        for pl in &self.plants {
            self.rsd_tot = self.rsd_tot.add(pl.rsd);
        }
    }

    /// SWAT+ `nut_nminrl`: humus mineralization, soil residue decomposition and denitrification.
    /// The top layer's rates use the second layer's water and temperature.
    pub(super) fn nut_nminrl(&mut self, b: &NutBasin, nd: &mut NutDay) {
        for k in 0..self.layers.len() {
            let kk = if k == 0 { 1.min(self.layers.len() - 1) } else { k };
            let (st, fc, tmp) = (self.layers[kk].st, self.layers[kk].fc, self.layers[kk].tmp);
            if tmp <= 0.0 {
                continue;
            }
            let (sut, cdg, csf) = sut_cdg(st, fc, tmp);
            let l = &mut self.layers[k];
            let mut rwn = 0.1e-4 * (l.hact.n * (1.0 / b.n_act_frac - 1.0) - l.hsta.n);
            if rwn > 0.0 {
                rwn = rwn.min(l.hact.n);
            } else {
                rwn = -(rwn.abs().min(l.hsta.n));
            }
            l.hsta.n = (l.hsta.n + rwn).max(1.0e-6);
            l.hact.n = (l.hact.n - rwn).max(1.0e-6);
            nd.act_sta_n += rwn;
            let mut hmn = b.cmn * csf * l.hact.n;
            hmn = hmn.min(l.hact.n);
            let xx = l.hsta.n + l.hact.n;
            let mut hmp = if xx > 1.0e-6 { 1.4 * hmn * l.hsta.p / xx } else { 0.0 };
            hmp = hmp.min(l.hsta.p);
            l.hact.n = (l.hact.n - hmn).max(1.0e-6);
            l.no3 += hmn;
            l.hsta.p -= hmp;
            l.lab += hmp;
            nd.act_nit_n += hmn;
            nd.org_lab_p += hmp;
            for pl in self.plants.iter_mut() {
                let r = &mut pl.soil_rsd[k];
                let ca = residue_ca(r);
                let mut decr = pl.db.rsdco_pl * ca * csf;
                decr = decr.max(b.decr_min);
                decr = decr.min(1.0);
                let decomp = OrgMass::scale(decr, *r);
                *r = r.sub(decomp);
                if r.m < 1.0e-10 {
                    r.m = 0.0;
                }
                if r.c < 1.0e-10 {
                    r.c = 0.0;
                }
                if r.n < 1.0e-10 {
                    r.n = 0.0;
                }
                if r.p < 1.0e-10 {
                    r.p = 0.0;
                }
                let l = &mut self.layers[k];
                l.no3 += 0.8 * decomp.n;
                l.hact.n += 0.2 * decomp.n;
                l.lab += 0.8 * decomp.p;
                l.hsta.p += 0.2 * decomp.p;
            }
            let l = &mut self.layers[k];
            let wdn = if sut >= b.sdnco { l.no3 * (1.0 - (-b.cdn * cdg * l.cbn / 100.0).exp()) } else { 0.0 };
            l.no3 = (l.no3 - wdn).max(0.0001);
            nd.denit += wdn;
        }
    }

    /// SWAT+ `nut_nitvol`: nitrification and volatilization of ammonium.
    pub(super) fn nut_nitvol(&mut self) {
        let cecf = 0.15_f32;
        for k in 0..self.layers.len() {
            let xx = if k == 0 { 0.0 } else { self.layers[k - 1].d };
            let l = &mut self.layers[k];
            let tf = 0.41 * (l.tmp - 5.0) / 10.0;
            if l.nh4 > 0.0 && tf >= 0.001 {
                let sw25 = l.wpmm + 0.25 * l.fc;
                let swwp = l.wpmm + l.st;
                let swf = if swwp < sw25 { (swwp - l.wpmm) / (sw25 - l.wpmm) } else { 1.0 };
                let dmidl = (l.d + xx) / 2.0;
                let dpf = 1.0 - dmidl / (dmidl + (4.706_f32 - 0.0305 * dmidl).exp());
                let akn = tf * swf;
                let akv = tf * dpf * cecf;
                let rnv = l.nh4 * (1.0 - (-akn - akv).exp());
                let mut rnit = 1.0 - (-akn).exp();
                let mut rvol = 1.0 - (-akv).exp();
                if rvol + rnit > 1.0e-6 {
                    rvol = rnv * rvol / (rvol + rnit);
                    rnit = rnv - rvol;
                    if rnit < 0.0 {
                        rnit = 0.0;
                    }
                    l.nh4 = (l.nh4 - rnit).max(1.0e-6);
                }
                if l.nh4 < 0.0 {
                    rnit += l.nh4;
                    l.nh4 = 0.0;
                }
                l.no3 += rnit;
                l.nh4 = (l.nh4 - rvol).max(1.0e-6);
                if l.nh4 < 0.0 {
                    l.nh4 = 0.0;
                }
            }
        }
    }

    /// SWAT+ `nut_pminrl`: phosphorus between the labile, active and stable pools.
    pub(super) fn nut_pminrl(&mut self, b: &NutBasin, nd: &mut NutDay) {
        let bk = 0.01_f32;
        let rto = b.psp / (1.0 - b.psp);
        for l in self.layers.iter_mut() {
            let mut rmp1 = l.lab - l.act * rto;
            if rmp1 > 0.0 {
                rmp1 *= 0.1;
            }
            if rmp1 < 0.0 {
                rmp1 *= 0.6;
            }
            rmp1 = rmp1.min(l.lab);
            let mut roc = bk * (4.0 * l.act - l.sta);
            if roc < 0.0 {
                roc *= 0.1;
            }
            roc = roc.min(l.act);
            l.sta += roc;
            if l.sta < 0.0 {
                l.sta = 0.0;
            }
            l.act = l.act - roc + rmp1;
            if l.act < 0.0 {
                l.act = 0.0;
            }
            l.lab -= rmp1;
            if l.lab < 0.0 {
                l.lab = 0.0;
            }
            nd.lab_min_p += rmp1;
            nd.act_sta_p += roc;
        }
    }

    /// `ero_cfactor`, which sets the C factor method to 1 before it tests it: the cover factor
    /// from residue and above-ground biomass.
    pub(super) fn c_factor(&self, rsd_covco: f32) -> f32 {
        let rsd_sumfac = (self.rsd_tot.m + 1.0) / 1000.0;
        let rsd_covfact = exp_w(-rsd_covco * rsd_sumfac);
        let ab_gr_t = self.ab_gr_com.m / 1000.0;
        let grcov_frac = ab_gr_t / (ab_gr_t + exp_w(1.175 - 1.748 * ab_gr_t));
        let mut bio_covfact = 1.0 - grcov_frac * exp_w(-0.328 * self.cht_mx);
        bio_covfact = bio_covfact.max(1.0e-10);
        bio_covfact = bio_covfact.min(1.0);
        (rsd_covfact * bio_covfact).max(1.0e-10)
    }

    /// Erosion in `surface` (`ero_pkq`, `ero_eiusle`, `ero_cfactor`, `ero_ysed`): peak runoff and
    /// sediment yield of the day's runoff `qday`. `precip` is the HRU's, `phh` the station's
    /// fraction of rain in the peak half hour.
    pub(super) fn erosion(&self, qday: f32, surfq: f32, precip: f32, phh: f32, b: &NutBasin, nd: &mut NutDay) {
        if qday > 1.0e-6 {
            nd.qp_cms = if b.sed_det == 1 {
                let xx = 2.0 * self.tconc * (1.0 - phh).ln();
                let altc = 1.0 - xx.exp();
                let q = altc * qday / self.tconc;
                q * self.km / 3.6
            } else {
                let qp_cfs = b.prf / 6578.6 * self.area_ha * qday / self.tconc;
                qp_cfs / 35.3
            };
        }
        if qday > 1.0e-6 && nd.qp_cms > 1.0e-6 && precip > 1.0e-4 {
            let xb = -2.0 * (1.0 - phh).ln();
            let pkrf30 = 2.0 * precip * phh;
            let pkrf = xb * precip;
            nd.usle_ei = precip * (12.1 + 8.9 * (pkrf.log10() - 0.4343)) * pkrf30 / 1000.0;
            if nd.usle_ei < 1.0e-4 {
                nd.usle_ei = 0.0;
            }
        }
        if surfq > 1.0e-6 && nd.qp_cms > 1.0e-6 {
            let c = self.c_factor(b.rsd_covco);
            // ero_ysed
            let cklsp = c * self.usle_mult;
            let mut sedyld = (10.0 * surfq * nd.qp_cms * self.area_ha).powf(0.56) * cklsp;
            if sedyld < 0.0 {
                sedyld = 0.0;
            }
            // The snow test is the wrong way round: snow only zeroes tiny yields, and without
            // snow the yield is divided by exp(0). Corrected: snow over 100 mm stops erosion,
            // less reduces it (as in SWAT 2012).
            if b.fixes {
                if self.sno_mm > 100.0 {
                    sedyld = 0.0;
                } else if self.sno_mm > 0.0 {
                    sedyld /= (self.sno_mm * 3.0 / 25.4).exp();
                }
            } else if self.sno_mm > 0.0 {
                if sedyld < 1.0e-6 {
                    sedyld = 0.0;
                }
            } else if self.sno_mm > 100.0 {
                sedyld = 0.0;
            } else {
                sedyld /= (self.sno_mm * 3.0 / 25.4).exp();
            }
            nd.sedyld = sedyld;
            nd.usle = 1.292 * nd.usle_ei * cklsp / 11.8;
        }
    }

    /// `pest_enrsb`, `nut_orgn` and `nut_psed`: the enrichment ratio, and the organic N and the
    /// P that leave with the sediment.
    /// `orgn` is false with the carbon model on, which has its own routine (`nut_orgnc2`, not
    /// ported yet) instead of `nut_orgn`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sediment_nutrients(
        &mut self,
        surfq: f32,
        erorgn: f32,
        erorgp: f32,
        orgn: bool,
        fixes: bool,
        nd: &mut NutDay,
        saved: &mut Saved,
    ) {
        if nd.sedyld < 1.0e-4 {
            nd.sedyld = 0.0;
        }
        let cy = 0.1 * nd.sedyld / (self.area_ha * surfq + 1.0e-6);
        nd.enratio = if cy > 1.0e-6 { 0.78 * cy.powf(-0.2468) } else { 0.0 };
        if nd.enratio > 3.0 {
            nd.enratio = 3.0;
        }
        saved.enratio = nd.enratio;
        let area = self.area_ha;
        let l = &mut self.layers[0];
        // nut_orgn
        let orgn_kgha = l.hsta.n + l.hact.n;
        let wt1 = 10000.0 * l.bd * l.d;
        let er = if erorgn > 0.001 { erorgn } else { nd.enratio };
        let frac = orgn_kgha * er / wt1;
        if orgn {
            nd.sedorgn = 1000.0 * frac * nd.sedyld / area;
        }
        if orgn && orgn_kgha > 1.0e-6 {
            l.hact.n -= nd.sedorgn * (l.hact.n / orgn_kgha);
            l.hsta.n -= nd.sedorgn * (l.hsta.n / orgn_kgha);
            if l.hact.n < 0.0 {
                nd.sedorgn += l.hact.n;
                l.hact.n = 0.0;
            }
            if l.hsta.n < 0.0 {
                nd.sedorgn += l.hsta.n;
                l.hsta.n = 0.0;
            }
        }
        // nut_psed (no manure). The mineral fractions are swapped: the "active" share is taken
        // from the stable pool's size and the "stable" share from the active pool's.
        let man_p = 0.0_f32;
        let sedp_attach = l.hsta.p + man_p + man_p + l.sta + l.act;
        if sedp_attach > 1.0e-9 {
            saved.fr_orgp = (l.hsta.p + man_p + man_p) / sedp_attach;
            // Swapped in SWAT+; corrected, each share from its own pool.
            if fixes {
                saved.fr_actmin = l.act / sedp_attach;
                saved.fr_stamin = l.sta / sedp_attach;
            } else {
                saved.fr_actmin = l.sta / sedp_attach;
                saved.fr_stamin = l.act / sedp_attach;
            }
        }
        let er = if erorgp > 0.001 { erorgp } else { nd.enratio };
        let frac = sedp_attach * er / wt1;
        let sedp = 1000.0 * frac * nd.sedyld / area;
        if sedp > 1.0e-9 {
            nd.sedorgp = sedp * saved.fr_orgp;
            nd.sedminpa = sedp * saved.fr_actmin;
            nd.sedminpa = nd.sedminpa.min(l.act);
            l.act -= nd.sedminpa;
            nd.sedminps = sedp * saved.fr_stamin;
            nd.sedminps = nd.sedminps.min(l.sta);
            l.sta -= nd.sedminps;
            let sed_orgp = l.hsta.p + man_p;
            if sed_orgp > 1.0e-6 {
                let mut sed_hump = nd.sedorgp * (l.hsta.p / sed_orgp);
                sed_hump = sed_hump.min(l.hsta.p);
                l.hsta.p -= sed_hump;
            }
        } else {
            nd.sedorgp = 0.0;
            nd.sedminpa = 0.0;
            nd.sedminps = 0.0;
        }
    }

    /// SWAT+ `nut_nlch`: nitrate in runoff, lateral flow and percolation, layer by layer.
    pub(super) fn nut_nlch(&mut self, surfq: f32, b: &NutBasin, nd: &mut NutDay) {
        let anion_excl = self.anion_excl;
        let nly = self.layers.len();
        let mut percnlyr = 0.0_f32;
        for jj in 0..nly {
            let l = &mut self.layers[jj];
            l.no3 += percnlyr;
            if l.no3 < 1.0e-6 {
                l.no3 = 0.0;
            }
            let sro = if jj == 0 { surfq } else { 0.0 };
            let vv = l.prk + sro + l.flat + 1.0e-10;
            let mut ww = -vv / ((1.0 - anion_excl) * l.ul);
            if ww < -80.0 {
                ww = -80.0;
            }
            let vno3 = l.no3 * (1.0 - ww.exp());
            let co = (vno3 / vv).max(0.0);
            if jj == 0 {
                nd.surqno3 = surfq * b.nperco * co;
                nd.surqno3 = nd.surqno3.min(l.no3);
                l.no3 = (l.no3 - nd.surqno3).max(0.0001);
            }
            let mut ssfnlyr = co * l.flat;
            ssfnlyr = ssfnlyr.min(l.no3);
            nd.latno3 += ssfnlyr;
            l.no3 = (l.no3 - ssfnlyr).max(0.0001);
            percnlyr = co * l.prk;
            percnlyr = percnlyr.min(l.no3);
            l.no3 = (l.no3 - percnlyr).max(0.0001);
            if jj == nly - 1 {
                nd.percn = percnlyr;
            }
        }
    }

    /// SWAT+ `nut_solp`: soluble P in runoff, and P leached down the profile.
    pub(super) fn nut_solp(&mut self, surfq: f32, b: &NutBasin, nd: &mut NutDay) {
        let nly = self.layers.len();
        {
            let l = &mut self.layers[0];
            l.lab += 0.0;
            let xx = l.bd * l.d * b.phoskd;
            nd.surqsolp = l.lab * surfq / (xx + 1.0);
            nd.surqsolp = nd.surqsolp.min(l.lab);
            nd.surqsolp = nd.surqsolp.max(0.0);
            l.lab -= nd.surqsolp;
        }
        for ly in 0..nly {
            let l = &mut self.layers[ly];
            let vap = -l.prk / (0.01 * l.st + 0.1 * b.pperco * l.bd);
            let mut plch = 0.001 * l.lab * (1.0 - exp_w(vap));
            plch = plch.min(l.lab);
            l.lab -= plch;
            if ly == nly - 1 {
                nd.lchlabp = plch;
            } else {
                self.layers[ly + 1].lab += plch;
            }
        }
    }

    /// `mgt_newtillmix_cswat0` as `time_control` calls it at the end of each year with the HRU's
    /// biological mixing efficiency (`hyd%biomix`): the layers above 50 mm exchange that share of
    /// their water, nutrients and humus, and the surface residue is partly mixed in.
    pub(super) fn year_end_mixing(&mut self, emix: f32) {
        if emix <= 1.0e-6 {
            return;
        }
        let nly = self.layers.len();
        let Some(last) = self.layers.last() else { return };
        let mut dtil = last.d.min(50.0);
        if dtil <= 0.0 {
            return;
        }
        let bd1 = self.layers[0].bd;
        let sol_mass: Vec<f32> = self.layers.iter().map(|l| (l.thick / 1000.0) * 10000.0 * bd1 * 1000.0 * (1.0 - l.rock / 100.0)).collect();
        if dtil < 10.0 {
            dtil = 11.0;
        }
        let mut sol_msn = vec![0.0_f32; nly];
        let mut frac_dep = vec![0.0_f32; nly];
        let mut mix_sw = 0.0_f32;
        let (mut mix_no3, mut mix_nh4) = (0.0_f32, 0.0_f32);
        let (mut mix_wsol, mut mix_lab, mut mix_act, mut mix_sta) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        let mut mix_hact = OrgMass::default();
        let mut mix_hsta = OrgMass::default();
        let mut mix_rsd = vec![OrgMass::default(); self.plants.len()];
        for l in 0..nly {
            let prev = if l == 0 { 0.0 } else { self.layers[l - 1].d };
            let ly = &self.layers[l];
            let sol_msm;
            if ly.d <= dtil {
                sol_msm = emix * sol_mass[l];
                sol_msn[l] = sol_mass[l] - sol_msm;
                frac_dep[l] = ly.thick / dtil;
            } else if ly.d > dtil && prev < dtil {
                sol_msm = emix * sol_mass[l] * (dtil - prev) / ly.thick;
                sol_msn[l] = sol_mass[l] - sol_msm;
                frac_dep[l] = (dtil - prev) / dtil;
            } else {
                sol_msm = 0.0;
                sol_msn[l] = sol_mass[l];
                frac_dep[l] = 0.0;
            }
            let fm = sol_msm / sol_mass[l];
            mix_sw += fm * ly.st;
            mix_no3 += fm * ly.no3;
            mix_nh4 += fm * ly.nh4;
            mix_wsol += fm * ly.wsol;
            mix_lab += fm * ly.lab;
            mix_act += fm * ly.act;
            mix_sta += fm * ly.sta;
            for (m, pl) in mix_rsd.iter_mut().zip(&self.plants) {
                *m = m.add(OrgMass::scale(fm, pl.rsd));
            }
            mix_hact = mix_hact.add(OrgMass::scale(fm, ly.hact));
            mix_hsta = mix_hsta.add(OrgMass::scale(fm, ly.hsta));
        }
        for l in 0..nly {
            let fnm = sol_msn[l] / sol_mass[l];
            let fd = frac_dep[l];
            let ly = &mut self.layers[l];
            ly.no3 = fnm * ly.no3 + fd * mix_no3;
            ly.nh4 = fnm * ly.nh4 + fd * mix_nh4;
            ly.wsol = fnm * ly.wsol + fd * mix_wsol;
            ly.lab = fnm * ly.lab + fd * mix_lab;
            ly.act = fnm * ly.act + fd * mix_act;
            ly.sta = fnm * ly.sta + fd * mix_sta;
            for (m, pl) in mix_rsd.iter().zip(self.plants.iter_mut()) {
                let r = &mut pl.soil_rsd[l];
                *r = OrgMass::scale(fnm, *r).add(OrgMass::scale(fd, *m));
                let surf_rsd = OrgMass::scale(emix * fd, pl.rsd);
                *r = r.add(surf_rsd);
                pl.rsd = pl.rsd.sub(surf_rsd);
                self.rsd_tot = self.rsd_tot.sub(surf_rsd);
            }
            ly.hact = OrgMass::scale(fnm, ly.hact).add(OrgMass::scale(fd, mix_hact));
            ly.hsta = OrgMass::scale(fnm, ly.hsta).add(OrgMass::scale(fd, mix_hsta));
            ly.st = fnm * ly.st + fd * mix_sw;
        }
    }
}
