//! ID, IDREF and IDREFS checking (RELAX NG DTD Compatibility, section 4),
//! with Jing's results and positions.

use teixml::relaxng::{SchemaSyntax, ValidationErrorKind};
use teixml::{DiagnosticKind, Error, Schema};

const SCHEMA: &str = r#"datatypes xsd = "http://www.w3.org/2001/XMLSchema-datatypes"
datatypes c = "http://relaxng.org/ns/compatibility/datatypes/1.0"
start = element doc {
  element p {
    attribute id { xsd:ID }?, attribute ref { xsd:IDREF }?,
    attribute refs { c:IDREFS }?, attribute n { xsd:integer }?, text
  }*
}"#;

fn errors(xml: &str) -> Vec<(String, String)> {
    let schema = Schema::from_text(SCHEMA, "ids.rnc", SchemaSyntax::Compact).unwrap();
    schema
        .validate_str(xml)
        .unwrap()
        .iter()
        .map(|diagnostic| {
            let position = diagnostic.position.unwrap();
            (
                format!("{}:{}", position.line, position.column),
                diagnostic.message().to_string(),
            )
        })
        .collect()
}

#[test]
fn unique_ids_and_resolved_references_are_valid() {
    assert!(errors(r#"<doc><p id="a" ref="b"/><p id="b" refs=" a  b "/></doc>"#).is_empty());
}

#[test]
fn duplicate_ids_are_reported_with_the_first_occurrence() {
    let xml = "<doc>\n<p id=\"a\">x</p>\n<p\n  id=\"a\"\n  n=\"z\">y</p>\n</doc>";
    assert_eq!(
        errors(xml),
        [
            ("5:9".into(), "invalid value `z` for attribute `n`".into()),
            ("5:9".into(), "ID `a` has already been defined".into()),
            ("2:11".into(), "first occurrence of ID `a`".into()),
        ]
    );
}

#[test]
fn unresolved_references_are_reported_last() {
    let xml = "<doc>\n<p id=\"a\" refs=\"a b\"/>\n<p n=\"x\"/>\n</doc>";
    assert_eq!(
        errors(xml),
        [
            ("3:11".into(), "invalid value `x` for attribute `n`".into()),
            ("2:23".into(), "IDREF `b` without matching ID".into()),
        ]
    );
}

#[test]
fn id_errors_make_a_document_invalid() {
    let schema = Schema::from_text(SCHEMA, "ids.rnc", SchemaSyntax::Compact).unwrap();
    let diagnostics = schema
        .validate_str(r#"<doc><p id="a"/><p id="a"/></doc>"#)
        .unwrap();
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.is_error()));
    assert!(matches!(
        &diagnostics[0].kind,
        DiagnosticKind::RelaxNg(ValidationErrorKind::DuplicateId(id)) if id == "a"
    ));
}

#[test]
fn schemas_must_give_id_types_consistently() {
    let cases = [
        (
            r#"start = element doc { (element p { attribute id { xsd:ID } } | element p { attribute id { text } })* }"#,
            "conflicting ID-types",
        ),
        (
            r#"start = element doc { element * { attribute id { xsd:ID }? }* }"#,
            "name class that contains only",
        ),
        (
            r#"start = element doc { element p { attribute id { xsd:ID | xsd:integer }? }* }"#,
            "must occur as the child of an \"attribute\" pattern",
        ),
    ];
    for (body, expected) in cases {
        let schema =
            format!("datatypes xsd = \"http://www.w3.org/2001/XMLSchema-datatypes\"\n{body}");
        match Schema::from_text(&schema, "s.rnc", SchemaSyntax::Compact) {
            Err(Error::Schema(message)) => assert!(message.contains(expected), "{message}"),
            Err(other) => panic!("{body}: {other}"),
            Ok(_) => panic!("{body}: compiled"),
        }
    }
}
