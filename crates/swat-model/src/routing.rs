//! Routing units and aquifers: `rout_unit.rtu`, `rout_unit.def` and `rout_unit.ele` (`ru_read`,
//! `ru_read_elements`), and `aquifer.aqu` (`aqu_read`).

/// An element of a routing unit (`routing_unit_elements`, a row of `rout_unit.ele`): the object
/// (`hru`, `ru`, `sdc`...) and its number, the share of it in the unit, and its delivery ratio's
/// name (`0` or `calc` computed from times of concentration, `1` or `full` for none).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuElement {
    pub name: String,
    pub obtyp: String,
    pub obtypno: i32,
    pub frac: f32,
    pub dr_name: String,
}

/// A routing unit's definition (`rout_unit.def`): its elements, as numbers of `rout_unit.ele`
/// rows (1-based).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuDef {
    pub name: String,
    pub elems: Vec<usize>,
}

/// A routing unit's data (`rout_unit.rtu`): its definition, delivery ratio, topography and
/// field, by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuData {
    pub name: String,
    pub elem_def: String,
    pub elem_dr: String,
    pub topo: String,
    pub field: String,
}

/// An aquifer (`aquifer_database`, a row of `aquifer.aqu`).
#[derive(Clone, Debug, PartialEq)]
pub struct AquiferDb {
    pub name: String,
    pub init: String,
    /// Initial return flow (mm).
    pub flo: f32,
    /// Depth to the aquifer's bottom and to the water table (m).
    pub dep_bot: f32,
    pub dep_wt: f32,
    /// Initial nitrate and mineral P (ppm).
    pub no3: f32,
    pub minp: f32,
    pub cbn: f32,
    pub flo_dist: f32,
    pub bf_max: f32,
    /// Recession constant (1/day).
    pub alpha: f32,
    pub revap_co: f32,
    /// Share of the recharge that seeps to the deep aquifer.
    pub seep: f32,
    pub spyld: f32,
    pub hlife_n: f32,
    /// Water table depths below which return flow and revap occur (m).
    pub flo_min: f32,
    pub revap_min: f32,
}

impl Default for AquiferDb {
    fn default() -> Self {
        AquiferDb {
            name: String::new(),
            init: String::new(),
            flo: 0.05,
            dep_bot: 0.0,
            dep_wt: 0.0,
            no3: 0.0,
            minp: 0.0,
            cbn: 0.5,
            flo_dist: 50.0,
            bf_max: 0.0,
            alpha: 0.0,
            revap_co: 0.0,
            seep: 0.0,
            spyld: 0.0,
            hlife_n: 30.0,
            flo_min: 0.0,
            revap_min: 0.0,
        }
    }
}

/// A SWAT-DEG channel's data names (`swatdeg_datafiles`, a row of `channel-lte.cha`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SdChannelData {
    pub name: String,
    pub init: String,
    pub hyd: String,
    pub sed: String,
    pub nut: String,
}
