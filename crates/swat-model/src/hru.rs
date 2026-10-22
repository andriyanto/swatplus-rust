//! HRUs (`hru-data.hru`), soils (`soils.sol`) and their initial conditions (`soil_plant.ini`).

/// One row of `hru-data.hru` (`hru_read`): the HRU's number and the names of its database entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HruData {
    pub id: i32,
    pub name: String,
    pub topo: String,
    pub hyd: String,
    pub soil: String,
    pub land_use_mgt: String,
    pub soil_plant_init: String,
    pub surf_stor: String,
    pub snow: String,
    pub field: String,
}

/// A soil of `soils.sol` (`soil_db_read`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Soil {
    pub name: String,
    pub nly: i32,
    /// Hydrologic group, A–D.
    pub hydgrp: String,
    /// Maximum rooting depth (mm).
    pub zmx: f32,
    pub anion_excl: f32,
    pub crk: f32,
    pub texture: String,
    pub layers: Vec<SoilLayer>,
}

/// A soil layer, in file order (`soildb%ly`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SoilLayer {
    /// Depth from the surface to the bottom of the layer (mm).
    pub z: f32,
    pub bd: f32,
    pub awc: f32,
    pub k: f32,
    pub cbn: f32,
    pub clay: f32,
    pub silt: f32,
    pub sand: f32,
    pub rock: f32,
    pub alb: f32,
    pub usle_k: f32,
    pub ec: f32,
    pub cal: f32,
    pub ph: f32,
}

/// A row of `soil_plant.ini` (`soil_plant_init`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoilPlantInit {
    pub name: String,
    /// Initial soil water as a fraction of field capacity.
    pub sw_frac: f32,
    pub nutc: String,
    pub pestc: String,
    pub pathc: String,
    pub saltc: String,
    pub hmetc: String,
    pub csc: String,
}

/// One plant of a community in `plant.ini` (`plant_init_db`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlantInit {
    /// Plant name in `plants.plt`.
    pub cpnm: String,
    /// Growing at the start (`y`) or not (`lc_status`).
    pub igro: String,
    pub lai: f32,
    pub bioms: f32,
    /// Fraction of heat units accumulated.
    pub phuacc: f32,
    pub pop: f32,
    /// Fraction of the years to maturity reached (`yrs_init`).
    pub fr_yrmat: f32,
    /// Initial surface residue (kg/ha).
    pub rsdin: f32,
}

impl Default for PlantInit {
    fn default() -> Self {
        Self { cpnm: "frsd".into(), igro: "y".into(), lai: 0.0, bioms: 0.0, phuacc: 0.0, pop: 0.0, fr_yrmat: 1.0, rsdin: 10000.0 }
    }
}

/// A plant community of `plant.ini` (`plant_community_db`, read by `readpcom`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlantCommunity {
    pub name: String,
    pub rot_yr_ini: i32,
    pub plants: Vec<PlantInit>,
}
