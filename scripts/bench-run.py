#!/usr/bin/env python3
"""Runs a command and writes its wall time, user and system CPU time (s) and peak memory (KB)
to a file, one line: `bench-run.py <out file> <command...>` (scripts/bench.sh). Output goes to
the files the caller redirects; the exit code is the command's."""
import os
import resource
import subprocess
import sys
import time

out, cmd = sys.argv[1], sys.argv[2:]
t0 = time.perf_counter()
code = subprocess.call(cmd)
wall = time.perf_counter() - t0
ru = resource.getrusage(resource.RUSAGE_CHILDREN)
# ru_maxrss is in KB on Linux, in bytes on macOS.
rss = ru.ru_maxrss / 1024 if sys.platform == "darwin" else ru.ru_maxrss
with open(out, "w") as f:
    f.write(f"{wall:.3f} {ru.ru_utime:.3f} {ru.ru_stime:.3f} {rss:.0f}\n")
sys.exit(code)
