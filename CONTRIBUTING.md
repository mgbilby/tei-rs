# Contributing to teixml

## Repository layout

| Path | What |
|---|---|
| `src/` | The `teixml` library and the `tei-rs` command-line tool (`src/main.rs`) |
| `src/schematron.rs`, `src/xpath/` | Schematron rules embedded in schemas (`teixml::schematron`, the `schematron` feature) and the XPath 2.0 evaluator they run on |
| `src/relaxng/` | RELAX NG engine (`teixml::relaxng`), derived from the MIT-licensed [`relax-ng`](https://crates.io/crates/relax-ng) 0.2.0 — every change is recorded in [`PROVENANCE.md`](src/relaxng/PROVENANCE.md) |
| `tests/corpus/relaxng/` | The official RELAX NG conformance suite (BSD-3-Clause), used by the engine's tests |
| `tests/tei/docs/` | Small TEI fixtures: TEI Consortium test files, GerDraCor, I.Sicily |
| `tests/tei/bench/` | Benchmark samples from 50 TEI projects, one folder per project |
| `tests/tei/SAMPLES.md`, `samples.tsv` | Every sample file with a link to its pinned upstream version and SHA-256 |
| `tests/tei/SOURCES.md` | Where every data file came from, and its licence |
| `tests/tei/expected*.tsv` | Jing's verdict for every sample, checked by `tests/jing_agreement.rs` |
| `tests/tei/schematron-expected.tsv` | The reference Schematron messages for every sample, checked by `tests/schematron_agreement.rs` |
| `tests/tei/schematron/` | Made-up documents that break as many embedded Schematron rules as possible |
| `schemas/` | Schema store with the project schemas the samples declare; see its [README](schemas/README.md) |
| `scripts/` | Sample download, Jing and Schematron comparison, and timing scripts |

## Sample documents

The TEI samples come from their upstream projects at pinned commits; see
[`tests/tei/SAMPLES.md`](tests/tei/SAMPLES.md) for the full list with links.
If your checkout doesn't include them, download them once (about 84 MB,
SHA-256 verified); after that everything runs offline:

```sh
scripts/fetch-samples.sh
```

Without the samples, `cargo test` still runs the unit tests and the RELAX NG
conformance suite, and skips (and reports) the comparisons with Jing.

A new sample needs a row in `tests/tei/samples.tsv` (with its pinned source
and SHA-256) and its project a row in `tests/tei/licences.tsv`, which records
the licence and whether the files may be published (`public`) or only linked
to (`fetch-only`: non-commercial or no-derivatives licences without a
fair-use case, and anything with no licence stated). Schemas added to the
store get a row in `schemas/licences.tsv` the same way.

## Tests

```sh
cargo test                                  # unit tests, RELAX NG conformance suite, Jing and Schematron agreement
cargo test --features sync                  # also the schema-download code
cargo test --no-default-features            # without the schematron feature
cargo test --release -- --ignored           # Jing and Schematron agreement on the large benchmark samples
cargo clippy --all-targets --all-features   # must report no warnings
cargo fmt
```

The Jing agreement tests cover three tables in `tests/tei/`: the fixtures and
the benchmark samples against TEI P5 `tei_all` (`expected.tsv`,
`expected-bench.tsv`), and each document against its own project schema from
`schemas/` (`expected-declared.tsv`). A change to the validator must keep
every verdict the same; never edit these tables to make a test pass.

## Schematron

Schematron support lives in `src/schematron.rs` (extracting the rules from a
schema and running them) and `src/xpath/` (a focused XPath 2.0 evaluator over
`roxmltree`, with its own unit tests). An expression it can't handle must fail
to compile, so that the test is reported as unsupported; it must never be
treated as passing or failing. `tests/schematron_agreement.rs` also pins how
many tests of each bundled schema are supported.

The reference is TEI's own pipeline: the rules are extracted from the `.rng`
(`scripts/schematron/extract.xsl`), compiled to XSLT 2.0 with
[SchXslt](https://github.com/schxslt/schxslt) and run with Saxon-HE. Both
come from Maven Central; re-record `tests/tei/schematron-expected.tsv` after
adding samples or schemas:

```sh
mvn dependency:get -Dartifact=net.sf.saxon:Saxon-HE:12.5
mvn dependency:get -Dartifact=name.dmaus.schxslt:schxslt:1.10.1
m2=~/.m2/repository
export SAXON_CP=$m2/net/sf/saxon/Saxon-HE/12.5/Saxon-HE-12.5.jar:$m2/org/xmlresolver/xmlresolver/5.2.2/xmlresolver-5.2.2.jar:$m2/org/xmlresolver/xmlresolver/5.2.2/xmlresolver-5.2.2-data.jar
export SCHXSLT_JAR=$m2/name/dmaus/schxslt/schxslt/1.10.1/schxslt-1.10.1.jar
scripts/record-schematron-expectations.sh
```

## Benchmarks

```sh
cargo bench                              # every sample against tei_all, 5 runs each
TEI_BENCH_SCHEMAS=declared cargo bench   # each document against its own schema from schemas/
TEI_BENCH_RUNS=10 cargo bench -- sarit   # more runs, only paths containing "sarit"
TEI_BENCH_SCHEMATRON=1 cargo bench       # RELAX NG plus the embedded Schematron rules
```

The benchmarks and `scripts/bench-vs-jing.sh` check RELAX NG only, as Jing
does, unless asked otherwise.

Comparing with Jing, or re-recording its verdicts after adding samples, needs
Java and a `jing.jar`, which ships with the TEI sources:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/TEIC/TEI ../TEI
git -C ../TEI sparse-checkout set P5/Utilities/lib
export JING_JAR=$(cd ../TEI/P5/Utilities/lib && pwd)/jing.jar

scripts/bench-table.sh                  # time with hyperfine; updates BENCHMARKS.md and README.md
scripts/bench-vs-jing.sh 3              # end-to-end timings for every sample, best of 3
scripts/record-jing-expectations.sh     # rewrite tests/tei/expected*.tsv
```

[BENCHMARKS.md](BENCHMARKS.md) has the published results and how they were
measured.

## Keeping the schema store current

```sh
cargo run --release --features sync -- schemas check --schemas schemas   # anything outdated?
cargo run --release --features sync -- schemas sync  --schemas schemas   # update it
```
