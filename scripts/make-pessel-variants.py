#!/usr/bin/env python3
"""Builds the phase 3e validation scenario from testdata/Pessel (docs/phase3.md).

  Ps_coast  The coastal lowland of Pessel: the 20 small catchments whose land is all under 20 m
            and which drain straight to the sea (74 SWAT-DEG channels, 74 routing units, 115
            HRUs with oil palm, peat swamp forest, mangrove and other wetlands, rice; 23 shallow
            and 20 deep aquifers), cut out of the project with everything renumbered. 2015-2017,
            every day printed (no warm-up), to keep the run and its tables small.

Objects are kept in their order in the project. Files pointed to by row number (hru-data.hru,
aquifer.aqu, channel-lte.cha, rout_unit.def/.ele/.rtu) are cut down and renumbered; files
pointed to by name are copied whole, but for the wetlands (wetland.wet, hydrology.wet), cut to
those kept since a quirk reads them by row. The landscape units (ls_unit.def/.ele, one per routing unit
in Pessel) are cut down the same way, with the HRUs' basin fractions taken anew over the
variant's area; the aquifer catchments only feed aggregated outputs and are dropped (file.cio
null). Hard calibration (calibration.cal) keeps its HRUs, renumbered.

  Ps_paddy  Ps_coast with its rice HRUs as rice paddies (SWAT+'s paddy module, phase 4): a weir
            raised for two seasons a year (IP 200), ponding irrigation, puddling, transplanted
            120-day rice, split urea and SP-36. The parameters and their sources are in
            docs/paddy.md.

  Ps_musk   Ps_coast with its channels routed by Muskingum (codes.bsn rte_cha 1) instead of the
            variable storage method.

  Ps_atmo   Ps_coast with monthly nitrogen deposition (atmo.cli), 2015-2017: two deposition
            stations for its weather stations in turn; dry deposition three times higher in the
            haze of September-October 2015 (docs/atmo.md).

  Ps_tile   Ps_coast with its oil palm drained (tiledrain.str peat_canal): drains 600 mm deep,
            the water table Indonesian peat rules keep under 0.4 m (docs/structures.md).

usage: scripts/make-pessel-variants.py <Pessel folder> <output folder>
"""
import os
import shutil
import sys

MAX_ELEV = 20.0


def read(path):
    with open(path, newline="") as f:
        return f.read().split("\n")


def write(path, lines):
    with open(path, "w", newline="") as f:
        f.write("\n".join(lines))


def set_field(line, k, text):
    """Replace the k-th whitespace-separated field, keeping the columns' widths."""
    pos, n = 0, -1
    while True:
        while line[pos] == " ":
            pos += 1
        n += 1
        end = pos
        while end < len(line) and line[end] not in " \r":
            end += 1
        if n == k:
            start = pos
            while start > 0 and line[start - 1] == " ":
                start -= 1
            width = end - start
            return line[:start] + text.rjust(width) + line[end:]
        pos = end


def expand(tokens):
    """SWAT+ element lists: a number, or a number followed by the negative of a range's end."""
    nums = [int(x) for x in tokens]
    out, k = [], 0
    while k < len(nums):
        if k + 1 < len(nums) and nums[k + 1] < 0:
            out += range(nums[k], -nums[k + 1] + 1)
            k += 2
        else:
            out.append(nums[k])
            k += 1
    return out


def compress(nums):
    out, k = [], 0
    while k < len(nums):
        j = k
        while j + 1 < len(nums) and nums[j + 1] == nums[j] + 1:
            j += 1
        out.append(str(nums[k]))
        if j > k:
            out.append(str(-nums[j]))
        k = j + 1
    return out


class Con:
    """A connection file: rows by id, with their outflows (type, id, hydrograph, fraction)."""

    def __init__(self, path):
        self.lines = read(path)
        self.rows = {}
        for i, line in enumerate(self.lines[2:], start=2):
            t = line.split()
            if not t:
                continue
            n = int(t[12])
            outs = [(t[13 + 4 * k], int(t[14 + 4 * k])) for k in range(n)]
            self.rows[int(t[0])] = dict(line=i, t=t, outs=outs)

    def write(self, path, keep, renum, pointer_map):
        """Rows `keep` (old ids, in order), renumbered; field 7 through `pointer_map`; outflow
        ids through `renum[type]`."""
        out = self.lines[:2]
        for new, old in enumerate(keep, start=1):
            r = self.rows[old]
            line = self.lines[r["line"]]
            line = set_field(line, 0, str(new))
            line = set_field(line, 7, str(pointer_map[int(r["t"][7])]))
            for k, (typ, oid) in enumerate(r["outs"]):
                line = set_field(line, 14 + 4 * k, str(renum[typ][oid]))
            out.append(line)
        out.append("")
        return out


def ps_coast(src, out):
    d = os.path.join(out, "Ps_coast")
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(src, d)
    p = lambda f: os.path.join(d, f)

    cha, ru, aq, hru = Con(p("chandeg.con")), Con(p("rout_unit.con")), Con(p("aquifer.con")), Con(p("hru.con"))
    ele = {}
    for line in read(p("rout_unit.ele"))[2:]:
        t = line.split()
        if t:
            ele[int(t[0])] = (t[2], int(t[3]))
    rudef = {}
    def_lines = read(p("rout_unit.def"))
    for line in def_lines[2:]:
        t = line.split()
        if t:
            rudef[int(t[0])] = expand(t[3:])

    # Each channel's outlet, and the catchments wholly on the coastal lowland.
    def outlet(c):
        while cha.rows[c]["outs"]:
            nxt = [o for o in cha.rows[c]["outs"] if o[0] == "sdc"]
            assert len(nxt) == len(cha.rows[c]["outs"]) == 1
            c = nxt[0][1]
        return c

    groups = {}
    for c in cha.rows:
        groups.setdefault(outlet(c), set()).add(c)
    chans = set()
    for o, cs in groups.items():
        rus = [r for r in ru.rows if any(t == "sdc" and i in cs for t, i in ru.rows[r]["outs"])]
        hrus = [ele[e][1] for r in rus for e in rudef[r] if ele[e][0] == "hru"]
        if hrus and max(float(hru.rows[h]["t"][6]) for h in hrus) < MAX_ELEV:
            chans |= cs

    keep_cha = sorted(chans)
    keep_ru = sorted(r for r in ru.rows if any(t == "sdc" and i in chans for t, i in ru.rows[r]["outs"]))
    keep_ele = sorted(e for r in keep_ru for e in rudef[r])
    assert all(ele[e][0] == "hru" for e in keep_ele)
    keep_hru = sorted(ele[e][1] for e in keep_ele)
    shallow = [a for a in aq.rows if any(t == "sdc" and i in chans for t, i in aq.rows[a]["outs"])]
    deep = {i for a in shallow for t, i in aq.rows[a]["outs"] if t == "aqu"}
    keep_aq = sorted(set(shallow) | deep)
    # The cut is closed: everything kept flows only to kept objects, and nothing else into them.
    for a in deep:
        assert not aq.rows[a]["outs"]
        assert all(b in keep_aq for b in aq.rows if any(t == "aqu" and i == a for t, i in aq.rows[b]["outs"]))
    for r in keep_ru:
        assert all((t == "sdc" and i in chans) or (t == "aqu" and i in keep_aq) for t, i in ru.rows[r]["outs"])
    for c in keep_cha:
        assert all(t == "sdc" and i in chans for t, i in cha.rows[c]["outs"])
    for c in cha.rows:
        if c not in chans:
            assert not any(t == "sdc" and i in chans for t, i in cha.rows[c]["outs"])

    num = lambda keep: {old: new for new, old in enumerate(keep, start=1)}
    n_cha, n_ru, n_aq, n_hru, n_ele = num(keep_cha), num(keep_ru), num(keep_aq), num(keep_hru), num(keep_ele)
    renum = {"sdc": n_cha, "aqu": n_aq, "ru": n_ru, "hru": n_hru}

    # Data pointed to by row number: hru-data.hru (from hru.con), aquifer.aqu, channel-lte.cha,
    # rout_unit.rtu (from rout_unit.con), cut to the kept objects' rows, renumbered.
    def cut(fname, rows):
        lines = read(p(fname))
        by_id = {int(l.split()[0]): l for l in lines[2:] if l.split()}
        new = lines[:2]
        for k, r in enumerate(rows, start=1):
            new.append(set_field(by_id[r], 0, str(k)))
        new.append("")
        write(p(fname), new)
        return {r: k for k, r in enumerate(rows, start=1)}

    ptr = lambda con, keep: [int(con.rows[i]["t"][7]) for i in keep]
    m_hru = cut("hru-data.hru", ptr(hru, keep_hru))
    m_aqu = cut("aquifer.aqu", ptr(aq, keep_aq))
    m_lcha = cut("channel-lte.cha", ptr(cha, keep_cha))
    m_rtu = cut("rout_unit.rtu", ptr(ru, keep_ru))

    write(p("hru.con"), hru.write(p("hru.con"), keep_hru, renum, m_hru))
    write(p("aquifer.con"), aq.write(p("aquifer.con"), keep_aq, renum, m_aqu))
    write(p("chandeg.con"), cha.write(p("chandeg.con"), keep_cha, renum, m_lcha))
    write(p("rout_unit.con"), ru.write(p("rout_unit.con"), keep_ru, renum, m_rtu))

    # wetland.wet and hydrology.wet: pointed to by name, but et_act reads `wet_hyd(ihyd)`, the
    # wetland data of the HRU numbered as the wetland's hydrology row; cut to the kept wetlands
    # (in HRU order) so that row is an HRU of the variant.
    hd = {int(l.split()[0]): l.split() for l in read(p("hru-data.hru"))[2:] if l.split()}
    wets = [hd[k][7] for k in sorted(hd) if hd[k][7] != "null"]
    lines = read(p("wetland.wet"))
    by_name = {l.split()[1]: l for l in lines[2:] if l.split()}
    new = lines[:2]
    for k, w in enumerate(wets, start=1):
        new.append(set_field(by_name[w], 0, str(k)))
    write(p("wetland.wet"), new + [""])
    hyds = [by_name[w].split()[3] for w in wets]
    lines = read(p("hydrology.wet"))
    by_name = {l.split()[0]: l for l in lines[2:] if l.split()}
    write(p("hydrology.wet"), lines[:2] + [by_name[h] for h in dict.fromkeys(hyds)] + [""])

    # rout_unit.ele and .def: the elements renumbered, with their HRUs.
    lines = read(p("rout_unit.ele"))
    by_id = {int(l.split()[0]): l for l in lines[2:] if l.split()}
    new = lines[:2]
    for e in keep_ele:
        line = set_field(by_id[e], 0, str(n_ele[e]))
        new.append(set_field(line, 3, str(n_hru[ele[e][1]])))
    write(p("rout_unit.ele"), new + [""])
    new = def_lines[:2]
    for r in keep_ru:
        t = def_lines[[i for i, l in enumerate(def_lines) if l.split()[:1] == [str(r)]][0]].split()
        els = compress([n_ele[e] for e in rudef[r]])
        new.append(f"{n_ru[r]:>8}  {t[1]:>16}  {len(els):>8}  " + "  ".join(f"{x:>8}" for x in els))
    write(p("rout_unit.def"), new + [""])

    # ls_unit.ele and .def: as rout_unit.ele and .def (Pessel's are the same), with each HRU's
    # share of the basin over the variant's area.
    area = sum(float(ru.rows[r]["t"][3]) for r in keep_ru)
    lines = read(p("ls_unit.ele"))
    by_id = {int(l.split()[0]): l for l in lines[2:] if l.split()}
    new = lines[:2]
    for e in keep_ele:
        assert by_id[e].split()[3] == str(ele[e][1])
        h = ele[e][1]
        line = set_field(by_id[e], 0, str(n_ele[e]))
        line = set_field(line, 3, str(n_hru[h]))
        new.append(set_field(line, 4, f"{float(hru.rows[h]['t'][3]) / area:.4E}"))
    write(p("ls_unit.ele"), new + [""])
    lines = read(p("ls_unit.def"))
    by_id = {int(l.split()[0]): l.split() for l in lines[3:] if l.split()}
    new = [lines[0], str(len(keep_ru)), lines[2]]
    for r in keep_ru:
        t = by_id[r]
        assert t[1] == ru.rows[r]["t"][1] and expand(t[4:]) == rudef[r]
        els = compress([n_ele[e] for e in rudef[r]])
        new.append(f"{n_ru[r]:>8}  {t[1]:>16}  {t[2]:>12}  {len(els):>8}  " + "  ".join(f"{x:>8}" for x in els))
    write(p("ls_unit.def"), new + [""])

    # calibration.cal: each change keeps the kept HRUs it covered.
    lines = read(p("calibration.cal"))
    new = lines[:3]
    for line in lines[3:]:
        t = line.split()
        if not t:
            continue
        assert t[3] == "0", "calibration with conditions"
        objs = [n_hru[h] for h in expand(t[11:]) if h in n_hru]
        new.append(" ".join(t[:10]) + f" {len(compress(objs))} " + " ".join(compress(objs)))
    write(p("calibration.cal"), new + [""])

    # object.cnt: the counts and the area.
    lines = read(p("object.cnt"))
    h = lines[1].split()
    t = lines[2]
    counts = {"obj": len(keep_hru) + len(keep_ru) + len(keep_aq) + len(keep_cha), "hru": len(keep_hru), "rtu": len(keep_ru), "aqu": len(keep_aq), "lcha": len(keep_cha)}
    t = set_field(t, h.index("ls_area"), f"{area:.5f}")
    t = set_field(t, h.index("tot_area"), f"{area:.5f}")
    for k, v in counts.items():
        t = set_field(t, h.index(k), str(v))
    lines[2] = t
    write(p("object.cnt"), lines)

    # file.cio: no landscape units or aquifer catchments.
    lines = read(p("file.cio"))
    for i, line in enumerate(lines):
        if line.startswith("regions"):
            for name in ("aqu_catunit.ele",):
                line = line.replace(name, "null".ljust(len(name)))
            lines[i] = line
    write(p("file.cio"), lines)

    # 2015-2017, printed from the first day.
    lines = read(p("time.sim"))
    lines[2] = set_field(lines[2], 3, "2017")
    write(p("time.sim"), lines)
    lines = read(p("print.prt"))
    lines[2] = set_field(lines[2], 0, "0")
    write(p("print.prt"), lines)
    if os.path.exists(p("README.md")):
        os.remove(p("README.md"))
    print(f"Ps_coast: {len(keep_cha)} channels, {len(keep_ru)} routing units, {len(keep_hru)} HRUs, {len(keep_aq)} aquifers, {area:.0f} ha")


# Ps_paddy's decision tables (lum.dtl), in SWAT+'s format. Depths in mm; days of the year.
PADDY_DTL = """
name                     conds      alts      acts       !raise the weir for the first season (mid Feb to mid Mar)
weir_up_s1                   1         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
prob_unif                  1.0        45              null                 -      69.00000         =
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
weir_height                hru         0             paddy               wet      50.00000       1.00000          weir_ps   y

name                     conds      alts      acts       !raise the weir for the second season (August)
weir_up_s2                   1         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
prob_unif                  1.0       213              null                 -     235.00000         =
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
weir_height                hru         0             paddy               wet      50.00000       1.00000          weir_ps   y

name                     conds      alts      acts       !drain the paddy before harvest
weir_down                    2         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
weirh                      hru         0              null                 -      30.00000         >
phu_plant                  hru         0              null                 -       0.85000         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
weir_height                hru         0             paddy               wet       0.00000       1.00000          weir_ps   y

name                     conds      alts      acts       !keep 50 mm of water over the paddy when it falls under 30 mm
pad_irr                      2         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
weirh                      hru         0              null                 -      30.00000         >
wet_depth                  hru         0            hwater                 -      30.00000         <
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
irrigate                  null         0           ponding        ponding_ps      50.00000      30.00000              null   y

name                     conds      alts      acts       !puddle once the first season's paddy is flooded
puddle_s1                    3         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
month                     null         0              null                 -       7.00000         <
weirh                      hru         0              null                 -      30.00000         >
wet_depth                  hru         0            hwater                 -       0.00000         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
puddle                     hru         0            puddle           med_eff     100.00000       1.00000              null   y

name                     conds      alts      acts       !puddle once the second season's paddy is flooded
puddle_s2                    3         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
month                     null         0              null                 -       6.00000         >
weirh                      hru         0              null                 -      30.00000         >
wet_depth                  hru         0            hwater                 -       0.00000         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
puddle                     hru         0            puddle           med_eff     100.00000       1.00000              null   y

name                     conds      alts      acts       !transplant rice into a flooded, puddled paddy
plant_rice                   2         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
weirh                      hru         0              null                 -      30.00000         >
wet_depth                  hru         0            hwater                 -      25.00000         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
plant                      hru         0           rice120           rice120       0.00000       1.00000              null   y

name                     conds      alts      acts       !harvest at maturity, two seasons a year
harv_rice                    1         1         1
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1
phu_plant                  hru         0              null                 -       1.00000         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
harvest_kill               hru         0        grain_harv           rice120       0.00000       2.00000             grain   y

name                     conds      alts      acts       !urea in three splits, SP-36 at planting: first season
fert_s1                      4         3         4
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1      alt2      alt3
month                     null         0              null                 -       8.00000         <         <         <
phu_plant                  hru         0              null                 -       0.02000         >         -         -
phu_plant                  hru         0              null                 -       0.25000         -         >         -
phu_plant                  hru         0              null                 -       0.50000         -         -         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
fertilize                  hru         0           basal_n            elem_n      38.00000       1.00000            inject   y   n   n
fertilize                  hru         0           basal_p            elem_p      15.70000       1.00000            inject   y   n   n
fertilize                  hru         0            top1_n            elem_n      38.00000       1.00000        side_dress   n   y   n
fertilize                  hru         0            top2_n            elem_n      38.00000       1.00000        side_dress   n   n   y

name                     conds      alts      acts       !urea in three splits, SP-36 at planting: second season
fert_s2                      4         3         4
var                        obj   obj_num           lim_var            lim_op     lim_const      alt1      alt2      alt3
month                     null         0              null                 -       7.00000         >         >         >
phu_plant                  hru         0              null                 -       0.02000         >         -         -
phu_plant                  hru         0              null                 -       0.25000         -         >         -
phu_plant                  hru         0              null                 -       0.50000         -         -         >
act_typ                    obj   obj_num              name            option         const        const2                fp  outcome
fertilize                  hru         0           basal_n            elem_n      38.00000       1.00000            inject   y   n   n
fertilize                  hru         0           basal_p            elem_p      15.70000       1.00000            inject   y   n   n
fertilize                  hru         0            top1_n            elem_n      38.00000       1.00000        side_dress   n   y   n
fertilize                  hru         0            top2_n            elem_n      38.00000       1.00000        side_dress   n   n   y
"""

PADDY_AUTOS = ["weir_up_s1", "weir_up_s2", "weir_down", "pad_irr", "puddle_s1", "puddle_s2", "plant_rice", "harv_rice", "fert_s1", "fert_s2"]


def ps_paddy(base, dst):
    """A copy of project `base` with its rice HRUs as paddies (see the module's docstring and
    docs/paddy.md)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)

    def p(name):
        return os.path.join(dst, name)

    def append(name, rows):
        lines = read(p(name))
        while lines and not lines[-1].strip():
            lines.pop()
        write(p(name), lines + rows + [""])

    # The rice HRUs: the paddy land use and the paddy wetland.
    lines = read(p("hru-data.hru"))
    n = 0
    for i in range(2, len(lines)):
        t = lines[i].split()
        if len(t) > 7 and t[5] == "rice_lum":
            lines[i] = set_field(set_field(lines[i], 5, "paddy_lum"), 7, "paddy_ps")
            n += 1
    write(p("hru-data.hru"), lines)

    # Land use, plant community and schedule.
    rice = next(l for l in read(p("landuse.lum")) if l.startswith("rice_lum"))
    paddy = set_field(set_field(set_field(rice, 0, "paddy_lum".ljust(len("rice_lum"))), 2, "paddy_comm"), 3, "paddy_ip200")
    append("landuse.lum", [paddy])
    append("plant.ini", ["paddy_comm               1         1  ",
                         "                                        rice120          n       0.00000       0.00000       0.00000       0.00000       0.00000   10000.00000  "])
    append("management.sch", [f"paddy_ip200                                       0         {len(PADDY_AUTOS)}  "]
           + [f"                                                   {a}" for a in PADDY_AUTOS])
    lines = read(p("lum.dtl"))
    lines[1] = str(int(lines[1].split()[0]) + PADDY_DTL.count("\nname "))
    while lines and not lines[-1].strip():
        lines.pop()
    write(p("lum.dtl"), lines + PADDY_DTL.split("\n"))

    # The paddy: a wetland of hydrology `paddy` (0.15 mm/h through the plough pan), its weir,
    # puddling, ponding irrigation and tillage. weir.res's first row is every wetland's weir:
    # 20 mm, Pessel's principal depth, leaves the other wetlands as they were; the paddies keep
    # 2 cm behind their bunds between seasons, 5 cm in them.
    lines = read(p("wetland.wet"))
    while lines and not lines[-1].strip():
        lines.pop()
    nwet = len(lines) - 2
    write(p("wetland.wet"), lines + [f"{nwet + 1:8d}  paddy_ps                      initwet1             paddy           wetland           sedwet1           nutwet1  ", ""])
    append("hydrology.wet", ["paddy                  1.00000      50.00000       1.00000     150.00000       0.15000       0.75000       1.00000       1.00000       1.00000       1.00000  "])
    write(p("weir.res"), ["weir.res: paddy weir (SWAT+ paddy example's coefficients)",
                          "name          linear_c         exp_k      width_m      height_m",
                          "weir_ps        1.83000       1.50000      5.00000       0.02000", ""])
    write(p("puddle.ops"), ["puddle.ops: puddling operations",
                            "name                  hydcon_mm/h   sed_ppm      orgn_ppm      sedp_ppm     no3_ppm       solp_ppm       nh3_ppm  no2_ppm",
                            "high_eff              0.00000         10000       0.00000     800.00000     10.00000      20.00000       1.00000  1.00000",
                            "med_eff               0.00000         10000       0.00000     500.00000     10.00000      20.00000       1.00000  1.00000",
                            "low_eff               0.00000         10000       0.00000     100.00000     10.00000      20.00000       1.00000  1.00000", ""])
    append("irr.ops", ["ponding_ps            50.00000       1.00000       0.00000       0.00000       0.00000       0.00000       0.00000  paddy_ponding"])
    append("tillage.til", ["puddle                 0.98000     150.00000       0.00000       0.00000       0.00000  rice_paddy_puddling"])
    lines = read(p("file.cio"))
    for i, line in enumerate(lines):
        if line.startswith("reservoir"):
            t = line.split()
            assert t[6] == "null", t
            lines[i] = line.replace("null              wetland.wet", "weir.res          wetland.wet")
    write(p("file.cio"), lines)
    print(f"{os.path.basename(dst)}: {n} rice HRUs as paddies")


def ps_musk(base, dst):
    """A copy of project `base` routing its channels by Muskingum."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "codes.bsn")
    lines = read(path)
    k = lines[1].split().index("rte_cha")
    lines[2] = set_field(lines[2], k, "1")
    write(path, lines)
    print(f"{os.path.basename(dst)}: Muskingum routing")


def ps_atmo(base, dst):
    """A copy of project `base` (2015-2017) with monthly nitrogen deposition."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)

    def p(name):
        return os.path.join(dst, name)

    # (name, rain NH4-N, rain NO3-N in mg/L, dry NH4-N, dry NO3-N in kg/ha a year)
    stations = [("dep_inland", 0.15, 0.25, 1.5, 1.0), ("dep_coast", 0.30, 0.30, 2.5, 1.5)]
    months = [(y, m) for y in (2015, 2016, 2017) for m in range(1, 13)]
    lines = ["atmo.cli: Ps_atmo (scripts/make-pessel-variants.py)", "num_sta timestep mo_init yr_init num",
             f"{len(stations)}  mo  1  2015  {len(months)}"]
    for name, nh4, no3, dnh4, dno3 in stations:
        haze = [3.0 if (y, m) in ((2015, 9), (2015, 10)) else 1.0 for y, m in months]
        lines.append(name)
        lines.append(" ".join(f"{nh4:.3f}" for _ in months))
        lines.append(" ".join(f"{no3:.3f}" for _ in months))
        lines.append(" ".join(f"{dnh4 * h:.3f}" for h in haze))
        lines.append(" ".join(f"{dno3 * h:.3f}" for h in haze))
    write(p("atmo.cli"), lines + [""])
    sta = read(p("weather-sta.cli"))
    k = sta[1].split().index("atmo_dep")
    for i in range(2, len(sta)):
        if sta[i].split():
            sta[i] = set_field(sta[i], k, stations[i % 2][0])
    write(p("weather-sta.cli"), sta)
    cio = read(p("file.cio"))
    for i, line in enumerate(cio):
        t = line.split()
        if t and t[0] == "climate":
            assert t[9] == "null", t
            t[9] = "atmo.cli"
            cio[i] = t[0].ljust(18) + "".join(x.ljust(18) for x in t[1:])
    write(p("file.cio"), cio)
    print(f"{os.path.basename(dst)}: monthly nitrogen deposition")


def ps_tile(base, dst):
    """A copy of project `base` with its oil palm on drains."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "tiledrain.str")
    lines = read(path)
    while lines and not lines[-1].strip():
        lines.pop()
    # depth (mm), time to drain to field capacity (h), lag (h), radius (mm), spacing (mm),
    # drainage coefficient (mm a day), pump capacity, lateral conductivity factor.
    lines.append("peat_canal           600.00000      48.00000      96.00000     100.00000   30000.00000      20.00000       0.00000       1.00000  ")
    write(path, lines + [""])
    path = os.path.join(dst, "landuse.lum")
    lines = read(path)
    k = lines[1].split().index("tile")
    for i in range(2, len(lines)):
        if lines[i].startswith("oilp_lum"):
            lines[i] = set_field(lines[i], k, "peat_canal")
    write(path, lines)
    print(f"{os.path.basename(dst)}: oil palm on drains")


def ps_bmp(base, dst):
    """A copy of project `base` with filter strips along its oil palm, grassed waterways in its
    row crops and the editor's user-defined practice on its generic agricultural land."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "filterstrip.str")
    lines = read(path)
    while lines and not lines[-1].strip():
        lines.pop()
    # SWAT's documented defaults (field 40 times the strip, half the runoff into its most
    # concentrated tenth, none of it channelized), White and Arnold (2009).
    lines.append("riparian                 1      40.00000       0.50000       0.00000  Riparian_buffer")
    write(path, lines + [""])
    path = os.path.join(dst, "landuse.lum")
    lines = read(path)
    h = lines[1].split()
    for i in range(2, len(lines)):
        if lines[i].startswith("oilp_lum"):
            lines[i] = set_field(lines[i], h.index("vfs"), "riparian")
        elif lines[i].startswith("agrr_lum"):
            lines[i] = set_field(lines[i], h.index("grww"), "grwway_low")
        elif lines[i].startswith("agrl_lum"):
            lines[i] = set_field(lines[i], h.index("bmp"), "bmpusr1")
    write(path, lines)
    print(f"{os.path.basename(dst)}: filter strips, grassed waterways, user practice")


if __name__ == "__main__":
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    ps_coast(src, out)
    ps_paddy(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_paddy"))
    ps_musk(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_musk"))
    ps_atmo(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_atmo"))
    ps_tile(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_tile"))
    ps_bmp(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_bmp"))
