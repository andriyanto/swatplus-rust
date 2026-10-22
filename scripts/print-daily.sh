#!/usr/bin/env bash
# Edits a scenario's print.prt for validation runs: the objects in ON print daily, monthly, yearly
# and average annual; the objects in OFF print no daily output (Ames_sub1's daily carbon
# diagnostics alone are 9 GB). Other rows keep their settings.
#
# usage: scripts/print-daily.sh <scenario folder> <on: a,b,...> [<off: c,d,...>]
set -euo pipefail
dir=$1
on=$2
off=${3:-}
prt="$dir/print.prt"
awk -v on="$on" -v off="$off" '
  BEGIN {
    n = split(on, a, ","); for (i = 1; i <= n; i++) ON[a[i]] = 1
    m = split(off, b, ","); for (i = 1; i <= m; i++) OFF[b[i]] = 1
  }
  { sub(/\r$/, "") }
  $1 == "objects" { table = 1; print; next }
  table && NF == 5 && ($1 in ON) { printf "%-25s%14s%14s%14s%14s\n", $1, "y", "y", "y", "y"; next }
  table && NF == 5 && ($1 in OFF) { printf "%-25s%14s%14s%14s%14s\n", $1, "n", $3, $4, $5; next }
  { print }
' "$prt" > "$prt.new"
mv "$prt.new" "$prt"
