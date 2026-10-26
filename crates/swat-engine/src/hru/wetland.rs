//! Wetlands in HRUs (`surf_stor` of `hru-data.hru`): water ponded on the HRU, released by a
//! decision table of `res_rel.dtl` (`wetland_control`, `res_hydro`, `res_sediment`,
//! `res_nutrient`), seeping into the soil and evaporating (`et_act`, `swr_satexcess`).

use super::{Hru, NutDay};
use crate::fortran::rpow;
use crate::hyd::Hyd;
use swat_model::Project;
use swat_model::table::Table;

/// A condition of a release table: the ponded water against a share of a volume (`vol_wet`).
#[derive(Clone, Debug, PartialEq)]
struct Cond {
    /// The emergency volume (else the principal one).
    evol: bool,
    op: char,
    lim_const: f32,
    alt: Vec<String>,
}

/// How an action releases water (`res_hydro`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Release {
    /// `rate`: m3/s.
    Rate(f32),
    /// `rate_pct`: percent of the principal volume.
    RatePct(f32),
    /// `ab_emer`: what is above the emergency volume.
    AbEmer,
    /// `days`: what is above a base volume, over so many days.
    Days { days: f32, base: Base, frac: f32 },
    /// `dyrt`: as `days`, plus a percent of the principal volume.
    Dyrt { days: f32, base: Base, pct: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Base {
    Zero,
    Pvol,
    Evol,
}

#[derive(Clone, Debug, PartialEq)]
struct Act {
    release: Release,
    outcomes: Vec<String>,
}

/// A wetland's data and state.
#[derive(Clone, Debug, PartialEq)]
pub struct Wetland {
    /// `hydrology.wet`: the vol-area and vol-depth coefficients, the bottom's conductivity
    /// (mm/h).
    acoef: f32,
    bcoef: f32,
    ccoef: f32,
    wet_hc: f32,
    /// The evaporation coefficient, and the one `et_act` uses: `wet_hyd(ihyd)`, of the HRU
    /// numbered as the wetland's `hydrology.wet` row (0.7 if that HRU has no wetland).
    pub evrsv: f32,
    pub evrsv_et: f32,
    /// Principal and emergency volumes (m3), weir height (m).
    pvol: f32,
    evol: f32,
    weir_hgt: f32,
    alts: usize,
    conds: Vec<Cond>,
    acts: Vec<Act>,
    /// `sediment.res`: normal sediment (mg/L) and its settling factor.
    nsed: f32,
    sed_stlr_co: f32,
    /// `nutrients.res`.
    ires1: i32,
    ires2: i32,
    nsetlr1: f32,
    nsetlr2: f32,
    psetlr1: f32,
    psetlr2: f32,
    nsolr: f32,
    psolr: f32,
    theta_n: f32,
    theta_p: f32,
    conc_nmin: f32,
    conc_pmin: f32,
    /// The ponded water and what it holds (`wet(j)`, m3, t, kg).
    pub water: Hyd,
    /// Today's outflow (`ht2`), what seeped (`wet_seep_day`: no3, nh3, orgn, solp, sedp) and
    /// how deep (mm, `hru%water_seep`).
    pub out: Hyd,
    seep_day: [f32; 5],
    pub water_seep: f32,
}

/// The table and row whose `field` is `name`.
fn find<'a>(t: Option<&'a Table>, field: &str, name: &str) -> Option<(&'a Table, usize)> {
    let t = t?;
    (0..t.rows.len()).find(|&r| t.text(r, field).is_some_and(|n| swat_model::table::fortran_eq(n, name))).map(|r| (t, r))
}

impl Wetland {
    /// `wet_initial` for an HRU of `area_ha` whose `surf_stor` is `name`. `hyd_row` comes back
    /// too: the 1-based `hydrology.wet` row (`wet_dat%hyd`).
    pub fn new(project: &Project, name: &str, area_ha: f32) -> Result<(Wetland, usize), String> {
        let (wt, r) = find(project.wetland.as_ref(), "name", name).ok_or_else(|| format!("surface storage {name} not in wetland.wet"))?;
        let s = |f: &str| wt.text(r, f).unwrap_or("").to_string();
        if project.weir_res {
            return Err("wetlands with weir.res".into());
        }
        let hyd_name = s("hyd");
        if hyd_name == "paddy" {
            return Err("paddy wetlands".into());
        }
        let (hy, ih) = find(project.wet_hyd.as_ref(), "name", &hyd_name)
            .ok_or_else(|| format!("wetland {name}: hydrology {hyd_name} not in hydrology.wet"))?;
        let x = |f: &str| hy.real(ih, f).unwrap_or(0.0);
        let (psa, pdep, esa, edep, k, evrsv) = (x("psa"), x("pdep"), x("esa"), x("edep"), x("k"), x("evrsv"));
        if psa <= 0.0 || esa <= 0.0 || evrsv <= 0.0 || k <= 0.0 {
            // wet_read_hyd fills these from arrays not set yet.
            return Err(format!("wetland hydrology {hyd_name} without psa, esa, evap or k"));
        }

        // The initial water: om_water.ini's row for initial.res's org_min.
        let init = s("init");
        if let Some((ti, ir)) = find(project.res_init.as_ref(), "name", &init) {
            let om = ti.text(ir, "org_min").unwrap_or("").to_string();
            if let Some(t) = project.om_water.as_ref()
                && let Some(or) = t.find(&om)
                && t.rows[or].iter().skip(1).any(|v| v.as_real().is_some_and(|x| x != 0.0))
            {
                return Err(format!("wetland {name}: initial water ({om})"));
            }
        }

        let rel = s("rel");
        let dt = project
            .res_dtl
            .iter()
            .find(|d| swat_model::table::fortran_eq(&d.name, &rel))
            .ok_or_else(|| format!("wetland {name}: release table {rel} not in res_rel.dtl"))?;
        let mut conds = Vec::new();
        for (c, alt) in dt.conds.iter().zip(&dt.alt) {
            if c.var != "vol_wet" || c.ob_num != 0 {
                return Err(format!("wetland release condition {} on object {}", c.var, c.ob_num));
            }
            let evol = match c.lim_var.as_str() {
                "pvol" => false,
                "evol" => true,
                v => return Err(format!("wetland release condition on {v}")),
            };
            let op = match c.lim_op.as_str() {
                "*" => '*',
                "+" => '+',
                "-" => '-',
                "/" => '/',
                o => return Err(format!("wetland release condition operator {o}")),
            };
            conds.push(Cond { evol, op, lim_const: c.lim_const, alt: alt.clone() });
        }
        let mut acts = Vec::new();
        for (a, out) in dt.acts.iter().zip(&dt.outcomes) {
            let base = || match a.file_pointer.as_str() {
                "null" => Ok(Base::Zero),
                "pvol" => Ok(Base::Pvol),
                "evol" => Ok(Base::Evol),
                fp => Err(format!("wetland release base {fp}")),
            };
            let release = match a.option.as_str() {
                "rate" => Release::Rate(a.const_),
                "rate_pct" => Release::RatePct(a.const_),
                "ab_emer" => Release::AbEmer,
                "days" => Release::Days { days: a.const_, base: base()?, frac: a.const2 },
                "dyrt" => Release::Dyrt { days: a.const_, base: base()?, pct: a.const2 },
                o => return Err(format!("wetland release option {o}")),
            };
            acts.push(Act { release, outcomes: out.clone() });
        }

        let sed = s("sed");
        let (nsed, d50) = match find(project.res_sed.as_ref(), "name", &sed) {
            Some((t, r)) => (t.real(r, "nsed").unwrap_or(0.0), t.real(r, "d50").unwrap_or(0.0)),
            None => (0.0, 0.0),
        };
        let nut = s("nut");
        let nt = find(project.res_nut.as_ref(), "name", &nut);
        let nr = |f: &str, d: f32| nt.map(|(t, r)| t.real(r, f).unwrap_or(d)).unwrap_or(d);
        let ni = |f: &str| nt.map(|(t, r)| t.int(r, f).unwrap_or(0)).unwrap_or(0);

        let w = Wetland {
            acoef: x("acoef"),
            bcoef: x("bcoef"),
            ccoef: x("ccoef"),
            wet_hc: k,
            evrsv,
            evrsv_et: evrsv,
            evol: area_ha * edep * 10.0,
            pvol: area_ha * pdep * 10.0,
            weir_hgt: pdep / 1000.0,
            alts: dt.alts,
            conds,
            acts,
            nsed,
            sed_stlr_co: (-0.184 * d50).exp(),
            ires1: ni("ires1"),
            ires2: ni("ires2"),
            nsetlr1: nr("nsetlr1", 0.0),
            nsetlr2: nr("nsetlr2", 0.0),
            psetlr1: nr("psetlr1", 0.0),
            psetlr2: nr("psetlr2", 0.0),
            nsolr: nr("nsolr", 1.0),
            psolr: nr("psolr", 1.0),
            theta_n: nr("theta_n", 1.0),
            theta_p: nr("theta_p", 1.0),
            conc_nmin: nr("conc_nmin", 0.1),
            conc_pmin: nr("conc_pmin", 0.01),
            water: Hyd::default(),
            out: Hyd::default(),
            seep_day: [0.0; 5],
            water_seep: 0.0,
        };
        Ok((w, ih + 1))
    }

    /// `conditions` and `res_hydro` (a day, one step): the release.
    fn release(&self) -> f32 {
        let mut hit = vec![true; self.alts];
        let cur = self.water.flo;
        for c in &self.conds {
            let v = if c.evol { self.evol } else { self.pvol };
            let tbl = match c.op {
                '*' => v * c.lim_const,
                '+' => v + c.lim_const,
                '-' => v - c.lim_const,
                _ => v / c.lim_const,
            };
            super::mgt::cond_test(&mut hit, &c.alt, cur, tbl);
        }
        let mut flo = 0.0_f32;
        for a in &self.acts {
            let action = self.alts == 0 || (0..self.alts).any(|k| hit[k] && a.outcomes.get(k).is_some_and(|o| o == "y"));
            if !action {
                continue;
            }
            let base = |b: Base| match b {
                Base::Zero => 0.0,
                Base::Pvol => self.pvol,
                Base::Evol => self.evol,
            };
            match a.release {
                Release::Rate(r) => flo += r * 86400.0,
                Release::RatePct(p) => flo += p * self.pvol / 100.0,
                Release::AbEmer => {
                    if cur > self.evol {
                        flo += cur - self.evol;
                    }
                }
                Release::Days { days, base: b, frac } => {
                    let b_lo = if b == Base::Zero { 0.0 } else { base(b) * frac };
                    flo += (cur - b_lo) / days;
                    flo = flo.max(0.0);
                }
                Release::Dyrt { days, base: b, pct } => {
                    let b_lo = base(b).max(0.0);
                    flo += (cur - b_lo) / days + pct * self.pvol / 100.0;
                    flo = flo.max(0.0);
                }
            }
        }
        flo
    }
}

fn theta(r20: f32, thk: f32, tmp: f32) -> f32 {
    r20 * rpow(thk, tmp - 20.0)
}

impl Hru {
    /// `wetland_control`, daily: rain and irrigation into the pond, seepage into the soil, the
    /// release, sediment and nutrients settling. Returns the runoff (mm, the release) and sets
    /// the day's sediment and nutrient loads in `nd`. `mo` is the month, `station_tave` the
    /// weather station's mean temperature.
    pub(super) fn wetland_control(
        &mut self,
        precip: f32,
        mo: i32,
        station_tave: f32,
        nut: &super::nutrient::NutBasin,
        nd: &mut NutDay,
    ) -> f32 {
        let prf = nut.prf;
        let area = self.area_ha;
        let wsa1 = area * 10.0;
        let c_factor = self.c_factor(nut.rsd_covco);
        let usle_mult = self.usle_mult;
        let tconc = self.tconc;
        let irr = self.irr_applied;
        let (mut layers_st, layers_ul, layers_k): (Vec<f32>, Vec<f32>, Vec<f32>) = (
            self.layers.iter().map(|l| l.st).collect(),
            self.layers.iter().map(|l| l.ul).collect(),
            self.layers.iter().map(|l| l.k).collect(),
        );
        let mut top = (self.layers[0].no3, self.layers[0].nh4, self.layers[0].act);
        let Some(wt) = self.wet.as_mut() else { return 0.0 };

        wt.out = Hyd::default();
        wt.water_seep = 0.0;
        let dep_init = wt.water.flo / wsa1 / 1000.0;
        wt.water.flo += precip * wsa1;
        wt.water.flo += irr * wsa1;
        if wt.water.flo > 0.0 {
            let x1 = wt.bcoef * wt.bcoef + 4.0 * wt.ccoef * (1.0 - wt.water.flo / (wt.pvol + 1.0e-9));
            let wet_h = if x1 < 1.0e-6 {
                0.0
            } else {
                let wet_h1 = (-wt.bcoef - x1.sqrt()) / (2.0 * wt.ccoef + 1.0e-9);
                wet_h1 + wt.bcoef
            };
            let mut wet_fr = 1.0 + wt.acoef * wet_h;
            wet_fr = wet_fr.min(1.0);
            wet_fr = wet_fr.max(0.01);
            let wet_area = area * wet_fr;
            let mut seep = wt.water.flo.min(wt.wet_hc * 24.0 * wet_area * 10.0);
            let mut volseep = seep / wsa1;
            if volseep > 0.1 {
                let nly = layers_st.len();
                for j1 in 0..nly {
                    layers_st[j1] += volseep;
                    if layers_st[j1] > layers_ul[j1] * 0.999 {
                        let volex = layers_st[j1] - layers_ul[j1] * 0.999;
                        volseep = volex.min(layers_k[j1] * 24.0);
                        layers_st[j1] -= volseep;
                    } else {
                        volseep = 0.0;
                    }
                }
                let mut volex = 0.0_f32;
                for j1 in (0..nly).rev() {
                    layers_st[j1] += volex;
                    if layers_st[j1] > layers_ul[j1] {
                        volex = (layers_st[j1] - layers_ul[j1]).max(0.0);
                        layers_st[j1] -= volex;
                    }
                }
                seep = (seep - volex * wsa1).max(0.0);
            }
            wt.water.flo -= seep;
            wt.water_seep = seep / wsa1;
            let seep_rto = if seep + wt.water.flo > 0.01 { seep / (seep + wt.water.flo) } else { 0.0 };
            top.0 += wt.water.no3 * seep_rto / area;
            top.1 += wt.water.nh3 * seep_rto / area;
            top.2 += wt.water.solp * seep_rto / area;
            wt.seep_day = [
                wt.water.no3 * seep_rto,
                wt.water.nh3 * seep_rto,
                wt.water.orgn * seep_rto,
                wt.water.solp * seep_rto,
                wt.water.sedp * seep_rto,
            ];
            wt.water.no3 -= wt.seep_day[0];
            wt.water.nh3 -= wt.seep_day[1];
            wt.water.orgn -= wt.seep_day[2];
            wt.water.solp -= wt.seep_day[3];
            wt.water.sedp -= wt.seep_day[4];
        }

        // The release, by the decision table.
        wt.out.flo = wt.release();
        wt.water.flo = (wt.water.flo - wt.out.flo).max(0.0);
        let surfq = wt.out.flo / wsa1;

        // res_sediment
        let b = &mut wt.water;
        if b.flo < 1.0e-6 {
            *b = Hyd::default();
            wt.out.sed = 0.0;
        } else {
            let mut sed_ppm = (1_000_000.0 * b.sed / b.flo).max(1.0e-6);
            if sed_ppm > wt.nsed {
                sed_ppm = (sed_ppm - wt.nsed) * wt.sed_stlr_co + wt.nsed;
                sed_ppm = sed_ppm.max(wt.nsed);
                b.sed = sed_ppm * b.flo / 1_000_000.0;
                wt.out.sed = sed_ppm * wt.out.flo / 1_000_000.0;
                b.sed = (b.sed - wt.out.sed).max(0.0);
                b.sil = 0.0;
                b.cla = 0.0;
                b.san = 0.0;
                b.sag = 0.0;
                b.lag = 0.0;
                b.grv = 0.0;
            }
        }

        // res_nutrient
        if wt.water.flo < 1.0e-6 {
            wt.water = Hyd::default();
        } else {
            let (nsetlr, psetlr) = if mo >= wt.ires1 && mo <= wt.ires2 { (wt.nsetlr1, wt.psetlr1) } else { (wt.nsetlr2, wt.psetlr2) };
            let b = &mut wt.water;
            let conc_n = 1000.0 * b.orgn / b.flo;
            let conc_p = 1000.0 * b.sedp / b.flo;
            let conc_soln = 1000.0 * (b.no3 + b.nh3 + b.no2) / b.flo;
            let conc_solp = 1000.0 * b.solp / b.flo;
            let k = |c: f32, cmin: f32, r: f32, th: f32| ((c - cmin) * theta(r, th, station_tave)).min(1.0).max(0.0);
            let nitrok = k(conc_n, wt.conc_nmin, nsetlr, wt.theta_n);
            let phosk = k(conc_p, wt.conc_pmin, psetlr, wt.theta_p);
            let nitrosolk = k(conc_soln, wt.conc_nmin, wt.nsolr, wt.theta_n);
            let phossolk = k(conc_solp, wt.conc_pmin, wt.psolr, wt.theta_p);
            let (soln_stl_fr, solp_stl_fr) = (0.2_f32, 0.2_f32);
            b.solp *= 1.0 - phossolk * solp_stl_fr;
            b.sedp *= 1.0 - phosk;
            b.orgn *= 1.0 - nitrok;
            b.no3 *= 1.0 - nitrosolk * soln_stl_fr;
            b.nh3 *= 1.0 - nitrosolk * soln_stl_fr;
            b.no2 *= 1.0 - nitrosolk * soln_stl_fr;
            b.chla = 0.0;
            let o = &mut wt.out;
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

        // A pond that was empty and spills is flushed: its sediment comes from erosion.
        let mut sedyld = 0.0_f32;
        if dep_init < 0.0001 && wt.out.flo > 0.0 {
            let qp_cms = prf / 6578.6 * area * surfq / tconc / 35.3;
            let cklsp = c_factor * usle_mult;
            wt.water.sed = (10.0 * surfq * qp_cms * area).powf(0.56) * cklsp;
            sedyld = wt.water.sed * wt.out.flo / (wt.water.flo + wt.out.flo);
            wt.water.sed = (wt.water.sed - sedyld).max(0.0);
            wt.out.sed = sedyld;
        }
        let o = wt.out;
        nd.sedyld = o.sed + sedyld;
        nd.sedorgn = o.orgn / area;
        nd.sedorgp = o.sedp / area;
        nd.surqno3 = o.no3 / area;
        nd.sedminps = o.solp / area / 2.0;
        nd.sedminpa = o.solp / area / 2.0;

        self.layers[0].no3 = top.0;
        self.layers[0].nh4 = top.1;
        self.layers[0].act = top.2;
        surfq
    }

    /// `swr_satexcess` for a wetland: what the saturated top layer can't hold goes back to the
    /// pond with the nutrients that seeped with it (or runs off, below a 1 mm weir).
    pub(super) fn wet_excess(&mut self, ul_excess: f32, surfq: &mut f32) {
        let area = self.area_ha;
        let Some(wt) = self.wet.as_mut() else { return };
        if wt.weir_hgt < 0.001 {
            wt.out.flo += ul_excess * 10.0 * area;
            *surfq += ul_excess;
        } else {
            wt.water.flo += ul_excess * 10.0 * area;
        }
        wt.water_seep = (wt.water_seep - ul_excess).max(0.0);
        if wt.water_seep > 1.0e-6 {
            let rto = (ul_excess / wt.water_seep).min(1.0);
            let l = &mut self.layers[0];
            l.no3 -= wt.seep_day[0] * rto / area;
            l.nh4 -= wt.seep_day[1] * rto / area;
            l.act -= wt.seep_day[3] * rto / area;
            wt.water.no3 += wt.seep_day[0] * rto;
            wt.water.nh3 += wt.seep_day[1] * rto;
            wt.water.orgn += wt.seep_day[2] * rto;
            wt.water.solp += wt.seep_day[3] * rto;
            wt.water.sedp += wt.seep_day[4] * rto;
        }
    }
}
