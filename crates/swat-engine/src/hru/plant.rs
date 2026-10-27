//! Plants: the plant database as `plantparm_init` completes it, a plant's setup (`plant_init`) and
//! its daily growth (`pl_community`, `pl_waterup`, `pl_grow` and the routines it calls).
//!
//! Communities of up to 20 plants: perennials, plants with heat units over the whole year
//! (`days_mat` 0) and annuals with days to maturity.

use std::ops::{Add, Sub};

use swat_model::table::Table;

use super::{Hru, ascrv};

/// Organic matter: mass, carbon, nitrogen and phosphorus (`organic_mass`, kg/ha), with the
/// operators of `organic_mineral_mass_module` (component by component).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OrgMass {
    pub m: f32,
    pub c: f32,
    pub n: f32,
    pub p: f32,
}

impl std::ops::Add for OrgMass {
    type Output = OrgMass;
    fn add(self, o: OrgMass) -> OrgMass {
        OrgMass { m: self.m + o.m, c: self.c + o.c, n: self.n + o.n, p: self.p + o.p }
    }
}

impl std::ops::Sub for OrgMass {
    type Output = OrgMass;
    fn sub(self, o: OrgMass) -> OrgMass {
        OrgMass { m: self.m - o.m, c: self.c - o.c, n: self.n - o.n, p: self.p - o.p }
    }
}

impl OrgMass {
    /// `const * o`.
    pub fn scale(k: f32, o: OrgMass) -> OrgMass {
        OrgMass { m: k * o.m, c: k * o.c, n: k * o.n, p: k * o.p }
    }
}

/// A plant of `plants.plt` (`plant_db`), as `plant_parm_read` and `plantparm_init` leave it, with
/// the shape parameters `plantparm_init` derives (`plant_cp`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlantDb {
    pub name: String,
    pub typ: String,
    pub trig: String,
    pub nfix_co: f32,
    pub days_mat: i32,
    pub bio_e: f32,
    pub hvsti: f32,
    pub blai: f32,
    pub dlai: f32,
    pub dlai_rate: f32,
    pub chtmx: f32,
    pub rdmx: f32,
    pub t_opt: f32,
    pub t_base: f32,
    pub cnyld: f32,
    pub cpyld: f32,
    pub wsyf: f32,
    pub pltnfr1: f32,
    pub pltnfr3: f32,
    pub pltpfr1: f32,
    pub pltpfr3: f32,
    pub usle_c: f32,
    pub wavp: f32,
    pub rsdco_pl: f32,
    pub alai_min: f32,
    pub laixco_tree: f32,
    pub mat_yrs: i32,
    pub bmx_peren: f32,
    pub ext_coef: f32,
    pub leaf_tov_min: f32,
    pub leaf_tov_max: f32,
    pub bm_dieoff: f32,
    pub rsr1: f32,
    pub rsr2: f32,
    pub aeration: f32,
    /// Maximum stomatal conductance (m/s) and the slope of its decline with the vapour pressure
    /// deficit (`plcp%vpd2`), for Penman-Monteith.
    pub gsi: f32,
    pub vpd2: f32,
    // plant_cp
    pub leaf1: f32,
    pub leaf2: f32,
    pub ruc1: f32,
    pub ruc2: f32,
    pub nup1: f32,
    pub nup2: f32,
    pub pup1: f32,
    pub pup2: f32,
    pub cvm: f32,
    /// Shares of residue going to metabolic, structural and lignin litter, above and below
    /// ground (`cswat_1_part_fracs`, carbon model only).
    pub fracs: PartFracs,
}

/// `lignin_derived_partition_fracs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartFracs {
    pub meta_abg: f32,
    pub str_abg: f32,
    pub lig_abg: f32,
    pub meta_blg: f32,
    pub str_blg: f32,
    pub lig_blg: f32,
}

impl Default for PartFracs {
    fn default() -> Self {
        Self { meta_abg: 0.85, str_abg: 0.15, lig_abg: 0.12, meta_blg: 0.85, str_blg: 0.15, lig_blg: 0.12 }
    }
}

impl PlantDb {
    /// Row `r` of `plants.plt`, with `plant_parm_read`'s and `plantparm_init`'s adjustments.
    /// `rsdco` is the basin's residue decomposition coefficient.
    pub fn new(t: &Table, r: usize, rsdco: f32) -> PlantDb {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        let i = |f: &str| t.int(r, f).unwrap_or(0);
        let s = |f: &str| t.text(r, f).unwrap_or("").to_string();
        let mut d = PlantDb {
            name: s("plantnm"),
            typ: s("typ"),
            trig: s("trig"),
            nfix_co: x("nfix_co"),
            days_mat: i("days_mat"),
            bio_e: x("bio_e"),
            hvsti: x("hvsti"),
            blai: x("blai"),
            dlai: x("dlai"),
            dlai_rate: x("dlai_rate"),
            chtmx: x("chtmx"),
            rdmx: x("rdmx"),
            t_opt: x("t_opt"),
            t_base: x("t_base"),
            cnyld: x("cnyld"),
            cpyld: x("cpyld"),
            wsyf: x("wsyf"),
            pltnfr1: x("pltnfr1"),
            pltnfr3: x("pltnfr3"),
            pltpfr1: x("pltpfr1"),
            pltpfr3: x("pltpfr3"),
            usle_c: x("usle_c"),
            wavp: x("wavp"),
            rsdco_pl: x("rsdco_pl"),
            alai_min: x("alai_min"),
            laixco_tree: x("laixco_tree"),
            mat_yrs: i("mat_yrs").max(1),
            bmx_peren: x("bmx_peren"),
            ext_coef: x("ext_coef"),
            leaf_tov_min: x("leaf_tov_min"),
            leaf_tov_max: x("leaf_tov_max"),
            bm_dieoff: x("bm_dieoff"),
            rsr1: x("rsr1"),
            rsr2: x("rsr2"),
            aeration: x("aeration"),
            gsi: x("gsi"),
            ..PlantDb::default()
        };
        let mut pltnfr2 = x("pltnfr2");
        let mut pltpfr2 = x("pltpfr2");
        let mut co2hi = x("co2hi");
        let bioehi = x("bioehi");
        let (laimx1, laimx2, frgrw1, frgrw2) = (x("laimx1"), x("laimx2"), x("frgrw1"), x("frgrw2"));
        // plant_parm_read with the carbon model: the file's above- and below-ground lignin
        // fractions (read into `str_frac` and `lig_frac`).
        let lig_blg = x("lig_frac");
        let lig_abg = x("str_frac");
        let str_blg = lig_blg / 0.80;
        let str_abg = lig_abg / 0.80;
        d.fracs = PartFracs { meta_abg: 1.0 - str_abg, str_abg, lig_abg, meta_blg: 1.0 - str_blg, str_blg, lig_blg };

        // plantparm_init
        if d.bm_dieoff <= 1.0e-6 {
            d.bm_dieoff = 1.00;
        }
        if d.ext_coef < 1.0e-6 {
            d.ext_coef = 0.65;
        }
        if d.rsdco_pl < 1.0e-6 {
            d.rsdco_pl = rsdco;
        }
        if d.usle_c <= 0.0 {
            d.usle_c = 0.001;
        }
        if d.usle_c >= 1.0 {
            d.usle_c = 1.0;
        }
        if d.blai <= 0.0 {
            d.blai = 0.0;
        }
        if d.blai >= 10.0 {
            d.blai = 10.0;
        }
        if d.rsr1 <= 0.0 {
            d.rsr1 = 0.4;
        }
        if d.rsr2 <= 0.0 {
            d.rsr2 = 0.2;
        }
        if d.aeration <= 0.0 {
            d.aeration = 0.2;
        }
        if d.typ == "warm_annual_tuber" || d.typ == "cold_annual_tuber" {
            d.rsr2 = 0.7;
        }
        if d.hvsti > 0.7 {
            d.hvsti = 0.02;
        }
        if d.bio_e > 0.0 && d.name != "WATR" {
            (d.leaf1, d.leaf2) = ascrv(laimx1, laimx2, frgrw1, frgrw2);
            let c1 = 330.0_f32;
            if co2hi == 330.0 {
                co2hi = 660.0;
            }
            let b1 = d.bio_e * 0.01;
            let b2 = bioehi * 0.01;
            (d.ruc1, d.ruc2) = ascrv(b1, b2, c1, co2hi);
            d.cvm = d.usle_c.ln();
            if d.pltnfr1 - pltnfr2 < 0.0001 {
                pltnfr2 = d.pltnfr1 - 0.0001;
            }
            if pltnfr2 - d.pltnfr3 < 0.0001 {
                d.pltnfr3 *= 0.75;
            }
            let b1 = d.pltnfr1 - d.pltnfr3;
            let b2 = 1.0 - (pltnfr2 - d.pltnfr3) / b1;
            let b3 = 1.0 - 0.00001 / b1;
            (d.nup1, d.nup2) = ascrv(b2, b3, 0.5, 1.0);
            if d.pltpfr1 - pltpfr2 < 0.0001 {
                pltpfr2 = d.pltpfr1 - 0.0001;
            }
            if pltpfr2 - d.pltpfr3 < 0.0001 {
                d.pltpfr3 *= 0.75;
            }
            let b1 = d.pltpfr1 - d.pltpfr3;
            let b2 = 1.0 - (pltpfr2 - d.pltpfr3) / b1;
            let b3 = 1.0 - 0.00001 / b1;
            (d.pup1, d.pup2) = ascrv(b2, b3, 0.5, 1.0);
            d.vpd2 = (1.0 - x("gmaxfr")) / (x("vpdfr") - 1.0);
        }
        d
    }

    pub fn perennial(&self) -> bool {
        self.typ == "perennial"
    }

    pub fn annual(&self) -> bool {
        matches!(self.typ.as_str(), "warm_annual" | "cold_annual" | "warm_annual_tuber" | "cold_annual_tuber")
    }
}

/// A plant growing in an HRU (`pcom(j)%plcur`, `plg`, `plm`, `plstr` and `pl_mass(j)` for it).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plant {
    pub db: PlantDb,
    /// The name `plant.ini` gives it, which operations refer to.
    pub cpnm: String,
    /// What its harvests yielded since the simulation started (`yield_tot`), and how many
    /// harvests counted (`harv_num`; the decision tables' harvest only doesn't count).
    pub yield_tot: OrgMass,
    pub harv_num: i32,
    /// Growing (`gro`) and dormant (`idorm`).
    pub gro: bool,
    pub idorm: bool,
    pub phuacc: f32,
    pub phuacc_p: f32,
    pub phumat: f32,
    pub phumat_p: f32,
    pub curyr_mat: i32,
    pub lai_pot: f32,
    pub harv_idx: f32,
    pub epco: f32,
    pub leaf_tov: f32,
    // plg
    pub lai: f32,
    pub olai: f32,
    pub dphu: f32,
    pub d_senes: f32,
    pub cht: f32,
    pub laimxfr: f32,
    pub root_dep: f32,
    pub root_frac: f32,
    pub rtfr: Vec<f32>,
    pub hi_adj: f32,
    pub hi_prev: f32,
    pub plet: f32,
    pub plpet: f32,
    // plm
    pub n_fr: f32,
    pub p_fr: f32,
    // plstr
    pub strsw: f32,
    pub strst: f32,
    pub strsn: f32,
    pub strsp: f32,
    pub strsa: f32,
    pub strss: f32,
    pub reg: f32,
    /// Stress summed over the season (`sum_w`, `sum_tmp`, `sum_n`, `sum_p`, `sum_a`), for
    /// `mgt_out.txt`.
    pub sum_w: f32,
    pub sum_tmp: f32,
    pub sum_n: f32,
    pub sum_p: f32,
    pub sum_a: f32,
    // pl_mass
    pub tot: OrgMass,
    pub ab_gr: OrgMass,
    pub leaf: OrgMass,
    pub stem: OrgMass,
    pub seed: OrgMass,
    pub root: OrgMass,
    /// Surface residue of this plant (`pl_mass(j)%rsd(ipl)`), and its residue in each soil
    /// layer (`soil1(j)%pl(ipl)%rsd`).
    pub rsd: OrgMass,
    pub soil_rsd: Vec<OrgMass>,
    /// Light intercepted today (`par(ipl)`).
    pub par: f32,
}

/// Basin values the plant routines read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlantBasin {
    pub co2: f32,
    pub n_updis: f32,
    pub p_updis: f32,
    pub nfixmx: f32,
    pub nostress: i32,
    /// `uptake%water_norm`, `n_norm`, `p_norm` (`basin_prm_default`).
    pub water_norm: f32,
    pub n_norm: f32,
    pub p_norm: f32,
    /// Correct SWAT+'s bugs instead of reproducing them (`RunOptions::fixes`).
    pub fixes: bool,
}

impl PlantBasin {
    pub fn new(project: &swat_model::Project) -> Self {
        let p = |n: &str| project.params.get(n).unwrap_or(0.0);
        let n_updis = p("n_updis");
        let p_updis = p("p_updis");
        Self {
            co2: p("co2"),
            n_updis,
            p_updis,
            nfixmx: p("nfixmx"),
            nostress: project.codes.nostress,
            water_norm: 1.0 - (-10.0_f32).exp(),
            n_norm: 1.0 - (-n_updis).exp(),
            p_norm: 1.0 - exp_w(-p_updis),
            fixes: false,
        }
    }
}

/// SWAT+ `exp_w`: `exp` with the argument kept within ±80.
pub fn exp_w(y: f32) -> f32 {
    if y < -80.0 {
        0.0
    } else if y > 80.0 {
        80.0_f32.exp()
    } else {
        y.exp()
    }
}

/// SWAT+ `nuts`: the fraction of optimal growth for an element held `u1` of an optimal `u2`.
fn nuts(u1: f32, u2: f32) -> f32 {
    let mut uu = 200.0 * (u1 / (u2 + 0.0001) - 0.5);
    if uu <= 0.0 {
        uu = 0.0;
    } else if uu < 99.0 {
        uu /= uu + (3.535_f32 - 0.02597 * uu).exp();
    } else {
        uu = 1.0;
    }
    if u2 <= 1.0e-6 {
        uu = 1.0;
    }
    uu
}

/// The day's values the plant routines share (`hru_module` globals for one HRU).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlantDay {
    pub ep_max: f32,
    pub ep_day: f32,
    pub pet_day: f32,
    pub vpd: f32,
    pub nplnt: f32,
    pub pplnt: f32,
    pub fixn: f32,
    pub uno3d: f32,
    pub uapd: f32,
    pub uno3d_tot: f32,
    pub uapd_tot: f32,
    pub sum_no3: f32,
    pub sum_solp: f32,
    pub rto_no3: f32,
    pub rto_solp: f32,
    pub un2: f32,
    pub up2: f32,
    pub bm_grow: f32,
    pub c_gro: f32,
    pub nuptake: f32,
    pub puptake: f32,
}

/// Weather and calendar for the plant routines: the HRU's weather, its station's unadjusted mean
/// temperature and day length, the generator's minimum day length, dormancy threshold and annual
/// temperature, and whether today ends the year.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlantWeather {
    pub tave: f32,
    pub tmin: f32,
    pub solrad: f32,
    pub station_tave: f32,
    pub daylength: f32,
    pub daylmn: f32,
    pub dormhr: f32,
    pub tmp_an: f32,
    pub end_yr: bool,
}

/// Variables the Fortran keeps from one call to the next (implicit SAVE), shared by all HRUs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    /// The random number seed of decision table conditions (`rndseed_cond`).
    pub rndseed_cond: i32,
    /// `pl_nut_demand`'s `delg_p`: kept when the plant's heat units are under 0.1.
    pub delg_p: f32,
    /// `uno3d`, `uapd`, `un2`, `up2`: arrays of the HRU module shared by every HRU.
    /// `pl_nut_demand` meant to zero them but zeroes the element of the loop counter left over
    /// (`ipl`, one past the last plant), so a plant that is dormant when its demand is computed
    /// and wakes the same day grows with the demand last computed for any HRU.
    /// Indexed by plant number (1-based; 20 in SWAT+, `mpc`).
    pub uno3d: [f32; 21],
    pub uapd: [f32; 21],
    pub un2: [f32; 21],
    pub up2: [f32; 21],
    /// `nut_psed`'s fractions, kept when there is no attached phosphorus.
    pub fr_orgp: f32,
    pub fr_actmin: f32,
    pub fr_stamin: f32,
    /// `nut_orgnc2`'s microbial carbon lost with sediment, taken again on later days.
    pub c_microb_sed: f32,
    /// `pest_enrsb`'s enrichment ratio (`enratio` of the HRU module; `varinit` zeroes a local of
    /// the same name), so `swr_subwq` uses the last one computed for any HRU.
    pub enratio: f32,
}

impl Default for Saved {
    fn default() -> Saved {
        Saved {
            rndseed_cond: 748932582,
            delg_p: 0.0,
            uno3d: [0.0; 21],
            uapd: [0.0; 21],
            un2: [0.0; 21],
            up2: [0.0; 21],
            fr_orgp: 0.0,
            fr_actmin: 0.0,
            fr_stamin: 0.0,
            c_microb_sed: 0.0,
            enratio: 0.0,
        }
    }
}

/// `plant_init` for the community's single plant. `wgn_months` gives each month's mean maximum
/// and minimum temperature, `ndays` the month table, `lat` the generator's latitude, `zmx` the soil's maximum rooting depth from
/// `soils.sol` (the HRU's own comes later, in `hydro_init`) and `layers` the depths and
/// thicknesses for the root distribution.
#[allow(clippy::too_many_arguments)]
pub fn plant_init(
    db: PlantDb,
    init: &swat_model::PlantInit,
    wgn_months: &[(f32, f32); 12],
    ndays: &[i32; 13],
    lat: f32,
    zmx: f32,
    layers: &[(f32, f32)],
    epco: f32,
) -> Result<Plant, String> {
    let mut pl = Plant { cpnm: init.cpnm.clone(), gro: init.igro == "y", idorm: true, ..Plant::default() };
    let r = init.rsdin;
    pl.rsd = OrgMass { m: 0.0 + r, c: 0.0 + 0.42 * r, n: 0.0 + 0.42 * r / 10.0, p: 0.0 + 0.42 * r / 100.0 };

    // xmon leaves the month as it was for a day past the year's end.
    let mut mo_prev = 1_i32;
    let mut tave_of = |iday: i32| {
        let (mo, _) = crate::clock::xmon(ndays, iday);
        if mo > 0 {
            mo_prev = mo;
        }
        let (mx, mn) = wgn_months.get((mo_prev - 1).max(0) as usize).copied().unwrap_or((0.0, 0.0));
        (mx + mn) / 2.0
    };
    let mut phu0 = 0.0_f32;
    for iday in 1..=365 {
        let tave = tave_of(iday);
        if tave > 0.0 {
            phu0 += tave;
        }
    }
    if db.days_mat == 0 {
        let mut phutot = 0.0_f32;
        for iday in 1..=365 {
            let phuday = tave_of(iday) - db.t_base;
            if phuday > 0.0 {
                phutot += phuday;
            }
        }
        pl.phumat = 0.95 * phutot;
    } else if db.days_mat < 0 {
        pl.phumat = -pl.phumat;
    } else {
        if !db.annual() {
            // igrow would be left from an earlier call (implicit SAVE) or set from day length.
            return Err(format!("plant {}: days to maturity {} for a {} plant (annuals only so far)", db.name, db.days_mat, db.typ));
        }
        // The planting day: when base-0 heat units reach 0.15 of the year's.
        let mut iday_sum = 181_i32;
        let mut iday_sh = 181_i32;
        let mut phutot = 0.0_f32;
        phu0 *= 0.15;
        let mut igrow = None;
        for iday in 1..=365 {
            let tave = if lat > 0.0 {
                tave_of(iday)
            } else {
                iday_sum += 1;
                iday_sh = if iday_sum > 365 { iday_sum - 365 } else { iday_sum };
                tave_of(iday_sh)
            };
            if tave > 0.0 {
                phutot += tave;
            }
            if phutot > phu0 {
                igrow = Some(if lat > 0.0 { iday } else { iday_sh });
                break;
            }
        }
        let Some(igrow) = igrow else {
            return Err(format!("plant {}: base-0 heat units never reach the planting share", db.name));
        };
        phutot = 0.0;
        for iday in igrow..=igrow + db.days_mat {
            let d = if lat > 0.0 || iday <= 365 { iday } else { iday - 365 };
            let phuday = tave_of(d) - db.t_base;
            if phuday > 0.0 {
                phutot += phuday;
            }
        }
        pl.phumat = phutot;
    }

    pl.phuacc = init.phuacc;
    let matur_frac;
    if db.perennial() {
        matur_frac = init.fr_yrmat;
        pl.cht = matur_frac * db.chtmx;
    } else {
        matur_frac = init.phuacc;
        let f = pl.phuacc / (pl.phuacc + (db.leaf1 - db.leaf2 * pl.phuacc).exp());
        pl.cht = db.chtmx * f.sqrt();
    }
    pl.laimxfr = matur_frac / (matur_frac + (db.leaf1 - db.leaf2 * matur_frac).exp());
    pl.lai = if init.igro == "y" { init.lai } else { 0.0 };
    pl.tot.m = init.bioms;
    pl.curyr_mat = ((init.fr_yrmat * db.mat_yrs as f32) as i32).max(1);
    pl.phumat_p = pl.phumat * db.mat_yrs as f32;
    pl.n_fr = (db.pltnfr1 - db.pltnfr3) * (1.0 - matur_frac / (matur_frac + (db.nup1 - db.nup2 * matur_frac).exp())) + db.pltnfr3;
    pl.tot.n = pl.n_fr * pl.tot.m;
    pl.p_fr = (db.pltpfr1 - db.pltpfr3) * (1.0 - matur_frac / (matur_frac + (db.pup1 - db.pup2 * matur_frac).exp())) + db.pltpfr3;
    pl.tot.p = pl.p_fr * pl.tot.m;
    if init.pop >= 1.0e-6 {
        return Err(format!("plant {}: plant population (not ported yet)", db.name));
    }
    pl.harv_idx = db.hvsti;
    pl.lai_pot = db.blai;
    pl.rtfr = vec![0.0; layers.len()];
    pl.soil_rsd = vec![OrgMass::default(); layers.len()];
    pl.db = db;
    if pl.gro {
        if pl.db.perennial() {
            pl.phuacc_p = if pl.phumat_p > 0.0 { init.fr_yrmat + (pl.phumat * init.phuacc) / pl.phumat_p } else { init.fr_yrmat };
        }
        pl.root_gro(zmx, layers);
        pl.seed_gro(0.0, 0.0, 0.0);
        pl.partition(OrgMass::default(), true);
        pl.rootfr(layers);
    }
    pl.epco = epco;
    pl.strsw = 1.0;
    pl.strst = 1.0;
    pl.strsn = 1.0;
    pl.strsp = 1.0;
    pl.strsa = 1.0;
    pl.strss = 1.0;
    Ok(pl)
}

impl Plant {
    /// `plstrz`: no stress.
    pub(super) fn reset_stress(&mut self) {
        self.strsw = 1.0;
        self.strst = 1.0;
        self.strsn = 1.0;
        self.strsp = 1.0;
        self.strsa = 1.0;
        self.strss = 1.0;
        self.reg = 0.0;
        self.sum_w = 0.0;
        self.sum_tmp = 0.0;
        self.sum_n = 0.0;
        self.sum_p = 0.0;
        self.sum_a = 0.0;
    }

    /// SWAT+ `pl_root_gro`: rooting depth and the root share of biomass.
    fn root_gro(&mut self, zmx: f32, layers: &[(f32, f32)]) {
        let db = &self.db;
        self.root_dep = if db.annual() { 2.5 * self.phuacc * 1000.0 * db.rdmx } else { 2.5 * self.phuacc_p * 1000.0 * db.rdmx };
        if self.root_dep > zmx {
            self.root_dep = zmx;
        }
        if self.root_dep < 25.4 {
            self.root_dep = 25.4;
        }
        if db.perennial() {
            if db.mat_yrs > 0 {
                let rto = self.curyr_mat as f32 / db.mat_yrs as f32;
                self.root_frac = if rto < 0.2 { db.rsr1 - (db.rsr1 - db.rsr2) * rto / 0.2 } else { db.rsr2 };
            } else {
                self.root_frac = db.rsr2;
            }
        } else {
            let phumax = self.phuacc.min(1.0);
            self.root_frac = db.rsr1 - (db.rsr1 - db.rsr2) * phumax;
        }
        self.rootfr(layers);
    }

    /// SWAT+ `pl_rootfr`: the share of roots in each layer. `layers` are (depth, thickness).
    pub(super) fn rootfr(&mut self, layers: &[(f32, f32)]) {
        for v in self.rtfr.iter_mut() {
            *v = 0.0;
        }
        if self.root_dep < 1.0e-6 {
            if let Some(v) = self.rtfr.first_mut() {
                *v = 1.0;
            }
            return;
        }
        let (a, b, c, d) = (1.15_f32, 11.7_f32, 0.022_f32, 0.12029_f32);
        let mut k = 0usize;
        let mut cum_rf = 0.0_f32;
        for (ly, &(dep, thick)) in layers.iter().enumerate() {
            let cum_rd = if dep >= self.root_dep { self.root_dep } else { dep };
            let x1 = (cum_rd - thick) / self.root_dep;
            let x2 = cum_rd / self.root_dep;
            let mut xx1 = -b * x1;
            if xx1 > 20.0 {
                xx1 = 20.0;
            }
            let mut xx2 = -b * x2;
            if xx2 > 20.0 {
                xx2 = 20.0;
            }
            self.rtfr[ly] = (a / b * (xx1.exp() - xx2.exp()) + c * (x2 - x1)) / d;
            let xx = cum_rf;
            cum_rf += self.rtfr[ly];
            if cum_rf > 1.0 {
                self.rtfr[ly] = 1.0 - xx;
                cum_rf = 1.0;
            }
            k = ly + 1;
            if cum_rd >= self.root_dep {
                break;
            }
        }
        for ly in 0..layers.len() {
            self.rtfr[ly] /= cum_rf;
            if ly + 1 == k {
                break;
            }
        }
    }

    /// SWAT+ `pl_seed_gro`: the harvest index as the season goes. `station_tave` is the
    /// unadjusted temperature of the station (the Fortran reads `wst(ob(j)%wst)`).
    fn seed_gro(&mut self, ep_day: f32, pet_day: f32, station_tave: f32) {
        let db = &self.db;
        let mut ajhi = self.harv_idx * 100.0 * self.phuacc / (100.0 * self.phuacc + (11.1_f32 - 10.0 * self.phuacc).exp());
        if self.phuacc > 0.5 {
            self.plet += ep_day;
            self.plpet += pet_day;
            let mut etr;
            if self.plpet > 1.0e-6 {
                etr = 100.0 * self.plet / self.plpet;
                let ajhi_min = ajhi / 2.0;
                ajhi = (ajhi - ajhi_min) * (etr / (etr + (6.13_f32 - 0.0883 * etr).exp())) + ajhi_min;
                if ajhi > db.hvsti {
                    ajhi = db.hvsti;
                }
            } else {
                etr = 1.0;
            }
            let dhi = ajhi - self.hi_prev;
            etr /= etr + (6.13_f32 - 0.0883 * etr).exp();
            let _ = etr;
            let _ = station_tave;
            self.hi_adj += dhi;
            self.hi_adj = db.hvsti.min(self.hi_adj);
            self.hi_adj = self.hi_adj.max(0.0);
            self.hi_prev = ajhi;
        } else {
            self.hi_prev = 0.0;
        }
    }

    /// SWAT+ `pl_partition`: the day's growth `up` shared between roots, seed, leaves and stem,
    /// or (`init`) the whole biomass divided by the target fractions.
    fn partition(&mut self, up: OrgMass, init: bool) {
        let db = self.db.clone();
        self.tot.m += up.m;
        self.tot.c += up.c;
        let leaf_frac_veg = if db.perennial() { 0.02 } else { 0.30 };
        let leaf_mass_frac_veg = leaf_frac_veg * self.lai / self.lai_pot;
        let root_frac;
        let ab_gr_frac;
        let seed_mass_frac;
        let leaf_mass_frac;
        let stem_mass_frac;
        if db.typ == "warm_annual_tuber" || db.typ == "cold_annual_tuber" {
            root_frac = self.root_frac;
            ab_gr_frac = (1.0 - root_frac) / (1.0 + self.hi_adj);
            seed_mass_frac = 1.0 - root_frac - ab_gr_frac;
            leaf_mass_frac = leaf_mass_frac_veg * ab_gr_frac;
            stem_mass_frac = (1.0 - leaf_mass_frac_veg) * ab_gr_frac;
        } else {
            root_frac = self.root_frac;
            ab_gr_frac = 1.0 - root_frac;
            seed_mass_frac = self.hi_adj;
            leaf_mass_frac = leaf_mass_frac_veg * (1.0 - seed_mass_frac);
            stem_mass_frac = 1.0 - (leaf_mass_frac_veg + seed_mass_frac);
        }
        if !init {
            let mut mass_left = up.m;
            for (part, frac) in [(0usize, root_frac), (1, seed_mass_frac), (2, leaf_mass_frac)] {
                let mass = match part {
                    0 => &mut self.root.m,
                    1 => &mut self.seed.m,
                    _ => &mut self.leaf.m,
                };
                let mass_act = *mass;
                let mass_opt = frac * self.tot.m;
                if mass_act < mass_opt {
                    let mut mass_add = mass_opt - mass_act;
                    mass_add = mass_add.min(mass_left);
                    *mass += mass_add;
                    mass_left -= mass_add;
                }
            }
            self.stem.m += mass_left;
            self.ab_gr.m = self.stem.m + self.leaf.m + self.seed.m;
        } else {
            self.ab_gr.m = ab_gr_frac * self.tot.m;
            self.root.m = root_frac * self.tot.m;
            self.leaf.m = leaf_mass_frac * self.ab_gr.m;
            self.seed.m = seed_mass_frac * self.ab_gr.m;
            self.stem.m = stem_mass_frac * self.ab_gr.m;
        }
        self.leaf.c = 0.41 * self.leaf.m;
        self.stem.c = 0.46 * self.stem.m;
        self.seed.c = 0.45 * self.seed.m;
        self.root.c = 0.46 * self.root.m;
        self.ab_gr.c = self.leaf.c + self.stem.c + self.seed.c;
        self.tot.c = self.ab_gr.c + self.root.c;
        if db.perennial() {
            let mass_left = self.leaf.m + self.stem.m + self.root.m;
            if mass_left > 1.0e-9 {
                self.seed.n = db.cnyld * self.seed.m;
                let mut n_left = self.tot.n - self.seed.n;
                if n_left < 0.0 {
                    self.seed.n = 0.0;
                    n_left += self.seed.n;
                }
                self.leaf.n = n_left * self.leaf.m / mass_left;
                self.stem.n = n_left * self.stem.m / mass_left;
                self.root.n = n_left * self.root.m / mass_left;
                self.ab_gr.n = self.seed.n + self.leaf.n + self.stem.n;
                self.seed.p = db.cpyld * self.seed.m;
                let mut p_left = self.tot.p - self.seed.p;
                if p_left < 0.0 {
                    self.seed.p = 0.0;
                    p_left += self.seed.p;
                }
                self.leaf.p = p_left * self.leaf.m / mass_left;
                self.stem.p = p_left * self.stem.m / mass_left;
                self.root.p = p_left * self.root.m / mass_left;
                self.ab_gr.p = self.seed.p + self.leaf.p + self.stem.p;
            }
        } else {
            self.seed.n = db.cnyld * self.seed.m;
            let mut n_left = self.tot.n - self.seed.n;
            if n_left < 0.0 {
                self.seed.n = 0.9 * self.tot.n;
                n_left = 0.1 * self.tot.n;
            }
            let n_frac = if self.tot.m - self.seed.m > 1.0e-6 { n_left / (self.tot.m - self.seed.m) } else { 0.0 };
            self.leaf.n = n_frac * self.leaf.m;
            self.stem.n = n_frac * self.stem.m;
            self.root.n = n_frac * self.root.m;
            self.ab_gr.n = self.seed.n + self.leaf.n + self.stem.n;
            self.seed.p = db.cpyld * self.seed.m;
            let mut p_left = self.tot.p - self.seed.p;
            if p_left < 0.0 {
                self.seed.p = 0.9 * self.tot.p;
                p_left = 0.1 * self.tot.p;
            }
            let p_frac = if self.tot.m - self.seed.m > 1.0e-6 { p_left / (self.tot.m - self.seed.m) } else { 0.0 };
            self.leaf.p = p_frac * self.leaf.m;
            self.stem.p = p_frac * self.stem.m;
            self.root.p = p_frac * self.root.m;
            self.ab_gr.p = self.seed.p + self.leaf.p + self.stem.p;
        }
    }

    /// SWAT+ `pl_tstr`: temperature stress. Its first estimate is always replaced: the test that
    /// should keep it (`rto > 0 .or. rto < 2`) is always true.
    fn tstr(&mut self, w: &PlantWeather) {
        let db = &self.db;
        let tgx = w.tave - db.t_base;
        if tgx <= 0.0 {
            self.strst = 0.0;
        }
        let rto = (w.tave - db.t_base) / (db.t_opt - db.t_base);
        self.strst = (1.5707_f32 * rto).sin();
        self.strst = self.strst.max(0.0);
        self.strst = self.strst.min(1.0);
    }

    /// SWAT+ `pl_nupd`: today's nitrogen demand.
    fn nupd(&mut self, d: &mut PlantDay) {
        let db = &self.db;
        let matur_frac = if db.perennial() { self.curyr_mat as f32 / db.mat_yrs as f32 } else { self.phuacc };
        self.n_fr = (db.pltnfr1 - db.pltnfr3) * (1.0 - matur_frac / (matur_frac + (db.nup1 - db.nup2 * matur_frac).exp())) + db.pltnfr3;
        d.un2 = self.n_fr * self.tot.m;
        if d.un2 < self.tot.n {
            d.un2 = self.tot.n;
        }
        d.uno3d = d.un2 - self.tot.n;
    }

    /// SWAT+ `pl_pupd`: today's phosphorus demand.
    fn pupd(&mut self, d: &mut PlantDay) {
        let db = &self.db;
        let matur_frac = if db.perennial() { self.curyr_mat as f32 / db.mat_yrs as f32 } else { self.phuacc };
        self.p_fr = (db.pltpfr1 - db.pltpfr3) * (1.0 - matur_frac / (matur_frac + (db.pup1 - db.pup2 * matur_frac).exp())) + db.pltpfr3;
        d.up2 = self.p_fr * self.ab_gr.m;
        if d.up2 < self.ab_gr.p {
            d.up2 = self.ab_gr.p;
        }
        d.uapd = d.up2 - self.ab_gr.p;
        d.uapd *= 1.5;
    }

    /// SWAT+ `pl_leaf_gro`: leaf area and canopy height.
    /// `chts` are the community's canopy heights (this plant's, `me`, is taken as it is now).
    fn leaf_gro(&mut self, chts: &[f32], me: usize) {
        let db = self.db.clone();
        let f = self.phuacc / (self.phuacc + (db.leaf1 - db.leaf2 * self.phuacc).exp());
        self.laimxfr = f.min(self.laimxfr);
        let ff = f - self.laimxfr;
        self.laimxfr = f;
        let mut exponent = db.leaf1 - db.leaf2 * self.phuacc_p;
        if exponent < -16.0 {
            exponent = -16.0;
        }
        let f_p = self.phuacc_p / (self.phuacc_p + exponent.exp());
        if self.phuacc < db.dlai {
            let laimax = if db.perennial() {
                if self.curyr_mat < 1 {
                    self.curyr_mat = 1;
                }
                let rto_lin = self.curyr_mat as f32 / db.mat_yrs as f32;
                let rto = rto_lin.log10();
                let lai_exp = rto * db.laixco_tree;
                (self.lai_pot * 10.0_f32.powf(lai_exp)).min(self.lai_pot)
            } else {
                self.lai_pot
            };
            self.cht = if db.perennial() { db.chtmx * f_p.sqrt() } else { db.chtmx * f.sqrt() };
            if self.lai > laimax {
                self.lai = laimax;
            }
            let mut deltalai = ff * laimax * (1.0 - (5.0 * (self.lai - laimax)).exp()) * self.strsw.sqrt();
            // The sum takes this plant's leaf area with every plant's height.
            let mut sumlaiht = 0.0_f32;
            for (j, &c) in chts.iter().enumerate() {
                sumlaiht += self.lai * if j == me { self.cht } else { c };
            }
            let rto = if sumlaiht > 1.0e-6 { (self.lai * self.cht) / sumlaiht } else { 1.0 };
            deltalai *= rto;
            self.lai += deltalai;
            if self.lai > laimax {
                self.lai = laimax;
            }
            self.olai = self.lai;
            self.dphu = self.phuacc;
        }
    }

    /// SWAT+ `pl_leaf_senes`. Returns the leaves dropped to the residue. A perennial growing on
    /// temperature drops leaf area but no mass: its turnover rate (`leaf_tov`) is only ever set
    /// for plants growing on moisture.
    fn leaf_senes(&mut self) -> OrgMass {
        let db = self.db.clone();
        let mut leaf_drop = OrgMass::default();
        if db.annual() && self.phuacc > db.dlai && self.phuacc < 1.0 {
            let rto = (1.0 - self.phuacc) / (1.0 - self.dphu);
            self.lai = self.olai * rto.powf(db.dlai_rate);
        }
        if db.perennial() && db.trig == "temp_gro" && self.phuacc > db.dlai && self.d_senes < 15.0 {
            self.d_senes += 1.0;
            let rto = 1.0 - (self.d_senes / 15.0);
            self.lai = (self.olai - db.alai_min) * rto + db.alai_min;
            self.lai = self.lai.max(db.alai_min);
            leaf_drop.m = self.leaf_tov * self.leaf.m;
            leaf_drop.n = (0.68 * leaf_drop.m * self.n_fr).max(0.0);
            leaf_drop.p = (0.68 * leaf_drop.m * self.p_fr).max(0.0);
        }
        if leaf_drop.m > 0.0 {
            self.leaf = self.leaf.sub(leaf_drop);
            self.tot = self.tot.sub(leaf_drop);
            self.ab_gr = self.ab_gr.sub(leaf_drop);
            self.rsd = self.rsd.add(leaf_drop);
            return leaf_drop;
        }
        OrgMass::default()
    }
}

impl Hru {
    /// SWAT+ `pl_community` with its calls of `pl_waterup`: community totals, transpiration and
    /// the light each plant intercepts.
    pub(super) fn pl_community(&mut self, d: &mut PlantDay, solrad: f32, b: &PlantBasin) {
        self.lai_sum = 0.0;
        for pl in &self.plants {
            self.lai_sum += pl.lai;
        }
        let lai_sum = self.lai_sum;
        let epmax: Vec<f32> = self.plants.iter().map(|pl| if lai_sum > 1.0e-6 { d.ep_max * pl.lai / lai_sum } else { 0.0 }).collect();
        self.tot_com = OrgMass::default();
        self.ab_gr_com = OrgMass::default();
        for pl in &self.plants {
            self.tot_com = self.tot_com.add(pl.tot);
            self.ab_gr_com = self.ab_gr_com.add(pl.ab_gr);
        }
        self.cht_mx = 0.0;
        for pl in &self.plants {
            self.cht_mx = self.cht_mx.max(pl.cht);
        }
        for i in 0..self.plants.len() {
            if self.plants[i].gro {
                self.pl_waterup(i, epmax[i], d, b);
            }
        }
        for pl in self.plants.iter_mut() {
            pl.par = 0.0;
            if !pl.idorm && pl.gro {
                pl.par = 0.5 * solrad * (1.0 - (-pl.db.ext_coef * (pl.lai + 0.05)).exp());
            }
        }
    }

    /// SWAT+ `pl_waterup` for plant `i`: aeration stress and water uptake by root depth.
    fn pl_waterup(&mut self, i: usize, epmax: f32, d: &mut PlantDay, b: &PlantBasin) {
        let (sw, sumfc, sumul) = (self.sw, self.sumfc, self.sumul);
        let pl = &mut self.plants[i];
        if sw > sumfc {
            let satco = (sw - sumfc) / (sumul - sumfc);
            let scparm = 100.0 * (satco - pl.db.aeration) / (1.0001 - pl.db.aeration);
            pl.strsa = if scparm > 0.0 { 1.0 - (scparm / (scparm + (2.9014_f32 - 0.03867 * scparm).exp())) } else { 1.0 };
        }
        if epmax <= 1.0e-6 {
            pl.strsw = 1.0;
            return;
        }
        let mut ir = 0usize;
        let mut sump = 0.0_f32;
        let mut sum_wuse = 0.0_f32;
        let mut sum_wusep = 0.0_f32;
        let root_dep = pl.root_dep;
        let epco = pl.epco;
        for k in 0..self.layers.len() {
            if ir > 0 {
                break;
            }
            let l = &mut self.layers[k];
            let gx = if root_dep <= l.d {
                ir = k + 1;
                root_dep
            } else {
                l.d
            };
            let sum =
                if root_dep <= 0.01 { epmax / b.water_norm } else { epmax * (1.0 - (-10.0_f32 * gx / root_dep).exp()) / b.water_norm };
            let mut wuse = if k == 1 { sum } else { sum - sump * (1.0 - epco) };
            wuse = wuse.min(l.st);
            sum_wuse += wuse;
            if sum_wuse > epmax {
                wuse = epmax - sum_wusep;
                sum_wuse = epmax;
            }
            sump = sum;
            sum_wusep = sum_wuse;
            if l.st < wuse {
                wuse = l.st;
            }
            l.st = (l.st - wuse).max(1.0e-6);
        }
        self.sw = 0.0;
        for l in &self.layers {
            self.sw += l.st;
        }
        let pl = &mut self.plants[i];
        pl.strss = 1.0;
        pl.strsw = sum_wuse / epmax;
        d.ep_day += sum_wuse;
    }

    /// SWAT+ `pl_grow` (with `pl_nut_demand` before it): heat units, dormancy and, for each plant
    /// awake and growing, biomass, roots, leaves, senescence, seed and partition. The nutrient
    /// demands are the module arrays `uno3d`, `uapd`, `un2`, `up2` by plant number, shared by all
    /// HRUs (see `Saved`).
    pub(super) fn pl_grow(&mut self, d: &mut PlantDay, w: &PlantWeather, b: &PlantBasin, saved: &mut Saved) {
        // pl_nut_demand zeroes the element of the loop counter left by pl_community (one past
        // the last plant) instead of the plants'.
        // Corrected: every plant's demand starts at zero, so a plant that isn't growing doesn't
        // take another HRU's.
        if b.fixes {
            for k in 1..=self.plants.len().min(saved.uno3d.len() - 1) {
                saved.uno3d[k] = 0.0;
                saved.uapd[k] = 0.0;
                saved.un2[k] = 0.0;
                saved.up2[k] = 0.0;
            }
        }
        let stale = self.plants.len() + 1;
        if !b.fixes && stale < saved.uno3d.len() {
            saved.uno3d[stale] = 0.0;
            saved.uapd[stale] = 0.0;
        }
        d.uno3d_tot = 0.0;
        d.uapd_tot = 0.0;
        for (i, pl) in self.plants.iter_mut().enumerate() {
            if !pl.idorm && pl.gro {
                let mut delg = 0.0_f32;
                if pl.phumat > 0.1 {
                    delg = (w.tave - pl.db.t_base) / pl.phumat;
                    saved.delg_p = (w.tave - pl.db.t_base) / pl.phumat_p;
                }
                if delg < 0.0 {
                    delg = 0.0;
                }
                if saved.delg_p < 0.0 {
                    saved.delg_p = 0.0;
                }
                pl.phuacc += delg;
                pl.phuacc_p += saved.delg_p;
                pl.nupd(d);
                pl.pupd(d);
                let k = i + 1;
                (saved.uno3d[k], saved.un2[k], saved.uapd[k], saved.up2[k]) = (d.uno3d, d.un2, d.uapd, d.up2);
                d.uno3d_tot += d.uno3d;
                d.uapd_tot += d.uapd;
            }
        }
        d.sum_no3 = 0.0;
        d.sum_solp = 0.0;
        for l in &self.layers {
            d.sum_no3 += l.no3;
            d.sum_solp += l.lab;
        }

        d.bm_grow = 0.0;
        d.c_gro = 0.0;
        for i in 0..self.plants.len() {
            let k = i + 1;
            (d.uno3d, d.un2, d.uapd, d.up2) = (saved.uno3d[k], saved.un2[k], saved.uapd[k], saved.up2[k]);
            let mut pl = std::mem::take(&mut self.plants[i]);
            let mut up = OrgMass::default();
            if pl.gro && pl.db.trig == "temp_gro" {
                self.pl_dormant(&mut pl, w);
            }
            if !pl.idorm && pl.gro {
                self.pl_biomass_gro(&mut pl, d, w, b, &mut up);
                let zmx = self.zmx;
                let layers: Vec<(f32, f32)> = self.layers.iter().map(|l| (l.d, l.thick)).collect();
                pl.root_gro(zmx, &layers);
                let chts: Vec<f32> = self.plants.iter().enumerate().map(|(j, p)| if j == i { f32::NAN } else { p.cht }).collect();
                pl.leaf_gro(&chts, i);
                let drop = pl.leaf_senes();
                if drop.m > 0.0 {
                    self.rsd_tot = self.rsd_tot.add(drop);
                    self.tot_com = self.tot_com.sub(drop);
                    self.ab_gr_com = self.ab_gr_com.sub(drop);
                }
                pl.seed_gro(d.ep_day, d.pet_day, w.station_tave);
                pl.partition(up, false);
            }
            if w.end_yr {
                self.pl_mortality(&mut pl);
            }
            self.plants[i] = pl;
            (saved.uno3d[k], saved.un2[k], saved.uapd[k], saved.up2[k]) = (d.uno3d, d.un2, d.uapd, d.up2);
        }
    }

    /// SWAT+ `pl_dormant` for perennials (annuals other than cold-season ones don't go dormant).
    fn pl_dormant(&mut self, pl: &mut Plant, w: &PlantWeather) {
        if !pl.idorm && w.daylength - w.dormhr < w.daylmn {
            if pl.db.perennial() {
                pl.idorm = true;
                let rto = 0.0_f32;
                let stem_drop = OrgMass::scale(rto, pl.stem);
                pl.lai *= rto;
                // The carbon of the dropped leaves is never set (`leaf_drop%c` stays 0).
                let mut leaf_drop = OrgMass { m: rto * pl.leaf.m, ..OrgMass::default() };
                leaf_drop.n = (leaf_drop.m * pl.n_fr).max(0.0);
                leaf_drop.p = (leaf_drop.m * pl.p_fr).max(0.0);
                let seed_drop = pl.seed;
                let abgr_drop = stem_drop.add(leaf_drop).add(seed_drop);
                pl.tot = pl.tot.sub(abgr_drop);
                if self.tot_com.m < 0.0 {
                    self.tot_com.m = 0.0;
                }
                pl.ab_gr = pl.ab_gr.sub(abgr_drop);
                pl.stem = pl.stem.sub(stem_drop);
                pl.leaf = pl.leaf.sub(leaf_drop);
                pl.seed = OrgMass::default();
                pl.rsd = pl.rsd.add(abgr_drop);
                self.rsd_tot = self.rsd_tot.add(abgr_drop);
                pl.phuacc = 0.0;
                pl.strsw = 1.0;
            }
            if pl.db.typ == "cold_annual" && pl.phuacc < 0.75 {
                pl.idorm = true;
                pl.strsw = 1.0;
            }
        }
        if pl.idorm && w.daylength - w.dormhr >= w.daylmn {
            if pl.db.perennial() {
                pl.idorm = false;
                pl.phuacc = 0.0;
                pl.d_senes = 0.0;
            }
            if pl.db.typ == "cold_annual" {
                pl.idorm = false;
                pl.phuacc = 0.0;
            }
        }
    }

    /// SWAT+ `pl_biomass_gro`: the day's biomass from intercepted light, with the stresses
    /// (temperature, nitrogen and phosphorus uptake) that reduce it.
    fn pl_biomass_gro(&mut self, pl: &mut Plant, d: &mut PlantDay, w: &PlantWeather, b: &PlantBasin, up: &mut OrgMass) {
        if pl.phuacc > 1.0 {
            return;
        }
        let db = pl.db.clone();
        let mut beadj = if b.co2 > 350.0 { 100.0 * b.co2 / (b.co2 + (db.ruc1 - b.co2 * db.ruc2).exp()) } else { db.bio_e };
        if d.vpd > 1.0 {
            let ruedecl = d.vpd - 1.0;
            beadj -= db.wavp * ruedecl;
            beadj = beadj.max(0.27 * db.bio_e);
        }
        let mut bioday = beadj * pl.par;
        if bioday < 0.0 {
            bioday = 0.0;
        }
        pl.tstr(w);
        d.rto_no3 = if d.uno3d_tot > d.sum_no3 { d.uno3d / d.uno3d_tot } else { 1.0 };
        d.rto_solp = if d.uapd_tot > d.sum_solp { d.uapd / d.uapd_tot } else { 1.0 };
        d.uno3d = (4.0 * db.pltnfr3 * bioday).min(d.uno3d);
        if d.uapd > 10.0 {
            d.uapd = (4.0 * db.pltpfr3 * bioday).min(d.uapd);
        }
        self.pl_nup(pl, d, b, up);
        self.pl_pup(pl, d, b, up);
        if b.nostress == 1 {
            pl.strsw = 1.0;
            pl.strst = 1.0;
            pl.strsn = 1.0;
            pl.strsp = 1.0;
            pl.strsa = 1.0;
            pl.strss = 1.0;
        }
        if b.nostress == 2 {
            pl.strsn = 1.0;
            pl.strsp = 1.0;
        }
        pl.reg = pl.strsw.min(pl.strst).min(pl.strsn).min(pl.strsp).min(pl.strsa).min(pl.strss);
        if pl.reg < 0.0 {
            pl.reg = 0.0;
        }
        if pl.reg > 1.0 {
            pl.reg = 1.0;
        }
        up.m = bioday * pl.reg;
        up.c = 0.42 * up.m;
        d.bm_grow += up.m;
        d.c_gro += up.c;
        pl.sum_w += 1.0 - pl.strsw;
        pl.sum_tmp += 1.0 - pl.strst;
        pl.sum_n += 1.0 - pl.strsn;
        pl.sum_p += 1.0 - pl.strsp;
        pl.sum_a += 1.0 - pl.strsa;
    }

    /// SWAT+ `pl_nup` (and `pl_nfix`): nitrogen uptake by depth and nitrogen stress.
    fn pl_nup(&mut self, pl: &mut Plant, d: &mut PlantDay, b: &PlantBasin, up: &mut OrgMass) {
        pl.strsn = 1.0;
        d.nuptake = 0.0;
        if d.uno3d < 1.0e-6 {
            return;
        }
        let mut root_depth = 10.1_f32.max(pl.root_dep);
        for l in &self.layers {
            if root_depth < l.d {
                root_depth = l.d;
                break;
            }
        }
        for l in self.layers.iter_mut() {
            let soil_depth = l.d;
            if root_depth < soil_depth {
                break;
            }
            let unmx = d.uno3d * d.rto_no3 * (1.0 - (-b.n_updis * soil_depth / root_depth).exp()) / b.n_norm;
            let uno3l = (unmx - d.nplnt).min(l.no3);
            d.nplnt += uno3l;
            l.no3 -= uno3l;
        }
        if d.nplnt < 0.0 {
            d.nplnt = 0.0;
        }
        if pl.db.nfix_co > 1.0e-6 {
            d.fixn = self.pl_nfix(pl, d, b);
        }
        d.nplnt += d.fixn;
        pl.tot.n += d.nplnt;
        pl.ab_gr.n += d.nplnt * (1.0 - pl.root_frac);
        pl.root.n += d.nplnt * pl.root_frac;
        up.n = d.nplnt;
        d.nuptake += d.nplnt;
        if pl.db.nfix_co > 1.0e-6 {
            pl.strsn = 1.0;
        } else {
            pl.strsn = nuts(pl.tot.n, d.un2);
            let xx = if d.uno3d > 1.0e-5 { d.nplnt / d.uno3d } else { 1.0 };
            pl.strsn = pl.strsn.max(xx);
            pl.strsn = pl.strsn.min(1.0);
        }
    }

    /// SWAT+ `pl_nfix`: nitrogen fixation by legumes.
    fn pl_nfix(&self, pl: &Plant, d: &PlantDay, b: &PlantBasin) -> f32 {
        if d.uno3d <= d.nplnt {
            return 0.0;
        }
        let uno3l = d.uno3d - d.nplnt;
        let fxw = self.sw / (0.85 * self.sumfc);
        let mut sumn = 0.0_f32;
        for l in &self.layers {
            sumn += l.no3;
        }
        let mut fxn = 0.0_f32;
        if sumn > 300.0 {
            fxn = 0.0;
        }
        if sumn > 100.0 && sumn <= 300.0 {
            fxn = 1.5 - 0.005 * sumn;
        }
        if sumn <= 100.0 {
            fxn = 1.0;
        }
        let ph = pl.phuacc;
        let mut fxg = 0.0_f32;
        if ph > 0.15 && ph <= 0.30 {
            fxg = 6.67 * ph - 1.0;
        }
        if ph > 0.30 && ph <= 0.55 {
            fxg = 1.0;
        }
        if ph > 0.55 && ph <= 0.75 {
            fxg = 3.75 - 5.0 * ph;
        }
        let fxr = (1.0_f32.min(fxw).min(fxn) * fxg).max(0.0);
        let mut fixn = (6.0_f32).min(fxr * uno3l);
        fixn = pl.db.nfix_co * fixn + (1.0 - pl.db.nfix_co) * uno3l;
        fixn = fixn.min(uno3l);
        fixn.min(b.nfixmx)
    }

    /// SWAT+ `pl_pup`: phosphorus uptake by depth and phosphorus stress.
    fn pl_pup(&mut self, pl: &mut Plant, d: &mut PlantDay, b: &PlantBasin, up: &mut OrgMass) {
        pl.strsp = 1.0;
        d.puptake = 0.0;
        if d.uapd < 1.0e-6 {
            return;
        }
        let mut root_depth = 10.1_f32.max(pl.root_dep);
        for l in &self.layers {
            if root_depth < l.d {
                root_depth = l.d;
                break;
            }
        }
        for l in self.layers.iter_mut() {
            let soil_depth = l.d;
            if root_depth < soil_depth {
                break;
            }
            let upmx = d.uapd * d.rto_solp * (1.0 - exp_w(-b.p_updis * soil_depth / root_depth)) / b.p_norm;
            let uapl = (upmx - d.pplnt).min(l.lab);
            d.pplnt += uapl;
            l.lab -= uapl;
        }
        if d.pplnt < 0.0 {
            d.pplnt = 0.0;
        }
        pl.tot.p += d.pplnt;
        pl.ab_gr.p += d.pplnt * (1.0 - pl.root_frac);
        pl.root.p += d.pplnt * pl.root_frac;
        up.p = d.pplnt;
        d.puptake += d.pplnt;
        pl.strsp = nuts(pl.ab_gr.p, d.up2);
    }

    /// SWAT+ `pl_mortality` (end of the year): above-ground biomass over the perennial maximum dies.
    fn pl_mortality(&mut self, pl: &mut Plant) {
        let bm_dieoff = (1.0 + pl.db.bm_dieoff) * (pl.ab_gr.m - (pl.db.bmx_peren * 1000.0));
        if bm_dieoff > 1.0e-6 && pl.ab_gr.m > bm_dieoff {
            let rto = (bm_dieoff / pl.ab_gr.m).min(1.0);
            let rto1 = (1.0 - rto).max(0.0);
            pl.rsd = pl.rsd.add(OrgMass::scale(rto, pl.ab_gr));
            self.rsd_tot = self.rsd_tot.add(OrgMass::scale(rto, pl.ab_gr));
            for ly in 0..self.layers.len() {
                let add = OrgMass::scale(rto * pl.rtfr[ly], pl.root);
                pl.soil_rsd[ly] = pl.soil_rsd[ly].add(add);
            }
            pl.tot = OrgMass::scale(rto1, pl.tot);
            pl.ab_gr = OrgMass::scale(rto1, pl.ab_gr);
            pl.leaf = OrgMass::scale(rto1, pl.leaf);
            pl.stem = OrgMass::scale(rto1, pl.stem);
            pl.seed = OrgMass::scale(rto1, pl.seed);
            pl.root = OrgMass::scale(rto1, pl.root);
        }
    }
}
