//! Management schedules: `management.sch` (`mgt_read_mgtops`, `read_mgtops`).

/// One scheduled operation (`management_ops`), as the file gives it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MgtOp {
    /// Operation code: `plnt`, `hvkl`, `till`, `irrm`, `fert`, `skip`, ...
    pub op: String,
    pub mon: i32,
    pub day: i32,
    /// Heat units (fraction) that trigger the operation; 0 for a dated one.
    pub husc: f32,
    /// First name (`op_data1`): the plant, tillage, irrigation or fertilizer.
    pub op_char: String,
    /// Second name (`op_data2`): the harvest operation, or the chemical application.
    pub op_plant: String,
    /// Amount (`op_data3`): kg/ha of fertilizer, say.
    pub op3: f32,
}

/// A schedule (`management_schedule`): its decision tables (by name) and operations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MgtSchedule {
    pub name: String,
    pub autos: Vec<String>,
    pub ops: Vec<MgtOp>,
}
