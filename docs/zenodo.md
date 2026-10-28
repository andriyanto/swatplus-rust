# Publishing a release on Zenodo

Published on 28 October 2026: 1.0.1 [10.5281/zenodo.23721044](https://doi.org/10.5281/zenodo.23721044), 1.0.2 [10.5281/zenodo.23728544](https://doi.org/10.5281/zenodo.23728544); all versions [10.5281/zenodo.23721043](https://doi.org/10.5281/zenodo.23721043). 1.0.2's files open (Geoscientific Model Development requires the code in a public archive).

Up to 1.0.2 the repository was private, so releases went to Zenodo by hand; it is public from
1.0.3, without the private scenarios (`testdata/PRIVATE.md`), and Zenodo's GitHub integration can
archive its releases. Its metadata is in `.zenodo.json` and `CITATION.cff`.

1. Merge the release into `main`, tag it (`git tag -a v1.0.1 -m "swatplus-rust 1.0.1"`, push the
   tag).
2. `scripts/make-source-archive.sh v1.0.1` writes `dist/swatplus-rust-1.0.1-src.tar.gz` and
   `.zip`: the tagged files without the private scenarios (Batanghari, Pessel).
3. On zenodo.org: *New upload*. Files: the `.zip` and `docs/nota-desain/nota-desain.pdf`.
   Fill the form from `.zenodo.json`: resource type *Software*, title, creators, description,
   version, license *LGPL-2.1-only*, keywords, related work (*Is derived from* SWAT+).
4. Access: *Public* files. (1.0.1 was published with *Restricted* files and access requests;
   from 1.0.2 they are open, as GMD's code policy needs.)
5. *Publish*. The DOI goes into `CITATION.cff`, `.zenodo.json` (as `doi`) and the README.

A new version later: *New version* on the record, the next tag's archive.
