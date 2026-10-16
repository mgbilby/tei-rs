//! A focused XPath 2.0 evaluator over `roxmltree` documents, for the
//! Schematron rules TEI schemas embed (`queryBinding="xslt2"`).
//!
//! It covers path expressions on every axis except `namespace`, predicates,
//! general/value/node comparisons, arithmetic, `if`, `for`, `some`/`every`,
//! `instance of`, `cast`/`castable` to the string, boolean, numeric, date,
//! time and duration types, sequences, and most of the XPath 2.0 string,
//! sequence, numeric, node, date and duration functions (plus XSLT's
//! `current()` and `document()`), with XPath regular expressions translated
//! for the `regex` crate. `doc()`, `document()` and `doc-available()` read
//! local files and, for `http(s)` URIs, the schema store, never the network
//! (see `resources`). As Saxon runs XSLT 2.0 stylesheets as XSLT 3.0, it
//! also accepts the XPath 3.0 operators `!`, `||`, `let` and `=>`, a few
//! XPath 3.0 functions (`has-children()`, `innermost()`, `outermost()`,
//! `path()`), and XSLT 3.0 match patterns with parenthesised unions as
//! steps. Anything else — schema types, back-references in regular
//! expressions, other XPath 3 syntax and functions — is rejected when the
//! expression is compiled, so callers can report it as unsupported instead
//! of guessing.
//!
//! Values follow XPath 2.0 typing, simplified: no schema types (node values
//! are `xs:untypedAtomic`), and `xs:decimal`/`xs:float` are approximated
//! by doubles. Dates follow XML Schema 1.1 as Saxon does (there is a year
//! 0), the implicit timezone is UTC, and `current-dateTime()` is read once
//! per document. `base-uri()` is the document's URI resolved against any
//! `xml:base`. A regular expression that isn't valid XPath is an error when
//! it is evaluated, as in Saxon.

mod ast;
mod datetime;
mod eval;
mod functions;
mod parse;
mod regex;
mod resources;
mod value;

use std::fmt;

pub(crate) use ast::{Axis, Expr, NodeTest, Pattern};
pub(crate) use eval::{Document, Evaluator, Focus};
pub(crate) use parse::{CompileError, StaticContext, parse_expr, parse_pattern};
pub(crate) use resources::{Access, Resources, file_uri, literal_documents, static_base};
pub(crate) use value::{Item, XNode, collapse_whitespace};

/// A dynamic error raised while evaluating an expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct XError {
    pub code: &'static str,
    pub message: String,
}

impl XError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for XError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

#[cfg(test)]
mod tests;
