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
