//! Unit tests for the XPath evaluator, against expectations from the
//! XPath 2.0 and XSLT 2.0 specifications.

use std::collections::HashMap;

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
        variables: Vec::new(),
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
    assert_eq!(
        eval("doc-available('https://example.org/x.xml')"),
        "error FODC0002"
    );
    assert_eq!(eval("doc-available('x.xml')"), "error FODC0002");
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    assert_eq!(
        eval(&format!("doc-available('{manifest}')")),
        "false",
        "not XML"
    );
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

#[test]
fn rejects_what_it_does_not_support() {
    let unsupported = [
        "current-date()",
        "doc('x.xml')",
        "xs:date('2020-01-01')",
        "@a cast as xs:date",
        "a ! b",
        "'a' || 'b'",
        "matches(., '(a)\\1')",
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
