//! `codes.bsn` (`basin_read_cc`) and `parameters.bsn` (`basin_read_prm`).

use swat_model::basin::{BasinCodes, BasinParams};

use crate::list::{Fields, Records};
use crate::{Error, Result};

/// One list-directed read of `n` values after the title and header, as SWAT+ reads a derived
/// type. At the end of the file the values read so far are kept (`None` when the file is short).
fn read_record(file: &str, text: &str, n: usize) -> Result<Option<Fields>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    match rec.read(n) {
        Ok(f) => Ok(Some(f)),
        Err(Error::Eof { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Reads `codes.bsn`. A file too short for every field leaves the defaults.
pub fn read_codes(text: &str) -> Result<BasinCodes> {
    let mut c = BasinCodes::default();
    let Some(f) = read_record("codes.bsn", text, 26)? else {
        return Ok(c);
    };
    c.petfile = f.text(0).unwrap_or(c.petfile);
    c.wwqfile = f.text(1).unwrap_or(c.wwqfile);
    let mut ints: [&mut i32; 20] = [
        &mut c.pet,
        &mut c.nam1,
        &mut c.crk,
        &mut c.swift_out,
        &mut c.sed_det,
        &mut c.rte,
        &mut c.deg,
        &mut c.wq,
        &mut c.nostress,
        &mut c.cn,
        &mut c.cfac,
        &mut c.cswat,
        &mut c.lapse,
        &mut c.uhyd,
        &mut c.sed_ch,
        &mut c.tdrn,
        &mut c.wtdn,
        &mut c.sol_p_model,
        &mut c.gampt,
        &mut c.smax,
    ];
    // Fields 3–21 are integers, 22 is a character (atmo), 23–26 integers again.
    let (before_atmo, after_atmo) = ints.split_at_mut(19);
    for (i, v) in before_atmo.iter_mut().enumerate() {
        **v = f.int_or(2 + i, **v)?;
    }
    c.atmo = f.text(21).unwrap_or(c.atmo);
    if let Some(v) = after_atmo.first_mut() {
        **v = f.int_or(22, **v)?;
    }
    c.qual2e = f.int_or(23, c.qual2e)?;
    c.gwflow = f.int_or(24, c.gwflow)?;
    c.idc_till = f.int_or(25, c.idc_till)?;
    Ok(c)
}

/// Reads `parameters.bsn`: 42 reals and two integers, by position.
///
/// As gfortran: a value that doesn't convert (SWAT+ Editor writes `day_lag_mx` as `0.00000`,
/// which is not an integer) ends the read there with a positive `iostat`, which SWAT+ ignores. The
/// values before it are kept and the rest keep their defaults, so `igen` stays 5.
pub fn read_params(text: &str) -> Result<BasinParams> {
    let mut p = BasinParams::default();
    let Some(f) = read_record("parameters.bsn", text, 44)? else {
        return Ok(p);
    };
    for (i, v) in p.reals.iter_mut().enumerate() {
        match f.real(i) {
            Ok(value) => *v = value.unwrap_or(*v),
            Err(_) => return Ok(p),
        }
    }
    match f.int(42) {
        Ok(value) => p.day_lag_mx = value.unwrap_or(p.day_lag_mx),
        Err(_) => return Ok(p),
    }
    if let Ok(value) = f.int(43) {
        p.igen = value.unwrap_or(p.igen);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let text = "codes.bsn: x\nhead\nnull null 1 0 0 1 0 0 0 1 0 0 0 0 1 1 0 0 0 0 0 a 0 0 0 3\n";
        let c = read_codes(text).unwrap();
        assert_eq!((c.petfile.as_str(), c.pet, c.swift_out, c.lapse, c.uhyd), ("null", 1, 1, 1, 1));
        assert_eq!((c.atmo.as_str(), c.idc_till), ("a", 3));
    }

    #[test]
    fn params_by_position() {
        let mut line: Vec<String> = (0..42).map(|i| format!("{}.5", i)).collect();
        line.push("2".into());
        line.push("0".into());
        let text = format!("t\nh\n{}\n", line.join(" "));
        let p = read_params(&text).unwrap();
        assert_eq!(p.adj_pkr(), 3.5);
        assert_eq!(p.tlaps(), 29.5);
        assert_eq!((p.day_lag_mx, p.igen), (2, 0));
    }

    #[test]
    fn a_bad_integer_ends_the_read() {
        let mut line: Vec<String> = (0..42).map(|i| format!("{}.5", i)).collect();
        line.push("0.00000".into());
        line.push("0".into());
        let p = read_params(&format!("t\nh\n{}\n", line.join(" "))).unwrap();
        assert_eq!(p.tlaps(), 29.5);
        assert_eq!((p.day_lag_mx, p.igen), (0, 5));
    }

    #[test]
    fn short_file_keeps_defaults() {
        let p = read_params("t\nh\n3.0 0.5\n").unwrap();
        assert_eq!(p.igen, 5);
        assert_eq!(p.tlaps(), 6.5);
    }
}
