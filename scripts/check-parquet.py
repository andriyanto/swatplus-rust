#!/usr/bin/env python3
"""Checks the Parquet tables of a run (`swatplus-rust run --format both`) against the text
tables beside them: every .parquet must be readable by pyarrow and DuckDB, and hold the text
table's rows, column by column (integers and names as written, numbers as the 32-bit float
nearest the text).

usage: scripts/check-parquet.py <output folder>
"""
import glob
import math
import os
import struct
import sys

import duckdb
import pyarrow.parquet as pq


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def text_rows(path):
    names, rows = None, []
    with open(path) as f:
        next(f)
        for line in f:
            t = line.split()
            if not t:
                continue
            if names is None:
                if t[0] in ("jday", "year"):
                    names = t
                continue
            try:
                int(t[0])
            except ValueError:
                continue
            rows.append(t)
    return names, rows


def main(folder):
    bad = 0
    files = sorted(glob.glob(os.path.join(folder, "*.parquet")))
    if not files:
        print("no parquet files")
        return 1
    con = duckdb.connect()
    for p in files:
        txt = p[: -len(".parquet")] + ".txt"
        table = pq.read_table(p)
        n_duck = con.execute(f"select count(*) from read_parquet('{p}')").fetchone()[0]
        names, rows = text_rows(txt)
        if table.num_rows != len(rows) or n_duck != len(rows) or table.num_columns != len(names):
            print(f"{os.path.basename(p)}: {table.num_rows} rows (duckdb {n_duck}), {table.num_columns} columns; text {len(rows)} rows, {len(names)} columns")
            bad += 1
            continue
        cols = table.to_pydict()
        for k, name in enumerate(table.column_names):
            vals = cols[name]
            for i, r in enumerate(rows):
                tok = r[k] if k < len(r) else ""
                v = vals[i]
                if isinstance(v, str):
                    ok = v == tok
                elif isinstance(v, int):
                    ok = v == int(tok)
                else:
                    try:
                        e = f32(float(tok.replace(".E", ".0E")))
                    except ValueError:
                        e = float("nan")
                    ok = (math.isnan(v) and math.isnan(e)) or v == e
                if not ok:
                    print(f"{os.path.basename(p)}: row {i + 1} {name}: parquet {v!r}, text {tok!r}")
                    bad += 1
                    break
    print(f"{len(files)} parquet tables, {bad} bad")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
