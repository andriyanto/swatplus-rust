//! Data model of a SWAT+ project.
//!
//! Plain data, no I/O: the readers in `swat-io` fill it from TxtInOut files, and the engine runs on
//! it. Field names follow SWAT+ where a reader of the Fortran source would look for them, and each
//! type names the file and the Fortran routine it mirrors. Reals are `f32`, as SWAT+'s default
//! `real` is single precision (decision K4 in the RustGIS design, docs/swat/design.md).
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable))]

pub mod basin;
pub mod cio;
pub mod climate;
pub mod hru;
pub mod objects;
pub mod print;
pub mod schema;
pub mod table;
pub mod time;

pub use basin::{BasinCodes, BasinParams};
pub use cio::{CioSection, FileCio};
pub use climate::{ClimateRecord, ClimateVar, StationList, StationSeries, WeatherStation, Wgn, WgnMonth};
pub use hru::{HruData, PlantCommunity, PlantInit, Soil, SoilLayer, SoilPlantInit};
pub use objects::{ObjectKind, Outflow, SpatialObject};
pub use print::{PrintObject, PrintSettings};
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
    /// `plants.plt`.
    pub plants: Option<Table>,
    /// `plant.ini`.
    pub plant_communities: Vec<PlantCommunity>,
    /// `nutrients.sol`.
    pub soil_test: Option<Table>,
    /// Whether the project has `co2_yr.dat` (yearly CO2, read by `co2_read`; not handled yet).
    pub co2_yr_dat: bool,
}

impl Project {
    /// The objects of one kind, in file order.
    pub fn objects_of(&self, kind: ObjectKind) -> impl Iterator<Item = &SpatialObject> {
        self.objects.iter().filter(move |o| o.kind == kind)
    }
}
