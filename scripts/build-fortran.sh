#!/usr/bin/env bash
# Builds the SWAT+ Fortran reference (the commit swatplus-rust follows) as the CI does
# (.github/workflows/fortran.yml): Release, gfortran, Ninja. Prints the program's path.
#
#   scripts/build-fortran.sh [folder]     (default: ../swatplus-fortran)
set -euo pipefail
REF=6f515368525e7d84089ae0f9d33f4c9115de5679
dir=${1:-../swatplus-fortran}
if [ ! -d "$dir/.git" ]; then
  git clone -q https://github.com/swat-model/swatplus.git "$dir"
fi
cd "$dir"
git fetch -q origin "$REF" 2>/dev/null || true
git checkout -q "$REF"
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_Fortran_COMPILER=gfortran -DTAG="${REF:0:7}" > build.log
cmake --build build >> build.log
exe=$(find "$PWD/build" -maxdepth 1 -type f -name 'swatplus-*' -perm -u+x | head -1)
test -n "$exe"
echo "$exe"
