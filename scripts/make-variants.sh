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
