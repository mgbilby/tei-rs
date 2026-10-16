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
      <sch:assert test=". castable as xs:date">unsupported</sch:assert>
      <sch:report test="@when and @notBefore">both</sch:report>
    </sch:rule>
  </sch:pattern>
  <sch:pattern>
    <sch:rule context="tei:*[current-date()]">
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

fn compiled() -> Schematron {
    Schematron::from_relaxng(SCHEMA, "schema.rng", &|_, _| Err("no includes".into())).unwrap()
}

fn messages() -> Vec<(String, Message)> {
    let doc = roxmltree::Document::parse(DOC).unwrap();
    compiled()
        .check(&doc, None)
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
    assert!(reasons[0].contains("xs:date"), "{reasons:#?}");
    assert!(reasons[1].contains("current-date"), "{reasons:#?}");
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
    let messages = compiled().check(&doc, None);
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
    let schematron = Schematron::from_relaxng(schema, "s.rng", &|_, _| Err("none".into())).unwrap();
    assert_eq!(
        schematron.unsupported_test_count(),
        0,
        "falls back to in-scope namespaces"
    );
    let doc = roxmltree::Document::parse("<r><n>5</n><n>x</n><n>-1</n></r>").unwrap();
    let messages = schematron.check(&doc, None);
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
    let schematron = Schematron::from_relaxng(main, "main.rng", &|href, base| {
        assert_eq!((href, base), ("rules.rng", "main.rng"));
        Ok((included.to_owned(), "rules.rng".to_owned()))
    })
    .unwrap();
    assert_eq!(schematron.test_count(), 1);
    let doc = roxmltree::Document::parse("<a/>").unwrap();
    assert_eq!(schematron.check(&doc, None)[0].1.text, "hit");
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

#[test]
fn compact_syntax_rules_are_listed_as_unchecked() {
    let schematron = Schematron::from_compact(
        r#"namespace s = "http://www.ascc.net/xml/schematron"
start = element doc {
    empty
    >> s:pattern [ name = "p" s:rule [ context = "doc" s:assert [ test = "@a" "needs a" ] ] ]
    >> s:pattern [ s:rule [ context = "doc" s:report [ test = "@b" "has b" ] ] ]
}"#,
    );
    assert_eq!(schematron.test_count(), 2);
    assert_eq!(schematron.unsupported_test_count(), 2);
    assert!(Schematron::from_compact("start = element doc { empty }").is_empty());
}
