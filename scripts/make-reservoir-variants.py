#!/usr/bin/env python3
"""Builds the phase 4 reservoir and point-source scenario from testdata/Batanghari.

  Bh_res  Batanghari, 2015-2017, with a reservoir at the head of channel cha037 (47,000 ha
          upstream): everything that flowed into cha037 (channels and routing units) flows
          into the reservoir, which releases into cha037 by Batanghari's own `flood_season`
          decision table. Two point sources (recall objects): the treated effluent of a palm
          oil mill, daily, into cha060, and a town's wastewater, yearly, into cha100. The
          reservoir and the sources are hypothetical; their sizes and loads are typical ones
          (docs/reservoirs.md).

  Bh_rel  Bh_res with the reservoir released by the newer options of `res_hydro`: below its
          principal volume as a natural lake (Döll et al. 2003) plus the Hanasaki and HYPE
          hydropower schemes, between the spillways by `dyrt1`, above the emergency spillway by
          `inflo_targ` (docs/engine1.md).

  Bh_nosed  Bh_res with the reservoir naming a sediment.res row that doesn't exist: SWAT+ then
          leaves its settling factors at 0 (all sediment above the normal concentration, 0,
          settles at once; no soluble N or P settles), and the port must do the same.

usage: scripts/make-reservoir-variants.py <Batanghari folder> <output folder>
"""
import calendar
import math
import os
import shutil
import sys

CHA = 37
MILL_CHA = 60
TOWN_CHA = 100


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


def append(path, rows):
    lines = read(path)
    while lines and not lines[-1].strip():
        lines.pop()
    write(path, lines + rows + [""])


# A hyd_output record: flo (m3), sed (t), orgn, sedp, no3, solp, chla, nh3, no2, cbod, dox (kg),
# san, sil, cla, sag, lag, grv (t), temp (deg C).
def hyd(flo, sed=0.0, orgn=0.0, sedp=0.0, no3=0.0, solp=0.0, nh3=0.0, cbod=0.0, dox=0.0, temp=0.0):
    v = [flo, sed, orgn, sedp, no3, solp, 0.0, nh3, 0.0, cbod, dox, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, temp]
    return " ".join(f"{x:14.5f}" for x in v)


def bh_res(src, out):
    dst = os.path.join(out, "Bh_res")
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst)

    def p(name):
        return os.path.join(dst, name)

    # Everything flowing into cha037 (other channels, routing units) now flows into res 1.
    moved = 0
    for name in ("chandeg.con", "rout_unit.con"):
        lines = read(p(name))
        for i in range(2, len(lines)):
            t = lines[i].split()
            if len(t) < 14:
                continue
            nout = int(t[12])
            for k in range(nout):
                typ, num = 13 + 4 * k, 14 + 4 * k
                if t[typ] == "sdc" and int(t[num]) == CHA:
                    lines[i] = set_field(set_field(lines[i], typ, "res"), num, "1")
                    moved += 1
        write(p(name), lines)
    cha = next(l.split() for l in read(p("chandeg.con"))[2:] if l.split() and int(l.split()[0]) == CHA)
    lat, lon, elev, wst = cha[4], cha[5], cha[6], cha[8]
    hdr = "      id  name                gis_id          area           lat           lon          elev       res               wst       cst      ovfl      rule   out_tot       obj_typ    obj_id       hyd_typ          frac  "
    write(p("reservoir.con"), ["reservoir.con: Bh_res (scripts/make-reservoir-variants.py)", hdr,
                               f"       1  res001                   1     300.00000  {lat:>12}  {lon:>12}  {elev:>12}         1  {wst:>16}         0         0         0         1           sdc        {CHA:3d}           tot       1.00000  ", ""])
    # The reservoir: 30 million m3 at the principal spillway over 300 ha (10 m deep), 50 million
    # m3 over 400 ha at the emergency spillway.
    write(p("reservoir.res"), ["reservoir.res: Bh_res",
                               "      id  name                    init               hyd               rel               sed               nut  ",
                               "       1  res001              initres1         hydres001      flood_season           sedres1           nutres1  ", ""])
    write(p("hydrology.res"), ["hydrology.res: Bh_res",
                               "name                   yr_op        mon_op       area_ps        vol_ps       area_es        vol_es             k      evap_co       shp_co1       shp_co2  ",
                               "hydres001                  0             0     300.00000    3000.00000     400.00000    5000.00000       0.05000       0.60000       0.00000       0.00000  ", ""])
    append(p("initial.res"), ["initres1                   init_res             null              null              null              null  null              "])
    append(p("om_water.ini"), ["init_res               0.80000      20.00000       0.50000       0.05000       0.50000       0.02000       0.00000       0.05000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000       0.00000  "])
    append(p("sediment.res"), ["sedres1               50.00000      10.00000       0.00000       0.00000       1.00000       1.00000  "])
    nut = next(l for l in read(p("nutrients.res")) if l.startswith("nutwet1"))
    append(p("nutrients.res"), [nut.replace("nutwet1", "nutres1", 1)])

    # Point sources. A palm oil mill of 45 t of fresh fruit bunches an hour, 20 h a day, gives
    # about 0.6 m3 of effluent per tonne: 540 m3 a day, treated in ponds to about 100 mg/L BOD,
    # 50 mg/L total N (half ammonium), 10 mg/L P, 200 mg/L suspended solids, at 30 deg C. It
    # crushes less in the low season (Feb-Apr).
    days = []
    for y in (2015, 2016, 2017):
        for d in range(1, 366 + calendar.isleap(y)):
            dt = __import__("datetime").date(y, 1, 1) + __import__("datetime").timedelta(d - 1)
            crop = 0.75 + 0.25 * math.cos(2.0 * math.pi * (d - 280) / 365.0)
            q = 540.0 * crop
            days.append(f"{d:6d}{dt.month:6d}{dt.day:6d}{y:6d}  sdc  cha{MILL_CHA:03d}  " +
                        hyd(q, sed=q * 200e-6, orgn=q * 25e-3, sedp=q * 3e-3, no3=q * 2e-3, solp=q * 7e-3, nh3=q * 25e-3, cbod=q * 100e-3, dox=q * 2e-3, temp=30.0))
    write(p("mill.rec"), ["mill.rec: palm oil mill effluent, daily (Bh_res)", "3",
                          "jday   mon   day    yr  ob_typ  ob_name  flo sed orgn sedp no3 solp chla nh3 no2 cbod dox san sil cla sag lag grv temp"] + days + [""])
    # A town of 20,000 at 120 L a person a day, through septic tanks: 876,000 m3 a year at
    # 30 mg/L N and 5 mg/L P, 150 mg/L BOD, 100 mg/L suspended solids.
    q = 20000 * 0.12 * 365.0
    years = [f"     1     1     1{y:6d}  sdc  cha{TOWN_CHA:03d}  " + hyd(q, sed=q * 100e-6, orgn=q * 10e-3, sedp=q * 1e-3, no3=q * 5e-3, solp=q * 4e-3, nh3=q * 15e-3, cbod=q * 150e-3, dox=q * 2e-3, temp=28.0) for y in (2015, 2016, 2017)]
    write(p("town.rec"), ["town.rec: town wastewater, yearly (Bh_res)", "3",
                          "jday   mon   day    yr  ob_typ  ob_name  flo sed orgn sedp no3 solp chla nh3 no2 cbod dox san sil cla sag lag grv temp"] + years + [""])
    write(p("recall.rec"), ["recall.rec: Bh_res", "      id  name                     rec_typ  file",
                            "       1  mill                           1  mill.rec",
                            "       2  town                           3  town.rec", ""])
    rows = []
    for k, (name, c) in enumerate((("mill", MILL_CHA), ("town", TOWN_CHA)), 1):
        t = next(l.split() for l in read(p("chandeg.con"))[2:] if l.split() and int(l.split()[0]) == c)
        rows.append(f"{k:8d}  {name:<16}{k:8d}       0.00000  {t[4]:>12}  {t[5]:>12}  {t[6]:>12}  {k:8d}  {t[8]:>16}         0         0         0         1           sdc        {c:3d}           tot       1.00000  ")
    write(p("recall.con"), ["recall.con: Bh_res", hdr.replace("       res      ", "       rec      "), *rows, ""])

    # file.cio: the new files.
    lines = read(p("file.cio"))
    for i, line in enumerate(lines):
        t = line.split()
        if t and t[0] == "connect":
            # hru, hru-lte, rout_unit, gwflow, aquifer, aquifer2d, channel, reservoir, recall...
            assert t[8] == "null" and t[9] == "null", t
            t[8], t[9] = "reservoir.con", "recall.con"
        elif t and t[0] == "reservoir":
            assert t[2] == "null" and t[3] == "null", t
            t[2], t[3] = "reservoir.res", "hydrology.res"
        elif t and t[0] == "recall":
            t[1] = "recall.rec"
        else:
            continue
        lines[i] = t[0].ljust(18) + "".join(x.ljust(18) for x in t[1:])
    write(p("file.cio"), lines)

    # object.cnt: one reservoir, two recalls.
    lines = read(p("object.cnt"))
    h = lines[1].split()
    t = lines[2].split()
    t[h.index("obj")] = str(int(t[h.index("obj")]) + 3)
    t[h.index("res")] = "1"
    t[h.index("rec")] = "2"
    lines[2] = " ".join(x.rjust(13) for x in t)
    write(p("object.cnt"), lines)

    # 2015-2017, printed from the first day.
    lines = read(p("time.sim"))
    lines[2] = set_field(lines[2], 3, "2017")
    write(p("time.sim"), lines)
    lines = read(p("print.prt"))
    lines[2] = set_field(lines[2], 0, "0")
    write(p("print.prt"), lines)
    if os.path.exists(p("README.md")):
        os.remove(p("README.md"))
    print(f"Bh_res: {moved} inflows into the reservoir above cha{CHA:03d}; 2 point sources")


def bh_rel(out):
    src = os.path.join(out, "Bh_res")
    dst = os.path.join(out, "Bh_rel")
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst)

    def p(name):
        return os.path.join(dst, name)

    lines = read(p("res_rel.dtl"))
    lines[1] = str(int(lines[1].split()[0]) + 1)
    while lines and not lines[-1].strip():
        lines.pop()
    lines += [
        "",
        "name                     conds      alts      acts  ",
        "lake_multi                   5         3         6  ",
        "var                        obj   obj_num           lim_var            lim_op     lim_const      alt1      alt2      alt3  ",
        "vol                        res         0              pvol                 *       1.00000         <         >         -  ",
        "vol                        res         0              pvol                 *       1.30000         -         -         -  ",
        "vol                        res         0              evol                 *       1.00000         -         <         >  ",
        "month                     null         0              null                 -       5.00000         -         -         -  ",
        "month                     null         0              null                 -       9.00000         -         -         -  ",
        "act_typ                    obj   obj_num              name            option         const        const2                fp  outcome           ",
        "release                    res         0          nat_lake           natlake       0.00000       0.01000              null  y   n   n   ",
        "release                    res         0          hanasaki        nonirr-h06       0.85000       0.00000              null  y   n   n   ",
        "release                    res         0             hydro            hydrop       0.30000      30.00000              null  y   n   n   ",
        "release                    res         0          drawdown             dyrt1      30.00000       1.00000              con3  n   y   n   ",
        "release                    res         0        over_emerg        inflo_targ       1.00000       0.00000              null  n   n   y   ",
        "release                    res         0           minimum              rate       2.00000       0.00000              null  y   y   y   ",
        "",
    ]
    write(p("res_rel.dtl"), lines)
    lines = read(p("reservoir.res"))
    lines[2] = lines[2].replace("      flood_season", "        lake_multi")
    write(p("reservoir.res"), lines)
    print("Bh_rel: the reservoir released by natlake, nonirr-h06, hydrop, dyrt1 and inflo_targ")


def bh_nosed(out):
    src = os.path.join(out, "Bh_res")
    dst = os.path.join(out, "Bh_nosed")
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(src, dst)
    path = os.path.join(dst, "reservoir.res")
    lines = read(path)
    lines[2] = lines[2].replace("           sedres1", "           nosed01")
    write(path, lines)
    print("Bh_nosed: the reservoir without its sediment.res row")


if __name__ == "__main__":
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    bh_res(src, out)
    bh_rel(out)
    bh_nosed(out)
