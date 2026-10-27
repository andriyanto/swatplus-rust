//! HRUs: their setup (`proc_hru` and `time_conc_init`) and their daily water balance
//! (`hru_control`, `hru_output`).
//!
//! So far: snow, soil temperature, potential and actual evaporation, curve-number runoff with
//! its lag, percolation, lateral flow and its lag, saturation excess, erosion, a single-plant
//! community's growth and water use (`plant`), the soil nutrients of the original carbon model
//! (`nutrient`) or of the carbon model (`carbon`, `cswat` 2), and dated management (`mgt`). [`init_hrus`] says which inputs
//! need what isn't ported yet.

pub mod carbon;
pub mod mgt;
pub mod nutrient;
pub mod plant;
pub mod wetland;

use std::ops::Add;

use swat_model::schema;
use swat_model::table::{Field, Table, Value, default_row};
use swat_model::{HruData, Project, Soil};

use crate::run::HruWeather;
use nutrient::{NutBasin, NutDay, SoilTest};
use plant::{OrgMass, Plant, PlantBasin, PlantDay, PlantDb, PlantWeather, Saved};

/// One soil layer (`soil(j)%phys(ly)` and `soil(j)%ly(ly)`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layer {
    /// Depth to the bottom of the layer (mm).
    pub d: f32,
    pub thick: f32,
    pub bd: f32,
    pub awc: f32,
    pub k: f32,
    pub clay: f32,
    pub silt: f32,
    pub sand: f32,
    pub rock: f32,
    pub alb: f32,
    /// Wilting point and field capacity as fractions (`wp`, `up`), and porosity.
    pub wp: f32,
    pub up: f32,
    pub por: f32,
    /// Water at saturation and at field capacity above wilting point (mm), and stored (mm).
    pub ul: f32,
    pub fc: f32,
    pub st: f32,
    /// Travel time of percolation (h).
    pub hk: f32,
    /// Temperature (deg C).
    pub tmp: f32,
    /// Biological mixing efficiency the layer starts with (`ly%init_bmix`).
    pub init_bmix: f32,
    /// Organic carbon (%), wilting point water (mm) and USLE K.
    pub cbn: f32,
    pub wpmm: f32,
    pub usle_k: f32,
    /// The day's percolation out of the layer and lateral flow from it (`ly%prk`, `ly%flat`).
    pub prk: f32,
    pub flat: f32,
    /// Mineral nitrogen and phosphorus (kg/ha): `mn%no3`, `mn%nh4`, `mp%wsol`, `lab`, `act`, `sta`.
    pub no3: f32,
    pub nh4: f32,
    pub wsol: f32,
    pub lab: f32,
    pub act: f32,
    pub sta: f32,
    /// Active and stable humus (`hact`, `hsta`).
    pub hact: OrgMass,
    pub hsta: OrgMass,
    /// The carbon model's pools, and the mixing effect on decomposition (`ly%tillagef`,
    /// `tillagef_biomix`).
    pub cp: carbon::Pools,
    pub tillagef: f32,
    pub tillagef_biomix: f32,
    /// The tillage part of `tillagef`, and the biological mixing efficiency of the day (`ly%bmix`).
    pub tillagef_tillmix: f32,
    pub bmix: f32,
}

/// A tile drain (`tiledrain.str`): its depth (mm), the time to drain the soil to field capacity
/// (h), the drainage coefficient (mm a day), and the layer it is in (1-based, 0 for none).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tile {
    pub depth: f32,
    pub time: f32,
    pub drain_co: f32,
    pub ldrain: usize,
    /// The lag (h) and the share of the held-back tile flow released a day (`tile_ttime`).
    pub lag: f32,
    pub ttime: f32,
}

/// Snow parameters (`snow.sno` row).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snow {
    pub falltmp: f32,
    pub melttmp: f32,
    pub meltmx: f32,
    pub meltmn: f32,
    pub timp: f32,
    pub covmx: f32,
    pub cov50: f32,
    pub init_mm: f32,
}

/// An HRU's parameters and state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hru {
    pub area_ha: f32,
    pub km: f32,
    pub elev: f32,
    pub slope: f32,
    pub slope_len: f32,
    pub lat_len: f32,
    pub esco: f32,
    pub cn3_swf: f32,
    /// Percolation coefficient (`hydrology.hyd`), and the share of a layer's water above field
    /// capacity it lets percolate in a day.
    pub perco: f32,
    pub perco_lim: f32,
    pub latq_co: f32,
    pub pet_co: f32,
    /// Fraction of the lateral flow store released each day (`hyd%lat_ttime` after `hydro_init`).
    pub lat_ttime: f32,
    pub sno: Snow,
    pub snocov1: f32,
    pub snocov2: f32,
    /// Manning's n for overland flow (`luse%ovn`).
    pub ovn: f32,
    /// The urban land type (`urban.urb`) and how its runoff picks up loads, for urban HRUs.
    pub urban: Option<Urban>,
    /// Days of solids build-up on the impervious area (`twash`).
    pub twash: f32,
    /// The wetland on the HRU (`surf_stor`), its `hydrology.wet` row, and the depth evaporated
    /// from it today (mm, `water_evap`).
    pub wet: Option<wetland::Wetland>,
    pub wet_hyd_row: usize,
    pub water_evap: f32,
    /// Fraction of the surface runoff store released each day (`brt`).
    pub brt: f32,
    pub cn2: f32,
    /// Retention parameter at moisture condition I (`smx`) and the shape of its curve (`wrt`).
    pub smx: f32,
    pub wrt: [f32; 2],
    pub hydgrp: String,
    pub layers: Vec<Layer>,
    pub sumul: f32,
    pub sumfc: f32,
    pub sw: f32,
    pub avbd: f32,
    pub zmx: f32,
    pub sw_300: f32,
    pub sno_mm: f32,
    pub canstor: f32,
    /// Surface runoff and lateral flow waiting to leave (`surf_bs(1)`, `bss(1)`).
    pub surf_bs: f32,
    pub bss: f32,
    pub phubase: f32,
    /// Generator (0-based) of the HRU's station, for its annual temperature.
    pub wgn: usize,
    /// Soil water and snow at the start of the printed period (`hwb_d%sw_init`, `sno_init`).
    pub sw_init: f32,
    pub sno_init: f32,
    /// Latitude of the HRU object, for the yearly resets.
    pub lat: f32,
    pub canmx: f32,
    pub erorgn: f32,
    pub erorgp: f32,
    /// Biological mixing efficiency (`hyd%biomix`), used at the end of each year with `cswat` 0.
    pub hyd_biomix: f32,
    pub anion_excl: f32,
    /// Time of concentration (h) and the USLE factors K·P·LS·rock·11.8 (`lumv%usle_mult`).
    pub tconc: f32,
    pub usle_mult: f32,
    /// The plant community, in `plant.ini` order.
    pub plants: Vec<Plant>,
    pub lai_sum: f32,
    pub laimx_sum: f32,
    pub cht_mx: f32,
    /// Community totals as `pl_community` sums them each day, before the day's growth.
    pub tot_com: OrgMass,
    pub ab_gr_com: OrgMass,
    /// Surface residue of all plants (`pl_mass%rsd_tot`).
    pub rsd_tot: OrgMass,
    /// Lag stores of sediment and its nutrients, surface nitrate and soluble P (`surf_bs(2..8)`):
    /// sediment, organic N, organic P, nitrate, soluble P, active and stable mineral P.
    pub surf_bs_sed: [f32; 7],
    /// Lag stores of detached sand, silt, clay, small and large aggregates (`surf_bs(13..17)`).
    pub surf_bs_cls: [f32; 5],
    /// Lag store of lateral nitrate (`bss(2)`).
    pub bss_no3: f32,
    /// The management schedule, if any.
    pub mgt: Option<mgt::Mgt>,
    /// The last tillage: whether it still has effect, days since, depth (`tillage_switch`,
    /// `tillage_days`, `tillage_depth`).
    pub till_switch: bool,
    pub till_days: i32,
    pub till_depth: f32,
    /// The day's irrigation reaching the soil and running off (`irrig%applied`, `runoff`), and
    /// the fertilizer N and P applied (`fertn`, `fertp`).
    pub irr_applied: f32,
    pub irr_runoff: f32,
    /// Irrigation applied so far (`hru%irr_yr`; its yearly reset is commented out in SWAT+).
    pub irr_yr: f32,
    pub fertn: f32,
    pub fertp: f32,
    /// The yield of today's harvest (`pl_yield`), printed and zeroed with the day's output.
    pub pl_yield: plant::OrgMass,
    /// Today's harvests: the plant's number in the community and the yield (kg/ha) the basin
    /// crop table adds (`bsn_crop_yld`).
    pub harvests: Vec<(usize, f32)>,
    /// Days since the last planting, harvest, kill and irrigation (`pcom%days_plant`…), which
    /// decision tables test: -366 before the first.
    pub days_plant: i32,
    pub days_harv: i32,
    pub days_kill: i32,
    pub days_irr: i32,
    /// The initial soil water as a share of field capacity (`soil%ffc`), and `hyd%epco`.
    pub ffc: f32,
    pub epco: f32,
    /// Tile drains (`hru%sdr`), if any, and today's tile flow (mm, `qtile`).
    pub tile: Option<Tile>,
    pub qtile: f32,
    /// Tile flow and its nitrate held back (`bss(3)`, `bss(4)`).
    pub bss_tile: f32,
    pub bss_tno3: f32,
    /// The last day's atmospheric nitrate and ammonium (kg/ha), printed again on days without.
    pub atmo_prev: [f32; 2],
    /// A paddy's target and threshold ponding depths (mm, `hru%irr_hmax`, `irr_hmin`).
    pub irr_hmax: f32,
    pub irr_hmin: f32,
}

/// One day of `hru_pw_day` (plants and weather), the columns the port computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruPw {
    pub lai: f32,
    pub bioms: f32,
    pub yield_: f32,
    pub residue: f32,
    pub sol_tmp: f32,
    pub strsw: f32,
    pub strsa: f32,
    pub strstmp: f32,
    pub strsn: f32,
    pub strsp: f32,
    pub strss: f32,
    pub nplnt: f32,
    pub percn: f32,
    pub pplnt: f32,
    pub phubase0: f32,
    pub lai_max: f32,
    pub bm_max: f32,
    pub bm_grow: f32,
    pub c_gro: f32,
}

impl HruPw {
    pub fn columns(&self) -> [(&'static str, f32); 19] {
        [
            ("lai", self.lai),
            ("bioms", self.bioms),
            ("yield", self.yield_),
            ("residue", self.residue),
            ("sol_tmp", self.sol_tmp),
            ("strsw", self.strsw),
            ("strsa", self.strsa),
            ("strstmp", self.strstmp),
            ("strsn", self.strsn),
            ("strsp", self.strsp),
            ("strss", self.strss),
            ("nplt", self.nplnt),
            ("percn", self.percn),
            ("pplnt", self.pplnt),
            ("phubas0", self.phubase0),
            ("lai_max", self.lai_max),
            ("bm_max", self.bm_max),
            ("bm_grow", self.bm_grow),
            ("c_gro", self.c_gro),
        ]
    }
}

/// One day of `hru_nb_day` (nutrient balance), the columns the port computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruNb {
    pub lab_min_p: f32,
    pub act_sta_p: f32,
    pub fertn: f32,
    pub fertp: f32,
    pub fixn: f32,
    pub denit: f32,
    pub act_nit_n: f32,
    pub act_sta_n: f32,
    pub org_lab_p: f32,
    pub rsd_nitorg_n: f32,
    pub rsd_laborg_p: f32,
    /// Nitrate and ammonium from the atmosphere, in rain and dry (`no3atmo`, `nh4atmo`).
    pub no3atmo: f32,
    pub nh4atmo: f32,
    pub nuptake: f32,
    pub puptake: f32,
}

impl HruNb {
    pub fn columns(&self) -> [(&'static str, f32); 15] {
        [
            ("lab_min_p", self.lab_min_p),
            ("act_sta_p", self.act_sta_p),
            ("fertn", self.fertn),
            ("fertp", self.fertp),
            ("fixn", self.fixn),
            ("denit", self.denit),
            ("act_nit_n", self.act_nit_n),
            ("act_sta_n", self.act_sta_n),
            ("org_lab_p", self.org_lab_p),
            ("rsd_nitorg_n", self.rsd_nitorg_n),
            ("rsd_laborg_p", self.rsd_laborg_p),
            ("no3atmo", self.no3atmo),
            ("nh4atmo", self.nh4atmo),
            ("nuptake", self.nuptake),
            ("puptake", self.puptake),
        ]
    }
}

/// One day of `hru_ls_day` (losses), the columns the port computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruLs {
    pub sedyld: f32,
    pub sedorgn: f32,
    pub sedorgp: f32,
    pub surqno3: f32,
    pub latno3: f32,
    pub surqsolp: f32,
    pub usle: f32,
    pub sedminp: f32,
    pub tileno3: f32,
    pub lchlabp: f32,
    pub tilelabp: f32,
    pub satexn: f32,
    pub percn: f32,
}

impl HruLs {
    pub fn columns(&self) -> [(&'static str, f32); 13] {
        [
            ("sedyld", self.sedyld),
            ("sedorgn", self.sedorgn),
            ("sedorgp", self.sedorgp),
            ("surqno3", self.surqno3),
            ("lat3no3", self.latno3),
            ("surqsolp", self.surqsolp),
            ("usle", self.usle),
            ("sedminp", self.sedminp),
            ("tileno3", self.tileno3),
            ("lchlabp", self.lchlabp),
            ("tilelabp", self.tilelabp),
            ("satexn", self.satexn),
            ("percn", self.percn),
        ]
    }
}

/// An HRU's day: the four daily tables. `nutrients` is false when the soil nutrients follow a
/// carbon model not ported (`cswat` 1): then `nb`, `ls` and nitrate percolation aren't SWAT+'s.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HruDay {
    pub wb: HruWb,
    pub pw: HruPw,
    pub nb: HruNb,
    pub ls: HruLs,
    pub nutrients: bool,
    /// What leaves the HRU (`hru_hyds`): total, recharge, surface, lateral and tile flow
    /// (`ob%hd(1..5)`).
    pub hd: [crate::hyd::Hyd; 5],
    /// Today's harvests (plant number in the community, yield in kg/ha for the basin's crops).
    pub harvests: Vec<(usize, f32)>,
}

/// What an HRU's day needs besides its weather: the day of the year, its station's unadjusted
/// mean temperature, day length and peak half-hour rain fraction, its generator's values, and
/// whether the day ends the year.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DayCtx {
    pub day: i32,
    pub mo: i32,
    pub day_mo: i32,
    pub tmp_an: f32,
    pub phutot: f32,
    pub daylmn: f32,
    pub dormhr: f32,
    pub station_tave: f32,
    pub daylength: f32,
    pub precip_half_hr: f32,
    /// The station's precipitation (mm), and the year's last day.
    pub station_precip: f32,
    pub day_end_yr: i32,
    pub end_yr: bool,
}

/// One day of `hru_wb_day`, the columns the port computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HruWb {
    pub precip: f32,
    pub snofall: f32,
    pub snomlt: f32,
    pub surq_gen: f32,
    pub latq: f32,
    pub wateryld: f32,
    pub perc: f32,
    pub et: f32,
    pub ecanopy: f32,
    pub eplant: f32,
    pub esoil: f32,
    pub surq_cont: f32,
    pub cn: f32,
    pub sw_init: f32,
    pub sw_final: f32,
    pub sw: f32,
    pub sw_300: f32,
    pub sno_init: f32,
    pub sno_final: f32,
    pub snopack: f32,
    pub pet: f32,
    pub qtile: f32,
    pub irr: f32,
    pub satex: f32,
    pub satex_chan: f32,
    pub delsw: f32,
    pub lagsurf: f32,
    pub laglatq: f32,
    pub lagsatex: f32,
    pub wet_evap: f32,
    pub wet_oflo: f32,
    pub wet_stor: f32,
}

impl HruWb {
    /// Column names in `hru_wb_day.txt` and their values.
    pub fn columns(&self) -> [(&'static str, f32); 32] {
        [
            ("precip", self.precip),
            ("snofall", self.snofall),
            ("snomlt", self.snomlt),
            ("surq_gen", self.surq_gen),
            ("latq", self.latq),
            ("wateryld", self.wateryld),
            ("perc", self.perc),
            ("et", self.et),
            ("ecanopy", self.ecanopy),
            ("eplant", self.eplant),
            ("esoil", self.esoil),
            ("surq_cont", self.surq_cont),
            ("cn", self.cn),
            ("sw_init", self.sw_init),
            ("sw_final", self.sw_final),
            ("sw_ave", self.sw),
            ("sw_300", self.sw_300),
            ("sno_init", self.sno_init),
            ("sno_final", self.sno_final),
            ("snopack", self.snopack),
            ("pet", self.pet),
            ("qtile", self.qtile),
            ("irr", self.irr),
            ("satex", self.satex),
            ("satex_chan", self.satex_chan),
            ("sw_change", self.delsw),
            ("lagsurf", self.lagsurf),
            ("laglatq", self.laglatq),
            ("lagsatex", self.lagsatex),
            ("wet_evap", self.wet_evap),
            ("wet_oflo", self.wet_oflo),
            ("wet_stor", self.wet_stor),
        ]
    }
}

/// Basin settings the daily HRU routines read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HruBasin {
    pub pet: i32,
    pub cn_froz: f32,
    pub cswat: i32,
    /// Biological mixing, done daily when carbon is on (`cswat` 2).
    pub biomix: Option<Biomix>,
    pub plant: PlantBasin,
    pub nut: NutBasin,
    pub carbon: Option<carbon::CarbonBasin>,
    /// Correct SWAT+'s bugs instead of reproducing them (`RunOptions::fixes`).
    pub fixes: bool,
}

/// Biological mixing settings: the `biomix` row of `tillage.til` and the temperature factor of
/// `carbon.bsn`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biomix {
    /// `bmix_eff`, `bmix_depth`.
    pub eff: f32,
    pub depth: f32,
    /// `org_con%tmpf`, `tn`, `top`, `tx`.
    pub tmpf: i32,
    pub tn: f32,
    pub top: f32,
    pub tx: f32,
}

impl HruBasin {
    pub fn new(project: &Project) -> Self {
        Self {
            pet: project.codes.pet,
            cn_froz: project.params.get("cn_froz").unwrap_or(0.000862),
            cswat: project.codes.cswat,
            biomix: if project.codes.cswat == 2 { Some(Biomix::new(project)) } else { None },
            plant: PlantBasin::new(project),
            nut: NutBasin::new(project),
            carbon: if project.codes.cswat == 2 { Some(carbon::CarbonBasin::new(project)) } else { None },
            fixes: false,
        }
    }

    /// The same settings, correcting SWAT+'s bugs when `fixes` is set.
    pub fn with_fixes(mut self, fixes: bool) -> Self {
        self.fixes = fixes;
        self.plant.fixes = fixes;
        self.nut.fixes = fixes;
        self
    }
}

impl Biomix {
    /// `till_parm_read` (the last row named `biomix`; 0.2 and 50 mm when there is none, nothing
    /// without a tillage file) and `carbon_bsn_read`.
    pub fn new(project: &Project) -> Self {
        let (eff, depth) = match project.tillage.as_ref() {
            None => (0.0, 0.0),
            Some(t) => {
                let mut v = (0.2, 50.0);
                for r in 0..t.rows.len() {
                    if t.text(r, "tillnm").is_some_and(|n| swat_model::table::fortran_eq(n, "biomix")) {
                        v = (t.real(r, "effmix").unwrap_or(0.0), t.real(r, "deptil").unwrap_or(0.0));
                    }
                }
                v
            }
        };
        let values = project.carbon.as_ref().and_then(|t| t.rows.first()).cloned().unwrap_or_else(|| default_row(schema::CARBON_BSN));
        let c = Row { values, fields: schema::CARBON_BSN };
        let tmpf = c.get("tmpf", 0).and_then(Value::as_int).unwrap_or(2);
        Self { eff, depth, tmpf, tn: c.real("tn"), top: c.real("top"), tx: c.real("tx") }
    }

    /// SWAT+ `fcgd`: the temperature factor.
    fn fcgd(&self, xx: f32) -> f32 {
        let (tn, top, tx) = (self.tn, self.top, self.tx);
        if xx <= tn || xx >= tx {
            return 0.0;
        }
        let qq = (tn - top) / (top - tx);
        let f = ((xx - tn).powf(qq)) * (tx - xx) / (((top - tn).powf(qq)) * (tx - top));
        if f < 0.0 { 0.0 } else { f }
    }

    fn temperature_factor(&self, emix: f32, stemp: f32) -> f32 {
        if self.tmpf == 3 { emix * (0.9 * (stemp / (stemp + (9.93_f32 - 0.312 * stemp).exp())) + 0.1) } else { emix * self.fcgd(stemp) }
    }
}

/// A row of a table found by name, or the type's defaults when the name isn't there (SWAT+ then
/// uses element 0 of the array, which keeps the defaults).
struct Row {
    values: Vec<Value>,
    fields: &'static [Field],
}

impl Row {
    /// For the data `hru-data.hru` names (`hru_databases`): a name not found, the Editor's `null`
    /// among them, leaves the pointer at its default, 1, so the HRU gets the table's first row.
    /// `false` only when the table is empty or missing.
    fn find_or_first(table: Option<&Table>, fields: &'static [Field], name: &str) -> (Self, bool) {
        let (row, found) = Self::find(table, fields, name);
        if found {
            return (row, true);
        }
        match table.and_then(|t| t.rows.first().map(|r| (t, r))) {
            Some((t, r)) => (Self { values: r.clone(), fields: t.fields }, true),
            None => (row, false),
        }
    }

    fn find(table: Option<&Table>, fields: &'static [Field], name: &str) -> (Self, bool) {
        if let Some(t) = table
            && let Some(r) = t.find(name)
            && let Some(values) = t.rows.get(r)
        {
            return (Self { values: values.clone(), fields: t.fields }, true);
        }
        (Self { values: default_row(fields), fields }, false)
    }

    fn get(&self, field: &str, index: usize) -> Option<&Value> {
        let mut at = 0;
        for fd in self.fields {
            if fd.name == field {
                return self.values.get(at + index);
            }
            at += fd.count;
        }
        None
    }

    fn real(&self, field: &str) -> f32 {
        self.get(field, 0).and_then(Value::as_real).unwrap_or(0.0)
    }

    fn text(&self, field: &str) -> &str {
        self.get(field, 0).and_then(Value::as_text).unwrap_or("")
    }
}

fn is_null(s: &str) -> bool {
    s.trim_end() == "null"
}

/// Builds the HRUs as `proc_hru` leaves them (`hrudb_init`, `topohyd_init`, `soils_init`,
/// `soil_phys_init`, `plant_init`, `cn2_init`, `hydro_init`) and `time_conc_init` completes them.
///
/// `wgn` gives each HRU's generator (0-based) and `wgns` the generators as `cli_initwgn` left
/// them. Returns `Err` with the reason when an input needs a part of SWAT+ not ported yet.
pub fn init_hrus(
    project: &Project,
    wgn: &[usize],
    wgns: &[crate::climate::WgnState],
    step: i32,
    ndays: &[i32; 13],
) -> Result<Vec<Hru>, String> {
    let bmix = Biomix::new(project);
    let codes = &project.codes;
    if codes.cswat != 0 && codes.cswat != 2 {
        return Err(format!("carbon model {}", codes.cswat));
    }
    if codes.sol_p_model != 0 {
        return Err("soil phosphorus model 1 (nut_pminrl2)".into());
    }
    if step > 1 {
        return Err("sub-daily time steps".into());
    }
    if !(0..=2).contains(&codes.pet) {
        return Err(format!("potential ET method {} (read from a file)", codes.pet));
    }
    if codes.gampt != 0 {
        return Err("Green and Ampt infiltration".into());
    }
    if codes.crk != 0 {
        return Err("crack flow".into());
    }
    if codes.gwflow != 0 {
        return Err("gwflow".into());
    }
    if project.objects.iter().any(|o| o.outflows.iter().any(|f| f.obj_type == "hru")) {
        return Err("water routed into HRUs".into());
    }

    let ffcb = project.params.get("ffcb").unwrap_or(0.0);
    let surlag = project.params.get("surlag").unwrap_or(4.0);
    let hru_objs: Vec<_> = project.objects_of(swat_model::ObjectKind::Hru).collect();
    let mut out = Vec::with_capacity(project.hru_data.len());
    for (i, hd) in project.hru_data.iter().enumerate() {
        let ob = hru_objs.get(i).ok_or_else(|| format!("HRU {} has no object in hru.con", i + 1))?;
        let g = wgn.get(i).and_then(|&k| wgns.get(k)).ok_or_else(|| format!("HRU {}: no weather generator", i + 1))?;
        let mut h = init_hru(project, hd, ob, g, ffcb, ndays)?;
        h.wgn = wgn.get(i).copied().unwrap_or(0);
        init_bmix(&mut h, &bmix);
        time_conc(&mut h, surlag);
        out.push(h);
    }
    // et_act takes the wetland's evaporation coefficient from `wet_hyd(ihyd)`: the HRU numbered
    // as its hydrology row, which has the type's default (0.7) unless it has a wetland too.
    let evrsv: Vec<Option<f32>> = out.iter().map(|h| h.wet.as_ref().map(|w| w.evrsv)).collect();
    for h in out.iter_mut() {
        let row = h.wet_hyd_row;
        if let Some(w) = h.wet.as_mut() {
            w.evrsv_et = evrsv.get(row.wrapping_sub(1)).copied().flatten().unwrap_or(0.7);
        }
    }
    cal_conditions(project, &mut out)?;
    Ok(out)
}

/// SWAT+ `chg_par`: parameter `cur` changed by calibration update `u`, within the limits of
/// `cal_parms.cal`'s `p`.
pub(crate) fn chg_par(u: &swat_model::cal::CalUpdate, p: &swat_model::cal::CalParm, cur: f32) -> f32 {
    let v = match u.chg_typ.as_str() {
        "absval" => u.val,
        "abschg" => cur + u.val,
        "pctchg" => (1.0 + u.val / 100.0) * cur,
        _ => (1.0 + u.val) * cur,
    };
    v.max(p.absmin).min(p.absmax)
}

/// The `cal_parms.cal` row of a calibration update (defaults when it isn't there).
pub(crate) fn cal_parm(project: &Project, u: &swat_model::cal::CalUpdate) -> swat_model::cal::CalParm {
    project.cal_parms.iter().find(|p| swat_model::table::fortran_eq(&p.name, &u.name)).cloned().unwrap_or_default()
}

/// SWAT+ `cal_conditions` (from `proc_cal`, after the HRUs are set up): the changes of
/// `calibration.cal` without conditions, of HRU, soil and basin parameters (the aquifers' are
/// made where the aquifers are set up).
fn cal_conditions(project: &Project, hrus: &mut [Hru]) -> Result<(), String> {
    for u in &project.cal_upd {
        let p = cal_parm(project, u);
        if !u.conds.is_empty() {
            return Err(format!("calibration of {} with conditions ({})", u.name, u.conds.join(", ")));
        }
        if !matches!(p.ob_typ.as_str(), "hru" | "sol" | "bsn" | "aqu") {
            return Err(format!("calibration of {} ({} parameter)", u.name, p.ob_typ));
        }
        if !matches!(u.chg_typ.as_str(), "absval" | "abschg" | "pctchg" | "relchg") {
            return Err(format!("calibration of {}: change {}", u.name, u.chg_typ));
        }
        let chg = |cur: f32| chg_par(u, &p, cur);
        if p.ob_typ == "aqu" {
            if !matches!(u.name.as_str(), "alpha" | "revap_co") {
                return Err(format!("calibration of {}", u.name));
            }
            continue;
        }
        if p.ob_typ == "bsn" {
            match u.name.as_str() {
                "surlag" => {
                    // Once (one element), for all HRUs.
                    let surlag = chg(project.params.get("surlag").unwrap_or(4.0));
                    for h in hrus.iter_mut() {
                        h.brt = 1.0 - (-surlag / h.tconc).exp();
                    }
                }
                other => return Err(format!("calibration of {other}")),
            }
            continue;
        }
        let elems: Vec<usize> = if u.elems.is_empty() { (1..=hrus.len()).collect() } else { u.elems.clone() };
        for e in elems {
            let h = hrus.get_mut(e.wrapping_sub(1)).ok_or_else(|| format!("calibration of {}: HRU {e}", u.name))?;
            match u.name.as_str() {
                // Not for tile-drained HRUs.
                "perco" | "cn3_swf" if h.tile.is_some() => {}
                "perco" => {
                    h.perco = chg(h.perco);
                    h.perco_lim = if h.perco > 1.0e-6 {
                        let perc_ln_func = 1.0052_f32 * (-(h.perco - 1.0e-6).ln()).ln() + 5.6862;
                        (-perc_ln_func).exp().min(1.0)
                    } else {
                        0.0
                    };
                }
                "cn2" => {
                    h.cn2 = chg(h.cn2);
                    curno(h);
                }
                "cn3_swf" => {
                    h.cn3_swf = chg(h.cn3_swf);
                    curno(h);
                }
                "petco" => h.pet_co = chg(h.pet_co),
                "latq_co" => h.latq_co = chg(h.latq_co),
                "canmx" => h.canmx = chg(h.canmx),
                "esco" => h.esco = chg(h.esco),
                "lat_len" => h.lat_len = chg(h.lat_len),
                "epco" => {
                    h.epco = chg(h.epco);
                    let epco = h.epco;
                    for pl in h.plants.iter_mut() {
                        pl.epco = epco;
                    }
                }
                "awc" => {
                    let nly = h.layers.len() as i32;
                    let lyr1 = u.lyr1.max(1);
                    let lyr2 = if u.lyr2 <= 0 { nly } else { u.lyr2.min(nly) };
                    for ly in lyr1..=lyr2 {
                        let l = &mut h.layers[(ly - 1) as usize];
                        l.awc = chg(l.awc);
                        soil_awc_init(h);
                        curno(h);
                    }
                }
                other => return Err(format!("calibration of {other}")),
            }
        }
    }
    Ok(())
}

/// SWAT+ `soil_awc_init`, again after a calibration of the available water: the layers' water
/// contents from it, their water back to the initial share of field capacity.
fn soil_awc_init(h: &mut Hru) {
    for l in h.layers.iter_mut() {
        if l.awc <= 1.0e-6 {
            l.awc = 0.005;
        }
        if l.awc >= 0.8 {
            l.awc = 0.8;
        }
        l.wp = 0.4 * l.clay * l.bd / 100.0;
        if l.wp <= 0.0 {
            l.wp = 0.005;
        }
        l.up = l.wp + l.awc;
        l.por = 1.0 - l.bd / 2.65;
        if l.up >= l.por {
            l.up = l.por - 0.05;
            l.wp = l.up - l.awc;
            if l.wp <= 0.0 {
                l.up = l.por * 0.75;
                l.wp = l.por * 0.25;
            }
        }
    }
    let mut depth_prev = 0.0_f32;
    let mut sumpor = 0.0_f32;
    h.sumfc = 0.0;
    h.sumul = 0.0;
    h.sw = 0.0;
    let ffc = h.ffc;
    for l in h.layers.iter_mut() {
        l.thick = l.d - depth_prev;
        let pormm = l.por * l.thick;
        sumpor += pormm;
        l.ul = (l.por - l.wp) * l.thick;
        h.sumul += l.ul;
        l.fc = l.thick * (l.up - l.wp);
        h.sumfc += l.fc;
        l.st = l.fc * ffc;
        l.hk = (l.ul - l.fc) / l.k;
        if l.hk < 1.0 {
            l.hk = 1.0;
        }
        h.sw += l.st;
        l.wpmm = l.wp * l.thick;
        depth_prev = l.d;
    }
    let dmax = h.layers.last().map(|l| l.d).unwrap_or(0.0);
    let avpor = sumpor / dmax;
    h.avbd = 2.65 * (1.0 - avpor);
}

fn init_hru(
    project: &Project,
    hd: &HruData,
    ob: &swat_model::SpatialObject,
    g: &crate::climate::WgnState,
    ffcb: f32,
    ndays: &[i32; 13],
) -> Result<Hru, String> {
    let n = &hd.name;
    let (lu, lu_found) = Row::find_or_first(project.landuse.as_ref(), schema::LANDUSE, &hd.land_use_mgt);
    for f in ["septic", "fstrip", "grassww", "bmpuser"] {
        let v = lu.text(f);
        if lu_found && !is_null(v) && !v.is_empty() {
            return Err(format!("{n}: land use {} has {f} {v}", hd.land_use_mgt));
        }
    }

    let mut h = Hru {
        area_ha: ob.area_ha,
        km: ob.area_ha / 100.0,
        elev: ob.elev,
        lat: ob.lat,
        days_plant: -366,
        days_harv: -366,
        days_kill: -366,
        days_irr: -366,
        ..Hru::default()
    };

    // wet_initial
    if !is_null(&hd.surf_stor) && !hd.surf_stor.is_empty() {
        if project.codes.cswat == 2 {
            return Err(format!("{n}: wetland with the carbon model"));
        }
        let (w, row) = wetland::Wetland::new(project, &hd.surf_stor, h.area_ha).map_err(|e| format!("{n}: {e}"))?;
        h.wet = Some(w);
        h.wet_hyd_row = row;
    }

    // plant_init: the urban land type by name (none when not in urban.urb).
    if lu_found
        && let Some(t) = project.urban.as_ref()
        && let Some(r) = t.find(lu.text("urb_lu"))
    {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        let ro = match lu.text("urb_ro") {
            "buildup_washoff" => UrbanRunoff::BuildupWashoff,
            "usgs_reg" => return Err(format!("{n}: urban loads by USGS regression (usgs_reg)")),
            _ => UrbanRunoff::None,
        };
        h.urban = Some(Urban {
            fimp: x("fimp"),
            fcimp: x("fcimp"),
            curbden: x("curbden"),
            urbcoef: x("urbcoef"),
            dirtmx: x("dirtmx"),
            thalf: x("thalf"),
            tnconc: x("tnconc"),
            tpconc: x("tpconc"),
            tno3conc: x("tno3conc"),
            ro,
        });
    }

    // topohyd_init
    let (topo, _) = Row::find_or_first(project.topography.as_ref(), schema::TOPOGRAPHY, &hd.topo);
    let (hyd, _) = Row::find_or_first(project.hydrology.as_ref(), schema::HYDROLOGY, &hd.hyd);
    let (sno, _) = Row::find_or_first(project.snow.as_ref(), schema::SNOW, &hd.snow);
    h.slope = topo.real("slope");
    h.slope_len = topo.real("slope_len");
    h.lat_len = topo.real("lat_len");
    if h.lat_len < 1.0e-6 {
        h.lat_len = 50.0;
    }
    h.lat_ttime = hyd.real("lat_ttime");
    h.canmx = hyd.real("canmx");
    h.erorgn = hyd.real("erorgn");
    h.erorgp = hyd.real("erorgp");
    h.hyd_biomix = hyd.real("biomix");
    h.esco = hyd.real("esco");
    h.epco = hyd.real("epco");
    h.cn3_swf = hyd.real("cn3_swf");
    if hyd.real("lat_sed") > 0.0 {
        return Err(format!("{n}: sediment in lateral flow (lat_sed)"));
    }
    let mut perco = hyd.real("perco");
    // topohyd_init: a tile-drained HRU's curve number and percolation are set for tiles.
    let tile_name = lu.text("tiledrain").to_string();
    if lu_found && !is_null(&tile_name) && !tile_name.is_empty() {
        let t = project.tiledrain.as_ref().and_then(|t| t.find(&tile_name).map(|r| (t, r)));
        let Some((t, r)) = t else {
            return Err(format!("{n}: tile drain {tile_name} not in tiledrain.str"));
        };
        if project.codes.tdrn == 1 {
            return Err(format!("{n}: tile drains by the DRAINMOD equations"));
        }
        if project.codes.wtdn == 1 {
            return Err(format!("{n}: tile drains with the water table of the soil moisture (wtable 1)"));
        }
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        h.tile = Some(Tile { depth: x("depth"), time: x("time"), drain_co: x("drain_co"), ldrain: 0, lag: x("lag"), ttime: 0.0 });
        h.cn3_swf = 0.95;
        perco = 0.1;
    }
    h.perco = perco;
    h.sno = Snow {
        falltmp: sno.real("falltmp"),
        melttmp: sno.real("melttmp"),
        meltmx: sno.real("meltmx"),
        meltmn: sno.real("meltmn"),
        timp: sno.real("timp"),
        covmx: sno.real("covmx"),
        cov50: sno.real("cov50"),
        init_mm: sno.real("init_mm"),
    };
    (h.snocov1, h.snocov2) = ascrv(0.5, 0.95, h.sno.cov50, 0.95);
    h.perco_lim = if perco > 1.0e-9 {
        let perc_ln_func = 1.0052_f32 * (-(perco - 1.0e-6).ln()).ln() + 5.6862;
        (-perc_ln_func).exp().min(1.0)
    } else {
        0.0
    };
    h.latq_co = hyd.real("latq_co");
    h.pet_co = hyd.real("pet_co");
    if h.pet_co < 1.0e-6 {
        h.pet_co = 1.0;
    } else if h.pet_co < 0.01 {
        h.pet_co /= 0.0023;
    }
    h.sno_mm = h.sno.init_mm;

    // soils_init, soil_phys_init
    let soil = project
        .soils
        .iter()
        .find(|s| swat_model::table::fortran_eq(&s.name, &hd.soil))
        .ok_or_else(|| format!("{n}: soil {} not found", hd.soil))?;
    soil_layers(&mut h, soil, project.soil_lyr_depths.as_deref())?;

    // plant_init: the community named by the land use (none when the name isn't in plant.ini).
    let com = if lu_found && !is_null(lu.text("plant_cov")) {
        project.plant_communities.iter().find(|c| swat_model::table::fortran_eq(&c.name, lu.text("plant_cov")))
    } else {
        None
    };
    if let Some(com) = com
        && !com.plants.is_empty()
    {
        if com.plants.len() > 20 {
            return Err(format!("{n}: plant community {} of {} plants (SWAT+ keeps 20)", com.name, com.plants.len()));
        }
        if project.codes.cswat == 1 {
            return Err(format!("{n}: plants with carbon model 1"));
        }
        if project.co2_yr_dat {
            return Err("co2_yr.dat".into());
        }
        let t = project.plants.as_ref().ok_or_else(|| format!("{n}: plants, but no plants.plt"))?;
        let months: [(f32, f32); 12] = std::array::from_fn(|m| (g.m[m].tmp_max_ave, g.m[m].tmp_min_ave));
        let layers: Vec<(f32, f32)> = h.layers.iter().map(|l| (l.d, l.thick)).collect();
        h.laimx_sum = 0.0;
        h.rsd_tot = OrgMass::default();
        for init in &com.plants {
            // A name not in plants.plt keeps the default pointer, 1: the first plant.
            let r = t.find(&init.cpnm).unwrap_or(0);
            let db = PlantDb::new(t, r, project.params.get("rsdco").unwrap_or(0.05));
            let pl = plant::plant_init(db, init, &months, ndays, g.lat, soil.zmx, &layers, hyd.real("epco"))?;
            h.rsd_tot = h.rsd_tot.add(pl.rsd);
            h.laimx_sum += pl.db.blai;
            h.plants.push(pl);
        }
    }

    // landuse_read and plant_init: the schedule named by the land use (none when the name isn't
    // in management.sch), and its first operation.
    let mgt_name = lu.text("mgt_ops");
    if lu_found
        && !is_null(mgt_name)
        && !mgt_name.is_empty()
        && let Some(s) = project.management.iter().find(|s| swat_model::table::fortran_eq(&s.name, mgt_name))
    {
        // plant_init sets the rotation year in its loop over the plants.
        let rot = com.filter(|c| !c.plants.is_empty()).map(|c| c.rot_yr_ini).unwrap_or(1);
        let mut m = mgt::Mgt::new(project, s, ndays, rot, project.time.normalized().day_start)?;
        // Without plants, plant_init leaves the HRU's first operation at its default, 1.
        if com.is_none_or(|c| c.plants.is_empty()) {
            m.cur_op = 0;
        }
        if !m.ops.is_empty() || !m.dtbls.is_empty() {
            h.mgt = Some(m);
        }
    }

    // plant_init: USLE P and LS (the slope length capped by the practice isn't used).
    let (cp, _) = Row::find(project.cons_practice.as_ref(), schema::CONS_PRACTICE, lu.text("cons_prac"));
    let xm = 0.6 * (1.0 - (-35.835_f32 * h.slope).exp());
    // gfortran folds Sin(Atan(slope)) into slope / sqrt(slope * slope + 1), which rounds
    // differently from sinf(atanf(slope)) on about two slopes in five.
    let sin_sl = h.slope / (h.slope * h.slope + 1.0).sqrt();
    let usle_ls = (h.slope_len / 22.128).powf(xm) * (65.41 * sin_sl * sin_sl + 4.56 * sin_sl + 0.065);
    let usle_p = cp.real("pfac");

    // plant_init: overland n from the land use's name in ovn_table.lum (0.05 when not found).
    h.ovn = 0.05;
    if let Some(t) = project.ovn_table.as_ref()
        && let Some(r) = t.find(lu.text("ovn"))
    {
        h.ovn = t.real(r, "ovn").unwrap_or(h.ovn);
    }

    // cn2_init: the curve number of the soil's hydrologic group (none outside A–D). A name not
    // in cntable.lum points at element 0, which keeps the type's defaults.
    let (cn, cn_found) = Row::find(project.cntable.as_ref(), schema::CNTABLE, lu.text("cn_lu"));
    let cn_of = |k: usize| {
        if cn_found { cn.get("cn", k).and_then(Value::as_real).unwrap_or(0.0) } else { [30.0, 55.0, 70.0, 77.0][k] }
    };
    h.cn2 = match h.hydgrp.as_str() {
        "A" => cn_of(0),
        "B" => cn_of(1),
        "C" => cn_of(2),
        "D" => cn_of(3),
        _ => 0.0,
    };
    curno(&mut h);

    // hydro_init
    let rock = (-0.053_f32 * h.layers.first().map(|l| l.rock).unwrap_or(0.0)).exp();
    h.usle_mult = rock * h.layers.first().map(|l| l.usle_k).unwrap_or(0.0) * usle_p * usle_ls * 11.8;
    let tsoil = (g.m[11].tmp_max_ave + g.m[11].tmp_max_ave) / 2.0;
    let sffc = if ffcb <= 0.0 { g.pcp_an / (g.pcp_an + (9.043_f32 - 0.002135 * g.pcp_an).exp()) } else { ffcb };
    h.sw = 0.0;
    h.ffc = sffc;
    for l in h.layers.iter_mut() {
        l.tmp = tsoil;
        l.st = sffc * l.fc;
        h.sw += l.st;
    }
    // Dormancy on the day hydro_init runs: day 0 (time%day isn't set yet).
    if !h.plants.is_empty() {
        let sd = (0.4_f32 * ((0.0_f32 - 82.0) / 58.09).sin()).asin();
        let sdlat = -g.latsin * sd.tan() / g.latcos;
        let hlat = if sdlat > 1.0 {
            0.0
        } else if sdlat >= -1.0 {
            sdlat.acos()
        } else {
            3.1416
        };
        let daylength = 7.6394 * hlat;
        for pl in h.plants.iter_mut() {
            pl.idorm = pl.gro && daylength - g.daylth < g.daylmn;
        }
    }
    h.zmx = h.layers.last().map(|l| l.d).unwrap_or(0.0);
    // structure_set_parms: the layer the tiles are in.
    let depths: Vec<f32> = h.layers.iter().map(|l| l.d).collect();
    if let Some(t) = h.tile.as_mut() {
        t.ldrain = if t.depth > 0.0 { depths.iter().position(|&d| t.depth < d).map_or(0, |k| k + 1) } else { 0 };
        t.ttime = if t.ldrain > 0 && t.lag > 0.01 { 1.0 - (-24.0 / t.lag).exp() } else { 0.0 };
    }
    h.lat_ttime = if h.lat_ttime <= 0.0 {
        let mut scmx = 0.0_f32;
        for l in &h.layers {
            if l.k > scmx {
                scmx = l.k;
            }
        }
        let mut xx = 10.4 * h.lat_len / scmx;
        if xx < 1.0 {
            xx = 1.0;
        }
        1.0 - (-1.0_f32 / xx).exp()
    } else {
        1.0 - (-1.0_f32 / h.lat_ttime).exp()
    };

    // soil_nutcarb_init (after proc_hru, from main): the soil-plant entry's nutrients.
    let spi = project
        .soil_plant_init
        .iter()
        .find(|s| swat_model::table::fortran_eq(&s.name, &hd.soil_plant_init))
        .or(project.soil_plant_init.first());
    let mut test = SoilTest::default();
    if let (Some(spi), Some(t)) = (spi, project.soil_test.as_ref())
        && let Some(r) = t.find(&spi.nutc)
    {
        let x = |f: &str| t.real(r, f).unwrap_or(0.0);
        test = SoilTest {
            exp_co: if x("exp_co") > 0.005 { 0.001 } else { x("exp_co") },
            lab_p: x("lab_p"),
            nitrate: x("nitrate"),
            fr_hum_act: x("fr_hum_act"),
            hum_c_n: x("hum_c_n"),
            hum_c_p: x("hum_c_p"),
        };
    }
    h.nutcarb_init(&test, project.params.get("psp").unwrap_or(0.4), project.codes.cswat);
    if project.codes.cswat == 2 {
        h.carbon_init(&carbon::CarbonBasin::new(project));
    }
    Ok(h)
}

/// `soils_init` with custom layer depths (`soil_lyr_depths.sol`): the profile is cut into 1 mm
/// slices that take their file layer's properties, and each custom layer averages its slices.
/// The depths are read up to the first one at or past the profile's bottom (which becomes the
/// last layer's) or to the end of the list (where the profile is cut short); a 10 mm top layer
/// comes first, and depths of 10 mm or less are skipped.
fn custom_layers(soil: &Soil, depths: &[i32]) -> Vec<Layer> {
    let db = &soil.layers;
    let Some(last) = db.last() else { return Vec::new() };
    let tot0 = last.z as i32;
    // The 1 mm slices: the index of the file layer each takes.
    let mut cell = vec![0usize; tot0.max(0) as usize + 1];
    let mut prev = 0_i32;
    for (j, l) in db.iter().enumerate() {
        let mut i = prev + 1;
        while i <= tot0 {
            if i as f32 <= l.z && i > prev {
                cell[i as usize] = j;
            } else {
                break;
            }
            i += 1;
        }
        prev = l.z as i32;
    }
    // Counting pass.
    let mut mlyr = 0;
    let mut csld = 0_i32;
    for &v in depths {
        csld = v;
        if csld <= tot0 {
            if csld > 10 {
                mlyr += 1;
                if csld == tot0 {
                    break;
                }
            }
        } else {
            csld = tot0;
            mlyr += 1;
            break;
        }
    }
    mlyr += 1;
    let tot = tot0.min(csld);
    // Layers: each read skips depths of 10 mm or less; the first is read again after the 10 mm layer.
    let mut out = Vec::with_capacity(mlyr);
    let mut p = 0usize;
    let mut pcd = 1_i32;
    for i in 0..mlyr {
        while let Some(&v) = depths.get(p) {
            csld = v;
            p += 1;
            if csld > 10 {
                break;
            }
        }
        let d;
        if i == 0 {
            d = 10.0;
            csld = 10;
            p = p.saturating_sub(1);
        } else {
            if csld > tot {
                csld = tot;
            }
            d = csld as f32;
        }
        let mut sum = [0.0_f32; 10];
        let mut n = 0;
        for jj in pcd..=csld {
            let l = &db[cell.get(jj.max(0) as usize).copied().unwrap_or(0)];
            for (s, v) in sum.iter_mut().zip([l.bd, l.awc, l.k, l.cbn, l.clay, l.silt, l.sand, l.rock, l.alb, l.usle_k]) {
                *s += v;
            }
            n += 1;
        }
        let nf = n as f32;
        out.push(Layer {
            d,
            bd: sum[0] / nf,
            awc: sum[1] / nf,
            k: sum[2] / nf,
            cbn: sum[3] / nf,
            clay: sum[4] / nf,
            silt: sum[5] / nf,
            sand: sum[6] / nf,
            rock: sum[7] / nf,
            alb: sum[8] / nf,
            usle_k: sum[9] / nf,
            ..Layer::default()
        });
        pcd = csld + 1;
    }
    out
}

/// `soils_init` and `soil_phys_init` for one HRU's soil; `depths` are the custom layer depths
/// when the project has them.
fn soil_layers(h: &mut Hru, soil: &Soil, depths: Option<&[i32]>) -> Result<(), String> {
    let db = &soil.layers;
    let first = db.first().ok_or_else(|| format!("soil {} has no layers", soil.name))?;
    let from = |l: &swat_model::SoilLayer, d: f32| Layer {
        d,
        bd: l.bd,
        awc: l.awc,
        k: l.k,
        clay: l.clay,
        silt: l.silt,
        sand: l.sand,
        rock: l.rock,
        alb: l.alb,
        cbn: l.cbn,
        usle_k: l.usle_k,
        ..Layer::default()
    };
    let mut layers = vec![from(first, 10.0)];
    if let Some(dep) = depths {
        layers = custom_layers(soil, dep);
    } else if first.z > 19.5 {
        // A 10 mm top layer is added; layer j takes the properties of the file's layer j - 1.
        for l in db {
            layers.push(from(l, l.z));
        }
    } else {
        for l in db.iter().skip(1) {
            layers.push(from(l, l.z));
        }
    }
    h.hydgrp = soil.hydgrp.clone();
    h.anion_excl = soil.anion_excl;
    if h.anion_excl <= 1.0e-6 {
        h.anion_excl = 0.5;
    }
    if h.anion_excl >= 1.0 {
        h.anion_excl = 0.99;
    }

    for l in layers.iter_mut() {
        if l.k <= 0.0 {
            l.k = match soil.hydgrp.as_str() {
                "A" => 50.0,
                "B" => 20.0,
                "C" => 5.0,
                "D" => 2.0,
                _ => 10.0,
            };
        }
        if l.bd <= 1.0e-6 {
            l.bd = 1.3;
        }
        if l.bd > 2.0 {
            l.bd = 2.0;
        }
        if l.awc <= 1.0e-6 {
            l.awc = 0.005;
        }
        if l.awc >= 0.8 {
            l.awc = 0.8;
        }
        if l.rock > 98.0 {
            l.rock = 98.0;
        }
    }
    for l in layers.iter_mut() {
        l.wp = 0.4 * l.clay * l.bd / 100.0;
        if l.wp <= 0.0 {
            l.wp = 0.005;
        }
        l.up = l.wp + l.awc;
        l.por = 1.0 - l.bd / 2.65;
        if l.up >= l.por {
            l.up = l.por - 0.05;
            l.wp = l.up - l.awc;
            if l.wp <= 0.0 {
                l.up = l.por * 0.75;
                l.wp = l.por * 0.25;
            }
        }
    }
    let mut depth_prev = 0.0_f32;
    let mut sumpor = 0.0_f32;
    h.sumul = 0.0;
    h.sumfc = 0.0;
    for l in layers.iter_mut() {
        l.thick = l.d - depth_prev;
        let pormm = l.por * l.thick;
        sumpor += pormm;
        l.ul = (l.por - l.wp) * l.thick;
        h.sumul += l.ul;
        l.fc = l.thick * (l.up - l.wp);
        h.sumfc += l.fc;
        l.hk = (l.ul - l.fc) / l.k;
        if l.hk < 1.0 {
            l.hk = 1.0;
        }
        l.wpmm = l.wp * l.thick;
        depth_prev = l.d;
    }
    let dmax = layers.last().map(|l| l.d).unwrap_or(0.0);
    let avpor = sumpor / dmax;
    h.avbd = 2.65 * (1.0 - avpor);
    h.layers = layers;
    Ok(())
}

/// `soil_nutcarb_init`: each layer's share of the biological mixing, by how much of it lies above
/// the mixing depth.
fn init_bmix(h: &mut Hru, b: &Biomix) {
    for ly in 0..h.layers.len() {
        let prev = if ly == 0 { 0.0 } else { h.layers[ly - 1].d };
        let l = &mut h.layers[ly];
        l.init_bmix = if b.depth >= l.d {
            b.eff
        } else if b.depth > prev && b.depth <= l.d {
            b.eff * (b.depth - prev) / l.thick
        } else {
            0.0
        };
    }
}

/// SWAT+ `ascrv`: the two shape coefficients of an S curve through (x3, x1) and (x4, x2).
pub fn ascrv(x1: f32, x2: f32, x3: f32, x4: f32) -> (f32, f32) {
    let xx = (x3 / x1 - x3).ln();
    let x6 = (xx - (x4 / x2 - x4).ln()) / (x4 - x3);
    let x5 = xx + x3 * x6;
    (x5, x6)
}

/// SWAT+ `curno`: retention parameters from the curve number at moisture condition II.
fn curno(h: &mut Hru) {
    let cnn = h.cn2;
    let c2 = 100.0 - cnn;
    let mut cn1 = cnn - 20.0 * c2 / (c2 + (2.533_f32 - 0.0636 * c2).exp());
    cn1 = cn1.max(0.4 * cnn);
    let cn3 = cnn * (0.006729_f32 * c2).exp();
    h.smx = 254.0 * (100.0 / cn1 - 1.0);
    let s3 = 254.0 * (100.0 / cn3 - 1.0);
    let rto3 = 1.0 - s3 / h.smx;
    let rtos = 1.0 - 2.54 / h.smx;
    let sumul = h.sumul;
    let mut sumfc = h.sumfc + h.cn3_swf * (h.sumul - h.sumfc);
    if sumfc < 0.0 {
        sumfc = 10.0;
    }
    sumfc = sumfc.max(0.05);
    sumfc = (sumul - 0.05).min(sumfc);
    let (w1, w2) = ascrv(rto3, rtos, sumfc, sumul);
    h.wrt = [w1, w2];
}

/// SWAT+ `time_conc_init` for an HRU: time of concentration and the runoff lag fraction.
fn time_conc(h: &mut Hru, surlag: f32) {
    let t_ov = 0.0556_f32 * (h.slope_len * h.ovn).powf(0.6) / (h.slope + 0.0001).powf(0.3);
    let ch_slope = 0.5 * h.slope;
    let ch_n = h.ovn;
    let ch_l = 0.5 * (h.area_ha / 2.0).sqrt();
    let t_ch = 0.31 * ch_l * ch_n.powf(0.75) / (h.km.powf(0.125) * (ch_slope + 0.001).powf(0.375));
    let tconc = t_ov + t_ch;
    h.tconc = tconc;
    h.brt = 1.0 - (-surlag / tconc).exp();
}

impl Hru {
    /// `basin_sw_init`: soil water and snow at the start of the printed years.
    pub fn start_balance(&mut self) {
        self.sw_init = self.sw;
        self.sno_init = self.sno_mm;
    }

    /// One day of `hru_control`: the HRU's weather `w` (lapse rates applied), the day's context
    /// `c`, basin settings `b`, and the values the Fortran keeps between calls (`saved`).
    pub fn day(&mut self, w: &HruWeather, c: &DayCtx, b: &HruBasin, saved: &mut Saved) -> HruDay {
        // time_control zeroes the irrigation before the day's objects; varinit the fertilizer.
        self.irr_applied = 0.0;
        self.irr_runoff = 0.0;
        self.fertn = 0.0;
        self.fertp = 0.0;
        self.water_evap = 0.0;
        // A tillage's effect lasts `till_eff_days` (carbon model).
        if let Some(cb) = &b.carbon
            && self.till_switch
        {
            if self.till_days >= cb.till_eff_days {
                self.till_switch = false;
                self.till_days = 0;
                for l in self.layers.iter_mut() {
                    l.tillagef_tillmix = 0.0;
                }
            } else {
                self.till_days += 1;
            }
        }
        // The schedule's decision tables, on yesterday's stresses; then a day more since the
        // last planting, harvest, kill and irrigation.
        self.dtbl_day(c, w.phubase0, b, saved);
        self.days_plant += 1;
        self.days_harv += 1;
        self.days_kill += 1;
        self.days_irr += 1;
        let mut sw_volume_begin = 0.0_f32;
        for l in &self.layers {
            sw_volume_begin += l.st;
        }
        let mut precip_eff = w.precip;

        // varinit
        for l in self.layers.iter_mut() {
            l.prk = 0.0;
            l.flat = 0.0;
        }

        if w.tave > 0.0 && c.phutot > 0.01 {
            self.phubase += w.tave / c.phutot;
        }
        for pl in self.plants.iter_mut() {
            pl.strsw = 1.0;
            pl.strst = 1.0;
            pl.strsn = 1.0;
            pl.strsp = 1.0;
            pl.strsa = 1.0;
            pl.strss = 1.0;
        }

        // albedo, with yesterday's leaf area and cover
        let alb1 = self.layers.first().map(|l| l.alb).unwrap_or(0.0);
        let cover = self.ab_gr_com.m + self.rsd_tot.m;
        let eaj = (-5.0e-5_f32 * (cover + 0.1)).exp();
        let albday = if self.sno_mm <= 0.5 { if self.lai_sum > 0.0 { 0.23 * (1.0 - eaj) + alb1 * eaj } else { alb1 } } else { 0.8 };

        self.soil_temperature(w, albday, c.tmp_an);

        // sq_canopyint
        if !(self.lai_sum < 0.001 || self.laimx_sum < 0.001) {
            let canmxl = self.canmx * self.lai_sum / self.laimx_sum;
            if precip_eff < canmxl - self.canstor {
                self.canstor += precip_eff;
                precip_eff = 0.0;
            } else {
                precip_eff -= canmxl - self.canstor;
                self.canstor = canmxl;
            }
        }

        // sq_snom
        let mut snofall = 0.0_f32;
        let mut snomlt;
        let s = self.sno;
        let snotmp = 0.0_f32 * (1.0 - s.timp) + w.tave * s.timp;
        if w.tave <= s.falltmp {
            self.sno_mm += precip_eff;
            snofall = precip_eff;
            precip_eff = 0.0;
        }
        if w.tmax > s.melttmp && self.sno_mm > 0.0 {
            let smfac = (s.meltmx + s.meltmn) / 2.0 + ((c.day - 81) as f32 / 58.09).sin() * (s.meltmx - s.meltmn) / 2.0;
            snomlt = smfac * (((snotmp + w.tmax) / 2.0) - s.melttmp);
            let snocov = if self.sno_mm < s.covmx {
                let rto_sno = self.sno_mm / s.covmx;
                rto_sno / (rto_sno + (self.snocov1 - self.snocov2 * rto_sno).exp())
            } else {
                1.0
            };
            snomlt *= snocov;
            if snomlt < 0.0 {
                snomlt = 0.0;
            }
            if snomlt > self.sno_mm {
                snomlt = self.sno_mm;
            }
            self.sno_mm -= snomlt;
            precip_eff += snomlt;
            if precip_eff < 0.0 {
                precip_eff = 0.0;
            }
        } else {
            snomlt = 0.0;
        }

        // et_pot, and the vapour pressure deficit
        let ea = crate::climate::ee(w.tave);
        let ed = ea * w.rhum;
        let vpd = ea - ed;
        let (pet_raw, ep_pm) = match b.pet {
            0 => (self.pet_priestley_taylor(w, ea, ed), 0.0),
            1 => self.pet_penman_monteith(w, albday, ea, ed, vpd, b),
            _ => (pet_hargreaves(w), 0.0),
        };
        let pet_day = self.pet_co * pet_raw;

        // et_act
        let (canev, es_day, ep_max) = self.evaporation(w, pet_day, ep_pm, b.fixes);

        // mgt_operatn
        if self.mgt.as_ref().is_some_and(|m| !m.yr_skip) {
            self.mgt_operatn(c.mo, c.day_mo, b);
        }

        // surface: daily curve number, runoff when there is enough water, erosion. Not for a
        // wetland: its runoff is what it releases, and its curve number is never set.
        let is_wet = self.wet.is_some();
        let cnday = if is_wet { 0.0 } else { self.daily_cn(b.cn_froz) };
        let mut surfq = 0.0_f32;
        if precip_eff > 0.1 && !is_wet {
            let r2 = 25400.0 / cnday - 254.0;
            let bb = 0.2 * r2;
            let pb = precip_eff - bb;
            if pb > 0.0 {
                surfq = pb * pb / (precip_eff + 0.8 * r2);
            }
            // Directly connected impervious area: curve number 98 (`urbcn2` is not used).
            if let Some(u) = &self.urban {
                let mut surfqimp = 0.0_f32;
                let cnimp = 98.0_f32;
                let r2 = 25400.0 / cnimp - 254.0;
                let bb = 0.2 * r2;
                let pb = precip_eff - bb;
                if pb > 0.0 {
                    surfqimp = pb * pb / (precip_eff + 0.8 * r2);
                }
                surfq = surfq * (1.0 - u.fcimp) + surfqimp * u.fcimp;
            }
        }
        // varinit zeroes the peak rate, so a wetland (which doesn't compute it) has none, and
        // its loads are what it releases.
        let mut nd = NutDay::default();
        let inflpcp;
        if is_wet {
            // wetland_control
            surfq = self.wetland_control(w.precip, c.mo, c.station_tave, &b.nut, &mut nd);
            inflpcp = self.wet.as_ref().map_or(0.0, |wt| wt.water_seep).max(0.0);
        } else {
            surfq += self.irr_runoff;
            self.erosion(surfq.max(0.0), surfq, w.precip, c.precip_half_hr, &b.nut, &mut nd);
            inflpcp = (precip_eff - surfq).max(0.0);
        }

        // swr_percmain (a wetland's irrigation went into the pond)
        let irr_in = if is_wet { 0.0 } else { self.irr_applied };
        let (latq_gen, sepbtm) = self.percolate(inflpcp + irr_in, &mut surfq);
        // The wetland's release (and saturation excess below a low weir), as runoff.
        let wet_outflow = self.wet.as_ref().map_or(0.0, |wt| if wt.out.flo > 0.0 { wt.out.flo / self.area_ha / 10.0 } else { 0.0 });

        if b.cswat == 0 {
            self.rsd_decomp(&b.nut);
            self.nut_nminrl(&b.nut, &mut nd);
        }
        // With the carbon model: biological mixing (water, nutrients and pools of the top layers),
        // residue decomposition and the pool transformations.
        if let (Some(bm), Some(cb)) = (&b.biomix, &b.carbon) {
            if bm.eff > 1.0e-6 {
                self.biomix(bm, cb);
            }
            self.cbn_surfrsd_decomp(cb);
            self.cbn_rsd_transfer();
            nd.denit = self.cbn_zhang2(cb, bm);
        }
        self.nut_nitvol();
        self.nut_pminrl(&b.nut, &mut nd);

        // pl_community and pl_grow
        let mut pd = PlantDay { ep_max, pet_day, vpd, ..PlantDay::default() };
        self.pl_community(&mut pd, w.solrad, &b.plant);
        let pw = PlantWeather {
            tave: w.tave,
            tmin: w.tmin,
            solrad: w.solrad,
            station_tave: c.station_tave,
            daylength: c.daylength,
            daylmn: c.daylmn,
            dormhr: c.dormhr,
            tmp_an: c.tmp_an,
            end_yr: c.end_yr,
        };
        self.pl_grow(&mut pd, &pw, &b.plant, saved);

        // Stress of the plants growing (as 1 - stress), averaged; the salt stress is summed only.
        let mut strs = [0.0_f32; 6];
        let mut npl_gro = 0;
        for pl in &self.plants {
            if pl.gro && !pl.idorm && pl.phuacc <= 1.0 {
                npl_gro += 1;
                let v = [1.0 - pl.strsw, 1.0 - pl.strsa, 1.0 - pl.strst, 1.0 - pl.strsn, 1.0 - pl.strsp, 1.0 - pl.strss];
                for k in 0..6 {
                    strs[k] += v[k];
                }
            }
        }
        if npl_gro > 0 {
            for v in strs.iter_mut().take(5) {
                *v /= npl_gro as f32;
            }
        }

        // sw_300
        self.sw_300 = 0.0;
        for ly in 0..self.layers.len() {
            let dep = if ly == 0 { 0.0 } else { self.layers[ly - 1].d };
            let l = &self.layers[ly];
            if l.d >= 300.0 {
                self.sw_300 += l.st * (300.0 - dep) / l.thick;
                break;
            }
            self.sw_300 += l.st;
        }

        let etday = pd.ep_day + es_day + canev;

        // Nutrients that leave with sediment, then nitrate and soluble P.
        if surfq > 0.0 && nd.qp_cms > 1.0e-6 && precip_eff > 0.0 {
            let (erorgn, erorgp) = (self.erorgn, self.erorgp);
            self.sediment_nutrients(surfq, erorgn, erorgp, b.cswat == 0, b.fixes, &mut nd, saved);
            if let Some(cb) = &b.carbon {
                nd.sedorgn = self.nut_orgnc2(surfq, nd.sedyld, nd.enratio, cb, saved);
            }
        }
        // nut_nrain: nitrate and ammonium in rain and dry deposition, into the top layer.
        // Past the deposition data nothing is added, and the table repeats the last day's
        // (`hnb_d` isn't zeroed).
        if let Some([no3_rf, no3_dry, nh4_rf, nh4_dry]) = w.atmo {
            nd.no3atmo = 0.01 * no3_rf * w.precip + no3_dry;
            if let Some(l) = self.layers.first_mut() {
                l.no3 += nd.no3atmo;
            }
            nd.nh4atmo = 0.01 * nh4_rf * w.precip + nh4_dry;
            if let Some(l) = self.layers.first_mut() {
                l.nh4 += nd.nh4atmo;
            }
            self.atmo_prev = [nd.no3atmo, nd.nh4atmo];
        } else {
            [nd.no3atmo, nd.nh4atmo] = self.atmo_prev;
        }
        self.nut_nlch(surfq, &b.nut, &mut nd);
        self.nut_solp(surfq, &b.nut, &mut nd);

        // hru_urban: loads washed off the impervious area.
        let silyld = self.hru_urban(surfq, c.precip_half_hr, &mut nd);

        // stor_surfstor: sediment and its nutrients lag like runoff.
        let brt = self.brt;
        let mut lagged = [nd.sedyld, nd.sedorgn, nd.sedorgp, nd.surqno3, nd.surqsolp, nd.sedminpa, nd.sedminps];
        for (k, v) in lagged.iter_mut().enumerate() {
            self.surf_bs_sed[k] = (self.surf_bs_sed[k] + *v).max(1.0e-9);
            *v = self.surf_bs_sed[k] * brt;
            self.surf_bs_sed[k] -= *v;
        }
        // The sediment released is taken from its store twice (once after computing it, once
        // with the other constituents), so the store goes negative after a sediment day.
        if !b.fixes {
            self.surf_bs_sed[0] -= lagged[0];
        }
        // The detached sediment classes come only from lateral flow (`swr_latsed`, none so far):
        // their stores start at 1e-6 and release that share every day.
        let mut cls = [0.0_f32; 5];
        for (k, v) in cls.iter_mut().enumerate() {
            let add = if k == 1 { silyld } else { 0.0 };
            self.surf_bs_cls[k] = (self.surf_bs_cls[k] + add).max(1.0e-6);
            *v = self.surf_bs_cls[k] * brt;
        }
        for (k, v) in cls.iter().enumerate() {
            self.surf_bs_cls[k] -= *v;
        }

        // swr_substor
        self.bss += latq_gen;
        self.bss_no3 += nd.latno3;
        if self.bss < 1.0e-6 {
            self.bss = 0.0;
        }
        if self.bss_no3 < 1.0e-6 {
            self.bss_no3 = 0.0;
        }
        let mut latq = self.bss * self.lat_ttime;
        let mut latno3 = self.bss_no3 * self.lat_ttime;
        if latq < 1.0e-6 {
            latq = 0.0;
        }
        if latno3 < 1.0e-6 {
            latno3 = 0.0;
        }
        self.bss -= latq;
        self.bss_no3 -= latno3;
        // The tile flow and its nitrate are lagged the same way, by the tiles' lag.
        let (mut qtile, mut tileno3) = (0.0_f32, 0.0_f32);
        if let Some(t) = self.tile {
            self.bss_tile += self.qtile;
            self.bss_tno3 += nd.tileno3;
            if self.bss_tile < 1.0e-6 {
                self.bss_tile = 0.0;
            }
            if self.bss_tno3 < 1.0e-6 {
                self.bss_tno3 = 0.0;
            }
            qtile = self.bss_tile * t.ttime;
            tileno3 = self.bss_tno3 * t.ttime;
            if qtile < 1.0e-6 {
                qtile = 0.0;
            }
            if tileno3 < 1.0e-6 {
                tileno3 = 0.0;
            }
            self.bss_tile -= qtile;
            self.bss_tno3 -= tileno3;
        }
        nd.tileno3 = tileno3;

        // sq_surfst
        self.surf_bs = (self.surf_bs + surfq).max(1.0e-6);
        let qday = self.surf_bs * self.brt;
        self.surf_bs -= qday;

        let mut qdr = qday + latq + qtile;
        if qdr < 0.0 {
            qdr = 0.0;
        }

        // swr_subwq: chlorophyll-a, CBOD and dissolved oxygen in the runoff.
        let (mut chl_a, mut cbodu, mut doxq) = (0.0_f32, 0.0_f32, 0.0_f32);
        if qdr > 1.0e-4 {
            let mut wtmp = 5.0 + 0.75 * w.tave;
            if wtmp <= 0.1 {
                wtmp = 0.1;
            }
            wtmp += 273.15;
            let tp = 100.0 * (lagged[1] + lagged[3]) / qdr;
            chl_a = 0.1 * tp;
            // The enrichment ratio is the last any HRU computed (`varinit` zeroes a local of the
            // same name); corrected, this HRU's own today.
            let enratio = if b.fixes { nd.enratio } else { saved.enratio };
            let org_c = (self.layers[0].cbn / 100.0) * enratio * lagged[0] * 1000.0;
            cbodu = 2.7 * org_c / (qdr * self.km) / 10000.0;
            let ww = -139.3441_f32 + (1.575701e05 / wtmp);
            let w2 = wtmp * wtmp;
            let xx = 6.642308e07 / w2;
            let yy = 1.2438e10 / (w2 * wtmp);
            let zz = 8.621949e11 / (w2 * w2);
            let mut soxy = (ww - xx + yy - zz).exp();
            if soxy < 0.0 {
                soxy = 0.0;
            }
            doxq = soxy * (-0.1 * cbodu).exp();
            if doxq < 0.0 {
                doxq = 0.0;
            }
            if doxq > soxy {
                doxq = soxy;
            }
        }

        // hru_hyds
        let hd = {
            use crate::hyd::Hyd;
            let cnv_m3 = self.area_ha * 10.0;
            let cnv_kg = self.area_ha;
            let mut sur = Hyd {
                temp: 5.0 + 0.75 * w.tave,
                flo: qday * cnv_m3,
                sed: lagged[0],
                orgn: lagged[1] * cnv_kg,
                sedp: (lagged[2] + lagged[5] + lagged[6]) * cnv_kg,
                no3: lagged[3] * cnv_kg,
                solp: (lagged[4] + nd.tilelabp) * cnv_kg,
                chla: chl_a * cnv_kg,
                san: cls[0],
                sil: cls[1],
                cla: cls[2],
                sag: cls[3],
                lag: cls[4],
                ..Hyd::default()
            };
            let cnv_ppm = if sur.flo > 0.01 { 1.0 / (1000.0 * sur.flo) } else { 0.0 };
            sur.cbod = cbodu * cnv_ppm;
            sur.dox = doxq * cnv_ppm;
            let rhg = Hyd { flo: sepbtm * cnv_m3, no3: nd.percn * cnv_kg, ..Hyd::default() };
            let lat = Hyd { flo: latq * cnv_m3, no3: latno3 * cnv_kg, ..Hyd::default() };
            let til = Hyd { flo: qtile * cnv_m3, no3: nd.tileno3 * cnv_kg, ..Hyd::default() };
            let tot = Hyd::default().plus(sur).plus(lat).plus(til);
            [tot, rhg, sur, lat, til]
        };

        let wb = HruWb {
            precip: w.precip,
            snofall,
            snomlt,
            surq_gen: qday,
            latq,
            wateryld: qdr,
            perc: sepbtm,
            et: etday,
            ecanopy: canev,
            eplant: pd.ep_day,
            esoil: es_day,
            surq_cont: surfq,
            cn: cnday,
            sw: self.sw,
            sw_final: self.sw,
            sw_300: self.sw_300,
            snopack: self.sno_mm,
            pet: pet_day,
            irr: self.irr_applied,
            delsw: self.sw - sw_volume_begin,
            lagsurf: self.surf_bs,
            laglatq: self.bss,
            wet_evap: self.water_evap,
            wet_oflo: wet_outflow,
            wet_stor: self.wet.as_ref().map_or(0.0, |wt| wt.water.flo / (10.0 * self.area_ha)),
            qtile,
            ..HruWb::default()
        };
        let pw = HruPw {
            lai: self.lai_sum,
            bioms: self.tot_com.m,
            yield_: self.pl_yield.m,
            residue: self.rsd_tot.m,
            sol_tmp: self.layers.get(1).map(|l| l.tmp).unwrap_or(0.0),
            strsw: strs[0],
            strsa: strs[1],
            strstmp: strs[2],
            strsn: strs[3],
            strsp: strs[4],
            strss: strs[5],
            nplnt: self.tot_com.n,
            percn: nd.percn,
            pplnt: self.tot_com.p,
            phubase0: self.phubase,
            lai_max: 0.0,
            bm_max: self.tot_com.m,
            bm_grow: pd.bm_grow,
            c_gro: pd.c_gro,
        };
        self.pl_yield = plant::OrgMass::default();
        if self.tot_com.m < 0.0 {
            self.tot_com.m = 0.0;
        }
        let nb = HruNb {
            lab_min_p: nd.lab_min_p,
            act_sta_p: nd.act_sta_p,
            fertn: self.fertn,
            fertp: self.fertp,
            fixn: pd.fixn,
            denit: nd.denit,
            act_nit_n: nd.act_nit_n,
            act_sta_n: nd.act_sta_n,
            org_lab_p: nd.org_lab_p,
            rsd_nitorg_n: 0.0,
            rsd_laborg_p: 0.0,
            no3atmo: nd.no3atmo,
            nh4atmo: nd.nh4atmo,
            nuptake: pd.nuptake,
            puptake: pd.puptake,
        };
        let ls = HruLs {
            sedyld: lagged[0] / self.area_ha,
            sedorgn: lagged[1],
            sedorgp: lagged[2],
            surqno3: lagged[3],
            latno3,
            surqsolp: lagged[4],
            usle: nd.usle,
            sedminp: lagged[5] + lagged[6],
            lchlabp: nd.lchlabp,
            tileno3: nd.tileno3,
            tilelabp: nd.tilelabp,
            percn: nd.percn,
            ..HruLs::default()
        };
        HruDay { wb, pw, nb, ls, nutrients: b.cswat == 0 || b.cswat == 2, hd, harvests: std::mem::take(&mut self.harvests) }
    }

    /// `time_control` at the end of a year: perennials age, the original carbon model's soil is
    /// mixed (`mgt_newtillmix_cswat0`), and in the northern hemisphere the base heat units reset.
    pub fn end_year(&mut self, cswat: i32, fixes: bool) {
        // The yearly irrigation total's reset is commented out in SWAT+; corrected, it restarts.
        if fixes {
            self.irr_yr = 0.0;
        }
        if cswat == 0 && self.hyd_biomix > 1.0e-6 {
            self.year_end_mixing(self.hyd_biomix);
        }
        for pl in self.plants.iter_mut() {
            if pl.db.perennial() {
                pl.curyr_mat = (pl.curyr_mat + 1).min(pl.db.mat_yrs);
            }
        }
        if self.lat >= 0.0 {
            self.phubase = 0.0;
            self.mgt_new_year();
        }
    }

    /// `time_control` on day 181: base heat units reset in the southern hemisphere.
    pub fn midyear(&mut self) {
        if self.lat < 0.0 {
            self.phubase = 0.0;
            self.mgt_new_year();
        }
    }

    /// `hru_output`'s daily part: the period's start and end and their averages. When the day is
    /// printed the next period starts from its end.
    pub fn close_day(&mut self, wb: &mut HruWb, printed: bool) {
        wb.sw_init = self.sw_init;
        wb.sno_init = self.sno_init;
        wb.sw_final = self.sw;
        wb.sw = (wb.sw_init + wb.sw_final) / 2.0;
        wb.sno_final = self.sno_mm;
        wb.snopack = (wb.sno_init + wb.sno_final) / 2.0;
        if printed {
            self.sw_init = wb.sw_final;
            self.sno_init = wb.sno_final;
        }
    }

    /// SWAT+ `mgt_biomix` without tillage, its soil water part: the thawed layers above the mixing
    /// depth exchange a share of their water. The total isn't summed again afterwards.
    fn biomix(&mut self, b: &Biomix, cb: &carbon::CarbonBasin) {
        let nly = self.layers.len();
        let Some(last) = self.layers.last() else { return };
        let mut dtil = last.d.min(b.depth);
        if dtil <= 1.0e-6 {
            return;
        }
        if dtil < 10.0 {
            dtil = 11.0;
        }
        let d_prev = |layers: &[Layer], l: usize| if l == 0 { 0.0 } else { layers[l - 1].d };
        let mut emix_sum = 0.0_f32;
        let mut lyr_exit = nly;
        let mut stemp = 0.0_f32;
        for l in 0..nly {
            // After a tillage the mixing stops, then comes back as the soil consolidates.
            let (till_days, till_switch) = (self.till_days, self.till_switch);
            let ly = &mut self.layers[l];
            let bmix = if till_days > 0 {
                let consf = if ly.st >= ly.fc { cb.bio_consf } else { cb.bio_consf * ly.st / ly.fc };
                ly.bmix += consf * ly.init_bmix;
                if ly.bmix > ly.init_bmix {
                    ly.bmix = ly.init_bmix;
                }
                ly.bmix
            } else if till_switch && till_days == 0 {
                ly.bmix = 0.0;
                0.0
            } else {
                ly.bmix = ly.init_bmix;
                ly.bmix
            };
            let ly = self.layers[l];
            if ly.tmp > 1.0e-6 {
                if ly.d <= dtil {
                    stemp = ly.tmp;
                    let emix = b.temperature_factor(bmix, stemp);
                    emix_sum += emix * ly.thick;
                } else if ly.d > dtil && d_prev(&self.layers, l) < dtil {
                    // The layer's own temperature isn't taken: `stemp` is the layer above's.
                    let emix = b.temperature_factor(bmix, stemp);
                    emix_sum += emix * (dtil - d_prev(&self.layers, l));
                } else {
                    lyr_exit = l;
                    break;
                }
            } else {
                self.layers[l].bmix = 0.0;
                dtil = d_prev(&self.layers, l);
                lyr_exit = l;
                break;
            }
        }
        if dtil <= 1.0e-6 {
            return;
        }
        let avg_emix = emix_sum / dtil;
        let mut sol_mass = vec![0.0_f32; nly];
        let mut sol_msn = vec![0.0_f32; nly];
        let mut frac_dep = vec![0.0_f32; nly];
        // The mass of every layer is taken with the first layer's bulk density.
        let bd1 = self.layers[0].bd;
        for l in 0..lyr_exit {
            let ly = &self.layers[l];
            sol_mass[l] = (ly.thick / 1000.0) * 10000.0 * bd1 * 1000.0 * (1.0 - ly.rock / 100.0);
        }
        let mut mix_sw = 0.0_f32;
        let mut frac_mixed_v = Vec::with_capacity(lyr_exit);
        for l in 0..lyr_exit {
            let ly = &self.layers[l];
            let prev = d_prev(&self.layers, l);
            let sol_msm;
            if ly.d <= dtil {
                sol_msm = avg_emix * sol_mass[l];
                sol_msn[l] = sol_mass[l] - sol_msm;
                frac_dep[l] = ly.thick / dtil;
            } else if ly.d > dtil && prev < dtil {
                sol_msm = avg_emix * sol_mass[l] * (dtil - prev) / ly.thick;
                sol_msn[l] = sol_mass[l] - sol_msm;
                frac_dep[l] = (dtil - prev) / dtil;
            } else {
                sol_msn[l] = sol_mass[l];
                frac_dep[l] = 0.0;
                break;
            }
            let frac_mixed = sol_msm / sol_mass[l];
            mix_sw += frac_mixed * ly.st;
            frac_mixed_v.push(frac_mixed);
        }
        let frac_non_mixed: Vec<f32> = (0..nly).map(|l| sol_msn[l] / sol_mass[l]).collect();
        for l in 0..lyr_exit {
            let ly = &mut self.layers[l];
            ly.st = frac_non_mixed[l] * ly.st + frac_dep[l] * mix_sw;
        }
        self.biomix_pools(&frac_mixed_v, &frac_non_mixed, &frac_dep, &sol_mass, avg_emix, lyr_exit, dtil, cb);
    }

    /// SWAT+ `stmp_solt`: daily soil temperature by layer.
    fn soil_temperature(&mut self, w: &HruWeather, albday: f32, tmp_an: f32) {
        let tlag = 0.8_f32;
        let avbd = self.avbd;
        let mut f = avbd / (avbd + 686.0 * (-5.63_f32 * avbd).exp());
        let dp = 1000.0 + 2500.0 * f;
        let ww = 0.356 - 0.144 * avbd;
        let dnly = self.layers.last().map(|l| l.d).unwrap_or(0.0);
        let wc = self.sw / (ww * dnly);
        let b = (500.0 / dp).ln();
        let q = (1.0 - wc) / (1.0 + wc);
        f = (b * (q * q)).exp();
        let dd = f * dp;
        let cover = self.ab_gr_com.m + self.rsd_tot.m;
        let mut bcv = cover / (cover + (7.563_f32 - 1.297e-4 * cover).exp());
        if self.sno_mm != 0.0 {
            let xx = if self.sno_mm <= 120.0 { self.sno_mm / (self.sno_mm + (6.055_f32 - 0.3002 * self.sno_mm).exp()) } else { 1.0 };
            bcv = xx.max(bcv);
        }
        let st0 = (w.solrad * (1.0 - albday) - 14.0) / 20.0;
        let tbare = w.tave + 0.5 * (w.tmax - w.tmin) * st0;
        let tmp2 = self.layers.get(1).map(|l| l.tmp).unwrap_or(0.0);
        let tcov = bcv * tmp2 + (1.0 - bcv) * tbare;
        let tmp_srf = 0.5 * (tbare + tcov);
        let mut xx = 0.0_f32;
        for l in self.layers.iter_mut() {
            let mut zd = (xx + l.d) / 2.0;
            zd /= dd;
            let df = zd / (zd + (-0.8669_f32 - 2.0775 * zd).exp());
            l.tmp = tlag * l.tmp + (1.0 - tlag) * (df * (tmp_an - tmp_srf) + tmp_srf);
            xx = l.d;
        }
    }

    /// SWAT+ `et_act` without ponded water: canopy evaporation, sublimation, soil evaporation
    /// and the nitrate it draws up from the second layer. Returns (canopy evaporation, soil
    /// evaporation, maximum transpiration).
    /// `et_act`; `ep_in` is the maximum transpiration `et_pot` left (Penman-Monteith's, else
    /// varinit's 0), kept when no PET is left after the canopy.
    fn evaporation(&mut self, w: &HruWeather, pet_day: f32, ep_in: f32, fixes: bool) -> (f32, f32, f32) {
        let esd = 500.0_f32;
        let etco = 0.80_f32;
        let effnup = 0.05_f32;
        let mut pet = pet_day;
        let canev;
        let mut es_day = 0.0_f32;
        let mut ep_max = ep_in;
        pet -= self.canstor;
        if pet < 0.0 {
            self.canstor = -pet;
            canev = pet_day;
            pet = 0.0;
            ep_max = 0.0;
        } else {
            canev = self.canstor;
            self.canstor = 0.0;
        }
        if pet > 1.0e-6 {
            ep_max = if self.lai_sum <= 3.0 { self.lai_sum * pet / 3.0 } else { pet };
            if ep_max < 0.0 {
                ep_max = 0.0;
            }
            let cej = -5.0e-5_f32;
            let cover = self.ab_gr_com.m + self.rsd_tot.m;
            let eaj = if self.sno_mm >= 0.5 { 0.5 } else { (cej * (cover + 0.1)).exp() };
            let mut es_max = pet * eaj;
            let mut eos1 = pet / (es_max + ep_max + 1.0e-10);
            eos1 *= es_max;
            es_max = es_max.min(eos1);
            es_max = es_max.max(0.0);
            let ponded = self.wet.as_ref().filter(|wt| wt.water.flo > 0.0).map(|wt| if fixes { wt.evrsv } else { wt.evrsv_et });
            if let Some(evrsv) = ponded {
                // Open water evaporates, less under the canopy (Sakaguchi et al. 2014); the
                // plants' and the soil's together may then exceed the PET.
                es_max = if self.lai_sum <= 4.0 { evrsv * (1.0 - self.lai_sum / 4.0) * pet } else { 0.0 };
            } else if pet_day < es_max + ep_max {
                es_max = pet_day - ep_max;
                if pet < es_max + ep_max {
                    es_max = pet * es_max / (es_max + ep_max);
                    // Uses the es_max just changed, as the Fortran does.
                    ep_max = pet * ep_max / (es_max + ep_max);
                }
            }
            let mut esleft = es_max;
            if w.tave > 0.0 {
                if self.sno_mm >= esleft {
                    self.sno_mm -= esleft;
                    esleft = 0.0;
                } else {
                    esleft -= self.sno_mm;
                    self.sno_mm = 0.0;
                }
            }
            // Evaporation from the ponded water before the soil.
            let area = self.area_ha;
            if let Some(wt) = self.wet.as_mut()
                && wt.water.flo > 0.0
            {
                let mut wetvol_mm = wt.water.flo / (10.0 * area);
                let evap;
                if wetvol_mm >= esleft {
                    wetvol_mm -= esleft;
                    evap = esleft * (10.0 * area);
                    esleft = 0.0;
                } else {
                    esleft -= wetvol_mm;
                    evap = wetvol_mm * (10.0 * area);
                    wetvol_mm = 0.0;
                }
                wt.water.flo = 10.0 * wetvol_mm * area;
                self.water_evap = evap / (10.0 * area);
            }
            let soil_evap = self.wet.as_ref().is_none_or(|wt| wt.water.flo < 1.0e-6);
            let mut evzp = 0.0_f32;
            let eosl = esleft;
            let mut sev = 0.0_f32;
            for ly in (0..self.layers.len()).filter(|_| soil_evap) {
                let dep = if ly == 0 { self.layers[0].d } else { self.layers[ly - 1].d };
                if dep < esd {
                    let esco = self.esco;
                    let l = &mut self.layers[ly];
                    let evz = eosl * l.d / (l.d + (2.374_f32 - 0.00713 * l.d).exp());
                    sev = evz - evzp * (1.0 - esco);
                    evzp = evz;
                    if l.st < l.fc {
                        let xx = 2.5 * (l.st - l.fc) / l.fc;
                        sev *= xx.exp();
                    }
                    sev = sev.min(l.st * etco);
                    if sev < 0.0 {
                        sev = 0.0;
                    }
                    if sev > esleft {
                        sev = esleft;
                    }
                    if l.st > sev {
                        esleft = (esleft - sev).max(0.0);
                        l.st = (l.st - sev).max(1.0e-6);
                    } else {
                        esleft = (esleft - l.st).max(0.0);
                        sev = l.st;
                        l.st = 0.0;
                    }
                }
                if ly == 1 {
                    let st2 = self.layers[1].st;
                    let mut sev_st = if st2 > 1.0e-3 { sev / st2 } else { 0.0 };
                    sev_st = sev_st.min(1.0);
                    let mut no3up = effnup * sev_st * self.layers[1].no3;
                    no3up = no3up.min(self.layers[1].no3);
                    self.layers[1].no3 = (self.layers[1].no3 - no3up).max(0.0001);
                    self.layers[0].no3 = (self.layers[0].no3 + no3up).max(0.0001);
                }
            }
            es_day = (es_max - esleft).max(0.0);
        }
        self.sum_sw();
        (canev, es_day, ep_max)
    }

    fn sum_sw(&mut self) {
        self.sw = 0.0;
        for l in &self.layers {
            self.sw += l.st;
        }
    }

    /// SWAT+ `sq_dailycn`: today's curve number from soil water and frozen soil.
    fn daily_cn(&self, cn_froz: f32) -> f32 {
        let mut sw_fac = self.wrt[0] - self.wrt[1] * self.sw;
        if sw_fac < -20.0 {
            sw_fac = -20.0;
        }
        if sw_fac > 20.0 {
            sw_fac = 20.0;
        }
        let mut r2 = if (self.sw + sw_fac.exp()) > 0.001 { self.smx * (1.0 - self.sw / (self.sw + sw_fac.exp())) } else { self.smx };
        if self.layers.get(1).map(|l| l.tmp).unwrap_or(0.0) <= 0.0 {
            r2 = self.smx * (1.0 - (-cn_froz * r2).exp());
        }
        r2 = r2.max(3.0);
        25400.0 / (r2 + 254.0)
    }

    /// SWAT+ `swr_percmain` (with `swr_percmicro` and `swr_satexcess`) without tiles, cracks or
    /// septic systems. Saturation excess joins `surfq`. Returns (lateral flow, percolation out of
    /// the profile).
    fn percolate(&mut self, inflpcp: f32, surfq: &mut f32) -> (f32, f32) {
        let nly = self.layers.len();
        let mut sepday = inflpcp;
        let slug = 1000.0_f32;
        let mut sep_left = sepday;
        let mut latq = 0.0_f32;
        let mut sepbtm = 0.0_f32;
        loop {
            sepday = sep_left.min(slug);
            sep_left -= sepday;
            sep_left = sep_left.max(0.0);
            for j1 in 0..nly {
                self.layers[j1].st += sepday;
                let sw_excess = self.layers[j1].st - self.layers[j1].fc;
                sepday = 0.0;
                let mut latlyr = 0.0_f32;
                let lyrtile = 0.0_f32;
                if sw_excess > 1.0e-5 {
                    (sepday, latlyr) = self.percmicro(j1, sw_excess);
                    let l = &mut self.layers[j1];
                    l.st = l.st - sepday - latlyr - lyrtile;
                    l.st = l.st.max(1.0e-6);
                }
                if j1 == nly - 1 {
                    sepbtm += sepday;
                }
                latq += latlyr;
                let l = &mut self.layers[j1];
                l.flat = latlyr + lyrtile;
                l.prk += sepday;
                if latq < 1.0e-6 {
                    latq = 0.0;
                }
                if l.flat < 1.0e-6 {
                    l.flat = 0.0;
                }
            }
            if sep_left <= 0.0 {
                break;
            }
        }
        self.sat_excess(surfq);
        self.sum_sw();
        self.qtile = self.tile_flow();
        (latq, sepbtm)
    }

    /// The end of `swr_percmain`: the shallow water table (`wtable` 0) and, above the tiles,
    /// tile flow (`swr_origtile`) taken from the layers' water above field capacity.
    fn tile_flow(&mut self) -> f32 {
        let Some(t) = self.tile else { return 0.0 };
        let mut qtile = 0.0_f32;
        let mut wt_shall = self.zmx;
        if self.layers.get(1).is_some_and(|l| l.tmp > 0.0) {
            let por_air = 0.9_f32;
            let d = self.zmx - t.depth;
            if self.sw > self.sumfc {
                let mut yy = self.sumul * por_air;
                if yy < 1.1 * self.sumfc {
                    yy = 1.1 * self.sumfc;
                }
                let mut xx = (self.sw - self.sumfc) / (yy - self.sumfc);
                if xx > 1.0 {
                    xx = 1.0;
                }
                wt_shall = xx * self.zmx;
            }
            if t.depth > 0.0 {
                if wt_shall <= d {
                    qtile = 0.0;
                } else {
                    // swr_origtile
                    if self.sw > self.sumfc {
                        let sw_excess = (wt_shall - d) / wt_shall * (self.sw - self.sumfc);
                        qtile = if t.time < 1.0 { sw_excess } else { sw_excess * (1.0 - (-24.0 / t.time).exp()) };
                        qtile = qtile.min(t.drain_co);
                    } else {
                        qtile = 0.0;
                    }
                    if qtile < 0.0 {
                        qtile = 0.0;
                    }
                }
            }
        }
        if qtile > 0.0 {
            let mut sumqtile = qtile;
            for l in self.layers.iter_mut() {
                let xx = l.st - l.fc;
                if xx > 0.0 {
                    if xx > sumqtile {
                        l.st -= sumqtile;
                        sumqtile = 0.0;
                    } else {
                        sumqtile -= xx;
                        l.st = l.fc;
                    }
                }
            }
            if sumqtile > 0.0 {
                qtile -= sumqtile;
                qtile = qtile.max(0.0);
            }
        }
        self.sum_sw();
        qtile
    }

    /// SWAT+ `swr_percmicro` for layer `ly1` (0-based): returns (percolation, lateral flow).
    fn percmicro(&mut self, ly1: usize, sw_excess: f32) -> (f32, f32) {
        let nly = self.layers.len();
        let (latq_co, slope, lat_len, perco_lim) = (self.latq_co, self.slope, self.lat_len, self.perco_lim);
        let l = &mut self.layers[ly1];
        if l.tmp <= 0.0 {
            return (0.0, 0.0);
        }
        let ho = if l.ul - l.fc <= 0.0 { 0.0 } else { 2.0 * sw_excess / ((l.ul - l.fc) / l.thick) };
        let mut latlyr = if ly1 == 0 { 0.0 } else { latq_co * ho * l.k * slope / lat_len * 0.024 };
        if latlyr < 0.0 {
            latlyr = 0.0;
        }
        if latlyr > sw_excess {
            latlyr = sw_excess;
        }
        l.hk = (l.ul - l.fc) / l.k;
        l.hk = l.hk.max(2.0);
        let mut sepday = (l.st - l.fc) * (1.0 - (-24.0_f32 / l.hk).exp());
        sepday = sepday.max(0.0);
        if ly1 == nly - 1 {
            sepday *= perco_lim;
        }
        if sepday + latlyr > sw_excess {
            let ratio = sepday / (latlyr + sepday);
            sepday = sw_excess * ratio;
            latlyr = sw_excess * (1.0 - ratio);
        }
        let lyrtile = 0.0_f32;
        if sepday + lyrtile > sw_excess {
            sepday = sw_excess - lyrtile;
        }
        (sepday, latlyr)
    }

    /// SWAT+ `swr_satexcess` without surface storage: water above saturation moves up, and what
    /// the top layer can't hold runs off.
    fn sat_excess(&mut self, surfq: &mut f32) {
        let nn = self.layers.len();
        for ly in (0..nn).rev() {
            if ly > 0 {
                if self.layers[ly].st > self.layers[ly].ul {
                    let ul_excess = self.layers[ly].st - self.layers[ly].ul;
                    self.layers[ly].st = self.layers[ly].ul;
                    self.layers[ly - 1].st += ul_excess;
                }
            } else {
                let mut ul_excess = self.layers[0].st - self.layers[0].ul;
                if ul_excess > 0.0 {
                    self.layers[0].st = self.layers[0].ul;
                    for ly1 in 1..nn {
                        self.layers[ly1].st += ul_excess;
                        if self.layers[ly1].st > self.layers[ly1].ul {
                            ul_excess = self.layers[ly1].st - self.layers[ly1].ul;
                            self.layers[ly1].st = self.layers[ly1].ul;
                        } else {
                            ul_excess = 0.0;
                            break;
                        }
                    }
                }
                if ul_excess > 0.0 {
                    if self.wet.is_some() {
                        self.wet_excess(ul_excess, surfq);
                    } else {
                        *surfq += ul_excess;
                    }
                }
            }
        }
    }
}

/// SWAT+ `et_pot`, Hargreaves (`pet` code 2), before the HRU's adjustment.
impl Hru {
    /// Net short-wave radiation for the reference PET, and the net long-wave radiation
    /// (`et_pot`).
    fn pet_radiation(&self, w: &HruWeather, ed: f32) -> (f32, f32) {
        let tk = w.tave + 273.15;
        let ralb = if self.sno_mm <= 0.5 { w.solrad * (1.0 - 0.23) } else { w.solrad * (1.0 - 0.8) };
        let rbo = -(0.34 - 0.139 * ed.sqrt());
        let rto = if w.solradmx < 1.0e-4 { 0.0 } else { 0.9 * (w.solrad / w.solradmx) + 0.1 };
        // tk**4: gfortran multiplies, squaring twice.
        let tk2 = tk * tk;
        let rout = rbo * rto * 4.9e-9 * (tk2 * tk2);
        (ralb, rout)
    }

    /// Mean barometric pressure, latent heat, psychrometric constant and the slope of the
    /// saturation vapour pressure curve (`et_pot`).
    fn pet_constants(&self, w: &HruWeather, ea: f32) -> (f32, f32, f32) {
        let pb = 101.3 - self.elev * (0.01152 - 0.544e-6 * self.elev);
        let xl = 2.501 - 2.361e-3 * w.tave;
        let gma = 1.013e-3 * pb / (0.622 * xl);
        let t = w.tave + 237.3;
        let dlt = 4098.0 * ea / (t * t);
        (xl, gma, dlt)
    }

    /// Priestley-Taylor PET (`pet` 0).
    fn pet_priestley_taylor(&self, w: &HruWeather, ea: f32, ed: f32) -> f32 {
        let (xl, gma, dlt) = self.pet_constants(w, ea);
        let (ralb, rout) = self.pet_radiation(w, ed);
        let rn_pet = ralb + rout;
        (1.28 * (dlt / (dlt + gma)) * rn_pet / xl).max(0.0)
    }

    /// Penman-Monteith PET (`pet` 1) for an alfalfa reference crop, and the maximum plant
    /// transpiration for the HRU's canopy. `w.windsp` comes already raised to 0.01 m/s (the run
    /// loop does it, since SWAT+ changes the HRU's weather and prints it).
    fn pet_penman_monteith(&self, w: &HruWeather, albday: f32, ea: f32, ed: f32, vpd: f32, b: &HruBasin) -> (f32, f32) {
        use crate::fortran::rpow;
        let (xl, gma, dlt) = self.pet_constants(w, ea);
        let (ralb, rout) = self.pet_radiation(w, ed);
        let ralb1 = w.solrad * (1.0 - albday);
        let rn = ralb1 + rout;
        let rn_pet = ralb + rout;
        let rho = 1710.0 - 6.85 * w.tave;
        let co2 = b.plant.co2;
        let rv = 114.0 / (w.windsp * rpow(170.0 / 1000.0, 0.2));
        let rc = 49.0 / (1.4 - 0.4 * co2 / 330.0);
        let pet_day = ((dlt * rn_pet + gma * rho * vpd / rv) / (xl * (dlt + gma * (1.0 + rc / rv)))).max(0.0);

        if !self.plants.iter().any(|p| p.gro) {
            return (pet_day, 0.0);
        }
        let zz = if self.cht_mx <= 1.0 { 170.0 } else { self.cht_mx * 100.0 + 100.0 };
        let uzz = w.windsp * rpow(zz / 1000.0, 0.2);
        let chz = if self.cht_mx < 0.01 { 1.0 } else { self.cht_mx * 100.0 };
        let zom = if chz <= 200.0 { 0.123 * chz } else { 0.058 * rpow(chz, 1.19) };
        let zov = 0.1 * zom;
        let d = 0.667 * chz;
        let mut rv = ((zz - d) / zom).ln() * ((zz - d) / zov).ln();
        rv /= 0.41_f32 * 0.41 * uzz;
        let (mut gsi_adj, mut gsi_wav) = (0.0_f32, 0.0_f32);
        for p in &self.plants {
            let rto = p.lai / (self.lai_sum + 0.01);
            let xx = vpd - 1.0;
            let fvpd = if xx > 0.0 { (1.0 - p.db.vpd2 * xx).max(0.1) } else { 1.0 };
            gsi_adj = p.db.gsi * fvpd;
            gsi_wav += gsi_adj * rto;
        }
        // SWAT+ takes the leaf resistance from the last plant's conductance, not the
        // leaf-area-weighted one it has just summed; corrected, the weighted one.
        let g = if b.fixes { gsi_wav } else { gsi_adj };
        let mut rc = 1.0 / (g + 0.01);
        rc /= 0.5 * (self.lai_sum + 0.01) * (1.4 - 0.4 * co2 / 330.0);
        let ep_max = ((dlt * rn + gma * rho * vpd / rv) / (xl * (dlt + gma * (1.0 + rc / rv)))).max(0.0).min(pet_day);
        (pet_day, ep_max)
    }
}

/// An urban land type of `urban.urb` (`urban_db`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Urban {
    /// Impervious and directly connected impervious fractions of the HRU.
    pub fimp: f32,
    pub fcimp: f32,
    /// Curb length density (km/ha), wash-off coefficient (1/mm), maximum solids (kg/curb km),
    /// days to half of it.
    pub curbden: f32,
    pub urbcoef: f32,
    pub dirtmx: f32,
    pub thalf: f32,
    /// Total N, total P and nitrate in the solids (mg/kg).
    pub tnconc: f32,
    pub tpconc: f32,
    pub tno3conc: f32,
    pub ro: UrbanRunoff,
}

/// How urban runoff picks up its loads (`urb_ro`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UrbanRunoff {
    BuildupWashoff,
    /// Anything else: no loads from the impervious area.
    None,
}

impl Hru {
    /// `hru_urban`, daily: build-up and wash-off of solids on the impervious area, which replace
    /// that share of the sediment and its nutrients. Returns the silt it adds (`silyld`).
    fn hru_urban(&mut self, surfq: f32, phh: f32, nd: &mut NutDay) -> f32 {
        let Some(u) = self.urban else { return 0.0 };
        if u.ro != UrbanRunoff::BuildupWashoff {
            return 0.0;
        }
        if surfq <= 0.1 {
            self.twash += 1.0;
            return 0.0;
        }
        let mut dirt = u.dirtmx * self.twash / (u.thalf + self.twash);
        let dirto = dirt;
        let urbk = u.urbcoef * (nd.qp_cms * 3.6 / self.km);
        let rp1 = -2.0 * (1.0 - phh).ln();
        let durf = 4.605 / rp1;
        let mut turo = durf + self.tconc;
        if turo > 24.0 {
            turo = 24.0;
        }
        let mut xx = urbk * turo;
        if xx > 24.0 {
            xx = 24.0;
        }
        dirt *= (-xx).exp();
        if dirt < 1.0e-6 {
            dirt = 0.0;
        }
        self.twash = u.thalf * dirt / (u.dirtmx - dirt);
        let sus_sol = ((dirto - dirt) * u.curbden).max(0.0);
        let tn = u.tnconc * sus_sol / 1.0e6;
        let tp = u.tpconc * sus_sol / 1.0e6;
        let tno3 = u.tno3conc * sus_sol / 1.0e6;
        let rest = 1.0 - u.fimp;
        nd.sedyld = (0.001 * sus_sol) * u.fimp + nd.sedyld * rest;
        // The silt is in t/ha times the area (t), the sediment in t/ha: as SWAT+ has it.
        let silyld = (0.001 * sus_sol * self.area_ha) * u.fimp;
        nd.surqno3 = tno3 * u.fimp + nd.surqno3 * rest;
        nd.sedorgn = (tn - tno3) * u.fimp + nd.sedorgn * rest;
        nd.sedorgp = 0.75 * tp * u.fimp + nd.sedorgp * rest;
        nd.surqsolp = 0.25 * tp * u.fimp + nd.surqsolp * rest;
        silyld
    }
}

fn pet_hargreaves(w: &HruWeather) -> f32 {
    let xl = 2.501 - 2.361e-3 * w.tave;
    let ramm = w.solradmx * 37.59 / 30.0;
    if w.tmax > w.tmin { (0.0023 * (ramm / xl) * (w.tave + 17.8) * crate::fortran::rpow(w.tmax - w.tmin, 0.5)).max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascrv_passes_through_its_points() {
        let (x5, x6) = ascrv(0.5, 0.95, 0.5, 0.95);
        // y = x / (x + exp(x5 - x6 x)) gives back the two points.
        let y = |x: f32| x / (x + (x5 - x6 * x).exp());
        assert!((y(0.5) - 0.5).abs() < 1e-5);
        assert!((y(0.95) - 0.95).abs() < 1e-5);
    }
}
