//! Connection files, `*.con` (SWAT+ `hyd_read_connect`).

use swat_model::{ObjectKind, Outflow, SpatialObject};

use crate::list::Records;
use crate::{Error, Result};

/// Values before the outflows: id, name, gis_id, area, lat, lon, elev, props, wst, cst, ovfl,
/// rule, out_tot.
const FIXED: usize = 13;

/// Reads a connection file of objects of `kind`. Each row is read twice, as SWAT+ does: once for
/// its fixed values, then again with as many outflows (type, number, part, fraction) as `out_tot`
/// says, so outflows may run on into the next lines.
pub fn read(kind: ObjectKind, file: &str, text: &str) -> Result<Vec<SpatialObject>> {
    let mut rec = Records::new(file, text);
    rec.skip()?;
    rec.skip()?;
    let mut out = Vec::new();
    loop {
        let mark = rec.mark();
        let fixed = match rec.read(FIXED) {
            Ok(f) => f,
            Err(Error::Eof { .. }) => break,
            Err(e) => return Err(e),
        };
        let line = mark;
        let n_out = fixed.int_or(12, 0)?.clamp(0, 10_000);
        let f = if n_out > 0 {
            rec.reset(line);
            rec.read(FIXED + 4 * n_out as usize)?
        } else {
            fixed
        };
        let mut obj = SpatialObject {
            kind,
            id: f.int_or(0, 0)?,
            name: f.text_or(1, ""),
            gis_id: f.int_or(2, 0)?,
            area_ha: f.real_or(3, 0.0)?,
            lat: f.real_or(4, 0.0)?,
            lon: f.real_or(5, 0.0)?,
            elev: f.real_or(6, 0.0)?,
            props: f.int_or(7, 0)?,
            weather_station: f.text_or(8, ""),
            constit: f.int_or(9, 0)?,
            props2: f.int_or(10, 0)?,
            ruleset: f.text_or(11, ""),
            outflows: Vec::with_capacity(n_out as usize),
        };
        for k in 0..n_out as usize {
            let base = FIXED + 4 * k;
            obj.outflows.push(Outflow {
                obj_type: f.text_or(base, ""),
                obj_id: f.int_or(base + 1, 0)?,
                hyd_type: f.text_or(base + 2, ""),
                frac: f.real_or(base + 3, 0.0)?,
            });
        }
        out.push(obj);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "rout_unit.con: x\nid name gis_id area lat lon elev rtu wst cst ovfl rule out_tot obj_typ obj_id hyd_typ frac\n";

    #[test]
    fn outflows() {
        let text = format!("{HEAD}1 rtu001 1 10.0 35.5 127.3 113.5 1 s35610n127290e 0 0 0 2 sdc 1 tot 1.0 aqu 1 rhg 1.0\n");
        let objs = read(ObjectKind::RoutingUnit, "rout_unit.con", &text).unwrap();
        assert_eq!(objs.len(), 1);
        let o = &objs[0];
        assert_eq!((o.id, o.name.as_str(), o.props), (1, "rtu001", 1));
        assert_eq!(o.weather_station, "s35610n127290e");
        assert_eq!(o.outflows.len(), 2);
        assert_eq!(o.outflows[1].obj_type, "aqu");
        assert_eq!(o.outflows[1].hyd_type, "rhg");
    }

    #[test]
    fn rows_without_outflows_and_continued_rows() {
        let text = format!(
            "{HEAD}1 hru0001 1 1.005 41.20 -96.63 347.5 1 wgn_01 0 0 0 0\n\
             2 hru0002 1 1.005 41.20 -96.63 347.5 2 wgn_01 0 0 0 1\n\
             sdc 3 tot 0.5\n"
        );
        let objs = read(ObjectKind::Hru, "hru.con", &text).unwrap();
        assert_eq!(objs.len(), 2);
        assert!(objs[0].outflows.is_empty());
        assert_eq!(objs[1].outflows[0].obj_id, 3);
        assert_eq!(objs[1].outflows[0].frac, 0.5);
    }
}
