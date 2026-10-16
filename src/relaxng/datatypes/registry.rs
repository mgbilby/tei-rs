//! The plugin surface: a [`DatatypeLibrary`] trait one `datatypeLibrary` URI
//! implements, and a [`DatatypeRegistry`] mapping URIs to implementations
//! that [`crate::relaxng::validate`] takes as a parameter.
//!
//! [`DatatypeRegistry::new`] always has the RELAX NG built-in (empty-string)
//! library and the XSD library (`http://www.w3.org/2001/XMLSchema-datatypes`)
//! registered — see the upstream project's design notes (Phase 06) for why both are
//! treated as "this crate natively supports them" rather than something a
//! caller must supply. Anything else (e.g. vnu's own
//! `http://whattf.org/datatype-draft`) is the caller's responsibility to
//! implement and [`DatatypeRegistry::register`] — this crate has no
//! knowledge of any non-standard library's types.

use std::collections::BTreeMap;
use std::fmt;

use super::{builtin, xsd};

/// The namespace URI identifying the XSD datatype library, as used in
/// `datatypeLibrary="..."` — see the upstream project's design notes.
pub const XSD_DATATYPE_LIBRARY: &str = "http://www.w3.org/2001/XMLSchema-datatypes";

/// The in-scope XML namespace prefix bindings at the point a value occurs
/// in the *document* (not the schema) — needed to resolve `QName`/
/// `NOTATION`-typed values, whose lexical form (`prefix:local`) only
/// denotes a value together with a prefix-to-URI binding. Built once per
/// element from [`crate::relaxng::Element::namespace_bindings`].
#[derive(Clone, Debug, Default)]
pub struct DatatypeContext {
    bindings: Vec<(String, String)>,
}

impl DatatypeContext {
    pub(crate) const fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub(crate) fn from_bindings(bindings: Vec<(String, String)>) -> Self {
        Self { bindings }
    }

    /// The namespace URI bound to `prefix` (`""` for the default
    /// namespace), or `None` if no such binding is in scope. Later
    /// bindings win over earlier ones, matching normal XML shadowing.
    pub fn resolve(&self, prefix: &str) -> Option<&str> {
        self.bindings
            .iter()
            .rev()
            .find(|(bound_prefix, _)| bound_prefix == prefix)
            .map(|(_, uri)| uri.as_str())
    }
}

/// A `datatypeLibrary` implementation: recognizes a set of type names,
/// each with its own applicable parameters (facets, in XSD's terminology).
///
/// Modeled on the datatype interface the RELAX NG spec itself describes
/// (type validity, parameter validity, value matching, value equality) —
/// independently designed for Rust, not ported from any existing
/// implementation.
///
/// ```
/// use teixml::relaxng::{DatatypeContext, DatatypeLibrary, DatatypeRegistry};
///
/// /// `urn:example:datatypes`: one type, `even`, for even integers.
/// struct Even;
///
/// impl DatatypeLibrary for Even {
///     fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String> {
///         match (type_name, params) {
///             ("even", []) => Ok(()),
///             ("even", _) => Err("`even` takes no parameters".into()),
///             _ => Err(format!("unknown type `{type_name}`")),
///         }
///     }
///     fn matches(&self, _: &str, _: &[(&str, &str)], value: &str, _: &DatatypeContext) -> bool {
///         value.trim().parse::<i64>().is_ok_and(|n| n % 2 == 0)
///     }
///     fn values_equal(
///         &self,
///         _: &str,
///         expected: &str,
///         _: &DatatypeContext,
///         actual: &str,
///         _: &DatatypeContext,
///     ) -> bool {
///         expected.trim().parse::<i64>().ok() == actual.trim().parse::<i64>().ok()
///     }
///     fn is_context_dependent(&self, _: &str) -> bool {
///         false
///     }
/// }
///
/// let mut registry = DatatypeRegistry::new();
/// registry.register("urn:example:datatypes", Even);
/// // Pass `registry` to `teixml::relaxng::Schema::validate` for schemas
/// // declaring `datatypes ex = "urn:example:datatypes"` and using `ex:even`.
/// ```
pub trait DatatypeLibrary: Send + Sync {
    /// Checks that `type_name` is known to this library and that `params`
    /// are individually well-formed and applicable to it (e.g. XSD's
    /// `pattern` facet takes a regular expression; `minInclusive` only
    /// applies to ordered types). Called once per `data`/`value` pattern,
    /// before any document is validated, so a bad schema fails fast with a
    /// message identifying the problem.
    fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String>;

    /// Whether `value` is a legal value of `type_name` (already checked
    /// valid via [`Self::validate_params`]) satisfying every param. Schema
    /// `except` branches are handled by the caller, not here.
    fn matches(
        &self,
        type_name: &str,
        params: &[(&str, &str)],
        value: &str,
        context: &DatatypeContext,
    ) -> bool;

    /// Whether `expected` (the schema's `<value>` literal, resolved
    /// against `expected_context` — the schema XML's *own* in-scope
    /// namespace bindings and its `ns` attribute, for `QName`/`NOTATION`-
    /// like types) and `actual` (the document's text, resolved against
    /// `actual_context` — the document's bindings at that point) denote
    /// the same value of `type_name`. The two contexts are deliberately
    /// separate: a `<value>` literal's prefixes are the *schema* author's,
    /// never the validated document's.
    fn values_equal(
        &self,
        type_name: &str,
        expected: &str,
        expected_context: &DatatypeContext,
        actual: &str,
        actual_context: &DatatypeContext,
    ) -> bool;

    /// Whether [`Self::matches`] or [`Self::values_equal`] can depend on the
    /// [`DatatypeContext`] for `type_name` (as for XSD's `QName`). If not,
    /// the validator may reuse a result for the same value elsewhere in the
    /// document. Defaults to `true`, which is always correct.
    fn is_context_dependent(&self, type_name: &str) -> bool {
        let _ = type_name;
        true
    }
}

/// The schema uses a `datatypeLibrary`/type/parameter this
/// [`DatatypeRegistry`] can't handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatatypeError {
    library: String,
    message: String,
}

impl DatatypeError {
    pub(crate) fn unsupported_library(library: impl Into<String>) -> Self {
        let library = library.into();
        Self {
            message: format!("unsupported datatype library `{library}`"),
            library,
        }
    }

    pub(crate) fn type_problem(library: impl Into<String>, type_name: &str, reason: &str) -> Self {
        let library = library.into();
        Self {
            message: format!("datatype `{type_name}` in library `{library}`: {reason}"),
            library,
        }
    }

    /// The offending `datatypeLibrary` URI (the empty string denotes the
    /// built-in library).
    pub fn library(&self) -> &str {
        &self.library
    }
}

impl fmt::Display for DatatypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DatatypeError {}

/// Maps `datatypeLibrary` URIs to [`DatatypeLibrary`] implementations for
/// one [`crate::relaxng::validate`] call.
///
/// ```
/// use teixml::relaxng::{DatatypeRegistry, Schema, SchemaSource, SchemaSyntax};
/// # use teixml::relaxng::{ResolveError, SchemaResolver};
/// # struct NoIncludes;
/// # impl SchemaResolver for NoIncludes {
/// #     fn resolve(&self, href: &str, _: &str) -> Result<SchemaSource, ResolveError> {
/// #         Err(ResolveError::new(href))
/// #     }
/// # }
/// # struct Leaf(&'static str);
/// # impl teixml::relaxng::Element for Leaf {
/// #     type Location = ();
/// #     fn name(&self) -> teixml::relaxng::ExpandedName {
/// #         teixml::relaxng::ExpandedName::new(None::<String>, "n")
/// #     }
/// #     fn attributes(&self) -> impl Iterator<Item = (teixml::relaxng::ExpandedName, String)> {
/// #         std::iter::empty()
/// #     }
/// #     fn children(&self) -> impl Iterator<Item = teixml::relaxng::Content<Self>> {
/// #         std::iter::once(teixml::relaxng::Content::Text(self.0.to_owned()))
/// #     }
/// # }
/// let source = SchemaSource::new("element n { xsd:integer }", "n.rnc", SchemaSyntax::Compact);
/// let schema = Schema::compile(&source, &NoIncludes)?;
///
/// // The default registry knows XSD types...
/// assert!(schema.validate(&DatatypeRegistry::new(), &Leaf("42"))?.is_empty());
/// // ...a built-in-only one refuses to validate rather than guess.
/// assert!(schema.validate(&DatatypeRegistry::builtin_only(), &Leaf("42")).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct DatatypeRegistry {
    libraries: BTreeMap<String, Box<dyn DatatypeLibrary>>,
}

impl DatatypeRegistry {
    /// A registry with the RELAX NG built-in library (`""`), the XSD library
    /// and the DTD Compatibility library (`ID`, `IDREF`, `IDREFS`)
    /// registered — the three this crate implements itself. Use
    /// [`Self::builtin_only`] to reject `xsd:`-typed schemas explicitly, or
    /// [`Self::register`] to add further libraries (e.g. a caller's own
    /// non-standard one).
    pub fn new() -> Self {
        let mut registry = Self::builtin_only();
        registry.register(XSD_DATATYPE_LIBRARY, xsd::XsdLibrary);
        registry.register(
            super::COMPATIBILITY_DATATYPE_LIBRARY,
            super::compatibility::CompatibilityLibrary,
        );
        registry
    }

    /// A registry with only the mandatory RELAX NG built-in library (`""`)
    /// registered — every other `datatypeLibrary`, including XSD, fails
    /// fast with [`DatatypeError`].
    pub fn builtin_only() -> Self {
        let mut libraries: BTreeMap<String, Box<dyn DatatypeLibrary>> = BTreeMap::new();
        libraries.insert(String::new(), Box::new(builtin::BuiltinLibrary));
        Self { libraries }
    }

    /// Registers `library` under `library_uri`, replacing any previous
    /// registration for that URI (including the built-in ones, should a
    /// caller want to override them — not recommended, but not prevented).
    pub fn register(
        &mut self,
        library_uri: impl Into<String>,
        library: impl DatatypeLibrary + 'static,
    ) -> &mut Self {
        self.libraries.insert(library_uri.into(), Box::new(library));
        self
    }

    pub(crate) fn get(&self, library_uri: &str) -> Option<&dyn DatatypeLibrary> {
        self.libraries.get(library_uri).map(Box::as_ref)
    }
}

impl Default for DatatypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
