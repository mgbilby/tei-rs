//! Checks that tei-rs reaches the same verdict as Jing (the reference RELAX NG
//! validator), with its first error on the same line, for every
//! schema/document pair recorded by `scripts/record-jing-expectations.sh`:
//!
//! - `tests/tei/expected.tsv` covers the small fixtures in `tests/tei/docs/`
//!   and runs with every `cargo test`.
//! - `tests/tei/expected-bench.tsv` covers the large benchmark samples in
//!   `tests/tei/bench/`; it is slow in debug builds, so run it with
//!   `cargo test --release -- --ignored`.
//! - `tests/tei/expected-declared.tsv` pairs documents with the schema they
//!   declare, taken from the schema store in `schemas/` (schema paths are
//!   relative to the repository root).
//!
//! Some sample documents are not in the public repository; fetch them with
//! `scripts/fetch-samples.sh`. Pairs whose document or schema is missing are
//! skipped (and reported), so the tests pass without them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use teixml::Error;

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> PathBuf {
    repository().join("tests/tei")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verdict {
    Valid,
    Invalid,
    NotWellFormed,
}

struct Expectation {
    schema: String,
    document: String,
    verdict: Verdict,
    /// The line of Jing's first error, for invalid documents.
    first_error_line: Option<u32>,
}

fn expectations(table: &str) -> Vec<Expectation> {
    let table = std::fs::read_to_string(fixtures().join(table))
        .unwrap_or_else(|error| panic!("tests/tei/{table}: {error}"));
    table
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert!(fields.len() >= 3, "malformed expectation line: {line}");
            Expectation {
                schema: fields[0].to_owned(),
                document: fields[1].to_owned(),
                verdict: match fields[2] {
                    "valid" => Verdict::Valid,
                    "invalid" => Verdict::Invalid,
                    "not-well-formed" => Verdict::NotWellFormed,
                    other => panic!("unknown verdict `{other}` in: {line}"),
                },
                first_error_line: fields.get(4).and_then(|field| field.parse().ok()),
            }
        })
        .collect()
}

fn check_agreement(table: &str) {
    check_agreement_with_schemas_in(table, &fixtures());
}

/// Like [`check_agreement`], with schema paths resolved against `schema_base`.
fn check_agreement_with_schemas_in(table: &str, schema_base: &Path) {
    let expectations = expectations(table);
    assert!(!expectations.is_empty(), "no expectations in {table}");

    let mut schemas = BTreeMap::new();
    let mut disagreements = Vec::new();
    let mut skipped = 0;
    for expectation in &expectations {
        if !schema_base.join(&expectation.schema).is_file()
            || !fixtures().join(&expectation.document).is_file()
        {
            skipped += 1;
            continue;
        }
        let schema = schemas
            .entry(expectation.schema.clone())
            .or_insert_with(|| {
                #[cfg_attr(not(feature = "schematron"), allow(unused_mut))]
                let mut schema = teixml::Schema::from_file(schema_base.join(&expectation.schema))
                    .unwrap_or_else(|error| panic!("{}: {error}", expectation.schema));
                // Jing checks RELAX NG only; Schematron has its own
                // agreement tests (tests/schematron_agreement.rs).
                #[cfg(feature = "schematron")]
                schema.set_schematron(false);
                schema
            });
        let (verdict, detail) = match schema.validate_file(fixtures().join(&expectation.document)) {
            Ok(diagnostics) if diagnostics.is_empty() => (Verdict::Valid, String::new()),
            Ok(diagnostics) => {
                let line = diagnostics[0].position.map(|position| position.line);
                if expectation.verdict == Verdict::Invalid
                    && expectation.first_error_line.is_some()
                    && line != expectation.first_error_line
                {
                    disagreements.push(format!(
                        "{} against {}: Jing's first error is on line {}, tei-rs's: {}",
                        expectation.document,
                        expectation.schema,
                        expectation.first_error_line.unwrap_or_default(),
                        diagnostics[0]
                    ));
                }
                (Verdict::Invalid, diagnostics[0].to_string())
            }
            Err(Error::Xml(message)) => (Verdict::NotWellFormed, message),
            Err(error) => {
                disagreements.push(format!(
                    "{} against {}: could not validate: {error}",
                    expectation.document, expectation.schema
                ));
                continue;
            }
        };
        if verdict != expectation.verdict {
            disagreements.push(format!(
                "{} against {}: Jing says {:?}, tei-rs says {verdict:?} {detail}",
                expectation.document, expectation.schema, expectation.verdict,
            ));
        }
    }
    report_skipped(table, skipped, expectations.len());
    assert!(
        disagreements.is_empty(),
        "{} of {} documents disagree with Jing:\n{}",
        disagreements.len(),
        expectations.len(),
        disagreements.join("\n")
    );
}

fn report_skipped(table: &str, skipped: usize, total: usize) {
    if skipped > 0 {
        eprintln!(
            "{table}: skipped {skipped} of {total} pairs whose sample files are not present; \
             run scripts/fetch-samples.sh to include them"
        );
    }
}

#[test]
fn agrees_with_jing_on_fixtures() {
    check_agreement("expected.tsv");
}

#[test]
#[ignore = "slow in debug builds; run with `cargo test --release -- --ignored`"]
fn agrees_with_jing_on_benchmark_samples() {
    check_agreement("expected-bench.tsv");
}

#[test]
fn agrees_with_jing_against_each_documents_own_schema() {
    check_agreement_with_schemas_in("expected-declared.tsv", &repository());
}

#[test]
fn documents_resolve_to_their_declared_schema_in_the_store() {
    let store = teixml::SchemaStore::open(repository().join("schemas")).expect("store opens");
    for expectation in expectations("expected-declared.tsv") {
        let document = fixtures().join(&expectation.document);
        let Ok(xml) = std::fs::read_to_string(&document) else {
            continue; // sample not fetched
        };
        if teixml::store::declared_schemas(&xml).is_empty() {
            continue; // paired by hand in the recording script
        }
        if !repository().join(&expectation.schema).is_file() {
            continue; // schema not in this checkout; `tei-rs schemas sync` fetches it
        }
        let resolved = teixml::store::schema_for_document(&xml, &document, &store)
            .unwrap_or_else(|error| panic!("{}: {error}", expectation.document));
        assert_eq!(
            resolved,
            repository().join("schemas").join(
                expectation
                    .schema
                    .strip_prefix("schemas/")
                    .expect("store-relative schema path")
            ),
            "{}",
            expectation.document
        );
    }
}
