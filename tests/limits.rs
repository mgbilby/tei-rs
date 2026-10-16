//! Deeply nested input is rejected with an error, never by overflowing the
//! stack, and input just within the limits validates on a thread with Rust's
//! default 2 MB stack.

use teixml::relaxng::SchemaSyntax;
use teixml::{Error, MAX_DOCUMENT_DEPTH, Schema};

const RECURSIVE: &str = "start = a\na = element a { a* }";

fn nested(depth: usize) -> String {
    "<a>".repeat(depth) + &"</a>".repeat(depth)
}

#[test]
fn documents_at_the_depth_limit_validate_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let schema = Schema::from_text(RECURSIVE, "a.rnc", SchemaSyntax::Compact).unwrap();
            let diagnostics = schema.validate_str(&nested(MAX_DOCUMENT_DEPTH)).unwrap();
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deeper_documents_are_rejected() {
    let schema = Schema::from_text(RECURSIVE, "a.rnc", SchemaSyntax::Compact).unwrap();
    for depth in [MAX_DOCUMENT_DEPTH + 1, 100_000] {
        match schema.validate_str(&nested(depth)) {
            Err(Error::Xml(message)) => assert!(message.contains("nested"), "{message}"),
            other => panic!("depth {depth}: {other:?}"),
        }
    }
}

#[test]
fn deeply_nested_schemas_are_rejected() {
    let depth = 100_000;
    let rng = format!(
        r#"<grammar xmlns="http://relaxng.org/ns/structure/1.0"><start>{}<element name="a"><empty/></element>{}</start></grammar>"#,
        "<group>".repeat(depth),
        "</group>".repeat(depth)
    );
    assert!(matches!(
        Schema::from_text(&rng, "deep.rng", SchemaSyntax::Xml),
        Err(Error::Schema(_))
    ));
    let rnc = format!(
        "start = {}element a {{ empty }}{}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    assert!(matches!(
        Schema::from_text(&rnc, "deep.rnc", SchemaSyntax::Compact),
        Err(Error::Schema(_))
    ));
}

#[test]
fn schemas_at_the_depth_limit_compile_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let depth = 500;
            let rnc = format!(
                "start = {}element a {{ empty }}{}",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            let schema = Schema::from_text(&rnc, "deep.rnc", SchemaSyntax::Compact).unwrap();
            assert!(schema.validate_str("<a/>").unwrap().is_empty());
        })
        .unwrap()
        .join()
        .unwrap();
}
