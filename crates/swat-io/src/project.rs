//! Loading a whole scenario: `file.cio` first, then the files it names.

use swat_model::schema;
use swat_model::{ClimateVar, ObjectKind, Project, StationList};

use crate::source::FileSource;
use crate::{Error, Result, basin, cio, climate, con, hru, print, table, time};

/// What loading found wrong without failing, as SWAT+ goes on in those cases too.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadReport {
    /// Files `file.cio` or a station list names that aren't in the scenario. SWAT+ checks the
    /// files `file.cio` names with `inquire (exist=…)` and skips them.
    pub missing: Vec<String>,
    /// Measured station files that ended before their header was complete (empty files, say).
    /// SWAT+ stops reading that variable's stations there: the files after it in the list are
    /// not read either, and are listed here too.
    pub unread: Vec<String>,
}

/// Loads the parts of a scenario the readers cover so far: `file.cio`, `time.sim`, `print.prt`,
/// the connection files, weather stations, the weather generator and the measured series.
pub fn load_project(src: &dyn FileSource) -> Result<(Project, LoadReport)> {
    let mut report = LoadReport::default();
    let cio_text = src.read("file.cio")?.ok_or_else(|| crate::Error::Missing("file.cio".into()))?;
    let cio = cio::read(&cio_text)?;
    let mut project = Project { cio, ..Project::default() };

    // Reads the file in `section`/`slot`, noting it when file.cio names it but it isn't there.
    let mut late_missing: Vec<String> = Vec::new();
    let mut fetch = |section: &str, slot: &str| -> Result<Option<(String, String)>> {
        let Some(name) = project.cio.file(section, slot).map(str::to_string) else {
            return Ok(None);
        };
        match src.read(&name)? {
            Some(text) => Ok(Some((name, text))),
            None => {
                report.missing.push(name);
                Ok(None)
            }
        }
    };

    if let Some((_, text)) = fetch("basin", "codes_bas")? {
        project.codes = basin::read_codes(&text)?;
    }
    if let Some((_, text)) = fetch("basin", "parms_bas")? {
        project.params = basin::read_params(&text)?;
    }
    project.params.complete();
    if let Some((_, text)) = fetch("simulation", "time")? {
        project.time = time::read(&text)?;
    }
    if let Some((_, text)) = fetch("simulation", "prt")? {
        project.print = print::read(&text)?;
    }
    // basin_read_objs: the basin's name and areas, and how many objects of each kind
    // hyd_connect reads (`sp_ob`).
    let mut counts: Option<Vec<i64>> = None;
    if let Some((name, text)) = fetch("simulation", "object_cnt")? {
        let mut rec = crate::list::Records::new(&name, &text);
        rec.skip()?;
        rec.skip()?;
        if let Ok(f) = rec.read(19) {
            project.basin_name = f.text_or(0, "");
            project.area_ls_ha = f.real_or(1, 0.0)?;
            project.area_tot_ha = f.real_or(2, 0.0)?;
            counts = Some((4..19).map(|k| f.int_or(k, 0).unwrap_or(0) as i64).collect());
        }
    }
    for kind in ObjectKind::ALL {
        // sp_ob: hru, hru_lte, ru, gwflow, aqu, chan, res, recall, exco, dr, canal, pump,
        // outlet, chandeg, aqu2d.
        let k = match kind {
            ObjectKind::Hru => Some(0),
            ObjectKind::HruLte => Some(1),
            ObjectKind::RoutingUnit => Some(2),
            ObjectKind::Gwflow => Some(3),
            ObjectKind::Aquifer => Some(4),
            ObjectKind::Channel => Some(5),
            ObjectKind::Reservoir => Some(6),
            ObjectKind::Recall => Some(7),
            ObjectKind::ExportCoefficient => Some(8),
            ObjectKind::DeliveryRatio => Some(9),
            ObjectKind::Outlet => Some(12),
            ObjectKind::ChannelDeg => Some(13),
            ObjectKind::Aquifer2d => Some(14),
        };
        let n = match (&counts, k) {
            (Some(c), Some(k)) => Some(c.get(k).copied().unwrap_or(0).max(0) as usize),
            _ => None,
        };
        if n == Some(0) {
            continue;
        }
        if let Some((name, text)) = fetch("connect", kind.cio_slot())? {
            let mut obs = con::read(kind, &name, &text)?;
            if let Some(n) = n {
                obs.truncate(n);
            }
            project.objects.extend(obs);
        }
    }
    if let Some((name, text)) = fetch("climate", "weat_sta")? {
        project.stations = climate::read_stations(&name, &text)?;
    }
    if let Some((name, text)) = fetch("climate", "weat_wgn")? {
        project.wgn = climate::read_wgn(&name, &text)?;
    }
    // HRUs and the databases they point to (`hru_read` and the readers before it).
    let tables: [(&str, &str, &'static [swat_model::table::Field]); 12] = [
        ("hydrology", "topogr_hyd", schema::TOPOGRAPHY),
        ("hydrology", "hydrol_hyd", schema::HYDROLOGY),
        ("hydrology", "field_fld", schema::FIELD),
        ("lum", "landuse_lum", schema::LANDUSE),
        ("lum", "cntable_lum", schema::CNTABLE),
        ("lum", "ovn_lum", schema::OVN_TABLE),
        ("lum", "cons_prac_lum", schema::CONS_PRACTICE),
        ("hru_parm_db", "snow", schema::SNOW),
        ("hru_parm_db", "till_til", schema::TILLAGE),
        ("hru_parm_db", "plants_plt", schema::PLANTS),
        ("soils", "nut_sol", schema::SOILTEST),
        ("hru_parm_db", "urban_urb", schema::URBAN),
    ];
    let mut read = Vec::new();
    for (section, slot, fields) in tables {
        read.push(match fetch(section, slot)? {
            Some((name, text)) => Some(table::read_table(&name, &text, fields)?),
            None => None,
        });
    }
    let mut it = read.into_iter();
    project.topography = it.next().flatten();
    project.hydrology = it.next().flatten();
    project.field = it.next().flatten();
    project.landuse = it.next().flatten();
    project.cntable = it.next().flatten();
    project.ovn_table = it.next().flatten();
    project.cons_practice = it.next().flatten();
    project.snow = it.next().flatten();
    project.tillage = it.next().flatten();
    project.plants = it.next().flatten();
    project.soil_test = it.next().flatten();
    project.urban = it.next().flatten();
    // proc_db: the operation databases, then the schedules that point into them.
    let ops: [(&str, &str, &'static [swat_model::table::Field]); 4] = [
        ("hru_parm_db", "fert_frt", schema::FERTILIZER),
        ("ops", "irr_ops", schema::IRR_OPS),
        ("ops", "chem_ops", schema::CHEM_APP),
        ("ops", "harv_ops", schema::HARV_OPS),
    ];
    let mut read = Vec::new();
    for (section, slot, fields) in ops {
        read.push(match fetch(section, slot)? {
            Some((name, text)) => Some(table::read_table(&name, &text, fields)?),
            None => None,
        });
    }
    let mut it = read.into_iter();
    project.fertilizer = it.next().flatten();
    project.irr_ops = it.next().flatten();
    project.chem_app = it.next().flatten();
    project.harv_ops = it.next().flatten();
    if let Some((name, text)) = fetch("decision_table", "dtbl_lum")? {
        project.lum_dtl = hru::read_decision_tables(&name, &text)?;
    }
    if let Some((name, text)) = fetch("decision_table", "dtbl_res")? {
        project.res_dtl = hru::read_res_decision_tables(&name, &text)?;
    }
    // proc_res: res_read_init, res_read_sed, res_read_nut, wet_read, wet_read_hyd.
    let res_tables: [(&str, &'static [swat_model::table::Field]); 5] = [
        ("init_res", schema::RES_INIT),
        ("sed_res", schema::RES_SED),
        ("nut_res", schema::RES_NUT),
        ("wet", schema::WETLAND),
        ("hyd_wet", schema::WET_HYD),
    ];
    let mut read = Vec::new();
    for (slot, fields) in res_tables {
        read.push(match fetch("reservoir", slot)? {
            Some((name, text)) => Some(table::read_table(&name, &text, fields)?),
            None => None,
        });
    }
    let mut it = read.into_iter();
    project.res_init = it.next().flatten();
    project.res_sed = it.next().flatten();
    project.res_nut = it.next().flatten();
    project.wetland = it.next().flatten();
    project.wet_hyd = it.next().flatten();
    for (slot, fields, dst) in [("res", schema::RES_DAT, &mut project.res_dat), ("hyd_res", schema::RES_HYD, &mut project.res_hyd)] {
        if let Some((name, text)) = fetch("reservoir", slot)? {
            *dst = Some(table::read_table(&name, &text, fields)?);
        }
    }
    if let Some((name, text)) = fetch("recall", "recall_rec")? {
        project.recalls = crate::recall::read_recalls(src, &name, &text)?;
    }
    project.weir_res = match fetch("reservoir", "weir_res")? {
        Some((name, text)) => Some(table::read_table(&name, &text, schema::WEIR_RES)?),
        None => None,
    };
    if let Some((_, text)) = fetch("lum", "management_sch")? {
        project.management = hru::read_management(&text)?;
    }
    if let Some((_, text)) = fetch("init", "plant")? {
        project.plant_communities = hru::read_plant_communities(&text)?;
    }
    // lsu_read_elements: landscape units.
    if let Some((name, text)) = fetch("regions", "def_lsu")? {
        project.lsu_defs = crate::routing::read_lsu_defs(&name, &text)?;
    }
    if let Some((name, text)) = fetch("regions", "ele_lsu")? {
        project.lsu_elems = crate::routing::read_lsu_elements(&name, &text)?;
    }
    project.aqu_elems = fetch("regions", "ele_aqu")?.is_some();
    // hyd_connect: ru_read and ru_read_elements; proc_aqu: aqu_read.
    if let Some((name, text)) = fetch("routing_unit", "ru_ele")? {
        project.ru_elements = crate::routing::read_ru_elements(&name, &text)?;
    }
    if let Some((name, text)) = fetch("routing_unit", "ru")? {
        project.ru_data = crate::routing::read_ru_data(&name, &text)?;
    }
    let n_ru = project.objects_of(swat_model::ObjectKind::RoutingUnit).count();
    if let Some((name, text)) = fetch("routing_unit", "ru_def")? {
        project.ru_defs = crate::routing::read_ru_defs(&name, &text, n_ru)?;
    }
    if let Some((name, text)) = fetch("aquifer", "aqu")? {
        project.aquifer_db = crate::routing::read_aquifers(&name, &text)?;
    }
    // proc_cha: sd_hydsed_read, ch_read_nut, ch_read_init, sd_channel_read.
    if let Some((name, text)) = fetch("channel", "hyd_sed")? {
        project.hyd_sed_lte = Some(table::read_table(&name, &text, schema::HYD_SED_LTE)?);
    }
    if let Some((name, text)) = fetch("channel", "nut")? {
        project.ch_nut = Some(table::read_table(&name, &text, schema::CH_NUT)?);
    }
    if let Some((name, text)) = fetch("channel", "init")? {
        project.ch_init = Some(table::read_table(&name, &text, schema::INIT_CHA)?);
    }
    if let Some((name, text)) = fetch("init", "om_water")? {
        project.om_water = Some(table::read_table(&name, &text, schema::OM_WATER)?);
    }
    if let Some((name, text)) = fetch("channel", "chan_ez")? {
        project.sd_channel_data = crate::routing::read_sd_channels(&name, &text)?;
    }
    project.ch_temp_file = project.cio.file("channel", "temp").is_some();
    // proc_cal
    if let Some((name, text)) = fetch("chg", "cal_parms")? {
        project.cal_parms = crate::cal::read_cal_parms(&name, &text)?;
    }
    if let Some((name, text)) = fetch("chg", "cal_upd")? {
        project.cal_upd = crate::cal::read_calibration(&name, &text)?;
    }
    project.co2_yr_dat = src.read("co2_yr.dat").ok().flatten().is_some();
    // mgt_read_puddle, manure_orgmin_read, manure_db_read, plant_transplant_read: fixed names.
    for (name, fields, slot) in [
        ("puddle.ops", schema::PUDDLE_OPS, &mut project.puddle_ops),
        ("manure_om.frt", schema::MANURE_OM, &mut project.manure_om),
        ("manure_db.frt", schema::MANURE_DB, &mut project.manure_db),
        ("transplant.ops", schema::TRANSPLANT, &mut project.transplant_ops),
    ] {
        if let Some(text) = src.read(name)? {
            *slot = Some(table::read_table(name, &text, fields)?);
        }
    }
    // soils_init opens it by this name, whatever file.cio says.
    if let Some(text) = src.read("soil_lyr_depths.sol")? {
        project.soil_lyr_depths = Some(hru::read_soil_lyr_depths(&text)?);
    }
    if project.codes.cswat == 2
        && let Some((name, text)) = fetch("basin", "carbon_bsn")?
    {
        project.carbon = Some(table::read_record(&name, &text, schema::CARBON_BSN)?);
        // carbon_bsn_read: the per-layer file is named after carbon.bsn ("x.bsn" -> "x_lyr.bsn").
        let lyr = match name.rfind('.') {
            Some(i) if i > 0 => format!("{}_lyr{}", &name[..i], &name[i..]),
            _ => format!("{name}_lyr.bsn"),
        };
        match src.read(&lyr)? {
            Some(text) => project.carbon_lyr = Some(table::read_table(&lyr, &text, schema::CARBON_LYR)?),
            None => late_missing.push(lyr),
        }
    }
    if let Some((_, text)) = fetch("soils", "soils_sol")? {
        project.soils = hru::read_soils(&text)?;
    }
    if let Some((_, text)) = fetch("init", "soil_plant_ini")? {
        project.soil_plant_init = hru::read_soil_plant_init(&text, project.codes.nam1)?;
    }
    if let Some((_, text)) = fetch("hru", "hru_data")? {
        project.hru_data = hru::read_hru_data(&text)?;
    }

    let mut lists = Vec::new();
    for var in ClimateVar::ALL {
        if let Some((name, text)) = fetch("climate", var.cio_slot())? {
            lists.push((var, climate::read_station_list(&name, &text)?));
        }
    }

    for (var, files) in &lists {
        // The variable's data folder from file.cio, if one is set.
        let folder = var
            .path_section()
            .and_then(|(section, slot)| project.cio.file(section, slot))
            .map(|f| f.trim().to_string())
            .unwrap_or_default();
        // As cli_pmeas and its siblings: a file that is missing (SWAT+ opens it, which creates it
        // empty) or ends inside its header stops the variable's stations there.
        let mut stopped = false;
        for file in files {
            let path = if folder.is_empty() { file.clone() } else { format!("{folder}{file}") };
            if stopped {
                report.unread.push(path);
                continue;
            }
            match src.read(&path)? {
                Some(text) => match climate::read_series(*var, file, &text) {
                    Ok(series) => project.series.push(series),
                    Err(Error::Eof { .. }) => {
                        report.unread.push(path);
                        stopped = true;
                    }
                    Err(e) => return Err(e),
                },
                None => {
                    report.missing.push(path);
                    stopped = true;
                }
            }
        }
    }
    project.station_lists = lists.into_iter().map(|(var, files)| StationList { var, files }).collect();
    report.missing.extend(late_missing);
    Ok((project, report))
}
