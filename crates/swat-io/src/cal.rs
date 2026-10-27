//! Readers for the hard-calibration files: `cal_parms.cal` (`cal_parm_read`) and
//! `calibration.cal` (`cal_parmchg_read`).

use swat_model::cal::{CalCond, CalParm, CalUpdate, define_unit_elements};

use crate::Result;
use crate::list::Records;

/// `cal_parms.cal`: a title, the number of parameters, a header, then a row per parameter.
pub fn read_cal_parms(file: &str, text: &str) -> Result<Vec<CalParm>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    let n = rec.read(1)?.int(0)?.unwrap_or(0).max(0);
    rec.skip()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let Some((f, eof)) = rec.read_partial(5) else { break };
        let d = CalParm::default();
        out.push(CalParm {
            name: f.text_or(0, &d.name),
            ob_typ: f.text_or(1, &d.ob_typ),
            absmin: f.real_or(2, d.absmin)?,
            absmax: f.real_or(3, d.absmax)?,
        });
        if eof {
            break;
        }
    }
    Ok(out)
}

/// `calibration.cal`: a title, the number of changes, a header, then each change: its row,
/// ending with the number of entries of its object list and the list, then a line per condition.
pub fn read_calibration(file: &str, text: &str) -> Result<Vec<CalUpdate>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    let n = rec.read(1)?.int(0)?.unwrap_or(0).max(0);
    rec.skip()?;
    let mut out = Vec::new();
    for _ in 0..n {
        let mark = rec.mark();
        let f = match rec.read(11) {
            Ok(f) => f,
            Err(crate::Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let nspu = f.int_or(10, 0)?.max(0) as usize;
        let mut u = CalUpdate {
            name: f.text_or(0, ""),
            chg_typ: f.text_or(1, ""),
            val: f.real_or(2, 0.0)?,
            lyr1: f.int_or(4, 0)?,
            lyr2: f.int_or(5, 0)?,
            year1: f.int_or(6, 0)?,
            year2: f.int_or(7, 0)?,
            day1: f.int_or(8, 0)?,
            day2: f.int_or(9, 0)?,
            ..CalUpdate::default()
        };
        let nconds = f.int_or(3, 0)?.max(0);
        if nspu > 0 {
            rec.reset(mark);
            let f = rec.read(11 + nspu)?;
            let cnt = (0..nspu).map(|k| f.int_or(11 + k, 0)).collect::<Result<Vec<i32>>>()?;
            // num_tot (the eleventh value) is the number of entries; 0 means all objects.
            if f.int_or(10, 0)? != 0 {
                u.elems = define_unit_elements(&cnt);
            }
        }
        for _ in 0..nconds {
            // `range var val1 val2`, or `var alt targ targc`.
            let mark = rec.mark();
            let first = rec.read(1)?.text_or(0, "");
            rec.reset(mark);
            if first == "range" {
                let c = rec.read(4)?;
                u.conds.push(CalCond { var: c.text_or(1, ""), ..CalCond::default() });
                u.val1 = c.real_or(2, 0.0)?;
                u.val2 = c.real_or(3, 0.0)?;
            } else {
                let c = rec.read(4)?;
                u.conds.push(CalCond { var: c.text_or(0, ""), alt: c.text_or(1, ""), targ: c.real_or(2, 0.0)?, targc: c.text_or(3, "") });
            }
        }
        out.push(u);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_with_object_list() {
        let text = "title\n1\nheader\nperco  absval  0.85  0  0  0  0  0  0  0  2  1  -5\n";
        let u = read_calibration("calibration.cal", text).unwrap();
        assert_eq!(u.len(), 1);
        assert_eq!((u[0].name.as_str(), u[0].chg_typ.as_str(), u[0].val), ("perco", "absval", 0.85));
        assert_eq!(u[0].elems, [1, 2, 3, 4, 5]);
        let p = read_cal_parms("cal_parms.cal", "t\n1\nh\nperco  hru  0.0  1.0  fraction\n").unwrap();
        assert_eq!((p[0].ob_typ.as_str(), p[0].absmax), ("hru", 1.0));
    }
}
