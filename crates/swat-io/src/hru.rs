//! `hru-data.hru` (`hru_read`), `soils.sol` (`soil_db_read`) and `soil_plant.ini`
//! (`soil_plant_init`).

use swat_model::{
    DecisionTable, DtblAct, DtblCond, HruData, MgtOp, MgtSchedule, PlantCommunity, PlantInit, Soil, SoilLayer, SoilPlantInit,
};

use crate::list::Records;
use crate::{Error, Result};

/// Reads `hru-data.hru`. SWAT+ sizes the table by the largest HRU number, then reads that many
/// rows, each placed at its own number; numbers not given keep an empty row.
pub fn read_hru_data(text: &str) -> Result<Vec<HruData>> {
    let mut rec = Records::new("hru-data.hru", text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0;
    loop {
        match rec.read(1) {
            Ok(f) => match f.int(0) {
                Ok(Some(i)) => imax = imax.max(i),
                // A value that isn't an integer: SWAT+ ignores the error and keeps counting.
                Ok(None) | Err(Error::Parse { .. }) => {}
                Err(e) => return Err(e),
            },
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        }
    }
    let imax = imax.clamp(0, 1_000_000) as usize;
    let mut out = vec![HruData::default(); imax];
    rec.reset(body);
    for _ in 0..imax {
        let Some((f, eof)) = rec.read_partial(10) else { break };
        let Ok(Some(i)) = f.int(0) else { break };
        if let Some(h) = (i as usize).checked_sub(1).and_then(|k| out.get_mut(k)) {
            let t = |k: usize| f.text_or(k, "");
            *h = HruData {
                id: i,
                name: t(1),
                topo: t(2),
                hyd: t(3),
                soil: t(4),
                land_use_mgt: t(5),
                soil_plant_init: t(6),
                surf_stor: t(7),
                snow: t(8),
                field: t(9),
            };
        }
        if eof {
            break;
        }
    }
    Ok(out)
}

/// Reads `soils.sol`: per soil a record of name, layers, hydrologic group, rooting depth, anion
/// exclusion, crack volume and texture, then one record per layer. A first layer thinner than
/// 20 mm becomes 20 mm when the soil has one layer or its second layer is deeper than 20 mm.
pub fn read_soils(text: &str) -> Result<Vec<Soil>> {
    let mut rec = Records::new("soils.sol", text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    // Count the soils: a record of name and layer count, then that many layer records.
    let mut imax = 0;
    while let Ok(f) = rec.read(2) {
        let nly = f.int(1).ok().flatten().unwrap_or(0).max(0);
        let mut ok = true;
        for _ in 0..nly {
            if rec.skip().is_err() {
                ok = false;
                break;
            }
        }
        if !ok {
            break;
        }
        imax += 1;
    }
    rec.reset(body);
    let mut out = Vec::with_capacity(imax);
    for _ in 0..imax {
        let mark = rec.mark();
        let Ok(head) = rec.read(2) else { break };
        let nly = head.int(1)?.unwrap_or(0).clamp(0, 100);
        rec.reset(mark);
        let Some((h, _)) = rec.read_partial(7) else { break };
        let mut soil = Soil {
            name: h.text_or(0, ""),
            nly,
            hydgrp: h.text_or(2, ""),
            zmx: h.real_or(3, 0.0)?,
            anion_excl: h.real_or(4, 0.0)?,
            crk: h.real_or(5, 0.0)?,
            texture: h.text_or(6, ""),
            layers: Vec::with_capacity(nly as usize),
        };
        for _ in 0..nly {
            let Some((l, _)) = rec.read_partial(14) else { break };
            let v = |k: usize| l.real_or(k, 0.0);
            soil.layers.push(SoilLayer {
                z: v(0)?,
                bd: v(1)?,
                awc: v(2)?,
                k: v(3)?,
                cbn: v(4)?,
                clay: v(5)?,
                silt: v(6)?,
                sand: v(7)?,
                rock: v(8)?,
                alb: v(9)?,
                usle_k: v(10)?,
                ec: v(11)?,
                cal: v(12)?,
                ph: v(13)?,
            });
        }
        let second = soil.layers.get(1).map(|l| l.z);
        if let Some(first) = soil.layers.first_mut()
            && first.z < 20.0
            && (soil.nly == 1 || second.is_some_and(|z| z > 20.0))
        {
            first.z = 20.0;
        }
        out.push(soil);
    }
    Ok(out)
}

/// Reads `soil_plant.ini`: name, initial soil water fraction and the names of the nutrient,
/// pesticide, pathogen, salt and heavy metal initial tables, and the constituent table when
/// `codes.bsn` turns `nam1` on.
pub fn read_soil_plant_init(text: &str, nam1: i32) -> Result<Vec<SoilPlantInit>> {
    let mut rec = Records::new("soil_plant.ini", text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0;
    while rec.skip().is_ok() {
        imax += 1;
    }
    rec.reset(body);
    let n = if nam1 == 0 { 7 } else { 8 };
    let mut out = Vec::new();
    for _ in 0..imax {
        let Some((f, eof)) = rec.read_partial(n) else { break };
        out.push(SoilPlantInit {
            name: f.text_or(0, ""),
            sw_frac: f.real_or(1, 0.0)?,
            nutc: f.text_or(2, ""),
            pestc: f.text_or(3, ""),
            pathc: f.text_or(4, ""),
            saltc: f.text_or(5, ""),
            hmetc: f.text_or(6, ""),
            csc: f.text_or(7, ""),
        });
        if eof {
            break;
        }
    }
    Ok(out)
}

/// Reads `plant.ini` (`readpcom`): per community a record of name, number of plants and first
/// rotation year, then a record per plant. Communities are counted first, as SWAT+ does.
pub fn read_plant_communities(text: &str) -> Result<Vec<PlantCommunity>> {
    let mut rec = Records::new("plant.ini", text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0;
    'count: while let Ok(f) = rec.read(2) {
        let n = f.int(1).ok().flatten().unwrap_or(0).max(0);
        for _ in 0..n {
            if rec.skip().is_err() {
                break 'count;
            }
        }
        imax += 1;
    }
    rec.reset(body);
    let mut out = Vec::with_capacity(imax);
    for _ in 0..imax {
        let Some((h, _)) = rec.read_partial(3) else { break };
        let n = h.int(1)?.unwrap_or(1).clamp(0, 100);
        let mut com = PlantCommunity { name: h.text_or(0, "frsd_frsd"), rot_yr_ini: h.int(2)?.unwrap_or(1), plants: Vec::new() };
        for _ in 0..n {
            let Some((p, _)) = rec.read_partial(8) else { break };
            let d = PlantInit::default();
            com.plants.push(PlantInit {
                cpnm: p.text_or(0, &d.cpnm),
                igro: p.text_or(1, &d.igro).chars().take(1).collect(),
                lai: p.real_or(2, d.lai)?,
                bioms: p.real_or(3, d.bioms)?,
                phuacc: p.real_or(4, d.phuacc)?,
                pop: p.real_or(5, d.pop)?,
                fr_yrmat: p.real_or(6, d.fr_yrmat)?,
                rsdin: p.real_or(7, d.rsdin)?,
            });
        }
        out.push(com);
    }
    Ok(out)
}

/// Reads `management.sch` as `mgt_read_mgtops` does: a count pass (name, number of operations
/// and of decision tables, then one record each), then the schedules. Each operation is one
/// list-directed read of seven values; SWAT+ stops on a bad one, and so does this.
pub fn read_management(text: &str) -> Result<Vec<MgtSchedule>> {
    let file = "management.sch";
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let body = rec.mark();
    let mut imax = 0;
    'count: while let Ok(f) = rec.read(3) {
        let nops = f.int(1).ok().flatten().unwrap_or(0).max(0);
        let nauto = f.int(2).ok().flatten().unwrap_or(0).max(0);
        for _ in 0..nauto + nops {
            if rec.skip().is_err() {
                break 'count;
            }
        }
        imax += 1;
    }
    rec.reset(body);
    let mut out = Vec::with_capacity(imax);
    for _ in 0..imax {
        let Some((h, _)) = rec.read_partial(3) else { break };
        let mut sched = MgtSchedule { name: h.text_or(0, ""), ..MgtSchedule::default() };
        let nops = h.int(1)?.unwrap_or(0).max(0);
        let nauto = h.int(2)?.unwrap_or(0).max(0);
        for _ in 0..nauto {
            let mark = rec.mark();
            let name = match rec.skip() {
                Ok(name) => name.unwrap_or_default(),
                Err(_) => break,
            };
            // A generic plant-and-harvest table: the record again, with its crops.
            let ncrop = match name.as_str() {
                "pl_hv_summer1" | "pl_hv_winter1" => 1,
                "pl_hv_summer2" => 2,
                _ => 0,
            };
            if ncrop > 0 {
                rec.reset(mark);
                let f = rec.read(1 + ncrop)?;
                sched.auto_crops = (0..ncrop).map(|k| f.text_or(1 + k, "")).collect();
            }
            sched.autos.push(name);
        }
        for _ in 0..nops {
            let f = rec.read(7)?;
            sched.ops.push(MgtOp {
                op: f.text_or(0, ""),
                mon: f.int_or(1, 0)?,
                day: f.int_or(2, 0)?,
                husc: f.real_or(3, 0.0)?,
                op_char: f.text_or(4, ""),
                op_plant: f.text_or(5, ""),
                op3: f.real_or(6, 0.0)?,
            });
        }
        out.push(sched);
    }
    Ok(out)
}

/// Reads a decision table file (`lum.dtl`) as `dtbl_lum_read` does: a title, the number of
/// tables, a record skipped, then each table (a header, its name and sizes, a header, the
/// conditions with their alternatives, a header, the actions with their outcomes).
pub fn read_decision_tables(file: &str, text: &str) -> Result<Vec<DecisionTable>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    let n = rec.read(1)?.int(0)?.unwrap_or(0).max(0);
    rec.read(0)?;
    let mut out = Vec::new();
    for _ in 0..n {
        if rec.skip().is_err() {
            break;
        }
        let h = rec.read(4)?;
        let conds = h.int(1)?.unwrap_or(0).max(0) as usize;
        let alts = h.int(2)?.unwrap_or(0).max(0) as usize;
        let acts = h.int(3)?.unwrap_or(0).max(0) as usize;
        let mut t = DecisionTable { name: h.text_or(0, ""), alts, ..DecisionTable::default() };
        rec.skip()?;
        for _ in 0..conds {
            let f = rec.read(6 + alts)?;
            let d = DtblCond::default();
            t.conds.push(DtblCond {
                var: f.text_or(0, &d.var),
                ob: f.text_or(1, &d.ob),
                ob_num: f.int_or(2, 0)?,
                lim_var: f.text_or(3, &d.lim_var),
                lim_op: f.text_or(4, &d.lim_op),
                lim_const: f.real_or(5, 0.0)?,
            });
            t.alt.push((0..alts).map(|a| f.text_or(6 + a, "")).collect());
        }
        rec.skip()?;
        for _ in 0..acts {
            let f = rec.read(8 + alts)?;
            t.acts.push(DtblAct {
                typ: f.text_or(0, ""),
                ob: f.text_or(1, ""),
                ob_num: f.int_or(2, 0)?,
                name: f.text_or(3, ""),
                option: f.text_or(4, ""),
                const_: f.real_or(5, 0.0)?,
                const2: f.real_or(6, 1.0)?.max(1.0),
                file_pointer: f.text_or(7, ""),
            });
            t.outcomes.push((0..alts).map(|a| f.text_or(8 + a, "")).collect());
        }
        out.push(t);
    }
    Ok(out)
}

/// Reads `soil_lyr_depths.sol` (`soils_init`): three header records, then one depth (mm, an
/// integer) per record to the end of the file.
pub fn read_soil_lyr_depths(text: &str) -> Result<Vec<i32>> {
    let mut rec = Records::new("soil_lyr_depths.sol", text);
    rec.skip()?;
    rec.skip()?;
    rec.skip()?;
    let mut out = Vec::new();
    while let Ok(f) = rec.read(1) {
        out.push(f.int(0)?.unwrap_or(0));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn management_schedules() {
        let text = "management.sch\nheader\nm1 2 1\nauto1\n till 4 1 0.0 chisplow null 0.0\t\n\
                    skip 0 0 0 null null 0\nm2 1 0\nfert 6 1 0.0 null elem_n 181.4\n";
        let m = read_management(text).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].autos, ["auto1"]);
        assert_eq!(m[0].ops[0].op_char, "chisplow");
        assert_eq!(m[0].ops[1].op, "skip");
        assert_eq!(m[1].ops[0].op3, 181.4);
        assert_eq!(m[1].ops[0].op_plant, "elem_n");
    }

    #[test]
    fn hru_rows_by_number() {
        let text = "hru-data.hru\nid name topo hyd soil lu ini surf snow field\n\
                    2 hru2 t2 h2 s2 lu2 i2 null sn null\n1 hru1 t1 h1 s1 lu1 i1 null sn null\n";
        let h = read_hru_data(text).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].name, "hru1");
        assert_eq!(h[1].land_use_mgt, "lu2");
    }

    #[test]
    fn soils_and_thin_first_layer() {
        let text = "soils.sol\nheader\n\
                    s1 2 B 2000.0 0.5 0.5 L- L- L\n\
                    10.0 1.3 0.2 30 2 20 30 50 0 0.16 0.2 0 0 6\n\
                    300.0 1.4 0.2 30 1 20 30 50 0 0.16 0.2 0 0 6\n\
                    s2 1 C 1000.0 0.5 0.5 CL\n\
                    500.0 1.4 0.2 30 1 20 30 50 0 0.16 0.2 0 0 6\n";
        let s = read_soils(text).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].hydgrp.as_str(), s[0].texture.as_str()), ("B", "L-"));
        assert_eq!(s[0].layers[0].z, 20.0);
        assert_eq!(s[1].layers.len(), 1);
        assert_eq!(s[1].layers[0].k, 30.0);
    }
}
