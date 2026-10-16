//! Whitespace-only text between child elements must be ignored, not matched
//! against a `text` branch: ParlaMint's `publisher` is
//! `choice(group(orgName+, ref*), text)`, and the indentation before
//! `<orgName>` used to commit validation to the `text` branch, rejecting the
//! elements that followed. Found by comparing with Jing on ParlaMint samples.

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
enum Node {
    Element(&'static str, Vec<Node>),
    Text(&'static str),
}

impl Element for Node {
    type Location = ();

    fn name(&self) -> ExpandedName {
        match self {
            Node::Element(name, _) => ExpandedName::new(None::<String>, *name),
            Node::Text(_) => unreachable!("only elements are asked for their name"),
        }
    }

    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
        std::iter::empty()
    }

    fn children(&self) -> impl Iterator<Item = Content<Self>> {
        let children = match self {
            Node::Element(_, children) => children.clone(),
            Node::Text(_) => Vec::new(),
        };
        children.into_iter().map(|child| match child {
            Node::Text(text) => Content::Text(text.to_owned()),
            element => Content::Element(element),
        })
    }
}

fn schema() -> Schema {
    let source = SchemaSource::new(
        "start = element publisher { (element orgName { text }+, element ref { text }*) | text }",
        "memory:/publisher.rnc",
        SchemaSyntax::Compact,
    );
    Schema::compile(&source, &NoResolver).expect("schema compiles")
}

fn errors(doc: &Node) -> usize {
    schema()
        .validate(&DatatypeRegistry::new(), doc)
        .expect("only built-in datatypes")
        .len()
}

#[test]
fn indentation_before_child_elements_does_not_select_the_text_branch() {
    let doc = Node::Element(
        "publisher",
        vec![
            Node::Text("\n   "),
            Node::Element("orgName", vec![Node::Text("CLARIN")]),
            Node::Text("\n   "),
            Node::Element("ref", vec![Node::Text("www.clarin.eu")]),
            Node::Text("\n"),
        ],
    );
    assert_eq!(errors(&doc), 0);
}

#[test]
fn text_branch_still_accepts_plain_text() {
    let doc = Node::Element("publisher", vec![Node::Text("CLARIN")]);
    assert_eq!(errors(&doc), 0);
}
