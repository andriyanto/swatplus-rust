//! Basin-wide control codes (`codes.bsn`, `basin_read_cc`) and parameters (`parameters.bsn`,
//! `basin_read_prm`). SWAT+ reads each file into its derived type in one list-directed read, so
//! values are matched to fields by position, whatever the header says.

/// `codes.bsn`, field by field in SWAT+'s order (`basin_control_codes`), with its defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasinCodes {
    pub petfile: String,
    pub wwqfile: String,
    /// Potential ET method: 0 Priestley-Taylor, 1 Penman-Monteith, 2 Hargreaves, 3 read in.
    pub pet: i32,
    pub nam1: i32,
    pub crk: i32,
    pub swift_out: i32,
    pub sed_det: i32,
    pub rte: i32,
    pub deg: i32,
    pub wq: i32,
    pub nostress: i32,
    pub cn: i32,
    pub cfac: i32,
    pub cswat: i32,
    /// 1: adjust precipitation and temperature for elevation (lapse rates).
    pub lapse: i32,
    pub uhyd: i32,
    pub sed_ch: i32,
    pub tdrn: i32,
    pub wtdn: i32,
    pub sol_p_model: i32,
    pub gampt: i32,
    pub atmo: String,
    pub smax: i32,
    pub qual2e: i32,
    pub gwflow: i32,
    pub idc_till: i32,
}

impl Default for BasinCodes {
    fn default() -> Self {
        Self {
            petfile: "         pet.cli".into(),
            wwqfile: String::new(),
            pet: 0,
            nam1: 0,
            crk: 0,
            swift_out: 0,
            sed_det: 0,
            rte: 0,
            deg: 0,
            wq: 0,
            nostress: 0,
            cn: 0,
            cfac: 0,
            cswat: 0,
            lapse: 0,
            uhyd: 1,
            sed_ch: 0,
            tdrn: 0,
            wtdn: 0,
            sol_p_model: 0,
            gampt: 0,
            atmo: "a".into(),
            smax: 0,
            qual2e: 0,
            gwflow: 0,
            idc_till: 3,
        }
    }
}

/// Names of the real fields of [`BasinParams`], in SWAT+'s order (`basin_parms`).
pub const BASIN_PARAM_NAMES: [&str; 42] = [
    "evlai",
    "ffcb",
    "surlag",
    "adj_pkr",
    "prf",
    "pestgwfact",
    "temp_decay",
    "cmn",
    "n_updis",
    "p_updis",
    "nperco",
    "pperco",
    "phoskd",
    "psp",
    "rsdco",
    "percop",
    "msk_co1",
    "msk_co2",
    "msk_x",
    "nperco_lchtile",
    "evrch",
    "scoef",
    "cdn",
    "sdnco",
    "bact_swf",
    "tb_adj",
    "cn_froz",
    "dorm_hr",
    "plaps",
    "tlaps",
    "nfixmx",
    "decr_min",
    "rsd_covco",
    "urb_init_abst",
    "petco_pmpt",
    "uhalpha",
    "eros_spl",
    "rill_mult",
    "eros_expo",
    "c_factor",
    "ch_d50",
    "co2",
];

/// SWAT+'s defaults for the real fields, in the order of [`BASIN_PARAM_NAMES`].
pub const BASIN_PARAM_DEFAULTS: [f32; 42] = [
    3.0, 0.0, 4.0, 1.0, 484.0, 0.0, 0.0, 0.003, 20.0, 20.0, 0.10, 10.0, 175.0, 0.40, 0.05, 0.5, 0.75, 0.25, 0.20, 0.5, 0.60, 1.0, 1.40,
    1.30, 0.15, 0.0, 0.000862, -1.0, 0.0, 6.5, 20.0, 0.01, 0.75, 1.0, 100.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 400.0,
];

/// `parameters.bsn`: 42 reals, then two integers.
#[derive(Clone, Debug, PartialEq)]
pub struct BasinParams {
    /// The reals, in the order of [`BASIN_PARAM_NAMES`].
    pub reals: [f32; 42],
    /// Maximum days to lag hydrographs.
    pub day_lag_mx: i32,
    /// Random number generator code: 0 keeps SWAT+'s fixed seeds; any other value moves them.
    pub igen: i32,
}

impl Default for BasinParams {
    fn default() -> Self {
        Self { reals: BASIN_PARAM_DEFAULTS, day_lag_mx: 0, igen: 5 }
    }
}

impl BasinParams {
    /// The real field `name` (see [`BASIN_PARAM_NAMES`]).
    pub fn get(&self, name: &str) -> Option<f32> {
        let i = BASIN_PARAM_NAMES.iter().position(|n| *n == name)?;
        self.reals.get(i).copied()
    }

    pub fn adj_pkr(&self) -> f32 {
        self.reals[3]
    }
    pub fn dorm_hr(&self) -> f32 {
        self.reals[27]
    }
    pub fn plaps(&self) -> f32 {
        self.reals[28]
    }
    pub fn tlaps(&self) -> f32 {
        self.reals[29]
    }
}
