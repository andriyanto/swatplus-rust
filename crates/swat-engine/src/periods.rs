//! Monthly, yearly and average annual output: the sums and averages `hru_output`, `ru_output`,
//! `aquifer_output` and `sd_channel_output` keep, in their order of operations (single
//! precision, so the order is the result).
//!
//! Each table's columns follow one of SWAT+'s operator rules: `+` adds a period's days, `/`
//! divides by the years printed and `//` by the days, each on its own set of columns.

use crate::channel::ChWat;
use crate::hru::HruDay;
use crate::hyd::Hyd;
use crate::route::AquDay;

/// Which period a row sums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    Mon,
    Yr,
    Aa,
}

impl Period {
    /// The file name suffix SWAT+ uses.
    pub fn suffix(self) -> &'static str {
        match self {
            Period::Mon => "mon",
            Period::Yr => "yr",
            Period::Aa => "aa",
        }
    }
}

/// How a column takes the operators.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    /// A flux: added; `/` divides it, `//` doesn't.
    Sum,
    /// A state: added; `//` divides it, `/` doesn't.
    Ave,
    /// `+` keeps the first operand's, `/` gives 0, `//` keeps it (soil water and snow at the
    /// start and end of the period).
    Keep,
}

/// `hruout_plantweather_add` leaves out the plants' P: the sum's is 0.
fn add_pw(acc: &mut [f32; 25], x: &[f32; 25]) {
    add(acc, x, &PW);
    acc[PW_PPLNT] = 0.0;
}

fn add(acc: &mut [f32], x: &[f32], rules: &[Rule]) {
    for ((a, &v), r) in acc.iter_mut().zip(x).zip(rules) {
        match r {
            Rule::Sum | Rule::Ave => *a += v,
            Rule::Keep => {}
        }
    }
}

/// `/`.
fn div(acc: &mut [f32], c: f32, rules: &[Rule]) {
    for (a, r) in acc.iter_mut().zip(rules) {
        match r {
            Rule::Sum => *a /= c,
            Rule::Keep => *a = 0.0,
            Rule::Ave => {}
        }
    }
}

/// `//`.
fn ave(acc: &mut [f32], c: f32, rules: &[Rule]) {
    for (a, r) in acc.iter_mut().zip(rules) {
        if *r == Rule::Ave {
            *a /= c;
        }
    }
}

use Rule::{Ave as A, Keep as K, Sum as S};

/// `hru_wb` in the port's column order (`HruWb::columns`).
const WB: [Rule; 32] = [S, S, S, S, S, S, S, S, S, S, S, S, A, K, K, A, A, K, K, A, S, S, S, S, S, S, S, S, S, S, S, A];
const WB_SW_INIT: usize = 13;
const WB_SW_FINAL: usize = 14;
const WB_SNO_INIT: usize = 17;
const WB_SNO_FINAL: usize = 18;
const WB_SW: usize = 15;
const WB_SNOPACK: usize = 19;

/// `hru_pw` in SWAT+'s order (`output_plantweather`): the plant columns, the weather, then
/// heat units, maximum leaf area and biomass, biomass and carbon growth.
pub const PW_NAMES: [&str; 25] = [
    "lai", "bioms", "yield", "residue", "sol_tmp", "strsw", "strsa", "strstmp", "strsn", "strsp", "strss", "nplt", "percn", "pplnt", "tmx",
    "tmn", "tmpav", "solarad", "wndspd", "rhum", "phubas0", "lai_max", "bm_max", "bm_grow", "c_gro",
];
const PW: [Rule; 25] = [A, A, S, A, A, S, S, S, S, S, S, A, S, A, A, A, A, A, A, A, A, A, A, A, A];
const PW_NPLT: usize = 11;
const PW_PPLNT: usize = 13;
const PW_BMMAX: usize = 22;
const PW_BIOMS: usize = 1;

const ALL_SUM_13: [Rule; 13] = [S; 13];

/// One HRU day's values for the period tables: water balance, nutrients, losses, and plants
/// with the weather in SWAT+'s order.
pub fn hru_values(d: &HruDay, weather: [f32; 6]) -> ([f32; 32], [f32; 13], [f32; 13], [f32; 25]) {
    let wb = d.wb.columns().map(|(_, v)| v);
    let nb = d.nb.columns().map(|(_, v)| v);
    let ls = d.ls.columns().map(|(_, v)| v);
    let p = d.pw.columns().map(|(_, v)| v);
    let mut pw = [0.0_f32; 25];
    pw[..14].copy_from_slice(&p[..14]);
    pw[14..20].copy_from_slice(&weather);
    pw[20..].copy_from_slice(&p[14..]);
    // hru_output prints the day's biomass as its maximum.
    pw[PW_BMMAX] = pw[PW_BIOMS];
    (wb, nb, ls, pw)
}

/// A finished period's rows for one HRU.
#[derive(Clone, Copy, Debug)]
pub struct HruRows {
    pub wb: [f32; 32],
    pub nb: [f32; 13],
    pub ls: [f32; 13],
    pub pw: [f32; 25],
}

/// The calendar facts the period tables need on a day.
#[derive(Clone, Copy, Debug)]
pub struct PeriodDay {
    pub end_mo: bool,
    pub end_yr: bool,
    pub end_sim: bool,
    /// Days in the month (`ndays(mo+1) - ndays(mo)`) and the year's last day (`day_end_yr`).
    pub mon_days: f32,
    pub day_end_yr: f32,
    /// `time%yrs_prt` (already divided on the last day) and `time%days_prt`.
    pub yrs_prt: f32,
    pub days_prt: f32,
}

/// `time%yrs_prt` and `time%days_prt` as `time_control` keeps them: during the skipped years
/// the years are set to the years simulated, then each printed year adds its days to both;
/// on the last day the years are divided by 365 plus the leap years over the years (in
/// integers, so 0 unless every year is a leap year).
#[derive(Clone, Copy, Debug, Default)]
pub struct PrintYears {
    pub yrs_prt: f32,
    pub days_prt: f32,
    pub num_leap: i32,
}

impl PrintYears {
    pub fn start_year(&mut self, yrs: i32, nyskip: i32, nbyr: i32, leap: bool, day_start: i32, day_end_yr: i32) {
        if leap {
            self.num_leap += 1;
        }
        if yrs > nyskip {
            let days = (day_end_yr - day_start + 1) as f32;
            self.yrs_prt += days;
            self.days_prt += days;
        } else {
            self.yrs_prt = nbyr as f32;
        }
    }

    pub fn end_sim(&mut self, nbyr: i32) {
        self.yrs_prt /= 365.0 + (self.num_leap / nbyr) as f32;
    }
}

/// The monthly, yearly and average annual sums of one HRU (`hwb_m`, `hwb_y`, `hwb_a`, …).
#[derive(Clone, Debug)]
pub struct HruAcc {
    wb: [[f32; 32]; 3],
    nb: [[f32; 13]; 3],
    ls: [[f32; 13]; 3],
    pw: [[f32; 25]; 3],
    started: bool,
}

impl Default for HruAcc {
    fn default() -> Self {
        HruAcc { wb: [[0.0; 32]; 3], nb: [[0.0; 13]; 3], ls: [[0.0; 13]; 3], pw: [[0.0; 25]; 3], started: false }
    }
}

impl HruAcc {
    /// `hru_output` for a printed day; `emit` gets each period that ends today.
    pub fn day(&mut self, d: &HruDay, weather: [f32; 6], c: &PeriodDay, mut emit: impl FnMut(Period, &HruRows)) {
        let (mut wb, nb, ls, pw) = hru_values(d, weather);
        // hru_output adds the day before it makes the soil water and snow the day's averages:
        // the sums take the end of the day's.
        wb[WB_SW] = wb[WB_SW_FINAL];
        wb[WB_SNOPACK] = wb[WB_SNO_FINAL];
        if !self.started {
            // basin_sw_init: the soil water and snow at the start of the first printed year.
            for k in 0..3 {
                self.wb[k][WB_SW_INIT] = wb[WB_SW_INIT];
                self.wb[k][WB_SNO_INIT] = wb[WB_SNO_INIT];
            }
            self.started = true;
        }
        let (m, y, a) = (0, 1, 2);
        let bioms = pw[PW_BIOMS];
        let bm_max_m = self.pw[m][PW_BMMAX];
        let bm_max_a = self.pw[a][PW_BMMAX];
        add(&mut self.wb[m], &wb, &WB);
        add(&mut self.nb[m], &nb, &ALL_SUM_13);
        add(&mut self.ls[m], &ls, &ALL_SUM_13);
        add_pw(&mut self.pw[m], &pw);
        self.pw[m][PW_BMMAX] = bm_max_m;
        self.pw[m][PW_BMMAX] = bioms.max(self.pw[m][PW_BMMAX]);
        self.pw[y][PW_BMMAX] = bioms.max(self.pw[y][PW_BMMAX]);
        self.pw[a][PW_BMMAX] = bioms.max(self.pw[a][PW_BMMAX]);
        let point = |mut p: [f32; 25]| {
            p[PW_NPLT] = pw[PW_NPLT];
            p[PW_PPLNT] = pw[PW_PPLNT];
            p
        };
        if c.end_mo {
            let bm_max_m = self.pw[m][PW_BMMAX];
            let (wbm, nbm, lsm, pwm) = (self.wb[m], self.nb[m], self.ls[m], self.pw[m]);
            add(&mut self.wb[y], &wbm, &WB);
            add(&mut self.nb[y], &nbm, &ALL_SUM_13);
            add(&mut self.ls[y], &lsm, &ALL_SUM_13);
            let bm_max_y = self.pw[y][PW_BMMAX];
            add_pw(&mut self.pw[y], &pwm);
            self.pw[y][PW_BMMAX] = bm_max_y;
            ave(&mut self.pw[m], c.mon_days, &PW);
            ave(&mut self.wb[m], c.mon_days, &WB);
            self.pw[m][PW_BMMAX] = bm_max_m;
            self.wb[m][WB_SW_FINAL] = wb[WB_SW_FINAL];
            self.wb[m][WB_SNO_FINAL] = wb[WB_SNO_FINAL];
            emit(Period::Mon, &HruRows { wb: self.wb[m], nb: self.nb[m], ls: self.ls[m], pw: point(self.pw[m]) });
            let (sw, sno) = (self.wb[m][WB_SW_FINAL], self.wb[m][WB_SNO_FINAL]);
            self.wb[m] = [0.0; 32];
            self.wb[m][WB_SW_INIT] = sw;
            self.wb[m][WB_SNO_INIT] = sno;
            self.nb[m] = [0.0; 13];
            self.ls[m] = [0.0; 13];
            self.pw[m] = [0.0; 25];
        }
        if c.end_yr {
            let bm_max_y = self.pw[y][PW_BMMAX];
            let (wby, nby, lsy, pwy) = (self.wb[y], self.nb[y], self.ls[y], self.pw[y]);
            add(&mut self.wb[a], &wby, &WB);
            add(&mut self.nb[a], &nby, &ALL_SUM_13);
            add(&mut self.ls[a], &lsy, &ALL_SUM_13);
            add_pw(&mut self.pw[a], &pwy);
            ave(&mut self.wb[y], c.day_end_yr, &WB);
            ave(&mut self.pw[y], c.day_end_yr, &PW);
            self.pw[y][PW_BMMAX] = bm_max_y;
            // The average annual maximum is restored to what it was before today: the last day
            // of each year doesn't count.
            self.pw[a][PW_BMMAX] = bm_max_a;
            self.wb[y][WB_SW_FINAL] = wb[WB_SW_FINAL];
            self.wb[y][WB_SNO_FINAL] = wb[WB_SNO_FINAL];
            emit(Period::Yr, &HruRows { wb: self.wb[y], nb: self.nb[y], ls: self.ls[y], pw: point(self.pw[y]) });
            // time_control, after the year's days.
            let (sw, sno) = (self.wb[y][WB_SW_FINAL], self.wb[y][WB_SNO_FINAL]);
            self.wb[y] = [0.0; 32];
            self.wb[y][WB_SW_INIT] = sw;
            self.wb[y][WB_SNO_INIT] = sno;
            self.nb[y] = [0.0; 13];
            self.ls[y] = [0.0; 13];
            self.pw[y] = [0.0; 25];
        }
        if c.end_sim {
            let (sw_init, sno_init) = (self.wb[a][WB_SW_INIT], self.wb[a][WB_SNO_INIT]);
            let mut wba = self.wb[a];
            div(&mut wba, c.yrs_prt, &WB);
            ave(&mut wba, c.days_prt, &WB);
            wba[WB_SW_INIT] = sw_init;
            wba[WB_SW_FINAL] = wb[WB_SW_FINAL];
            wba[WB_SNO_INIT] = sno_init;
            wba[WB_SNO_FINAL] = wb[WB_SNO_FINAL];
            let mut nba = self.nb[a];
            div(&mut nba, c.yrs_prt, &ALL_SUM_13);
            let mut lsa = self.ls[a];
            div(&mut lsa, c.yrs_prt, &ALL_SUM_13);
            let mut pwa = self.pw[a];
            div(&mut pwa, c.yrs_prt, &PW);
            ave(&mut pwa, c.days_prt, &PW);
            pwa[PW_BMMAX] = bm_max_a;
            emit(Period::Aa, &HruRows { wb: wba, nb: nba, ls: lsa, pw: point(pwa) });
        }
    }
}

/// A routing unit's sums (`ru_m`, `ru_y`, `ru_a`): hydrographs added (`hydout_add`), the
/// average annual divided by the years.
#[derive(Clone, Debug, Default)]
pub struct RuAcc {
    m: Hyd,
    y: Hyd,
    a: Hyd,
}

impl RuAcc {
    pub fn day(&mut self, d: &Hyd, c: &PeriodDay, mut emit: impl FnMut(Period, &Hyd)) {
        self.m = self.m.plus(*d);
        if c.end_mo {
            self.y = self.y.plus(self.m);
            emit(Period::Mon, &self.m);
            self.m = Hyd::default();
        }
        if c.end_yr {
            self.a = self.a.plus(self.y);
            emit(Period::Yr, &self.y);
            self.y = Hyd::default();
        }
        if c.end_sim {
            emit(Period::Aa, &self.a.over(c.yrs_prt));
        }
    }
}

/// An aquifer's sums (`aqu_m`, …): storage, water table depth and nitrate stored are averaged
/// over the month's days, then over the 12 months; the average annual is divided by the years,
/// all but the carbon (the last day's).
#[derive(Clone, Debug, Default)]
pub struct AquAcc {
    m: AquDay,
    y: AquDay,
    a: AquDay,
}

fn aqu_add(a: &mut AquDay, b: &AquDay) {
    a.flo += b.flo;
    a.dep_wt += b.dep_wt;
    a.stor += b.stor;
    a.no3_st += b.no3_st;
    a.minp += b.minp;
    a.cbn = b.cbn;
    a.orgn += b.orgn;
    a.rchrg += b.rchrg;
    a.no3_rchg += b.no3_rchg;
    a.no3_loss += b.no3_loss;
    a.seep += b.seep;
    a.revap += b.revap;
    a.no3_lat += b.no3_lat;
    a.no3_seep += b.no3_seep;
    a.flo_cha += b.flo_cha;
    a.flo_res += b.flo_res;
    a.flo_ls += b.flo_ls;
}

fn aqu_div(a: &AquDay, c: f32) -> AquDay {
    AquDay {
        flo: a.flo / c,
        dep_wt: a.dep_wt / c,
        stor: a.stor / c,
        no3_st: a.no3_st / c,
        minp: a.minp / c,
        cbn: a.cbn,
        orgn: a.orgn / c,
        rchrg: a.rchrg / c,
        no3_rchg: a.no3_rchg / c,
        no3_loss: a.no3_loss / c,
        seep: a.seep / c,
        revap: a.revap / c,
        no3_lat: a.no3_lat / c,
        no3_seep: a.no3_seep / c,
        flo_cha: a.flo_cha / c,
        flo_res: a.flo_res / c,
        flo_ls: a.flo_ls / c,
    }
}

impl AquAcc {
    pub fn day(&mut self, d: &AquDay, c: &PeriodDay, mut emit: impl FnMut(Period, &AquDay)) {
        aqu_add(&mut self.m, d);
        if c.end_mo {
            self.m.stor /= c.mon_days;
            self.m.dep_wt /= c.mon_days;
            self.m.no3_st /= c.mon_days;
            let m = self.m;
            aqu_add(&mut self.y, &m);
            emit(Period::Mon, &self.m);
            self.m = AquDay::default();
        }
        if c.end_yr {
            self.y.stor /= 12.0;
            self.y.dep_wt /= 12.0;
            self.y.no3_st /= 12.0;
            let y = self.y;
            aqu_add(&mut self.a, &y);
            emit(Period::Yr, &self.y);
            self.y = AquDay::default();
        }
        if c.end_sim {
            emit(Period::Aa, &aqu_div(&self.a, c.yrs_prt));
        }
    }
}

/// A SWAT-DEG channel's sums: inflow and outflow hydrographs added (their flows averaged over
/// the days for the month and the year, the year's added to the average annual after that), the
/// water body's precipitation, evaporation and seepage summed and its area averaged (twice
/// divided by the years for the average annual). Storage and water temperature are the day's.
#[derive(Clone, Debug, Default)]
pub struct ChaAcc {
    hin: [Hyd; 3],
    hout: [Hyd; 3],
    wat: [ChWat; 3],
}

fn wat_add(a: &mut ChWat, b: &ChWat) {
    a.area_ha += b.area_ha;
    a.precip += b.precip;
    a.evap += b.evap;
    a.seep += b.seep;
}

impl ChaAcc {
    #[allow(clippy::type_complexity)]
    pub fn day(&mut self, w: &ChWat, hin: &Hyd, hout: &Hyd, c: &PeriodDay, mut emit: impl FnMut(Period, &ChWat, &Hyd, &Hyd)) {
        let (m, y, a) = (0, 1, 2);
        self.hin[m] = self.hin[m].plus(*hin);
        self.hout[m] = self.hout[m].plus(*hout);
        wat_add(&mut self.wat[m], w);
        if c.end_mo {
            self.hin[y] = self.hin[y].plus(self.hin[m]);
            self.hout[y] = self.hout[y].plus(self.hout[m]);
            let wm = self.wat[m];
            wat_add(&mut self.wat[y], &wm);
            self.hin[m].flo /= c.mon_days;
            self.hout[m].flo /= c.mon_days;
            self.wat[m].area_ha /= c.mon_days;
            emit(Period::Mon, &self.wat[m], &self.hin[m], &self.hout[m]);
            self.hin[m] = Hyd::default();
            self.hout[m] = Hyd::default();
            self.wat[m] = ChWat::default();
        }
        if c.end_yr {
            self.hin[y].flo /= c.day_end_yr;
            self.hout[y].flo /= c.day_end_yr;
            self.hin[a] = self.hin[a].plus(self.hin[y]);
            self.hout[a] = self.hout[a].plus(self.hout[y]);
            let wy = self.wat[y];
            wat_add(&mut self.wat[a], &wy);
            self.wat[y].area_ha /= c.day_end_yr;
            emit(Period::Yr, &self.wat[y], &self.hin[y], &self.hout[y]);
            self.hin[y] = Hyd::default();
            self.hout[y] = Hyd::default();
            self.wat[y] = ChWat::default();
        }
        if c.end_sim {
            let k = c.yrs_prt;
            let wa = &self.wat[a];
            let wat = ChWat { area_ha: wa.area_ha / k / k, precip: wa.precip / k, evap: wa.evap / k, seep: wa.seep / k };
            emit(Period::Aa, &wat, &self.hin[a].over(k), &self.hout[a].over(k));
        }
    }
}

/// One HRU day's values as the basin and landscape-unit tables take them, after `hru_output`:
/// the day's average soil water and snow, the biomass as maximum.
pub fn region_values(d: &HruDay, weather: [f32; 6]) -> HruRows {
    let (wb, nb, ls, pw) = hru_values(d, weather);
    HruRows { wb, nb, ls, pw }
}

/// `region = region + hru * frac` (`basin_output`, `lsu_output`): the operators `*` and `+`;
/// soil water and snow at the end are summed on their own; organic N and P from residue under
/// 1e-10 count as 0 (`hruout_nutbal_mult`).
pub fn region_add(r: &mut HruRows, h: &HruRows, c: f32) {
    for k in 0..32 {
        match WB[k] {
            Rule::Keep => {}
            _ => r.wb[k] += h.wb[k] * c,
        }
    }
    r.wb[WB_SW_FINAL] += h.wb[WB_SW_FINAL] * c;
    r.wb[WB_SNO_FINAL] += h.wb[WB_SNO_FINAL] * c;
    for k in 0..13 {
        let x = if (k == 9 || k == 10) && h.nb[k] < 1.0e-10 { 0.0 } else { h.nb[k] * c };
        r.nb[k] += x;
        r.ls[k] += h.ls[k] * c;
    }
    for k in 0..25 {
        r.pw[k] += h.pw[k] * c;
    }
    r.pw[PW_PPLNT] = 0.0;
}

/// An empty region day, keeping the soil water and snow at its start.
pub fn region_start(prev: &HruRows) -> HruRows {
    let mut r = HruRows { wb: [0.0; 32], nb: [0.0; 13], ls: [0.0; 13], pw: [0.0; 25] };
    r.wb[WB_SW_INIT] = prev.wb[WB_SW_INIT];
    r.wb[WB_SNO_INIT] = prev.wb[WB_SNO_INIT];
    r
}

/// The basin's or a landscape unit's sums. `point` (landscape units) prints the plants' N and
/// P as the day's; the basin prints their averages.
#[derive(Clone, Debug)]
pub struct RegionAcc {
    /// The daily values' start (`bwb_d%sw_init`, kept from day to day when the daily table is
    /// printed).
    pub d_init: [f32; 2],
    wb: [[f32; 32]; 3],
    nb: [[f32; 13]; 3],
    ls: [[f32; 13]; 3],
    pw: [[f32; 25]; 3],
    point: bool,
    started: bool,
}

impl RegionAcc {
    pub fn new(point: bool) -> Self {
        RegionAcc {
            d_init: [0.0; 2],
            wb: [[0.0; 32]; 3],
            nb: [[0.0; 13]; 3],
            ls: [[0.0; 13]; 3],
            pw: [[0.0; 25]; 3],
            point,
            started: false,
        }
    }

    /// `basin_sw_init`: the region's soil water and snow at the start of the first printed
    /// year, from its HRUs' (`init` as summed with the same weights).
    pub fn start(&mut self, init: [f32; 2]) {
        if self.started {
            return;
        }
        self.started = true;
        self.d_init = init;
        for k in 0..3 {
            self.wb[k][WB_SW_INIT] = init[0];
            self.wb[k][WB_SNO_INIT] = init[1];
        }
    }

    /// The daily row as printed: soil water and snow the day's averages, and the start of the
    /// next day.
    pub fn daily_row(&mut self, d: &HruRows) -> HruRows {
        let mut r = HruRows { wb: d.wb, nb: d.nb, ls: d.ls, pw: d.pw };
        r.wb[WB_SW_INIT] = self.d_init[0];
        r.wb[WB_SNO_INIT] = self.d_init[1];
        r.wb[WB_SW] = (self.d_init[0] + d.wb[WB_SW_FINAL]) / 2.0;
        r.wb[WB_SNOPACK] = (self.d_init[1] + d.wb[WB_SNO_FINAL]) / 2.0;
        self.d_init = [d.wb[WB_SW_FINAL], d.wb[WB_SNO_FINAL]];
        r
    }

    pub fn day(&mut self, d: &HruRows, c: &PeriodDay, mut emit: impl FnMut(Period, &HruRows)) {
        let (m, y, a) = (0, 1, 2);
        add(&mut self.wb[m], &d.wb, &WB);
        add(&mut self.nb[m], &d.nb, &ALL_SUM_13);
        add(&mut self.ls[m], &d.ls, &ALL_SUM_13);
        add_pw(&mut self.pw[m], &d.pw);
        let point = |p: [f32; 25], on: bool| {
            let mut p = p;
            if on {
                p[PW_NPLT] = d.pw[PW_NPLT];
                p[PW_PPLNT] = d.pw[PW_PPLNT];
            }
            p
        };
        if c.end_mo {
            let (wbm, nbm, lsm, pwm) = (self.wb[m], self.nb[m], self.ls[m], self.pw[m]);
            add(&mut self.wb[y], &wbm, &WB);
            add(&mut self.nb[y], &nbm, &ALL_SUM_13);
            add(&mut self.ls[y], &lsm, &ALL_SUM_13);
            add_pw(&mut self.pw[y], &pwm);
            ave(&mut self.wb[m], c.mon_days, &WB);
            ave(&mut self.pw[m], c.mon_days, &PW);
            self.wb[m][WB_SW_FINAL] = d.wb[WB_SW_FINAL];
            self.wb[m][WB_SNO_FINAL] = d.wb[WB_SNO_FINAL];
            emit(Period::Mon, &HruRows { wb: self.wb[m], nb: self.nb[m], ls: self.ls[m], pw: point(self.pw[m], self.point) });
            let (sw, sno) = (self.wb[m][WB_SW_FINAL], self.wb[m][WB_SNO_FINAL]);
            self.wb[m] = [0.0; 32];
            self.wb[m][WB_SW_INIT] = sw;
            self.wb[m][WB_SNO_INIT] = sno;
            self.nb[m] = [0.0; 13];
            self.ls[m] = [0.0; 13];
            self.pw[m] = [0.0; 25];
        }
        if c.end_yr {
            let (wby, nby, lsy, pwy) = (self.wb[y], self.nb[y], self.ls[y], self.pw[y]);
            add(&mut self.wb[a], &wby, &WB);
            add(&mut self.nb[a], &nby, &ALL_SUM_13);
            add(&mut self.ls[a], &lsy, &ALL_SUM_13);
            add_pw(&mut self.pw[a], &pwy);
            ave(&mut self.wb[y], c.day_end_yr, &WB);
            ave(&mut self.pw[y], c.day_end_yr, &PW);
            self.wb[y][WB_SW_FINAL] = d.wb[WB_SW_FINAL];
            self.wb[y][WB_SNO_FINAL] = d.wb[WB_SNO_FINAL];
            emit(Period::Yr, &HruRows { wb: self.wb[y], nb: self.nb[y], ls: self.ls[y], pw: point(self.pw[y], self.point) });
            let (sw, sno) = (self.wb[y][WB_SW_FINAL], self.wb[y][WB_SNO_FINAL]);
            self.wb[y] = [0.0; 32];
            self.wb[y][WB_SW_INIT] = sw;
            self.wb[y][WB_SNO_INIT] = sno;
            self.nb[y] = [0.0; 13];
            self.ls[y] = [0.0; 13];
            self.pw[y] = [0.0; 25];
        }
        if c.end_sim {
            let (sw_init, sno_init) = (self.wb[a][WB_SW_INIT], self.wb[a][WB_SNO_INIT]);
            let mut wba = self.wb[a];
            div(&mut wba, c.yrs_prt, &WB);
            ave(&mut wba, c.days_prt, &WB);
            wba[WB_SW_INIT] = sw_init;
            wba[WB_SW_FINAL] = d.wb[WB_SW_FINAL];
            wba[WB_SNO_INIT] = sno_init;
            wba[WB_SNO_FINAL] = d.wb[WB_SNO_FINAL];
            let mut nba = self.nb[a];
            div(&mut nba, c.yrs_prt, &ALL_SUM_13);
            let mut lsa = self.ls[a];
            div(&mut lsa, c.yrs_prt, &ALL_SUM_13);
            let mut pwa = self.pw[a];
            div(&mut pwa, c.yrs_prt, &PW);
            ave(&mut pwa, c.days_prt, &PW);
            emit(Period::Aa, &HruRows { wb: wba, nb: nba, ls: lsa, pw: point(pwa, self.point) });
        }
    }
}

/// `aqu * const` (`aqu_mult`): all but the carbon.
pub fn aqu_scaled(a: &AquDay, c: f32) -> AquDay {
    let mut s = *a;
    s.flo = a.flo * c;
    s.dep_wt = a.dep_wt * c;
    s.stor = a.stor * c;
    s.no3_st = a.no3_st * c;
    s.minp = a.minp * c;
    s.orgn = a.orgn * c;
    s.rchrg = a.rchrg * c;
    s.no3_rchg = a.no3_rchg * c;
    s.no3_loss = a.no3_loss * c;
    s.seep = a.seep * c;
    s.revap = a.revap * c;
    s.no3_lat = a.no3_lat * c;
    s.no3_seep = a.no3_seep * c;
    s.flo_cha = a.flo_cha * c;
    s.flo_res = a.flo_res * c;
    s.flo_ls = a.flo_ls * c;
    s
}

/// `basin_aquifer_output`'s daily sum: each aquifer's state weighted by its share of the
/// basin's area (`aqu_add`: the carbon is the last aquifer's).
pub fn aqu_sum(acc: &mut AquDay, a: &AquDay, c: f32) {
    aqu_add(acc, &aqu_scaled(a, c));
}

/// The basin's SWAT-DEG channel sums (`basin_sdchannel_output`): the day's storage, inflow,
/// outflow and water body of all channels added; inflow and outflow summed over the period, the
/// water body's area averaged over the days; the average annual divided by the years.
#[derive(Clone, Debug, Default)]
pub struct BasinChaAcc {
    hin: [Hyd; 3],
    hout: [Hyd; 3],
    wat: [ChWat; 3],
}

impl BasinChaAcc {
    #[allow(clippy::type_complexity)]
    pub fn day(&mut self, w: &ChWat, hin: &Hyd, hout: &Hyd, c: &PeriodDay, mut emit: impl FnMut(Period, &ChWat, &Hyd, &Hyd)) {
        let (m, y, a) = (0, 1, 2);
        self.hin[m] = self.hin[m].plus(*hin);
        self.hout[m] = self.hout[m].plus(*hout);
        wat_add(&mut self.wat[m], w);
        if c.end_mo {
            self.hin[y] = self.hin[y].plus(self.hin[m]);
            self.hout[y] = self.hout[y].plus(self.hout[m]);
            let wm = self.wat[m];
            wat_add(&mut self.wat[y], &wm);
            self.wat[m].area_ha /= c.mon_days;
            emit(Period::Mon, &self.wat[m], &self.hin[m], &self.hout[m]);
            self.hin[m] = Hyd::default();
            self.hout[m] = Hyd::default();
            self.wat[m] = ChWat::default();
        }
        if c.end_yr {
            self.hin[a] = self.hin[a].plus(self.hin[y]);
            self.hout[a] = self.hout[a].plus(self.hout[y]);
            let wy = self.wat[y];
            wat_add(&mut self.wat[a], &wy);
            self.wat[y].area_ha /= c.day_end_yr;
            emit(Period::Yr, &self.wat[y], &self.hin[y], &self.hout[y]);
            self.hin[y] = Hyd::default();
            self.hout[y] = Hyd::default();
            self.wat[y] = ChWat::default();
        }
        if c.end_sim {
            let k = c.yrs_prt;
            let wa = &self.wat[a];
            let wat = ChWat { area_ha: wa.area_ha / k, precip: wa.precip / k, evap: wa.evap / k, seep: wa.seep / k };
            emit(Period::Aa, &wat, &self.hin[a].over(k), &self.hout[a].over(k));
        }
    }
}

/// The basin's water body sum of a day.
pub fn wat_sum(acc: &mut ChWat, w: &ChWat) {
    wat_add(acc, w);
}
