#!/usr/bin/env python3
"""A copy of a SWAT+ project whose precipitation comes from GSMaP instead of its own files.

    python3 scripts/gsmap-to-pcp.py <TxtInOut> <gsmap_daily_G_crop.zip> <new TxtInOut>

The zip is what calibration/../gsmap_crop.py (or the script given to users) writes: daily
GSMaP_Gauge v8 values (00Z-23Z) cut to a box, as JAXA stores them. For each precipitation station of
the project's pcp.cli, a new file gs<name>.pcp gets the daily value of the GSMaP cell holding the
station (the daily product is a mean rain rate, mm/h: times 24 gives mm/day; a day without data is
-99, which SWAT+ fills from its weather generator). pcp.cli and weather-sta.cli of the copy point to
the new files; everything else is copied unchanged. The station's latitude, longitude and elevation
are kept.

GSMaP data are JAXA's, under JAXA's terms; neither the zip nor the files made from it belong in a
public repository.
"""

import datetime as dt
import json
import os
import shutil
import sys
import zipfile

import numpy as np


def main(src, zpath, dest):
    z = zipfile.ZipFile(zpath)
    h = json.loads(z.read("header.json"))
    rows, cols = h["rows"], h["cols"]
    north, west = h["box"]["north"], h["box"]["west"]
    days = sorted(n for n in z.namelist() if n.startswith("days/") and n.endswith(".f32"))
    dates = [dt.date(int(n[5:9]), int(n[9:11]), int(n[11:13])) for n in days]
    grid = np.stack([np.frombuffer(z.read(n), "<f4").reshape(rows, cols) for n in days])
    if os.path.exists(dest):
        shutil.rmtree(dest)
    shutil.copytree(src, dest)

    cli = open(os.path.join(src, "pcp.cli")).read().splitlines()
    names = [l.strip() for l in cli[2:] if l.strip()]
    renamed = {}
    for name in names:
        lines = open(os.path.join(src, name)).read().splitlines()
        lat, lon, elev = (float(v) for v in lines[2].split()[2:5])
        r, c = int((north - lat) / 0.1), int((lon - west) / 0.1)
        if not (0 <= r < rows and 0 <= c < cols):
            raise SystemExit(f"{name}: station at {lat}, {lon} is outside the GSMaP box")
        new = "gs" + name
        with open(os.path.join(dest, new), "w") as f:
            f.write(f"{new}: Precipitation data - GSMaP_Gauge v8 daily 00Z-23Z (JAXA), cell {r},{c}, mm/h x 24\n")
            f.write("nbyr     tstep       lat       lon      elev\n")
            f.write(f"{dates[-1].year - dates[0].year + 1:>4}{0:>10}{lat:>10.3f}{lon:>10.3f}{elev:>10.3f}\n")
            for d, v in zip(dates, grid[:, r, c]):
                mm = float(v) * 24.0 if v >= 0 else -99.0
                f.write(f"{d.year:>4}{d.timetuple().tm_yday:>5}{mm:>11.5f}\n")
        renamed[name] = new
    with open(os.path.join(dest, "pcp.cli"), "w") as f:
        f.write("pcp.cli: Precipitation file names - GSMaP_Gauge v8 daily (JAXA), written by gsmap-to-pcp.py\n")
        f.write(cli[1] + "\n")
        for name in names:
            f.write(renamed[name] + "\n")
    sta = os.path.join(dest, "weather-sta.cli")
    text = open(sta).read()
    for old, new in renamed.items():
        text = text.replace(f" {old} ", f" {new} ")
    open(sta, "w").write(text)
    print(f"{len(names)} stations, {dates[0]} to {dates[-1]}, written to {dest}")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    main(*sys.argv[1:])
