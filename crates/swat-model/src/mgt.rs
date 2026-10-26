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
    /// The crops of a generic plant-and-harvest table (`auto_crop`: one for `pl_hv_summer1` and
    /// `pl_hv_winter1`, two for `pl_hv_summer2`), given after its name.
    pub auto_crops: Vec<String>,
    pub ops: Vec<MgtOp>,
}

/// A condition of a decision table (`conditions_var`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DtblCond {
    pub var: String,
    pub ob: String,
    pub ob_num: i32,
    pub lim_var: String,
    pub lim_op: String,
    pub lim_const: f32,
}

/// An action of a decision table (`actions_var`).
#[derive(Clone, Debug, PartialEq)]
pub struct DtblAct {
    pub typ: String,
    pub ob: String,
    pub ob_num: i32,
    pub name: String,
    pub option: String,
    pub const_: f32,
    /// At least 1 (`dtbl_lum_read`).
    pub const2: f32,
    pub file_pointer: String,
}

/// A decision table of `lum.dtl` (`decision_table`, read by `dtbl_lum_read`): its conditions with
/// the alternatives each takes (`alt(cond, alt)`), and its actions with their outcome for each
/// alternative (`act_outcomes(act, alt)`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecisionTable {
    pub name: String,
    pub alts: usize,
    pub conds: Vec<DtblCond>,
    pub alt: Vec<Vec<String>>,
    pub acts: Vec<DtblAct>,
    pub outcomes: Vec<Vec<String>>,
}
