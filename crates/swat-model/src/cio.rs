//! `file.cio`, the master file: which input file SWAT+ reads for each slot.

/// The rows of `file.cio`, in the order SWAT+ reads them (`readcio_read`), each with its slots and
/// their default file names (`input_file_module`). A slot SWAT+ never reads from a row keeps its
/// default.
pub const SCHEMA: &[(&str, &[(&str, &str)])] = &[
    (
        "simulation",
        &[
            ("time", "time.sim"),
            ("prt", "print.prt"),
            ("object_prt", "object.prt"),
            ("object_cnt", "object.cnt"),
            ("cs_db", "constituents.cs"),
        ],
    ),
    ("basin", &[("codes_bas", "codes.bsn"), ("parms_bas", "parameters.bsn"), ("carbon_bsn", "carbon.bsn")]),
    (
        "climate",
        &[
            ("weat_sta", "weather-sta.cli"),
            ("weat_wgn", "weather-wgn.cli"),
            ("pet_cli", "pet.cli"),
            ("pcp_cli", "pcp.cli"),
            ("tmp_cli", "tmp.cli"),
            ("slr_cli", "slr.cli"),
            ("hmd_cli", "hmd.cli"),
            ("wnd_cli", "wnd.cli"),
            ("atmo_cli", "atmodep.cli"),
        ],
    ),
    (
        "connect",
        &[
            ("hru_con", "hru.con"),
            ("hruez_con", "hru-lte.con"),
            ("ru_con", "rout_unit.con"),
            ("gwflow_con", "gwflow.con"),
            ("aqu_con", "aquifer.con"),
            ("aqu2d_con", "aquifer2d.con"),
            ("chan_con", "channel.con"),
            ("res_con", "reservoir.con"),
            ("rec_con", "recall.con"),
            ("exco_con", "exco.con"),
            ("delr_con", "delratio.con"),
            ("out_con", "outlet.con"),
            ("chandeg_con", "chandeg.con"),
        ],
    ),
    (
        "channel",
        &[
            ("init", "initial.cha"),
            ("dat", "channel.cha"),
            ("hyd", "hydrology.cha"),
            ("sed", "sediment.cha"),
            ("nut", "nutrients.cha"),
            ("chan_ez", "channel-lte.cha"),
            ("hyd_sed", "hyd-sed-lte.cha"),
            ("temp", "temperature.cha"),
        ],
    ),
    (
        "reservoir",
        &[
            ("init_res", "initial.res"),
            ("res", "reservoir.res"),
            ("hyd_res", "hydrology.res"),
            ("sed_res", "sediment.res"),
            ("nut_res", "nutrients.res"),
            ("weir_res", "weir.res"),
            ("wet", "wetland.wet"),
            ("hyd_wet", "hydrology.wet"),
        ],
    ),
    ("routing_unit", &[("ru_def", "rout_unit.def"), ("ru_ele", "rout_unit.ele"), ("ru", "rout_unit.rtu"), ("ru_dr", "rout_unit.dr")]),
    ("hru", &[("hru_data", "hru-data.hru"), ("hru_ez", "hru-lte.hru")]),
    (
        "exco",
        &[
            ("exco", "exco.exc"),
            ("om", "exco_om.exc"),
            ("pest", "exco_pest.exc"),
            ("path", "exco_path.exc"),
            ("hmet", "exco_hmet.exc"),
            ("salt", "exco_salt.exc"),
        ],
    ),
    ("recall", &[("recall_rec", "recall.rec")]),
    (
        "dr",
        &[
            ("del_ratio", "delratio.del"),
            ("om", "dr_om.del"),
            ("pest", "dr_pest.del"),
            ("path", "dr_path.del"),
            ("hmet", "dr_hmet.del"),
            ("salt", "dr_salt.del"),
        ],
    ),
    ("aquifer", &[("init", "initial.aqu"), ("aqu", "aquifer.aqu")]),
    ("herd", &[("animal", "animal.hrd"), ("herd", "herd.hrd"), ("ranch", "ranch.hrd")]),
    ("water_rights", &[("pou", "place_of_use.wro"), ("pod", "point_of_diver.wro")]),
    ("link", &[("chan_surf", "chan-surf.lin"), ("aqu_cha", "aqu_cha.lin")]),
    ("hydrology", &[("hydrol_hyd", "hydrology.hyd"), ("topogr_hyd", "topography.hyd"), ("field_fld", "field.fld")]),
    (
        "structural",
        &[
            ("tiledrain_str", "tiledrain.str"),
            ("septic_str", "septic.str"),
            ("fstrip_str", "filterstrip.str"),
            ("grassww_str", "grassedww.str"),
            ("bmpuser_str", "bmpuser.str"),
        ],
    ),
    (
        "hru_parm_db",
        &[
            ("plants_plt", "plants.plt"),
            ("fert_frt", "fertilizer.frt"),
            ("till_til", "tillage.til"),
            ("pest", "pesticide.pes"),
            ("pathcom_db", "pathogens.pth"),
            ("hmetcom_db", "metals.mtl"),
            ("saltcom_db", "salt.slt"),
            ("urban_urb", "urban.urb"),
            ("septic_sep", "septic.sep"),
            ("snow", "snow.sno"),
        ],
    ),
    (
        "ops",
        &[
            ("harv_ops", "harv.ops"),
            ("graze_ops", "graze.ops"),
            ("irr_ops", "irr.ops"),
            ("chem_ops", "chem_app.ops"),
            ("fire_ops", "fire.ops"),
            ("sweep_ops", "sweep.ops"),
        ],
    ),
    (
        "lum",
        &[
            ("landuse_lum", "landuse.lum"),
            ("management_sch", "management.sch"),
            ("cntable_lum", "cntable.lum"),
            ("cons_prac_lum", "cons_practice.lum"),
            ("ovn_lum", "ovn_table.lum"),
        ],
    ),
    (
        "chg",
        &[
            ("cal_parms", "cal_parms.cal"),
            ("cal_upd", "calibration.cal"),
            ("codes_sft", "codes.sft"),
            ("wb_parms_sft", "wb_parms.sft"),
            ("water_balance_sft", "water_balance.sft"),
            ("ch_sed_budget_sft", "ch_sed_budget.sft"),
            ("ch_sed_parms_sft", "ch_sed_parms.sft"),
            ("plant_parms_sft", "plant_parms.sft"),
            ("plant_gro_sft", "plant_gro.sft"),
        ],
    ),
    (
        "init",
        &[
            ("plant", "plant.ini"),
            ("soil_plant_ini", "soil_plant.ini"),
            ("om_water", "om_water.ini"),
            ("pest_soil", "pest_hru.ini"),
            ("pest_water", "pest_water.ini"),
            ("path_soil", "path_hru.ini"),
            ("path_water", "path_water.ini"),
            ("hmet_soil", "hmet_hru.ini"),
            ("hmet_water", "hmet_water.ini"),
            ("salt_soil", "salt_hru.ini"),
            ("salt_water", "salt_water.ini"),
        ],
    ),
    ("soils", &[("soils_sol", "soils.sol"), ("nut_sol", "nutrients.sol"), ("lte_sol", "soils_lte.sol")]),
    ("decision_table", &[("dtbl_lum", "lum.dtl"), ("dtbl_res", "res_rel.dtl"), ("dtbl_scen", "scen_lu.dtl"), ("dtbl_flo", "flo_con.dtl")]),
    (
        "regions",
        &[
            ("ele_lsu", "ls_unit.ele"),
            ("def_lsu", "ls_unit.def"),
            ("ele_reg", "ls_reg.ele"),
            ("def_reg", "ls_reg.def"),
            ("cal_lcu", "ls_cal.reg"),
            ("ele_cha", "ch_catunit.ele"),
            ("def_cha", "ch_catunit.def"),
            ("def_cha_reg", "ch_reg.def"),
            ("ele_aqu", "aqu_catunit.ele"),
            ("def_aqu", "aqu_catunit.def"),
            ("def_aqu_reg", "aqu_reg.def"),
            ("ele_res", "res_catunit.ele"),
            ("def_res", "res_catunit.def"),
            ("def_res_reg", "res_reg.def"),
            ("ele_psc", "rec_catunit.ele"),
            ("def_psc", "rec_catunit.def"),
            ("def_psc_reg", "rec_reg.def"),
        ],
    ),
    // Weather data folders: blank by default, which means the scenario folder.
    ("pcp_path", &[("pcp", "")]),
    ("tmp_path", &[("tmp", "")]),
    ("slr_path", &[("slr", "")]),
    ("hmd_path", &[("hmd", "")]),
    ("wnd_path", &[("wnd", "")]),
];

/// One row of `file.cio`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CioSection {
    /// The row's key in [`SCHEMA`] (SWAT+ reads rows by position; the name in the file is ignored).
    pub key: String,
    /// The name written at the start of the row, when the row was there.
    pub name_in_file: Option<String>,
    /// Each slot and its file name; `None` when the file says `null` (the slot is off).
    pub files: Vec<(String, Option<String>)>,
}

/// `file.cio`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileCio {
    pub title: String,
    pub sections: Vec<CioSection>,
    /// Output folder from the line after the weather paths, when there is one.
    pub out_path: Option<String>,
}

impl FileCio {
    /// `file.cio` with every slot at its default, as SWAT+ starts before reading it.
    pub fn defaults() -> Self {
        let sections = SCHEMA
            .iter()
            .map(|(key, slots)| CioSection {
                key: (*key).to_string(),
                name_in_file: None,
                files: slots.iter().map(|(slot, file)| ((*slot).to_string(), Some((*file).to_string()))).collect(),
            })
            .collect();
        Self { title: String::new(), sections, out_path: None }
    }

    /// The file in `section`/`slot`: `None` when it is `null`, blank, or the slot doesn't exist.
    pub fn file(&self, section: &str, slot: &str) -> Option<&str> {
        let s = self.sections.iter().find(|s| s.key == section)?;
        let (_, file) = s.files.iter().find(|(name, _)| name == slot)?;
        file.as_deref().filter(|f| !f.trim().is_empty())
    }
}
