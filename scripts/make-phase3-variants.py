#!/usr/bin/env python3
"""Builds the phase 3 validation scenarios from testdata/Batanghari (docs/phase3.md).

  Bh_land  Batanghari with what the port doesn't simulate yet replaced: Hargreaves PET instead of
           Penman-Monteith (codes.bsn pet 2), and the urban and wetland HRUs (urbn_lum, wetw_lum)
           given evergreen forest (frse_lum) and no surface storage. Everything else, the 1,858
           HRUs, their soils, plants, schedules and the whole object network, is Batanghari's.
  Bh_urb   Batanghari with only the wetland HRUs replaced (as in Bh_land): Penman-Monteith PET
           and the urban HRUs as they are.

usage: scripts/make-phase3-variants.py <Batanghari folder> <output folder>
"""
import os
import shutil
import sys


def rewrite(path, fn):
    with open(path, newline="") as f:
        lines = f.read().split("\n")
    with open(path, "w", newline="") as f:
        f.write("\n".join(fn(lines)))


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


def variant(src, out, name, pet, replaced):
    d = os.path.join(out, name)
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(src, d)

    def codes(lines):
        k = lines[1].split().index("pet")
        lines[2] = set_field(lines[2], k, pet)
        return lines

    if pet is not None:
        rewrite(os.path.join(d, "codes.bsn"), codes)

    def hrus(lines):
        h = lines[1].split()
        k_lu, k_ss = h.index("lu_mgt"), h.index("surf_stor")
        for i in range(2, len(lines)):
            t = lines[i].split()
            if len(t) <= k_ss:
                continue
            if t[k_lu] in replaced:
                lines[i] = set_field(lines[i], k_lu, "frse_lum".ljust(len(t[k_lu])).strip())
                lines[i] = set_field(lines[i], k_ss, "null")
        return lines

    rewrite(os.path.join(d, "hru-data.hru"), hrus)
    os.remove(os.path.join(d, "README.md"))


if __name__ == "__main__":
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    variant(src, out, "Bh_land", "2", ("urbn_lum", "wetw_lum"))
    variant(src, out, "Bh_urb", None, ("wetw_lum",))
