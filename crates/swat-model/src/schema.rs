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
