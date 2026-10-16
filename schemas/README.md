# Schema store

A local copy of the RELAX NG schemas the bundled sample documents declare
(`<?xml-model href="…"?>`), so `tei-rs validate` can use each document's own
schema without network access. `manifest.json` lists every schema by its
canonical URL, with alias hrefs, fallback mirrors, the stored file, its
SHA-256 and when it was last fetched or confirmed current. Files are stored
under paths derived from their URLs, so relative `include`s resolve inside the
store; the manifest is written by `tei-rs schemas add` / `sync`, not by hand.

```sh
tei-rs schemas list  --schemas schemas    # what is stored
tei-rs schemas check --schemas schemas    # compare with the published versions; changes nothing
tei-rs schemas sync  --schemas schemas    # download outdated or missing schemas
tei-rs schemas which --schemas schemas doc.xml
```

`list` and `which` work in any build; `add`, `check` and `sync` need the
optional `sync` feature (`cargo install teixml --features sync`, or
`cargo run --release --features sync -- schemas …` in this repository).

`check` exits 0 when everything is current, 1 when anything is outdated or
missing, and 2 when a source can't be reached. It compares each schema and
every file it includes byte for byte with what its URL serves now; mirrors are
tried only when the canonical URL is unreachable.

## Contents (2026-10-16)

| Schema (canonical URL) | Fetched from | Licence (as found) |
|---|---|---|
| `https://epidoc.stoa.org/schema/8.13/tei-epidoc.rng` | mirror in github.com/papyri/idp.data, `Validation/EpiDoc/8.13/` | idp.data is CC BY 3.0; EpiDoc's own terms not verified |
| `https://epidoc.stoa.org/schema/8.16/tei-epidoc.rng` | mirror in github.com/papyri/idp.data, `Validation/EpiDoc/8.16/` | as above |
| `https://papyri.info/schemas/papyri.info.rng` | mirror in github.com/papyri/navigator, `pn-site/schemas/` | that repository is GPL-3.0 |
| `https://raw.githubusercontent.com/COST-ELTeC/Schemas/master/eltec-1.rng` (alias `../../Schemas/eltec-1.rng`, as ELTeC files declare it) | canonical URL | no licence file in the repository |
| `https://raw.githubusercontent.com/clarin-eric/ParlaMint/main/Schema/ParlaMint-TEI.rng` and `ParlaMint-TEI.ana.rng` (+ included `ParlaMint.rng`) | canonical URL | no licence file in the repository root |
| `https://raw.githubusercontent.com/BetaMasaheft/Schema/master/tei-betamesaheft.rng` (alias with lowercase `schema/`, as some files declare it) | canonical URL | CC BY-SA 4.0 (stated in the file) |
| `https://raw.githubusercontent.com/cbeta-org/xml-p5/master/schema/cbeta-p5.rnc` (compact syntax) | canonical URL | no licence file in the repository; CBETA's texts are non-commercial |
| `https://raw.githubusercontent.com/freedict/fd-dictionaries/master/shared/freedict-P5.rng` (FreeDict files don't declare it) | canonical URL | GPL-2.0-or-later (stated in the file) |
| `https://raw.githubusercontent.com/srophe/Gaddel/refs/heads/main/documentation/schemas/out/syriacaPersons.compiled.rng` and `syriacaPlaces.compiled.rng` | canonical URL | CC BY 3.0 (stated in the files) |
| `https://raw.githubusercontent.com/TEI-Correspondence-SIG/CMIF/master/schema/cmi-customization.rng` (most CMIF files don't declare it) | canonical URL | BSD-2-Clause or CC BY 4.0 |
| `https://raw.githubusercontent.com/84000/data-tei/master/schema/current/translation.rng` and `knowledgebase.rng` (aliases `../../../schema/current/translation.rng` and `../schema/current/knowledgebase.rng`, as 84000 files declare them) | canonical URL | CC BY-NC-ND 3.0 (repository README) |
| `https://raw.githubusercontent.com/bodleian/consolidated-tei-schema/master/msdesc.rng` (aliases with `msdesc/` and `msDesc/` as the owner, as Fihrist files declare it) | canonical URL | CC BY 4.0 (stated in the file) |
| `https://raw.githubusercontent.com/cligs/reference/master/tei/cligs.rng` | canonical URL | CC BY-SA 3.0 or BSD-2-Clause (stated in the file) |
| `https://raw.githubusercontent.com/DARIAH-ERIC/lexicalresources/master/Events/LexMC2017/Data/Etymology/basisformat.rng` (alias `basisformat.rng`) | canonical URL | CC BY-SA 3.0 DE (stated in the file) |
| `https://raw.githubusercontent.com/erc-dharma/project-documentation/master/schema/latest/DHARMA_Schema.rng` and `DHARMA_CritEdSchema.rng` | canonical URL | CC BY 4.0 (repository LICENCE.txt); `DHARMA_CritEdSchema.rng` states CC BY-SA 3.0 or BSD-2-Clause |
| `https://raw.githubusercontent.com/pruizf/disco/master/schema/tei_all_DISCO.rnc` (compact syntax) | canonical URL | BSD-2-Clause or CC BY 3.0 (TEI, stated in the file) |
| `https://raw.githubusercontent.com/chartes/encpos/master/data/encpos.rng` (alias `../encpos.rng`) | canonical URL | no licence stated |
| `https://raw.githubusercontent.com/FrankensteinVariorum/fv-data/master/FV_ODD/out/FV_ODD.rng` (alias `../FV_ODD/out/FV_ODD.rng`) | canonical URL | CC BY-SA 3.0 or BSD-2-Clause (stated in the file) |
| `https://raw.githubusercontent.com/HistoryAtState/frus/master/schema/frus.rnc` (compact syntax; FRUS volumes don't declare it) | canonical URL | public domain (U.S. government work; repository README) |
| `https://raw.githubusercontent.com/Jean-Baptiste-Camps/Geste/master/odd/out/Modele.rng` (alias `../odd/out/Modele.rng`) | canonical URL | CC BY-SA 4.0 (repository README) |
| `https://raw.githubusercontent.com/MiMoText/roman18/master/Schemas/eltec-x.rng` (alias `../../Schemas/eltec-x.rng`) | canonical URL | MIT (repository LICENSE) |
| `https://raw.githubusercontent.com/COST-ELTeC/Schemas/master/eltec-x.rng` (alias `https://github.com/COST-ELTEC/Schemas/raw/master/eltec-x.rng`) | canonical URL | no licence file in the repository |
| `http://oeuvres.github.io/Teinte/teinte.rng` (+ included `teinteHeader.rng`; alias with lowercase `teinte/`), used by OBVIL | mirror in github.com/oeuvres/Teinte, branch `main` | BSD-3-Clause (repository LICENSE) |
| `https://openarabicpe.github.io/OpenArabicPE_ODD/schema/tei_periodical.rng` | mirror in github.com/OpenArabicPE/OpenArabicPE_ODD, branch `master` | CC BY-SA 4.0 (stated in the file) |
| `https://tei-c.org/release/xml/tei/custom/schema/relaxng/tei_all.rng` | **not downloaded yet** | BSD-2 or CC BY 3.0 (TEI) |
| `https://epidoc.stoa.org/schema/latest/tei-epidoc.rng`, `…/9.7/tei-epidoc.rng` | **not downloaded yet** | |
| `https://dracor.org/schema.rng` | **not downloaded yet** | |

The entries marked "not downloaded yet" could not be reached from the
environment that created this store. Run `tei-rs schemas sync --schemas
schemas` on a machine with internet access to fetch them (and to replace the
mirror copies with the canonical ones if they differ).

These files are third-party and used unmodified for local validation and
benchmarking. [`licences.tsv`](licences.tsv) gives each file's licence and
whether it may be published: files with no licence stated are left out of the
public repository, and `tei-rs schemas sync --schemas schemas` downloads them.
