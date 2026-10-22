//! `time.sim` (SWAT+ `time_read`).

use swat_model::TimeSim;

use crate::Result;
use crate::list::Records;

/// Reads `time.sim` as written; [`TimeSim::normalized`] applies SWAT+'s adjustments.
pub fn read(text: &str) -> Result<TimeSim> {
    let mut rec = Records::new("time.sim", text);
    rec.skip()?;
    rec.skip()?;
    let f = rec.read(5)?;
    Ok(TimeSim {
        day_start: f.int_or(0, 0)?,
        yrc_start: f.int_or(1, 0)?,
        day_end: f.int_or(2, 0)?,
        yrc_end: f.int_or(3, 0)?,
        step: f.int_or(4, 0)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads() {
        let t = read("time.sim: x\nday_start yrc_start day_end yrc_end step\n 1 2010 365 2023 0\n").unwrap();
        assert_eq!(t, TimeSim { day_start: 1, yrc_start: 2010, day_end: 365, yrc_end: 2023, step: 0 });
        assert!(read("t\nh\n1 2010 x 2023 0\n").is_err());
    }
}
