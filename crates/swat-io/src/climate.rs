//! Climate files: weather stations (`cli_staread`), weather generator (`cli_wgnread`), station
//! lists (`pcp.cli`, `tmp.cli`…) and measured series (`cli_pmeas`, `cli_tmeas`, `cli_smeas`,
//! `cli_hmeas`, `cli_wmeas`, `cli_petmeas`).

use swat_model::{ClimateRecord, ClimateVar, StationSeries, WeatherStation, Wgn, WgnMonth};

use crate::list::Records;
use crate::{Error, Result};

fn opt(value: Option<String>) -> Option<String> {
    value.filter(|v| v != "null")
}

/// Reads `weather-sta.cli`: name, wgn, pcp, tmp, slr, hmd, wnd, pet (the `wnd_dir` column) and
/// atmospheric deposition, one station per record.
pub fn read_stations(file: &str, text: &str) -> Result<Vec<WeatherStation>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let mut out = Vec::new();
    loop {
        let f = match rec.read(9) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        out.push(WeatherStation {
            name: f.text_or(0, ""),
            wgn: opt(f.text(1)),
            pcp: opt(f.text(2)),
            tmp: opt(f.text(3)),
            slr: opt(f.text(4)),
            hmd: opt(f.text(5)),
            wnd: opt(f.text(6)),
            pet: opt(f.text(7)),
            atmo_dep: opt(f.text(8)),
        });
    }
    Ok(out)
}

/// Reads `weather-wgn.cli`: per station a record with name, lat, lon, elev and rain years, a
/// header, and twelve monthly records of fourteen values.
pub fn read_wgn(file: &str, text: &str) -> Result<Vec<Wgn>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    let mut out = Vec::new();
    loop {
        let f = match rec.read(5) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let mut w = Wgn {
            name: f.text_or(0, ""),
            lat: f.real_or(1, 0.0)?,
            lon: f.real_or(2, 0.0)?,
            elev: f.real_or(3, 0.0)?,
            rain_yrs: f.real_or(4, 0.0)?,
            months: [WgnMonth::default(); 12],
        };
        rec.skip()?;
        for month in w.months.iter_mut() {
            let m = rec.read(14)?;
            let v = |i: usize| m.real_or(i, 0.0);
            *month = WgnMonth {
                tmp_max_ave: v(0)?,
                tmp_min_ave: v(1)?,
                tmp_max_sd: v(2)?,
                tmp_min_sd: v(3)?,
                pcp_ave: v(4)?,
                pcp_sd: v(5)?,
                pcp_skew: v(6)?,
                wet_dry: v(7)?,
                wet_wet: v(8)?,
                pcp_days: v(9)?,
                pcp_hhr: v(10)?,
                slr_ave: v(11)?,
                dew_ave: v(12)?,
                wnd_ave: v(13)?,
            };
        }
        out.push(w);
    }
    Ok(out)
}

/// Reads a station list (`pcp.cli`, `tmp.cli`…): a title, a header, then one file name per record.
pub fn read_station_list(file: &str, text: &str) -> Result<Vec<String>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let mut out = Vec::new();
    loop {
        match rec.skip() {
            Ok(Some(name)) => out.push(name),
            Ok(None) => {}
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// Reads a measured station file: a title, a header, `nbyr tstep lat lon elev`, then records of
/// year, day of year and the values (two for temperature). Sub-daily precipitation (`tstep > 0`)
/// has year, day, month, day of month, hour and value.
pub fn read_series(var: ClimateVar, file: &str, text: &str) -> Result<StationSeries> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let h = rec.read(5)?;
    let mut s = StationSeries {
        var,
        file: file.to_string(),
        nbyr: h.int_or(0, 0)?,
        tstep: h.int_or(1, 0)?,
        lat: h.real_or(2, 0.0)?,
        lon: h.real_or(3, 0.0)?,
        elev: h.real_or(4, 0.0)?,
        records: Vec::new(),
    };
    let subdaily = s.tstep > 0 && var == ClimateVar::Pcp;
    let width = if subdaily { 6 } else { 2 + var.values() };
    loop {
        let f = match rec.read(width) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let year = f.int_or(0, 0)?;
        let jday = f.int_or(1, 0)?;
        let record = if subdaily {
            ClimateRecord { year, jday, hour: Some(f.int_or(4, 0)?), values: [f.real_or(5, 0.0)?, 0.0] }
        } else {
            let second = if var.values() == 2 { f.real_or(3, 0.0)? } else { 0.0 };
            ClimateRecord { year, jday, hour: None, values: [f.real_or(2, 0.0)?, second] }
        };
        s.records.push(record);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stations() {
        let text = "weather-sta.cli: x\nname wgn pcp tmp slr hmd wnd wnd_dir atmo_dep\n\
                    s1 Imsil Imsilpcp.pcp Imsiltmp.tmp Imsilsol.slr sim Imsilwind.wnd null null\n";
        let st = read_stations("weather-sta.cli", text).unwrap();
        assert_eq!(st.len(), 1);
        assert_eq!(st[0].hmd.as_deref(), Some("sim"));
        assert_eq!(st[0].pet, None);
    }

    #[test]
    fn series_tabs_and_missing() {
        let text = "AME.Tmp\n NBYR TSTEP LAT LONG ELEV\n   121 0 42.04 -93.89 316\n\
                    1900\t1\t-99\t-99\n1900\t2\t3.5\t-1.25\n";
        let s = read_series(ClimateVar::Tmp, "ames.tmp", text).unwrap();
        assert_eq!((s.nbyr, s.tstep), (121, 0));
        assert_eq!(s.records.len(), 2);
        assert_eq!(s.records[0].values, [-99.0, -99.0]);
        assert_eq!(s.records[1].values, [3.5, -1.25]);
    }

    #[test]
    fn subdaily_precipitation() {
        let text = "t\nh\n1 24 0 0 0\n2001 1 1 1 1 0.5\n2001 1 1 1 2 0.0\n";
        let s = read_series(ClimateVar::Pcp, "p.pcp", text).unwrap();
        assert_eq!(s.records.len(), 2);
        assert_eq!(s.records[0].hour, Some(1));
        assert_eq!(s.records[0].values[0], 0.5);
    }

    #[test]
    fn station_list() {
        let list = read_station_list("pcp.cli", "pcp.cli \r\nFILENAME\r\names.pcp \r\n").unwrap();
        assert_eq!(list, vec!["ames.pcp".to_string()]);
    }
}
