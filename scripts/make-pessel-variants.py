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


if __name__ == "__main__":
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    ps_coast(src, out)
