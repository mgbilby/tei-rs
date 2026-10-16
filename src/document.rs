//! Adapts a parsed `roxmltree` document to the validator's [`Element`] trait.

use std::fmt;

use std::collections::HashMap;

use crate::relaxng::{
    Content, Element, ElementPath, ExpandedName, IdType, IdTypes, ValidationErrorKind,
};

/// A 1-based line and column in the source document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// Line number, starting at 1.
    pub line: u32,
    /// Column number (in characters), starting at 1.
    pub column: u32,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Converts byte offsets in a document to line and column numbers. Built once
/// per document, so each lookup is a binary search instead of a scan from
/// the start of the text.
pub(crate) struct LineIndex<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();
        Self { text, line_starts }
    }

    /// The 1-based line and column (counted in characters, as `roxmltree`
    /// does) of the byte at `offset`.
    pub(crate) fn position(&self, offset: usize) -> Position {
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let start = self.line_starts[line - 1];
        let column = self.text[start..offset].chars().count() + 1;
        Position {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(column).unwrap_or(u32::MAX),
        }
    }
}

/// An element node in a `roxmltree` document.
#[derive(Clone, Copy)]
pub(crate) struct Node<'a, 'input> {
    node: roxmltree::Node<'a, 'input>,
}

impl<'a, 'input> Node<'a, 'input> {
    pub(crate) fn root(doc: &'a roxmltree::Document<'input>) -> Self {
        Self {
            node: doc.root_element(),
        }
    }
}

impl Element for Node<'_, '_> {
    /// The byte offset of the element's start tag; see [`LineIndex`].
    type Location = usize;

    fn name(&self) -> ExpandedName {
        let tag = self.node.tag_name();
        ExpandedName::new(tag.namespace(), tag.name())
    }

    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
        self.node.attributes().map(|attribute| {
            (
                ExpandedName::new(attribute.namespace(), attribute.name()),
                attribute.value().to_owned(),
            )
        })
    }

    fn children(&self) -> impl Iterator<Item = Content<Self>> {
        Children {
            next: self.node.first_child(),
            text: String::new(),
            pending: None,
        }
    }

    /// Just past the `>` that ends the element's start tag, where Jing
    /// reports problems with the element and its attributes.
    fn location(&self) -> Option<usize> {
        Some(start_tag_end(self.node))
    }

    fn start_location(&self) -> Option<usize> {
        Some(self.node.range().start)
    }

    fn end_tag_location(&self) -> Option<usize> {
        let range = self.node.range();
        let text = &self.node.document().input_text()[range.clone()];
        text.rfind("</").map(|offset| range.start + offset)
    }

    /// Just past the element's end tag, where Jing also reports it.
    fn end_location(&self) -> Option<usize> {
        Some(self.node.range().end)
    }

    fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
        self.node.namespaces().map(|namespace| {
            (
                namespace.name().unwrap_or_default().to_owned(),
                namespace.uri().to_owned(),
            )
        })
    }
}

/// The offset just past the `>` that ends `node`'s start tag.
fn start_tag_end(node: roxmltree::Node<'_, '_>) -> usize {
    let text = node.document().input_text().as_bytes();
    let mut quote = None;
    for (offset, &byte) in text.iter().enumerate().skip(node.range().start) {
        match (quote, byte) {
            (Some(q), _) if byte == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return offset + 1,
            _ => {}
        }
    }
    node.range().start
}

/// A problem found by [`id_errors`]: where (a byte offset), what, and the
/// path of element names to it.
pub(crate) type IdError = (usize, ValidationErrorKind, ElementPath);

/// Checks the attributes `ids` gives an ID-type, as Jing does: every ID value
/// is unique, and every IDREF names one. Returns the duplicates (each
/// followed by where that ID first occurred), in document order, and the
/// unresolved references, which are only known at the end.
pub(crate) fn id_errors(
    doc: &roxmltree::Document<'_>,
    ids: &IdTypes,
) -> (Vec<IdError>, Vec<IdError>) {
    let mut defined: HashMap<&str, roxmltree::Node<'_, '_>> = HashMap::new();
    let mut references: Vec<(&str, roxmltree::Node<'_, '_>)> = Vec::new();
    let mut duplicates = Vec::new();
    for element in doc.descendants().filter(roxmltree::Node::is_element) {
        let element_name = (
            element.tag_name().namespace().unwrap_or_default(),
            element.tag_name().name(),
        );
        for attribute in element.attributes() {
            let attribute_name = (attribute.namespace().unwrap_or_default(), attribute.name());
            let Some(id_type) = ids.get(element_name, attribute_name) else {
                continue;
            };
            let value = attribute.value();
            match id_type {
                IdType::Id => {
                    let id = value.trim_matches(|c: char| c.is_ascii_whitespace());
                    if let Some(&first) = defined.get(id) {
                        duplicates.push((
                            start_tag_end(element),
                            ValidationErrorKind::DuplicateId(id.into()),
                            path(element),
                        ));
                        duplicates.push((
                            start_tag_end(first),
                            ValidationErrorKind::FirstIdOccurrence(id.into()),
                            path(first),
                        ));
                    } else {
                        defined.insert(id, element);
                    }
                }
                IdType::Idref => references.push((
                    value.trim_matches(|c: char| c.is_ascii_whitespace()),
                    element,
                )),
                IdType::Idrefs => {
                    references.extend(value.split_ascii_whitespace().map(|id| (id, element)))
                }
            }
        }
    }
    let unresolved = references
        .into_iter()
        .filter(|(id, _)| !defined.contains_key(id))
        .map(|(id, element)| {
            (
                start_tag_end(element),
                ValidationErrorKind::UnresolvedIdref(id.into()),
                path(element),
            )
        })
        .collect();
    (duplicates, unresolved)
}

/// The names of `element` and its ancestors.
pub(crate) fn path(element: roxmltree::Node<'_, '_>) -> ElementPath {
    let mut names: Vec<ExpandedName> = element
        .ancestors()
        .filter(roxmltree::Node::is_element)
        .map(|ancestor| {
            let tag = ancestor.tag_name();
            ExpandedName::new(tag.namespace(), tag.name())
        })
        .collect();
    names.reverse();
    ElementPath::from_names(names)
}

/// Yields child elements and merged runs of text, skipping comments and
/// processing instructions (and merging text across them, as RELAX NG
/// requires).
struct Children<'a, 'input> {
    next: Option<roxmltree::Node<'a, 'input>>,
    text: String,
    pending: Option<Node<'a, 'input>>,
}

impl<'a, 'input> Iterator for Children<'a, 'input> {
    type Item = Content<Node<'a, 'input>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(element) = self.pending.take() {
            return Some(Content::Element(element));
        }
        while let Some(node) = self.next {
            self.next = node.next_sibling();
            if node.is_element() {
                let element = Node { node };
                if self.text.is_empty() {
                    return Some(Content::Element(element));
                }
                self.pending = Some(element);
                return Some(Content::Text(std::mem::take(&mut self.text)));
            }
            if let Some(text) = node.text().filter(|_| node.is_text()) {
                self.text.push_str(text);
            }
        }
        (!self.text.is_empty()).then(|| Content::Text(std::mem::take(&mut self.text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_matches_roxmltree() {
        let text = "<a>\n  <b/>\r\n<c>é<d/></c>\n\n<e/></a>";
        let doc = roxmltree::Document::parse(text).unwrap();
        let index = LineIndex::new(text);
        for node in doc.descendants().filter(roxmltree::Node::is_element) {
            let start = node.range().start;
            let expected = doc.text_pos_at(start);
            assert_eq!(
                index.position(start),
                Position {
                    line: expected.row,
                    column: expected.col
                },
                "{:?}",
                node.tag_name()
            );
        }
    }
}
