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


def ps_septic(base, dst):
    """A copy of project `base` with septic systems under the villages of its dry fields."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "septic.str")
    lines = read(path)
    h = lines[1].split()
    for i in range(2, len(lines)):
        if lines[i].startswith("standard"):
            # Operated, for a household of four (BPS); the editor's biozone 0.5 m deep and
            # 50 mm thick (Jeong et al. 2011).
            lines[i] = set_field(set_field(lines[i], h.index("operation"), "1"), h.index("residents"), "4.00000")
    write(path, lines)
    path = os.path.join(dst, "landuse.lum")
    lines = read(path)
    h = lines[1].split()
    for i in range(2, len(lines)):
        if lines[i].startswith("agrr_lum"):
            lines[i] = set_field(lines[i], h.index("sep"), "standard")
    write(path, lines)
    print(f"{os.path.basename(dst)}: septic systems in the dry fields")


def ps_solp(base, dst):
    """A copy of project `base` with the dynamic soil P model (`codes.bsn` soil_p 1)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "codes.bsn")
    lines = read(path)
    lines[2] = set_field(lines[2], lines[1].split().index("soil_p"), "1")
    write(path, lines)
    print(f"{os.path.basename(dst)}: dynamic soil P")


def ps_urban(base, dst):
    """A copy of project `base` whose generic farmland is a medium-density town, its loads from
    the USGS regression equations (`urb_ro` usgs_reg)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "landuse.lum")
    lines = read(path)
    h = lines[1].split()
    for i in range(2, len(lines)):
        if lines[i].startswith("agrl_lum"):
            lines[i] = set_field(set_field(lines[i], h.index("urban"), "urmd"), h.index("urb_ro"), "usgs_reg")
    write(path, lines)
    print(f"{os.path.basename(dst)}: a town by the USGS regressions")


# Mononobe's distribution of a day's rain over a storm of T hours: the depth fallen by hour t is
# R (t / T)^(1/3). Four hours is the dominant duration of heavy rain it matches best in Java
# (Citanduy, Jurnal Sumber Daya Air 2024); over Sumatra's land convective rain peaks between 15:00
# and 20:00 local time (Mori et al. 2004).
STORM_HOURS = 4
STORM_START = 15  # the storm falls in the hours ending 16:00 to 19:00


def mononobe(depth, hours=STORM_HOURS):
    cum = [depth * (t / hours) ** (1.0 / 3.0) for t in range(hours + 1)]
    return [cum[t + 1] - cum[t] for t in range(hours)]


def ps_hourly(base, dst, years=(2015, 2017)):
    """A copy of project `base` run hourly with Green-Ampt infiltration: each day's rain of its
    precipitation files falls in a four-hour afternoon storm (Mononobe)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    lines = read(p("codes.bsn"))
    lines[2] = set_field(lines[2], lines[1].split().index("gampt"), "1")
    write(p("codes.bsn"), lines)
    lines = read(p("time.sim"))
    lines[2] = set_field(lines[2], 4, "24")
    write(p("time.sim"), lines)
    names = [l.split()[0] for l in read(p("pcp.cli"))[2:] if l.split()]
    for name in names:
        src = read(p(name))
        head = src[2].split()
        out = [src[0].split(":")[0] + ": hourly from the daily values, a four-hour Mononobe storm from 15:00 (Ps_hourly)",
               "nbyr     tstep       lat       lon      elev",
               f"{years[1] - years[0] + 1:4d}{24:10d}{head[2]:>10}{head[3]:>10}{head[4]:>10}"]
        for line in src[3:]:
            t = line.split()
            if len(t) < 3:
                continue
            yr, jday, v = int(t[0]), int(t[1]), float(t[2])
            if not years[0] <= yr <= years[1]:
                continue
            hourly = [0.0] * 24
            if v > 0.0:
                for k, x in enumerate(mononobe(v)):
                    hourly[STORM_START + k] = x
            d = __import__("datetime").date(yr, 1, 1) + __import__("datetime").timedelta(jday - 1)
            for h in range(24):
                out.append(f"{yr:4d}{jday:5d}{d.month:4d}{d.day:4d}{h + 1:4d}{hourly[h]:12.5f}")
        write(p(name), out + [""])
    print(f"{os.path.basename(dst)}: hourly, Green-Ampt ({len(names)} precipitation files)")


def ps_uh(base, dst):
    """A copy of project `base` routed hourly with the daily curve number runoff (`gampt` 0):
    each HRU's and routing unit's runoff is spread over the hours by its unit hydrograph, and
    the channels are routed by the hour. The precipitation stays daily."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    lines = read(os.path.join(dst, "time.sim"))
    lines[2] = set_field(lines[2], 4, "24")
    write(os.path.join(dst, "time.sim"), lines)
    print(f"{os.path.basename(dst)}: hourly, curve number and unit hydrographs")


def ps_gap(base, dst):
    """A copy of the hourly project `base` with generated rain: one station's precipitation file
    loses the hours of a few rainy days (-99, filled by `cli_pgenhr`), and another station has
    no file at all (`sim`)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    lines = read(p("weather-sta.cli"))
    k = lines[1].split().index("pcp")
    gap_file = None
    for i in range(2, len(lines)):
        t = lines[i].split()
        if t and t[0] == "s2175s100875e":
            gap_file = t[k]
        if t and t[0] == "s2275s100875e":
            lines[i] = set_field(lines[i], k, "sim")
    write(p("weather-sta.cli"), lines)
    # The hours from 17:00 of days 40, 41 and 300 of 2016 are missing.
    src = read(p(gap_file))
    out = src[:3]
    for line in src[3:]:
        t = line.split()
        if len(t) == 6 and t[0] == "2016" and int(t[1]) in (40, 41, 300) and int(t[4]) >= 17:
            line = line[:-12] + f"{-99.0:12.5f}"
        out.append(line)
    write(p(gap_file), out)
    print(f"{os.path.basename(dst)}: hourly, gaps in {gap_file} and a generated station")


def ps_pet(base, dst, years=(2015, 2017)):
    """A copy of project `base` whose PET is "read in" (`codes.bsn` pet 3), every station with
    the same measured PET file (pet.cli), a constant 4 mm a day."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    lines = read(p("codes.bsn"))
    lines[2] = set_field(lines[2], lines[1].split().index("pet"), "3")
    write(p("codes.bsn"), lines)
    write(p("pet.cli"), ["pet.cli: measured PET", "filename", "const4.pet"])
    rows = ["const4.pet: a constant 4 mm a day", "nbyr     tstep       lat       lon      elev",
            f"{years[1] - years[0] + 1:4d}{0:10d}{-1.675:10.3f}{100.975:10.3f}{10.0:10.3f}"]
    for y in range(years[0], years[1] + 1):
        for d in range(1, 366 + (y % 4 == 0)):
            rows.append(f"{y}{d:5d}{4.0:11.5f}")
    write(p("const4.pet"), rows)
    lines = read(p("weather-sta.cli"))
    k = lines[1].split().index("pet")
    for i in range(2, len(lines)):
        if lines[i].split():
            lines[i] = set_field(lines[i], k, "const4.pet")
    write(p("weather-sta.cli"), lines)
    lines = read(p("file.cio"))
    for i, l in enumerate(lines):
        if l.startswith("climate"):
            t = l.split()
            t[3] = "pet.cli"
            lines[i] = "".join(f"{x:<18}" for x in t)
    write(p("file.cio"), lines)
    print(f"{os.path.basename(dst)}: PET read in")


# Ps_cal's changes: (parameter, change, value, conditions, first and last soil layer, objects).
# Each kind of change and condition of calibration.cal, at values within cal_parms.cal's ranges.
PS_CAL = [
    ("cn2", "pctchg", -8.0, ["hsg = 0 D"], 0, 0, []),
    ("cn2", "pctchg", -5.0, ["hsg = 0 C"], 0, 0, []),
    ("esco", "absval", 0.85, [], 0, 0, []),
    ("epco", "absval", 0.9, ["landuse = 0 oilp_lum"], 0, 0, []),
    ("canmx", "absval", 4.0, ["plant = 0 oilp"], 0, 0, []),
    ("awc", "relchg", 0.1, ["texture = 0 CLAY_LOAM"], 1, 2, []),
    ("k", "pctchg", 25.0, [], 0, 0, []),
    ("bd", "pctchg", 5.0, [], 1, 1, []),
    ("anion_excl", "pctchg", 10.0, [], 0, 0, []),
    ("usle_k", "relchg", -0.1, [], 1, 1, []),
    ("alb", "absval", 0.15, [], 1, 1, []),
    ("lat_ttime", "absval", 4.0, [], 0, 0, []),
    ("latq_co", "absval", 0.4, [], 0, 0, [1, -40]),
    ("slope", "pctchg", 10.0, [], 0, 0, []),
    ("elev", "abschg", 50.0, ["landuse = 0 rice_lum"], 0, 0, []),
    ("erorgn", "absval", 1.5, [], 0, 0, []),
    ("petco", "absval", 1.05, ["hsg = 0 B", "landuse = 0 oilp_lum"], 0, 0, []),
    ("surlag", "absval", 2.0, [], 0, 0, []),
    ("alpha", "absval", 0.05, [], 0, 0, []),
    ("deep_seep", "absval", 0.05, [], 0, 0, []),
    ("flo_min", "absval", 3.0, [], 0, 0, []),
    ("revap_co", "absval", 0.1, [], 0, 0, []),
    ("revap_min", "absval", 5.0, [], 0, 0, []),
    ("sp_yld", "absval", 0.1, [], 0, 0, [2, -10]),
]


def ps_cal(base, dst):
    """A copy of project `base` with more hard calibration: changes of HRU, soil, basin and
    aquifer parameters, some only where conditions are met (`PS_CAL`)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    lines = read(p("calibration.cal"))
    rows = [l for l in lines[3:] if l.split()]
    for name, typ, val, conds, l1, l2, objs in PS_CAL:
        tail = f"{len(objs):>10}" + "".join(f"{o:>10}" for o in objs)
        rows.append(f"{name:<16}{typ:>10}{val:>14.5f}{len(conds):>10}{l1:>10}{l2:>10}{0:>10}{0:>10}{0:>10}{0:>10}" + tail)
        rows.extend(conds)
    write(p("calibration.cal"), [lines[0], str(len(rows) - sum(len(c[3]) for c in PS_CAL)), lines[2]] + rows + [""])
    print(f"{os.path.basename(dst)}: {len(PS_CAL)} more calibration changes")


def ps_harv(base, dst):
    """A copy of project `base` with every kind of harvest: oil palm's fruit bunches picked each
    month (a biomass harvest only, `harv`), the generic farmland's grain harvested, its stover
    baled and the crop killed by separate operations, and the row crops lifted as tubers and
    killed by a decision table's `harvest` and `kill` actions."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    # Fresh fruit bunches: 3 % of the palm's biomass above ground a month (about the 20-25 t a
    # hectare a year of a mature plantation, for 60-80 t of standing biomass).
    lines = read(p("harv.ops"))
    while lines and not lines[-1].strip():
        lines.pop()
    lines.append(f"{'ffb':<18}{'biomass':>14}{0.03:14.5f}{1.0:14.5f}{0.0:14.5f}  ")
    write(p("harv.ops"), lines + [""])
    ops = lambda rows: [f"{'':<40}{o:<10}{mo:>6}{d:>6}{0.0:>10.5f}  {a1:<16}  {a2:<16}{a3:>10.5f}" for o, mo, d, a1, a2, a3 in rows]
    lines = read(p("management.sch"))
    while lines and not lines[-1].strip():
        lines.pop()
    lines += [f"{'oilp_sched':<34}{12:>3}{0:>11}  "] + ops([("harv", m, 15, "oilp", "ffb", 0.0) for m in range(1, 13)])
    lines += [f"{'agrl_seq':<34}{4:>3}{0:>11}  "] + ops([("plnt", 3, 1, "agrl", "null", 0.0), ("harv", 7, 1, "agrl", "grain", 0.0),
                                                      ("harv", 7, 5, "agrl", "stover_med", 0.5), ("kill", 7, 10, "agrl", "null", 0.0)])
    lines += [f"{'agrr_dt':<34}{0:>3}{1:>11}  ", f"{'':<51}agrr_tuber  "]
    write(p("management.sch"), lines + [""])
    lines = read(p("lum.dtl"))
    lines[1] = str(int(lines[1].split()[0]) + 1)
    while lines and not lines[-1].strip():
        lines.pop()
    lines += [
        "",
        "name                     conds      alts      acts  ",
        "agrr_tuber                   3         3         3  ",
        "var                        obj   obj_num           lim_var            lim_op     lim_const      alt1      alt2      alt3  ",
        "jday                       hru         0              null                 -      60.00000         =         -         -  ",
        "jday                       hru         0              null                 -     300.00000         -         =         -  ",
        "jday                       hru         0              null                 -     305.00000         -         -         =  ",
        "act_typ                    obj   obj_num              name            option         const        const2                fp  outcome           ",
        "plant                      hru         0             plant              agrr       0.00000       1.00000              null  y   n   n   ",
        "harvest                    hru         0        tuber_harv              agrr       0.00000       1.00000             tuber  n   y   n   ",
        "kill                       hru         0              kill              agrr       0.00000       1.00000              null  n   n   y   ",
        "",
    ]
    write(p("lum.dtl"), lines)
    lines = read(p("landuse.lum"))
    k = lines[1].split().index("mgt")
    for i in range(2, len(lines)):
        t = lines[i].split()
        for lum, sch in (("oilp_lum", "oilp_sched"), ("agrl_lum", "agrl_seq"), ("agrr_lum", "agrr_dt")):
            if t and t[0] == lum:
                lines[i] = set_field(lines[i], k, sch)
    write(p("landuse.lum"), lines)
    print(f"{os.path.basename(dst)}: biomass, grain, residue and tuber harvests, kills")


def ps_peren(base, dst):
    """A copy of project `base` whose oil palm has days to maturity (a year from its planting
    day, which for a perennial is the last one computed for an annual), and plant populations in
    plant.ini (136 palms a hectare, the usual triangular spacing of 9 m; 50,000 row crop plants)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    lines = read(p("plants.plt"))
    k = lines[1].split().index("days_mat")
    for i in range(2, len(lines)):
        t = lines[i].split()
        if t and t[0] == "oilp":
            lines[i] = set_field(lines[i], k, "365.00000")
    write(p("plants.plt"), lines)
    lines = read(p("plant.ini"))
    k = lines[1].split().index("plnt_pop") - 3
    for i in range(2, len(lines)):
        t = lines[i].split()
        if t and t[0] in ("oilp", "agrr"):
            lines[i] = set_field(lines[i], k, "136.00000" if t[0] == "oilp" else "50000.00000")
    write(p("plant.ini"), lines)
    print(f"{os.path.basename(dst)}: perennial days to maturity, plant populations")


def ps_urbhr(base, dst):
    """A copy of the hourly project `base` whose generic farmland is a medium-density town, its
    loads washed off the impervious area by the hour (`hru_urbanhr`)."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    path = os.path.join(dst, "landuse.lum")
    lines = read(path)
    h = lines[1].split()
    for i in range(2, len(lines)):
        if lines[i].startswith("agrl_lum"):
            lines[i] = set_field(set_field(lines[i], h.index("urban"), "urmd"), h.index("urb_ro"), "buildup_washoff")
    write(path, lines)
    print(f"{os.path.basename(dst)}: hourly, a town washed off by the hour")


def ps_land(base, dst):
    """A copy of project `base` where three upland oil palm HRUs drain across the land into the
    HRU below them in their routing unit (landscape routing), instead of to the channel."""
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(base, dst)
    p = lambda f: os.path.join(dst, f)
    # (routing unit, upslope HRU, downslope HRU, how): into a peat swamp forest (wetland), into
    # dry fields as a whole, and into dry fields by surface and lateral flow separately.
    pairs = [(51, 4, 5, "tot"), (37, 8, 9, "tot"), (36, 10, 11, "sur_lat")]
    lines = read(p("rout_unit.con"))
    aqu = {}
    for l in lines[2:]:
        t = l.split()
        if t:
            k = t.index("rhg")
            aqu[int(t[0])] = int(t[k - 1])
    lines = read(p("rout_unit.def"))
    for i in range(2, len(lines)):
        t = lines[i].split()
        for ru, up, down, _ in pairs:
            if t and int(t[0]) == ru:
                lines[i] = f"{ru:8d}{t[1]:>18}{1:10d}{down:10d}"
    write(p("rout_unit.def"), lines)
    lines = read(p("hru.con"))
    for i in range(2, len(lines)):
        t = lines[i].split()
        for ru, up, down, how in pairs:
            if t and int(t[0]) == up:
                outs = [("hru", down, "tot")] if how == "tot" else [("hru", down, "sur"), ("hru", down, "lat")]
                outs.append(("aqu", aqu[ru], "rhg"))
                line = set_field(lines[i].rstrip(), 12, str(len(outs)))
                for typ, num, hyd in outs:
                    line += f"{typ:>14}{num:10d}{hyd:>14}{1.0:14.5f}"
                lines[i] = line + "  "
    write(p("hru.con"), lines)
    print(f"{os.path.basename(dst)}: {len(pairs)} upland HRUs routed across the land")


if __name__ == "__main__":
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    ps_coast(src, out)
    ps_paddy(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_paddy"))
    ps_musk(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_musk"))
    ps_atmo(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_atmo"))
    ps_tile(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_tile"))
    ps_bmp(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_bmp"))
    ps_septic(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_septic"))
    ps_solp(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_solp"))
    ps_urban(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_urban"))
    ps_hourly(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_hourly"))
    ps_land(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_land"))
    ps_urbhr(os.path.join(out, "Ps_hourly"), os.path.join(out, "Ps_urbhr"))
    ps_pet(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_pet"))
    ps_cal(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_cal"))
    ps_harv(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_harv"))
    ps_peren(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_peren"))
    ps_uh(os.path.join(out, "Ps_coast"), os.path.join(out, "Ps_uh"))
    ps_gap(os.path.join(out, "Ps_hourly"), os.path.join(out, "Ps_gap"))
