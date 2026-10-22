//! The simulation clock: SWAT+'s `time` variables as `time_control` moves them.

use swat_model::TimeSim;
use swat_model::time::{NDAYS_LEAP, NDAYS_NOLEAP, is_leap};

/// SWAT+ `time` and the month table `ndays`, which several routines read as global state.
#[derive(Clone, Debug)]
pub struct Clock {
    /// Calendar year (`time%yrc`).
    pub yrc: i32,
    /// Year of the simulation, 1 = first (`time%yrs`).
    pub yrs: i32,
    /// Years simulated (`time%nbyr`).
    pub nbyr: i32,
    /// Day of year (`time%day`).
    pub day: i32,
    /// Month (`time%mo`) and day of month (`time%day_mo`).
    pub mo: i32,
    pub day_mo: i32,
    /// First day of the current year's loop (`time%day_start`; 1 after the first year).
    pub day_start: i32,
    /// Last day of the last year, as read (`time%day_end`; 0 is 31 December).
    pub day_end: i32,
    /// Last day of the current year's loop (`time%day_end_yr`).
    pub day_end_yr: i32,
    /// Time steps per day (`time%step`).
    pub step: i32,
    /// Last day of the year (`time%end_yr`).
    pub end_yr: bool,
    /// SWAT+'s global `ndays`: cumulative days per month. It starts as the leap-year table and is
    /// changed by the temperature reader and by each year of the loop; routines read it as it is.
    pub ndays: [i32; 13],
}

impl Clock {
    /// The clock after `time_read`: day and month of the first day, from the leap-year table that
    /// `ndays` starts as.
    pub fn new(time: &TimeSim) -> Self {
        let t = time.normalized();
        let ndays = NDAYS_LEAP;
        let (mo, day_mo) = xmon(&ndays, t.day_start);
        Self {
            yrc: t.yrc_start,
            yrs: 1,
            nbyr: t.years(),
            day: t.day_start,
            mo,
            day_mo,
            day_start: t.day_start,
            day_end: t.day_end,
            day_end_yr: 0,
            step: t.step,
            end_yr: false,
            ndays,
        }
    }

    /// Sets `ndays` the way the readers and `time_control` do: leap if the year divides by 4
    /// (only `time_control` applies the century rules; see [`Clock::start_year`]).
    pub fn set_ndays_mod4(&mut self, year: i32) {
        self.ndays = if year % 4 == 0 { NDAYS_LEAP } else { NDAYS_NOLEAP };
    }

    /// The state `time_control` sets up before its year loop, for the first day's precipitation.
    pub fn before_loop(&mut self) {
        self.day_end_yr = if self.yrc % 4 == 0 { NDAYS_LEAP[12] } else { NDAYS_NOLEAP[12] };
        self.yrs = 1;
        let (mo, day_mo) = xmon(&self.ndays, self.day);
        self.mo = mo;
        self.day_mo = day_mo;
    }

    /// The start of simulated year `curyr` (1-based): the year's month table (Gregorian here), its
    /// first day and its last.
    pub fn start_year(&mut self, curyr: i32) {
        self.yrs = curyr;
        self.ndays = if is_leap(self.yrc) { NDAYS_LEAP } else { NDAYS_NOLEAP };
        if self.yrs > 1 {
            self.day_start = 1;
        }
        self.day_end_yr = self.ndays[12];
        if self.yrs == self.nbyr && self.day_end > 0 {
            self.day_end_yr = self.day_end.min(self.ndays[12]);
        }
    }

    /// Moves to day `jday` of the current year.
    pub fn set_day(&mut self, jday: i32) {
        self.day = jday;
        let (mo, day_mo) = xmon(&self.ndays, jday);
        self.mo = mo;
        self.day_mo = day_mo;
        self.end_yr = self.day == self.day_end_yr;
    }

    /// Index 0–11 of the current month, for monthly tables.
    pub fn month_index(&self) -> usize {
        (self.mo.clamp(1, 12) - 1) as usize
    }
}

/// SWAT+ `xmon`: month and day of month of day of year `jd` in the table `ndays`. A day past the
/// table keeps month 0, as the Fortran leaves its outputs unset.
pub fn xmon(ndays: &[i32; 13], jd: i32) -> (i32, i32) {
    for mo in 1..=12usize {
        if jd <= ndays[mo] {
            return (mo as i32, jd - ndays[mo - 1]);
        }
    }
    (0, 0)
}
