//! Hydrographs: the water an object passes on in a day and what it carries (`hyd_output`), with
//! the operators SWAT+ defines on them (`hydrograph_module`).

/// One day's water (m3) and what it carries: sediment and its classes (t), nutrients (kg),
/// chlorophyll-a, CBOD and oxygen, and the water's temperature.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hyd {
    pub flo: f32,
    pub sed: f32,
    pub orgn: f32,
    pub sedp: f32,
    pub no3: f32,
    pub solp: f32,
    pub chla: f32,
    pub nh3: f32,
    pub no2: f32,
    pub cbod: f32,
    pub dox: f32,
    pub san: f32,
    pub sil: f32,
    pub cla: f32,
    pub sag: f32,
    pub lag: f32,
    pub grv: f32,
    pub temp: f32,
}

impl Hyd {
    /// The 17 amounts, without the temperature, in file order.
    fn amounts(&self) -> [f32; 17] {
        [
            self.flo, self.sed, self.orgn, self.sedp, self.no3, self.solp, self.chla, self.nh3, self.no2, self.cbod, self.dox, self.san,
            self.sil, self.cla, self.sag, self.lag, self.grv,
        ]
    }

    fn from_amounts(a: [f32; 17], temp: f32) -> Hyd {
        Hyd {
            flo: a[0],
            sed: a[1],
            orgn: a[2],
            sedp: a[3],
            no3: a[4],
            solp: a[5],
            chla: a[6],
            nh3: a[7],
            no2: a[8],
            cbod: a[9],
            dox: a[10],
            san: a[11],
            sil: a[12],
            cla: a[13],
            sag: a[14],
            lag: a[15],
            grv: a[16],
            temp,
        }
    }

    /// `hyd1 + hyd2` (`hydout_add`): the amounts add, the temperature is the flow-weighted mean
    /// (0 when the two flows are under 1e-6 together).
    pub fn plus(self, o: Hyd) -> Hyd {
        let (a, b) = (self.amounts(), o.amounts());
        let s = std::array::from_fn(|k| a[k] + b[k]);
        let temp = if self.flo + o.flo > 1.0e-6 { (self.flo * self.temp + o.flo * o.temp) / (self.flo + o.flo) } else { 0.0 };
        Hyd::from_amounts(s, temp)
    }

    /// `const * hyd` (`hydout_mult_const`); the temperature is kept.
    pub fn scale(k: f32, h: Hyd) -> Hyd {
        let a = h.amounts();
        Hyd::from_amounts(std::array::from_fn(|i| k * a[i]), h.temp)
    }

    /// `hyd1 ** hyd2` (`hydout_mult`): amount by amount; the temperature is the first's.
    pub fn times(self, o: Hyd) -> Hyd {
        let (a, b) = (self.amounts(), o.amounts());
        Hyd::from_amounts(std::array::from_fn(|k| a[k] * b[k]), self.temp)
    }

    /// `hyd / const` (`hydout_div_const`); the temperature is kept.
    pub fn over(self, k: f32) -> Hyd {
        let a = self.amounts();
        Hyd::from_amounts(std::array::from_fn(|i| a[i] / k), self.temp)
    }

    /// `const .add. hyd` (`hydout_add_const`); the temperature is kept.
    pub fn add_const(k: f32, h: Hyd) -> Hyd {
        let a = h.amounts();
        Hyd::from_amounts(std::array::from_fn(|i| k + a[i]), h.temp)
    }

    /// The columns of a hydrograph table (`hyd_hdr`): the temperature's is headed `null`.
    pub fn columns(&self) -> [(&'static str, f32); 18] {
        let a = self.amounts();
        const NAMES: [&str; 17] =
            ["flo", "sed", "orgn", "sedp", "no3", "solp", "chla", "nh3", "no2", "cbod", "dox", "san", "sil", "cla", "sag", "lag", "grv"];
        let mut out = [("null", self.temp); 18];
        for k in 0..17 {
            out[k] = (NAMES[k], a[k]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_mixes_by_flow() {
        let a = Hyd { flo: 1.0, temp: 10.0, no3: 2.0, ..Hyd::default() };
        let b = Hyd { flo: 3.0, temp: 20.0, no3: 1.0, ..Hyd::default() };
        let c = a.plus(b);
        assert_eq!((c.flo, c.no3, c.temp), (4.0, 3.0, 17.5));
        assert_eq!(Hyd::default().plus(Hyd::default()).temp, 0.0);
        assert_eq!(Hyd::scale(2.0, a).temp, 10.0);
    }
}
