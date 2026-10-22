//! Loading a whole scenario: `file.cio` first, then the files it names.

use swat_model::{ClimateVar, ObjectKind, Project, StationList};

use crate::source::FileSource;
use crate::{Error, Result, cio, climate, con, print, time};

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
