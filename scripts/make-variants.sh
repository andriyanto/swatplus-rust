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
