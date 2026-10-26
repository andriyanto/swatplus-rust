//! Readers for routing units (`ru_read`, `ru_read_elements`) and aquifers (`aqu_read`).

use swat_model::cal::define_unit_elements;
use swat_model::routing::{AquiferDb, RuData, RuDef, RuElement, SdChannelData};

use crate::list::Records;
use crate::{Error, Result};

/// Counts the records after the two header lines (blank lines don't count), then rewinds there.
fn count_rows(rec: &mut Records) -> Result<usize> {
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut n = 0;
    while rec.skip().is_ok() {
        n += 1;
    }
    rec.reset(body);
    Ok(n)
}

/// `rout_unit.ele`: each row put at its own number (`read i`, then the row again).
pub fn read_ru_elements(file: &str, text: &str) -> Result<Vec<RuElement>> {
    let mut rec = Records::new(file, text);
    let body = {
        rec.skip()?;
        rec.skip()?;
        rec.mark()
    };
    let mut imax = 0usize;
    let mut n = 0usize;
    loop {
        match rec.read(1) {
            Ok(f) => {
                let i = f.int_or(0, 0)?;
                if i == 0 {
                    break;
                }
                imax = imax.max(i.max(0) as usize);
                n += 1;
            }
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        }
    }
    rec.reset(body);
    let mut out = vec![RuElement::default(); imax + 1];
    for _ in 0..n {
        let f = match rec.read(6) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let i = f.int_or(0, 0)?.max(0) as usize;
        if let Some(e) = out.get_mut(i) {
            *e = RuElement {
                name: f.text_or(1, ""),
                obtyp: f.text_or(2, ""),
                obtypno: f.int_or(3, 0)?,
                frac: f.real_or(4, 0.0)?,
                dr_name: f.text_or(5, ""),
            };
        }
    }
    Ok(out)
}

/// `rout_unit.def`: a row per routing unit, in order: its number, name, the number of entries of
/// its element list and the list (`define_unit_elements`).
pub fn read_ru_defs(file: &str, text: &str, n_ru: usize) -> Result<Vec<RuDef>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let mut out = Vec::with_capacity(n_ru);
    for _ in 0..n_ru {
        let mark = rec.mark();
        let f = match rec.read(3) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let nspu = f.int_or(2, 0)?.max(0) as usize;
        let mut d = RuDef::default();
        if nspu > 0 {
            rec.reset(mark);
            let f = rec.read(3 + nspu)?;
            d.name = f.text_or(1, "");
            let cnt = (0..nspu).map(|k| f.int_or(3 + k, 0)).collect::<Result<Vec<i32>>>()?;
            d.elems = define_unit_elements(&cnt);
        }
        out.push(d);
    }
    Ok(out)
}

/// `ls_unit.def` (`lsu_read_elements`): a title, the number of units, a header, then per unit
/// its number, name, area, the length of its element list and the list.
pub fn read_lsu_defs(file: &str, text: &str) -> Result<Vec<swat_model::routing::LsuDef>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    let mlsu = rec.read(1)?.int_or(0, 0)?.max(0) as usize;
    rec.skip()?;
    let mut out = Vec::with_capacity(mlsu);
    for _ in 0..mlsu {
        let mark = rec.mark();
        let f = match rec.read(4) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let nspu = f.int_or(3, 0)?.max(0) as usize;
        let mut d = swat_model::routing::LsuDef { name: f.text_or(1, ""), area_ha: f.real_or(2, 0.0)?, elems: Vec::new() };
        if nspu > 0 {
            rec.reset(mark);
            let f = rec.read(4 + nspu)?;
            let cnt = (0..nspu).map(|k| f.int_or(4 + k, 0)).collect::<Result<Vec<i32>>>()?;
            d.elems = define_unit_elements(&cnt);
        }
        out.push(d);
    }
    Ok(out)
}

/// `ls_unit.ele` (`lsu_read_elements`): by row number, element 0 unused.
pub fn read_lsu_elements(file: &str, text: &str) -> Result<Vec<swat_model::routing::LsuElem>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0usize;
    let mut n = 0usize;
    loop {
        match rec.read(1) {
            Ok(f) => {
                imax = imax.max(f.int_or(0, 0)?.max(0) as usize);
                n += 1;
            }
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        }
    }
    rec.reset(body);
    let mut out = vec![swat_model::routing::LsuElem::default(); imax + 1];
    for _ in 0..n {
        let f = match rec.read(6) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let i = f.int_or(0, 0)?.max(0) as usize;
        if let Some(e) = out.get_mut(i) {
            *e = swat_model::routing::LsuElem {
                name: f.text_or(1, ""),
                obtyp: f.text_or(2, ""),
                obtypno: f.int_or(3, 0)?,
                bsn_frac: f.real_or(4, 0.0)?,
                ru_frac: f.real_or(5, 0.0)?,
            };
        }
    }
    Ok(out)
}

/// `rout_unit.rtu`: each routing unit's names of definition, delivery ratio, topography and field.
pub fn read_ru_data(file: &str, text: &str) -> Result<Vec<RuData>> {
    let mut rec = Records::new(file, text);
    let n = count_rows(&mut rec)?;
    let mut out = Vec::new();
    for _ in 0..n {
        let f = match rec.read(6) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let i = f.int_or(0, 0)?.max(1) as usize;
        if out.len() < i {
            out.resize(i, RuData::default());
        }
        out[i - 1] = RuData {
            name: f.text_or(1, ""),
            elem_def: f.text_or(2, ""),
            elem_dr: f.text_or(3, ""),
            topo: f.text_or(4, ""),
            field: f.text_or(5, ""),
        };
    }
    Ok(out)
}

/// `aquifer.aqu`: each row put at its own number; element 0 keeps the defaults.
pub fn read_aquifers(file: &str, text: &str) -> Result<Vec<AquiferDb>> {
    let mut rec = Records::new(file, text);
    let n = count_rows(&mut rec)?;
    let mut out = vec![AquiferDb::default()];
    for _ in 0..n {
        let Some((f, eof)) = rec.read_partial(18) else { break };
        let i = f.int_or(0, 0)?.max(0) as usize;
        if out.len() <= i {
            out.resize(i + 1, AquiferDb::default());
        }
        let d = AquiferDb::default();
        let r = |k: usize, v: f32| f.real_or(k, v);
        out[i] = AquiferDb {
            name: f.text_or(1, ""),
            init: f.text_or(2, ""),
            flo: r(3, d.flo)?,
            dep_bot: r(4, d.dep_bot)?,
            dep_wt: r(5, d.dep_wt)?,
            no3: r(6, d.no3)?,
            minp: r(7, d.minp)?,
            cbn: r(8, d.cbn)?,
            flo_dist: r(9, d.flo_dist)?,
            bf_max: r(10, d.bf_max)?,
            alpha: r(11, d.alpha)?,
            revap_co: r(12, d.revap_co)?,
            seep: r(13, d.seep)?,
            spyld: r(14, d.spyld)?,
            hlife_n: r(15, d.hlife_n)?,
            flo_min: r(16, d.flo_min)?,
            revap_min: r(17, d.revap_min)?,
        };
        if eof {
            break;
        }
    }
    Ok(out)
}

/// `channel-lte.cha`: each row put at its own number (`sd_channel_read`).
pub fn read_sd_channels(file: &str, text: &str) -> Result<Vec<SdChannelData>> {
    let mut rec = Records::new(file, text);
    let n = count_rows(&mut rec)?;
    let mut out = vec![SdChannelData::default()];
    for _ in 0..n {
        let f = match rec.read(6) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let i = f.int_or(0, 0)?.max(0) as usize;
        if out.len() <= i {
            out.resize(i + 1, SdChannelData::default());
        }
        out[i] = SdChannelData {
            name: f.text_or(1, ""),
            init: f.text_or(2, ""),
            hyd: f.text_or(3, ""),
            sed: f.text_or(4, ""),
            nut: f.text_or(5, ""),
        };
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_unit_files() {
        let e = read_ru_elements("rout_unit.ele", "t\nh\n1 hru1 hru 1 0.5 0\n2 hru2 hru 2 0.5 0\n").unwrap();
        assert_eq!((e.len(), e[2].obtypno, e[2].frac, e[2].dr_name.as_str()), (3, 2, 0.5, "0"));
        let d = read_ru_defs("rout_unit.def", "t\nh\n1 rtu1 2 1 -2\n", 1).unwrap();
        assert_eq!(d[0].elems, [1, 2]);
        let a = read_aquifers("aquifer.aqu", "t\nh\n1 aqu1 init 0.05 10 3 0 0 0.5 50 1 0.05 0.02 0.05 0.05 30 3 5\n").unwrap();
        assert_eq!((a[1].name.as_str(), a[1].alpha, a[1].revap_min), ("aqu1", 0.05, 5.0));
    }
}
