//! Data model of a SWAT+ project.
//!
//! Plain data, no I/O: the readers in `swat-io` fill it from TxtInOut files, and the engine runs on
//! it. Field names follow SWAT+ where a reader of the Fortran source would look for them, and each
//! type names the file and the Fortran routine it mirrors. Reals are `f32`, as SWAT+'s default
//! `real` is single precision (decision K4 in the RustGIS design, docs/swat/design.md).
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable))]

pub mod basin;
pub mod cal;
pub mod cio;
pub mod climate;
pub mod hru;
pub mod mgt;
pub mod objects;
pub mod print;
pub mod routing;
pub mod schema;
pub mod table;
pub mod time;

pub use basin::{BasinCodes, BasinParams};
pub use cal::{CalParm, CalUpdate};
pub use cio::{CioSection, FileCio};
pub use climate::{ClimateRecord, ClimateVar, StationList, StationSeries, WeatherStation, Wgn, WgnMonth};
pub use hru::{HruData, PlantCommunity, PlantInit, Soil, SoilLayer, SoilPlantInit};
pub use mgt::{DecisionTable, DtblAct, DtblCond, MgtOp, MgtSchedule};
pub use objects::{ObjectKind, Outflow, SpatialObject};
pub use print::{PrintObject, PrintSettings};
pub use routing::{AquiferDb, RuData, RuDef, RuElement, SdChannelData};
pub use table::Table;
pub use time::{SimDay, TimeSim};

/// A SWAT+ project as far as the readers go so far: the master file, the simulation period, the
/// print settings, the spatial objects and the climate inputs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    pub cio: FileCio,
    pub time: TimeSim,
    pub print: PrintSettings,
    pub codes: BasinCodes,
    pub params: BasinParams,
    pub objects: Vec<SpatialObject>,
    pub stations: Vec<WeatherStation>,
    pub wgn: Vec<Wgn>,
    /// The station lists, one per measured variable `file.cio` names.
    pub station_lists: Vec<StationList>,
    /// The measured station files SWAT+ reads, in list order.
    pub series: Vec<StationSeries>,
    /// `hru-data.hru`, indexed by HRU number - 1 (SWAT+ places rows by their number).
    pub hru_data: Vec<HruData>,
    pub soils: Vec<Soil>,
    pub soil_plant_init: Vec<SoilPlantInit>,
    pub topography: Option<Table>,
    pub hydrology: Option<Table>,
    pub field: Option<Table>,
    pub landuse: Option<Table>,
    pub cntable: Option<Table>,
    pub ovn_table: Option<Table>,
    pub cons_practice: Option<Table>,
    pub snow: Option<Table>,
    pub tillage: Option<Table>,
    /// `carbon.bsn`, one row.
    pub carbon: Option<Table>,
    /// `carbon_lyr.bsn` (its name made from `carbon.bsn`'s), one row per layer kind.
    pub carbon_lyr: Option<Table>,
    /// `plants.plt`.
    pub plants: Option<Table>,
    /// `urban.urb`.
    pub urban: Option<Table>,
    /// `wetland.wet`, `hydrology.wet`, `initial.res`, `sediment.res`, `nutrients.res`, the
    /// release tables of `res_rel.dtl`, and `weir.res`.
    pub wetland: Option<Table>,
    pub wet_hyd: Option<Table>,
    pub res_init: Option<Table>,
    pub res_sed: Option<Table>,
    pub res_nut: Option<Table>,
    pub res_dtl: Vec<DecisionTable>,
    pub weir_res: Option<Table>,
    /// `reservoir.res` and `hydrology.res`.
    pub res_dat: Option<Table>,
    pub res_hyd: Option<Table>,
    /// `atmo.cli`.
    pub atmodep: Option<AtmoDep>,
    /// `recall.rec` and the files it names (`recall_read`).
    pub recalls: Vec<RecallDb>,
    /// Read by these names whatever `file.cio` says: `puddle.ops`, `manure_db.frt`,
    /// `manure_om.frt`, and whether `transplant.ops` is there.
    pub puddle_ops: Option<Table>,
    pub manure_db: Option<Table>,
    pub manure_om: Option<Table>,
    pub transplant_ops: Option<Table>,
    /// `plant.ini`.
    pub plant_communities: Vec<PlantCommunity>,
    /// `nutrients.sol`.
    pub soil_test: Option<Table>,
    /// `management.sch`.
    pub management: Vec<MgtSchedule>,
    /// `lum.dtl`.
    pub lum_dtl: Vec<DecisionTable>,
    /// `harv.ops`, `fertilizer.frt`, `irr.ops` and `chem_app.ops`.
    pub harv_ops: Option<Table>,
    pub fertilizer: Option<Table>,
    pub irr_ops: Option<Table>,
    pub chem_app: Option<Table>,
    /// `soil_lyr_depths.sol` (custom layer depths, mm, as listed), when the project has it.
    pub soil_lyr_depths: Option<Vec<i32>>,
    /// `object.cnt`'s basin name and land and total areas (`bsn`).
    pub basin_name: String,
    pub area_ls_ha: f32,
    pub area_tot_ha: f32,
    /// `ls_unit.def` and `ls_unit.ele` (by row number, element 0 unused; empty without the
    /// file), and whether `aqu_catunit.ele` is there (the basin aquifer table needs it).
    pub lsu_defs: Vec<routing::LsuDef>,
    pub lsu_elems: Vec<routing::LsuElem>,
    pub aqu_elems: bool,
    /// `rout_unit.ele` (by row number, element 0 unused), `rout_unit.def` and `rout_unit.rtu`.
    pub ru_elements: Vec<RuElement>,
    pub ru_defs: Vec<RuDef>,
    pub ru_data: Vec<RuData>,
    /// `aquifer.aqu`, by row number (element 0 holds the defaults).
    pub aquifer_db: Vec<AquiferDb>,
    /// `channel-lte.cha` (by row number, element 0 unused), `hyd-sed-lte.cha`, `nutrients.cha`,
    /// `initial.cha`, `om_water.ini`, and whether `temperature.cha` is named.
    pub sd_channel_data: Vec<SdChannelData>,
    pub hyd_sed_lte: Option<Table>,
    pub ch_nut: Option<Table>,
    pub ch_init: Option<Table>,
    pub om_water: Option<Table>,
    pub ch_temp_file: bool,
    /// `cal_parms.cal` and `calibration.cal`.
    pub cal_parms: Vec<CalParm>,
    pub cal_upd: Vec<CalUpdate>,
    /// Whether the project has `co2_yr.dat` (yearly CO2, read by `co2_read`; not handled yet).
    pub co2_yr_dat: bool,
}

impl Project {
    /// The objects of one kind, in file order.
    pub fn objects_of(&self, kind: ObjectKind) -> impl Iterator<Item = &SpatialObject> {
        self.objects.iter().filter(move |o| o.kind == kind)
    }
}

/// A recall object's data (`recall.rec` and its file): measured or modelled loads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecallDb {
    /// The row number in `recall.rec`, the name, the time step (0 subdaily, 1 daily, 2 monthly,
    /// 3 yearly, 4 an export coefficient) and the file.
    pub id: usize,
    pub name: String,
    pub typ: i32,
    pub file: String,
    /// The file's records: day of the year, month, day of the month, year, and the 18 values
    /// of a hydrograph (flow m3, sediment t, nutrients kg…, temperature).
    pub rows: Vec<RecallRow>,
}

/// A record of a recall file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecallRow {
    pub jday: i32,
    pub mo: i32,
    pub day_mo: i32,
    pub iyr: i32,
    pub vals: [f32; 18],
}

/// `atmo.cli` (`cli_read_atmodep`): nitrogen deposition by station, average annual (`aa`),
/// yearly (`yr`) or monthly (`mo`) from `yr_init`/`mo_init`, `num` values.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AtmoDep {
    pub timestep: String,
    pub mo_init: i32,
    pub yr_init: i32,
    pub num: usize,
    pub stations: Vec<AtmoStation>,
}

/// A deposition station: ammonium and nitrate in rain (mg/L) and dry (kg/ha a year), one value
/// a time step (one for `aa`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AtmoStation {
    pub name: String,
    pub nh4_rf: Vec<f32>,
    pub no3_rf: Vec<f32>,
    pub nh4_dry: Vec<f32>,
    pub no3_dry: Vec<f32>,
}
