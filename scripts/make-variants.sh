#!/usr/bin/env bash
# Builds the phase 2 validation scenarios from Ames_sub1, which entangles water with crops,
# management and irrigation. Each variant changes only the HRUs' land use:
#
#   Ames_bare  bare soil: a land use with no plant community and no management (curve numbers of
#              fallow bare soil). Isolates the soil water balance: snow, soil evaporation, runoff,
#              percolation, lateral flow, soil temperature.
#   Ames_past  pasture (past_lum): a perennial plant community without management. Adds plant
#              growth, canopy interception and transpiration, and with them the soil nutrients
#              that limit growth (the carbon model as Ames has it, cswat 2).
#   Ames_past0 the same pasture with the original carbon and nutrient model (codes.bsn carbon 0,
#              SWAT+ Editor's default).
#   Ames_corn  corn alone under mgt_01's dated operations (below).
#   Ames_cornf Ames_corn with its fertilizer actually applied (below).
#   Ames_cosy, Ames_cosy2, Ames_dtbl: Ames_sub1's own land uses, step by step (below).
#   Ames_tile  Ames_sub1 with tile drains under its corn and soybean (phase 4).
#
# Both drop soil_lyr_depths.sol, so SWAT+ keeps the layers of soils.sol (plus its 10 mm top layer)
# instead of re-layering every soil to custom depths; that path is validated later.
#
# usage: scripts/make-variants.sh <Ames_sub1 folder> <output folder>
set -euo pipefail
src=$1
out=$2
mkdir -p "$out"

variant() {
  local name=$1 lum=$2
  rm -rf "$out/$name"
  cp -r "$src" "$out/$name"
  awk -v lu="$lum" '
    { sub(/\r$/, "") }
    NR <= 2 { print; next }
    NF >= 10 { $6 = lu; printf "%8s  %-22s%-18s%-18s%-17s%-16s%-18s%-16s%-14s%s\n", $1, $2, $3, $4, $5, $6, $7, $8, $9, $10; next }
    { print }
  ' "$src/hru-data.hru" > "$out/$name/hru-data.hru"
  rm -f "$out/$name/soil_lyr_depths.sol"
}

variant Ames_bare bare_lum
printf "%-34s%-18s%-48s%-15s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s\n" \
  bare_lum null null null fal_bare up_down_slope null null convtill_nores null null null null null \
  >> "$out/Ames_bare/landuse.lum"

variant Ames_past past_lum

rm -rf "$out/Ames_past0"
cp -r "$out/Ames_past" "$out/Ames_past0"
awk '{ sub(/\r$/, "") } NR == 3 { $14 = 0 } { print }' "$out/Ames_past/codes.bsn" > "$out/Ames_past0/codes.bsn"

# Ames_corn: the HRUs' land use with corn alone (corn_comm) and mgt_01 without its decision
# table (irrigation on conditions): planting, harvest, tillage, fertilizer and the one
# scheduled irrigation.
variant Ames_corn corn_lum
printf "%-34s%-18s%-48s%-15s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s%-18s\n" \
  corn_lum null corn_comm mgt_corn rc_strow_g up_down_slope null null convtill_nores null null null null null \
  >> "$out/Ames_corn/landuse.lum"
cp "$src/management.sch" "$out/Ames_corn/management.sch"
echo >> "$out/Ames_corn/management.sch"
awk '{ sub(/\r$/, "") }
  $1 == "mgt_01" && !done { n = $2; a = $3; print "mgt_corn", n, 0; skip = a; take = n; done = 1; next }
  skip > 0 { skip--; next }
  take > 0 { print; take-- }' "$src/management.sch" >> "$out/Ames_corn/management.sch"

# Ames_cornf: Ames_corn with its fertilizer applied. mgt_01 names the fertilizers in the second
# column of its fert operations, where SWAT+ looks for the application method, so nothing is
# applied in Ames_corn. Here the names move to the first column, elemental N goes in by
# injection (20% at the surface) and the rest is broadcast; 05_10_15 becomes dairy manure, to
# take the organic path of the carbon model.
rm -rf "$out/Ames_cornf"
cp -r "$out/Ames_corn" "$out/Ames_cornf"
cp "$(dirname "$0")/../testdata/Osu_1hru/chem_app.ops" "$out/Ames_cornf/chem_app.ops"
awk '$1 == "mgt_corn" { on = 1 }
  on && $1 == "fert" { f = ($6 == "05_10_15") ? "dairy_fr" : $6; m = ($6 == "elem_n") ? "inject" : "broadcast";
    printf "%56s %10s %10s %6s %16s %16s %9s\n", $1, $2, $3, $4, f, m, $7; next }
  { print }' "$out/Ames_corn/management.sch" > "$out/Ames_cornf/management.sch"

# Ames_cosy: Ames_sub1's own land uses (cosy_comm, corn and soybean, under mgt_01 in 11 HRUs;
# corn_comm under mgt_08 in the 12th) with mgt_01's decision table (irrigation on conditions)
# left out: a community of two plants, of which mgt_01 only ever plants corn.
rm -rf "$out/Ames_cosy"
cp -r "$src" "$out/Ames_cosy"
rm -f "$out/Ames_cosy/soil_lyr_depths.sol"
awk '{ sub(/\r$/, "") } $1 == "cosy_lum" { sub(/mgt_01 /, "mgt_corn") } { print }' \
  "$src/landuse.lum" > "$out/Ames_cosy/landuse.lum"
cp "$out/Ames_corn/management.sch" "$out/Ames_cosy/management.sch"

# Ames_cosy2: Ames_cosy with corn and soybean grown together: every planting and every
# harvest of corn is followed, the same day, by one of soybean (so the two compete for light,
# water and nutrients).
rm -rf "$out/Ames_cosy2"
cp -r "$out/Ames_cosy" "$out/Ames_cosy2"
awk '{ sub(/\r$/, "") }
  $1 == "mgt_corn" { on = 1; buf = ""; count = 0; head = $0; next }
  !on { print; next }
  { line = $0; buf = buf line "\n"; count++
    if (($1 == "plnt" || $1 == "hvkl") && $5 == "corn") { sub(/ corn /, " soyb ", line); buf = buf line "\n"; count++ } }
  END { split(head, h, " "); print h[1], count, h[3]; printf "%s", buf }' \
  "$out/Ames_cosy/management.sch" > "$out/Ames_cosy2/management.sch"

# Ames_dtbl: Ames_sub1 as it is, decision table included, but without soil_lyr_depths.sol.
rm -rf "$out/Ames_dtbl"
cp -r "$src" "$out/Ames_dtbl"
rm -f "$out/Ames_dtbl/soil_lyr_depths.sol"

# Ames_tile (phase 4): Ames_sub1 as it is, with its corn and soybean (cosy_lum) on tile drains
# 1 m deep (tiledrain.str mw24_1000), as much of Iowa's cropland is.
rm -rf "$out/Ames_tile"
cp -r "$src" "$out/Ames_tile"
awk '
  { sub(/\r$/, "") }
  NR > 2 && $1 == "cosy_lum" { $10 = "mw24_1000"; line = ""; for (i = 1; i <= NF; i++) line = line sprintf("%-18s", $i); print line; next }
  { print }
' "$src/landuse.lum" > "$out/Ames_tile/landuse.lum"
