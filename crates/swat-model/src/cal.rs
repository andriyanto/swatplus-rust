//! Hard calibration: `cal_parms.cal` (`cal_parm_read`) and `calibration.cal`
//! (`cal_parmchg_read`), the parameter changes applied once the objects are set up
//! (`cal_conditions`).

/// A parameter that can be changed (`calibration_parameters`): the kind of object it belongs to
/// and the range the change is held to.
#[derive(Clone, Debug, PartialEq)]
pub struct CalParm {
    pub name: String,
    pub ob_typ: String,
    pub absmin: f32,
    pub absmax: f32,
}

impl Default for CalParm {
    fn default() -> Self {
        CalParm { name: "default".into(), ob_typ: "plt".into(), absmin: 0.0, absmax: 1.0e6 }
    }
}

/// A change of `calibration.cal` (`update_parameters`): the parameter, how it changes
/// (`absval`, `abschg`, `pctchg`, `relchg`) and by how much, its conditions (the variable each
/// tests), the soil layers, years and days it applies to, and the objects (1-based; empty for all
/// of the parameter's kind).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalUpdate {
    pub name: String,
    pub chg_typ: String,
    pub val: f32,
    pub conds: Vec<CalCond>,
    /// The bounds of a `range` condition (the last one read).
    pub val1: f32,
    pub val2: f32,
    pub lyr1: i32,
    pub lyr2: i32,
    pub year1: i32,
    pub year2: i32,
    pub day1: i32,
    pub day2: i32,
    pub elems: Vec<usize>,
}

/// A condition of a change (`calibration_conditions`): the variable it tests, an operator and
/// a number (unused by the conditions SWAT+ applies), and the text the variable must equal. A
/// `range` condition has only its variable (its bounds are the update's `val1` and `val2`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalCond {
    pub var: String,
    pub alt: String,
    pub targ: f32,
    pub targc: String,
}

/// SWAT+ `define_unit_elements`: a list of object numbers where a negative number closes a range
/// opened by the one before it (`1 -1858` is 1 to 1858).
pub fn define_unit_elements(cnt: &[i32]) -> Vec<usize> {
    let n = cnt.len();
    let mut out = Vec::new();
    if n == 1 {
        out.push(cnt[0].max(0) as usize);
        return out;
    }
    let mut ii = 1usize;
    while ii <= n {
        if ii == n {
            let (ie1, ie2) = (cnt[ii - 2], cnt[ii - 1]);
            if ie2 > 0 {
                out.push(ie2 as usize);
            } else {
                out.extend((ie1..=ie2.abs()).map(|e| e.max(0) as usize));
            }
            ii += 1;
        } else {
            let (ie1, ie2) = (cnt[ii - 1], cnt[ii]);
            if ie2 > 0 {
                out.push(ie1.max(0) as usize);
                ii += 1;
            } else {
                out.extend((ie1..=ie2.abs()).map(|e| e.max(0) as usize));
                ii += 2;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_elements() {
        assert_eq!(define_unit_elements(&[1, -4]), [1, 2, 3, 4]);
        assert_eq!(define_unit_elements(&[7]), [7]);
        assert_eq!(define_unit_elements(&[2, 5, -7, 9]), [2, 5, 6, 7, 9]);
    }
}
