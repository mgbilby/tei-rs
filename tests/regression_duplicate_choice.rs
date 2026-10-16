//! A child element reachable through two different `define`s (as TEI's
//! model classes routinely do) used to derive to `choice(empty, empty)`,
//! which never collapsed back to `empty`: the working pattern grew with
//! every sibling and validation went quadratic or worse. Thousands of
//! siblings must validate quickly.

use std::time::{Duration, Instant};

use teixml::relaxng::{
    Content, DatatypeRegistry, Element, ExpandedName, ResolveError, Schema, SchemaResolver,
    SchemaSource, SchemaSyntax,
};

struct NoResolver;

impl SchemaResolver for NoResolver {
    fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
        Err(ResolveError::new(format!("no such resource: {href}")))
    }
}

#[derive(Clone)]
struct Doc {
    name: &'static str,
    children: usize,
}

impl Element for Doc {
    type Location = ();

    fn name(&self) -> ExpandedName {
        ExpandedName::new(None::<String>, self.name)
    }

    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
        std::iter::empty()
    }

    fn children(&self) -> impl Iterator<Item = Content<Self>> {
        (0..self.children).map(|_| {
            Content::Element(Doc {
                name: "x",
                children: 0,
            })
        })
    }
}

#[test]
fn element_reachable_through_two_defines_does_not_grow_the_pattern() {
    let source = SchemaSource::new(
        "start = element root { (a | b)* }
         a = element x { empty }
         b = element x { empty }",
        "memory:/dup.rnc",
        SchemaSyntax::Compact,
    );
    let schema = Schema::compile(&source, &NoResolver).expect("schema compiles");
    let doc = Doc {
        name: "root",
        children: 5_000,
    };

    let started = Instant::now();
    let errors = schema
        .validate(&DatatypeRegistry::new(), &doc)
        .expect("only built-in datatypes");
    assert!(errors.is_empty(), "document is valid: {errors:?}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "validating 5,000 siblings took {:?}",
        started.elapsed()
    );
}
