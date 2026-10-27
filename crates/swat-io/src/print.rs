//! `print.prt` (SWAT+ `basin_print_codes_read`).

use swat_model::{PrintObject, PrintSettings};

use crate::list::Records;
use crate::{Error, Result};

/// Reads `print.prt`. The object table is read row by row to the end of the file; SWAT+ matches
/// the rows by name, so they are kept with their names in file order.
pub fn read(text: &str) -> Result<PrintSettings> {
    let mut p = PrintSettings::default();
    let mut rec = Records::new("print.prt", text);
    rec.skip()?;
    rec.skip()?;
    let f = rec.read(6)?;
    p.nyskip = f.int_or(0, 0)?;
    p.day_start = f.int_or(1, 0)?;
    p.yrc_start = f.int_or(2, 0)?;
    p.day_end = f.int_or(3, 0)?;
    p.yrc_end = f.int_or(4, 0)?;
    p.interval = f.int_or(5, 0)?;
    rec.skip()?;
    // The count of average annual intervals, then (re-reading the same record) their years.
    let mark = rec.mark();
    let count = rec.read(1)?.int_or(0, 0)?;
    if count > 0 {
        rec.reset(mark);
        let n = usize::try_from(count.min(10_000)).unwrap_or(0);
        let f = rec.read(1 + n)?;
        p.aa_years = (1..=n).map(|i| f.int_or(i, 0)).collect::<Result<_>>()?;
    }
    rec.skip()?;
    let f = rec.read(3)?;
    p.csvout = f.text_or(0, "");
    p.use_obj_labels = f.text_or(1, "");
    p.cdfout = f.text_or(2, "");
    rec.skip()?;
    let f = rec.read(4)?;
    p.crop_yld = f.text_or(0, "");
    p.mgtout = f.text_or(1, "");
    p.hydcon = f.text_or(2, "");
    p.fdcout = f.text_or(3, "");
    rec.skip()?;
    loop {
        let f = match rec.read(5) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        p.objects.push(PrintObject {
            name: f.text_or(0, ""),
            daily: f.text_or(1, ""),
            monthly: f.text_or(2, ""),
            yearly: f.text_or(3, ""),
            avann: f.text_or(4, ""),
        });
    }
    // basin_print_codes_read: without object labels (`dbout` n) the rows are read in a fixed
    // order whatever their names; a print.prt with more or fewer rows shifts the flags.
    if p.use_obj_labels == "n" {
        p.objects.truncate(SEQUENTIAL.len());
        for (o, name) in p.objects.iter_mut().zip(SEQUENTIAL) {
            o.name = name.to_string();
        }
    }
    Ok(p)
}

/// The rows `basin_print_codes_read` reads when it doesn't go by their names.
const SEQUENTIAL: [&str; 52] = [
    "basin_wb",
    "basin_nb",
    "basin_ls",
    "basin_pw",
    "basin_aqu",
    "basin_res",
    "basin_cha",
    "basin_sd_cha",
    "basin_psc",
    "region_wb",
    "region_nb",
    "region_ls",
    "region_pw",
    "region_aqu",
    "region_res",
    "region_sd_cha",
    "region_psc",
    "water_allo",
    "lsunit_wb",
    "lsunit_nb",
    "lsunit_ls",
    "lsunit_pw",
    "hru_wb",
    "hru_nb",
    "hru_ls",
    "hru_pw",
    "hru-lte_wb",
    "hru-lte_nb",
    "hru-lte_ls",
    "hru-lte_pw",
    "channel",
    "channel_sd",
    "aquifer",
    "reservoir",
    "recall",
    "hyd",
    "ru",
    "pest",
    "basin_salt",
    "hru_salt",
    "ru_salt",
    "aqu_salt",
    "channel_salt",
    "res_salt",
    "wetland_salt",
    "basin_cs",
    "hru_cs",
    "ru_cs",
    "aqu_cs",
    "channel_cs",
    "res_cs",
    "wetland_cs",
];

#[cfg(test)]
mod tests {
    use super::*;

    const PRT: &str = "print.prt: x\n\
        nyskip day_start yrc_start day_end yrc_end interval\n\
        3 0 0 0 0 1\n\
        aa_int_cnt\n\
        2 2015 2020\n\
        csvout dbout cdfout\n\
        n n n\n\
        soilout mgtout hydcon fdcout\n\
        n y n n\n\
        objects daily monthly yearly avann\n\
        basin_wb n n y y\n\
        hru_cb l y y y\n";

    #[test]
    fn reads() {
        let p = read(PRT).unwrap();
        assert_eq!(p.nyskip, 3);
        assert_eq!(p.interval, 1);
        assert_eq!(p.aa_years, vec![2015, 2020]);
        assert_eq!(p.mgtout, "y");
        assert_eq!(p.objects.len(), 2);
        // Read in SWAT+'s fixed order (no object labels): the second row is basin_nb's.
        assert_eq!(p.object("hru_cb"), None);
        assert_eq!(p.object("basin_nb").map(|o| o.daily.as_str()), Some("l"));
        let p = read(&PRT.replace("n n n\n", "n y n\n")).unwrap();
        assert_eq!(p.object("hru_cb").map(|o| o.daily.as_str()), Some("l"));
    }

    #[test]
    fn no_intervals() {
        let p = read(&PRT.replace("2 2015 2020", "0")).unwrap();
        assert!(p.aa_years.is_empty());
        assert_eq!(p.objects.len(), 2);
    }
}
