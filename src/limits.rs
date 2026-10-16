//! Limits on nesting depth. Parsing (roxmltree's included), schema
//! simplification and validation are recursive, so input nested deeply
//! enough would overflow the stack and abort the process. Input is checked
//! against these limits first and rejected with an error instead.

/// How deeply a document's elements may nest; deeper documents are rejected
/// with [`Error::Xml`](crate::Error::Xml). Real TEI documents nest a few
/// dozen levels.
pub const MAX_DOCUMENT_DEPTH: usize = 1024;

/// How deeply a schema's patterns may nest, in either syntax.
pub(crate) const MAX_SCHEMA_DEPTH: usize = 512;

/// The largest file Schematron rules read with `doc()`, `document()` or
/// `doc-available()`; larger files, like anything but regular files, are
/// never read.
#[cfg(feature = "schematron")]
pub(crate) const MAX_RESOURCE_BYTES: u64 = 256 * 1024 * 1024;

/// How many documents, and how many bytes in all, `doc()` and `document()`
/// may read while one document's Schematron rules run.
#[cfg(feature = "schematron")]
pub(crate) const MAX_RESOURCES: usize = 1000;
#[cfg(feature = "schematron")]
pub(crate) const MAX_RESOURCES_BYTES: u64 = 1024 * 1024 * 1024;

/// Whether the elements in the XML `text` nest more than `limit` levels
/// deep. A quick scan of the markup that skips comments, CDATA sections,
/// processing instructions, the document type declaration and attribute
/// values; it may overestimate the depth of text that isn't well-formed,
/// which the parser rejects anyway.
pub(crate) fn xml_nesting_exceeds(text: &str, limit: usize) -> bool {
    let bytes = text.as_bytes();
    let skip_past = |from: usize, end: &[u8]| {
        bytes[from..]
            .windows(end.len())
            .position(|window| window == end)
            .map_or(bytes.len(), |offset| from + offset + end.len())
    };
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        // Markup always starts at an ASCII `<`, so `i` stays on a character
        // boundary.
        match text[i..].find('<') {
            Some(offset) => i += offset,
            None => break,
        }
        let rest = &bytes[i..];
        if rest.starts_with(b"<!--") {
            i = skip_past(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_past(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_past(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            // A declaration, with an internal subset in brackets.
            let mut brackets = 0usize;
            let mut quote = None;
            i += 2;
            while i < bytes.len() {
                let byte = bytes[i];
                i += 1;
                match (quote, byte) {
                    (Some(q), _) if byte == q => quote = None,
                    (Some(_), _) => {}
                    (None, b'"' | b'\'') => quote = Some(byte),
                    (None, b'[') => brackets += 1,
                    (None, b']') => brackets = brackets.saturating_sub(1),
                    (None, b'>') if brackets == 0 => break,
                    _ => {}
                }
            }
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = skip_past(i + 2, b">");
        } else {
            // A start tag; attribute values may contain `>`.
            let mut quote = None;
            let mut self_closing = false;
            i += 1;
            while i < bytes.len() {
                let byte = bytes[i];
                i += 1;
                match (quote, byte) {
                    (Some(q), _) if byte == q => quote = None,
                    (Some(_), _) => {}
                    (None, b'"' | b'\'') => quote = Some(byte),
                    (None, b'>') => {
                        self_closing = bytes[i - 2] == b'/';
                        break;
                    }
                    _ => {}
                }
            }
            if !self_closing {
                depth += 1;
                if depth > limit {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_element_nesting_only() {
        let text = r#"<?xml version="1.0"?><!DOCTYPE a [<!ENTITY e "<b>">]>
            <a x="<c>"><!-- <d><d><d> --><![CDATA[<e><e>]]><f/><g><?pi <h>?></g></a>"#;
        assert!(!xml_nesting_exceeds(text, 2));
        assert!(xml_nesting_exceeds(text, 1));
    }

    #[test]
    fn deep_documents_are_detected_without_parsing() {
        let deep = "<a>".repeat(100_000) + &"</a>".repeat(100_000);
        assert!(xml_nesting_exceeds(&deep, MAX_DOCUMENT_DEPTH));
        let ok = "<a>".repeat(MAX_DOCUMENT_DEPTH) + &"</a>".repeat(MAX_DOCUMENT_DEPTH);
        assert!(!xml_nesting_exceeds(&ok, MAX_DOCUMENT_DEPTH));
    }
}
