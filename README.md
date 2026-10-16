# teixml

[![crates.io](https://img.shields.io/crates/v/teixml.svg)](https://crates.io/crates/teixml)
[![Documentation](https://docs.rs/teixml/badge.svg)](https://docs.rs/teixml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](#rust-version-support)

Validate [TEI](https://tei-c.org/) XML documents against RELAX NG schemas and
the Schematron rules they embed, in pure Rust — a library (`teixml`) and a
command-line tool (`tei-rs`), with no Java or libxml2 needed.

- Gives the same verdicts as [Jing](https://relaxng.org/jclark/jing.html), the
  reference RELAX NG validator, on every bundled test document (241 documents
  from 50 TEI projects and the TEI Consortium's test suite), with the first
  error on the same line.
- Checks IDs as Jing does (RELAX NG DTD Compatibility): every `xml:id` or
  other `ID`-typed attribute unique, every `IDREF` resolving.
<!-- bench:readme -->
- 4–24× faster than Jing end-to-end on every sample (7× at the
  median), and 17–36× faster than Jing with SchXslt when checking
  Schematron too; see [BENCHMARKS.md](BENCHMARKS.md).
<!-- /bench:readme -->
- Reads both the XML (`.rng`) and compact (`.rnc`) syntax, including
  `include`/`externalRef`, which are resolved as local files only: validation
  never touches the network.
- Checks the ISO Schematron rules TEI ODD-generated schemas embed, with its
  own XPath 2.0 evaluator, and reports the same messages as the reference
  pipeline (SchXslt on Saxon-HE) on every bundled test document. 203 of the
  205 tests in the bundled TEI, EpiDoc, ELTeC and papyri.info schemas are
  supported; the rest are listed and skipped, never guessed.
- Validates each document against the schema it declares in
  `<?xml-model?>`, from a local schema store.

**Status: early development.** Schematron tests that use XPath teixml doesn't
implement (dates and durations, `doc()`, XPath 3 syntax, …), and Schematron
embedded in compact-syntax (`.rnc`) schemas, are reported as unsupported and
skipped.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
teixml = "0.1"
```

Then:

```rust
use teixml::Schema;
use teixml::relaxng::SchemaSyntax;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::from_text(
        r#"default namespace = "http://www.tei-c.org/ns/1.0"
           start = element TEI { element text { element p { text }+ } }"#,
        "inline.rnc",
        SchemaSyntax::Compact,
    )?;

    let valid = r#"<TEI xmlns="http://www.tei-c.org/ns/1.0"><text><p>Hello</p></text></TEI>"#;
    assert!(schema.validate_str(valid)?.is_empty());

    let invalid = r#"<TEI xmlns="http://www.tei-c.org/ns/1.0"><text><div/></text></TEI>"#;
    for diagnostic in schema.validate_str(invalid)? {
        println!("{diagnostic}");
    }
    Ok(())
}
```

```text
1:48: unexpected element `div` (in /TEI/text)
1:61: missing required content (in /TEI/text)
```

`Schema::from_file` and `Schema::validate_file` do the same for files on disk.
Each `Diagnostic` says where it comes from (`DiagnosticKind::RelaxNg` or
`DiagnosticKind::Schematron`, with the rule's message, role, test and
location) and how serious it is (`severity()`); a document is valid when
none is an error. `Schema::set_schematron(false)` turns Schematron checking
off, and `Schema::schematron()` lists the tests that are unsupported. See the
[documentation](https://docs.rs/teixml) for the full API, including the
schema store (`teixml::store`) and the RELAX NG engine (`teixml::relaxng`).

## Command-line tool

```sh
cargo install teixml                    # validation only
cargo install teixml --features sync    # also download and update schemas
```

Either installs the `tei-rs` command:

```sh
tei-rs validate --schema tei_all.rng edition.xml
```

```text
edition.xml:15:13: error: unexpected element `placeName` (in /TEI/teiHeader/fileDesc/publicationStmt)
edition.xml:93:9: error: Schematron assert failed: The @spanTo attribute of delSpan is required. (in /TEI/text/body/p/delSpan)
edition.xml:353:56: warning: Schematron assert failed: The @new attribute should always be supplied; use the special value "normal" to indicate that the feature concerned ceases to be remarkable at this point. [role: warning] (in /TEI/text/body/div/u/shift)
```

RELAX NG errors come first, then the messages of the schema's Schematron
rules (failed asserts and successful reports). A Schematron message is an
error, warning or info according to its rule's `role` (`warning`, `warn` and
TEI's `nonfatal` are warnings; `info` is info; anything else, or no role, is
an error). Schematron tests that use XPath tei-rs doesn't support are listed
once per schema on stderr and skipped. `--no-schematron` checks RELAX NG only.

The exit status is 0 when every document is valid (warnings and info don't
count), 1 when any is invalid, and 2 when a schema or document can't be read
or parsed. When several documents share a schema, it is compiled once per run.

Without `--schema`, each document is validated against the schema it declares
in `<?xml-model href="…"?>`, taken from a local schema store:

```sh
tei-rs schemas add https://epidoc.stoa.org/schema/latest/tei-epidoc.rng   # download once
tei-rs validate inscriptions/*.xml                                         # then offline
tei-rs schemas check     # are the stored schemas still the latest published versions?
tei-rs schemas sync      # update the ones that aren't
```

`check` compares every stored schema, and every file it includes, with what
its URL serves now; `sync` downloads whatever changed. `schemas list` shows the
store and `schemas which doc.xml` prints the schema a document resolves to;
`--alias` and `--mirror` on `add` record other hrefs for the same schema and
fallback download locations. The store defaults to `$TEI_RS_SCHEMAS`, else a
`tei-rs` folder in your user cache directory; `--schemas DIR` picks another.
Run `tei-rs --help` for the full reference.

## Cargo features

- `schematron` (on by default): checking the ISO Schematron rules embedded in
  schemas — the `teixml::schematron` module, `Schema::set_schematron` and the
  CLI's Schematron messages. It adds no dependencies. Rules are compiled on
  first use, so RELAX NG-only validation (`--no-schematron`) costs nothing
  extra. `default-features = false` builds a RELAX NG-only validator.
- `sync` (off by default): downloading and updating schemas in a schema store
  — the `teixml::sync` module and `tei-rs schemas add/check/sync`. Downloads
  use HTTPS only (redirects included), honour `HTTPS_PROXY` and use the
  operating system's certificates. Without
  it, the crate contains no network code; validation, `schemas list` and
  `schemas which` work in every build.

```toml
[dependencies]
teixml = { version = "0.1", features = ["sync"] }
```

## Rust version support

The minimum supported Rust version is **1.88**. It may be raised in a minor
release; such changes are noted in the release notes.

## Contributing

Bug reports and pull requests are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md) for how the repository is laid out and how
to run the tests, the comparisons with Jing and the benchmarks.

## Security

Documents are treated as untrusted input; schemas as trusted. See
[SECURITY.md](SECURITY.md) for what that means and how to report a
vulnerability.

## License

Licensed under the [MIT license](LICENSE).

The RELAX NG engine in `src/relaxng/` is derived from CASOON's MIT-licensed
[`relax-ng`](https://crates.io/crates/relax-ng) and keeps its copyright notice
in [`src/relaxng/LICENSE`](src/relaxng/LICENSE); see
[`src/relaxng/PROVENANCE.md`](src/relaxng/PROVENANCE.md). Test data and
schemas in this repository keep their own licences, listed in
[`tests/tei/SOURCES.md`](tests/tei/SOURCES.md) and
[`schemas/README.md`](schemas/README.md).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in teixml by you shall be licensed as MIT, without any
additional terms or conditions.
