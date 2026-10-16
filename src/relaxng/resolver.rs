use std::fmt;

/// Which of RELAX NG's two equivalent syntaxes a [`SchemaSource`]'s text is
/// written in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaSyntax {
    /// The compact (non-XML) syntax, conventionally using a `.rnc` file
    /// extension.
    Compact,
    /// The XML syntax, conventionally using a `.rng` file extension.
    Xml,
}

/// Schema text plus enough context (a base URI, and which syntax it's
/// written in) to parse it and resolve any `include`/`externalRef` it
/// contains. This crate never reads files or network resources itself —
/// callers supply schema text directly, and a [`SchemaResolver`] for
/// anything the schema references by `href`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaSource {
    text: String,
    base_uri: String,
    syntax: SchemaSyntax,
}

impl SchemaSource {
    /// Builds a source from its text, base URI (used to resolve any
    /// relative `href`s the schema references), and syntax.
    pub fn new(text: impl Into<String>, base_uri: impl Into<String>, syntax: SchemaSyntax) -> Self {
        Self {
            text: text.into(),
            base_uri: base_uri.into(),
            syntax,
        }
    }

    /// The schema's raw text.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// The base URI relative `href`s in this schema resolve against.
    pub fn base_uri(&self) -> &str {
        &self.base_uri
    }
    /// Which syntax [`Self::text`] is written in.
    pub fn syntax(&self) -> SchemaSyntax {
        self.syntax
    }
}

/// Resolves an `include`/`externalRef` `href` (relative to a base URI) to
/// the [`SchemaSource`] it names. This crate has no file-system or
/// network access of its own — implement this over whatever loading
/// mechanism (files, an in-memory map, HTTP, ...) fits the caller, with
/// whatever security boundary (allowed schemes, path restrictions) is
/// appropriate there.
///
/// ```
/// use std::collections::HashMap;
/// use teixml::relaxng::{ResolveError, Schema, SchemaResolver, SchemaSource, SchemaSyntax};
///
/// /// Schemas held in memory, by name.
/// struct InMemory(HashMap<&'static str, &'static str>);
///
/// impl SchemaResolver for InMemory {
///     fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
///         let text = self.0.get(href).ok_or_else(|| ResolveError::new(format!("no {href}")))?;
///         Ok(SchemaSource::new(*text, href, SchemaSyntax::Compact))
///     }
/// }
///
/// let resolver = InMemory(HashMap::from([("names.rnc", "name = element name { text }")]));
/// let main = SchemaSource::new(
///     "include \"names.rnc\"\nstart = element person { name+ }",
///     "main.rnc",
///     SchemaSyntax::Compact,
/// );
/// let schema = Schema::compile(&main, &resolver)?;
/// # let _ = schema;
///
/// let missing = SchemaSource::new("include \"other.rnc\"", "main.rnc", SchemaSyntax::Compact);
/// assert!(Schema::compile(&missing, &resolver).is_err());
/// # Ok::<(), teixml::relaxng::CompileError>(())
/// ```
pub trait SchemaResolver {
    /// Resolves `href` (as it literally appears in the schema) relative to
    /// `base_uri`, or fails with a [`ResolveError`] if it can't be found,
    /// read, or is disallowed.
    fn resolve(&self, href: &str, base_uri: &str) -> Result<SchemaSource, ResolveError>;
}

/// A [`SchemaResolver`] failed to resolve an `href`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolveError {
    message: String,
}

impl ResolveError {
    /// Builds a `ResolveError` with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ResolveError {}
