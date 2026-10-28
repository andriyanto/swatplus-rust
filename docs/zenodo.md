# Publishing a release on Zenodo

Published: 1.0.1 on 28 October 2026, [10.5281/zenodo.23721044](https://doi.org/10.5281/zenodo.23721044); all versions [10.5281/zenodo.23721043](https://doi.org/10.5281/zenodo.23721043). Files restricted, shared on request.

The repository is private, so a release goes to Zenodo by hand (Zenodo's GitHub integration
archives public repositories). Its metadata is in `.zenodo.json` and `CITATION.cff`.

1. Merge the release into `main`, tag it (`git tag -a v1.0.1 -m "swatplus-rust 1.0.1"`, push the
   tag).
2. `scripts/make-source-archive.sh v1.0.1` writes `dist/swatplus-rust-1.0.1-src.tar.gz` and
   `.zip`: the tagged files without the private scenarios (Batanghari, Pessel).
3. On zenodo.org: *New upload*. Files: the `.zip` and `docs/nota-desain/nota-desain.pdf`.
   Fill the form from `.zenodo.json`: resource type *Software*, title, creators, description,
   version, license *LGPL-2.1-only*, keywords, related work (*Is derived from* SWAT+).
4. Access: *Restricted* files, with access requests on. The record and its DOI are public; the
   files go to whom you grant them.
5. *Publish*. The DOI goes into `CITATION.cff`, `.zenodo.json` (as `doi`) and the README.

A new version later: *New version* on the record, the next tag's archive.
