//! Schemas of the tables read one derived type per row, from the Fortran type definitions (names,
//! order, defaults). The comment on each names the type and the routine that reads the file.

use crate::table::Kind::{Int as I, Real as R, Text as T};
use crate::table::{Field, a, f};

/// `topography.hyd`: `topography_db` (`topo_read`).
pub const TOPOGRAPHY: &[Field] = &[
    f("name", T, "default"),
    f("slope", R, ".02"),
    f("slope_len", R, "50."),
    f("lat_len", R, "50."),
    f("dis_stream", R, "100."),
    f("dep_co", R, "1."),
];

/// `hydrology.hyd`: `hydrology_db` (`hydrol_read`).
pub const HYDROLOGY: &[Field] = &[
    f("name", T, ""),
    f("lat_ttime", R, "0."),
    f("lat_sed", R, "0."),
    f("canmx", R, "0."),
    f("esco", R, "0."),
    f("epco", R, "0."),
    f("erorgn", R, "0."),
    f("erorgp", R, "0."),
    f("cn3_swf", R, "0."),
    f("biomix", R, "0."),
    f("perco", R, "0."),
    f("lat_orgn", R, "0."),
    f("lat_orgp", R, "0."),
    f("pet_co", R, "1.0"),
    f("latq_co", R, "0.3"),
];

/// `landuse.lum`: `land_use_management` (`landuse_read`).
/// `wetland.wet` (`wet_read`): an id, then the wetland's name and the names of its data.
pub const WETLAND: &[Field] =
    &[f("id", I, "0"), f("name", T, ""), f("init", T, ""), f("hyd", T, ""), f("rel", T, ""), f("sed", T, ""), f("nut", T, "")];

/// `hydrology.wet` (`wetland_hyd_data`).
pub const WET_HYD: &[Field] = &[
    f("name", T, "default"),
    f("psa", R, "0."),
    f("pdep", R, "0."),
    f("esa", R, "0."),
    f("edep", R, "0."),
    f("k", R, "0.01"),
    f("evrsv", R, "0.7"),
    f("acoef", R, "1."),
    f("bcoef", R, "1."),
    f("ccoef", R, "1."),
    f("frac", R, "0.5"),
];

/// `initial.res` (`res_init_dat_c`).
pub const RES_INIT: &[Field] =
    &[f("name", T, ""), f("org_min", T, ""), f("pest", T, ""), f("path", T, ""), f("hmet", T, ""), f("salt", T, "")];

/// `sediment.res` (`reservoir_sed_data`).
pub const RES_SED: &[Field] = &[
    f("name", T, ""),
    f("nsed", R, "0."),
    f("d50", R, "0."),
    f("carbon", R, "0."),
    f("bd", R, "0."),
    f("sed_stlr", R, "0."),
    f("velsetlr", R, "0."),
];

/// `nutrients.res` (`reservoir_nut_data`).
pub const RES_NUT: &[Field] = &[
    f("name", T, ""),
    f("ires1", I, "0"),
    f("ires2", I, "0"),
    f("nsetlr1", R, "0."),
    f("nsetlr2", R, "0."),
    f("psetlr1", R, "0."),
    f("psetlr2", R, "0."),
    f("nsolr", R, "1."),
    f("psolr", R, "1."),
    f("theta_n", R, "1."),
    f("theta_p", R, "1."),
    f("conc_nmin", R, "0.1"),
    f("conc_pmin", R, "0.01"),
];

/// `urban.urb` (`urban_db`).
pub const URBAN: &[Field] = &[
    f("name", T, ""),
    f("fimp", R, "0.05"),
    f("fcimp", R, "0.05"),
    f("curbden", R, "0."),
    f("urbcoef", R, "0."),
    f("dirtmx", R, "1000."),
    f("thalf", R, "1."),
    f("tnconc", R, "0."),
    f("tpconc", R, "0."),
    f("tno3conc", R, "0."),
    f("urbcn2", R, "98."),
];

pub const LANDUSE: &[Field] = &[
    f("name", T, ""),
    f("cal_group", T, ""),
    f("plant_cov", T, ""),
    f("mgt_ops", T, ""),
    f("cn_lu", T, ""),
    f("cons_prac", T, ""),
    f("urb_lu", T, ""),
    f("urb_ro", T, ""),
    f("ovn", T, ""),
    f("tiledrain", T, ""),
    f("septic", T, ""),
    f("fstrip", T, ""),
    f("grassww", T, ""),
    f("bmpuser", T, ""),
];

/// `cntable.lum`: `curvenumber_table` (`cntbl_read`): curve numbers for hydrologic groups A–D.
/// The Fortran default is (30, 55, 70, 77); rows give all four.
pub const CNTABLE: &[Field] = &[f("name", T, ""), a("cn", R, "0.", 4)];

/// `ovn_table.lum`: `overlandflow_n_table` (`overland_n_read`).
pub const OVN_TABLE: &[Field] = &[f("name", T, ""), f("ovn", R, "0.5"), f("ovn_min", R, "0.5"), f("ovn_max", R, "0.5")];

/// `cons_practice.lum`: `conservation_practice_table` (`cons_prac_read`).
pub const CONS_PRACTICE: &[Field] = &[f("name", T, ""), f("pfac", R, "1.0"), f("sl_len_mx", R, "1.0")];

/// `snow.sno`: `snow_parameters` (`snowdb_read`).
pub const SNOW: &[Field] = &[
    f("name", T, ""),
    f("falltmp", R, "0."),
    f("melttmp", R, "0.5"),
    f("meltmx", R, "4.5"),
    f("meltmn", R, "0.5"),
    f("timp", R, "0.8"),
    f("covmx", R, "25.0"),
    f("cov50", R, "0.5"),
    f("init_mm", R, "0."),
];

/// `field.fld`: `field` (`field_read`).
pub const FIELD: &[Field] = &[f("name", T, "default"), f("length", R, "500."), f("wid", R, "100."), f("ang", R, "30.")];

/// `tillage.til`: `tillage_db` (`till_parm_read`). The row named `biomix` sets the basin's
/// biological mixing (`bmix_eff`, `bmix_depth`).
pub const TILLAGE: &[Field] = &[
    f("tillnm", T, " "),
    f("effmix", R, "0."),
    f("deptil", R, "0."),
    f("ranrns", R, "0."),
    f("ridge_ht", R, "0."),
    f("ridge_sp", R, "0."),
];

/// `carbon.bsn`: the basin carbon scalars `carbon_bsn_read` reads in one record when `codes.bsn`
/// turns carbon on (`cswat` 2). Defaults are the Fortran variables' initial values.
pub const CARBON_BSN: &[Field] = &[
    f("frac_seq", R, ".95"),
    f("frac_hum_microb", R, ".02"),
    f("frac_hum_slow", R, ".54"),
    f("frac_hum_passive", R, ".44"),
    f("prmt_21", R, "1000."),
    f("prmt_44", R, ".5"),
    f("till_eff_days", I, "30"),
    f("rtof", R, ".5"),
    f("bio_consf", R, ".15"),
    f("till_consf", R, ".10"),
    f("tmpf", I, "2"),
    f("watf", I, "1"),
    f("tn", R, "-5."),
    f("top", R, "30."),
    f("tx", R, "50."),
    f("bmix_a", R, "3.0"),
    f("bmix_b", R, "0."),
    f("bmix_c", R, "0."),
    f("tillmix_a", R, "3.0"),
    f("tillmix_b", R, "0."),
    f("tillmix_c", R, "0."),
    f("photo_degrade_factor", R, "0."),
    f("n_act_frac", R, ".02"),
    f("cnr_cap", R, "500."),
    f("cnr_ref", R, "25."),
    f("cpr_cap", R, "5000."),
    f("cpr_ref", R, "200."),
    f("mathers_int", I, "0"),
];

/// `plants.plt`: `plant_db` (`plant_parm_read`), one row per plant, read as the whole type (the
/// description at the end of a row is left over and skipped). The last three are
/// `res_part_fracs`: the file's `avg_lig_frac`, `ab_lig_frac`, `bg_lig_frac`.
pub const PLANTS: &[Field] = &[
    f("plantnm", T, ""),
    f("typ", T, ""),
    f("trig", T, ""),
    f("nfix_co", R, "0."),
    f("days_mat", I, "110"),
    f("bio_e", R, "15.0"),
    f("hvsti", R, "0.76"),
    f("blai", R, "5.0"),
    f("frgrw1", R, "0.05"),
    f("laimx1", R, "0.05"),
    f("frgrw2", R, "0.4"),
    f("laimx2", R, "0.95"),
    f("dlai", R, "0.99"),
    f("dlai_rate", R, "1."),
    f("chtmx", R, "6.0"),
    f("rdmx", R, "3.5"),
    f("t_opt", R, "30."),
    f("t_base", R, "10."),
    f("cnyld", R, "0.0015"),
    f("cpyld", R, "0.0003"),
    f("pltnfr1", R, "0.006"),
    f("pltnfr2", R, "0.002"),
    f("pltnfr3", R, "0.0015"),
    f("pltpfr1", R, "0.0007"),
    f("pltpfr2", R, "0.0004"),
    f("pltpfr3", R, "0.0003"),
    f("wsyf", R, "0.01"),
    f("usle_c", R, "0.001"),
    f("gsi", R, "0.002"),
    f("vpdfr", R, "4."),
    f("gmaxfr", R, "0.75"),
    f("wavp", R, "8."),
    f("co2hi", R, "660."),
    f("bioehi", R, "16."),
    f("rsdco_pl", R, "0.05"),
    f("alai_min", R, "0.75"),
    f("laixco_tree", R, "0.3"),
    f("mat_yrs", I, "10"),
    f("bmx_peren", R, "1000."),
    f("ext_coef", R, "0.65"),
    f("leaf_tov_min", R, "12."),
    f("leaf_tov_max", R, "3."),
    f("bm_dieoff", R, "0."),
    f("rsr1", R, "0."),
    f("rsr2", R, "0."),
    f("pop1", R, "0."),
    f("frlai1", R, "0."),
    f("pop2", R, "0."),
    f("frlai2", R, "0."),
    f("frsw_gro", R, ".5"),
    f("aeration", R, "0.2"),
    f("rsd_pctcov", R, "0."),
    f("rsd_covfac", R, "0."),
    f("meta_frac", R, "0.85"),
    f("str_frac", R, "0.15"),
    f("lig_frac", R, "0.12"),
];

/// `nutrients.sol`: `soiltest_db` (`solt_db_read`), initial soil nutrients by name.
pub const SOILTEST: &[Field] = &[
    f("name", T, "default"),
    f("exp_co", R, ".001"),
    f("lab_p", R, "5."),
    f("nitrate", R, "7."),
    f("fr_hum_act", R, ".02"),
    f("hum_c_n", R, "10."),
    f("hum_c_p", R, "80."),
    f("inorgp", R, "0.5"),
    f("watersol_p", R, ".15"),
    f("h3a_p", R, ".25"),
    f("mehlich_p", R, "1.2"),
    f("bray_strong_p", R, ".85"),
];

/// `carbon_lyr.bsn`: per-layer rates of the carbon model (`carbon_bsn_read`): layer 1 for the
/// top soil layer, 2 for the others.
pub const CARBON_LYR: &[Field] = &[
    f("layer", I, "0"),
    f("hp_rate", R, "0."),
    f("hs_rate", R, "0."),
    f("microb_rate", R, "0."),
    f("meta_rate", R, "0."),
    f("str_rate", R, "0."),
    f("microb_top_rate", R, "0."),
    f("hs_hp", R, "0."),
    f("a1co2", R, "0."),
    f("asco2", R, "0."),
    f("apco2", R, "0."),
    f("abco2", R, "0."),
];

/// `harv.ops`: `harvest_operation` (`mgt_read_harvops`).
pub const HARV_OPS: &[Field] = &[f("name", T, ""), f("typ", T, ""), f("hi_ovr", R, "0."), f("eff", R, "0."), f("bm_min", R, "0")];

/// `fertilizer.frt`: `fertilizer_db` (`fert_parm_read`). The name is 16 characters in SWAT+.
pub const FERTILIZER: &[Field] =
    &[f("fertnm", T, " "), f("fminn", R, "0."), f("fminp", R, "0."), f("forgn", R, "0."), f("forgp", R, "0."), f("fnh3n", R, "0.")];

/// `irr.ops`: `irrigation_operation` (`mgt_read_irrops`).
pub const IRR_OPS: &[Field] = &[
    f("name", T, ""),
    f("amt_mm", R, "25.4"),
    f("eff", R, "0."),
    f("surq", R, "0."),
    f("dep_mm", R, "0."),
    f("salt", R, "0."),
    f("no3", R, "0."),
    f("po4", R, "0."),
];

/// `chem_app.ops`: `chemical_application_operation` (`mgt_read_chemapp`).
pub const CHEM_APP: &[Field] = &[
    f("name", T, ""),
    f("form", T, " "),
    f("op_typ", T, " "),
    f("app_eff", R, "0."),
    f("foliar_eff", R, "0."),
    f("inject_dep", R, "0."),
    f("surf_frac", R, "0."),
    f("drift_pot", R, "0."),
    f("aerial_unif", R, "0."),
];

/// `hyd-sed-lte.cha`: `swatdeg_hydsed_data` (`sd_hydsed_read`), a SWAT-DEG channel's geometry,
/// hydraulics and bank and bed properties. The file's `erod_fact`, `cov_fact` and `eq_slp` are
/// `bank_exp`, `cov` and `vcr_coef`; the description at the end of a row is skipped.
pub const HYD_SED_LTE: &[Field] = &[
    f("name", T, ""),
    f("order", I, "0"),
    f("chw", R, "0."),
    f("chd", R, "0."),
    f("chs", R, "0."),
    f("chl", R, "0."),
    f("chn", R, "0."),
    f("chk", R, "0."),
    f("bank_exp", R, "0."),
    f("cov", R, "0."),
    f("sinu", R, "0."),
    f("vcr_coef", R, "0."),
    f("d50", R, "0."),
    f("ch_clay", R, "0."),
    f("carbon", R, "0."),
    f("ch_bd", R, "0."),
    f("chss", R, "0."),
    f("bankfull_flo", R, "0."),
    f("fps", R, "0.000001"),
    f("fpn", R, "0.1"),
    f("n_conc", R, "0."),
    f("p_conc", R, "0."),
    f("p_bio", R, "0."),
];

/// `nutrients.cha`: `channel_nut_data` (`ch_read_nut`), QUAL2E rates and coefficients.
pub const CH_NUT: &[Field] = &[
    f("name", T, ""),
    f("onco", R, "0."),
    f("opco", R, "0."),
    f("rs1", R, "1."),
    f("rs2", R, ".05"),
    f("rs3", R, ".5"),
    f("rs4", R, ".05"),
    f("rs5", R, ".05"),
    f("rs6", R, "2.5"),
    f("rs7", R, "2.5"),
    f("rk1", R, "1.71"),
    f("rk2", R, "1."),
    f("rk3", R, "2."),
    f("rk4", R, "0."),
    f("rk5", R, "1.71"),
    f("rk6", R, "1.71"),
    f("bc1", R, ".55"),
    f("bc2", R, "1.1"),
    f("bc3", R, ".21"),
    f("bc4", R, ".35"),
    f("lao", R, "2"),
    f("igropt", I, "2"),
    f("ai0", R, "50."),
    f("ai1", R, "0.08"),
    f("ai2", R, "0.015"),
    f("ai3", R, "1.60"),
    f("ai4", R, "2.0"),
    f("ai5", R, "3.5"),
    f("ai6", R, "1.07"),
    f("mumax", R, "2.0"),
    f("rhoq", R, "2.5"),
    f("tfact", R, "0.3"),
    f("k_l", R, "0.75"),
    f("k_n", R, "0.02"),
    f("k_p", R, "0.025"),
    f("lambda0", R, "1.0"),
    f("lambda1", R, "0.03"),
    f("lambda2", R, "0.054"),
    f("p_n", R, "0.5"),
];

/// `initial.cha` and the other `initial.*` files: the initial organic-mineral and constituent
/// data by name (`ch_init`).
pub const INIT_CHA: &[Field] =
    &[f("name", T, ""), f("org_min", T, ""), f("pest", T, ""), f("path", T, ""), f("hmet", T, ""), f("salt", T, "")];

/// `om_water.ini`: initial water and its organic-mineral content (`om_init_water`, a
/// `hyd_output` by name).
pub const OM_WATER: &[Field] = &[
    f("name", T, ""),
    f("flo", R, "0."),
    f("sed", R, "0."),
    f("orgn", R, "0."),
    f("sedp", R, "0."),
    f("no3", R, "0."),
    f("solp", R, "0."),
    f("chla", R, "0."),
    f("nh3", R, "0."),
    f("no2", R, "0."),
    f("cbod", R, "0."),
    f("dox", R, "0."),
    f("san", R, "0."),
    f("sil", R, "0."),
    f("cla", R, "0."),
    f("sag", R, "0."),
    f("lag", R, "0."),
    f("grv", R, "0."),
    f("temp", R, "0."),
];

/// `weir.res` (`reservoir_weir_outflow`): the weir's discharge coefficients, width and height
/// (m).
pub const WEIR_RES: &[Field] = &[f("name", T, ""), f("c", R, "1.84"), f("k", R, "2.6"), f("w", R, "2.5"), f("h", R, "0.")];

/// `puddle.ops` (`puddle_operation`): the top layer's conductivity (mm/h) and the water's
/// concentrations (ppm) after puddling.
pub const PUDDLE_OPS: &[Field] = &[
    f("name", T, ""),
    f("wet_hc", R, "0."),
    f("sed", R, "0."),
    f("orgn", R, "0."),
    f("sedp", R, "0."),
    f("no3", R, "0."),
    f("solp", R, "0."),
    f("nh3", R, "0."),
    f("no2", R, "0."),
];

/// `manure_db.frt` (`manure_database`).
pub const MANURE_DB: &[Field] = &[
    f("name", T, ""),
    f("org_min", T, ""),
    f("pests", T, ""),
    f("paths", T, ""),
    f("hmets", T, ""),
    f("salts", T, ""),
    f("constit", T, ""),
];

/// `manure_om.frt` (`manure_attributes`).
pub const MANURE_OM: &[Field] = &[
    f("name", T, " "),
    f("frac_water", R, "0."),
    f("fcbn", R, "0."),
    f("fminn", R, "0."),
    f("fminp", R, "0."),
    f("forgn", R, "0."),
    f("forgp", R, "0."),
    f("fnh3n", R, "0."),
];

/// `transplant.ops` (`transplant_operation`), only its names so far: a transplant isn't ported.
pub const TRANSPLANT: &[Field] = &[f("name", T, "")];

/// `reservoir.res` (`reservoir_data_char_input`, after the row number).
pub const RES_DAT: &[Field] =
    &[f("id", I, "0"), f("name", T, "default"), f("init", T, ""), f("hyd", T, ""), f("rel", T, ""), f("sed", T, ""), f("nut", T, "")];

/// `hydrology.res` (`reservoir_hyd_data`): areas (ha) and volumes (ha-m) at the principal and
/// emergency spillways.
pub const RES_HYD: &[Field] = &[
    f("name", T, "default"),
    f("iyres", I, "0"),
    f("mores", I, "0"),
    f("psa", R, "0."),
    f("pvol", R, "0."),
    f("esa", R, "0."),
    f("evol", R, "0."),
    f("k", R, ".01"),
    f("evrsv", R, ".7"),
    f("br1", R, "0."),
    f("br2", R, "0."),
];

/// `filterstrip.str` (`filtstrip_operation`).
pub const FSTRIP: &[Field] = &[f("name", T, ""), f("flag", I, "0"), f("fld_vfs", R, "0."), f("con_vfs", R, "0."), f("cha_q", R, "0.")];

/// `grassedww.str` (`grwaterway_operation`).
pub const GRASSWW: &[Field] = &[
    f("name", T, ""),
    f("flag", I, "0"),
    f("mann", R, "0."),
    f("sed_co", R, "0."),
    f("dp", R, "0."),
    f("wd", R, "0."),
    f("len", R, "0."),
    f("slp", R, "0."),
];

/// `bmpuser.str` (`bmpuser_operation`): removal shares.
pub const BMPUSER: &[Field] = &[
    f("name", T, ""),
    f("flag", I, "0"),
    f("sed_eff", R, "0."),
    f("ptlp_eff", R, "0."),
    f("solp_eff", R, "0."),
    f("ptln_eff", R, "0."),
    f("soln_eff", R, "0."),
    f("bact_eff", R, "0."),
];

/// `tiledrain.str` (`subsurface_drainage_parameters`).
pub const TILEDRAIN: &[Field] = &[
    f("name", T, "null"),
    f("depth", R, "0."),
    f("time", R, "0."),
    f("lag", R, "0."),
    f("radius", R, "0."),
    f("dist", R, "0."),
    f("drain_co", R, "0."),
    f("pumpcap", R, "0."),
    f("latksat", R, "0."),
];
