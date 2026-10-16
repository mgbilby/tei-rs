//! Checks that tei-rs's Schematron messages agree with the reference ISO
//! Schematron pipeline (rules extracted from the .rng, compiled to XSLT 2.0
//! with SchXslt and run with Saxon-HE), as recorded by
//! `scripts/record-schematron-expectations.sh` in
//! `tests/tei/schematron-expected.tsv`.
//!
//! For every schema/document pair, the failed asserts and successful
//! reports must match exactly: same kind, role, rule, test and location
//! (SchXslt's `/Q{uri}local[n]` XPath). The only exceptions are messages from
//! tests tei-rs reports as unsupported, which are counted and skipped. Pairs
//! from `tests/tei/docs/` run with every `cargo test`; the large benchmark
//! samples in `tests/tei/bench/` with `cargo test --release -- --ignored`.
//! Pairs whose files are missing are skipped (and reported).

#![cfg(feature = "schematron")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use teixml::schematron::{Outcome, TestKind};
use teixml::{DiagnosticKind, Error};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// One message: kind, role, location, rule context, test.
type Row = (String, String, String, String, String);

#[derive(Default)]
struct Pair {
    not_well_formed: bool,
    rows: Vec<Row>,
}

fn expectations() -> BTreeMap<(String, String), Pair> {
    let path = repository().join("tests/tei/schematron-expected.tsv");
    let table = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut pairs: BTreeMap<(String, String), Pair> = BTreeMap::new();
    for line in table
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
    {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 7, "malformed line: {line}");
        let pair = pairs
            .entry((fields[0].to_owned(), fields[1].to_owned()))
            .or_default();
        match fields[2] {
            "none" => {}
            "not-well-formed" => pair.not_well_formed = true,
            "assert" | "report" => pair.rows.push((
                fields[2].to_owned(),
                fields[3].to_owned(),
                fields[4].to_owned(),
                fields[5].to_owned(),
                fields[6].to_owned(),
            )),
            other => panic!("unknown kind `{other}` in: {line}"),
        }
    }
    pairs
}

fn check_agreement(bench: bool) {
    let pairs = expectations();
    let mut schemas: BTreeMap<String, teixml::Schema> = BTreeMap::new();
    let mut disagreements = Vec::new();
    let (mut checked, mut skipped, mut messages, mut unsupported) = (0, 0, 0, 0);
    for ((schema_path, document), expected) in &pairs {
        if document.starts_with("bench/") != bench {
            continue;
        }
        let document_path = repository().join("tests/tei").join(document);
        if !repository().join(schema_path).is_file() || !document_path.is_file() {
            skipped += 1;
            continue;
        }
        checked += 1;
        let schema = schemas.entry(schema_path.clone()).or_insert_with(|| {
            teixml::Schema::from_file(repository().join(schema_path))
                .unwrap_or_else(|error| panic!("{schema_path}: {error}"))
        });
        let rules = schema.schematron().expect("Schematron rules compile");
        let skips = |row: &Row| {
            rules.unsupported().iter().any(|entry| {
                entry.context == row.3
                    && entry
                        .test
                        .as_ref()
                        .is_none_or(|(kind, test)| kind.to_string() == row.0 && *test == row.4)
            })
        };
        let diagnostics = match schema.validate_file(&document_path) {
            Err(Error::Xml(_)) if expected.not_well_formed => continue,
            Err(error) => {
                disagreements.push(format!("{document} against {schema_path}: {error}"));
                continue;
            }
            Ok(_) if expected.not_well_formed => {
                disagreements.push(format!("{document}: the reference couldn't parse it"));
                continue;
            }
            Ok(diagnostics) => diagnostics,
        };
        let mut actual: Vec<Row> = Vec::new();
        for diagnostic in diagnostics {
            let DiagnosticKind::Schematron(message) = diagnostic.kind else {
                continue;
            };
            if let Outcome::EvaluationError(error) = &message.outcome {
                disagreements.push(format!(
                    "{document} against {schema_path}: rule `{}` test `{}` failed to evaluate: {error}",
                    message.context, message.test
                ));
                continue;
            }
            actual.push((
                match message.kind {
                    TestKind::Assert => "assert".into(),
                    TestKind::Report => "report".into(),
                },
                message.role.clone().unwrap_or_else(|| "-".into()),
                message.location.clone(),
                message.context.clone(),
                message.test.clone(),
            ));
        }
        let mut wanted: Vec<Row> = Vec::new();
        for row in &expected.rows {
            if skips(row) {
                unsupported += 1;
            } else {
                wanted.push(row.clone());
            }
        }
        actual.sort();
        wanted.sort();
        messages += wanted.len();
        if actual != wanted {
            let missing: Vec<_> = wanted.iter().filter(|row| !actual.contains(row)).collect();
            let extra: Vec<_> = actual.iter().filter(|row| !wanted.contains(row)).collect();
            disagreements.push(format!(
                "{document} against {schema_path}:\n  missing (reference only): {missing:#?}\n  extra (tei-rs only): {extra:#?}"
            ));
        }
    }
    eprintln!(
        "Schematron agreement: {checked} pairs checked, {messages} reference messages matched, \
         {unsupported} skipped as unsupported, {skipped} pairs skipped for missing files"
    );
    if skipped > 0 {
        eprintln!("run scripts/fetch-samples.sh to include the missing sample documents");
    }
    assert!(
        disagreements.is_empty(),
        "{} pairs disagree with the reference Schematron implementation:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

#[test]
fn agrees_with_reference_schematron_on_fixtures() {
    check_agreement(false);
}

#[test]
#[ignore = "slow in debug builds; run with `cargo test --release -- --ignored`"]
fn agrees_with_reference_schematron_on_benchmark_samples() {
    check_agreement(true);
}

/// How many of the Schematron tests in the bundled schemas tei-rs checks.
/// A change that makes a test unsupported (or supports another) shows here.
#[test]
fn supports_the_rules_of_the_bundled_schemas() {
    let expected = [
        // (schema, tests, unsupported tests)
        ("tests/tei/schemas/tei_all-2.7.0.rng", 47, 2),
        ("schemas/epidoc.stoa.org/schema/8.16/tei-epidoc.rng", 36, 0),
        ("schemas/epidoc.stoa.org/schema/8.13/tei-epidoc.rng", 25, 0),
        (
            "schemas/raw.githubusercontent.com/COST-ELTeC/Schemas/master/eltec-1.rng",
            18,
            0,
        ),
        ("schemas/papyri.info/schemas/papyri.info.rng", 79, 0),
        (
            "schemas/raw.githubusercontent.com/clarin-eric/ParlaMint/main/Schema/ParlaMint-TEI.ana.rng",
            0,
            0,
        ),
    ];
    let mut found = Vec::new();
    // Schemas that aren't in this checkout (`tei-rs schemas sync` fetches
    // them) are left out of the comparison.
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|(path, _, _)| {
            let present = repository().join(path).is_file();
            if !present {
                eprintln!("skipped {path}: not in this checkout");
            }
            present
        })
        .collect();
    for &(path, _, _) in &expected {
        let schema = teixml::Schema::from_file(repository().join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let rules = schema.schematron().expect("Schematron rules compile");
        eprintln!(
            "{path}: {} tests, {} unsupported",
            rules.test_count(),
            rules.unsupported_test_count()
        );
        for entry in rules.unsupported() {
            eprintln!("  {entry}");
        }
        found.push((path, rules.test_count(), rules.unsupported_test_count()));
    }
    assert_eq!(found, expected);
}
