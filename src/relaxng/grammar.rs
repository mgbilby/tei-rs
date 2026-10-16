use std::collections::BTreeMap;
use std::sync::Arc;

/// A parsed (but not yet composed/simplified) RELAX NG schema, as returned
/// by [`crate::relaxng::parse`]. Pass it to [`crate::relaxng::simplify`] (or use
/// [`crate::relaxng::Schema::compile`], which does both steps) to get a
/// [`crate::relaxng::CompiledSchema`] ready to validate documents against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDocument {
    pub(crate) root: Root,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Root {
    Pattern(Pattern),
    Grammar(Grammar),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Grammar {
    pub(crate) context: Context,
    pub(crate) items: Vec<GrammarItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GrammarItem {
    Start {
        combine: Option<Combine>,
        body: Vec<Pattern>,
    },
    Define {
        name: String,
        combine: Option<Combine>,
        body: Vec<Pattern>,
    },
    Div(Vec<GrammarItem>),
    Include {
        href: String,
        inherit_namespace: Option<String>,
        body: Vec<GrammarItem>,
        context: Context,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Combine {
    Choice,
    Interleave,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Pattern {
    Element {
        name: NameClass,
        body: Vec<Pattern>,
        context: Context,
    },
    Attribute {
        name: NameClass,
        body: Vec<Pattern>,
        context: Context,
    },
    Group(Vec<Pattern>),
    Interleave(Vec<Pattern>),
    Choice(Vec<Pattern>),
    Optional(Vec<Pattern>),
    ZeroOrMore(Vec<Pattern>),
    OneOrMore(Vec<Pattern>),
    List(Vec<Pattern>),
    Mixed(Vec<Pattern>),
    Empty,
    NotAllowed,
    Text,
    Data {
        datatype: String,
        datatype_library: Option<String>,
        params: Vec<Param>,
        except: Vec<Pattern>,
        context: Context,
    },
    Value {
        value: String,
        datatype: Option<String>,
        datatype_library: Option<String>,
        namespace: Option<String>,
        context: Context,
    },
    Ref(String),
    ParentRef(String),
    ExternalRef {
        href: String,
        inherit_namespace: Option<String>,
        context: Context,
    },
    Grammar(Grammar),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Param {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) context: Context,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NameClass {
    Name {
        lexical: String,
        namespace: Option<String>,
        context: Context,
    },
    Any {
        except: Vec<NameClass>,
        context: Context,
    },
    Namespace {
        namespace: Option<String>,
        except: Vec<NameClass>,
        context: Context,
    },
    Choice(Vec<NameClass>),
}

/// The schema-side context of a pattern: base URI, namespace bindings and
/// inherited `datatypeLibrary`/`ns`. Nearly every pattern carries one, and
/// validation clones patterns constantly, so the data is shared behind an
/// `Arc` (copy-on-write through `DerefMut`) instead of copied each time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Context(Arc<ContextData>);

impl Context {
    pub(crate) fn new(data: ContextData) -> Self {
        Self(Arc::new(data))
    }
}

impl std::ops::Deref for Context {
    type Target = ContextData;

    fn deref(&self) -> &ContextData {
        &self.0
    }
}

impl std::ops::DerefMut for Context {
    fn deref_mut(&mut self) -> &mut ContextData {
        Arc::make_mut(&mut self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContextData {
    pub(crate) base_uri: String,
    pub(crate) namespaces: BTreeMap<String, String>,
    pub(crate) default_namespace: Option<String>,
    /// §4.3: the nearest ancestor's `datatypeLibrary`, inherited down for
    /// any `data`/`value` that doesn't specify its own.
    pub(crate) datatype_library: Option<String>,
    /// §4.9: the nearest ancestor's `ns`, inherited down for any `name`/
    /// `nsName`/`value` that doesn't specify its own (`None` here means
    /// "never set by any ancestor", which resolves to the empty string —
    /// distinct from an ancestor explicitly setting `ns=""`, though both
    /// end up meaning the same no-namespace default in practice).
    pub(crate) ns: Option<String>,
}
