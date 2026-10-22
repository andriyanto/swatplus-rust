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

    /// SWAT+ `basin_prm_default`, run after reading: a value below 1e-6 (zero, or left out) takes
    /// a default, which for some isn't the type's own default (`prf` 1, `cmn` 0.0003,
    /// `urb_init_abst` 0). `petco_pmpt` between 0 and 0.5 becomes 0; `co2` below 100 becomes 400.
    pub fn complete(&mut self) {
        const FLOORED: [(&str, f32); 34] = [
            ("evlai", 3.0),
            ("ffcb", 0.0),
            ("surlag", 4.0),
            ("adj_pkr", 1.0),
            ("prf", 1.0),
            ("cmn", 0.0003),
            ("n_updis", 20.0),
            ("p_updis", 20.0),
            ("nperco", 0.1),
            ("pperco", 10.0),
            ("phoskd", 175.0),
            ("psp", 0.40),
            ("rsdco", 0.05),
            ("percop", 0.5),
            ("msk_co1", 0.75),
            ("msk_co2", 0.25),
            ("msk_x", 0.20),
            ("nperco_lchtile", 0.50),
            ("evrch", 0.60),
            ("cdn", 1.40),
            ("sdnco", 1.30),
            ("bact_swf", 0.15),
            ("tb_adj", 0.0),
            ("cn_froz", 0.000862),
            ("nfixmx", 20.0),
            ("decr_min", 0.01),
            ("rsd_covco", 0.75),
            ("urb_init_abst", 0.0),
            ("uhalpha", 1.0),
            ("eros_spl", 0.0),
            ("rill_mult", 0.0),
            ("eros_expo", 0.0),
            ("c_factor", 0.0),
            ("ch_d50", 0.0),
        ];
        let at = |name: &str| BASIN_PARAM_NAMES.iter().position(|n| *n == name);
        for (name, value) in FLOORED {
            if let Some(v) = at(name).and_then(|i| self.reals.get_mut(i))
                && *v < 1.0e-6
            {
                *v = value;
            }
        }
        if let Some(v) = at("petco_pmpt").and_then(|i| self.reals.get_mut(i))
            && *v < 0.5
            && *v > 0.0
        {
            *v = 0.0;
        }
        if let Some(v) = at("co2").and_then(|i| self.reals.get_mut(i))
            && *v < 100.0
        {
            *v = 400.0;
        }
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
