//! Unit tests for `doc()`, `document()` and `doc-available()`: URI
//! resolution, the directories rules may read, caching and node identity
//! across documents.

use std::path::{Path, PathBuf};

use super::super::{Document, Evaluator, Focus, Item, StaticContext, parse_expr};
use super::*;

/// A fresh directory for one test: `schema/` (the schema's directory),
/// `docs/` (the validated document's), `store/` (a schema store) and
/// `outside/` (none of them).
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("teixml-resources-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["schema/lists", "docs/sub", "store/example.org", "outside"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    let write = |path: &str, text: &str| std::fs::write(dir.join(path), text).unwrap();
    write(
        "schema/lists/kinds.xml",
        "<kinds><kind>poem</kind><kind>novel</kind></kinds>",
    );
    write(
        "docs/sub/terms.xml",
        r#"<terms xml:base="nested/"><term xml:id="noun">noun</term><term xml:id="empty"/></terms>"#,
    );
    write("store/example.org/remote.xml", "<remote>stored</remote>");
    write("outside/secret.xml", "<secret>secret</secret>");
    write("docs/broken.xml", "<broken>");
    write(
        "store/manifest.json",
        r#"{"schemas": [
            {"url": "https://example.org/remote.xml", "file": "example.org/remote.xml"},
            {"url": "https://example.org/listed.xml"}
        ]}"#,
    );
    std::fs::create_dir_all(dir.join("docs/sub/nested")).unwrap();
    write("docs/sub/nested/deep.xml", "<deep>deep</deep>");
    dir
}

const MAIN: &str = r#"<doc xmlns:xml="http://www.w3.org/XML/1998/namespace">
  <item ana="sub/terms.xml#noun"/>
  <item ana="sub/terms.xml#empty"/>
  <item ana="sub/terms.xml#missing"/>
  <item ana="sub/terms.xml"/>
  <group xml:base="sub/"><item ana="terms.xml#noun"/></group>
</doc>"#;

/// Evaluates `expr` on `docs/main.xml` (with `MAIN` as its text), with the
/// schema in `schema/` and, if `store` is set, the store in `store/`.
/// Returns the string values joined with `|`, or the error code; with
/// `detail`, the error message too.
fn run(dir: &Path, store: bool, expr: &str, detail: bool) -> (String, usize) {
    let main_path = dir.join("docs/main.xml");
    std::fs::write(&main_path, MAIN).unwrap();
    let doc = roxmltree::Document::parse(MAIN).unwrap();
    let access = Access {
        store: store.then(|| SchemaStore::open(dir.join("store")).unwrap()),
    };
    let resources = Resources::new(&access, Some(&dir.join("schema")), Some(&main_path));
    let document = Document::new(&doc, Some(file_uri(&main_path)));
    let context = StaticContext {
        base_uri: Some(file_uri(&dir.join("schema/schema.rng"))),
        ..StaticContext::default()
    };
    let compiled = parse_expr(expr, &context).unwrap_or_else(|error| panic!("{expr}: {error}"));
    let mut evaluator = Evaluator::with_resources(&document, &resources);
    let result = match evaluator.eval(&compiled, Some(&Focus::node(document.root()))) {
        Ok(items) => items.iter().map(Item::string).collect::<Vec<_>>().join("|"),
        Err(error) if detail => format!("error {}: {}", error.code, error.message),
        Err(error) => format!("error {}", error.code),
    };
    (result, resources.count.get())
}

fn eval(dir: &Path, expr: &str) -> String {
    run(dir, true, expr, false).0
}

#[test]
fn resolves_uris_like_rfc_3986() {
    let base = Some("file:/a/b/c.rng");
    assert_eq!(resolve_uri("d.xml", base).unwrap(), "file:/a/b/d.xml");
    assert_eq!(resolve_uri("../d.xml", base).unwrap(), "file:/a/d.xml");
    assert_eq!(
        resolve_uri("../../../../d.xml", base).unwrap(),
        "file:/d.xml"
    );
    assert_eq!(
        resolve_uri("./x/./y/../d.xml#f", base).unwrap(),
        "file:/a/b/x/d.xml#f"
    );
    assert_eq!(resolve_uri("/d.xml", base).unwrap(), "file:/d.xml");
    assert_eq!(resolve_uri("", base).unwrap(), "file:/a/b/c.rng");
    assert_eq!(resolve_uri("#x", base).unwrap(), "file:/a/b/c.rng#x");
    assert_eq!(
        resolve_uri("../i/p.xml", Some("https://h.org/a/b.rdf")).unwrap(),
        "https://h.org/i/p.xml"
    );
    assert_eq!(
        resolve_uri("p.xml", Some("https://h.org")).unwrap(),
        "https://h.org/p.xml"
    );
    assert_eq!(
        resolve_uri("//other.org/p.xml", Some("https://h.org/a")).unwrap(),
        "https://other.org/p.xml"
    );
    assert_eq!(
        resolve_uri("https://x.org/y", base).unwrap(),
        "https://x.org/y"
    );
    assert_eq!(resolve_uri("/abs.xml", None).unwrap(), "file:/abs.xml");
    assert_eq!(resolve_uri("rel.xml", None), None);
    assert_eq!(
        file_path("file:///a%20b/c.xml").unwrap(),
        PathBuf::from("/a b/c.xml")
    );
    assert_eq!(
        file_path("file:/a/c.xml").unwrap(),
        PathBuf::from("/a/c.xml")
    );
    assert_eq!(file_path("file://elsewhere/a/c.xml"), None);
    assert_eq!(file_path("https://x.org/a"), None);
}

#[test]
fn doc_resolves_against_the_schema_and_reads_each_file_once() {
    let dir = fixture("doc");
    assert_eq!(eval(&dir, "doc('lists/kinds.xml')//kind[2]"), "novel");
    let (result, reads) = run(
        &dir,
        true,
        "count(doc('lists/kinds.xml') | doc('./lists/../lists/kinds.xml') | \
         doc('lists/kinds.xml')//kind/root())",
        false,
    );
    // The same document node each time, read once.
    assert_eq!((result.as_str(), reads), ("1", 1));
    assert_eq!(
        eval(&dir, "doc('lists/kinds.xml') is doc('lists/kinds.xml')"),
        "true"
    );
    assert_eq!(eval(&dir, "doc(())"), "");
    assert_eq!(
        eval(&dir, "document-uri(doc('lists/kinds.xml'))"),
        file_uri(&dir.join("schema/lists/kinds.xml"))
    );
    assert_eq!(eval(&dir, "doc('lists/kinds.xml#x')"), "error FODC0005");
    // The validated document itself is the same node as `/`.
    assert_eq!(eval(&dir, "doc('../docs/main.xml') is /"), "true");
}

#[test]
fn missing_and_unreadable_documents_are_errors() {
    let dir = fixture("missing");
    assert_eq!(eval(&dir, "doc('lists/missing.xml')"), "error FODC0002");
    assert_eq!(eval(&dir, "doc-available('lists/missing.xml')"), "false");
    assert_eq!(eval(&dir, "doc('../docs/broken.xml')"), "error FODC0002");
    assert_eq!(eval(&dir, "doc-available('../docs/broken.xml')"), "false");
    // A directory is not a regular file.
    assert_eq!(eval(&dir, "doc('lists')"), "error FODC0002");
    assert_eq!(eval(&dir, "doc-available('lists')"), "false");
    assert_eq!(eval(&dir, "doc-available('lists/kinds.xml')"), "true");
    // Too large: a sparse file just over the limit.
    let large = std::fs::File::create(dir.join("schema/large.xml")).unwrap();
    large.set_len(MAX_RESOURCE_BYTES + 1).unwrap();
    let (result, _) = run(&dir, true, "doc('large.xml')", true);
    assert!(
        result.starts_with("error FODC0002") && result.contains("larger than"),
        "{result}"
    );
    assert_eq!(eval(&dir, "doc-available('large.xml')"), "false");
}

#[test]
fn reads_only_under_the_schema_document_and_store_directories() {
    let dir = fixture("roots");
    // Each of the three directories.
    assert_eq!(eval(&dir, "string(doc('lists/kinds.xml'))"), "poemnovel");
    assert_eq!(
        eval(&dir, "string(doc('../docs/sub/nested/deep.xml'))"),
        "deep"
    );
    assert_eq!(
        eval(&dir, "string(doc('../store/example.org/remote.xml'))"),
        "stored"
    );
    // Without a store, its directory isn't readable.
    assert_eq!(
        run(&dir, false, "doc('../store/example.org/remote.xml')", false).0,
        "error FODC0002"
    );
    // Anywhere else, however it is named.
    let (result, _) = run(&dir, true, "doc('../outside/secret.xml')", true);
    assert!(result.starts_with("error FODC0002"), "{result}");
    assert!(result.contains("only read files under"), "{result}");
    assert!(
        result.contains(
            &dir.join("schema")
                .canonicalize()
                .unwrap()
                .display()
                .to_string()
        )
    );
    let absolute = dir.join("outside/secret.xml").display().to_string();
    assert_eq!(eval(&dir, &format!("doc('{absolute}')")), "error FODC0002");
    assert_eq!(
        eval(&dir, &format!("doc('file://{absolute}')")),
        "error FODC0002"
    );
    assert_eq!(eval(&dir, &format!("doc-available('{absolute}')")), "false");
    // `..` can't climb out of an allowed directory.
    assert_eq!(
        eval(&dir, "doc('lists/../../outside/secret.xml')"),
        "error FODC0002"
    );
    assert_eq!(
        eval(&dir, "document('sub/../../outside/secret.xml', /)"),
        "error FODC0002"
    );
    // Nor can a symbolic link.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("outside/secret.xml"), dir.join("schema/link.xml"))
            .unwrap();
        std::os::unix::fs::symlink(dir.join("outside"), dir.join("docs/linked")).unwrap();
        assert_eq!(eval(&dir, "doc('link.xml')"), "error FODC0002");
        assert_eq!(eval(&dir, "doc-available('link.xml')"), "false");
        assert_eq!(
            eval(&dir, "document('linked/secret.xml', /)"),
            "error FODC0002"
        );
    }
}

#[test]
fn http_uris_come_from_the_store() {
    let dir = fixture("store");
    assert_eq!(
        eval(&dir, "string(doc('https://example.org/remote.xml'))"),
        "stored"
    );
    assert_eq!(
        eval(&dir, "document-uri(doc('https://example.org/remote.xml'))"),
        "https://example.org/remote.xml"
    );
    assert_eq!(
        eval(&dir, "doc-available('https://example.org/remote.xml')"),
        "true"
    );
    let (result, _) = run(&dir, true, "doc('https://example.org/other.xml')", true);
    assert!(
        result.contains("run `tei-rs schemas add https://example.org/other.xml`"),
        "{result}"
    );
    let (result, _) = run(&dir, true, "doc('https://example.org/listed.xml')", true);
    assert!(result.contains("run `tei-rs schemas sync`"), "{result}");
    assert_eq!(
        eval(&dir, "doc-available('https://example.org/other.xml')"),
        "false"
    );
    let (result, _) = run(&dir, false, "doc('https://example.org/remote.xml')", true);
    assert!(result.contains("never touches the network"), "{result}");
    assert_eq!(
        run(
            &dir,
            false,
            "doc-available('https://example.org/remote.xml')",
            false
        )
        .0,
        "false"
    );
    assert_eq!(
        eval(&dir, "doc('ftp://example.org/x.xml')"),
        "error FODC0002"
    );
}

#[test]
fn document_resolves_nodes_against_their_base_uri() {
    let dir = fixture("document");
    // Fragments select by xml:id; one that isn't there selects nothing.
    assert_eq!(eval(&dir, "document(//item[1]/@ana)"), "noun");
    assert_eq!(eval(&dir, "count(document(//item[2]/@ana))"), "1");
    assert_eq!(eval(&dir, "count(document(//item[3]/@ana))"), "0");
    assert_eq!(eval(&dir, "name(document(//item[4]/@ana)/*)"), "terms");
    // xml:base on an ancestor counts.
    assert_eq!(eval(&dir, "document(//group/item/@ana)"), "noun");
    assert_eq!(
        eval(
            &dir,
            "document(//item[1]/@ana) is document(//group/item/@ana)"
        ),
        "true"
    );
    // Several URIs: the nodes in document order, without duplicates.
    assert_eq!(eval(&dir, "count(document(//item/@ana))"), "3");
    // A string resolves against the static base; a second argument gives
    // the base for all.
    assert_eq!(eval(&dir, "document('lists/kinds.xml')//kind[1]"), "poem");
    assert_eq!(eval(&dir, "document('sub/nested/deep.xml', /)"), "deep");
    // In a document read this way, relative URIs resolve against it (and
    // its xml:base).
    assert_eq!(
        eval(&dir, "document('deep.xml', document('sub/terms.xml', /)/*)"),
        "deep"
    );
    // The empty URI is the base's own document.
    assert_eq!(eval(&dir, "document('', /) is /"), "true");
    assert_eq!(eval(&dir, "document(//missing)"), "");
}

#[test]
fn nodes_of_several_documents_have_a_stable_order_and_identity() {
    let dir = fixture("order");
    // The validated document first, then the others in the order read.
    assert_eq!(
        eval(
            &dir,
            "for $k in doc('lists/kinds.xml'), $d in doc('../docs/sub/nested/deep.xml') \
             return string-join((($d/*, /*, $k/*) | ())/name(), ' ')"
        ),
        "doc kinds deep"
    );
    assert_eq!(
        eval(
            &dir,
            "doc('lists/kinds.xml')/* << doc('../docs/sub/nested/deep.xml')/*"
        ),
        "true"
    );
    assert_eq!(eval(&dir, "/* << doc('lists/kinds.xml')/*"), "true");
    assert_eq!(
        eval(
            &dir,
            "count(doc('lists/kinds.xml')//kind | doc('lists/kinds.xml')//kind[1])"
        ),
        "2"
    );
    // Same positions in different documents are different nodes.
    assert_eq!(
        eval(
            &dir,
            "generate-id(/*) = generate-id(doc('lists/kinds.xml')/*)"
        ),
        "false"
    );
    assert_eq!(eval(&dir, "/* is doc('lists/kinds.xml')/*"), "false");
    // Axes and id() stay in the node's own document.
    assert_eq!(
        eval(
            &dir,
            "string-join(doc('lists/kinds.xml')//kind[1]/following::*/name(), ' ')"
        ),
        "kind"
    );
    assert_eq!(
        eval(&dir, "id('noun', doc('../docs/sub/terms.xml'))"),
        "noun"
    );
    assert_eq!(eval(&dir, "count(id('noun'))"), "0");
}

#[test]
fn literal_documents_are_listed_resolved() {
    let context = StaticContext {
        base_uri: Some("file:/s/schema.rng".into()),
        ..StaticContext::default()
    };
    let expr = parse_expr(
        "doc('https://x.org/a.xml')//b[doc-available('c.xml')] | document('d.xml#e', .)",
        &context,
    )
    .unwrap();
    let mut found = Vec::new();
    literal_documents(&expr, &mut found);
    assert_eq!(
        found,
        ["https://x.org/a.xml", "file:/s/c.xml", "file:/s/d.xml"]
    );
}
