//! Evaluates compiled expressions and matches patterns against a parsed
//! document.

use std::cell::OnceCell;
use std::collections::HashMap;

use super::XError;
use super::ast::*;
use super::datetime::{self, DateTime};
use super::functions;
use super::resources::Resources;
use super::value::{Atomic, AtomicType, Item, Seq, XML_NS, XNode};

type Result<T> = std::result::Result<T, XError>;

/// A parsed document, with the lookups some functions need built lazily.
pub(crate) struct Document<'a> {
    pub(crate) doc: &'a roxmltree::Document<'a>,
    /// What `document-uri()` returns for the document node.
    pub(crate) uri: Option<String>,
    /// Where the document comes in document order among the documents an
    /// evaluation uses: 0 for the validated one, then in the order the
    /// others were read.
    pub(crate) rank: u32,
    ids: OnceCell<HashMap<&'a str, roxmltree::Node<'a, 'a>>>,
    node_count: OnceCell<u32>,
    /// The current date and time, fixed when first asked for so that every
    /// `current-dateTime()` while checking the document agrees.
    pub(crate) now: OnceCell<DateTime>,
}

impl<'a> Document<'a> {
    pub(crate) fn new(doc: &'a roxmltree::Document<'a>, uri: Option<String>) -> Self {
        Self {
            doc,
            uri,
            rank: 0,
            ids: OnceCell::new(),
            node_count: OnceCell::new(),
            now: OnceCell::new(),
        }
    }

    pub(crate) fn now(&self) -> DateTime {
        *self.now.get_or_init(DateTime::now)
    }

    pub(crate) fn root(&self) -> XNode<'a> {
        XNode::Node(self.doc.root())
    }

    /// The element whose `xml:id` is `id` (the first, if repeated).
    pub(crate) fn element_by_id(&self, id: &str) -> Option<roxmltree::Node<'a, 'a>> {
        self.ids
            .get_or_init(|| {
                let mut ids = HashMap::new();
                for node in self.doc.descendants().filter(|node| node.is_element()) {
                    if let Some(id) = node.attribute((XML_NS, "id")) {
                        ids.entry(id).or_insert(node);
                    }
                }
                ids
            })
            .get(id)
            .copied()
    }

    fn node_count(&self) -> u32 {
        *self.node_count.get_or_init(|| {
            self.doc
                .root()
                .descendants()
                .next_back()
                .map_or(1, |node| node.id().get() + 1)
        })
    }

    fn node(&self, id: u32) -> Option<roxmltree::Node<'a, 'a>> {
        self.doc.get_node(roxmltree::NodeId::new(id))
    }
}

/// The focus: context item, position and size.
#[derive(Clone)]
pub(crate) struct Focus<'a> {
    pub item: Item<'a>,
    pub position: usize,
    pub size: usize,
}

impl<'a> Focus<'a> {
    pub(crate) fn node(node: XNode<'a>) -> Self {
        Self {
            item: Item::Node(node),
            position: 1,
            size: 1,
        }
    }
}

/// Evaluation state: the document, `current()`, and variable bindings.
pub(crate) struct Evaluator<'d, 'a> {
    pub(crate) doc: &'d Document<'a>,
    /// The other documents `doc()` and `document()` read, if they may.
    pub(crate) resources: Option<&'a Resources<'a>>,
    pub(crate) current: Option<XNode<'a>>,
    variables: Vec<(String, Seq<'a>)>,
    /// `for`/`some`/`every`/`!` iterations so far, against [`MAX_ITERATIONS`].
    iterations: u64,
}

/// How many `for`/`some`/`every`/`!` iterations all the tests on one document
/// may run, and how long a sequence a `for` or `!` may build: enough for any
/// real rule,
/// while a hostile one fails with an error instead of running for hours or
/// exhausting memory.
const MAX_ITERATIONS: u64 = 100_000_000;
const MAX_SEQUENCE: usize = 10_000_000;

impl<'d, 'a> Evaluator<'d, 'a> {
    pub(crate) fn new(doc: &'d Document<'a>) -> Self {
        Self {
            doc,
            resources: None,
            current: None,
            variables: Vec::new(),
            iterations: 0,
        }
    }

    /// An evaluator whose `doc()` and `document()` read from `resources`.
    pub(crate) fn with_resources(doc: &'d Document<'a>, resources: &'a Resources<'a>) -> Self {
        Self {
            resources: Some(resources),
            ..Self::new(doc)
        }
    }

    fn count_iteration(&mut self) -> Result<()> {
        self.iterations += 1;
        if self.iterations > MAX_ITERATIONS {
            return Err(XError::new(
                "FOER0000",
                format!("more than {MAX_ITERATIONS} iterations of for/some/every/!"),
            ));
        }
        Ok(())
    }

    pub(crate) fn bind(&mut self, name: &str, value: Seq<'a>) {
        self.variables.push((name.to_owned(), value));
    }

    pub(crate) fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Drops the bindings made since the count was `count`.
    pub(crate) fn unbind_to(&mut self, count: usize) {
        self.variables.truncate(count);
    }

    /// Evaluates `expr` and takes its effective boolean value.
    pub(crate) fn boolean(&mut self, expr: &Expr, focus: Option<&Focus<'a>>) -> Result<bool> {
        match expr {
            // `@name` as a condition (the commonest test in TEI rules):
            // check for the attribute without building a sequence.
            Expr::Step(Step {
                axis: Axis::Attribute,
                test: NodeTest::Name(name),
                predicates,
            }) if predicates.is_empty() => {
                let node = context_node(focus)?;
                Ok(match node {
                    XNode::Node(node) if node.is_element() => node
                        .attributes()
                        .any(|attribute| name.matches(attribute.namespace(), attribute.name())),
                    _ => false,
                })
            }
            // `self::x`, `parent::x`, `child::x`, `ancestor::x`: stop at the
            // first node found.
            Expr::Step(Step {
                axis: axis @ (Axis::Itself | Axis::Parent | Axis::Child | Axis::Ancestor),
                test,
                predicates,
            }) if predicates.is_empty() => {
                let node = context_node(focus)?;
                let found = match axis {
                    Axis::Itself => node_test(test, node, false),
                    Axis::Parent => node
                        .parent()
                        .is_some_and(|parent| node_test(test, parent, false)),
                    Axis::Child => match node {
                        XNode::Node(node) => node
                            .children()
                            .any(|child| node_test(test, XNode::Node(child), false)),
                        XNode::Attr(..) => false,
                    },
                    _ => std::iter::successors(node.parent(), |ancestor| ancestor.parent())
                        .any(|ancestor| node_test(test, ancestor, false)),
                };
                Ok(found)
            }
            Expr::And(left, right) => Ok(self.boolean(left, focus)? && self.boolean(right, focus)?),
            Expr::Or(left, right) => Ok(self.boolean(left, focus)? || self.boolean(right, focus)?),
            Expr::Call(functions::Function::Not, args) => Ok(!self.boolean(&args[0], focus)?),
            _ => {
                let value = self.eval(expr, focus)?;
                effective_boolean(&value)
            }
        }
    }

    pub(crate) fn eval(&mut self, expr: &Expr, focus: Option<&Focus<'a>>) -> Result<Seq<'a>> {
        match expr {
            Expr::Literal(value) => Ok(vec![Item::Atomic(value.clone())]),
            Expr::Var(name) => self
                .variables
                .iter()
                .rev()
                .find(|(bound, _)| bound == name)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| XError::new("XPST0008", format!("variable `${name}` is not bound"))),
            Expr::ContextItem => Ok(vec![context_item(focus)?.clone()]),
            Expr::Root => Ok(vec![Item::Node(context_node(focus)?.root())]),
            Expr::Sequence(items) => {
                let mut out = Vec::new();
                for item in items {
                    out.extend(self.eval(item, focus)?);
                }
                Ok(out)
            }
            Expr::Filter(base, predicates) => {
                let items = self.eval(base, focus)?;
                self.filter(items, predicates)
            }
            Expr::Step(step) => {
                let node = context_node(focus)?;
                self.step(node, step)
            }
            Expr::Path(left, right) => self.path(left, right, focus),
            Expr::Or(left, right) => Ok(boolean(
                self.boolean(left, focus)? || self.boolean(right, focus)?,
            )),
            Expr::And(left, right) => Ok(boolean(
                self.boolean(left, focus)? && self.boolean(right, focus)?,
            )),
            Expr::GeneralCompare(op, left, right) => {
                let left = atomize(self.eval(left, focus)?);
                let right = atomize(self.eval(right, focus)?);
                for a in &left {
                    for b in &right {
                        if general_compare(*op, a, b)? {
                            return Ok(boolean(true));
                        }
                    }
                }
                Ok(boolean(false))
            }
            Expr::ValueCompare(op, left, right) => {
                let (Some(a), Some(b)) = (
                    self.single_atomic(left, focus)?,
                    self.single_atomic(right, focus)?,
                ) else {
                    return Ok(Vec::new());
                };
                let a = untyped_to_string(a);
                let b = untyped_to_string(b);
                Ok(boolean(value_compare(*op, &a, &b)?))
            }
            Expr::NodeCompare(op, left, right) => {
                let (Some(a), Some(b)) = (
                    self.single_node(left, focus)?,
                    self.single_node(right, focus)?,
                ) else {
                    return Ok(Vec::new());
                };
                Ok(boolean(match op {
                    NodeOp::Is => a == b,
                    NodeOp::Precedes => self.order(a) < self.order(b),
                    NodeOp::Follows => self.order(a) > self.order(b),
                }))
            }
            Expr::Arithmetic(op, left, right) => {
                let (Some(a), Some(b)) = (
                    self.single_atomic(left, focus)?,
                    self.single_atomic(right, focus)?,
                ) else {
                    return Ok(Vec::new());
                };
                Ok(vec![Item::Atomic(arithmetic(*op, a, b)?)])
            }
            Expr::Negate(operand) => {
                let Some(value) = self.single_atomic(operand, focus)? else {
                    return Ok(Vec::new());
                };
                Ok(vec![Item::Atomic(match numeric_operand(value)? {
                    Atomic::Int(value) => Atomic::Int(-value),
                    Atomic::Dec(value) => Atomic::Dec(-value),
                    Atomic::Dbl(value) => Atomic::Dbl(-value),
                    _ => unreachable!("numeric"),
                })])
            }
            Expr::Range(low, high) => {
                let (Some(low), Some(high)) = (
                    self.single_atomic(low, focus)?,
                    self.single_atomic(high, focus)?,
                ) else {
                    return Ok(Vec::new());
                };
                let low = integer_operand(low)?;
                let high = integer_operand(high)?;
                if high.saturating_sub(low) > 10_000_000 {
                    return Err(XError::new("FOAR0002", "range too large"));
                }
                Ok((low..=high)
                    .map(|value| Item::Atomic(Atomic::Int(value)))
                    .collect())
            }
            Expr::Union(left, right) => {
                let mut nodes = self.nodes(left, focus)?;
                nodes.extend(self.nodes(right, focus)?);
                Ok(self.in_document_order(nodes))
            }
            Expr::Intersect(left, right) | Expr::Except(left, right) => {
                let left = self.nodes(left, focus)?;
                let right = self.nodes(right, focus)?;
                let keep = matches!(expr, Expr::Intersect(..));
                Ok(self.in_document_order(
                    left.into_iter()
                        .filter(|node| right.contains(node) == keep)
                        .collect(),
                ))
            }
            Expr::If(condition, then, otherwise) => {
                if self.boolean(condition, focus)? {
                    self.eval(then, focus)
                } else {
                    self.eval(otherwise, focus)
                }
            }
            Expr::For(name, domain, body) => {
                let domain = self.eval(domain, focus)?;
                let mut out = Vec::new();
                for item in domain {
                    self.count_iteration()?;
                    self.variables.push((name.clone(), vec![item]));
                    let result = self.eval(body, focus);
                    self.variables.pop();
                    out.extend(result?);
                    if out.len() > MAX_SEQUENCE {
                        return Err(XError::new(
                            "FOER0000",
                            format!("sequence longer than {MAX_SEQUENCE} items"),
                        ));
                    }
                }
                Ok(out)
            }
            Expr::Map(left, right) => {
                let items = self.eval(left, focus)?;
                let size = items.len();
                let mut out = Vec::new();
                for (index, item) in items.into_iter().enumerate() {
                    self.count_iteration()?;
                    let focus = Focus {
                        item,
                        position: index + 1,
                        size,
                    };
                    out.extend(self.eval(right, Some(&focus))?);
                    if out.len() > MAX_SEQUENCE {
                        return Err(XError::new(
                            "FOER0000",
                            format!("sequence longer than {MAX_SEQUENCE} items"),
                        ));
                    }
                }
                Ok(out)
            }
            Expr::Let(name, value, body) => {
                let value = self.eval(value, focus)?;
                self.variables.push((name.clone(), value));
                let result = self.eval(body, focus);
                self.variables.pop();
                result
            }
            Expr::Quantified(every, name, domain, body) => {
                let domain = self.eval(domain, focus)?;
                for item in domain {
                    self.count_iteration()?;
                    self.variables.push((name.clone(), vec![item]));
                    let result = self.boolean(body, focus);
                    self.variables.pop();
                    if result? != *every {
                        return Ok(boolean(!every));
                    }
                }
                Ok(boolean(*every))
            }
            Expr::InstanceOf(operand, sequence_type) => {
                let value = self.eval(operand, focus)?;
                Ok(boolean(instance_of(&value, sequence_type)))
            }
            Expr::Cast(operand, target, optional) => {
                let value = atomize(self.eval(operand, focus)?);
                match value.as_slice() {
                    [] if *optional => Ok(Vec::new()),
                    [] => Err(XError::new("XPTY0004", "cast of an empty sequence")),
                    [value] => Ok(vec![Item::Atomic(value.cast(*target)?)]),
                    _ => Err(XError::new(
                        "XPTY0004",
                        "cast of a sequence of more than one item",
                    )),
                }
            }
            Expr::Castable(operand, target, optional) => {
                let value = atomize(self.eval(operand, focus)?);
                Ok(boolean(match value.as_slice() {
                    [] => *optional,
                    [value] => value.cast(*target).is_ok(),
                    _ => false,
                }))
            }
            Expr::Call(function, args) => functions::call(self, function, args, focus),
        }
    }

    fn path(&mut self, left: &Expr, right: &Expr, focus: Option<&Focus<'a>>) -> Result<Seq<'a>> {
        let context = self.eval(left, focus)?;
        let size = context.len();
        let mut out: Seq<'a> = Vec::new();
        let mut all_nodes = true;
        let mut any_nodes = false;
        for (index, item) in context.into_iter().enumerate() {
            if !matches!(item, Item::Node(_)) {
                return Err(XError::new(
                    "XPTY0019",
                    "a path step applied to an atomic value",
                ));
            }
            let focus = Focus {
                item,
                position: index + 1,
                size,
            };
            let result = self.eval(right, Some(&focus))?;
            for item in &result {
                match item {
                    Item::Node(_) => any_nodes = true,
                    Item::Atomic(_) => all_nodes = false,
                }
            }
            out.extend(result);
        }
        if any_nodes && !all_nodes {
            return Err(XError::new(
                "XPTY0018",
                "a path yields both nodes and atomic values",
            ));
        }
        if any_nodes && size > 1 {
            let nodes = out
                .into_iter()
                .map(|item| match item {
                    Item::Node(node) => node,
                    Item::Atomic(_) => unreachable!("all nodes"),
                })
                .collect();
            return Ok(self.in_document_order(nodes));
        }
        Ok(out)
    }

    fn filter(&mut self, mut items: Seq<'a>, predicates: &[Expr]) -> Result<Seq<'a>> {
        for predicate in predicates {
            if let Expr::Literal(Atomic::Int(position)) = predicate {
                items = usize::try_from(*position)
                    .ok()
                    .and_then(|position| position.checked_sub(1))
                    .and_then(|index| items.get(index).cloned())
                    .into_iter()
                    .collect();
                continue;
            }
            if let Some(kept) = self.filter_by_fixed_comparison(&items, predicate)? {
                items = kept;
                continue;
            }
            let size = items.len();
            let mut kept = Vec::new();
            for (index, item) in items.into_iter().enumerate() {
                let focus = Focus {
                    item,
                    position: index + 1,
                    size,
                };
                let keep = if never_numeric(predicate) {
                    self.boolean(predicate, Some(&focus))?
                } else {
                    let value = self.eval(predicate, Some(&focus))?;
                    predicate_truth(&value, index + 1)?
                };
                if keep {
                    kept.push(focus.item);
                }
            }
            items = kept;
        }
        Ok(items)
    }

    /// Filters `items` by a general comparison one side of which doesn't
    /// depend on the item being tested, such as TEI's
    /// `following::*[@xml:id = substring(current()/@spanTo, 2)]`: that side
    /// is evaluated once instead of once per item. `None` if `predicate`
    /// isn't such a comparison (or the fixed side can't be evaluated on its
    /// own), and the caller filters as usual.
    fn filter_by_fixed_comparison(
        &mut self,
        items: &Seq<'a>,
        predicate: &Expr,
    ) -> Result<Option<Seq<'a>>> {
        let Expr::GeneralCompare(op, left, right) = predicate else {
            return Ok(None);
        };
        let (left_fixed, right_fixed) = (!uses_focus(left), !uses_focus(right));
        if items.is_empty() || left_fixed == right_fixed {
            return Ok(None);
        }
        let (fixed, varying) = if left_fixed {
            (left, right)
        } else {
            (right, left)
        };
        let Ok(fixed) = self.eval(fixed, None) else {
            return Ok(None);
        };
        let fixed = atomize(fixed);
        let size = items.len();
        let mut kept = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let focus = Focus {
                item: item.clone(),
                position: index + 1,
                size,
            };
            let varying = atomize(self.eval(varying, Some(&focus))?);
            let (outer, inner) = if left_fixed {
                (&fixed, &varying)
            } else {
                (&varying, &fixed)
            };
            let mut keep = false;
            'pairs: for a in outer {
                for b in inner {
                    if general_compare(*op, a, b)? {
                        keep = true;
                        break 'pairs;
                    }
                }
            }
            if keep {
                kept.push(focus.item);
            }
        }
        Ok(Some(kept))
    }

    fn step(&mut self, node: XNode<'a>, step: &Step) -> Result<Seq<'a>> {
        let mut nodes = Vec::new();
        self.axis(node, step.axis, &step.test, &mut nodes);
        let items: Seq<'a> = nodes.into_iter().map(Item::Node).collect();
        let mut items = self.filter(items, &step.predicates)?;
        if step.axis.is_reverse() {
            items.reverse();
        }
        Ok(items)
    }

    /// Appends the nodes on `axis` from `node` that pass `test`, in axis
    /// order.
    pub(crate) fn axis(
        &self,
        node: XNode<'a>,
        axis: Axis,
        test: &NodeTest,
        out: &mut Vec<XNode<'a>>,
    ) {
        let attribute_axis = axis == Axis::Attribute;
        let mut push = |candidate: XNode<'a>| {
            if node_test(test, candidate, attribute_axis) {
                out.push(candidate);
            }
        };
        match (axis, node) {
            (Axis::Child, XNode::Node(node)) => node.children().map(XNode::Node).for_each(push),
            (Axis::Attribute, XNode::Node(node)) if node.is_element() => {
                (0..node.attributes().len() as u32).for_each(|index| push(XNode::Attr(node, index)))
            }
            (Axis::Itself, _) => push(node),
            (Axis::Descendant, XNode::Node(node)) => {
                node.descendants().skip(1).map(XNode::Node).for_each(push)
            }
            (Axis::DescendantOrSelf, XNode::Node(node)) => {
                node.descendants().map(XNode::Node).for_each(push)
            }
            (Axis::DescendantOrSelf, XNode::Attr(..)) => push(node),
            (Axis::Parent, _) => {
                if let Some(parent) = node.parent() {
                    push(parent)
                }
            }
            (Axis::Ancestor | Axis::AncestorOrSelf, _) => {
                if axis == Axis::AncestorOrSelf {
                    push(node);
                }
                let mut next = node.parent();
                while let Some(ancestor) = next {
                    push(ancestor);
                    next = ancestor.parent();
                }
            }
            (Axis::FollowingSibling, XNode::Node(node)) => {
                node.next_siblings().skip(1).map(XNode::Node).for_each(push)
            }
            (Axis::PrecedingSibling, XNode::Node(node)) => {
                node.prev_siblings().skip(1).map(XNode::Node).for_each(push)
            }
            (Axis::Following, _) => {
                let start = match node {
                    // An attribute's following nodes include its element's
                    // descendants.
                    XNode::Attr(owner, _) => owner.id().get() + 1,
                    XNode::Node(node) => last_descendant(node).id().get() + 1,
                };
                let doc = self.document_of(node);
                for id in start..doc.node_count() {
                    if let Some(node) = doc.node(id) {
                        push(XNode::Node(node));
                    }
                }
            }
            (Axis::Preceding, _) => {
                let element = match node {
                    XNode::Attr(owner, _) => owner,
                    XNode::Node(node) => node,
                };
                let ancestors: Vec<u32> =
                    element.ancestors().skip(1).map(|a| a.id().get()).collect();
                let mut next_ancestor = 0;
                let doc = self.document_of(node);
                for id in (0..element.id().get()).rev() {
                    if ancestors.get(next_ancestor) == Some(&id) {
                        next_ancestor += 1;
                        continue;
                    }
                    if let Some(node) = doc.node(id) {
                        push(XNode::Node(node));
                    }
                }
            }
            _ => {}
        }
    }

    fn single_atomic(&mut self, expr: &Expr, focus: Option<&Focus<'a>>) -> Result<Option<Atomic>> {
        let mut value = atomize(self.eval(expr, focus)?);
        match value.len() {
            0 => Ok(None),
            1 => Ok(value.pop()),
            _ => Err(XError::new(
                "XPTY0004",
                "a sequence of more than one item where one value is required",
            )),
        }
    }

    fn single_node(&mut self, expr: &Expr, focus: Option<&Focus<'a>>) -> Result<Option<XNode<'a>>> {
        let mut nodes = self.nodes(expr, focus)?;
        match nodes.len() {
            0 => Ok(None),
            1 => Ok(nodes.pop()),
            _ => Err(XError::new(
                "XPTY0004",
                "more than one node where one is required",
            )),
        }
    }

    pub(crate) fn nodes(
        &mut self,
        expr: &Expr,
        focus: Option<&Focus<'a>>,
    ) -> Result<Vec<XNode<'a>>> {
        self.eval(expr, focus)?
            .into_iter()
            .map(|item| match item {
                Item::Node(node) => Ok(node),
                Item::Atomic(_) => Err(XError::new(
                    "XPTY0004",
                    "an atomic value where a node is required",
                )),
            })
            .collect()
    }

    // --- Patterns -------------------------------------------------------

    /// Whether `node` matches the pattern. Within the pattern, `current()`
    /// is the node being matched.
    pub(crate) fn matches(&mut self, pattern: &Pattern, node: XNode<'a>) -> Result<bool> {
        let current = self.current.replace(node);
        let mut result = Ok(false);
        for alternative in &pattern.alternatives {
            result = self.matches_path(alternative, alternative.steps.len(), node);
            if !matches!(result, Ok(false)) {
                break;
            }
        }
        self.current = current;
        result
    }

    /// Whether `node` matches the first `count` steps of `path`.
    fn matches_path(&mut self, path: &PathPattern, count: usize, node: XNode<'a>) -> Result<bool> {
        if count == 0 {
            // Only a rooted pattern reaches past its first step, and then
            // it needs the document node.
            return Ok(path.rooted && matches!(node, XNode::Node(node) if node.is_root()));
        }
        let pattern_step = &path.steps[count - 1];
        if !self.matches_step(pattern_step, node)? {
            return Ok(false);
        }
        if count == 1 && !path.rooted {
            return Ok(true);
        }
        let Some(parent) = node.parent() else {
            return Ok(false);
        };
        if !pattern_step.descendant {
            return self.matches_path(path, count - 1, parent);
        }
        if count == 1 {
            // `//step` (rooted): any node in a document tree qualifies.
            return Ok(true);
        }
        let mut ancestor = Some(parent);
        while let Some(candidate) = ancestor {
            if self.matches_path(path, count - 1, candidate)? {
                return Ok(true);
            }
            ancestor = candidate.parent();
        }
        Ok(false)
    }

    fn matches_step(&mut self, pattern_step: &PatternStep, node: XNode<'a>) -> Result<bool> {
        let step = &pattern_step.step;
        let attribute_axis = step.axis == Axis::Attribute;
        if attribute_axis != node.is_attribute() || !node_test(&step.test, node, attribute_axis) {
            return Ok(false);
        }
        if step.predicates.is_empty() {
            return Ok(true);
        }
        if !pattern_step.positional {
            let focus = Focus::node(node);
            let mut needs_position = false;
            for predicate in &step.predicates {
                // These never yield a number, so need no position.
                if never_numeric(predicate) {
                    if !self.boolean(predicate, Some(&focus))? {
                        return Ok(false);
                    }
                    continue;
                }
                let value = self.eval(predicate, Some(&focus))?;
                if let [Item::Atomic(value)] = value.as_slice()
                    && value.is_numeric()
                {
                    needs_position = true;
                    break;
                }
                if !effective_boolean(&value)? {
                    return Ok(false);
                }
            }
            if !needs_position {
                return Ok(true);
            }
        }
        // Evaluate the step from the parent, as the pattern's path would,
        // so positions count among the matching siblings.
        let Some(parent) = node.parent() else {
            return Ok(false);
        };
        let selected = self.step(parent, step)?;
        Ok(selected.contains(&Item::Node(node)))
    }
}

/// Whether a predicate's value can never be a number, so it is a plain
/// condition (the focus it is evaluated with has the right position).
fn never_numeric(predicate: &Expr) -> bool {
    matches!(
        predicate,
        Expr::Step(_)
            | Expr::Path(..)
            | Expr::And(..)
            | Expr::Or(..)
            | Expr::GeneralCompare(..)
            | Expr::ValueCompare(..)
            | Expr::NodeCompare(..)
            | Expr::Call(functions::Function::Not, _)
    )
}

/// Whether evaluating `expr` can depend on the focus (context item,
/// position or size). Conservative: any step, `.`, `/`, and every function
/// that can use the context item implicitly count as depending on it.
fn uses_focus(expr: &Expr) -> bool {
    use functions::Function as F;
    match expr {
        Expr::Literal(_) | Expr::Var(_) => false,
        Expr::ContextItem | Expr::Root | Expr::Step(_) => true,
        Expr::Sequence(items) => items.iter().any(uses_focus),
        // The predicates and the right-hand side have their own focus.
        Expr::Filter(primary, _) | Expr::Path(primary, _) | Expr::Map(primary, _) => {
            uses_focus(primary)
        }
        Expr::Or(a, b)
        | Expr::And(a, b)
        | Expr::GeneralCompare(_, a, b)
        | Expr::ValueCompare(_, a, b)
        | Expr::NodeCompare(_, a, b)
        | Expr::Arithmetic(_, a, b)
        | Expr::Range(a, b)
        | Expr::Union(a, b)
        | Expr::Intersect(a, b)
        | Expr::Except(a, b)
        | Expr::For(_, a, b)
        | Expr::Let(_, a, b)
        | Expr::Quantified(_, _, a, b) => uses_focus(a) || uses_focus(b),
        Expr::If(a, b, c) => uses_focus(a) || uses_focus(b) || uses_focus(c),
        Expr::Negate(a)
        | Expr::InstanceOf(a, _)
        | Expr::Cast(a, _, _)
        | Expr::Castable(a, _, _) => uses_focus(a),
        Expr::Call(function, args) => {
            matches!(
                function,
                F::Position
                    | F::Last
                    | F::Name
                    | F::LocalName
                    | F::NamespaceUri
                    | F::NormalizeSpace
                    | F::Number
                    | F::Root
                    | F::String
                    | F::StringLength
                    | F::Data
                    | F::Lang
                    | F::Id
                    | F::GenerateId
                    | F::DocumentUri
                    | F::BaseUri
                    | F::HasChildren
                    | F::Path
                    | F::Trace
            ) || args.iter().any(uses_focus)
        }
    }
}

fn last_descendant<'a>(node: roxmltree::Node<'a, 'a>) -> roxmltree::Node<'a, 'a> {
    let mut last = node;
    while let Some(child) = last.last_child() {
        last = child;
    }
    last
}

pub(crate) fn node_test(test: &NodeTest, node: XNode<'_>, attribute_axis: bool) -> bool {
    match test {
        NodeTest::Name(name) => {
            (if attribute_axis {
                node.is_attribute()
            } else {
                node.is_element()
            }) && node
                .expanded_name()
                .is_some_and(|(namespace, local)| name.matches(namespace, local))
        }
        NodeTest::AnyNode => true,
        NodeTest::Text => matches!(node, XNode::Node(node) if node.is_text()),
        NodeTest::Comment => matches!(node, XNode::Node(node) if node.is_comment()),
        NodeTest::ProcessingInstruction(target) => match node {
            XNode::Node(node) => node
                .pi()
                .is_some_and(|pi| target.as_deref().is_none_or(|target| pi.target == target)),
            XNode::Attr(..) => false,
        },
        NodeTest::Element(name) => {
            node.is_element()
                && name.as_ref().is_none_or(|name| {
                    node.expanded_name()
                        .is_some_and(|(namespace, local)| name.matches(namespace, local))
                })
        }
        NodeTest::Attribute(name) => {
            node.is_attribute()
                && name.as_ref().is_none_or(|name| {
                    node.expanded_name()
                        .is_some_and(|(namespace, local)| name.matches(namespace, local))
                })
        }
        NodeTest::Document => matches!(node, XNode::Node(node) if node.is_root()),
    }
}

pub(crate) fn context_item<'f, 'a>(focus: Option<&'f Focus<'a>>) -> Result<&'f Item<'a>> {
    focus
        .map(|focus| &focus.item)
        .ok_or_else(|| XError::new("XPDY0002", "there is no context item"))
}

pub(crate) fn context_node<'a>(focus: Option<&Focus<'a>>) -> Result<XNode<'a>> {
    match context_item(focus)? {
        Item::Node(node) => Ok(*node),
        Item::Atomic(_) => Err(XError::new("XPTY0020", "the context item is not a node")),
    }
}

pub(crate) fn boolean<'a>(value: bool) -> Seq<'a> {
    vec![Item::Atomic(Atomic::Bool(value))]
}

pub(crate) fn atomize(items: Seq<'_>) -> Vec<Atomic> {
    items
        .into_iter()
        .map(|item| match item {
            Item::Node(node) => Atomic::Untyped(node.string_value()),
            Item::Atomic(atomic) => atomic,
        })
        .collect()
}

pub(crate) fn sorted_nodes(mut nodes: Vec<XNode<'_>>) -> Seq<'_> {
    nodes.sort_by_key(|node| node.order());
    nodes.dedup();
    nodes.into_iter().map(Item::Node).collect()
}

pub(crate) fn effective_boolean(value: &[Item<'_>]) -> Result<bool> {
    match value {
        [] => Ok(false),
        [Item::Node(_), ..] => Ok(true),
        [Item::Atomic(atomic)] => match atomic {
            Atomic::Bool(value) => Ok(*value),
            Atomic::Str(text) | Atomic::Untyped(text) => Ok(!text.is_empty()),
            Atomic::Int(value) => Ok(*value != 0),
            Atomic::Dec(value) | Atomic::Dbl(value) => Ok(*value != 0.0 && !value.is_nan()),
            Atomic::DateTime(_) | Atomic::Duration(_) => Err(XError::new(
                "FORG0006",
                "no effective boolean value for a date, time or duration",
            )),
        },
        _ => Err(XError::new(
            "FORG0006",
            "no effective boolean value for a sequence of several atomic values",
        )),
    }
}

/// A predicate keeps an item when its value is the item's position (for a
/// number) or has a true effective boolean value.
fn predicate_truth(value: &[Item<'_>], position: usize) -> Result<bool> {
    if let [Item::Atomic(atomic)] = value
        && atomic.is_numeric()
    {
        return Ok(atomic.as_f64() == Some(position as f64));
    }
    effective_boolean(value)
}

fn untyped_to_string(value: Atomic) -> Atomic {
    match value {
        Atomic::Untyped(text) => Atomic::Str(text),
        other => other,
    }
}

fn general_compare(op: CompareOp, a: &Atomic, b: &Atomic) -> Result<bool> {
    // An untyped operand takes the other operand's type (double for
    // numbers, string for strings and other untyped values).
    let promote = |value: &Atomic, other: &Atomic| -> Result<Atomic> {
        match value {
            Atomic::Untyped(text) => match other {
                Atomic::Untyped(_) | Atomic::Str(_) => Ok(Atomic::Str(text.clone())),
                other if other.is_numeric() => value.cast(AtomicType::Double),
                Atomic::Bool(_) => value.cast(AtomicType::Boolean),
                Atomic::DateTime(other) => value.cast(AtomicType::DateTime(other.kind)),
                Atomic::Duration(other) => value.cast(AtomicType::Duration(other.kind)),
                _ => Ok(Atomic::Str(text.clone())),
            },
            other => Ok(other.clone()),
        }
    };
    let a2 = promote(a, b)?;
    let b2 = promote(b, a)?;
    value_compare(op, &a2, &b2)
}

fn value_compare(op: CompareOp, a: &Atomic, b: &Atomic) -> Result<bool> {
    if !a.comparable(b) {
        return Err(XError::new(
            "XPTY0004",
            format!(
                "cannot compare `{}` with `{}`",
                a.to_string_value(),
                b.to_string_value()
            ),
        ));
    }
    if !matches!(op, CompareOp::Eq | CompareOp::Ne) && !a.ordered_with(b) {
        return Err(XError::new(
            "XPTY0004",
            format!(
                "`{}` and `{}` can only be compared for equality",
                a.to_string_value(),
                b.to_string_value()
            ),
        ));
    }
    let Some(ordering) = a.compare(b) else {
        // NaN compares false with everything, except for `ne`.
        return Ok(op == CompareOp::Ne);
    };
    Ok(match op {
        CompareOp::Eq => ordering.is_eq(),
        CompareOp::Ne => ordering.is_ne(),
        CompareOp::Lt => ordering.is_lt(),
        CompareOp::Le => ordering.is_le(),
        CompareOp::Gt => ordering.is_gt(),
        CompareOp::Ge => ordering.is_ge(),
    })
}

pub(crate) fn numeric_operand(value: Atomic) -> Result<Atomic> {
    match value {
        Atomic::Untyped(_) => value.cast(AtomicType::Double),
        value if value.is_numeric() => Ok(value),
        other => Err(XError::new(
            "XPTY0004",
            format!("`{}` is not a number", other.to_string_value()),
        )),
    }
}

fn integer_operand(value: Atomic) -> Result<i64> {
    match value {
        Atomic::Int(value) => Ok(value),
        Atomic::Untyped(_) => match value.cast(AtomicType::Integer)? {
            Atomic::Int(value) => Ok(value),
            _ => unreachable!("cast to integer"),
        },
        other => Err(XError::new(
            "XPTY0004",
            format!("`{}` is not an integer", other.to_string_value()),
        )),
    }
}

fn arithmetic(op: ArithmeticOp, a: Atomic, b: Atomic) -> Result<Atomic> {
    let temporal = |value: &Atomic| matches!(value, Atomic::DateTime(_) | Atomic::Duration(_));
    if temporal(&a) || temporal(&b) {
        let a = if temporal(&a) { a } else { numeric_operand(a)? };
        let b = if temporal(&b) { b } else { numeric_operand(b)? };
        return datetime::arithmetic(op, &a, &b);
    }
    let a = numeric_operand(a)?;
    let b = numeric_operand(b)?;
    if let (Atomic::Int(x), Atomic::Int(y)) = (&a, &b) {
        let (x, y) = (*x, *y);
        let overflow = || XError::new("FOAR0002", "integer overflow");
        return match op {
            ArithmeticOp::Add => x.checked_add(y).map(Atomic::Int).ok_or_else(overflow),
            ArithmeticOp::Subtract => x.checked_sub(y).map(Atomic::Int).ok_or_else(overflow),
            ArithmeticOp::Multiply => x.checked_mul(y).map(Atomic::Int).ok_or_else(overflow),
            ArithmeticOp::Divide if y == 0 => Err(XError::new("FOAR0001", "division by zero")),
            ArithmeticOp::Divide => Ok(Atomic::Dec(x as f64 / y as f64)),
            ArithmeticOp::IntegerDivide | ArithmeticOp::Modulo if y == 0 => {
                Err(XError::new("FOAR0001", "division by zero"))
            }
            ArithmeticOp::IntegerDivide => Ok(Atomic::Int(x.wrapping_div(y))),
            ArithmeticOp::Modulo => Ok(Atomic::Int(x.wrapping_rem(y))),
        };
    }
    let double = matches!(a, Atomic::Dbl(_)) || matches!(b, Atomic::Dbl(_));
    let x = a.as_f64().expect("numeric");
    let y = b.as_f64().expect("numeric");
    let wrap = |value: f64| {
        if double {
            Atomic::Dbl(value)
        } else {
            Atomic::Dec(value)
        }
    };
    match op {
        ArithmeticOp::Add => Ok(wrap(x + y)),
        ArithmeticOp::Subtract => Ok(wrap(x - y)),
        ArithmeticOp::Multiply => Ok(wrap(x * y)),
        ArithmeticOp::Divide if !double && y == 0.0 => {
            Err(XError::new("FOAR0001", "division by zero"))
        }
        ArithmeticOp::Divide => Ok(wrap(x / y)),
        ArithmeticOp::Modulo if !double && y == 0.0 => {
            Err(XError::new("FOAR0001", "division by zero"))
        }
        ArithmeticOp::Modulo => Ok(wrap(x % y)),
        ArithmeticOp::IntegerDivide => {
            let quotient = (x / y).trunc();
            if !quotient.is_finite() {
                return Err(XError::new(
                    "FOAR0001",
                    "integer division by zero or overflow",
                ));
            }
            Ok(Atomic::Int(quotient as i64))
        }
    }
}

fn instance_of(value: &[Item<'_>], sequence_type: &SequenceType) -> bool {
    let Some(item_type) = &sequence_type.item else {
        return value.is_empty();
    };
    if value.len() < sequence_type.min || (!sequence_type.many && value.len() > 1) {
        return false;
    }
    value.iter().all(|item| match (item_type, item) {
        (ItemType::Item, _) => true,
        (ItemType::Node(test), Item::Node(node)) => node_test(test, *node, false),
        (ItemType::Atomic(atomic_type), Item::Atomic(atomic)) => atomic_type.admits(atomic),
        _ => false,
    })
}
