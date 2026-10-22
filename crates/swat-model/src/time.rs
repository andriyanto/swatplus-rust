//! The simulation period (`time.sim`) and the days SWAT+ simulates in it.

/// Cumulative days at the end of each month, leap year (SWAT+ `ndays_leap`).
pub const NDAYS_LEAP: [i32; 13] = [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366];
/// Cumulative days at the end of each month, common year (SWAT+ `ndays_noleap`).
pub const NDAYS_NOLEAP: [i32; 13] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];

/// `time.sim` as written: first and last day (day of year) and year, and time steps per day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimeSim {
    pub day_start: i32,
    pub yrc_start: i32,
    pub day_end: i32,
    pub yrc_end: i32,
    pub step: i32,
}

/// One simulated day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimDay {
    /// Calendar year.
    pub year: i32,
    /// Day of year, 1 = 1 January.
    pub jday: i32,
    /// Month, 1–12.
    pub month: u8,
    /// Day of month.
    pub day: u8,
    /// Year of the simulation, 1 = first (SWAT+ `time%yrs`).
    pub year_index: i32,
}

impl TimeSim {
    /// The period as SWAT+ uses it after reading (`time_read`): a step of 0 or less is daily (1),
    /// and a first day of 0 or less is 1 January.
    pub fn normalized(self) -> Self {
        Self { step: if self.step <= 0 { 1 } else { self.step }, day_start: if self.day_start <= 0 { 1 } else { self.day_start }, ..self }
    }

    /// Number of simulated years (SWAT+ `time%nbyr`).
    pub fn years(&self) -> i32 {
        self.yrc_end - self.yrc_start + 1
    }

    /// The days SWAT+ simulates, in order, as the loops in `time_control` walk them: the first year
    /// from `day_start`, every later year from 1 January, and the last year up to `day_end` when it
    /// is above 0 (capped at the year's length), otherwise to 31 December.
    pub fn days(&self) -> Vec<SimDay> {
        let t = self.normalized();
        let years = t.years().max(0);
        let mut out = Vec::new();
        for year_index in 1..=years {
            let year = t.yrc_start + year_index - 1;
            let ndays = month_table(year);
            let year_len = ndays[12];
            let first = if year_index > 1 { 1 } else { t.day_start };
            let last = if year_index == years && t.day_end > 0 { t.day_end.min(year_len) } else { year_len };
            for jday in first..=last {
                if let Some((month, day)) = month_day_in(&ndays, jday) {
                    out.push(SimDay { year, jday, month, day, year_index });
                }
            }
        }
        out
    }
}

/// Gregorian leap year, as `time_control` decides it.
pub fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The cumulative month table of `year`.
pub fn month_table(year: i32) -> [i32; 13] {
    if is_leap(year) { NDAYS_LEAP } else { NDAYS_NOLEAP }
}

/// Month and day of month of day of year `jday` in `year` (SWAT+ `xmon`); `None` past the year.
pub fn month_day(year: i32, jday: i32) -> Option<(u8, u8)> {
    month_day_in(&month_table(year), jday)
}

fn month_day_in(ndays: &[i32; 13], jday: i32) -> Option<(u8, u8)> {
    if jday < 1 {
        return None;
    }
    (1..=12usize).find_map(|mo| {
        let end = *ndays.get(mo)?;
        let before = *ndays.get(mo - 1)?;
        (jday <= end).then(|| (mo as u8, (jday - before) as u8))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
    }

    #[test]
    fn xmon() {
        assert_eq!(month_day(2023, 1), Some((1, 1)));
        assert_eq!(month_day(2023, 59), Some((2, 28)));
        assert_eq!(month_day(2023, 60), Some((3, 1)));
        assert_eq!(month_day(2024, 60), Some((2, 29)));
        assert_eq!(month_day(2024, 366), Some((12, 31)));
        assert_eq!(month_day(2023, 366), None);
        assert_eq!(month_day(2023, 0), None);
    }

    #[test]
    fn whole_years() {
        let t = TimeSim { day_start: 1, yrc_start: 2010, day_end: 365, yrc_end: 2023, step: 0 };
        let days = t.days();
        // 2012, 2016 and 2020 are leap years; day_end 365 cuts 31 December of 2023 only if leap.
        assert_eq!(days.len(), 14 * 365 + 3);
        assert_eq!(days.first().map(|d| (d.year, d.jday)), Some((2010, 1)));
        assert_eq!(days.last().map(|d| (d.year, d.jday, d.month, d.day)), Some((2023, 365, 12, 31)));
        assert_eq!(t.normalized().step, 1);
    }

    #[test]
    fn zeros_mean_whole_years() {
        // Ames_sub1: day_start 0 and day_end 0.
        let t = TimeSim { day_start: 0, yrc_start: 1975, day_end: 0, yrc_end: 1976, step: 0 };
        let days = t.days();
        assert_eq!(days.len(), 365 + 366);
        assert_eq!(days.last().map(|d| (d.year, d.jday, d.year_index)), Some((1976, 366, 2)));
    }

    #[test]
    fn partial_first_and_last_year() {
        let t = TimeSim { day_start: 100, yrc_start: 2021, day_end: 10, yrc_end: 2022, step: 1 };
        let days = t.days();
        assert_eq!(days.len(), (365 - 100 + 1) + 10);
        // A day_end past the year's length is capped.
        let t = TimeSim { day_start: 1, yrc_start: 2023, day_end: 366, yrc_end: 2023, step: 1 };
        assert_eq!(t.days().len(), 365);
        // An end before the start gives no days.
        let t = TimeSim { day_start: 1, yrc_start: 2023, day_end: 0, yrc_end: 2020, step: 1 };
        assert!(t.days().is_empty());
    }
}
