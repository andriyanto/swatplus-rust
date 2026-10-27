#!/usr/bin/env bash
# Builds the SWAT+ Fortran reference (the commit swatplus-rust follows) as the CI does
# (.github/workflows/fortran.yml): Release, gfortran, Ninja. Prints the program's path.
#
#   scripts/build-fortran.sh [folder]     (default: ../swatplus-fortran)
#
# FORTRAN_FLAGS adds flags to every compile, e.g. FORTRAN_FLAGS=-ffp-contract=off: on ARM64
# (aarch64) gcc fuses a*b+c into one rounding (FMA) by default, which x86-64 builds and the
# port don't, so the tables can differ in the last bits. The build goes to build/ or, with
# flags, to build-<flags>/.
set -euo pipefail
REF=6f515368525e7d84089ae0f9d33f4c9115de5679
dir=${1:-../swatplus-fortran}
if [ ! -d "$dir/.git" ]; then
  git clone -q https://github.com/swat-model/swatplus.git "$dir"
fi
cd "$dir"
git fetch -q origin "$REF" 2>/dev/null || true
git checkout -q "$REF"
flags=${FORTRAN_FLAGS:-}
b=build
[ -z "$flags" ] || b=build-$(echo "$flags" | tr -c 'A-Za-z0-9=-' '_' | sed 's/_*$//')
cmake -B "$b" -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_Fortran_COMPILER=gfortran -DCMAKE_Fortran_FLAGS="$flags" -DTAG="${REF:0:7}" > "$b.log"
cmake --build "$b" >> "$b.log"
exe=$(find "$PWD/$b" -maxdepth 1 -type f -name 'swatplus-*' -perm -u+x | head -1)
test -n "$exe"
echo "$exe"
