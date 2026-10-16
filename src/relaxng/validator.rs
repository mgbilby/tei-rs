//! Validates a document (via the generic [`Element`] trait) against a
//! [`CompiledSchema`](crate::relaxng::CompiledSchema) using Brzozowski-derivative
//! pattern matching — the standard approach for RELAX NG validators,
//! derived independently from the pattern algebra in the spec (not ported
//! from any other implementation; see the upstream project's design notes).
//!
//! # The derivative idea
//!
//! A pattern describes the set of "things" (child sequences, attribute
//! sets, text values) it accepts. Given one such thing has just been seen
//! (an attribute, a child element, a run of text), the *derivative* of the
//! pattern with respect to it is a new pattern describing what must still
//! follow. A pattern is *nullable* if it accepts having nothing more
//! follow at all. Validating a whole element reduces to: derive its
//! content pattern through every attribute (unordered) and then every
//! child in order, and check the result is nullable.
//!
//! Patterns are hash-consed into a [`PatternStore`] (see
//! `crate::relaxng::interned`), and derivatives that don't depend on an
//! attribute value or a text are cached per pattern, so repeated content
//! models cost one lookup instead of a fresh derivation.
//!
//! Every `ref` is transparent here — including inside `interleave` — since
//! interning replaces it by its definition. That is deliberate: the most serious bug found
//! in the `xmloxide` RELAX NG parser (the reason this crate exists at all,
//! see the upstream project's design notes) was exactly a `ref` inside `interleave` not
//! being resolved for attribute matching, rejecting almost every attribute
//! in a modular schema. [`attribute_matches_through_ref_and_interleave`]
//! (in the test module) is a standing regression test for that failure
//! mode.
//!
//! # Datatypes
//!
//! `data`/`value` matching is delegated to a [`DatatypeRegistry`] the
//! caller supplies to [`validate`] — this module never hardcodes datatype
//! semantics itself (see `crate::relaxng::datatypes`). Every `data`/`value`
//! reachable in the schema is checked against the registry once, upfront,
//! before any document is touched, so an unregistered `datatypeLibrary` or
//! an invalid facet fails fast with a single, clear [`DatatypeError`]
//! rather than partway through validation.
use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;

use crate::relaxng::CompiledSchema;
use crate::relaxng::datatypes::{DatatypeContext, DatatypeError, DatatypeRegistry};
use crate::relaxng::grammar::{Context, Param, Pattern};
use crate::relaxng::interned::{
    Definitions, EMPTY, FastMap, Id, Leaf, NOT_ALLOWED, Node, PatternStore, TEXT,
};
use crate::relaxng::simplify::name_class_matches;

/// An expanded (namespace-resolved) name: local name plus an optional
/// namespace URI. `None` means "no namespace" — the caller's document
/// implementation is responsible for resolving any prefixes itself (this
/// crate has no XML/namespace parser of its own).
///
/// ```
/// use teixml::relaxng::ExpandedName;
///
/// let name = ExpandedName::new(Some("http://www.tei-c.org/ns/1.0"), "p");
/// assert_eq!(name.to_string(), "{http://www.tei-c.org/ns/1.0}p");
/// assert_eq!(ExpandedName::new(None::<String>, "p").to_string(), "p");
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ExpandedName {
    /// The namespace URI, or `None` for "no namespace".
    pub namespace: Option<String>,
    /// The local (unprefixed) name.
    pub local: String,
}

impl ExpandedName {
    /// Builds an expanded name from a namespace (`None` for "no
    /// namespace") and a local name.
    pub fn new(namespace: Option<impl Into<String>>, local: impl Into<String>) -> Self {
        Self {
            namespace: namespace.map(Into::into),
            local: local.into(),
        }
    }
}

impl fmt::Display for ExpandedName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(namespace) => write!(formatter, "{{{namespace}}}{}", self.local),
            None => formatter.write_str(&self.local),
        }
    }
}

/// One child of an element, in document order. Comments and processing
/// instructions are not part of this model — a correct `Element`
/// implementation simply never yields them here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Content<E> {
    /// A child element.
    Element(E),
    /// A single, non-empty, already-merged run of text — if two text
    /// nodes are adjacent in the underlying document (however "adjacent"
    /// is defined there, e.g. around a comment), the implementation must
    /// merge them into one `Text` before they reach here.
    Text(String),
}

/// The minimal view of a document element this crate needs to validate it.
/// Implement this over whatever tree type the caller already has — no
/// document is parsed by this crate itself.
pub trait Element: Sized {
    /// Whatever shape of location this implementation can report per
    /// element — a formatted `String` (`"line:column"` or an XPath-like
    /// pointer), a structured `{line, column, byte_offset}` struct the
    /// caller defines, or `()` for callers with no location tracking at
    /// all. Carried through unchanged into any [`ValidationError`] raised
    /// against the element, so callers with real position data don't
    /// have to re-parse a formatted string to get it back out — see
    /// [`ValidationError::location`].
    type Location;
    /// This element's expanded name.
    fn name(&self) -> ExpandedName;
    /// This element's attributes, in any order (attribute order is never
    /// significant in XML, and RELAX NG never treats it as such either).
    /// `xmlns`/`xmlns:*` namespace declarations are not attributes for
    /// RELAX NG's purposes and must not be yielded here.
    fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)>;
    /// This element's children, in document order.
    fn children(&self) -> impl Iterator<Item = Content<Self>>;
    /// An optional location, included in any [`ValidationError`] raised
    /// against this element when available.
    fn location(&self) -> Option<Self::Location> {
        None
    }
    /// Where the element ends, used for problems only detectable there
    /// ([`ValidationErrorKind::MissingContent`]). Default: none, in which
    /// case [`Self::location`] is used.
    fn end_location(&self) -> Option<Self::Location> {
        None
    }
    /// Where the element's start tag begins. Text that isn't allowed is
    /// reported where it ends: at the next child element's start, or at
    /// [`Self::end_tag_location`] for text at the end of an element.
    /// Default: none, in which case the parent's [`Self::location`] is used.
    fn start_location(&self) -> Option<Self::Location> {
        None
    }
    /// Where the element's end tag begins. Default: none.
    fn end_tag_location(&self) -> Option<Self::Location> {
        None
    }
    /// The XML namespace prefix bindings in scope at this element
    /// (`("", uri)` for the default namespace), including any inherited
    /// from ancestors — needed only to resolve `QName`/`NOTATION`-typed
    /// attribute/text *values* (not element/attribute names, which arrive
    /// already-expanded via [`Self::name`]). Default: no bindings, so
    /// `QName`/`NOTATION` values only match when unprefixed and the
    /// default namespace (if any) is what the schema expects.
    fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
        std::iter::empty()
    }
}

/// Validates `root` against `schema` using `registry` to check `data`/
/// `value` patterns. Returns the list of problems found (empty means
/// valid) — or, if the schema uses a `datatypeLibrary`/type/parameter
/// `registry` doesn't recognize, a [`DatatypeError`] *instead of*
/// attempting validation at all, since we can't know whether the document
/// would pass a check we can't perform.
pub fn validate<E: Element>(
    schema: &CompiledSchema,
    registry: &DatatypeRegistry,
    root: &E,
) -> Result<Vec<ValidationError<E::Location>>, DatatypeError> {
    let mut checked_refs = BTreeSet::new();
    check_datatypes_supported(
        &schema.start,
        &schema.definitions,
        registry,
        &mut checked_refs,
    )?;
    for pattern in schema.definitions.values() {
        check_datatypes_supported(pattern, &schema.definitions, registry, &mut checked_refs)?;
    }

    let mut validator = Validator::new(&schema.definitions, registry);
    let mut errors = Vec::new();
    let mut path = PathStack::default();
    let start = validator.store.intern(&schema.start);
    let root_name = root.name();
    let symbol = validator.symbol(&root_name);
    match validator.find_element_contents(start, symbol, &root_name) {
        Some(content) => {
            validator.validate_element_content(content, root, root_name, &mut path, &mut errors);
        }
        None => errors.push(ValidationError {
            kind: ValidationErrorKind::UnexpectedElement(root_name),
            path: ElementPath::default(),
            location: root.location(),
        }),
    }
    Ok(errors)
}

/// A single validation problem, with the path of element names from the
/// document root down to (and including) the element it occurred on, and
/// an optional location if [`Element::location`] provided one.
///
/// Generic over `L` (== the reporting [`Element`] implementation's own
/// [`Element::Location`]) rather than hardcoding a formatted `String`, so
/// a caller with real per-node position data (line/column/byte offset,
/// or anything else) gets it back out structured, not as text to
/// re-parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError<L> {
    kind: ValidationErrorKind,
    path: ElementPath,
    location: Option<L>,
}

/// The names of the elements from the document root down to one element.
/// Cheap to clone: errors on the same element, and on elements with the
/// same ancestors, share the names.
///
/// ```
/// use teixml::ElementPath;
/// use teixml::relaxng::ExpandedName;
///
/// let tei = |local| ExpandedName::new(Some("http://www.tei-c.org/ns/1.0"), local);
/// let text = ElementPath::from_names([tei("TEI"), tei("text")]);
/// let body = text.child(tei("body"));
///
/// assert_eq!(body.to_string(), "/TEI/text/body");
/// assert_eq!(body.names().len(), 3);
/// assert_eq!(body.last().map(|name| name.local.as_str()), Some("body"));
/// assert_eq!(text.to_string(), "/TEI/text"); // unchanged by `child`
/// assert!(ElementPath::default().is_empty());
/// ```
#[derive(Clone, Default)]
pub struct ElementPath(Option<std::sync::Arc<PathNode>>);

struct PathNode {
    name: ExpandedName,
    parent: ElementPath,
}

impl ElementPath {
    /// A path from the names, root first.
    pub fn from_names(names: impl IntoIterator<Item = ExpandedName>) -> Self {
        names
            .into_iter()
            .fold(Self::default(), |path, name| path.child(name))
    }

    /// This path extended by a child element.
    pub fn child(&self, name: ExpandedName) -> Self {
        Self(Some(std::sync::Arc::new(PathNode {
            name,
            parent: self.clone(),
        })))
    }

    /// The names, root first.
    pub fn names(&self) -> Vec<&ExpandedName> {
        let mut names = Vec::new();
        let mut path = self;
        while let Some(node) = &path.0 {
            names.push(&node.name);
            path = &node.parent;
        }
        names.reverse();
        names
    }

    /// The innermost element's name.
    pub fn last(&self) -> Option<&ExpandedName> {
        self.0.as_ref().map(|node| &node.name)
    }

    /// Whether the path is empty (the problem isn't on any element).
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

/// `/TEI/text/body`: the local names, root first.
impl fmt::Display for ElementPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for name in self.names() {
            formatter.write_str("/")?;
            formatter.write_str(&name.local)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ElementPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.names()).finish()
    }
}

impl PartialEq for ElementPath {
    fn eq(&self, other: &Self) -> bool {
        self.names() == other.names()
    }
}

impl Eq for ElementPath {}

/// The path to the element being validated: names on a stack, turned into
/// an [`ElementPath`] only for elements that have an error.
#[derive(Default)]
struct PathStack {
    names: Vec<ExpandedName>,
    shared: Vec<Option<ElementPath>>,
}

impl PathStack {
    fn push(&mut self, name: ExpandedName) {
        self.names.push(name);
        self.shared.push(None);
    }

    fn pop(&mut self) {
        self.names.pop();
        self.shared.pop();
    }

    fn current(&mut self) -> ElementPath {
        let depth = self.names.len();
        let mut built = depth;
        while built > 0 && self.shared[built - 1].is_none() {
            built -= 1;
        }
        let mut path = match built {
            0 => ElementPath::default(),
            _ => self.shared[built - 1].clone().unwrap_or_default(),
        };
        for index in built..depth {
            path = path.child(self.names[index].clone());
            self.shared[index] = Some(path.clone());
        }
        path
    }
}

impl<L> ValidationError<L> {
    /// What kind of problem this is.
    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }
    /// The element names from the document root down to (and including)
    /// the element this problem occurred on.
    pub fn path(&self) -> &ElementPath {
        &self.path
    }
    /// The location [`Element::location`] provided for the affected
    /// element, if any.
    pub fn location(&self) -> Option<&L> {
        self.location.as_ref()
    }
}

impl<L: fmt::Display> fmt::Display for ValidationError<L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.kind)?;
        if let Some(location) = &self.location {
            write!(formatter, " at {location}")?;
        }
        Ok(())
    }
}

impl<L: fmt::Debug + fmt::Display> std::error::Error for ValidationError<L> {}

/// What kind of problem a [`ValidationError`] describes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationErrorKind {
    /// An element with this name isn't allowed at this position.
    UnexpectedElement(ExpandedName),
    /// A text child isn't allowed at this position (and isn't part of a
    /// `data`/`value`/`list` that could accept it).
    UnexpectedText,
    /// This attribute name isn't allowed on this element at all.
    UnexpectedAttribute(ExpandedName),
    /// This attribute is allowed on this element, but its value doesn't
    /// match what the schema requires.
    InvalidAttributeValue(ExpandedName, String),
    /// The element ended while the schema still required more content
    /// (a missing child element, missing text, or a missing attribute).
    MissingContent,
    /// An `ID`-typed attribute repeats a value an earlier one has.
    DuplicateId(String),
    /// Where an ID reported as [`Self::DuplicateId`] was first defined.
    FirstIdOccurrence(String),
    /// An `IDREF`/`IDREFS`-typed attribute names an ID no element has.
    UnresolvedIdref(String),
}

/// A name as diagnostics show it: the local name, like Jing. An attribute in
/// the XML namespace gets its reserved `xml:` prefix, and one in another
/// namespace keeps the `{namespace}local` form.
struct Readable<'a>(&'a ExpandedName, bool);

impl fmt::Display for Readable<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Readable(name, is_element) = *self;
        match name.namespace.as_deref() {
            _ if is_element => write!(formatter, "`{}`", name.local),
            None | Some("") => write!(formatter, "`{}`", name.local),
            Some("http://www.w3.org/XML/1998/namespace") => {
                write!(formatter, "`xml:{}`", name.local)
            }
            Some(_) => write!(formatter, "`{name}`"),
        }
    }
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedElement(name) => {
                write!(formatter, "unexpected element {}", Readable(name, true))
            }
            Self::UnexpectedText => formatter.write_str("unexpected text"),
            Self::UnexpectedAttribute(name) => {
                write!(formatter, "unexpected attribute {}", Readable(name, false))
            }
            Self::InvalidAttributeValue(name, value) => {
                write!(
                    formatter,
                    "invalid value `{value}` for attribute {}",
                    Readable(name, false)
                )
            }
            Self::MissingContent => formatter.write_str("missing required content"),
            Self::DuplicateId(id) => write!(formatter, "ID `{id}` has already been defined"),
            Self::FirstIdOccurrence(id) => write!(formatter, "first occurrence of ID `{id}`"),
            Self::UnresolvedIdref(id) => write!(formatter, "IDREF `{id}` without matching ID"),
        }
    }
}

/// Where a run of text in `parent` ends: at the start of the child element
/// that follows it, or of the parent's end tag; Jing reports text there.
fn text_end<E: Element>(parent: &E, next: Option<&Content<E>>) -> Option<E::Location> {
    match next {
        Some(Content::Element(element)) => element.start_location(),
        _ => parent.end_tag_location(),
    }
    .or_else(|| parent.location())
}

/// Whether `text` consists only of XML whitespace (space, tab, CR, LF).
fn is_xml_whitespace(text: &str) -> bool {
    text.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}

fn param_pairs(params: &[Param]) -> Vec<(&str, &str)> {
    params
        .iter()
        .map(|param| (param.name.as_str(), param.value.as_str()))
        .collect()
}

/// The namespace-resolution context for a `value` pattern's *own* literal
/// text, as written in the schema — used as the "expected" side of
/// [`DatatypeLibrary::values_equal`] for `QName`/`NOTATION`-like types.
/// Deliberately built from the schema, never the document: `namespace` is
/// §4.9's (already-inherited) `ns` attribute, used to resolve the literal
/// if it's unprefixed; `schema_context.namespaces` are the schema XML's
/// own in-scope `xmlns:` bindings, used to resolve it if it has a prefix.
fn schema_value_context(namespace: &Option<String>, schema_context: &Context) -> DatatypeContext {
    let mut bindings: Vec<(String, String)> = schema_context
        .namespaces
        .iter()
        .map(|(prefix, uri)| (prefix.clone(), uri.clone()))
        .collect();
    bindings.push((String::new(), namespace.clone().unwrap_or_default()));
    DatatypeContext::from_bindings(bindings)
}

/// Every `data`/`value` reachable in the schema must use a `datatypeLibrary`
/// `registry` has an implementation for, with a known type name and
/// well-formed, applicable parameters — checked once, upfront, over the
/// *whole* schema (not lazily as validation happens to reach one), so the
/// caller gets a clear, consistent error regardless of which document path
/// would have triggered it.
fn check_datatypes_supported(
    pattern: &Pattern,
    definitions: &Definitions,
    registry: &DatatypeRegistry,
    checked_refs: &mut BTreeSet<String>,
) -> Result<(), DatatypeError> {
    match pattern {
        Pattern::Data {
            datatype,
            datatype_library,
            params,
            except,
            ..
        } => {
            let library = datatype_library.as_deref().unwrap_or("");
            let implementation = registry
                .get(library)
                .ok_or_else(|| DatatypeError::unsupported_library(library))?;
            implementation
                .validate_params(datatype, &param_pairs(params))
                .map_err(|reason| DatatypeError::type_problem(library, datatype, &reason))?;
            for child in except {
                check_datatypes_supported(child, definitions, registry, checked_refs)?;
            }
            Ok(())
        }
        Pattern::Value {
            datatype,
            datatype_library,
            ..
        } => {
            let library = datatype_library.as_deref().unwrap_or("");
            let type_name = datatype.as_deref().unwrap_or("token");
            let implementation = registry
                .get(library)
                .ok_or_else(|| DatatypeError::unsupported_library(library))?;
            implementation
                .validate_params(type_name, &[])
                .map_err(|reason| DatatypeError::type_problem(library, type_name, &reason))
        }
        Pattern::Element { body, .. } | Pattern::Attribute { body, .. } => {
            body.iter().try_for_each(|child| {
                check_datatypes_supported(child, definitions, registry, checked_refs)
            })
        }
        Pattern::Group(values)
        | Pattern::Interleave(values)
        | Pattern::Choice(values)
        | Pattern::OneOrMore(values)
        | Pattern::List(values) => values.iter().try_for_each(|child| {
            check_datatypes_supported(child, definitions, registry, checked_refs)
        }),
        Pattern::Ref(name) => {
            if checked_refs.insert(name.clone())
                && let Some(target) = definitions.get(name)
            {
                check_datatypes_supported(target, definitions, registry, checked_refs)?;
            }
            Ok(())
        }
        Pattern::Empty | Pattern::NotAllowed | Pattern::Text => Ok(()),
        Pattern::Optional(_)
        | Pattern::ZeroOrMore(_)
        | Pattern::Mixed(_)
        | Pattern::ExternalRef { .. }
        | Pattern::ParentRef(_)
        | Pattern::Grammar(_) => {
            unreachable!("resolved into other patterns by simplify_pattern")
        }
    }
}

/// The context for a value that can't depend on namespace bindings.
static NO_BINDINGS: DatatypeContext = DatatypeContext::empty();

/// An interned element or attribute name, so cache keys stay small.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Symbol(u32);

/// One event a pattern is derived by.
#[derive(Clone, Copy)]
enum Event<'e> {
    /// An attribute and its value.
    Attribute {
        name: &'e ExpandedName,
        symbol: Symbol,
        value: &'e str,
        context: &'e DatatypeContext,
    },
    /// A child element, by name only — its own content is validated
    /// separately and recursively, see [`Validator::find_element_contents`].
    Element {
        name: &'e ExpandedName,
        symbol: Symbol,
    },
    /// A text value: a child `Content::Text` (the whole merged run of text)
    /// or, from [`Validator::list_matches`], one token of a `list`.
    Text {
        text: &'e str,
        context: &'e DatatypeContext,
    },
}

/// A cache of one boolean property per pattern id.
#[derive(Default)]
struct Flags(Vec<Option<bool>>);

impl Flags {
    fn get(&self, id: Id) -> Option<bool> {
        self.0.get(id.index()).copied().flatten()
    }

    fn set(&mut self, id: Id, value: bool) -> bool {
        if self.0.len() <= id.index() {
            self.0.resize(id.index() + 1, None);
        }
        self.0[id.index()] = Some(value);
        value
    }
}

/// The derivative machinery for one validation run: the store of
/// hash-consed patterns plus caches of everything about them that doesn't
/// depend on a particular attribute value or text. All of these are pure
/// functions of their keys, so caching never changes a result; it only
/// means that, say, the derivative of `<p>`'s content model by `<hi>` is
/// worked out once per document instead of once per occurrence.
struct Validator<'s> {
    store: PatternStore<'s>,
    registry: &'s DatatypeRegistry,
    symbols: FastMap<ExpandedName, Symbol>,
    nullable: Flags,
    structurally_nullable: Flags,
    wants_text: Flags,
    /// Whether a text derivative can depend on the text itself (some
    /// `data`/`value`/`list` is reachable); if not, it is cached by id.
    text_sensitive: Flags,
    /// Whether some reachable `data`/`value` has a datatype whose checks
    /// can depend on the document's namespace bindings; see
    /// [`DatatypeLibrary::is_context_dependent`](crate::relaxng::DatatypeLibrary::is_context_dependent).
    context_dependent: Flags,
    closed: FastMap<Id, Id>,
    element_derivs: FastMap<(Id, Symbol), Id>,
    text_derivs: FastMap<Id, Id>,
    /// Derivatives of an element's content by one of its attributes, by
    /// attribute name and value, for content that isn't
    /// [`Self::context_dependent`]. Attribute values repeat a lot (TEI's
    /// `rend="italic"`), and each derivative otherwise rebuilds the
    /// content's attribute group and rechecks the value's datatype.
    attribute_derivs: FastMap<(Id, Symbol), HashMap<Box<str>, Id>>,
    element_contents: FastMap<(Id, Symbol), Option<Id>>,
    attribute_names: FastMap<(Id, Symbol), bool>,
    /// The one `attribute` pattern in an element's content that an
    /// attribute name can match, if exactly one can.
    attribute_leaves: FastMap<(Id, Symbol), Option<Id>>,
    /// The derivative of an element's content by an attribute whose only
    /// matching pattern ([`Self::attribute_leaves`]) accepts its value:
    /// the same for every accepted value, so unique values (`xml:id`)
    /// don't each need a full derivative.
    accepted_attribute_derivs: FastMap<(Id, Symbol), Id>,
}

impl<'s> Validator<'s> {
    fn new(definitions: &'s Definitions, registry: &'s DatatypeRegistry) -> Self {
        Self {
            store: PatternStore::new(definitions),
            registry,
            symbols: FastMap::default(),
            nullable: Flags::default(),
            structurally_nullable: Flags::default(),
            wants_text: Flags::default(),
            text_sensitive: Flags::default(),
            context_dependent: Flags::default(),
            closed: FastMap::default(),
            element_derivs: FastMap::default(),
            text_derivs: FastMap::default(),
            attribute_derivs: FastMap::default(),
            element_contents: FastMap::default(),
            attribute_names: FastMap::default(),
            attribute_leaves: FastMap::default(),
            accepted_attribute_derivs: FastMap::default(),
        }
    }

    fn symbol(&mut self, name: &ExpandedName) -> Symbol {
        if let Some(&symbol) = self.symbols.get(name) {
            return symbol;
        }
        let symbol = Symbol(u32::try_from(self.symbols.len()).expect("fewer than 2^32 names"));
        self.symbols.insert(name.clone(), symbol);
        symbol
    }

    /// Validates `element` (named `name`) against its content pattern.
    fn validate_element_content<E: Element>(
        &mut self,
        content: Id,
        element: &E,
        name: ExpandedName,
        path: &mut PathStack,
        errors: &mut Vec<ValidationError<E::Location>>,
    ) {
        path.push(name);
        let mut pattern = content;
        // The namespace bindings are needed only to check a value against a
        // datatype, which most elements never do (see `deriv_attribute` and
        // `text_context`), so they are collected on first use.
        let bindings = OnceCell::new();
        let context = || {
            bindings.get_or_init(|| {
                DatatypeContext::from_bindings(element.namespace_bindings().collect())
            })
        };

        // Attributes are unordered — fold the derivative over them in
        // whatever order the caller's iterator happens to yield.
        for (name, value) in element.attributes() {
            let symbol = self.symbol(&name);
            if !self.attribute_name_known(pattern, symbol, &name) {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::UnexpectedAttribute(name),
                    path: path.current(),
                    location: element.location(),
                });
                continue;
            }
            let derived = self.deriv_attribute(pattern, &name, symbol, &value, context);
            if derived == NOT_ALLOWED {
                errors.push(ValidationError {
                    kind: ValidationErrorKind::InvalidAttributeValue(name, value),
                    path: path.current(),
                    location: element.location(),
                });
            } else {
                pattern = derived;
            }
        }

        // Children are ordered. Whitespace-only text follows §6.2.7 as James
        // Clark's derivative algorithm (and Jing) implement it: among other
        // children it is dropped entirely, and as an element's only child it may
        // be either matched or ignored — ignored only if the content model
        // strictly accepts no content. Committing to whichever branch accepts it
        // would wrongly rule out the others, e.g. `choice(group(element a, ...),
        // text)` given whitespace before `<a>`.
        let mut children = element.children().peekable();
        let mut first = true;
        while let Some(child) = children.next() {
            let only_child = first && children.peek().is_none();
            first = false;
            match child {
                Content::Text(text) if is_xml_whitespace(&text) => {
                    if !only_child {
                        continue;
                    }
                    let event = Event::Text {
                        text: &text,
                        context: self.text_context(pattern, context),
                    };
                    let derived = self.deriv(pattern, event);
                    if derived == NOT_ALLOWED {
                        // Nothing can match it: it is insignificant unless the
                        // content model needs text right here.
                        if self.wants_text(pattern) {
                            errors.push(ValidationError {
                                kind: ValidationErrorKind::UnexpectedText,
                                path: path.current(),
                                location: text_end(element, children.peek()),
                            });
                        }
                    } else if self.structurally_nullable(pattern) {
                        pattern = self.store.choice(vec![pattern, derived]);
                    } else {
                        pattern = derived;
                    }
                }
                Content::Text(text) => {
                    let event = Event::Text {
                        text: &text,
                        context: self.text_context(pattern, context),
                    };
                    let derived = self.deriv(pattern, event);
                    if derived == NOT_ALLOWED {
                        errors.push(ValidationError {
                            kind: ValidationErrorKind::UnexpectedText,
                            path: path.current(),
                            location: text_end(element, children.peek()),
                        });
                    } else {
                        pattern = derived;
                    }
                }
                Content::Element(child_element) => {
                    let name = child_element.name();
                    let symbol = self.symbol(&name);
                    match self.find_element_contents(pattern, symbol, &name) {
                        Some(child_content) => {
                            pattern = self.deriv(
                                pattern,
                                Event::Element {
                                    name: &name,
                                    symbol,
                                },
                            );
                            self.validate_element_content(
                                child_content,
                                &child_element,
                                name,
                                path,
                                errors,
                            );
                        }
                        None => errors.push(ValidationError {
                            kind: ValidationErrorKind::UnexpectedElement(name),
                            path: path.current(),
                            location: child_element.location(),
                        }),
                    }
                }
            }
        }

        if !self.nullable(pattern) {
            errors.push(ValidationError {
                kind: ValidationErrorKind::MissingContent,
                path: path.current(),
                location: element.end_location().or_else(|| element.location()),
            });
        }
        path.pop();
    }

    /// Whether `pattern` accepts having nothing more follow — i.e. whether
    /// the content/attributes/text matched *so far* (already folded into
    /// `pattern` via [`Self::deriv`]) are already enough. `data`/`value`
    /// nullability asks whether the registry would accept the implicit
    /// empty string (e.g. the built-in `string`/`token` do; most XSD types
    /// don't).
    fn nullable(&mut self, id: Id) -> bool {
        if let Some(known) = self.nullable.get(id) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Empty | Node::Text => true,
            Node::NotAllowed | Node::Element(_) | Node::Attribute(..) => false,
            Node::Data(Leaf(data), except) => {
                let empty = DatatypeContext::empty();
                self.data_matches(data, &except, "", &empty)
            }
            Node::Value(Leaf(value)) => self.value_equals(value, "", &DatatypeContext::empty()),
            // A `list` with no text child at all behaves like its inner
            // pattern seeing zero tokens — same as `list_matches` below would
            // compute for empty/whitespace-only text, without needing an
            // actual `Content::Text` event to run that logic. Not `nullable`
            // (rule-3-inclusive): a `list` around a bare `data`/`value` slot
            // still needs an actual token, even though that datatype alone
            // would also tolerate the literal empty string — see
            // `structurally_nullable`'s docs.
            Node::List(inner) => self.structurally_nullable(inner),
            Node::Choice(values) => values.iter().any(|&value| self.nullable(value)),
            Node::Group(values) | Node::Interleave(values) => {
                values.iter().all(|&value| self.nullable(value))
            }
            Node::OneOrMore(inner) => self.nullable(inner),
        };
        self.nullable.set(id, result)
    }

    /// Whether `pattern` is satisfied by *zero* events, in the strict
    /// structural sense — unlike [`Self::nullable`], a bare `data`/`value`/
    /// `list` (which structurally requires exactly one text event to occur
    /// at all) is never considered satisfied here just because it happens
    /// to accept the *literal empty string* as a value (that narrower
    /// equivalence is §6.2.7 weak-match rule 3, and belongs only at the true
    /// top level — deciding whether a whole element/attribute that saw *no*
    /// relevant event at all is still valid, which [`Self::nullable`]
    /// already handles).
    ///
    /// This stricter notion is what §6.2.7 weak-match rule 2 actually needs
    /// (dropping a whitespace-only text/attribute event requires the
    /// pattern to *already*, strictly, accept zero events — not "would
    /// accept the empty string if offered one"), what deciding whether a
    /// `list` has enough tokens needs (a `list` with a `data`/`value` slot
    /// no token ever reached is not satisfied just because that slot's
    /// datatype tolerates `""`), and what gates `group`'s sequential "skip a
    /// satisfied-so-far prefix" logic in [`Self::deriv_positions`] (an
    /// `attribute` operand is the one exception: attributes are orthogonal
    /// to child/token sequencing entirely, matched separately).
    fn structurally_nullable(&mut self, id: Id) -> bool {
        if let Some(known) = self.structurally_nullable.get(id) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Attribute(..) | Node::Empty | Node::Text => true,
            Node::NotAllowed | Node::Element(_) | Node::Data(..) | Node::Value(_) => false,
            Node::List(inner) | Node::OneOrMore(inner) => self.structurally_nullable(inner),
            Node::Choice(values) => values
                .iter()
                .any(|&value| self.structurally_nullable(value)),
            Node::Group(values) | Node::Interleave(values) => values
                .iter()
                .all(|&value| self.structurally_nullable(value)),
        };
        self.structurally_nullable.set(id, result)
    }

    /// Whether some *reachable* position in `pattern` is a text-desiring
    /// leaf (`text`/`data`/`value`/`list`) that a whitespace-only text/
    /// attribute event, once it fails to match normally, should be treated
    /// as a real (failed) candidate for — rather than silently dropped as
    /// formatting/insignificant whitespace (§6.2.7). "Reachable" mirrors
    /// [`Self::deriv_positions`]'s own sequential-prefix gating for `group`
    /// ([`Self::structurally_nullable`] decides what counts as "already
    /// past"); for `choice`, a branch not taken is never a candidate for
    /// *this* event, so only refuse to drop when *every* branch wants text —
    /// if even one doesn't, that's the branch this whitespace can harmlessly
    /// fall through to (e.g. `choice(value "x", element bar {empty})`:
    /// leading whitespace before `<bar/>` is fine, even though the *other*
    /// branch wants text). `interleave`/`group`, by contrast, require
    /// *every* operand to eventually be satisfied (unordered or ordered
    /// respectively), so any one of them still wanting text is enough to not
    /// drop.
    fn wants_text(&mut self, id: Id) -> bool {
        if let Some(known) = self.wants_text.get(id) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Text | Node::Data(..) | Node::Value(_) | Node::List(_) => true,
            Node::Attribute(..) | Node::Empty | Node::NotAllowed | Node::Element(_) => false,
            Node::Choice(values) => values.iter().all(|&value| self.wants_text(value)),
            Node::Interleave(values) => values.iter().any(|&value| self.wants_text(value)),
            Node::Group(values) => {
                let mut reachable = true;
                let mut found = false;
                for &value in values.iter() {
                    if reachable {
                        found = found || self.wants_text(value);
                    }
                    reachable = reachable && self.structurally_nullable(value);
                }
                found
            }
            Node::OneOrMore(inner) => self.wants_text(inner),
        };
        self.wants_text.set(id, result)
    }

    /// Whether some `data`/`value`/`list` is reachable through the
    /// structural nodes of `pattern` — if not, its derivative by text
    /// doesn't depend on what the text is.
    fn text_sensitive(&mut self, id: Id) -> bool {
        if let Some(known) = self.text_sensitive.get(id) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Data(..) | Node::Value(_) | Node::List(_) => true,
            Node::Empty
            | Node::NotAllowed
            | Node::Text
            | Node::Element(_)
            | Node::Attribute(..) => false,
            Node::Choice(values) | Node::Group(values) | Node::Interleave(values) => {
                values.iter().any(|&value| self.text_sensitive(value))
            }
            Node::OneOrMore(inner) => self.text_sensitive(inner),
        };
        self.text_sensitive.set(id, result)
    }

    /// Whether `pattern`'s derivatives can depend on the document's
    /// namespace bindings, through a reachable `data`/`value` (also inside
    /// an attribute, `list` or `except`) whose datatype library says so.
    /// Element content isn't looked into: it is validated separately.
    fn context_dependent(&mut self, id: Id) -> bool {
        if let Some(known) = self.context_dependent.get(id) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Data(Leaf(data), except) => {
                let Pattern::Data {
                    datatype,
                    datatype_library,
                    ..
                } = data
                else {
                    unreachable!("a data node holds a data pattern")
                };
                self.library_context_dependent(datatype_library.as_deref(), datatype)
                    || except.iter().any(|&item| self.context_dependent(item))
            }
            Node::Value(Leaf(value)) => {
                let Pattern::Value {
                    datatype,
                    datatype_library,
                    ..
                } = value
                else {
                    unreachable!("a value node holds a value pattern")
                };
                let type_name = datatype.as_deref().unwrap_or("token");
                self.library_context_dependent(datatype_library.as_deref(), type_name)
            }
            Node::Empty | Node::NotAllowed | Node::Text | Node::Element(_) => false,
            Node::Attribute(_, inner) | Node::List(inner) | Node::OneOrMore(inner) => {
                self.context_dependent(inner)
            }
            Node::Choice(values) | Node::Group(values) | Node::Interleave(values) => {
                values.iter().any(|&value| self.context_dependent(value))
            }
        };
        self.context_dependent.set(id, result)
    }

    fn library_context_dependent(&self, library: Option<&str>, type_name: &str) -> bool {
        self.registry
            .get(library.unwrap_or(""))
            .is_none_or(|library| library.is_context_dependent(type_name))
    }

    /// The derivative of an element's content `id` by one of its
    /// attributes, cached by name and value when the namespace bindings
    /// can't affect it.
    fn deriv_attribute<'c>(
        &mut self,
        id: Id,
        name: &ExpandedName,
        symbol: Symbol,
        value: &str,
        context: impl FnOnce() -> &'c DatatypeContext,
    ) -> Id {
        let cacheable = !self.context_dependent(id);
        if cacheable
            && let Some(&derived) = self
                .attribute_derivs
                .get(&(id, symbol))
                .and_then(|derivs| derivs.get(value))
        {
            return derived;
        }
        // With one attribute pattern for this name, the derivative is either
        // `notAllowed` (the value doesn't match it) or the same pattern for
        // every value it accepts, so only the value needs checking.
        let derived = if let Some(leaf) = self.attribute_leaf(id, symbol, name) {
            let event = Event::Attribute {
                name,
                symbol,
                value,
                context: context(),
            };
            if self.deriv(leaf, event) == NOT_ALLOWED {
                NOT_ALLOWED
            } else if let Some(&derived) = self.accepted_attribute_derivs.get(&(id, symbol)) {
                derived
            } else {
                let derived = self.deriv(id, event);
                self.accepted_attribute_derivs.insert((id, symbol), derived);
                derived
            }
        } else {
            let event = Event::Attribute {
                name,
                symbol,
                value,
                context: context(),
            };
            self.deriv(id, event)
        };
        if cacheable {
            self.attribute_derivs
                .entry((id, symbol))
                .or_default()
                .insert(value.into(), derived);
        }
        derived
    }

    /// The namespace bindings for deriving `pattern` by a text: none at all
    /// when no `data`/`value`/`list` is reachable, since then the text
    /// itself (let alone the bindings) can't matter.
    fn text_context<'c>(
        &mut self,
        pattern: Id,
        context: impl FnOnce() -> &'c DatatypeContext,
    ) -> &'c DatatypeContext {
        if self.text_sensitive(pattern) {
            context()
        } else {
            &NO_BINDINGS
        }
    }

    /// The "closed" form of a `group` position that a later position's
    /// event was matched *past* (see [`Self::deriv_positions`]'s
    /// `sequential` case): standard Brzozowski concatenation-derivative
    /// rule, D_c(A·B)'s "skip A" branch is exactly D_c(B), not A·D_c(B) — so
    /// A is fixed to `Empty` (done, contributing nothing further to *child*
    /// matching), not left open to match more child events later.
    /// `attribute` sub-patterns are the one exception: they're orthogonal to
    /// child sequencing (matched entirely separately) and must stay exactly
    /// as they were — closing one here would wrongly mark a not-yet-matched
    /// attribute as satisfied.
    fn close_child_position(&mut self, id: Id) -> Id {
        if let Some(&closed) = self.closed.get(&id) {
            return closed;
        }
        let result = match self.store.node(id).clone() {
            Node::Attribute(..) => id,
            Node::Group(values) => {
                let closed = self.close_all(&values);
                self.store.group(closed)
            }
            Node::Interleave(values) => {
                let closed = self.close_all(&values);
                self.store.interleave(closed)
            }
            Node::Choice(values) => {
                let closed = self.close_all(&values);
                self.store.choice(closed)
            }
            Node::OneOrMore(inner) => self.close_child_position(inner),
            _ => EMPTY,
        };
        self.closed.insert(id, result);
        result
    }

    fn close_all(&mut self, values: &[Id]) -> Vec<Id> {
        values
            .iter()
            .map(|&value| self.close_child_position(value))
            .collect()
    }

    /// The derivative of `pattern` with respect to one event. Derivatives by
    /// a child element, and by text where the text can't matter, are
    /// cached; an attribute derivative is cut short as `notAllowed` wherever
    /// no attribute of that name is reachable at all.
    fn deriv(&mut self, id: Id, event: Event) -> Id {
        match event {
            Event::Element { symbol, .. } => {
                if let Some(&derived) = self.element_derivs.get(&(id, symbol)) {
                    return derived;
                }
                let derived = self.deriv_uncached(id, event);
                self.element_derivs.insert((id, symbol), derived);
                derived
            }
            Event::Text { .. } if !self.text_sensitive(id) => {
                if let Some(&derived) = self.text_derivs.get(&id) {
                    return derived;
                }
                let derived = self.deriv_uncached(id, event);
                self.text_derivs.insert(id, derived);
                derived
            }
            Event::Attribute { name, symbol, .. }
                if !self.attribute_name_known(id, symbol, name) =>
            {
                NOT_ALLOWED
            }
            _ => self.deriv_uncached(id, event),
        }
    }

    /// `group`/`interleave`/`choice`/`oneOrMore` recurse the same way for
    /// every kind of event; only the leaves differ. `group` is sequential
    /// for element and text events but not for attributes, which have no
    /// order in XML; `interleave` is never sequential.
    fn deriv_uncached(&mut self, id: Id, event: Event) -> Id {
        let group_sequential = !matches!(event, Event::Attribute { .. });
        match self.store.node(id).clone() {
            Node::Group(values) => {
                self.deriv_positions(&values, group_sequential, Node::Group, event)
            }
            Node::Interleave(values) => {
                self.deriv_positions(&values, false, Node::Interleave, event)
            }
            Node::Choice(values) => {
                let derived = values
                    .iter()
                    .map(|&value| self.deriv(value, event))
                    .collect();
                self.store.choice(derived)
            }
            // `oneOrMore(p)`'s derivative: consume one occurrence of `p`
            // now, then either the `oneOrMore` continues or (having
            // satisfied the "one") it may now stop.
            Node::OneOrMore(inner) => {
                let derived = self.deriv(inner, event);
                let rest = self.store.choice(vec![id, EMPTY]);
                self.store.group(vec![derived, rest])
            }
            Node::Attribute(Leaf(attribute), value_pattern) => match event {
                Event::Attribute {
                    name,
                    value,
                    context,
                    ..
                } if leaf_name_matches(attribute, name)
                    && self.value_matches(value_pattern, value, context) =>
                {
                    EMPTY
                }
                _ => NOT_ALLOWED,
            },
            Node::Element(Leaf(element)) => match event {
                Event::Element { name, .. } if leaf_name_matches(element, name) => EMPTY,
                _ => NOT_ALLOWED,
            },
            // `text` absorbs any amount of text, including across separate
            // (non-adjacent, interleaved-with-elements) runs, so it stays
            // `text` rather than being "used up".
            Node::Text => match event {
                Event::Text { .. } => TEXT,
                _ => NOT_ALLOWED,
            },
            Node::Data(Leaf(data), except) => match event {
                Event::Text { text, context }
                    if self.data_matches(data, &except, text, context) =>
                {
                    EMPTY
                }
                _ => NOT_ALLOWED,
            },
            Node::Value(Leaf(value)) => match event {
                Event::Text { text, context } if self.value_equals(value, text, context) => EMPTY,
                _ => NOT_ALLOWED,
            },
            Node::List(inner) => match event {
                Event::Text { text, context } if self.list_matches(inner, text, context) => EMPTY,
                _ => NOT_ALLOWED,
            },
            Node::Empty | Node::NotAllowed => NOT_ALLOWED,
        }
    }

    /// Derives `values` (the operands of a `group`/`interleave`) with
    /// respect to one event, trying every position the event can match at
    /// and rebuilding the container (`wrap`) around whichever one operand
    /// changed, leaving the rest untouched. If `sequential`, a position is
    /// only reachable once every earlier operand is structurally satisfied
    /// (`group`'s ordered semantics for element/text content); if not,
    /// every position is always reachable (`interleave`'s semantics — and,
    /// deliberately, `group`'s *attribute*-matching semantics too, since
    /// unlike child content, attributes have no order in XML at all).
    fn deriv_positions(
        &mut self,
        values: &[Id],
        sequential: bool,
        wrap: fn(Rc<[Id]>) -> Node<'s>,
        event: Event,
    ) -> Id {
        let mut alternatives = Vec::new();
        let mut reachable = true;
        for (index, &value) in values.iter().enumerate() {
            if reachable {
                let derived = self.deriv(value, event);
                if derived != NOT_ALLOWED {
                    let mut branch = values.to_vec();
                    if sequential {
                        // Standard Brzozowski concatenation-derivative rule:
                        // D_c(A·B)'s "skip A" branch (taken because A is
                        // already nullable) is exactly D_c(B) — A itself is
                        // *closed*, fixed to `Empty`, not left as A. Left
                        // unchanged, A would wrongly stay available to match
                        // *more* events later, after B has already started
                        // (e.g. `group(text, element bar)` would let text
                        // reappear after `bar`, which isn't sequential at
                        // all). Interleave/attribute-matching (`!sequential`)
                        // has no such "prefix must close" concept — every
                        // other position, before or after `index`, genuinely
                        // does stay open regardless of order.
                        for earlier in branch.iter_mut().take(index) {
                            *earlier = self.close_child_position(*earlier);
                        }
                    }
                    branch[index] = derived;
                    alternatives.push(self.store.sequence(branch, wrap));
                }
            }
            if sequential {
                reachable = reachable && self.structurally_nullable(value);
            }
        }
        self.store.choice(alternatives)
    }

    fn data_matches(
        &mut self,
        data: &Pattern,
        except: &[Id],
        text: &str,
        context: &DatatypeContext,
    ) -> bool {
        let Pattern::Data {
            datatype,
            datatype_library,
            params,
            ..
        } = data
        else {
            unreachable!("a data node holds a data pattern")
        };
        let library = datatype_library.as_deref().unwrap_or("");
        let implementation = self
            .registry
            .get(library)
            .expect("checked upfront by check_datatypes_supported");
        implementation.matches(datatype, &param_pairs(params), text, context)
            && !except.iter().any(|&item| {
                let derived = self.deriv(item, Event::Text { text, context });
                self.nullable(derived)
            })
    }

    fn value_equals(&self, value_pattern: &Pattern, text: &str, context: &DatatypeContext) -> bool {
        let Pattern::Value {
            value,
            datatype,
            datatype_library,
            namespace,
            context: schema_context,
        } = value_pattern
        else {
            unreachable!("a value node holds a value pattern")
        };
        let library = datatype_library.as_deref().unwrap_or("");
        let type_name = datatype.as_deref().unwrap_or("token");
        let implementation = self
            .registry
            .get(library)
            .expect("checked upfront by check_datatypes_supported");
        implementation.values_equal(
            type_name,
            value,
            &schema_value_context(namespace, schema_context),
            text,
            context,
        )
    }

    /// Whether `text`, split on XML whitespace into tokens, matches
    /// `pattern` (a `list`'s inner pattern) when each token is fed through
    /// in sequence.
    fn list_matches(&mut self, pattern: Id, text: &str, context: &DatatypeContext) -> bool {
        let mut current = pattern;
        for token in text.split_ascii_whitespace() {
            current = self.deriv(
                current,
                Event::Text {
                    text: token,
                    context,
                },
            );
            if current == NOT_ALLOWED {
                return false;
            }
        }
        // Not `nullable`: a slot no token ever reached (e.g. the second
        // `data` in `list { data type="token", data type="token" }` given
        // only one token) must not be excused just because its datatype
        // would also tolerate the literal empty string — see
        // `structurally_nullable`'s docs.
        self.structurally_nullable(current)
    }

    /// Whether `value` (an attribute's string value) satisfies `pattern` on
    /// its own — including §6.2.7: a value that's whitespace-only (or
    /// empty) and doesn't otherwise fit still matches if nothing in
    /// `pattern` actually wants text (e.g. `attribute id { empty }` accepts
    /// `id=" "`) — see [`Self::wants_text`].
    fn value_matches(&mut self, pattern: Id, value: &str, context: &DatatypeContext) -> bool {
        let derived = self.deriv(
            pattern,
            Event::Text {
                text: value,
                context,
            },
        );
        if derived != NOT_ALLOWED && self.structurally_nullable(derived) {
            return true;
        }
        value.trim().is_empty() && !self.wants_text(pattern)
    }

    /// Whether *some* `attribute` pattern reachable in `pattern` (through
    /// `group`, `interleave`, `choice`, `oneOrMore` — the same positions an
    /// attribute derivative looks at) has a name class matching `name`,
    /// regardless of value. Used to tell "this attribute isn't allowed at
    /// all" apart from "this attribute is allowed, but its value is wrong".
    fn attribute_name_known(&mut self, id: Id, symbol: Symbol, name: &ExpandedName) -> bool {
        if let Some(&known) = self.attribute_names.get(&(id, symbol)) {
            return known;
        }
        let result = match self.store.node(id).clone() {
            Node::Attribute(Leaf(attribute), _) => leaf_name_matches(attribute, name),
            Node::Group(values) | Node::Interleave(values) | Node::Choice(values) => values
                .iter()
                .any(|&value| self.attribute_name_known(value, symbol, name)),
            Node::OneOrMore(inner) => self.attribute_name_known(inner, symbol, name),
            _ => false,
        };
        self.attribute_names.insert((id, symbol), result);
        result
    }

    /// The single `attribute` pattern in `id` (not inside elements) whose
    /// name class matches `name`, or `None` if none or several do.
    fn attribute_leaf(&mut self, id: Id, symbol: Symbol, name: &ExpandedName) -> Option<Id> {
        if let Some(&leaf) = self.attribute_leaves.get(&(id, symbol)) {
            return leaf;
        }
        let mut leaves = Vec::new();
        self.collect_attribute_leaves(id, name, &mut leaves);
        leaves.sort_unstable();
        leaves.dedup();
        let leaf = match leaves.as_slice() {
            [only] => Some(*only),
            _ => None,
        };
        self.attribute_leaves.insert((id, symbol), leaf);
        leaf
    }

    fn collect_attribute_leaves(&mut self, id: Id, name: &ExpandedName, out: &mut Vec<Id>) {
        match self.store.node(id).clone() {
            Node::Attribute(Leaf(attribute), _) => {
                if leaf_name_matches(attribute, name) {
                    out.push(id);
                }
            }
            Node::Group(values) | Node::Interleave(values) | Node::Choice(values) => {
                for &value in values.iter() {
                    self.collect_attribute_leaves(value, name, out);
                }
            }
            Node::OneOrMore(inner) => self.collect_attribute_leaves(inner, name, out),
            _ => {}
        }
    }

    /// The union (as a `choice`) of every `element` pattern's content
    /// reachable in `pattern` (through `group` (sequentially, like an
    /// element derivative), `interleave`, `choice` and `oneOrMore`) whose
    /// name class matches `name` — `None` if none do. Used both to find
    /// what a child element's own content must be validated against, and to
    /// tell whether this element name is expected here at all.
    fn find_element_contents(&mut self, id: Id, symbol: Symbol, name: &ExpandedName) -> Option<Id> {
        if let Some(&contents) = self.element_contents.get(&(id, symbol)) {
            return contents;
        }
        let mut found = Vec::new();
        self.collect_element_contents(id, name, &mut found);
        let contents = if found.is_empty() {
            None
        } else {
            Some(self.store.choice(found))
        };
        self.element_contents.insert((id, symbol), contents);
        contents
    }

    fn collect_element_contents(&mut self, id: Id, name: &ExpandedName, out: &mut Vec<Id>) {
        match self.store.node(id).clone() {
            Node::Element(Leaf(element)) if leaf_name_matches(element, name) => {
                let content = self.store.element_content(id);
                out.push(content);
            }
            Node::Choice(values) | Node::Interleave(values) => {
                for &value in values.iter() {
                    self.collect_element_contents(value, name, out);
                }
            }
            Node::Group(values) => {
                for &value in values.iter() {
                    self.collect_element_contents(value, name, out);
                    if !self.structurally_nullable(value) {
                        break;
                    }
                }
            }
            Node::OneOrMore(inner) => self.collect_element_contents(inner, name, out),
            _ => {}
        }
    }
}

/// Whether the name class of a schema `element`/`attribute` pattern
/// matches `name`.
fn leaf_name_matches(pattern: &Pattern, name: &ExpandedName) -> bool {
    match pattern {
        Pattern::Element {
            name: name_class, ..
        }
        | Pattern::Attribute {
            name: name_class, ..
        } => name_class_matches(name_class, &name.namespace, &name.local),
        _ => unreachable!("only element and attribute patterns have names"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relaxng::datatypes::DatatypeLibrary;
    use crate::relaxng::{ResolveError, SchemaResolver, SchemaSource, SchemaSyntax};

    // Pattern-level wrappers around `Validator`, so the derivative rules
    // can be tested on hand-built patterns. Each call interns its inputs
    // into a fresh store and turns the result back into a `Pattern`.

    fn nullable(pattern: &Pattern, definitions: &Definitions, registry: &DatatypeRegistry) -> bool {
        let mut validator = Validator::new(definitions, registry);
        let id = validator.store.intern(pattern);
        validator.nullable(id)
    }

    fn derive<'e>(
        pattern: &Pattern,
        definitions: &Definitions,
        name: &ExpandedName,
        event: impl FnOnce(Symbol) -> Event<'e>,
    ) -> Pattern {
        let registry = registry();
        let mut validator = Validator::new(definitions, &registry);
        let id = validator.store.intern(pattern);
        let event = event(validator.symbol(name));
        let derived = validator.deriv(id, event);
        validator.store.to_pattern(derived)
    }

    fn deriv_element_event(
        pattern: &Pattern,
        name: &ExpandedName,
        definitions: &Definitions,
    ) -> Pattern {
        derive(pattern, definitions, name, |symbol| Event::Element {
            name,
            symbol,
        })
    }

    fn deriv_attribute(
        pattern: &Pattern,
        name: &ExpandedName,
        value: &str,
        definitions: &Definitions,
        _registry: &DatatypeRegistry,
        context: &DatatypeContext,
    ) -> Pattern {
        derive(pattern, definitions, name, |symbol| Event::Attribute {
            name,
            symbol,
            value,
            context,
        })
    }

    fn deriv_text(
        pattern: &Pattern,
        text: &str,
        definitions: &Definitions,
        _registry: &DatatypeRegistry,
        context: &DatatypeContext,
    ) -> Pattern {
        let unused = ExpandedName::new(None::<String>, "");
        derive(pattern, definitions, &unused, |_| Event::Text {
            text,
            context,
        })
    }

    fn attribute_name_known(
        pattern: &Pattern,
        name: &ExpandedName,
        definitions: &Definitions,
    ) -> bool {
        let registry = registry();
        let mut validator = Validator::new(definitions, &registry);
        let id = validator.store.intern(pattern);
        let symbol = validator.symbol(name);
        validator.attribute_name_known(id, symbol, name)
    }

    struct NoResolver;

    impl SchemaResolver for NoResolver {
        fn resolve(&self, href: &str, _base_uri: &str) -> Result<SchemaSource, ResolveError> {
            Err(ResolveError::new(format!(
                "validator tests don't use includes (requested `{href}`)"
            )))
        }
    }

    fn schema(text: &str) -> CompiledSchema {
        let document = crate::relaxng::parse(&SchemaSource::new(
            text,
            "memory:/main.rnc",
            SchemaSyntax::Compact,
        ))
        .expect("schema parses");
        crate::relaxng::simplify(&document, &NoResolver).expect("schema simplifies")
    }

    fn xml_schema(text: &str) -> CompiledSchema {
        let document = crate::relaxng::parse(&SchemaSource::new(
            text,
            "memory:/main.rng",
            SchemaSyntax::Xml,
        ))
        .expect("schema parses");
        crate::relaxng::simplify(&document, &NoResolver).expect("schema simplifies")
    }

    fn registry() -> DatatypeRegistry {
        DatatypeRegistry::new()
    }

    /// A minimal, structured (not pre-formatted) location — exercises
    /// [`Element::Location`] as something other than `String`/`()`, the
    /// whole point of that associated type being generic rather than
    /// hardcoded.
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestLocation {
        line: u32,
        column: u32,
    }

    impl fmt::Display for TestLocation {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}:{}", self.line, self.column)
        }
    }

    #[derive(Clone, Debug)]
    struct TestElement {
        name: ExpandedName,
        attributes: Vec<(ExpandedName, String)>,
        children: Vec<Content<TestElement>>,
        namespace_bindings: Vec<(String, String)>,
        location: Option<TestLocation>,
    }

    impl TestElement {
        fn new(local: &str) -> Self {
            Self {
                name: ExpandedName::new(None::<String>, local),
                attributes: Vec::new(),
                children: Vec::new(),
                namespace_bindings: Vec::new(),
                location: None,
            }
        }

        fn new_namespaced(namespace: &str, local: &str) -> Self {
            Self {
                name: ExpandedName::new(Some(namespace), local),
                attributes: Vec::new(),
                children: Vec::new(),
                namespace_bindings: Vec::new(),
                location: None,
            }
        }

        fn at(mut self, line: u32, column: u32) -> Self {
            self.location = Some(TestLocation { line, column });
            self
        }

        fn attr(mut self, local: &str, value: &str) -> Self {
            self.attributes
                .push((ExpandedName::new(None::<String>, local), value.to_owned()));
            self
        }

        fn text(mut self, text: &str) -> Self {
            self.children.push(Content::Text(text.to_owned()));
            self
        }

        fn child(mut self, element: TestElement) -> Self {
            self.children.push(Content::Element(element));
            self
        }

        fn xmlns(mut self, prefix: &str, uri: &str) -> Self {
            self.namespace_bindings
                .push((prefix.to_owned(), uri.to_owned()));
            self
        }
    }

    impl Element for TestElement {
        type Location = TestLocation;
        fn name(&self) -> ExpandedName {
            self.name.clone()
        }
        fn attributes(&self) -> impl Iterator<Item = (ExpandedName, String)> {
            self.attributes.clone().into_iter()
        }
        fn children(&self) -> impl Iterator<Item = Content<Self>> {
            self.children.clone().into_iter()
        }
        fn location(&self) -> Option<Self::Location> {
            self.location.clone()
        }
        fn namespace_bindings(&self) -> impl Iterator<Item = (String, String)> {
            self.namespace_bindings.clone().into_iter()
        }
    }

    fn kinds(errors: &[ValidationError<TestLocation>]) -> Vec<ValidationErrorKind> {
        errors.iter().map(|error| error.kind().clone()).collect()
    }

    fn name(local: &str) -> ExpandedName {
        ExpandedName::new(None::<String>, local)
    }

    // --- nullable ---

    #[test]
    fn nullable_base_cases() {
        let definitions = Definitions::new();
        let registry = registry();
        assert!(nullable(&Pattern::Empty, &definitions, &registry));
        assert!(!nullable(&Pattern::NotAllowed, &definitions, &registry));
        assert!(nullable(&Pattern::Text, &definitions, &registry));
    }

    #[test]
    fn nullable_group_requires_every_operand_nullable() {
        let definitions = Definitions::new();
        let registry = registry();
        assert!(nullable(
            &Pattern::Group(vec![Pattern::Empty, Pattern::Text]),
            &definitions,
            &registry
        ));
        assert!(!nullable(
            &Pattern::Group(vec![Pattern::Empty, Pattern::NotAllowed]),
            &definitions,
            &registry
        ));
    }

    #[test]
    fn nullable_choice_requires_any_operand_nullable() {
        let definitions = Definitions::new();
        let registry = registry();
        assert!(nullable(
            &Pattern::Choice(vec![Pattern::NotAllowed, Pattern::Empty]),
            &definitions,
            &registry
        ));
        assert!(!nullable(
            &Pattern::Choice(vec![Pattern::NotAllowed, Pattern::NotAllowed]),
            &definitions,
            &registry
        ));
    }

    #[test]
    fn nullable_one_or_more_delegates_to_inner() {
        let definitions = Definitions::new();
        let registry = registry();
        assert!(nullable(
            &Pattern::OneOrMore(vec![Pattern::Empty]),
            &definitions,
            &registry
        ));
        assert!(!nullable(
            &Pattern::OneOrMore(vec![Pattern::NotAllowed]),
            &definitions,
            &registry
        ));
    }

    #[test]
    fn nullable_list_delegates_to_inner_same_as_zero_tokens_would() {
        let definitions = Definitions::new();
        let registry = registry();
        // `list { empty }` accepts having no text at all.
        assert!(nullable(
            &Pattern::List(vec![Pattern::Empty]),
            &definitions,
            &registry
        ));
        assert!(!nullable(
            &Pattern::List(vec![Pattern::NotAllowed]),
            &definitions,
            &registry
        ));
    }

    #[test]
    fn nullable_ref_resolves_through_definitions() {
        let definitions = Definitions::from([("x".to_owned(), Pattern::Empty)]);
        let registry = registry();
        assert!(nullable(&Pattern::Ref("x".into()), &definitions, &registry));
    }

    // --- individual derivative functions: element / attribute / text ---

    #[test]
    fn deriv_element_event_matches_by_name_and_rejects_others() {
        let compiled = schema("start = element foo { empty }");
        let registry = registry();
        assert!(nullable(
            &deriv_element_event(&compiled.start, &name("foo"), &compiled.definitions),
            &compiled.definitions,
            &registry
        ));
        assert!(matches!(
            deriv_element_event(&compiled.start, &name("bar"), &compiled.definitions),
            Pattern::NotAllowed
        ));
    }

    #[test]
    fn deriv_attribute_checks_name_and_value() {
        let compiled = schema(r#"start = element foo { attribute id { "x" } }"#);
        let registry = registry();
        let context = DatatypeContext::empty();
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        assert!(nullable(
            &deriv_attribute(
                &body[0],
                &name("id"),
                "x",
                &compiled.definitions,
                &registry,
                &context
            ),
            &compiled.definitions,
            &registry
        ));
        assert!(matches!(
            deriv_attribute(
                &body[0],
                &name("id"),
                "y",
                &compiled.definitions,
                &registry,
                &context
            ),
            Pattern::NotAllowed
        ));
        assert!(matches!(
            deriv_attribute(
                &body[0],
                &name("other"),
                "x",
                &compiled.definitions,
                &registry,
                &context
            ),
            Pattern::NotAllowed
        ));
    }

    #[test]
    fn deriv_text_data_accepts_any_string_value_requires_exact_match() {
        let registry = registry();
        let context = DatatypeContext::empty();
        let compiled = schema("start = element foo { token }");
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        assert!(nullable(
            &deriv_text(
                &body[0],
                "whatever text",
                &compiled.definitions,
                &registry,
                &context
            ),
            &compiled.definitions,
            &registry
        ));

        let compiled = schema(r#"start = element foo { "exact" }"#);
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        assert!(nullable(
            &deriv_text(
                &body[0],
                "exact",
                &compiled.definitions,
                &registry,
                &context
            ),
            &compiled.definitions,
            &registry
        ));
        assert!(matches!(
            deriv_text(
                &body[0],
                "different",
                &compiled.definitions,
                &registry,
                &context
            ),
            Pattern::NotAllowed
        ));
    }

    #[test]
    fn deriv_text_token_equality_collapses_whitespace() {
        let registry = registry();
        let context = DatatypeContext::empty();
        let compiled = schema(r#"start = element foo { token "a b" }"#);
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        assert!(nullable(
            &deriv_text(
                &body[0],
                "a   b",
                &compiled.definitions,
                &registry,
                &context
            ),
            &compiled.definitions,
            &registry
        ));
    }

    // --- `group` (sequential/"after") vs `interleave` (unordered) ---

    #[test]
    fn group_derivative_is_sequential() {
        let registry = registry();
        let compiled = schema("start = element root { element a { empty }, element b { empty } }");
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        let content = &body[0];
        // "b" cannot be matched before "a" — group is ordered.
        assert!(matches!(
            deriv_element_event(content, &name("b"), &compiled.definitions),
            Pattern::NotAllowed
        ));
        let after_a = deriv_element_event(content, &name("a"), &compiled.definitions);
        let after_a_b = deriv_element_event(&after_a, &name("b"), &compiled.definitions);
        assert!(nullable(&after_a_b, &compiled.definitions, &registry));
    }

    #[test]
    fn interleave_derivative_allows_either_order() {
        let registry = registry();
        let compiled = schema("start = element root { element a { empty } & element b { empty } }");
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        let content = &body[0];
        // Unlike `group`, "b" is allowed to come before "a".
        let after_b = deriv_element_event(content, &name("b"), &compiled.definitions);
        assert!(!matches!(after_b, Pattern::NotAllowed));
        let after_b_a = deriv_element_event(&after_b, &name("a"), &compiled.definitions);
        assert!(nullable(&after_b_a, &compiled.definitions, &registry));
    }

    // --- the xmloxide regression this crate exists to fix (issue #56):
    // `ref` inside `interleave` must resolve for *attribute* matching. ---

    #[test]
    fn attribute_matches_through_ref_and_interleave_xmloxide_issue_56() {
        let compiled = schema(
            "start = element foo { attrs & text }\n\
             attrs = attribute id { text }",
        );
        let document = TestElement::new("foo").attr("id", "x").text("hello");
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }

    #[test]
    fn attribute_name_known_resolves_ref_through_group_and_choice_too() {
        // The same transparency, exercised for `group` (attribute-matching
        // semantics, unordered) and `choice`.
        let compiled = schema(
            "start = element foo { a, b }\n\
             a = attribute x { text } | attribute y { text }\n\
             b = empty",
        );
        let Pattern::Element { body, .. } = &compiled.start else {
            panic!("expected an element pattern");
        };
        assert!(attribute_name_known(
            &body[0],
            &name("x"),
            &compiled.definitions
        ));
        assert!(attribute_name_known(
            &body[0],
            &name("y"),
            &compiled.definitions
        ));
        assert!(!attribute_name_known(
            &body[0],
            &name("z"),
            &compiled.definitions
        ));
    }

    #[test]
    fn ref_is_transparent_for_child_elements_through_choice_and_group() {
        let compiled = schema(
            "start = element foo { greeting, farewell }\n\
             greeting = element hello { empty } | element hi { empty }\n\
             farewell = element bye { empty }",
        );
        let document = TestElement::new("foo")
            .child(TestElement::new("hi"))
            .child(TestElement::new("bye"));
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }

    // --- end-to-end `validate()`: hand-picked valid/invalid document pairs ---

    #[test]
    fn valid_document_with_required_attribute_and_text() {
        let compiled = schema("start = element foo { attribute id { text }, text }");
        let document = TestElement::new("foo").attr("id", "1").text("hello");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_required_attribute_is_reported() {
        let compiled = schema("start = element foo { attribute id { text }, text }");
        let document = TestElement::new("foo").text("hello");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::MissingContent]);
    }

    #[test]
    fn unexpected_attribute_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").attr("id", "1");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![ValidationErrorKind::UnexpectedAttribute(name("id"))]
        );
    }

    /// `Element::Location` is generic — a caller with real structured
    /// position data (not just a pre-formatted string) gets it back out
    /// of `ValidationError::location()` unchanged, not re-parsed from
    /// text. `TestLocation` (`{line, column}`) stands in for whatever
    /// shape a real caller's own position type has.
    #[test]
    fn structured_location_is_reported_on_the_validation_error() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").attr("id", "1").at(3, 7);
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].location(),
            Some(&TestLocation { line: 3, column: 7 })
        );
        // `Display` still renders it as formatted text for messages that
        // want that.
        assert_eq!(errors[0].to_string(), "unexpected attribute `id` at 3:7");
    }

    #[test]
    fn invalid_attribute_value_is_reported_distinctly_from_unexpected_attribute() {
        let compiled = schema(r#"start = element foo { attribute id { "x" } }"#);
        let document = TestElement::new("foo").attr("id", "y");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        // The mismatched attribute is left unconsumed (so later attributes
        // keep being checked against the schema rather than being drowned
        // out by one bad value), which is also, truthfully, "required
        // content still missing" once the element ends.
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::InvalidAttributeValue(name("id"), "y".to_owned()),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unexpected_child_element_is_reported() {
        let compiled = schema("start = element foo { element bar { empty } }");
        let document = TestElement::new("foo").child(TestElement::new("baz"));
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::UnexpectedElement(name("baz")),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unexpected_root_element_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("bar");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![ValidationErrorKind::UnexpectedElement(name("bar"))]
        );
    }

    #[test]
    fn unexpected_text_is_reported() {
        let compiled = schema("start = element foo { empty }");
        let document = TestElement::new("foo").text("surprise");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::UnexpectedText]);
    }

    #[test]
    fn one_or_more_children_are_validated_recursively_and_repeatedly() {
        let compiled = schema("start = element foo { element item { text }+ }");
        let document = TestElement::new("foo")
            .child(TestElement::new("item").text("a"))
            .child(TestElement::new("item").text("b"));
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn one_or_more_requires_at_least_one_occurrence() {
        let compiled = schema("start = element foo { element item { empty }+ }");
        let document = TestElement::new("foo");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(kinds(&errors), vec![ValidationErrorKind::MissingContent]);
    }

    #[test]
    fn interleaved_text_and_element_children_validate_regardless_of_order() {
        let compiled = schema("start = element foo { mixed { element bar { empty } } }");
        let document = TestElement::new("foo")
            .text("before ")
            .child(TestElement::new("bar"))
            .text(" after");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn list_datatype_matches_whitespace_separated_tokens() {
        let compiled = schema("start = element foo { list { token+ } }");
        let document = TestElement::new("foo").text("a b   c");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn list_datatype_rejects_a_token_that_does_not_match() {
        let compiled = schema(r#"start = element foo { list { "a", "b" } }"#);
        let document = TestElement::new("foo").text("a c");
        let errors = validate(&compiled, &registry(), &document).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn unsupported_datatype_library_fails_fast_before_validating() {
        // Deliberately the whattf library, not XSD — `DatatypeRegistry::new`
        // now supports XSD out of the box (see the upstream project's design notes), so
        // this needs a genuinely unregistered library to still exercise
        // the fail-fast path; the whattf library is exactly that (a
        // caller like html-conform must register its own implementation).
        // XML syntax, not compact: compact-syntax identifiers don't accept
        // `-`, and `non-empty-string` is vnu's real type name.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="non-empty-string" datatypeLibrary="http://whattf.org/datatype-draft"/>
               </element>"#,
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &registry(), &document).unwrap_err();
        assert_eq!(error.library(), "http://whattf.org/datatype-draft");
    }

    #[test]
    fn builtin_only_registry_rejects_xsd_too() {
        let compiled = schema(
            "datatypes xsd = \"http://www.w3.org/2001/XMLSchema-datatypes\"\n\
             start = element foo { xsd:string }",
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &DatatypeRegistry::builtin_only(), &document).unwrap_err();
        assert_eq!(
            error.library(),
            "http://www.w3.org/2001/XMLSchema-datatypes"
        );
    }

    // --- XSD, via the default registry ---

    #[test]
    fn xsd_string_and_integer_types_match() {
        // Note: RNG XML's `type` attribute is always a bare string, never a
        // namespace-prefixed QName — the library comes from the separate
        // `datatypeLibrary` attribute (or §4.3 inheritance from an
        // ancestor), unlike compact syntax's `prefix:typeName` sugar.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <group>
                   <attribute name="count"><data type="positiveInteger"/></attribute>
                   <data type="string"/>
                 </group>
               </element>"#,
        );
        let valid = TestElement::new("foo").attr("count", "3").text("hello");
        assert!(validate(&compiled, &registry(), &valid).unwrap().is_empty());

        let invalid = TestElement::new("foo").attr("count", "0").text("hello");
        let errors = validate(&compiled, &registry(), &invalid).unwrap();
        assert_eq!(
            kinds(&errors),
            vec![
                ValidationErrorKind::InvalidAttributeValue(name("count"), "0".to_owned()),
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn xsd_pattern_facet_is_enforced() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <data type="string"><param name="pattern">[a-z]+</param></data>
               </element>"#,
        );
        let valid = TestElement::new("foo").text("abc");
        assert!(validate(&compiled, &registry(), &valid).unwrap().is_empty());
        let invalid = TestElement::new("foo").text("ABC");
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &invalid).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    #[test]
    fn xsd_decimal_value_equality_ignores_lexical_differences() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" xmlns:xsd="http://www.w3.org/2001/XMLSchema-datatypes" name="foo">
                 <value type="decimal" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">1.50</value>
               </element>"#,
        );
        // `1.5` and `01.50` are the same `decimal` value, just spelled
        // differently — real bignum-style equality, not string equality.
        let document = TestElement::new("foo").text("01.500");
        assert!(
            validate(&compiled, &registry(), &document)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn xsd_qname_resolves_prefix_via_document_namespace_bindings() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="QName" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes"/>
               </element>"#,
        );
        let document = TestElement::new("foo")
            .xmlns("h", "urn:example:h")
            .text("h:widget");
        assert!(
            validate(&compiled, &registry(), &document)
                .unwrap()
                .is_empty()
        );

        // No `h` binding in scope at all — the QName can't be resolved, so
        // it isn't a legal QName *value* here even though it's lexically
        // shaped like one.
        let unresolved = TestElement::new("foo").text("h:widget");
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &unresolved).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    /// Attribute derivatives are cached by name and value, except where a
    /// datatype depends on the namespace bindings in scope.
    #[test]
    fn repeated_attribute_values_are_checked_in_their_own_context() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="root" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <oneOrMore>
                   <element name="item">
                     <optional><attribute name="ref"><data type="QName"/></attribute></optional>
                     <optional><attribute name="rend"><data type="string"><param name="pattern">[a-z]+</param></data></attribute></optional>
                   </element>
                 </oneOrMore>
               </element>"#,
        );
        let item = |rend: &str| TestElement::new("item").attr("rend", rend);
        let document = TestElement::new("root")
            .child(item("ok"))
            .child(item("BAD"))
            .child(item("ok"))
            .child(item("BAD"));
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &document).unwrap()),
            vec![
                ValidationErrorKind::InvalidAttributeValue(name("rend"), "BAD".to_owned()),
                ValidationErrorKind::InvalidAttributeValue(name("rend"), "BAD".to_owned()),
            ]
        );

        let reference = || TestElement::new("item").attr("ref", "h:widget");
        let document = TestElement::new("root")
            .child(reference().xmlns("h", "urn:example:h"))
            .child(reference())
            .child(reference().xmlns("h", "urn:example:other"));
        assert_eq!(
            kinds(&validate(&compiled, &registry(), &document).unwrap()),
            vec![ValidationErrorKind::InvalidAttributeValue(
                name("ref"),
                "h:widget".to_owned()
            )]
        );
    }

    #[test]
    fn xsd_unknown_type_name_is_a_datatype_error() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
                 <data type="not-a-real-type"/>
               </element>"#,
        );
        let document = TestElement::new("foo").text("hello");
        let error = validate(&compiled, &registry(), &document).unwrap_err();
        assert_eq!(
            error.library(),
            "http://www.w3.org/2001/XMLSchema-datatypes"
        );
    }

    // --- external plugin: a dummy library, demonstrating the trait
    // contract a caller like html-conform would implement for vnu's
    // `http://whattf.org/datatype-draft` (not shipped by this crate). ---

    struct DummyNonEmptyStringLibrary;

    impl DatatypeLibrary for DummyNonEmptyStringLibrary {
        fn validate_params(&self, type_name: &str, params: &[(&str, &str)]) -> Result<(), String> {
            if type_name != "non-empty-string" {
                return Err(format!("unknown dummy type `{type_name}`"));
            }
            if let Some((param, _)) = params.first() {
                return Err(format!(
                    "`non-empty-string` takes no parameters, found `{param}`"
                ));
            }
            Ok(())
        }

        fn matches(
            &self,
            type_name: &str,
            _params: &[(&str, &str)],
            value: &str,
            _context: &DatatypeContext,
        ) -> bool {
            type_name == "non-empty-string" && !value.is_empty()
        }

        fn values_equal(
            &self,
            type_name: &str,
            expected: &str,
            _expected_context: &DatatypeContext,
            actual: &str,
            _actual_context: &DatatypeContext,
        ) -> bool {
            type_name == "non-empty-string" && expected == actual
        }
    }

    #[test]
    fn external_plugin_library_can_be_registered_and_used() {
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="foo">
                 <data type="non-empty-string" datatypeLibrary="http://whattf.org/datatype-draft"/>
               </element>"#,
        );
        let mut registry = DatatypeRegistry::new();
        registry.register(
            "http://whattf.org/datatype-draft",
            DummyNonEmptyStringLibrary,
        );

        let valid = TestElement::new("foo").text("hello");
        assert!(validate(&compiled, &registry, &valid).unwrap().is_empty());

        // An empty text *event* against a bare `data` isn't excused by
        // §6.2.7 weak-match rule 2: that rule only drops a whitespace-only
        // event when the content seen *so far* already, strictly,
        // satisfies the pattern without it — a bare `data`/`value` never
        // does (it structurally requires a real event), so this is
        // reported directly, same as a genuinely non-empty mismatch would
        // be.
        let invalid = TestElement::new("foo").text("");
        assert_eq!(
            kinds(&validate(&compiled, &registry, &invalid).unwrap()),
            vec![
                ValidationErrorKind::UnexpectedText,
                ValidationErrorKind::MissingContent,
            ]
        );
    }

    // --- adapter smoke test: a realistic html-conform/xmloxide-shaped tree
    // (namespace-qualified elements, an unprefixed attribute, nested
    // children, text) — this crate has no dependency on html-conform or
    // xmloxide (no HTML/XML-parser dependency in the core, see
    // the upstream project's design notes), so `TestElement` stands in for its sibling
    // project's `NormalizedNode` adapter output, confirming the `Element`
    // trait contract works for that shape before the Public-API phase. ---

    #[test]
    fn adapter_smoke_test_with_a_realistic_xhtml_shaped_tree() {
        const XHTML: &str = "http://www.w3.org/1999/xhtml";
        // Compact-syntax (`.rnc`) prefixed element names are a known,
        // separate gap outside Phase 05's scope (see the upstream project's status notes)
        // — the XML (`.rng`) syntax resolves `xmlns:`-qualified names
        // correctly, so this realistic, namespace-qualified smoke test
        // uses that syntax instead.
        let compiled = xml_schema(
            r#"<element xmlns="http://relaxng.org/ns/structure/1.0" xmlns:xhtml="http://www.w3.org/1999/xhtml" name="xhtml:html">
                 <group>
                   <element name="xhtml:head"><empty/></element>
                   <element name="xhtml:body">
                     <element name="xhtml:p">
                       <group>
                         <attribute name="id"><text/></attribute>
                         <text/>
                       </group>
                     </element>
                   </element>
                 </group>
               </element>"#,
        );
        let document = TestElement::new_namespaced(XHTML, "html")
            .child(TestElement::new_namespaced(XHTML, "head"))
            .child(
                TestElement::new_namespaced(XHTML, "body").child(
                    TestElement::new_namespaced(XHTML, "p")
                        .attr("id", "intro")
                        .text("hello"),
                ),
            );
        let errors = validate(&compiled, &registry(), &document).expect("builtin datatypes only");
        assert_eq!(kinds(&errors), Vec::<ValidationErrorKind>::new());
    }
}
