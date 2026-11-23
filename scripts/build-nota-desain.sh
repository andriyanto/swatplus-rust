#!/usr/bin/env bash
# Rebuilds the design note's single files from its chapters (docs/nota-desain): nota-desain.md
# (the chapters in order under one title), then nota-desain.docx and nota-desain.pdf with pandoc
# (xelatex and DejaVu fonts for the PDF). Run after changing a chapter.
#
#   scripts/build-nota-desain.sh "<subtitle>"     e.g. "swatplus-rust 1.1, fungsi demi fungsi"
set -euo pipefail
cd "$(dirname "$0")/../docs/nota-desain"
title="Nota Desain SWAT+ Rust Engine 1.1"
subtitle=${1:-"swatplus-rust 1.1, fungsi demi fungsi, dengan catatan kerja jalur M (Lampiran B)"}
{
  echo "# $title"
  echo
  for f in [0-9][0-9]-*.md; do
    cat "$f"
    echo
  done
} > nota-desain.md
common=(nota-desain.md --shift-heading-level-by=-1 -M "title=$title" -M "subtitle=$subtitle" --toc --resource-path=.)
pandoc "${common[@]}" -o nota-desain.docx
pandoc "${common[@]}" --pdf-engine=xelatex -V mainfont="DejaVu Serif" -V monofont="DejaVu Sans Mono" \
  -V geometry:margin=2.5cm -V lang=id -o nota-desain.pdf
ls -l nota-desain.md nota-desain.docx nota-desain.pdf
