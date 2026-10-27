#!/usr/bin/env bash
# Times SWAT+ Fortran and swatplus-rust on the same scenarios, on this machine (docs/benchmark.md).
#
#   FORTRAN=<SWAT+ program> scripts/bench.sh [scenario...]
#
# Scenarios are folders of testdata/ (default: Osu_1hru Ames_sub1 Batanghari Pessel). Each run
# copies the scenario into a fresh folder under $WORK, runs one engine there with the scenario's
# own time.sim and print.prt, and records the wall time, the peak memory and what it wrote. One
# warm-up run per engine and scenario is not counted.
#
#   REPS=5        runs counted per engine and scenario
#   END_YEAR=     if set, every scenario ends that year (a quicker run)
#   WORK=...      where the runs go (on Windows: inside WSL, not under /mnt/c)
#   RUST=...      swatplus-rust (default: target/release/swatplus-rust, built here)
set -euo pipefail
cd "$(dirname "$0")/.."
REPS=${REPS:-5}
WORK=${WORK:-$HOME/swat-bench}
RUST=${RUST:-$PWD/target/release/swatplus-rust}
: "${FORTRAN:?set FORTRAN to the SWAT+ Fortran program (scripts/build-fortran.sh builds it)}"
FORTRAN=$(readlink -f "$FORTRAN")
scenarios=("$@")
[ ${#scenarios[@]} -gt 0 ] || scenarios=(Osu_1hru Ames_sub1 Batanghari Pessel)
TIMER="$PWD/scripts/bench-run.py"

if [ "$RUST" = "$PWD/target/release/swatplus-rust" ]; then
  cargo build --release -q -p swatplus-rust
fi

stamp=$(date +%Y%m%d-%H%M%S)
out=bench-results/$stamp
mkdir -p "$out" "$WORK"
csv=$out/runs.csv
echo "scenario,engine,rep,wall_s,user_s,sys_s,max_rss_kb,files,bytes" > "$csv"

# The machine and the programs, for the record.
{
  echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "host: $(uname -srm)"
  echo "arch: $(uname -m)"
  grep -m1 "model name" /proc/cpuinfo 2>/dev/null | sed 's/^/cpu: /' || true
  echo "cores: $(nproc)"
  awk '/MemTotal/ {printf "memory: %.1f GB\n", $2/1048576}' /proc/meminfo 2>/dev/null || true
  grep -qi microsoft /proc/version 2>/dev/null && echo "wsl: yes" || echo "wsl: no"
  echo "swatplus-rust: $("$RUST" --version) ($(git rev-parse --short HEAD))"
  echo "rustc: $(rustc --version 2>/dev/null || echo unknown)"
  echo "fortran: $FORTRAN"
  echo "gfortran: $(gfortran --version 2>/dev/null | head -1 || echo unknown)"
  echo "fortran sha256: $(sha256sum "$FORTRAN" | cut -d' ' -f1)"
  echo "reps: $REPS"
  echo "end year: ${END_YEAR:-as in time.sim}"
} > "$out/machine.txt"
cat "$out/machine.txt"

prepare() { # scenario dest
  rm -rf "$2"
  cp -r "testdata/$1" "$2"
  if [ -n "${END_YEAR:-}" ]; then
    awk -v y="$END_YEAR" 'NR == 3 { $4 = y; printf "%8s%10s%10s%10s%10s\n", $1, $2, $3, $4, $5; next } { print }' "$2/time.sim" > "$2/time.tmp"
    mv "$2/time.tmp" "$2/time.sim"
  fi
}

run_one() { # scenario engine rep(0 = warm-up)
  local s=$1 e=$2 r=$3 d="$WORK/$1-$2" before
  prepare "$s" "$d"
  before=$(find "$d" -type f | sort)
  if [ "$e" = fortran ]; then
    (cd "$d" && python3 "$TIMER" "$d/.time" "$FORTRAN" > "$d/.log" 2>&1) || { echo "$s fortran failed (see $d/.log)" >&2; exit 1; }
    test -f "$d/success.fin"
    outdir=$d
  else
    python3 "$TIMER" "$d/.time" "$RUST" run "$d" --out "$d/out" > "$d/.log" 2>&1 || { echo "$s rust failed (see $d/.log)" >&2; exit 1; }
    outdir=$d/out
  fi
  # What the run wrote: the files that weren't in the scenario (all of the port's --out folder).
  local files bytes
  if [ "$e" = fortran ]; then
    read -r files bytes < <(comm -13 <(echo "$before") <(find "$d" -type f ! -name '.time' ! -name '.log' | sort) | xargs -r stat -c %s | awk '{n++; b+=$1} END {print n+0, b+0}')
  else
    read -r files bytes < <(find "$outdir" -type f -printf '%s\n' | awk '{n++; b+=$1} END {print n+0, b+0}')
  fi
  read -r wall user sys rss < "$d/.time"
  if [ "$r" -gt 0 ]; then
    echo "$s,$e,$r,$wall,$user,$sys,$rss,$files,$bytes" >> "$csv"
  fi
  printf '  %-11s %-8s %s  %8ss  %7.0f MB\n' "$s" "$e" "$([ "$r" -gt 0 ] && echo "run $r" || echo "warm-up")" "$wall" "$(awk -v k="$rss" 'BEGIN {print k / 1024}')"
}

for s in "${scenarios[@]}"; do
  test -d "testdata/$s" || { echo "no testdata/$s" >&2; exit 1; }
  echo "$s"
  for e in fortran rust; do run_one "$s" "$e" 0; done
  # Alternate the engines so a slow spell of the machine hits both.
  for r in $(seq 1 "$REPS"); do
    for e in fortran rust; do run_one "$s" "$e" "$r"; done
  done
done

# Do the two engines agree here? Every table the port wrote against SWAT+'s, by
# `swatplus-rust compare` (decision K12's tolerance), from each scenario's last runs.
echo "scenario,table,result" > "$out/agreement.csv"
for s in "${scenarios[@]}"; do
  for t in "$WORK/$s-rust/out"/*.txt; do
    [ -e "$t" ] || continue
    f="$WORK/$s-fortran/$(basename "$t")"
    [ -f "$f" ] || continue
    if "$RUST" compare "$f" "$t" > "$out/$s-$(basename "$t" .txt).cmp" 2>&1; then r=ok; else r=mismatch; fi
    cmp -s <(tail -n +2 "$f" | tr -s ' ') <(tail -n +2 "$t" | tr -s ' ') && r=identical
    echo "$s,$(basename "$t" .txt),$r" >> "$out/agreement.csv"
  done
done
echo "agreement: $(tail -n +2 "$out/agreement.csv" | cut -d, -f3 | sort | uniq -c | tr '\n' ' ')"

python3 scripts/bench-summary.py "$csv" "$out/machine.txt" "$out/agreement.csv" > "$out/summary.md"
cat "$out/summary.md"
echo
echo "results: $out (runs.csv, agreement.csv, machine.txt, summary.md)"
