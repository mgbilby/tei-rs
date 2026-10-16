//! Unit tests for Schematron extraction, rule dispatch and messages.

use super::*;

const SCHEMA: &str = r##"<grammar xmlns="http://relaxng.org/ns/structure/1.0"
    xmlns:sch="http://purl.oclc.org/dsdl/schematron">
  <sch:ns prefix="tei" uri="http://www.tei-c.org/ns/1.0"/>
  <sch:let name="limit" value="2"/>
  <start><element name="TEI"><text/></element></start>
  <sch:pattern id="spans">
    <sch:rule context="tei:delSpan">
      <sch:assert test="@spanTo" diagnostics="d1">The <sch:name/> needs @spanTo</sch:assert>
    </sch:rule>
    <sch:rule context="tei:*[@spanTo]">
      <sch:assert test="id(substring(@spanTo, 2))">No target for <sch:value-of select="@spanTo"/></sch:assert>
    </sch:rule>
  </sch:pattern>
  <sch:pattern>
    <sch:rule context="tei:list">
      <sch:let name="items" value="count(tei:item)"/>
      <sch:report test="$items gt $limit" role="warning">More than <sch:value-of select="$limit"/> items: <sch:value-of select="$items"/></sch:report>
    </sch:rule>
  </sch:pattern>
  <sch:pattern>
    <sch:rule context="tei:ref/@target">
      <sch:report test="starts-with(., 'http:')" role="info">Insecure link on <sch:name path=".."/></sch:report>
    </sch:rule>
  </sch:pattern>
  <sch:pattern>
    <sch:rule context="tei:date">
      <sch:assert test=". castable as xs:QName">unsupported</sch:assert>
      <sch:report test="@when and @notBefore">both</sch:report>
    </sch:rule>
  </sch:pattern>
  <sch:pattern>
    <sch:rule context="tei:*[namespace::tei]">
      <sch:assert test="true()">never</sch:assert>
    </sch:rule>
    <sch:rule context="tei:p">
      <sch:assert test="true()">blocked by the rule before</sch:assert>
    </sch:rule>
  </sch:pattern>
  <sch:diagnostics>
    <sch:diagnostic id="d1">see the Guidelines</sch:diagnostic>
  </sch:diagnostics>
</grammar>"##;

const DOC: &str = r##"<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <delSpan/>
  <delSpan spanTo="#x"/>
  <addSpan spanTo="#a"/>
  <anchor xml:id="a"/>
  <list><item/><item/><item/></list>
  <list><item/></list>
  <ref target="http://example.org"/>
  <ref target="https://example.org"/>
  <date when="1900" notBefore="1899"/>
</TEI>"##;

fn xml(text: &str, uri: &str) -> SchemaSource {
    SchemaSource::new(text, uri, SchemaSyntax::Xml)
}

fn rnc(text: &str, uri: &str) -> SchemaSource {
    SchemaSource::new(text, uri, SchemaSyntax::Compact)
}

fn no_includes(_: &str, _: &str) -> Result<SchemaSource, String> {
    Err("no includes".into())
}

fn compiled() -> Schematron {
    Schematron::from_relaxng(&xml(SCHEMA, "schema.rng"), &no_includes).unwrap()
}

fn messages() -> Vec<(String, Message)> {
    let doc = roxmltree::Document::parse(DOC).unwrap();
    compiled()
        .check(&doc, None, &Access::default())
        .into_iter()
        .map(|(node, message)| (location(node), message))
        .collect()
}

#[test]
fn counts_supported_and_unsupported_tests() {
    let schematron = compiled();
    assert_eq!(schematron.test_count(), 8);
    assert_eq!(schematron.unsupported_test_count(), 3);
    let reasons: Vec<String> = schematron
        .unsupported()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(reasons.len(), 3, "{reasons:#?}");
    assert!(reasons[0].contains("xs:QName"), "{reasons:#?}");
    assert!(reasons[1].contains("namespace"), "{reasons:#?}");
    assert!(reasons[2].contains("follows rule"), "{reasons:#?}");
}

#[test]
fn reports_failed_asserts_and_successful_reports() {
    let messages = messages();
    let summary: Vec<String> = messages
        .iter()
        .map(|(_, message)| {
            format!(
                "{:?} {:?} {}",
                message.outcome, message.severity, message.text
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            "FailedAssert Error The delSpan needs @spanTo",
            // The second delSpan has @spanTo, and the first rule of the
            // pattern claims it, so the id() rule doesn't run on it.
            "SuccessfulReport Warning More than 2 items: 3",
            "SuccessfulReport Info Insecure link on ref",
            "SuccessfulReport Error both",
        ]
    );
    assert_eq!(messages[0].1.diagnostics, ["see the Guidelines"]);
    assert_eq!(messages[0].1.pattern.as_deref(), Some("spans"));
    assert_eq!(
        messages[0].0,
        "/Q{http://www.tei-c.org/ns/1.0}TEI[1]/Q{http://www.tei-c.org/ns/1.0}delSpan[1]"
    );
    assert_eq!(
        messages[2].0,
        "/Q{http://www.tei-c.org/ns/1.0}TEI[1]/Q{http://www.tei-c.org/ns/1.0}ref[1]/@Q{}target"
    );
    assert_eq!(messages[2].1.location, messages[2].0);
}

#[test]
fn second_rule_of_a_pattern_runs_when_the_first_does_not_match() {
    let doc = roxmltree::Document::parse(
        r##"<TEI xmlns="http://www.tei-c.org/ns/1.0"><addSpan spanTo="#nowhere"/></TEI>"##,
    )
    .unwrap();
    let messages = compiled().check(&doc, None, &Access::default());
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1.text, "No target for #nowhere");
    assert_eq!(messages[0].1.outcome, Outcome::FailedAssert);
}

#[test]
fn evaluation_errors_are_reported_not_hidden() {
    let schema = r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"
        xmlns:sch="http://purl.oclc.org/dsdl/schematron">
      <sch:pattern><sch:rule context="n">
        <sch:assert test="xs:integer(.) gt 0" xmlns:xs="http://www.w3.org/2001/XMLSchema">positive</sch:assert>
      </sch:rule></sch:pattern>
    </grammar>"#;
    let schematron = Schematron::from_relaxng(&xml(schema, "s.rng"), &no_includes).unwrap();
    assert_eq!(
        schematron.unsupported_test_count(),
        0,
        "falls back to in-scope namespaces"
    );
    let doc = roxmltree::Document::parse("<r><n>5</n><n>x</n><n>-1</n></r>").unwrap();
    let messages = schematron.check(&doc, None, &Access::default());
    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0].1.outcome, Outcome::EvaluationError(_)));
    assert_eq!(messages[0].1.severity, Severity::Warning);
    assert_eq!(messages[1].1.outcome, Outcome::FailedAssert);
}

#[test]
fn follows_includes() {
    let main = r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0">
      <include href="rules.rng"/></grammar>"#;
    let included = r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"
        xmlns:sch="http://purl.oclc.org/dsdl/schematron">
      <sch:pattern><sch:rule context="a"><sch:report test="true()">hit</sch:report></sch:rule></sch:pattern>
    </grammar>"#;
    let schematron = Schematron::from_relaxng(&xml(main, "main.rng"), &|href, base| {
        assert_eq!((href, base), ("rules.rng", "main.rng"));
        Ok(xml(included, "rules.rng"))
    })
    .unwrap();
    assert_eq!(schematron.test_count(), 1);
    let doc = roxmltree::Document::parse("<a/>").unwrap();
    assert_eq!(
        schematron.check(&doc, None, &Access::default())[0].1.text,
        "hit"
    );
}

#[test]
fn severity_follows_role() {
    assert_eq!(severity(None), Severity::Error);
    assert_eq!(severity(Some("ERROR")), Severity::Error);
    assert_eq!(severity(Some("fatal")), Severity::Error);
    assert_eq!(severity(Some("WARN")), Severity::Warning);
    assert_eq!(severity(Some("nonfatal")), Severity::Warning);
    assert_eq!(severity(Some("info")), Severity::Info);
}

/// The rules' messages on `doc`, as `kind: text`.
fn compact_messages(schema: &str, doc: &str) -> Vec<String> {
    let schematron = Schematron::from_relaxng(&rnc(schema, "s.rnc"), &no_includes).unwrap();
    assert!(schematron.unsupported().is_empty());
    let doc = roxmltree::Document::parse(doc).unwrap();
    schematron
        .check(&doc, None, &Access::default())
        .into_iter()
        .map(|(_, message)| format!("{}: {}", message.kind, message.text))
        .collect()
}

#[test]
fn reads_follow_annotations_in_the_compact_syntax() {
    let schema = r#"namespace s = "http://purl.oclc.org/dsdl/schematron"
start = element doc {
    attribute a { text }?
    >> s:pattern [ id = "p" s:rule [ context = "doc" s:assert [ test = "@a" "needs " ~ 'a' ] ] ]
    >> s:pattern [
         s:rule [
           context = "doc"
           s:report [ test = "true()" role = "info" "on " s:name [ ] ", " s:value-of [ select = "count(*)" ] ]
         ]
       ]
}"#;
    assert_eq!(
        compact_messages(schema, "<doc/>"),
        ["assert: needs a", "report: on doc, 0"]
    );
}

#[test]
fn reads_leading_and_grammar_level_annotations() {
    // A datatype name followed by the next definition's leading annotation
    // (whose attributes belong to the definition), and annotation elements
    // between definitions and inside `div`.
    let schema = r#"
namespace sch = "http://purl.oclc.org/dsdl/schematron"
datatypes d = "http://www.w3.org/2001/XMLSchema-datatypes"
start = element doc { [ sch:pattern [ sch:rule [ context = "doc" sch:report [ test = "true()" "1" ] ] ] ] empty }
code = d:token
[ sch:x = "y" sch:pattern [ sch:rule [ context = "doc" sch:report [ test = "true()" "2" ] ] ] ]
other = d:token
sch:pattern [ sch:rule [ context = "doc" sch:report [ test = "true()" "3" ] ] ]
div {
  sch:pattern [ sch:rule [ context = "doc" sch:report [ test = "true()" "4" ] ] ]
  more = element more { d:string >> sch:pattern [ sch:rule [ context = "doc" sch:report [ test = "true()" "5" ] ] ] }
}
"#;
    assert_eq!(
        compact_messages(schema, "<doc/>"),
        [
            "report: 1",
            "report: 2",
            "report: 3",
            "report: 4",
            "report: 5"
        ]
    );
}

#[test]
fn reads_compact_syntax_literals_and_names() {
    let xml = compact::annotations(
        r#"# a comment with [ brackets ] and "quotes
## documentation, not an annotation
default namespace s = "urn:s"
namespace x = "urn:\x{78}"
\element = element doc { empty }
x:a [ t = """say "hi" """ ~ '''it's''' n = "1\x{A}2\x{22}"
  "line\x{a}" ~ """two
lines""" x:b [ x:c [ ] ] ]
"#,
    )
    .unwrap();
    assert_eq!(
        xml,
        "<annotations xmlns:s=\"urn:s\" xmlns:x=\"urn:x\"><x:a t=\"say &quot;hi&quot; it's\" \
         n=\"1&#xA;2&quot;\">line\ntwo\nlines<x:b><x:c></x:c></x:b></x:a></annotations>"
    );
}

#[test]
fn rejects_malformed_compact_annotations() {
    for (schema, error) in [
        (
            "start = empty >> y:a [ ]",
            "undeclared namespace prefix `y`",
        ),
        (
            "namespace y = \"u\"\nstart = empty >> y:a [ \"text\"",
            "unterminated annotation",
        ),
        ("start = empty >> \"a\"", "expected a name in annotation"),
        ("x = \"a\nb\"", "newline in a string literal"),
        ("x = \"\\x{110000}\"", "invalid escape"),
    ] {
        let found = compact::annotations(schema).unwrap_err();
        assert!(found.contains(error), "{schema:?}: {found}");
    }
    // Deep nesting is rejected without recursing.
    let deep = format!(
        "namespace y = \"u\"\nstart = empty >> {}{}",
        "y:a [ ".repeat(100_000),
        "] ".repeat(100_000)
    );
    let error = Schematron::from_relaxng(&rnc(&deep, "deep.rnc"), &no_includes).unwrap_err();
    assert!(error.contains("nested too deeply"), "{error}");
}

#[test]
fn follows_compact_includes() {
    let main = r#"namespace sch = "http://purl.oclc.org/dsdl/schematron"
sch:ns [ prefix = "t" uri = "urn:t" ]
include "rules.rnc" inherit = sch {
  start = element t:doc { empty }
  sch:pattern [ sch:rule [ context = "t:doc" sch:report [ test = "true()" "override" ] ] ]
}
other = external "external.rnc""#;
    let schematron = Schematron::from_relaxng(&rnc(main, "main.rnc"), &|href, base| {
        assert_eq!(base, "main.rnc");
        let rules = format!(
            "namespace s = \"http://purl.oclc.org/dsdl/schematron\"\n\
             start = notAllowed\n\
             s:pattern [ s:rule [ context = \"t:doc\" s:report [ test = \"true()\" \"{href}\" ] ] ]"
        );
        Ok(rnc(&rules, href))
    })
    .unwrap();
    let doc = roxmltree::Document::parse(r#"<doc xmlns="urn:t"/>"#).unwrap();
    let texts: Vec<String> = schematron
        .check(&doc, None, &Access::default())
        .into_iter()
        .map(|(_, message)| message.text)
        .collect();
    assert_eq!(texts, ["rules.rnc", "override", "external.rnc"]);
}

#[test]
fn reads_schematron_1_5_as_iso() {
    let schema = r#"namespace s = "http://www.ascc.net/xml/schematron"
start = element doc {
    empty
    >> s:pattern [ name = "p" s:rule [ context = "doc" s:assert [ test = "@a" "needs a" ] ] ]
    >> s:pattern [ s:rule [ context = "doc" s:report [ test = "not(@b)" "lacks " s:name [ ] ] ] ]
}"#;
    assert_eq!(
        compact_messages(schema, "<doc/>"),
        ["assert: needs a", "report: lacks doc"]
    );
}
