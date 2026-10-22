//! Loads the SWAT+ reference scenarios in testdata/ and checks what they hold.

use std::path::PathBuf;

use swat_io::{DirSource, load_project};
use swat_model::{ClimateVar, ObjectKind};

fn scenario(name: &str) -> DirSource {
    DirSource::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata").join(name))
}

#[test]
fn osu_1hru() {
    let (p, report) = load_project(&scenario("Osu_1hru")).unwrap();
    assert!(report.missing.is_empty(), "missing: {:?}", report.missing);

    assert_eq!((p.time.yrc_start, p.time.yrc_end, p.time.day_start, p.time.day_end), (2010, 2023, 1, 365));
    assert_eq!(p.time.days().len(), 14 * 365 + 3);
    assert_eq!(p.print.nyskip, 3);
    // day_lag_mx is written 0.00000: not an integer, so the read stops and igen stays 5.
    assert_eq!(p.params.igen, 5);
    assert_eq!(p.params.adj_pkr(), 1.0);
    assert_eq!(p.codes.pet, 1);
    assert_eq!(p.print.object("hru_wb").map(|o| o.avann.as_str()), Some("y"));

    // object.cnt: 1 HRU, 1 routing unit, 1 aquifer, 1 SWAT-DEG channel.
    assert_eq!(p.objects_of(ObjectKind::Hru).count(), 1);
    assert_eq!(p.objects_of(ObjectKind::RoutingUnit).count(), 1);
    assert_eq!(p.objects_of(ObjectKind::Aquifer).count(), 1);
    assert_eq!(p.objects_of(ObjectKind::ChannelDeg).count(), 1);
    let ru = p.objects_of(ObjectKind::RoutingUnit).next().unwrap();
    assert_eq!(ru.outflows.len(), 2);
    assert_eq!((ru.outflows[0].obj_type.as_str(), ru.outflows[0].hyd_type.as_str()), ("sdc", "tot"));
    assert_eq!((ru.outflows[1].obj_type.as_str(), ru.outflows[1].hyd_type.as_str()), ("aqu", "rhg"));
    let hru = p.objects_of(ObjectKind::Hru).next().unwrap();
    assert_eq!(hru.area_ha, 10.0);
    assert_eq!(hru.weather_station, "s35610n127290e");

    assert_eq!(p.stations.len(), 1);
    assert_eq!(p.stations[0].hmd.as_deref(), Some("sim"));
    assert_eq!(p.wgn.len(), 1);
    assert_eq!(p.wgn[0].name, "Imsil");
    assert_eq!(p.wgn[0].months[0].tmp_max_ave, 3.79032);

    let pcp = p.series.iter().find(|s| s.var == ClimateVar::Pcp).unwrap();
    assert_eq!(pcp.nbyr, 42);
    assert_eq!(pcp.records.first().map(|r| (r.year, r.jday, r.values[0])), Some((1981, 1, 11.4)));
    let tmp = p.series.iter().find(|s| s.var == ClimateVar::Tmp).unwrap();
    assert_eq!(tmp.records.first().map(|r| r.values), Some([2.7, -17.5]));
    // The header says 42 years, but the file holds 43: one record per day, 1981 to 2023.
    let days: usize = (1981..=2023).map(|y| if swat_model::time::is_leap(y) { 366 } else { 365 }).sum();
    assert_eq!(pcp.records.len(), days);
    assert_eq!(pcp.records.last().map(|r| (r.year, r.jday)), Some((2023, 365)));
}

#[test]
fn ames_sub1() {
    let (p, report) = load_project(&scenario("Ames_sub1")).unwrap();
    // file.cio names an aquifer file "nbull" (a typo for null). It isn't there, and as there are
    // no aquifers SWAT+ never opens it; neither does the loader.
    assert!(report.missing.is_empty(), "missing: {:?}", report.missing);
    assert_eq!(p.cio.file("aquifer", "init"), Some("nbull"));
    // The init row lists 12 names for 11 slots; the extra one is ignored.
    assert_eq!(p.cio.file("init", "soil_plant_ini"), Some("soil_plant.ini"));

    // day_start and day_end 0: whole years 1975–2020.
    assert_eq!(p.time.normalized().day_start, 1);
    assert_eq!(p.time.days().first().map(|d| (d.year, d.jday)), Some((1975, 1)));
    assert_eq!(p.time.days().last().map(|d| (d.year, d.jday)), Some((2020, 366)));

    assert_eq!(p.objects_of(ObjectKind::Hru).count(), 12);
    assert!(p.objects.iter().all(|o| o.kind == ObjectKind::Hru && o.outflows.is_empty()));

    // tmp.cli lists ames.tmp, which is empty: SWAT+ reads no measured temperature. The station
    // names ames.tem (which has the data) as its gage, which SWAT+ doesn't find in that list.
    assert_eq!(report.unread, vec!["ames.tmp".to_string()]);
    assert!(p.series.iter().all(|s| s.var != ClimateVar::Tmp));
    let tmp_list = p.station_lists.iter().find(|l| l.var == ClimateVar::Tmp).unwrap();
    assert_eq!(tmp_list.files, vec!["ames.tmp".to_string()]);
    assert_eq!(p.stations[0].tmp.as_deref(), Some("ames.tem"));

    // ames.pcp has CRLF line ends and -99 for missing values.
    let pcp = p.series.iter().find(|s| s.var == ClimateVar::Pcp).unwrap();
    assert_eq!(pcp.nbyr, 121);
    assert_eq!(pcp.records.first().map(|r| (r.year, r.jday, r.values[0])), Some((1900, 1, -99.0)));
}
