//! Climate inputs: weather stations (`weather-sta.cli`), weather generator parameters
//! (`weather-wgn.cli`) and measured station series (`*.pcp`, `*.tmp`, `*.slr`, `*.hmd`, `*.wnd`).

/// A weather station (`cli_staread`): which generator and which measured file each variable uses.
/// A value of `sim` means the variable is generated; `None` means the file says `null`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeatherStation {
    pub name: String,
    pub wgn: Option<String>,
    pub pcp: Option<String>,
    pub tmp: Option<String>,
    pub slr: Option<String>,
    pub hmd: Option<String>,
    pub wnd: Option<String>,
    /// The eighth column: `wnd_dir` in the header, the PET gage in SWAT+ (`petgage`).
    pub pet: Option<String>,
    pub atmo_dep: Option<String>,
}

/// Weather generator parameters of one station (`cli_wgnread`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wgn {
    pub name: String,
    pub lat: f32,
    pub lon: f32,
    pub elev: f32,
    /// Years of record behind the half-hour rainfall statistic.
    pub rain_yrs: f32,
    pub months: [WgnMonth; 12],
}

/// One month of weather generator parameters, in file order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WgnMonth {
    pub tmp_max_ave: f32,
    pub tmp_min_ave: f32,
    pub tmp_max_sd: f32,
    pub tmp_min_sd: f32,
    pub pcp_ave: f32,
    pub pcp_sd: f32,
    pub pcp_skew: f32,
    pub wet_dry: f32,
    pub wet_wet: f32,
    pub pcp_days: f32,
    pub pcp_hhr: f32,
    pub slr_ave: f32,
    pub dew_ave: f32,
    pub wnd_ave: f32,
}

/// The measured variables, one station list file each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClimateVar {
    Pcp,
    Tmp,
    Slr,
    Hmd,
    Wnd,
    Pet,
}

impl ClimateVar {
    pub const ALL: [ClimateVar; 6] = [ClimateVar::Pcp, ClimateVar::Tmp, ClimateVar::Slr, ClimateVar::Hmd, ClimateVar::Wnd, ClimateVar::Pet];

    /// The slot of its station list in the `climate` row of `file.cio`.
    pub fn cio_slot(self) -> &'static str {
        match self {
            ClimateVar::Pcp => "pcp_cli",
            ClimateVar::Tmp => "tmp_cli",
            ClimateVar::Slr => "slr_cli",
            ClimateVar::Hmd => "hmd_cli",
            ClimateVar::Wnd => "wnd_cli",
            ClimateVar::Pet => "pet_cli",
        }
    }

    /// The `file.cio` row holding its data folder, if it has one.
    pub fn path_section(self) -> Option<(&'static str, &'static str)> {
        match self {
            ClimateVar::Pcp => Some(("pcp_path", "pcp")),
            ClimateVar::Tmp => Some(("tmp_path", "tmp")),
            ClimateVar::Slr => Some(("slr_path", "slr")),
            ClimateVar::Hmd => Some(("hmd_path", "hmd")),
            ClimateVar::Wnd => Some(("wnd_path", "wnd")),
            ClimateVar::Pet => None,
        }
    }

    /// Values per daily record: maximum and minimum for temperature, one for the others.
    pub fn values(self) -> usize {
        if self == ClimateVar::Tmp { 2 } else { 1 }
    }
}

/// A station list (`pcp.cli`, `tmp.cli`…) as written: the order matters, because SWAT+ finds a
/// station's file in it with a binary search (`search`), which only works on a sorted list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationList {
    pub var: ClimateVar,
    pub files: Vec<String>,
}

/// A measured station file, as written. Records are kept as read: SWAT+ places them into its
/// daily arrays when the simulation starts, which the engine reproduces.
#[derive(Clone, Debug, PartialEq)]
pub struct StationSeries {
    pub var: ClimateVar,
    pub file: String,
    /// Years in the file, as its header says.
    pub nbyr: i32,
    /// Time steps per day: 0 is daily.
    pub tstep: i32,
    pub lat: f32,
    pub lon: f32,
    pub elev: f32,
    pub records: Vec<ClimateRecord>,
}

/// One record: year, day of year, the hour for sub-daily precipitation, and its values (the
/// second only for temperature). SWAT+ marks missing values with -99.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimateRecord {
    pub year: i32,
    pub jday: i32,
    pub hour: Option<i32>,
    pub values: [f32; 2],
}
