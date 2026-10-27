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
    // no aquifers SWAT+ never opens it; neither does the loader. nutrients.sol is named but not
    // there: SWAT+ then uses its default soil nutrients. Nor is chem_app.ops: fertilizer then
    // goes on with element 0 of the chemical applications, all zero. Nor is urban.urb: no urban
    // land types then. Nor is bmpuser.str: no user-defined practices.
    assert_eq!(report.missing, ["nutrients.sol", "urban.urb", "chem_app.ops", "bmpuser.str"]);
    let mgt = p.management.iter().find(|m| m.name == "mgt_01").unwrap();
    assert_eq!(mgt.autos, ["irr_year_irr"]);
    assert_eq!(mgt.ops.len(), 305);
    assert_eq!((mgt.ops[0].op.as_str(), mgt.ops[0].mon, mgt.ops[0].day), ("till", 4, 1));
    assert_eq!(mgt.ops[0].op_char, "chisplow");
    assert!(p.harv_ops.as_ref().unwrap().find("grain").is_some());
    assert_eq!(p.soil_lyr_depths.as_deref().map(|d| (d.len(), d[0], d[d.len() - 1])), Some((16, 10, 2100)));
    let dt = &p.lum_dtl[0];
    assert_eq!((dt.name.as_str(), dt.conds.len(), dt.alts, dt.acts.len()), ("irr_year_irr", 4, 1, 1));
    assert_eq!((dt.conds[2].var.as_str(), dt.conds[2].lim_const, dt.alt[2][0].as_str()), ("phu_plant", 0.2, ">"));
    assert_eq!((dt.acts[0].option.as_str(), dt.acts[0].const_, dt.acts[0].const2), ("sprinkler_ilm", 25.0, 20.0));
    assert_eq!(p.lum_dtl[1].acts[0].ob, "unlim");
    assert!(p.fertilizer.as_ref().unwrap().find("elem_n").is_some());
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

#[test]
fn ames_hru_inputs() {
    let (p, _) = load_project(&scenario("Ames_sub1")).unwrap();
    assert_eq!(p.hru_data.len(), 12);
    let h1 = &p.hru_data[0];
    assert_eq!((h1.id, h1.topo.as_str(), h1.soil.as_str(), h1.land_use_mgt.as_str()), (1, "topohru0001", "soil_01-h1", "cosy_lum"));

    // 14 soils; the texture "L- L- L- L" reads as its first word.
    assert_eq!(p.soils.len(), 14);
    let s = p.soils.iter().find(|s| s.name == "soil_01-h1").unwrap();
    assert_eq!((s.nly, s.hydgrp.as_str(), s.zmx), (4, "B", 2000.0));
    assert_eq!(p.soils[0].texture, "L-");
    assert_eq!(p.soils[0].layers.len(), 4);
    assert_eq!((p.soils[0].layers[0].z, p.soils[0].layers[0].k), (50.0, 33.01));

    // parameters.bsn writes cn_froz as 0; basin_prm_default makes it 0.000862.
    assert_eq!(p.params.get("cn_froz"), Some(0.000862));
    assert_eq!(p.params.get("ffcb"), Some(0.5));
    // Biological mixing: the biomix row of tillage.til and carbon.bsn's temperature factor.
    let till = p.tillage.as_ref().unwrap();
    let r = till.find("biomix").unwrap();
    assert_eq!((till.real(r, "effmix"), till.real(r, "deptil")), (Some(0.01), Some(50.0)));
    let c = p.carbon.as_ref().unwrap();
    assert_eq!((c.int(0, "tmpf"), c.real(0, "tn"), c.real(0, "top"), c.real(0, "tx")), (Some(2), Some(-0.5), Some(30.0), Some(50.0)));
    assert_eq!(c.int(0, "mathers_int"), Some(1));

    // Plants and plant communities.
    let plt = p.plants.as_ref().unwrap();
    let r = plt.find("past").unwrap();
    assert_eq!((plt.text(r, "typ"), plt.text(r, "trig")), (Some("perennial"), Some("temp_gro")));
    assert_eq!((plt.int(r, "days_mat"), plt.real(r, "bio_e"), plt.int(r, "mat_yrs")), (Some(0), Some(35.0), Some(2)));
    assert_eq!(plt.real(r, "lig_frac"), Some(0.235));
    let com = p.plant_communities.iter().find(|c| c.name == "past_comm").unwrap();
    assert_eq!(com.plants.len(), 1);
    let pl = &com.plants[0];
    assert_eq!((pl.cpnm.as_str(), pl.igro.as_str(), pl.bioms, pl.fr_yrmat, pl.rsdin), ("past", "y", 20000.0, 1.0, 10000.0));
    assert_eq!(p.plant_communities.iter().find(|c| c.name == "cosy_comm").map(|c| c.plants.len()), Some(2));
    assert!(p.soil_test.is_none());
    assert!(!p.co2_yr_dat);

    let topo = p.topography.as_ref().unwrap();
    let r = topo.find("topohru0001").unwrap();
    assert_eq!((topo.real(r, "slope"), topo.real(r, "slope_len")), (Some(0.025), Some(91.463)));
    let hyd = p.hydrology.as_ref().unwrap();
    let r = hyd.find("hyd0001").unwrap();
    assert_eq!((hyd.real(r, "esco"), hyd.real(r, "perco"), hyd.real(r, "latq_co")), (Some(0.95), Some(0.5), Some(0.01)));
    let lu = p.landuse.as_ref().unwrap();
    let r = lu.find("cosy_lum").unwrap();
    assert_eq!(
        (lu.text(r, "plant_cov"), lu.text(r, "mgt_ops"), lu.text(r, "cn_lu")),
        (Some("cosy_comm"), Some("mgt_01"), Some("rc_strow_g"))
    );
    let cn = p.cntable.as_ref().unwrap();
    let r = cn.find("fal_bare").unwrap();
    assert_eq!(cn.get(r, "cn", 3).and_then(|v| v.as_real()), Some(94.0));
    let sno = p.snow.as_ref().unwrap();
    assert_eq!(sno.real(0, "meltmn"), Some(4.5));
    assert_eq!(p.soil_plant_init.first().map(|s| (s.name.as_str(), s.sw_frac, s.nutc.as_str())), Some(("soilplant1", 0.5, "soilnut1")));
}

#[test]
fn osu_hru_inputs() {
    let (p, _) = load_project(&scenario("Osu_1hru")).unwrap();
    assert_eq!(p.hru_data.len(), 1);
    assert_eq!(p.hru_data[0].surf_stor, "paddy0001");
    let s = p.soils.iter().find(|s| s.name == p.hru_data[0].soil).unwrap();
    assert!(s.nly > 0 && !s.layers.is_empty());
}
