//! Compiled XPath expressions and XSLT match patterns. Names are resolved
//! against the static namespace bindings while parsing.

use super::functions::Function;
use super::value::{Atomic, AtomicType};

#[derive(Clone, Debug)]
pub(crate) enum Expr {
    Literal(Atomic),
    /// A variable, by name (the parser checks that it is in scope).
    Var(String),
    /// `.`
    ContextItem,
    /// A leading `/`: the root of the context node's tree.
    Root,
    /// `(a, b, …)`; `()` is the empty sequence.
    Sequence(Vec<Expr>),
    /// A primary expression followed by predicates.
    Filter(Box<Expr>, Vec<Expr>),
    /// An axis step relative to the context node.
    Step(Step),
    /// `a/b`: `b` evaluated for each node `a` selects.
    Path(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    GeneralCompare(CompareOp, Box<Expr>, Box<Expr>),
    ValueCompare(CompareOp, Box<Expr>, Box<Expr>),
    NodeCompare(NodeOp, Box<Expr>, Box<Expr>),
    Arithmetic(ArithmeticOp, Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Range(Box<Expr>, Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    Intersect(Box<Expr>, Box<Expr>),
    Except(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `for $var in a return b` (several bindings are nested).
    For(String, Box<Expr>, Box<Expr>),
    /// `some`/`every $var in a satisfies b`; `true` for `every`.
    Quantified(bool, String, Box<Expr>, Box<Expr>),
    InstanceOf(Box<Expr>, SequenceType),
    /// `cast as T` / `castable as T`, with `?` allowing the empty sequence.
    Cast(Box<Expr>, AtomicType, bool),
    Castable(Box<Expr>, AtomicType, bool),
    Call(Function, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeOp {
    Is,
    Precedes,
    Follows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Child,
    Descendant,
    Attribute,
    Itself,
    DescendantOrSelf,
    FollowingSibling,
    Following,
    Parent,
    Ancestor,
    PrecedingSibling,
    Preceding,
    AncestorOrSelf,
}

impl Axis {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "child" => Self::Child,
            "descendant" => Self::Descendant,
            "attribute" => Self::Attribute,
            "self" => Self::Itself,
            "descendant-or-self" => Self::DescendantOrSelf,
            "following-sibling" => Self::FollowingSibling,
            "following" => Self::Following,
            "parent" => Self::Parent,
            "ancestor" => Self::Ancestor,
            "preceding-sibling" => Self::PrecedingSibling,
            "preceding" => Self::Preceding,
            "ancestor-or-self" => Self::AncestorOrSelf,
            _ => return None,
        })
    }

    /// Reverse axes number their nodes in reverse document order.
    pub(crate) fn is_reverse(self) -> bool {
        matches!(
            self,
            Self::Parent
                | Self::Ancestor
                | Self::PrecedingSibling
                | Self::Preceding
                | Self::AncestorOrSelf
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub axis: Axis,
    pub test: NodeTest,
    pub predicates: Vec<Expr>,
}

/// A name: `None` matches any namespace / any local name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NameTest {
    /// `Some(None)` is "no namespace"; `None` is any namespace (`*:local`).
    pub namespace: Option<Option<String>>,
    pub local: Option<String>,
}

impl NameTest {
    pub(crate) fn matches(&self, namespace: Option<&str>, local: &str) -> bool {
        self.local.as_deref().is_none_or(|name| name == local)
            && self
                .namespace
                .as_ref()
                .is_none_or(|uri| uri.as_deref() == namespace)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NodeTest {
    /// A name test, matching the axis's principal node kind.
    Name(NameTest),
    /// `node()`
    AnyNode,
    Text,
    Comment,
    ProcessingInstruction(Option<String>),
    /// `element()` / `element(name)`
    Element(Option<NameTest>),
    /// `attribute()` / `attribute(name)`
    Attribute(Option<NameTest>),
    /// `document-node()`
    Document,
}

#[derive(Clone, Debug)]
pub(crate) enum ItemType {
    Item,
    Node(NodeTest),
    Atomic(AtomicType),
}

#[derive(Clone, Debug)]
pub(crate) struct SequenceType {
    /// `None` is `empty-sequence()`.
    pub item: Option<ItemType>,
    pub min: usize,
    pub many: bool,
}

/// An XSLT match pattern: a union of path patterns.
#[derive(Clone, Debug)]
pub(crate) struct Pattern {
    pub alternatives: Vec<PathPattern>,
}

#[derive(Clone, Debug)]
pub(crate) struct PathPattern {
    /// Starts with `/`: the first step's parent must be the document node.
    /// (A bare `/` has no steps and matches the document node.)
    pub rooted: bool,
    pub steps: Vec<PatternStep>,
}

#[derive(Clone, Debug)]
pub(crate) struct PatternStep {
    /// Joined to the previous step (or the root) with `//` rather than `/`.
    pub descendant: bool,
    pub step: Step,
    /// A predicate may depend on the node's position among its siblings.
    pub positional: bool,
}

impl Expr {
    /// Whether evaluating this as a predicate might need the context
    /// position or size (`position()`, `last()`, or a numeric value).
    pub(crate) fn may_be_positional(&self) -> bool {
        match self {
            Self::Literal(atomic) => atomic.is_numeric(),
            Self::Arithmetic(..) | Self::Negate(_) | Self::Var(_) => true,
            Self::Call(function, args) => {
                function.uses_position()
                    || function.may_return_number()
                    || args.iter().any(Self::uses_focus_position)
            }
            other => other.uses_focus_position(),
        }
    }

    /// Whether `position()` or `last()` occurs outside any nested focus.
    fn uses_focus_position(&self) -> bool {
        match self {
            Self::Call(function, args) => {
                function.uses_position() || args.iter().any(Self::uses_focus_position)
            }
            Self::Step(_) | Self::Literal(_) | Self::Var(_) | Self::ContextItem | Self::Root => {
                false
            }
            Self::Path(left, _) => left.uses_focus_position(),
            Self::Filter(base, _) => base.uses_focus_position(),
            Self::Sequence(items) => items.iter().any(Self::uses_focus_position),
            Self::Or(a, b)
            | Self::And(a, b)
            | Self::GeneralCompare(_, a, b)
            | Self::ValueCompare(_, a, b)
            | Self::NodeCompare(_, a, b)
            | Self::Arithmetic(_, a, b)
            | Self::Range(a, b)
            | Self::Union(a, b)
            | Self::Intersect(a, b)
            | Self::Except(a, b)
            | Self::For(_, a, b)
            | Self::Quantified(_, _, a, b) => a.uses_focus_position() || b.uses_focus_position(),
            Self::If(a, b, c) => {
                a.uses_focus_position() || b.uses_focus_position() || c.uses_focus_position()
            }
            Self::Negate(a)
            | Self::InstanceOf(a, _)
            | Self::Cast(a, ..)
            | Self::Castable(a, ..) => a.uses_focus_position(),
        }
    }
}
