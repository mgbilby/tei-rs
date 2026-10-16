# Third-party notices

teixml is licensed under the [MIT license](LICENSE). This file lists the
third-party material in this repository, under its own licences. Only the
code under `src/` is published to crates.io.

## Code

| Material | Licence | Notice |
|---|---|---|
| `src/relaxng/`: RELAX NG engine derived from [`relax-ng`](https://crates.io/crates/relax-ng) 0.2.0 by CASOON | MIT | [`src/relaxng/LICENSE`](src/relaxng/LICENSE); changes in [`src/relaxng/PROVENANCE.md`](src/relaxng/PROVENANCE.md) |
| Cargo dependencies | permissive only: MIT, Apache-2.0, ISC, BSD-3-Clause, Zlib, Unicode-3.0 (several dual-licensed); with the `sync` feature also CDLA-Permissive-2.0 (`webpki-roots`) | in each crate; `cargo metadata` lists them |

## Test and benchmark data

| Material | Licence | Notice |
|---|---|---|
| `tests/corpus/relaxng/`: the RELAX NG test suite from [jing-trang](https://github.com/relaxng/jing-trang) | BSD-3-Clause, © 2001–2003 Thai Open Source Software Center Ltd | [`tests/corpus/relaxng/copying.txt`](tests/corpus/relaxng/copying.txt) |
| `tests/tei/schemas/tei_all-2.7.0.rng` and `tests/tei/docs/tei-c/`: TEI P5 | BSD-2-Clause or CC BY 3.0, © TEI Consortium | [`LICENSES/BSD-2-Clause.txt`](LICENSES/BSD-2-Clause.txt) |
| `tests/tei/bench/`, `tests/tei/docs/`: sample documents from 20 TEI projects | per project: CC0, CC BY, CC BY-SA, GPL; some non-commercial or with no licence stated | [`tests/tei/SOURCES.md`](tests/tei/SOURCES.md), [`tests/tei/licences.tsv`](tests/tei/licences.tsv) |
| `schemas/`: project schemas | per file: CC BY, CC BY-SA, GPL, BSD-2-Clause; some with no licence stated | [`schemas/README.md`](schemas/README.md), [`schemas/licences.tsv`](schemas/licences.tsv) |

GPL-licensed files (FreeDict dictionaries and schema, the papyri.info
schema) come with the GPL texts in [`LICENSES/`](LICENSES). Creative Commons
material is attributed in the files listed above, with links to the licences.

Files whose licence doesn't allow redistribution, or that state no licence,
are not in the public repository; `scripts/fetch-samples.sh` and
`tei-rs schemas sync` download them from their sources.
