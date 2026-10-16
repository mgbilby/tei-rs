//! Hash-consed patterns for validation.
//!
//! The validator works on derivatives: every attribute, child element and
//! run of text turns the current pattern into a new one. Built as owned
//! [`Pattern`] trees, every derivative step cloned large parts of the
//! schema, and nothing learned while deriving one element could be reused
//! for the next element of the same type. Here a pattern is instead a
//! small [`Id`] into a [`PatternStore`] in which every distinct node exists
//! only once (hash-consing, as in James Clark's and Jing's validators).
//! Building a derivative only creates the few nodes that actually change,
//! and since equal patterns get equal ids, results such as "the derivative
//! of this content model by `<p>`" can be cached by id and looked up the
//! next time the same situation arises, which in TEI it constantly does.
//!
//! `ref`s are followed when a schema pattern is interned (they're always
//! transparent to validation), so the store never needs to look up a
//! definition by name while deriving. An `element`'s content is interned
//! only when a document element is actually validated against it, which
//! is what keeps recursive definitions finite.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

use crate::relaxng::grammar::Pattern;

pub(crate) type Definitions = std::collections::BTreeMap<String, Pattern>;

/// A `HashMap` with a fast, non-cryptographic hasher (the "FxHash" used
/// inside rustc). Keys here are small integers chosen by this crate, not by
/// documents, so hash flooding is not a concern.
pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;

#[derive(Clone, Copy, Default)]
pub(crate) struct FastHasher {
    hash: u64,
}

impl FastHasher {
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.add(u64::from_le_bytes(chunk.try_into().expect("8 bytes")));
        }
        for &byte in chunks.remainder() {
            self.add(u64::from(byte));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

/// A pattern in a [`PatternStore`]. Two ids are equal exactly when the
/// patterns they stand for were built the same way from the same parts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Id(u32);

impl Id {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

pub(crate) const EMPTY: Id = Id(0);
pub(crate) const NOT_ALLOWED: Id = Id(1);
pub(crate) const TEXT: Id = Id(2);

/// A leaf taken unchanged from the schema (`element`, `attribute`, `data`,
/// `value`), compared and hashed by address: the schema outlives the store,
/// and the same schema node always plays the same role.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Leaf<'s>(pub(crate) &'s Pattern);

impl PartialEq for Leaf<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Leaf<'_> {}

impl Hash for Leaf<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

/// One node of a hash-consed pattern: the simplified pattern forms the
/// validator understands, with operands as [`Id`]s.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Node<'s> {
    Empty,
    NotAllowed,
    Text,
    /// The schema's `element` pattern; its content is interned on demand
    /// ([`PatternStore::element_content`]).
    Element(Leaf<'s>),
    /// The schema's `attribute` pattern and its interned value pattern.
    Attribute(Leaf<'s>, Id),
    /// The schema's `data` pattern and its interned `except` patterns.
    Data(Leaf<'s>, Rc<[Id]>),
    Value(Leaf<'s>),
    List(Id),
    Group(Rc<[Id]>),
    Interleave(Rc<[Id]>),
    Choice(Rc<[Id]>),
    OneOrMore(Id),
}

/// The arena of hash-consed patterns built while validating one document.
pub(crate) struct PatternStore<'s> {
    nodes: Vec<Node<'s>>,
    ids: FastMap<Node<'s>, Id>,
    /// Schema subtrees already interned, by address.
    interned: FastMap<*const Pattern, Id>,
    element_contents: FastMap<Id, Id>,
    definitions: &'s Definitions,
}

impl<'s> PatternStore<'s> {
    pub(crate) fn new(definitions: &'s Definitions) -> Self {
        let mut store = Self {
            nodes: Vec::new(),
            ids: FastMap::default(),
            interned: FastMap::default(),
            element_contents: FastMap::default(),
            definitions,
        };
        for (node, id) in [
            (Node::Empty, EMPTY),
            (Node::NotAllowed, NOT_ALLOWED),
            (Node::Text, TEXT),
        ] {
            assert_eq!(store.make(node), id);
        }
        store
    }

    pub(crate) fn node(&self, id: Id) -> &Node<'s> {
        &self.nodes[id.index()]
    }

    /// The id of `node`, adding it to the store if it isn't there yet.
    fn make(&mut self, node: Node<'s>) -> Id {
        if let Some(&id) = self.ids.get(&node) {
            return id;
        }
        let id = Id(u32::try_from(self.nodes.len()).expect("fewer than 2^32 patterns"));
        self.nodes.push(node.clone());
        self.ids.insert(node, id);
        id
    }

    /// Interns a simplified schema pattern, structure unchanged except that
    /// `ref`s are replaced by their definitions. Terminates because §4.19
    /// (checked at compile time) rules out `ref` cycles that don't pass
    /// through an `element`, and element content is not interned here.
    pub(crate) fn intern(&mut self, pattern: &'s Pattern) -> Id {
        let key = pattern as *const Pattern;
        if let Some(&id) = self.interned.get(&key) {
            return id;
        }
        let node = match pattern {
            Pattern::Empty => Node::Empty,
            Pattern::NotAllowed => Node::NotAllowed,
            Pattern::Text => Node::Text,
            Pattern::Element { .. } => Node::Element(Leaf(pattern)),
            Pattern::Attribute { body, .. } => {
                Node::Attribute(Leaf(pattern), self.intern(&body[0]))
            }
            Pattern::Data { except, .. } => Node::Data(Leaf(pattern), self.intern_all(except)),
            Pattern::Value { .. } => Node::Value(Leaf(pattern)),
            Pattern::List(values) => Node::List(self.intern(&values[0])),
            Pattern::Group(values) => Node::Group(self.intern_all(values)),
            Pattern::Interleave(values) => Node::Interleave(self.intern_all(values)),
            Pattern::Choice(values) => Node::Choice(self.intern_all(values)),
            Pattern::OneOrMore(values) => Node::OneOrMore(self.intern(&values[0])),
            Pattern::Ref(name) => {
                let definitions = self.definitions;
                let target = definitions
                    .get(name)
                    .expect("RELAX NG §4.18 guarantees every ref target exists");
                let id = self.intern(target);
                self.interned.insert(key, id);
                return id;
            }
            Pattern::Optional(_)
            | Pattern::ZeroOrMore(_)
            | Pattern::Mixed(_)
            | Pattern::ExternalRef { .. }
            | Pattern::ParentRef(_)
            | Pattern::Grammar(_) => {
                unreachable!("resolved into other patterns by simplify_pattern")
            }
        };
        let id = self.make(node);
        self.interned.insert(key, id);
        id
    }

    fn intern_all(&mut self, patterns: &'s [Pattern]) -> Rc<[Id]> {
        patterns
            .iter()
            .map(|pattern| self.intern(pattern))
            .collect()
    }

    /// The content pattern of the `element` node `id`.
    pub(crate) fn element_content(&mut self, id: Id) -> Id {
        if let Some(&content) = self.element_contents.get(&id) {
            return content;
        }
        let Node::Element(Leaf(Pattern::Element { body, .. })) = self.node(id) else {
            unreachable!("element_content called on an element node")
        };
        let content = self.intern(&body[0]);
        self.element_contents.insert(id, content);
        content
    }

    /// Builds a normalized `group`, exactly as [`crate::relaxng::simplify`]'s
    /// `group` does for owned patterns.
    pub(crate) fn group(&mut self, values: Vec<Id>) -> Id {
        self.sequence(values, Node::Group)
    }

    /// Builds a normalized `interleave`, like [`Self::group`].
    pub(crate) fn interleave(&mut self, values: Vec<Id>) -> Id {
        self.sequence(values, Node::Interleave)
    }

    /// `notAllowed` anywhere collapses the whole sequence; `empty` operands
    /// are dropped when there are others; one operand stands for itself.
    pub(crate) fn sequence(&mut self, mut values: Vec<Id>, wrap: fn(Rc<[Id]>) -> Node<'s>) -> Id {
        if values.contains(&NOT_ALLOWED) {
            return NOT_ALLOWED;
        }
        if values.len() > 1 {
            values.retain(|&value| value != EMPTY);
        }
        match values.len() {
            0 => EMPTY,
            1 => values[0],
            _ => self.make(wrap(values.into())),
        }
    }

    /// Builds a normalized `choice`: nested choices flattened, `notAllowed`
    /// dropped, duplicates removed. Operands are also sorted, which changes
    /// nothing about what the choice matches but lets the same set of
    /// alternatives reached in a different order share one id (and so one
    /// cache entry).
    pub(crate) fn choice(&mut self, values: Vec<Id>) -> Id {
        let mut flat = Vec::with_capacity(values.len());
        for value in values {
            match self.node(value) {
                Node::Choice(nested) => flat.extend(nested.iter().copied()),
                _ => flat.push(value),
            }
        }
        flat.retain(|&value| value != NOT_ALLOWED);
        flat.sort_unstable();
        flat.dedup();
        match flat.len() {
            0 => NOT_ALLOWED,
            1 => flat[0],
            _ => self.make(Node::Choice(flat.into())),
        }
    }

    /// Rebuilds an owned [`Pattern`] equivalent to `id` (with `ref`s
    /// already expanded) — for unit tests that inspect derivatives.
    #[cfg(test)]
    pub(crate) fn to_pattern(&self, id: Id) -> Pattern {
        let all = |values: &[Id]| values.iter().map(|&value| self.to_pattern(value)).collect();
        match self.node(id) {
            Node::Empty => Pattern::Empty,
            Node::NotAllowed => Pattern::NotAllowed,
            Node::Text => Pattern::Text,
            Node::Element(leaf)
            | Node::Attribute(leaf, _)
            | Node::Data(leaf, _)
            | Node::Value(leaf) => leaf.0.clone(),
            Node::List(inner) => Pattern::List(vec![self.to_pattern(*inner)]),
            Node::Group(values) => Pattern::Group(all(values)),
            Node::Interleave(values) => Pattern::Interleave(all(values)),
            Node::Choice(values) => Pattern::Choice(all(values)),
            Node::OneOrMore(inner) => Pattern::OneOrMore(vec![self.to_pattern(*inner)]),
        }
    }
}
