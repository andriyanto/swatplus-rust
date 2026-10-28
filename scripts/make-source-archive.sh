#!/usr/bin/env bash
# The source archive of a release, for Zenodo: the files git tracks at HEAD (or at the commit
# given), without testdata/Batanghari and testdata/Pessel, which are private data that stays in
# this repository. SWAT+'s own scenarios (Osu_1hru, Ames_sub1) are kept.
#
#   scripts/make-source-archive.sh [commit]    -> dist/swatplus-rust-<version>-src.{tar.gz,zip}
set -euo pipefail
cd "$(dirname "$0")/.."
rev=${1:-HEAD}
version=$(git show "$rev:Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)
name=swatplus-rust-$version
mkdir -p dist
tmp=$(mktemp -d)
git archive --format=tar --prefix="$name/" "$rev" | tar -x -C "$tmp"
rm -rf "$tmp/$name/testdata/Batanghari" "$tmp/$name/testdata/Pessel"
cat > "$tmp/$name/testdata/PRIVATE-DATA.txt" <<'TXT'
Two of the validation scenarios, Batanghari and Pessel (West Sumatra, Indonesia), are private
data of their owner and are not in this archive. The tests and scripts that use them skip them
or need them in testdata/. They can be requested from the author for validation.
TXT
grep -rIl -e 'private data, do not ''publish' --exclude=make-source-archive.sh "$tmp/$name" && { echo "private data left in the archive" >&2; exit 1; }
(cd "$tmp" && tar -czf "$OLDPWD/dist/$name-src.tar.gz" "$name" && zip -qr "$OLDPWD/dist/$name-src.zip" "$name")
rm -rf "$tmp"
ls -l dist/"$name"-src.*
sha256sum dist/"$name"-src.*
