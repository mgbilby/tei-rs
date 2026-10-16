//! Unit tests for the XPath evaluator, against expectations from the
//! XPath 2.0 and XSLT 2.0 specifications.

use std::collections::HashMap;

use super::datetime::DateKind;
use super::value::{Atomic, AtomicType};
use super::*;

const TEI: &str = "http://www.tei-c.org/ns/1.0";

const DOC: &str = r##"<TEI xmlns="http://www.tei-c.org/ns/1.0" xml:lang="en">
  <text>
    <body>
      <p n="1">Alpha <hi rend="b">beta</hi> gamma</p>
      <p n="2" xml:id="p2">delta</p>
      <lg><l n="1">one</l><l n="2">two</l><l n="10">ten</l></lg>
      <addSpan spanTo="#end"/>
      <anchor xml:id="end"/>
      <!-- a comment -->
      <idno type="filename">12345a</idno>
    </body>
  </text>
</TEI>"##;

fn context() -> StaticContext {
    StaticContext {
        namespaces: HashMap::from([
            ("tei".to_owned(), TEI.to_owned()),
            (
                "xs".to_owned(),
                "http://www.w3.org/2001/XMLSchema".to_owned(),
            ),
        ]),
        ..StaticContext::default()
    }
}

/// Evaluates `expr` with the root element as context item; returns the
/// string values joined with `|`, or the error code.
fn eval_at(xml: &str, path: &str, expr: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let document = Document::new(&doc, Some("file:/data/DDbDP/1/1.xml".into()));
    let compiled = parse_expr(expr, &context()).unwrap_or_else(|error| panic!("{expr}: {error}"));
    let mut evaluator = Evaluator::new(&document);
    let start = if path.is_empty() {
        document.root()
    } else {
        let selector = parse_expr(path, &context()).unwrap();
        let found = evaluator
            .nodes(&selector, Some(&Focus::node(document.root())))
            .unwrap();
        found[0]
    };
    evaluator.current = Some(start);
    match evaluator.eval(&compiled, Some(&Focus::node(start))) {
        Ok(items) => items.iter().map(Item::string).collect::<Vec<_>>().join("|"),
        Err(error) => format!("error {}", error.code),
    }
}

fn eval(expr: &str) -> String {
    eval_at(DOC, "/tei:TEI", expr)
}

#[test]
fn paths_and_axes() {
    assert_eq!(eval("count(//tei:p)"), "2");
    assert_eq!(eval("tei:text/tei:body/tei:p[2]/@n"), "2");
    assert_eq!(eval("//tei:l[last()]"), "ten");
    assert_eq!(eval("(//tei:l)[1]"), "one");
    assert_eq!(eval("//tei:lg/tei:l[1]"), "one");
    assert_eq!(eval("//tei:l[@n = 2]/preceding-sibling::*"), "one");
    assert_eq!(eval("//tei:l[1]/following-sibling::tei:l[1]"), "two");
    assert_eq!(eval("//tei:hi/ancestor::*[1]/@n"), "1");
    assert_eq!(eval("count(//tei:hi/ancestor-or-self::*)"), "5");
    assert_eq!(eval("//tei:hi/../@n"), "1");
    assert_eq!(eval("name(//tei:addSpan/following::*[1])"), "anchor");
    assert_eq!(eval("count(//tei:anchor/preceding::tei:l)"), "3");
    assert_eq!(eval("count(//tei:anchor/preceding::*)"), "8");
    assert_eq!(eval("//tei:p[1]/@n/following::tei:hi"), "beta");
    assert_eq!(eval("count(//comment())"), "1");
    assert_eq!(eval("count(//tei:p[1]/text())"), "2");
    assert_eq!(eval("count(//tei:p/node())"), "4");
    assert_eq!(eval("count(//*:l)"), "3");
    assert_eq!(eval("count(//tei:*[@n])"), "5");
    assert_eq!(
        eval("count(//l)"),
        "0",
        "unprefixed names are in no namespace"
    );
    assert_eq!(eval("count(//tei:p | //tei:l | //tei:p)"), "5");
    assert_eq!(eval("count(//tei:* except //tei:l)"), "10");
    assert_eq!(eval("count(//tei:l intersect //*[@n = '1'])"), "1");
    assert_eq!(eval("//tei:l[. = 'two']/@n"), "2");
    assert_eq!(eval("string(/tei:TEI/@xml:lang)"), "en");
    assert_eq!(eval("//tei:l/string(@n)"), "1|2|10");
    assert_eq!(eval("count(/)"), "1");
}

#[test]
fn comparisons_and_arithmetic() {
    assert_eq!(eval("//tei:l/@n = 10"), "true");
    assert_eq!(eval("//tei:l/@n = '10'"), "true");
    assert_eq!(eval("//tei:l/@n > 9"), "true");
    assert_eq!(
        eval("//tei:l/@n > '9'"),
        "false",
        "untyped vs string compares as strings"
    );
    assert_eq!(eval("//tei:l[3]/@n gt '9'"), "false");
    assert_eq!(eval("//tei:l[3]/@n eq 10"), "error XPTY0004");
    assert_eq!(eval("() = 1"), "false");
    assert_eq!(eval("() eq 1"), "");
    assert_eq!(eval("(1, 2) != (1, 2)"), "true");
    assert_eq!(eval("1 + 2 * 3"), "7");
    assert_eq!(eval("7 div 2"), "3.5");
    assert_eq!(eval("7 idiv 2"), "3");
    assert_eq!(eval("-7 mod 2"), "-1");
    assert_eq!(eval("1 div 0"), "error FOAR0001");
    assert_eq!(eval("1e0 div 0"), "INF");
    assert_eq!(eval("//tei:l[2]/@n * 2"), "4");
    assert_eq!(eval("60 * 1000000.0"), "60000000");
    assert_eq!(eval("1e7"), "1.0E7");
    assert_eq!(eval("0.1 + 0.2 = 0.3"), "false");
    assert_eq!(eval("//tei:l[1] << //tei:l[2]"), "true");
    assert_eq!(eval("//tei:l[1] >> //tei:l[2]"), "false");
    assert_eq!(eval("//tei:l[1] is (//tei:l)[1]"), "true");
    assert_eq!(eval("1 to 3"), "1|2|3");
    assert_eq!(eval("'abc' = 1"), "error XPTY0004");
    assert_eq!(eval("//tei:p[1]/@n = 'x' or 1"), "true");
}

#[test]
fn control_flow_and_types() {
    assert_eq!(eval("if (//tei:lg) then 'yes' else 'no'"), "yes");
    assert_eq!(eval("for $l in //tei:l return string-length($l)"), "3|3|3");
    assert_eq!(
        eval("for $a in (1, 2), $b in (10, 20) return $a * $b"),
        "10|20|20|40"
    );
    assert_eq!(eval("some $l in //tei:l satisfies $l/@n = 10"), "true");
    assert_eq!(eval("every $l in //tei:l satisfies $l/@n < 5"), "false");
    assert_eq!(eval("'12' cast as xs:integer + 1"), "13");
    assert_eq!(eval("xs:integer('x')"), "error FORG0001");
    assert_eq!(eval("'x' castable as xs:integer"), "false");
    assert_eq!(eval("'a1' castable as xs:NCName"), "true");
    assert_eq!(eval("'1a' castable as xs:NCName"), "false");
    assert_eq!(eval("xs:float('1.5') eq 1.5"), "true");
    assert_eq!(eval("1 instance of xs:integer"), "true");
    assert_eq!(eval("(1, 'a') instance of xs:integer*"), "false");
    assert_eq!(eval("//tei:l instance of element()+"), "true");
    assert_eq!(eval("() instance of empty-sequence()"), "true");
    assert_eq!(eval("boolean(('a', 'b'))"), "error FORG0006");
    assert_eq!(eval("(: comment :) 1"), "1");
}

#[test]
fn functions() {
    assert_eq!(eval("normalize-space(//tei:p[1])"), "Alpha beta gamma");
    assert_eq!(eval("string-length(//tei:p[1])"), "16");
    assert_eq!(eval("substring('12345', 2, 3)"), "234");
    assert_eq!(eval("substring('12345', 1.5, 2.6)"), "234");
    assert_eq!(eval("substring('12345', 0, 3)"), "12");
    assert_eq!(eval("substring-before('a,b', ',')"), "a");
    assert_eq!(eval("substring-after('a,b', ',')"), "b");
    assert_eq!(eval("concat('a', 1, ())"), "a1");
    assert_eq!(eval("string-join(//tei:l, '-')"), "one-two-ten");
    assert_eq!(eval("translate('abc', 'abc', 'AB')"), "AB");
    assert_eq!(eval("upper-case('ß')"), "SS");
    assert_eq!(eval("contains(//tei:addSpan/@spanTo, '#')"), "true");
    assert_eq!(eval("starts-with('#end', '#')"), "true");
    assert_eq!(eval("ends-with('a/', '/')"), "true");
    assert_eq!(eval("matches('ab12', '^[a-z]+[0-9]+$')"), "true");
    assert_eq!(eval("matches('AB', 'ab', 'i')"), "true");
    assert_eq!(eval("replace('a;b;;c', ';+', '/')"), "a/b/c");
    assert_eq!(eval("replace('abc', '(b)', '[$1]')"), "a[b]c");
    assert_eq!(eval("replace('abc', 'x*', '-')"), "error FORX0003");
    assert_eq!(eval("tokenize(' a  b ', '\\s+')"), "|a|b|");
    assert_eq!(eval("count(tokenize('', ','))"), "0");
    assert_eq!(eval("tokenize('1,2 3,4', ' ')[last()]"), "3,4");
    assert_eq!(eval("floor(12345 div 1000)"), "12");
    assert_eq!(
        eval("ceiling(xs:integer(replace('12345a', '\\D', '')) div 1000)"),
        "13"
    );
    assert_eq!(eval("round(2.5)"), "3");
    assert_eq!(eval("round(-2.5)"), "-2");
    assert_eq!(eval("abs(-3)"), "3");
    assert_eq!(eval("sum(//tei:l/@n)"), "13");
    assert_eq!(eval("avg((1, 2))"), "1.5");
    assert_eq!(eval("max(//tei:l/@n)"), "10");
    assert_eq!(eval("min(('b', 'a'))"), "a");
    assert_eq!(eval("count(distinct-values(('a', 'b', 'a')))"), "2");
    assert_eq!(eval("index-of(('a', 'b', 'a'), 'a')"), "1|3");
    assert_eq!(eval("reverse((1, 2))"), "2|1");
    assert_eq!(eval("subsequence((1, 2, 3), 2)"), "2|3");
    assert_eq!(eval("exists(()) or empty(())"), "true");
    assert_eq!(eval("number('x') != number('x')"), "true");
    assert_eq!(eval("number(' 12 ')"), "12");
    assert_eq!(eval("name(//tei:p[2]/@xml:id)"), "xml:id");
    assert_eq!(eval("local-name(//tei:hi)"), "hi");
    assert_eq!(eval("namespace-uri(//tei:hi)"), TEI);
    assert_eq!(eval("id('p2')/@n"), "2");
    assert_eq!(
        eval("id(substring(//tei:addSpan/@spanTo, 2)) >> //tei:addSpan"),
        "true"
    );
    assert_eq!(eval("count(id('p2 end nowhere'))"), "2");
    assert_eq!(
        eval("generate-id(//tei:l[1]) = generate-id((//tei:l)[1])"),
        "true"
    );
    assert_eq!(
        eval("generate-id(//tei:l[1]) = generate-id(//tei:l[2])"),
        "false"
    );
    assert_eq!(eval("lang('EN')"), "true");
    assert_eq!(eval("document-uri(root(.))"), "file:/data/DDbDP/1/1.xml");
    assert_eq!(eval("document-uri(.)"), "");
    assert_eq!(
        eval("codepoints-to-string(string-to-codepoints('ab'))"),
        "ab"
    );
    assert_eq!(eval("compare('a', 'b')"), "-1");
    assert_eq!(eval("exactly-one(())"), "error FORG0003");
    assert_eq!(eval("doc-available('/nonexistent/x.xml')"), "false");
    assert_eq!(eval("doc-available('file:/nonexistent/x.xml')"), "false");
    assert_eq!(eval("doc-available('https://example.org/x.xml')"), "false");
    // Relative, with no static base URI to resolve it against.
    assert_eq!(eval("doc-available('x.xml')"), "error FODC0002");
}

#[test]
fn current_refers_to_the_starting_node() {
    assert_eq!(
        eval_at(DOC, "//tei:l[2]", "count(//tei:l[@n = current()/@n])"),
        "1"
    );
    assert_eq!(
        eval_at(
            DOC,
            "//tei:addSpan",
            "following::*[@xml:id = substring(current()/@spanTo, 2)]/name()"
        ),
        "anchor"
    );
}

/// Evaluates `expr` with the root element as context item and the current
/// dateTime fixed at `now`.
fn eval_at_time(now: &str, expr: &str) -> String {
    let doc = roxmltree::Document::parse(DOC).unwrap();
    let document = Document::new(&doc, None);
    let Some(Atomic::DateTime(now)) = Atomic::Str(now.into())
        .cast(AtomicType::DateTime(DateKind::DateTime))
        .ok()
    else {
        panic!("{now} is not a dateTime");
    };
    document.now.set(now).unwrap();
    let compiled = parse_expr(expr, &context()).unwrap_or_else(|error| panic!("{expr}: {error}"));
    let mut evaluator = Evaluator::new(&document);
    let start = document.root();
    match evaluator.eval(&compiled, Some(&Focus::node(start))) {
        Ok(items) => items.iter().map(Item::string).collect::<Vec<_>>().join("|"),
        Err(error) => format!("error {}", error.code),
    }
}

#[test]
fn date_and_time_lexical_forms() {
    assert_eq!(eval("xs:date('2020-01-01')"), "2020-01-01");
    assert_eq!(eval("xs:date(' 2020-01-01 ')"), "2020-01-01");
    assert_eq!(eval("xs:date('2020-01-01-00:00')"), "2020-01-01Z");
    assert_eq!(eval("xs:date('2020-01-01+14:00')"), "2020-01-01+14:00");
    assert_eq!(eval("xs:date('2020-01-01+15:00')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-01-01+00:60')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-01-01+1:00')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-1-01')"), "error FORG0001");
    assert_eq!(eval("xs:date('+2020-01-01')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-01-01T00:00:00')"), "error FORG0001");
    // Leap years.
    assert_eq!(eval("xs:date('2000-02-29')"), "2000-02-29");
    assert_eq!(eval("xs:date('2020-02-29')"), "2020-02-29");
    assert_eq!(eval("xs:date('1900-02-29')"), "error FORG0001");
    assert_eq!(eval("xs:date('2019-02-29')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-04-31')"), "error FORG0001");
    assert_eq!(eval("xs:date('2020-13-01')"), "error FORG0001");
    // Years: at least four digits, year 0 exists (XML Schema 1.1).
    assert_eq!(eval("xs:date('0000-01-01')"), "0000-01-01");
    assert_eq!(eval("xs:date('-0001-12-31')"), "-0001-12-31");
    assert_eq!(eval("xs:date('-0000-01-01')"), "0000-01-01");
    assert_eq!(eval("xs:date('10000-01-01')"), "10000-01-01");
    assert_eq!(eval("xs:date('01000-01-01')"), "error FORG0001");
    assert_eq!(eval("xs:date('200-01-01')"), "error FORG0001");
    assert_eq!(
        eval("xs:dateTime('2020-01-01T10:00:00.1230+01:00')"),
        "2020-01-01T10:00:00.123+01:00"
    );
    assert_eq!(
        eval("xs:dateTime('2020-01-01T10:00:00.1234567891234')"),
        "2020-01-01T10:00:00.123456789"
    );
    assert_eq!(
        eval("xs:dateTime('2020-12-31T24:00:00')"),
        "2021-01-01T00:00:00"
    );
    assert_eq!(eval("xs:dateTime('2020-01-01T10:00')"), "error FORG0001");
    assert_eq!(
        eval("xs:dateTime('2020-01-01T10:00:00.')"),
        "error FORG0001"
    );
    assert_eq!(eval("xs:time('24:00:00')"), "00:00:00");
    assert_eq!(eval("xs:time('24:00:01')"), "error FORG0001");
    assert_eq!(eval("xs:time('23:59:60')"), "error FORG0001");
    assert_eq!(eval("xs:time('12:60:00')"), "error FORG0001");
    assert_eq!(eval("xs:gYear('-0044')"), "-0044");
    assert_eq!(eval("xs:gYear('2020Z')"), "2020Z");
    assert_eq!(eval("xs:gYearMonth('2020-02')"), "2020-02");
    assert_eq!(eval("xs:gYearMonth('2020-13')"), "error FORG0001");
    assert_eq!(eval("xs:gMonthDay('--02-29')"), "--02-29");
    assert_eq!(eval("xs:gMonthDay('--04-31')"), "error FORG0001");
    assert_eq!(eval("xs:gDay('---31')"), "---31");
    assert_eq!(eval("xs:gDay('---32')"), "error FORG0001");
    assert_eq!(eval("xs:gMonth('--12')"), "--12");
    assert_eq!(eval("xs:gMonth('--12--')"), "error FORG0001");
}

#[test]
fn duration_lexical_forms() {
    assert_eq!(eval("xs:duration('P1Y13M')"), "P2Y1M");
    assert_eq!(eval("xs:duration('-P1DT25H61M')"), "-P2DT2H1M");
    assert_eq!(eval("xs:duration('PT1.5S')"), "PT1.5S");
    assert_eq!(eval("xs:duration('P0D')"), "PT0S");
    assert_eq!(eval("xs:dayTimeDuration('-PT0S')"), "PT0S");
    assert_eq!(eval("xs:yearMonthDuration('P0Y')"), "P0M");
    assert_eq!(
        eval("xs:dayTimeDuration('PT1.123456789S')"),
        "PT1.123456789S"
    );
    assert_eq!(eval("xs:dayTimeDuration('PT0.0000000001S')"), "PT0S");
    for invalid in [
        "xs:dayTimeDuration('P1Y')",
        "xs:yearMonthDuration('P1D')",
        "xs:yearMonthDuration('PT1H')",
        "xs:duration('P')",
        "xs:duration('PT')",
        "xs:duration('P1.5D')",
        "xs:duration('PT1.5M')",
        "xs:duration('P1D1Y')",
        "xs:duration('P1H')",
        "xs:duration('1D')",
        "xs:duration('+P1D')",
    ] {
        assert_eq!(eval(invalid), "error FORG0001", "{invalid}");
    }
}

#[test]
fn date_and_duration_casts() {
    assert_eq!(eval("'2020-01-01' cast as xs:date"), "2020-01-01");
    assert_eq!(eval("//tei:l[1]/@n castable as xs:date"), "false");
    assert_eq!(eval("'1850' castable as xs:gYear"), "true");
    assert_eq!(
        eval("xs:date('2020-01-01') cast as xs:dateTime"),
        "2020-01-01T00:00:00"
    );
    assert_eq!(
        eval("xs:dateTime('2020-01-01T10:00:00Z') cast as xs:gMonthDay"),
        "--01-01Z"
    );
    assert_eq!(
        eval("xs:dateTime('2020-01-01T10:30:00') cast as xs:time"),
        "10:30:00"
    );
    assert_eq!(eval("xs:date('2020-05-01') cast as xs:gYear"), "2020");
    assert_eq!(
        eval("xs:time('10:00:00') cast as xs:date"),
        "error XPTY0004"
    );
    assert_eq!(eval("xs:gYear('2020') cast as xs:date"), "error XPTY0004");
    assert_eq!(eval("xs:gYear('2020') castable as xs:date"), "false");
    assert_eq!(eval("1 cast as xs:date"), "error XPTY0004");
    assert_eq!(
        eval("xs:date('2020-01-01') cast as xs:integer"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("string(xs:date('2020-01-01')) castable as xs:date"),
        "true"
    );
    assert_eq!(
        eval("xs:yearMonthDuration('P1Y') cast as xs:dayTimeDuration"),
        "PT0S"
    );
    assert_eq!(
        eval("xs:duration('P1Y2DT3H') cast as xs:yearMonthDuration"),
        "P1Y"
    );
    assert_eq!(
        eval("xs:duration('P1Y2DT3H') cast as xs:dayTimeDuration"),
        "P2DT3H"
    );
    assert_eq!(eval("xs:date('2020-01-01') instance of xs:date"), "true");
    assert_eq!(
        eval("xs:dateTime('2020-01-01T00:00:00') instance of xs:date"),
        "false"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') instance of xs:duration"),
        "true"
    );
    assert_eq!(
        eval("xs:duration('P1D') instance of xs:dayTimeDuration"),
        "false"
    );
    assert_eq!(eval("boolean(xs:date('2020-01-01'))"), "error FORG0006");
}

#[test]
fn date_and_duration_comparisons() {
    assert_eq!(
        eval("xs:date('2020-01-01') lt xs:date('2020-01-02')"),
        "true"
    );
    assert_eq!(
        eval("xs:date('2020-01-01Z') eq xs:date('2020-01-01')"),
        "true"
    );
    assert_eq!(
        eval("xs:time('10:00:00Z') eq xs:time('11:00:00+01:00')"),
        "true"
    );
    assert_eq!(
        eval("xs:dateTime('2020-01-01T00:00:00+14:00') lt xs:dateTime('2019-12-31T12:00:00Z')"),
        "true"
    );
    assert_eq!(eval("xs:gDay('---01Z') eq xs:gDay('---01+01:00')"), "false");
    assert_eq!(eval("xs:gYear('2020') eq xs:gYear('2020Z')"), "true");
    assert_eq!(
        eval("xs:gYear('2020') lt xs:gYear('2021')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') = '2020-01-01'"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') eq xs:dateTime('2020-01-01T00:00:00')"),
        "error XPTY0004"
    );
    // An untyped value takes the type of the other operand.
    assert_eq!(
        eval_at(
            r#"<d when="2020-01-01"/>"#,
            "/d",
            "@when = xs:date('2020-01-01')"
        ),
        "true"
    );
    assert_eq!(
        eval_at(
            r#"<d when="2020-01-01"/>"#,
            "/d",
            "@when > xs:date('2019-01-01')"
        ),
        "true"
    );
    assert_eq!(
        eval_at(r#"<d when="1850"/>"#, "/d", "@when = xs:date('2019-01-01')"),
        "error FORG0001"
    );
    assert_eq!(
        eval_at(
            r#"<d when="2020-01-01"/>"#,
            "/d",
            "@when eq xs:date('2020-01-01')"
        ),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:duration('P1Y') eq xs:yearMonthDuration('P12M')"),
        "true"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('PT0S') eq xs:yearMonthDuration('P0M')"),
        "true"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') lt xs:dayTimeDuration('PT25H')"),
        "true"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') lt xs:yearMonthDuration('P1M')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:duration('P1D') lt xs:duration('P2D')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("max((xs:date('2020-01-01'), xs:date('2021-01-01')))"),
        "2021-01-01"
    );
    assert_eq!(
        eval("min((xs:gYear('2020'), xs:gYear('2021')))"),
        "error FORG0006"
    );
    assert_eq!(
        eval(
            "count(distinct-values((xs:date('2020-01-01'), xs:date('2020-01-01Z'), '2020-01-01')))"
        ),
        "2"
    );
    assert_eq!(
        eval("index-of((xs:date('2020-01-01')), xs:date('2020-01-01Z'))"),
        "1"
    );
}

#[test]
fn date_and_duration_arithmetic() {
    assert_eq!(
        eval("xs:date('2020-01-01') - xs:date('2019-01-01')"),
        "P365D"
    );
    assert_eq!(
        eval("xs:date('2021-01-01') - xs:date('2020-01-01')"),
        "P366D"
    );
    assert_eq!(
        eval("xs:date('2020-01-01Z') - xs:date('2020-01-01+05:00')"),
        "PT5H"
    );
    assert_eq!(
        eval("xs:dateTime('2020-01-01T00:00:00') - xs:dateTime('2020-01-01T00:00:00Z')"),
        "PT0S"
    );
    assert_eq!(
        eval("xs:date('2020-01-31') + xs:yearMonthDuration('P1M')"),
        "2020-02-29"
    );
    assert_eq!(
        eval("xs:dateTime('2020-03-31T00:00:00') - xs:yearMonthDuration('P1M')"),
        "2020-02-29T00:00:00"
    );
    assert_eq!(
        eval("xs:date('-0001-12-31') + xs:dayTimeDuration('P1D')"),
        "0000-01-01"
    );
    assert_eq!(
        eval("xs:date('-0044-03-15') + xs:yearMonthDuration('P50Y')"),
        "0006-03-15"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') - xs:dayTimeDuration('PT1H')"),
        "2019-12-31"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') + xs:dayTimeDuration('PT25H')"),
        "2020-01-02"
    );
    assert_eq!(
        eval("xs:time('23:00:00') + xs:dayTimeDuration('PT2H')"),
        "01:00:00"
    );
    assert_eq!(
        eval("xs:yearMonthDuration('P1Y') + xs:date('2020-02-29')"),
        "2021-02-28"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') + xs:duration('P1D')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:time('10:00:00') + xs:yearMonthDuration('P1M')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') - xs:date('2020-01-01')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:date('2020-01-01') + xs:date('2020-01-01')"),
        "error XPTY0004"
    );
    assert_eq!(eval("60 * xs:dayTimeDuration('P1D')"), "P60D");
    assert_eq!(eval("xs:yearMonthDuration('P1Y') * 1.55"), "P1Y7M");
    assert_eq!(eval("xs:yearMonthDuration('P1M') * 0.5"), "P1M");
    assert_eq!(eval("xs:yearMonthDuration('-P1M') * 0.5"), "P0M");
    assert_eq!(eval("xs:yearMonthDuration('P1M') div 2"), "P1M");
    assert_eq!(eval("xs:dayTimeDuration('-P1D') * 2.5"), "-P2DT12H");
    assert_eq!(eval("xs:dayTimeDuration('PT1S') div 3"), "PT0.333333333S");
    assert_eq!(
        eval("xs:yearMonthDuration('P1Y') div xs:yearMonthDuration('P5M')"),
        "2.4"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') div xs:dayTimeDuration('PT0S')"),
        "error FOAR0001"
    );
    assert_eq!(eval("xs:dayTimeDuration('P1D') div 0"), "error FODT0002");
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') * xs:double('NaN')"),
        "error FOCA0005"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') * xs:double('INF')"),
        "error FODT0002"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') + xs:dayTimeDuration('-PT1H')"),
        "PT23H"
    );
    assert_eq!(
        eval("xs:duration('P1D') + xs:duration('P1D')"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') + xs:yearMonthDuration('P1M')"),
        "error XPTY0004"
    );
    assert_eq!(eval("-xs:dayTimeDuration('P1D')"), "error XPTY0004");
    assert_eq!(
        eval("xs:dayTimeDuration('P1D') + //tei:l[1]/@n"),
        "error XPTY0004",
        "untyped operands are doubles"
    );
    assert_eq!(
        eval("sum((xs:dayTimeDuration('P1D'), xs:dayTimeDuration('PT1H')))"),
        "P1DT1H"
    );
    assert_eq!(
        eval("avg((xs:yearMonthDuration('P1Y'), xs:yearMonthDuration('P2Y')))"),
        "P1Y6M"
    );
    assert_eq!(
        eval("xs:date('999999999-12-31') + xs:dayTimeDuration('P1D')"),
        "error FODT0001"
    );
}

#[test]
fn date_and_duration_functions() {
    assert_eq!(eval("year-from-date(xs:date('0000-01-01'))"), "0");
    assert_eq!(eval("year-from-date(xs:date('-0044-03-15'))"), "-44");
    assert_eq!(
        eval_at(r#"<d when="2020-07-14"/>"#, "/d", "month-from-date(@when)"),
        "7"
    );
    assert_eq!(
        eval_at(r#"<d when="1850"/>"#, "/d", "month-from-date(@when)"),
        "error FORG0001"
    );
    assert_eq!(eval("day-from-date(())"), "");
    assert_eq!(
        eval("day-from-date(xs:dateTime('2020-01-01T00:00:00'))"),
        "error XPTY0004"
    );
    assert_eq!(
        eval("hours-from-dateTime(xs:dateTime('2020-01-01T24:00:00'))"),
        "0"
    );
    assert_eq!(
        eval("seconds-from-dateTime(xs:dateTime('2020-01-01T10:00:01.50'))"),
        "1.5"
    );
    assert_eq!(eval("minutes-from-time(xs:time('10:42:00'))"), "42");
    assert_eq!(
        eval("timezone-from-date(xs:date('2020-01-01-05:30'))"),
        "-PT5H30M"
    );
    assert_eq!(eval("timezone-from-time(xs:time('10:00:00'))"), "");
    assert_eq!(eval("months-from-duration(xs:duration('-P1Y2M'))"), "-2");
    assert_eq!(eval("years-from-duration(xs:duration('P1Y14M'))"), "2");
    assert_eq!(eval("days-from-duration(xs:yearMonthDuration('P1Y'))"), "0");
    assert_eq!(
        eval("hours-from-duration(xs:dayTimeDuration('P1DT25H'))"),
        "1"
    );
    assert_eq!(
        eval("seconds-from-duration(xs:duration('-PT1.5S'))"),
        "-1.5"
    );
    assert_eq!(eval("implicit-timezone()"), "PT0S");
    assert_eq!(
        eval("adjust-date-to-timezone(xs:date('2020-01-01'))"),
        "2020-01-01Z"
    );
    assert_eq!(
        eval(
            "adjust-dateTime-to-timezone(xs:dateTime('2020-01-01T10:00:00+05:00'), xs:dayTimeDuration('-PT10H'))"
        ),
        "2019-12-31T19:00:00-10:00"
    );
    assert_eq!(
        eval("adjust-dateTime-to-timezone(xs:dateTime('2020-01-01T10:00:00+05:00'), ())"),
        "2020-01-01T10:00:00"
    );
    assert_eq!(
        eval("adjust-date-to-timezone(xs:date('2020-01-01+05:00'), xs:dayTimeDuration('-PT10H'))"),
        "2019-12-31-10:00"
    );
    assert_eq!(
        eval("adjust-time-to-timezone(xs:time('10:00:00'), xs:dayTimeDuration('PT15H'))"),
        "error FODT0003"
    );
    assert_eq!(
        eval("adjust-time-to-timezone(xs:time('10:00:00Z'), xs:dayTimeDuration('PT2H'))"),
        "12:00:00+02:00"
    );
    assert_eq!(
        eval("dateTime(xs:date('2020-01-01Z'), xs:time('10:00:00'))"),
        "2020-01-01T10:00:00Z"
    );
    assert_eq!(
        eval("dateTime(xs:date('2020-01-01Z'), xs:time('10:00:00+01:00'))"),
        "error FORG0008"
    );
}

#[test]
fn current_date_and_time_are_stable() {
    let now = "2026-10-16T17:52:52.5Z";
    assert_eq!(eval_at_time(now, "current-dateTime()"), now);
    assert_eq!(eval_at_time(now, "current-date()"), "2026-10-16Z");
    assert_eq!(eval_at_time(now, "current-time()"), "17:52:52.5Z");
    assert_eq!(
        eval_at_time(now, "current-date() + 60 * xs:dayTimeDuration('P1D')"),
        "2026-12-15Z"
    );
    assert_eq!(
        eval_at_time(now, "xs:date('2026-10-16') ge current-date()"),
        "true"
    );
    assert_eq!(
        eval_at_time(now, "xs:date('2026-10-15') ge current-date()"),
        "false"
    );
    // The real clock: read once per document.
    assert_eq!(
        eval(
            "current-dateTime() eq current-dateTime() and current-date() eq xs:date(current-dateTime())"
        ),
        "true"
    );
    assert_eq!(eval("current-dateTime() instance of xs:dateTime"), "true");
}

#[test]
fn xpath_3_syntax() {
    assert_eq!(eval("'a' || 1 || () || 'b'"), "a1b");
    assert_eq!(eval("'x' || 'y' = 'xy'"), "true");
    assert_eq!(eval("(1, 2) ! (. * 2) ! string()"), "2|4");
    assert_eq!(eval("//tei:l ! @n"), "1|2|10");
    assert_eq!(eval("1 ! (2, 3) ! position()"), "1|2");
    assert_eq!(eval("(5, 6) ! last()"), "2|2");
    assert_eq!(eval("//tei:l[2] ! (. || '!')"), "two!");
    assert_eq!(eval("let $x := 2, $y := $x + 1 return $x * $y"), "6");
    assert_eq!(eval("let $l := //tei:l return count($l)"), "3");
    assert_eq!(eval("'a' => upper-case()"), "A");
    assert_eq!(eval("'a,b' => tokenize(',')"), "a|b");
    assert_eq!(eval("'abc' => substring(2) => upper-case()"), "BC");
    assert_eq!(eval("-1 => abs()"), "1");
    assert_eq!(eval("string-join(1 to 3)"), "123");
    assert_eq!(eval("has-children(//tei:lg)"), "true");
    assert_eq!(eval("has-children(//tei:anchor)"), "false");
    assert_eq!(eval("has-children(())"), "false");
    assert_eq!(eval("count(innermost(//*))"), "8");
    assert_eq!(
        eval("outermost(//tei:l | //tei:lg | //tei:p)/name()"),
        "p|p|lg"
    );
    assert_eq!(
        eval("innermost(//tei:l | //tei:lg | //tei:body)/@n"),
        "1|2|10"
    );
    assert_eq!(
        eval("path(//tei:l[2])"),
        "/Q{http://www.tei-c.org/ns/1.0}TEI[1]/Q{http://www.tei-c.org/ns/1.0}text[1]/Q{http://www.tei-c.org/ns/1.0}body[1]/Q{http://www.tei-c.org/ns/1.0}lg[1]/Q{http://www.tei-c.org/ns/1.0}l[2]"
    );
    assert_eq!(
        eval("path(//tei:p[2]/@xml:id)"),
        "/Q{http://www.tei-c.org/ns/1.0}TEI[1]/Q{http://www.tei-c.org/ns/1.0}text[1]/Q{http://www.tei-c.org/ns/1.0}body[1]/Q{http://www.tei-c.org/ns/1.0}p[2]/@Q{http://www.w3.org/XML/1998/namespace}id"
    );
    assert_eq!(eval("path(//tei:p[2]/@n) => ends-with('/@n')"), "true");
    assert_eq!(eval("path(/)"), "/");
    // A chain of `!` or `=>` nests like any expression.
    let chain = vec!["1"; 300].join(" ! ");
    assert!(parse_expr(&chain, &context()).is_err());
    let chain = format!("'a'{}", " => string()".repeat(300));
    assert!(parse_expr(&chain, &context()).is_err());
}

#[test]
fn base_uris() {
    let xml = r#"<r xml:base="http://example.org/a/b.xml"><c xml:base="../d/"><e/></c><f xml:base="g/#frag"/><h/></r>"#;
    assert_eq!(
        eval_at(xml, "/r/c/e", "base-uri()"),
        "http://example.org/d/"
    );
    assert_eq!(
        eval_at(xml, "/r/c/e", "base-uri(..)"),
        "http://example.org/d/"
    );
    assert_eq!(
        eval_at(xml, "/r/c", "base-uri(@xml:base)"),
        "http://example.org/d/"
    );
    assert_eq!(
        eval_at(xml, "/r", "base-uri(f)"),
        "http://example.org/a/g/#frag"
    );
    assert_eq!(
        eval_at(xml, "/r", "base-uri(h)"),
        "http://example.org/a/b.xml"
    );
    assert_eq!(
        eval_at(xml, "/r", "base-uri(/)"),
        "file:/data/DDbDP/1/1.xml"
    );
    assert_eq!(eval_at(xml, "/r", "base-uri(())"), "");
    assert_eq!(eval("base-uri(//tei:l[1])"), "file:/data/DDbDP/1/1.xml");
    assert_eq!(
        eval_at(r#"<r xml:base="sub/x.xml"/>"#, "/r", "base-uri()"),
        "file:/data/DDbDP/1/sub/x.xml"
    );
    assert_eq!(
        eval_at(r#"<r xml:base="/abs/"/>"#, "/r", "base-uri()"),
        "file:/abs/"
    );
    assert_eq!(
        eval_at(r#"<r xml:base="../../x/./y/../z"/>"#, "/r", "base-uri()"),
        "file:/data/x/z"
    );
}

#[test]
fn invalid_regular_expressions_fail_when_evaluated() {
    // Saxon compiles these and raises the error when the call runs.
    assert!(parse_expr("matches(., '\\,')", &context()).is_ok());
    assert_eq!(eval("matches('a,b', '\\,')"), "error FORX0002");
    assert_eq!(eval("if (false()) then matches('a,b', '\\,') else 1"), "1");
    assert_eq!(eval("replace('a', '[a', 'b')"), "error FORX0002");
    assert_eq!(eval("matches('a', 'a', 'z')"), "error FORX0001");
}

#[test]
fn rejects_what_it_does_not_support() {
    let unsupported = [
        "format-date(current-date(), '[Y]')",
        "xs:QName('a')",
        "@a cast as xs:dateTimeStamp",
        "matches(., '(a)\\1')",
        "matches(., '\\p{IsBasicLatin}')",
        "'a' => $f()",
        "foo:bar()",
        "$undeclared",
        "namespace::*",
        "1 +",
    ];
    for expr in unsupported {
        assert!(
            parse_expr(expr, &context()).is_err(),
            "{expr} should not compile"
        );
    }
}

fn matches(path: &str, pattern: &str) -> bool {
    let doc = roxmltree::Document::parse(DOC).unwrap();
    let document = Document::new(&doc, None);
    let mut evaluator = Evaluator::new(&document);
    let node = if path == "/" {
        document.root()
    } else {
        let selector = parse_expr(path, &context()).unwrap();
        evaluator
            .nodes(&selector, Some(&Focus::node(document.root())))
            .unwrap()[0]
    };
    let pattern = parse_pattern(pattern, &context()).unwrap();
    evaluator.matches(&pattern, node).unwrap()
}

#[test]
fn match_patterns() {
    assert!(matches("//tei:l[1]", "tei:l"));
    assert!(matches("//tei:l[1]", "tei:lg/tei:l"));
    assert!(matches("//tei:l[1]", "tei:body//tei:l"));
    assert!(matches("//tei:l[1]", "/tei:TEI//tei:l"));
    assert!(!matches("//tei:l[1]", "/tei:l"));
    assert!(matches("//tei:l[1]", "tei:*[@n]"));
    assert!(matches("//tei:l[1]", "tei:l[1]"));
    assert!(!matches("//tei:l[2]", "tei:l[1]"));
    assert!(matches("//tei:l[3]", "tei:l[last()]"));
    assert!(matches("//tei:l[2]", "tei:l[@n = 2]"));
    assert!(!matches("//tei:l[2]", "tei:p | tei:hi"));
    assert!(matches("//tei:hi", "tei:p | tei:hi"));
    assert!(matches("//tei:addSpan/@spanTo", "tei:addSpan/@spanTo"));
    assert!(matches("//tei:addSpan/@spanTo", "@*"));
    assert!(!matches("//tei:addSpan", "@*"));
    assert!(matches("/", "/"));
    assert!(!matches("//tei:hi", "/"));
    assert!(matches("//tei:p[1]/text()[1]", "text()"));
    assert!(matches("//tei:hi", "tei:p//*"));
    assert!(matches("//tei:hi", "*[ancestor::tei:body]"));
    assert!(matches("//tei:hi", "tei:hi[current()/@rend = 'b']"));
    // XSLT 3.0: parenthesised unions as steps.
    assert!(matches("//tei:l[1]", "(tei:p | tei:lg)/tei:l"));
    assert!(!matches("//tei:hi", "(tei:lg | tei:body)/tei:hi"));
    assert!(matches("//tei:hi", "(tei:lg | tei:p)/tei:hi | tei:x"));
    assert!(matches("//tei:hi", "tei:body/(tei:lg | tei:p)/tei:hi"));
    assert!(matches("//tei:hi", "tei:text//(tei:lg | tei:p)/tei:hi"));
    assert!(matches(
        "//tei:hi",
        "/tei:TEI//(tei:lg | tei:p)/(tei:hi | tei:l)"
    ));
    assert!(matches("//tei:hi", "(tei:p | tei:lg)[@n]/tei:hi"));
    assert!(!matches("//tei:hi", "(tei:p | tei:lg)[@xml:id]/tei:hi"));
    assert!(matches("//tei:hi", "((tei:p | tei:lg) | tei:x)/tei:hi"));
    assert!(matches("//tei:hi", "(//tei:p)/tei:hi"));
    assert!(matches(
        "//tei:addSpan/@spanTo",
        "(tei:addSpan | tei:delSpan)/@spanTo"
    ));
    assert!(matches("//tei:l[1]", "(tei:l | tei:p)"));
}

#[test]
fn rejects_patterns_it_does_not_support() {
    for pattern in [
        "(tei:p | tei:lg)[1]/tei:hi",
        "tei:body/(//tei:p)",
        "tei:body/(/tei:p)",
        "id('x')",
        "ancestor::tei:p",
        "(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)\
         /(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)/(tei:a | tei:b)",
    ] {
        assert!(
            parse_pattern(pattern, &context()).is_err(),
            "{pattern} should not compile"
        );
    }
}

#[test]
fn predicates_comparing_with_a_fixed_value() {
    let xml = r##"<r><span to="#b"/><x id="a">1</x><x id="b">2</x><x id="b">3</x><y n="2"/><y n="5"/></r>"##;
    // The right-hand side uses only current(), so it is evaluated once.
    assert_eq!(
        eval_at(
            xml,
            "//span",
            "following::*[@id = substring(current()/@to, 2)]"
        ),
        "2|3"
    );
    // Either side may be the fixed one, and a fixed sequence compares
    // existentially.
    assert_eq!(
        eval_at(xml, "//span", "following::x[('a', 'b') = @id]"),
        "1|2|3"
    );
    assert_eq!(eval_at(xml, "//span", "following::y[@n > 1 + 1]/@n"), "5");
    // Comparisons that depend on the item are evaluated per item.
    assert_eq!(eval_at(xml, "//span", "following::x[position() = 2]"), "2");
    assert_eq!(eval_at(xml, "//span", "following::x[. = @id]"), "");
    assert_eq!(
        eval_at(xml, "//span", "following::x[string-length() = 1]"),
        "1|2|3"
    );
}
