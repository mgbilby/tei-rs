//! A focused XPath 2.0 evaluator over `roxmltree` documents, for the
//! Schematron rules TEI schemas embed (`queryBinding="xslt2"`).
//!
//! It covers path expressions on every axis except `namespace`, predicates,
//! general/value/node comparisons, arithmetic, `if`, `for`, `some`/`every`,
//! `instance of`, `cast`/`castable` to the string, boolean and numeric types,
//! sequences, and most of the XPath 2.0 string, sequence, numeric and node
//! functions (plus XSLT's `current()`, and `doc-available()` for local
//! files), with XPath regular expressions
//! translated for the `regex` crate. Anything else — dates and durations,
//! `doc()`, schema types, back-references in regular
//! expressions, XPath 3 syntax — is rejected when the expression is
//! compiled, so callers can report it as unsupported instead of guessing.
//!
//! Values follow XPath 2.0 typing, simplified: no schema types (node values
//! are `xs:untypedAtomic`), and `xs:decimal`/`xs:float` are approximated
//! by doubles.

mod ast;
mod eval;
mod functions;
mod parse;
mod regex;
mod value;

use std::fmt;

pub(crate) use ast::{Axis, Expr, NodeTest, Pattern};
pub(crate) use eval::{Document, Evaluator, Focus};
pub(crate) use parse::{CompileError, StaticContext, parse_expr, parse_pattern};
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
