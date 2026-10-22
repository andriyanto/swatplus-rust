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
    if let Some((_, text)) = fetch("simulation", "time")? {
        project.time = time::read(&text)?;
    }
    if let Some((_, text)) = fetch("simulation", "prt")? {
        project.print = print::read(&text)?;
    }
    for kind in ObjectKind::ALL {
        if let Some((name, text)) = fetch("connect", kind.cio_slot())? {
            project.objects.extend(con::read(kind, &name, &text)?);
        }
    }
    if let Some((name, text)) = fetch("climate", "weat_sta")? {
        project.stations = climate::read_stations(&name, &text)?;
    }
    if let Some((name, text)) = fetch("climate", "weat_wgn")? {
        project.wgn = climate::read_wgn(&name, &text)?;
    }
    // HRUs and the databases they point to (`hru_read` and the readers before it).
    let tables: [(&str, &str, &'static [swat_model::table::Field]); 8] = [
        ("hydrology", "topogr_hyd", schema::TOPOGRAPHY),
        ("hydrology", "hydrol_hyd", schema::HYDROLOGY),
        ("hydrology", "field_fld", schema::FIELD),
        ("lum", "landuse_lum", schema::LANDUSE),
        ("lum", "cntable_lum", schema::CNTABLE),
        ("lum", "ovn_lum", schema::OVN_TABLE),
        ("lum", "cons_prac_lum", schema::CONS_PRACTICE),
        ("hru_parm_db", "snow", schema::SNOW),
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
    Ok((project, report))
}
