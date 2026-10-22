//! `print.prt`: which outputs SWAT+ writes, and when (`basin_print_codes_read`).

/// `print.prt`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrintSettings {
    /// Years skipped before printing (warm-up).
    pub nyskip: i32,
    pub day_start: i32,
    pub yrc_start: i32,
    pub day_end: i32,
    pub yrc_end: i32,
    /// Print every n days (SWAT+ `int_day`).
    pub interval: i32,
    /// Years of the average annual intervals (empty: the whole period).
    pub aa_years: Vec<i32>,
    /// `y` writes CSV files too.
    pub csvout: String,
    /// The second flag of that row (`dbout` in the header, `use_obj_labels` in SWAT+).
    pub use_obj_labels: String,
    pub cdfout: String,
    /// The first flag of the next row (`soilout` in the header, `crop_yld` in SWAT+).
    pub crop_yld: String,
    pub mgtout: String,
    pub hydcon: String,
    pub fdcout: String,
    /// The object table, in file order.
    pub objects: Vec<PrintObject>,
}

/// One row of the object table: the output's name and its daily, monthly, yearly and average
/// annual flags (`y`, `n`, and `l` for some carbon outputs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrintObject {
    pub name: String,
    pub daily: String,
    pub monthly: String,
    pub yearly: String,
    pub avann: String,
}

impl PrintSettings {
    /// The row of output `name`, if the file lists it.
    pub fn object(&self, name: &str) -> Option<&PrintObject> {
        self.objects.iter().find(|o| o.name == name)
    }
}
