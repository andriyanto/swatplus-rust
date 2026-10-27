//! Sub-daily time steps (`time.sim` step > 1) on the land: Green and Ampt infiltration
//! (`sq_greenampt`), splash and overland erosion by time step (`ero_ovrsed`), and the lags of
//! the runoff and sediment by time step (`sq_surfst`, `stor_surfstor`).

use super::Hru;

/// The most time steps a day the port keeps (15 minutes).
pub const MAX_STEP: usize = 96;

/// What an HRU keeps for sub-daily steps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubDaily {
    /// Time steps a day and their length (minutes, `time%dtm`).
    pub step: usize,
    pub dtm: f32,
    /// Green and Ampt infiltration (`gampt` 1); otherwise the day's curve number runoff is
    /// spread over the steps by a unit hydrograph when routed.
    pub ga: bool,
    /// Wetting front suction (`wfsh`, mm), from the top layer's porosity, clay and sand.
    pub wfsh: f32,
    /// The plants' average log of their minimum C factor (`cvm_com`).
    pub cvm_com: f32,
    /// A wet day's infiltration rate is carried to the next (`swtrg`, `rateinf_prev`).
    pub swtrg: bool,
    pub rateinf_prev: f32,
    /// Initial abstraction left on the impervious area (`urb_abstinit`).
    pub urb_abstinit: f32,
    /// The runoff and sediment held back at the end of each step (`hhsurf_bs(1:2, j, :)`).
    pub bs_q: Vec<f32>,
    pub bs_sed: Vec<f32>,
}

/// The basin parameters the sub-daily steps use.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SubBasin {
    pub cn_froz: f32,
    pub urb_init_abst: f32,
    pub eros_spl: f32,
    pub rill_mult: f32,
    pub eros_expo: f32,
    pub c_factor: f32,
}

/// `soils_init`: the wetting front suction of a soil from its top layer (Rawls and Brakensiek),
/// with sand as `soil_phys_init` leaves it for sub-daily steps (100 − clay − silt).
pub fn wfsh(por: f32, clay: f32, sand: f32) -> f32 {
    let x = 6.5309 - 7.32561 * por + 3.809479 * (por * por) + 0.001583 * (clay * clay) + 0.000344 * sand * clay - 0.049837 * por * sand
        + 0.001608 * (por * por) * (sand * sand)
        + 0.001602 * (por * por) * (clay * clay)
        - 0.0000136 * (sand * sand) * clay
        - 0.003479 * (clay * clay) * por
        - 0.000799 * (sand * sand) * por;
    10.0 * x.exp()
}

impl Hru {
    /// `sq_greenampt`: the day's runoff by time step (`hhqday`, and with the impervious area's
    /// `hhsurfq`), from the steps' rain after the canopy `ts` (and the station's, `sta_ts`, for
    /// the impervious area). `cnday` is updated each step; `ubnrunoff` is SWAT+'s array of the
    /// impervious runoff, shared by all HRUs (only urban HRUs set it). Returns the day's runoff.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn green_ampt(
        &mut self,
        ts: &[f32],
        sta_ts: &[f32],
        cnday: &mut f32,
        pet_day: f32,
        b: &SubBasin,
        ubnrunoff: &mut [f32],
        hhqday: &mut [f32],
        hhsurfq: &mut [f32],
    ) -> f32 {
        let Some(sd) = self.sub.as_mut() else { return 0.0 };
        let n = sd.step;
        let dtm = sd.dtm;
        let mut cumr = vec![0.0_f32; n + 2];
        let mut cuminf = vec![0.0_f32; n + 2];
        let mut excum = vec![0.0_f32; n + 2];
        let mut exinc = vec![0.0_f32; n + 2];
        let mut rateinf = vec![0.0_f32; n + 2];
        let mut rintns = vec![0.0_f32; n + 2];
        let l1 = self.layers[0];
        let dthet;
        if sd.swtrg {
            sd.swtrg = false;
            dthet = 0.001 * l1.por * 0.95;
            rateinf[1] = sd.rateinf_prev;
            sd.rateinf_prev = 0.0;
        } else {
            let soilw = if self.sw >= self.sumfc { 0.999 * self.sumfc } else { self.sw };
            dthet = (1.0 - soilw / self.sumfc) * l1.por * 0.95;
            rateinf[1] = 2000.0;
        }
        let psidt = dthet * sd.wfsh;
        let fcimp = self.urban.map(|u| u.fcimp);
        let tmp2 = self.layers.get(1).map(|l| l.tmp).unwrap_or(0.0);
        let mut surfq = 0.0_f32;
        for k in 1..=n {
            let mut adj_hc = (56.82 * l1.k.powf(0.286)) / (1.0 + 0.051 * (0.062 * *cnday).exp()) - 2.0;
            if adj_hc <= 0.0 {
                adj_hc = 0.001;
            }
            cumr[k] = cumr[k - 1] + ts[k - 1];
            rintns[k] = 60.0 * ts[k - 1] / dtm;
            if rateinf[k] >= rintns[k] {
                cuminf[k] = cuminf[k - 1] + ts[k - 1];
                excum[k] = if excum[k - 1] > 0.0 { excum[k - 1] } else { 0.0 };
            } else {
                let mut tst = adj_hc * dtm / 60.0;
                loop {
                    let f1 = cuminf[k - 1] + adj_hc * dtm / 60.0 + psidt * ((tst + psidt) / (cuminf[k - 1] + psidt)).ln();
                    if (f1 - tst).abs() <= 0.001 {
                        cuminf[k] = f1;
                        excum[k] = (cumr[k] - cuminf[k]).max(0.0);
                        exinc[k] = (excum[k] - excum[k - 1]).max(0.0);
                        hhqday[k - 1] = exinc[k];
                        break;
                    }
                    tst = f1;
                }
            }
            if let (Some(fcimp), Some(sd)) = (fcimp, self.sub.as_mut()) {
                if rintns[k] > 0.017 {
                    sd.urb_abstinit = (sd.urb_abstinit - ts[k - 1]).max(0.0);
                } else {
                    sd.urb_abstinit = b.urb_init_abst.min(sd.urb_abstinit + pet_day / n as f32);
                }
                hhqday[k - 1] *= 1.0 - fcimp;
                ubnrunoff[k - 1] = (sta_ts[k - 1] - sd.urb_abstinit) * fcimp;
                if ubnrunoff[k - 1] < 0.0 {
                    ubnrunoff[k - 1] = 0.0;
                }
            }
            hhsurfq[k - 1] = hhqday[k - 1] + ubnrunoff[k - 1];
            surfq += hhsurfq[k - 1];
            rateinf[k + 1] = adj_hc * (psidt / (cuminf[k] + 1.0e-6) + 1.0);
            let swdt = self.sw + cuminf[k];
            let mut sw_fac = self.wrt[0] - self.wrt[1] * swdt;
            if sw_fac < -20.0 {
                sw_fac = -20.0;
            }
            if sw_fac > 20.0 {
                sw_fac = 20.0;
            }
            let mut r2 = if (swdt + sw_fac.exp()) > 0.001 { self.smx * (1.0 - swdt / (swdt + sw_fac.exp())) } else { self.smx };
            if tmp2 <= 0.0 {
                r2 = self.smx * (1.0 - (-b.cn_froz * r2).exp());
            }
            r2 = r2.max(3.0);
            *cnday = 25400.0 / (r2 + 254.0);
        }
        let mut tot = 0.0_f32;
        for &v in ts.iter().take(n) {
            tot += v;
        }
        if tot > 12.0
            && let Some(sd) = self.sub.as_mut()
        {
            sd.swtrg = true;
            sd.rateinf_prev = rateinf[n + 1];
        }
        surfq
    }

    /// `ero_ovrsed` (with Green and Ampt): splash and overland flow erosion by time step (t),
    /// from the station's rain by step and the steps' runoff before the lag.
    pub(super) fn ero_ovrsed(&self, sta_ts: &[f32], hhqday: &[f32], b: &SubBasin) -> Vec<f32> {
        let Some(sd) = self.sub.as_ref() else { return Vec::new() };
        let l1 = &self.layers[0];
        let clay = l1.clay;
        let silt = l1.silt;
        let sand = 100.0 - clay - silt;
        let erod_k = if clay >= 40.0 && (20.0..=45.0).contains(&sand) {
            2.0
        } else if clay >= 27.0 && (20.0..=45.0).contains(&sand) {
            1.7
        } else if silt <= 40.0 && sand <= 20.0 {
            2.0
        } else if silt > 40.0 && clay >= 40.0 {
            1.6
        } else if clay >= 35.0 && sand >= 45.0 {
            1.9
        } else if clay >= 27.0 && sand < 20.0 {
            1.6
        } else if clay <= 10.0 && silt >= 80.0 {
            1.2
        } else if silt >= 50.0 {
            1.5
        } else if clay >= 7.0 && sand <= 52.0 && silt >= 28.0 {
            2.0
        } else if clay >= 20.0 {
            2.1
        } else if clay >= sand - 70.0 {
            2.6
        } else if clay >= (2.0 * sand) - 170.0 {
            3.0
        } else {
            1.9
        };
        let canopy_cover = if self.lai_sum >= 1.0 { 1.0 } else { self.lai_sum };
        let fimp = self.urban.map(|u| u.fimp);
        let n = sd.step;
        let dtm = sd.dtm;
        let mut out = vec![0.0_f32; n];
        for k in 0..n {
            let rintnsty = 60.0 * sta_ts[k] / dtm;
            let rain_d50 = 0.188 * rintnsty.powf(0.182);
            let (ke_direct, ke_leaf, rdepth_leaf, rdepth_direct);
            if rintnsty > 0.0 {
                let mut kd = 8.95 + 8.44 * rintnsty.log10();
                if kd < 0.0 {
                    kd = 0.0;
                }
                let pheff = 0.5 * self.cht_mx;
                let mut kl = 15.8 * pheff.powf(0.5) - 5.87;
                if kl < 0.0 {
                    kl = 0.0;
                }
                let rdepth_tot = sta_ts[k] / (dtm * 60.0);
                ke_direct = kd;
                ke_leaf = kl;
                rdepth_leaf = rdepth_tot * canopy_cover;
                rdepth_direct = rdepth_tot - rdepth_leaf;
            } else {
                (ke_direct, ke_leaf, rdepth_leaf, rdepth_direct) = (0.0, 0.0, 0.0, 0.0);
            }
            let ke_total = 0.001 * (rdepth_direct * ke_direct + rdepth_leaf * ke_leaf);
            let mut sedspl = erod_k * ke_total * (-b.eros_spl * hhqday[k] / 1000.0).exp() * self.km;
            if let Some(fimp) = fimp {
                sedspl *= 1.0 - fimp;
            }
            if hhqday[k] >= 3.0 * rain_d50 || hhqday[k] <= 1.0e-3 {
                sedspl = 0.0;
            }
            let cover = self.ab_gr_com.m + self.rsd_tot.m;
            let c = ((-0.2231 - sd.cvm_com) * (-0.00115 * cover).exp() + sd.cvm_com).exp();
            let bed_shear = 9807.0 * (hhqday[k] / 1000.0) * self.slope;
            let mut sedov = 11.02 * b.rill_mult * l1.usle_k * b.c_factor * c * bed_shear.powf(b.eros_expo);
            sedov = 16.667 * sedov * self.km * dtm;
            if let Some(fimp) = fimp {
                sedov *= 1.0 - fimp;
            }
            out[k] = sedspl + sedov;
            if out[k] < 1.0e-10 {
                out[k] = 0.0;
            }
        }
        out
    }

    /// `stor_surfstor`'s sediment by time step: lagged step by step; returns the day's total.
    pub(super) fn lag_sediment(&mut self, hhsedy: &mut [f32]) -> f32 {
        let brt = self.brt;
        let Some(sd) = self.sub.as_mut() else { return 0.0 };
        let n = sd.step;
        let mut prev = sd.bs_sed[n - 1];
        for k in 0..n {
            sd.bs_sed[k] = (prev + hhsedy[k]).max(1.0e-9);
            if sd.bs_sed[k] == 1.0e-9 {
                sd.bs_sed[k] = 0.0;
            }
            hhsedy[k] = sd.bs_sed[k] * brt;
            sd.bs_sed[k] -= hhsedy[k];
            prev = sd.bs_sed[k];
        }
        let mut tot = 0.0_f32;
        for &v in hhsedy.iter().take(n) {
            tot += v;
        }
        tot
    }

    /// `sq_surfst` with Green and Ampt: the runoff lagged step by step (`hhsurfq` is replaced
    /// by what leaves); returns the day's runoff (`qday`).
    pub(super) fn lag_runoff(&mut self, hhsurfq: &mut [f32]) -> f32 {
        let brt = self.brt;
        let Some(sd) = self.sub.as_mut() else { return 0.0 };
        let n = sd.step;
        let mut prev = sd.bs_q[n - 1];
        let mut qday = 0.0_f32;
        for k in 0..n {
            sd.bs_q[k] = (prev + hhsurfq[k]).max(1.0e-9);
            if sd.bs_q[k] == 1.0e-9 {
                sd.bs_q[k] = 0.0;
            }
            hhsurfq[k] = sd.bs_q[k] * brt;
            sd.bs_q[k] -= hhsurfq[k];
            prev = sd.bs_q[k];
            qday += hhsurfq[k];
        }
        qday
    }
}
