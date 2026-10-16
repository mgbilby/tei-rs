//! ISO Schematron rules embedded in RELAX NG schemas, as TEI ODD-generated
//! schemas carry them.
//!
//! The rules are extracted the way TEI's toolchain does (every `sch:ns`,
//! schema-level `sch:let`, `sch:pattern` and `sch:diagnostics` in the
//! schema and the files it includes), in either syntax: compact-syntax
//! annotations are read as the XML elements they stand for, and Schematron
//! 1.5 elements as ISO ones. They are compiled once, and run over each
//! document with the XPath 2.0 evaluator in `crate::xpath`. As in ISO
//! Schematron, each node is checked against every pattern, by the first
//! rule of that pattern whose context matches it.
//!
//! Rules and tests that use XPath this crate doesn't implement are not
//! guessed at: they are listed by [`Schematron::unsupported`] and skipped.
//!
//! Rules may read other XML documents with `doc()` and XSLT's `document()`:
//! files under the schema's directory, the validated document's directory or
//! the schema store set with [`Schema::set_store`](crate::Schema::set_store),
//! which also holds the `http(s)` documents rules name: validation never
//! touches the network. A document that can't be read is an evaluation error
//! of the rule that reads it.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::Severity;
use crate::relaxng::{SchemaSource, SchemaSyntax};
use crate::xpath::{
    Access, CompileError, Document, Evaluator, Expr, Focus, Item, NodeTest, Pattern, Resources,
    StaticContext, XNode, file_uri, literal_documents, parse_expr, parse_pattern, static_base,
};

mod compact;

/// Reads an included schema: given an `href` and the including schema's
/// URI, returns the included schema.
type Resolve = dyn Fn(&str, &str) -> Result<SchemaSource, String>;

/// Variable values, in binding order.
type Bindings<'a> = Vec<(String, Vec<Item<'a>>)>;

/// The ISO Schematron namespace.
pub const SCHEMATRON_NS: &str = "http://purl.oclc.org/dsdl/schematron";
/// The Schematron 1.5 namespace, whose elements are read as ISO ones.
const SCHEMATRON_1_5_NS: &str = "http://www.ascc.net/xml/schematron";
const RELAXNG_NS: &str = "http://relaxng.org/ns/structure/1.0";

/// Whether a message comes from an `assert` or a `report`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TestKind {
    /// `sch:assert`: a message when the test is false.
    Assert,
    /// `sch:report`: a message when the test is true.
    Report,
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Assert => "assert",
            Self::Report => "report",
        })
    }
}

/// What a Schematron message says happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// An `assert` whose test was false.
    FailedAssert,
    /// A `report` whose test was true.
    SuccessfulReport,
    /// The test (or a variable it uses) raised an XPath error, so the rule
    /// could not be checked here. Carries the error message.
    EvaluationError(String),
}

/// One Schematron message about a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// What happened.
    pub outcome: Outcome,
    /// Whether the test is an `assert` or a `report`.
    pub kind: TestKind,
    /// The test's `role` attribute (TEI uses `warning`, `nonfatal`, …).
    pub role: Option<String>,
    /// How serious the message is, derived from `role`.
    pub severity: Severity,
    /// The message text, with `sch:name` and `sch:value-of` filled in and
    /// whitespace normalised.
    pub text: String,
    /// Text of the diagnostics the test refers to, if any.
    pub diagnostics: Vec<String>,
    /// The XPath test.
    pub test: String,
    /// The rule's context pattern.
    pub context: String,
    /// The `id` of the pattern, if it has one.
    pub pattern: Option<String>,
    /// The `id` of the assert or report, if it has one.
    pub id: Option<String>,
    /// Where the context node is, as an XPath of `Q{uri}local[n]` steps
    /// (the form SchXslt reports).
    pub location: String,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.outcome {
            Outcome::FailedAssert => write!(f, "Schematron assert failed: {}", self.text)?,
            Outcome::SuccessfulReport => write!(f, "Schematron report: {}", self.text)?,
            Outcome::EvaluationError(error) => write!(
                f,
                "Schematron {} `{}` (rule `{}`) could not be evaluated: {error}",
                self.kind, self.test, self.context
            )?,
        }
        for diagnostic in &self.diagnostics {
            write!(f, " ({diagnostic})")?;
        }
        if let Some(role) = &self.role {
            write!(f, " [role: {role}]")?;
        }
        Ok(())
    }
}

/// A rule or test that is skipped because it uses something this crate's
/// XPath evaluator doesn't implement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    /// The rule's context pattern.
    pub context: String,
    /// The test, or `None` when the whole rule is skipped (an unsupported
    /// context or variable).
    pub test: Option<(TestKind, String)>,
    /// The number of `assert`/`report` tests skipped.
    pub tests: usize,
    /// Why.
    pub reason: String,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.test {
            Some((kind, test)) => write!(
                f,
                "rule `{}`, {kind} `{test}`: {}",
                self.context, self.reason
            ),
            None => write!(
                f,
                "rule `{}` ({} test{}): {}",
                self.context,
                self.tests,
                if self.tests == 1 { "" } else { "s" },
                self.reason
            ),
        }
    }
}

/// The Schematron rules of a schema, compiled.
pub struct Schematron {
    globals: Vec<(String, Expr)>,
    patterns: Vec<CompiledPattern>,
    rules: Vec<Rule>,
    dispatch: Dispatch,
    test_count: usize,
    unsupported: Vec<Unsupported>,
    /// The schema's directory, whose files rules may read.
    schema_dir: Option<PathBuf>,
    remote_documents: Vec<String>,
}

impl fmt::Debug for Schematron {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schematron")
            .field("patterns", &self.patterns.len())
            .field("rules", &self.rules.len())
            .field("tests", &self.test_count)
            .field("unsupported", &self.unsupported)
            .finish()
    }
}

struct CompiledPattern {
    id: Option<String>,
    lets: Vec<(String, Expr)>,
}

struct Rule {
    pattern: usize,
    context_text: String,
    context: Pattern,
    lets: Vec<(String, Expr)>,
    tests: Vec<Test>,
}

struct Test {
    kind: TestKind,
    text: String,
    expr: Expr,
    role: Option<String>,
    id: Option<String>,
    message: Vec<Part>,
    diagnostics: Vec<Vec<Part>>,
    subject: Option<Expr>,
}

/// A piece of message text.
enum Part {
    Text(String),
    /// `sch:name`, of the context node or of the node `path` selects.
    Name(Option<Expr>),
    /// `sch:value-of select`.
    ValueOf(Expr),
    /// A `sch:name`/`sch:value-of` whose XPath isn't supported: shown as
    /// written.
    Unsupported(String),
}

/// Which rules can match which nodes, by the last step of their context,
/// so most nodes are only checked against a few rules. Rule indexes are in
/// schema order, so a pattern's rules are contiguous and in priority order.
#[derive(Default)]
struct Dispatch {
    elements_by_name: HashMap<String, Vec<usize>>,
    any_element: Vec<usize>,
    attributes: Vec<usize>,
    other_nodes: Vec<usize>,
}

impl Schematron {
    /// Extracts and compiles the Schematron rules embedded in a RELAX NG
    /// schema, in either syntax. `resolve` reads an included or externally
    /// referenced schema: given an `href` and the including schema's URI, it
    /// returns that schema.
    pub(crate) fn from_relaxng(source: &SchemaSource, resolve: &Resolve) -> Result<Self, String> {
        let mut collected = Collected::default();
        collect(source, resolve, &mut collected, 0)?;
        let mut schematron = compile(collected);
        let base_uri = source.base_uri();
        schematron.schema_dir = (!base_uri.is_empty() && !base_uri.contains("://"))
            .then(|| {
                std::path::absolute(base_uri)
                    .ok()?
                    .parent()
                    .map(Path::to_owned)
            })
            .flatten();
        Ok(schematron)
    }

    /// Whether the schema embeds no Schematron rules at all.
    pub fn is_empty(&self) -> bool {
        self.test_count == 0
    }

    /// The number of `assert` and `report` tests in the schema.
    pub fn test_count(&self) -> usize {
        self.test_count
    }

    /// The number of tests that are skipped as unsupported.
    pub fn unsupported_test_count(&self) -> usize {
        self.unsupported.iter().map(|entry| entry.tests).sum()
    }

    /// The rules and tests that are skipped, and why.
    pub fn unsupported(&self) -> &[Unsupported] {
        &self.unsupported
    }

    /// The `http(s)` URIs of the documents the rules read with `doc()`,
    /// `document()` or `doc-available()` of a string literal, sorted.
    /// Validation reads them from the schema store
    /// ([`Schema::set_store`](crate::Schema::set_store)); rules that read
    /// one the store lacks report an evaluation error instead (and
    /// `doc-available()` is false).
    pub fn remote_documents(&self) -> &[String] {
        &self.remote_documents
    }

    /// Checks a parsed document, read from `path` if it was. `access` says
    /// what else the rules may read.
    pub(crate) fn check<'a>(
        &self,
        doc: &'a roxmltree::Document<'a>,
        path: Option<&Path>,
        access: &Access,
    ) -> Vec<(XNode<'a>, Message)> {
        let resources = Resources::new(access, self.schema_dir.as_deref(), path);
        let uri = path.map(file_uri);
        // The messages are all about the validated document's nodes; they
        // are looked up again so they outlive the other documents.
        self.check_with(doc, uri, &resources)
            .into_iter()
            .map(|(node, message)| {
                let (XNode::Node(found) | XNode::Attr(found, _)) = node;
                let same = doc
                    .get_node(found.id())
                    .expect("a node of the validated document");
                let node = match node {
                    XNode::Node(_) => XNode::Node(same),
                    XNode::Attr(_, index) => XNode::Attr(same, index),
                };
                (node, message)
            })
            .collect()
    }

    fn check_with<'a>(
        &self,
        doc: &'a roxmltree::Document<'a>,
        uri: Option<String>,
        resources: &'a Resources<'a>,
    ) -> Vec<(XNode<'a>, Message)> {
        let document = Document::new(doc, uri);
        let mut evaluator = Evaluator::with_resources(&document, resources);
        let mut messages = Vec::new();
        let root = document.root();
        let root_focus = Focus::node(root);

        // Schema-level and pattern-level variables are evaluated once, with
        // the document node as context.
        for (name, expr) in &self.globals {
            match evaluator.eval(expr, Some(&root_focus)) {
                Ok(value) => evaluator.bind(name, value),
                Err(error) => {
                    messages.push((root, variable_error(name, &error.to_string())));
                    return messages;
                }
            }
        }
        let mut pattern_values: Vec<Option<Bindings<'a>>> = Vec::new();
        for pattern in &self.patterns {
            let mut values: Vec<(String, Vec<Item<'a>>)> = Vec::new();
            let mut failed = false;
            for (name, expr) in &pattern.lets {
                let mark = evaluator.variable_count();
                for (bound, value) in &values {
                    evaluator.bind(bound, value.clone());
                }
                let result = evaluator.eval(expr, Some(&root_focus));
                evaluator.unbind_to(mark);
                match result {
                    Ok(value) => values.push((name.clone(), value)),
                    Err(error) => {
                        messages.push((root, variable_error(name, &error.to_string())));
                        failed = true;
                        break;
                    }
                }
            }
            pattern_values.push((!failed).then_some(values));
        }

        let check_attributes = !self.dispatch.attributes.is_empty();
        let mut candidates = Vec::new();
        for node in doc.root().descendants() {
            let xnode = XNode::Node(node);
            candidates.clear();
            if node.is_element() {
                let by_name = self
                    .dispatch
                    .elements_by_name
                    .get(node.tag_name().name())
                    .map_or(&[][..], Vec::as_slice);
                merge_sorted(by_name, &self.dispatch.any_element, &mut candidates);
            } else {
                candidates.extend_from_slice(&self.dispatch.other_nodes);
            }
            self.check_node(
                &mut evaluator,
                xnode,
                &candidates,
                &pattern_values,
                &mut messages,
            );
            if check_attributes && node.is_element() {
                for index in 0..node.attributes().len() as u32 {
                    self.check_node(
                        &mut evaluator,
                        XNode::Attr(node, index),
                        &self.dispatch.attributes,
                        &pattern_values,
                        &mut messages,
                    );
                }
            }
        }
        messages
    }

    /// Runs the first matching rule of each pattern among `candidates` on
    /// `node`.
    fn check_node<'a>(
        &self,
        evaluator: &mut Evaluator<'_, 'a>,
        node: XNode<'a>,
        candidates: &[usize],
        pattern_values: &[Option<Bindings<'a>>],
        messages: &mut Vec<(XNode<'a>, Message)>,
    ) {
        let mut fired_pattern = None;
        for &index in candidates {
            let rule = &self.rules[index];
            if fired_pattern == Some(rule.pattern) {
                continue;
            }
            let Some(values) = &pattern_values[rule.pattern] else {
                continue;
            };
            match evaluator.matches(&rule.context, node) {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) => {
                    messages.push((node, self.rule_error(rule, &error.to_string())));
                    fired_pattern = Some(rule.pattern);
                    continue;
                }
            }
            fired_pattern = Some(rule.pattern);
            let mark = evaluator.variable_count();
            for (name, value) in values {
                evaluator.bind(name, value.clone());
            }
            self.fire(evaluator, rule, node, messages);
            evaluator.unbind_to(mark);
        }
    }

    fn fire<'a>(
        &self,
        evaluator: &mut Evaluator<'_, 'a>,
        rule: &Rule,
        node: XNode<'a>,
        messages: &mut Vec<(XNode<'a>, Message)>,
    ) {
        let focus = Focus::node(node);
        evaluator.current = Some(node);
        for (name, expr) in &rule.lets {
            match evaluator.eval(expr, Some(&focus)) {
                Ok(value) => evaluator.bind(name, value),
                Err(error) => {
                    messages.push((node, self.rule_error(rule, &format!("${name}: {error}"))));
                    return;
                }
            }
        }
        for test in &rule.tests {
            let outcome = match evaluator.boolean(&test.expr, Some(&focus)) {
                Ok(true) if test.kind == TestKind::Report => Outcome::SuccessfulReport,
                Ok(false) if test.kind == TestKind::Assert => Outcome::FailedAssert,
                Ok(_) => continue,
                Err(error) => Outcome::EvaluationError(error.to_string()),
            };
            let errored = matches!(outcome, Outcome::EvaluationError(_));
            let location_node = test
                .subject
                .as_ref()
                .and_then(|subject| evaluator.nodes(subject, Some(&focus)).ok())
                .and_then(|nodes| nodes.first().copied())
                .filter(|subject| evaluator.in_main_document(*subject))
                .unwrap_or(node);
            let (text, diagnostics) = if errored {
                (String::new(), Vec::new())
            } else {
                (
                    render(evaluator, &test.message, &focus),
                    test.diagnostics
                        .iter()
                        .map(|parts| render(evaluator, parts, &focus))
                        .collect(),
                )
            };
            messages.push((
                location_node,
                Message {
                    severity: if errored {
                        Severity::Warning
                    } else {
                        severity(test.role.as_deref())
                    },
                    outcome,
                    kind: test.kind,
                    role: test.role.clone(),
                    text,
                    diagnostics,
                    test: test.text.clone(),
                    context: rule.context_text.clone(),
                    pattern: self.patterns[rule.pattern].id.clone(),
                    id: test.id.clone(),
                    location: location(location_node),
                },
            ));
        }
    }

    fn rule_error(&self, rule: &Rule, error: &str) -> Message {
        Message {
            outcome: Outcome::EvaluationError(error.to_owned()),
            kind: TestKind::Assert,
            role: None,
            severity: Severity::Warning,
            text: String::new(),
            diagnostics: Vec::new(),
            test: String::new(),
            context: rule.context_text.clone(),
            pattern: self.patterns[rule.pattern].id.clone(),
            id: None,
            location: String::new(),
        }
    }
}

fn variable_error(name: &str, error: &str) -> Message {
    Message {
        outcome: Outcome::EvaluationError(format!("${name}: {error}")),
        kind: TestKind::Assert,
        role: None,
        severity: Severity::Warning,
        text: String::new(),
        diagnostics: Vec::new(),
        test: String::new(),
        context: "/".into(),
        pattern: None,
        id: None,
        location: "/".into(),
    }
}

/// How serious a message with this `role` is. Schematron leaves roles
/// free-form; this follows common usage (and oXygen's): `warning`/`warn`
/// and TEI's `nonfatal` are warnings, `info`/`information` is
/// informational, and anything else (`error`, `fatal`, or no role) is an
/// error.
pub fn severity(role: Option<&str>) -> Severity {
    match role.map(str::to_ascii_lowercase).as_deref() {
        Some("warning" | "warn" | "nonfatal") => Severity::Warning,
        Some("info" | "information") => Severity::Info,
        _ => Severity::Error,
    }
}

fn render<'a>(evaluator: &mut Evaluator<'_, 'a>, parts: &[Part], focus: &Focus<'a>) -> String {
    let mut text = String::new();
    for part in parts {
        match part {
            Part::Text(literal) => text.push_str(literal),
            Part::Name(None) => {
                if let Item::Node(node) = &focus.item {
                    text.push_str(&node.qname());
                }
            }
            Part::Name(Some(path)) => match evaluator.nodes(path, Some(focus)) {
                Ok(nodes) => {
                    text.push_str(&nodes.first().map(|node| node.qname()).unwrap_or_default())
                }
                Err(error) => text.push_str(&format!("[{error}]")),
            },
            Part::ValueOf(expr) => match evaluator.eval(expr, Some(focus)) {
                Ok(items) => {
                    let values: Vec<String> = items.iter().map(Item::string).collect();
                    text.push_str(&values.join(" "));
                }
                Err(error) => text.push_str(&format!("[{error}]")),
            },
            Part::Unsupported(source) => text.push_str(&format!("[{source}]")),
        }
    }
    crate::xpath::collapse_whitespace(&text)
}

/// The SchXslt-style location of a node: `/Q{uri}local[n]/…/@Q{uri}local`.
pub(crate) fn location(node: XNode<'_>) -> String {
    let mut segments = Vec::new();
    let mut current = Some(node);
    while let Some(node) = current {
        match node {
            XNode::Attr(..) => {
                let (namespace, local) = node.expanded_name().unwrap_or((None, ""));
                segments.push(format!("@Q{{{}}}{local}", namespace.unwrap_or_default()));
            }
            XNode::Node(inner) => {
                let same = |sibling: &roxmltree::Node<'_, '_>| {
                    if inner.is_element() {
                        sibling.is_element() && sibling.tag_name() == inner.tag_name()
                    } else {
                        sibling.node_type() == inner.node_type()
                            && (!inner.is_pi()
                                || sibling.pi().map(|pi| pi.target)
                                    == inner.pi().map(|pi| pi.target))
                    }
                };
                let position = inner.prev_siblings().filter(same).count();
                match inner.node_type() {
                    roxmltree::NodeType::Root => {}
                    roxmltree::NodeType::Element => {
                        let name = inner.tag_name();
                        segments.push(format!(
                            "Q{{{}}}{}[{position}]",
                            name.namespace().unwrap_or_default(),
                            name.name()
                        ));
                    }
                    roxmltree::NodeType::Text => segments.push(format!("text()[{position}]")),
                    roxmltree::NodeType::Comment => segments.push(format!("comment()[{position}]")),
                    roxmltree::NodeType::PI => segments.push(format!(
                        "processing-instruction(\"{}\")[{position}]",
                        inner.pi().map(|pi| pi.target).unwrap_or_default()
                    )),
                }
            }
        }
        current = node.parent();
    }
    segments.reverse();
    format!("/{}", segments.join("/"))
}

fn merge_sorted(a: &[usize], b: &[usize], out: &mut Vec<usize>) {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(&a[i..]);
    out.extend_from_slice(&b[j..]);
}

// --- Extraction ---------------------------------------------------------

/// A Schematron element copied out of the schema document.
#[derive(Clone, Debug)]
struct SchElement {
    /// Local name; `None` for elements in other namespaces.
    name: Option<String>,
    attributes: HashMap<String, String>,
    children: Vec<SchContent>,
    /// Namespace declarations in scope, as a fallback for prefixes no
    /// `sch:ns` declares.
    namespaces: Vec<(String, String)>,
    /// For a pattern or schema-level variable: the static base URI of the
    /// expressions in it (the URI of the schema file it is in).
    base: Option<String>,
}

#[derive(Clone, Debug)]
enum SchContent {
    Element(SchElement),
    Text(String),
}

impl SchElement {
    fn from_node(node: roxmltree::Node<'_, '_>) -> Self {
        let in_schematron = is_schematron(node.tag_name().namespace());
        Self {
            name: in_schematron.then(|| node.tag_name().name().to_owned()),
            attributes: node
                .attributes()
                .filter(|attribute| attribute.namespace().is_none())
                .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
                .collect(),
            children: node
                .children()
                .filter_map(|child| {
                    if child.is_element() {
                        Some(SchContent::Element(Self::from_node(child)))
                    } else if child.is_text() {
                        Some(SchContent::Text(
                            child.text().unwrap_or_default().to_owned(),
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
            namespaces: node
                .namespaces()
                .filter_map(|ns| {
                    ns.name()
                        .map(|prefix| (prefix.to_owned(), ns.uri().to_owned()))
                })
                .collect(),
            base: None,
        }
    }

    /// The element, with its expressions' static base URI.
    fn with_base(mut self, uri: &str) -> Self {
        self.base = static_base(uri);
        self
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    fn elements<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'s SchElement> + 's {
        self.children.iter().filter_map(move |child| match child {
            SchContent::Element(element) if element.name.as_deref() == Some(name) => Some(element),
            _ => None,
        })
    }
}

#[derive(Default)]
struct Collected {
    namespaces: Vec<(String, String)>,
    lets: Vec<SchElement>,
    patterns: Vec<SchElement>,
    diagnostics: Vec<SchElement>,
}

fn collect(
    source: &SchemaSource,
    resolve: &Resolve,
    collected: &mut Collected,
    depth: usize,
) -> Result<(), String> {
    let uri = source.base_uri();
    if depth > 50 {
        return Err(format!("{uri}: includes nest too deeply"));
    }
    let annotations;
    let text = match source.syntax() {
        SchemaSyntax::Xml => source.text(),
        SchemaSyntax::Compact => {
            annotations =
                compact::annotations(source.text()).map_err(|error| format!("{uri}: {error}"))?;
            &annotations
        }
    };
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    if crate::limits::xml_nesting_exceeds(text, crate::limits::MAX_SCHEMA_DEPTH) {
        return Err(format!("{uri}: elements nested too deeply"));
    }
    let doc = roxmltree::Document::parse_with_options(text, options)
        .map_err(|error| format!("{uri}: {error}"))?;
    collect_node(doc.root(), uri, resolve, collected, depth)
}

fn is_schematron(namespace: Option<&str>) -> bool {
    matches!(namespace, Some(SCHEMATRON_NS | SCHEMATRON_1_5_NS))
}

fn collect_node(
    node: roxmltree::Node<'_, '_>,
    uri: &str,
    resolve: &Resolve,
    collected: &mut Collected,
    depth: usize,
) -> Result<(), String> {
    for child in node.children().filter(|child| child.is_element()) {
        let name = child.tag_name();
        match (name.namespace(), name.name()) {
            (namespace, "ns") if is_schematron(namespace) => {
                if let (Some(prefix), Some(namespace)) =
                    (child.attribute("prefix"), child.attribute("uri"))
                {
                    collected
                        .namespaces
                        .push((prefix.to_owned(), namespace.to_owned()));
                }
            }
            (namespace, "pattern") if is_schematron(namespace) => collected
                .patterns
                .push(SchElement::from_node(child).with_base(uri)),
            (namespace, "let") if is_schematron(namespace) => collected
                .lets
                .push(SchElement::from_node(child).with_base(uri)),
            (namespace, "diagnostics") if is_schematron(namespace) => {
                collected.diagnostics.push(SchElement::from_node(child))
            }
            (Some(RELAXNG_NS), "include" | "externalRef") => {
                if let Some(href) = child.attribute("href") {
                    let included = resolve(href, uri)?;
                    collect(&included, resolve, collected, depth + 1)?;
                }
                collect_node(child, uri, resolve, collected, depth)?;
            }
            _ => collect_node(child, uri, resolve, collected, depth)?,
        }
    }
    Ok(())
}

// --- Compilation --------------------------------------------------------

fn compile(collected: Collected) -> Schematron {
    let mut namespaces = HashMap::new();
    for (prefix, uri) in collected.namespaces {
        namespaces.entry(prefix).or_insert(uri);
    }
    let mut context = StaticContext {
        namespaces,
        ..StaticContext::default()
    };
    let mut unsupported = Vec::new();
    let mut globals = Vec::new();
    for element in &collected.lets {
        let (Some(name), Some(value)) = (element.attribute("name"), element.attribute("value"))
        else {
            continue;
        };
        context.base_uri = element.base.clone();
        match parse_expr(value, &context) {
            Ok(expr) => {
                globals.push((name.to_owned(), expr));
                context.variables.push(name.to_owned());
            }
            Err(error) => unsupported.push(Unsupported {
                context: "/".into(),
                test: None,
                tests: 0,
                reason: format!("variable ${name}: {error}"),
            }),
        }
    }
    let diagnostics: HashMap<&str, &SchElement> = collected
        .diagnostics
        .iter()
        .flat_map(|element| element.elements("diagnostic"))
        .filter_map(|diagnostic| Some((diagnostic.attribute("id")?, diagnostic)))
        .collect();
    let abstract_rules: HashMap<&str, &SchElement> = collected
        .patterns
        .iter()
        .flat_map(|pattern| pattern.elements("rule"))
        .filter(|rule| rule.attribute("abstract") == Some("true"))
        .filter_map(|rule| Some((rule.attribute("id")?, rule)))
        .collect();

    let mut compiler = Compiler {
        diagnostics,
        abstract_rules,
        patterns: Vec::new(),
        rules: Vec::new(),
        unsupported,
        test_count: 0,
    };
    for pattern in &collected.patterns {
        compiler.pattern(pattern, &context);
    }
    let Compiler {
        patterns,
        rules,
        unsupported,
        test_count,
        ..
    } = compiler;
    let dispatch = dispatch(&rules);
    let remote_documents = remote_documents(&globals, &patterns, &rules);
    Schematron {
        globals,
        patterns,
        rules,
        dispatch,
        test_count,
        unsupported,
        schema_dir: None,
        remote_documents,
    }
}

/// The `http(s)` URIs the compiled expressions read with `doc()`,
/// `document()` or `doc-available()` of a string literal.
fn remote_documents(
    globals: &[(String, Expr)],
    patterns: &[CompiledPattern],
    rules: &[Rule],
) -> Vec<String> {
    let mut found = Vec::new();
    let lets = globals
        .iter()
        .chain(patterns.iter().flat_map(|pattern| &pattern.lets))
        .chain(rules.iter().flat_map(|rule| &rule.lets));
    for (_, expr) in lets {
        literal_documents(expr, &mut found);
    }
    for test in rules.iter().flat_map(|rule| &rule.tests) {
        literal_documents(&test.expr, &mut found);
        if let Some(subject) = &test.subject {
            literal_documents(subject, &mut found);
        }
        for part in test.message.iter().chain(test.diagnostics.iter().flatten()) {
            if let Part::Name(Some(expr)) | Part::ValueOf(expr) = part {
                literal_documents(expr, &mut found);
            }
        }
    }
    found.retain(|uri| uri.starts_with("http://") || uri.starts_with("https://"));
    found.sort();
    found.dedup();
    found
}

struct Compiler<'s> {
    diagnostics: HashMap<&'s str, &'s SchElement>,
    abstract_rules: HashMap<&'s str, &'s SchElement>,
    patterns: Vec<CompiledPattern>,
    rules: Vec<Rule>,
    unsupported: Vec<Unsupported>,
    test_count: usize,
}

impl<'s> Compiler<'s> {
    fn pattern(&mut self, pattern: &'s SchElement, schema_context: &StaticContext) {
        if pattern.attribute("abstract") == Some("true") {
            return;
        }
        let rules: Vec<&SchElement> = pattern
            .elements("rule")
            .filter(|rule| rule.attribute("abstract") != Some("true"))
            .collect();
        let test_total = |rules: &[&SchElement], compiler: &Self| -> usize {
            rules
                .iter()
                .map(|rule| compiler.rule_children(rule).1.len())
                .sum()
        };
        if let Some(base) = pattern.attribute("is-a") {
            let tests = test_total(&rules, self);
            self.test_count += tests;
            self.unsupported.push(Unsupported {
                context: format!("pattern is-a=\"{base}\""),
                test: None,
                tests,
                reason: "abstract pattern instances are not supported".into(),
            });
            return;
        }
        let index = self.patterns.len();
        let mut context = schema_context.clone();
        context.base_uri = pattern.base.clone();
        let mut lets = Vec::new();
        for element in pattern.elements("let") {
            let (Some(name), Some(value)) = (element.attribute("name"), element.attribute("value"))
            else {
                continue;
            };
            match parse_expr(value, &context) {
                Ok(expr) => {
                    lets.push((name.to_owned(), expr));
                    context.variables.push(name.to_owned());
                }
                Err(error) => {
                    let tests = test_total(&rules, self);
                    self.test_count += tests;
                    self.unsupported.push(Unsupported {
                        context: format!(
                            "pattern {}",
                            pattern.attribute("id").unwrap_or("(no id)")
                        ),
                        test: None,
                        tests,
                        reason: format!("variable ${name}: {error}"),
                    });
                    return;
                }
            }
        }
        self.patterns.push(CompiledPattern {
            id: pattern.attribute("id").map(str::to_owned),
            lets,
        });
        // Once a rule's context can't be compiled, the rules after it in
        // the pattern can't be run either: which nodes they get depends on
        // which ones the skipped rule would have claimed.
        let mut blocked: Option<String> = None;
        for rule in rules {
            let context_text = normalize(rule.attribute("context").unwrap_or_default());
            let (lets, tests) = self.rule_children(rule);
            self.test_count += tests.len();
            if let Some(reason) = &blocked {
                self.unsupported.push(Unsupported {
                    context: context_text,
                    test: None,
                    tests: tests.len(),
                    reason: reason.clone(),
                });
                continue;
            }
            let ns_context = with_fallback_namespaces(&context, rule);
            let compiled_context = match parse_pattern(&context_text, &ns_context) {
                Ok(pattern) => pattern,
                Err(error) => {
                    self.unsupported.push(Unsupported {
                        context: context_text.clone(),
                        test: None,
                        tests: tests.len(),
                        reason: format!("context: {error}"),
                    });
                    blocked = Some(format!(
                        "follows rule `{context_text}`, which is not supported"
                    ));
                    continue;
                }
            };
            let mut rule_context = ns_context;
            let mut compiled_lets = Vec::new();
            let mut let_error = None;
            for element in lets {
                let (Some(name), Some(value)) =
                    (element.attribute("name"), element.attribute("value"))
                else {
                    let_error = Some(CompileError("a variable without a name and value".into()));
                    break;
                };
                match parse_expr(value, &rule_context) {
                    Ok(expr) => {
                        compiled_lets.push((name.to_owned(), expr));
                        rule_context.variables.push(name.to_owned());
                    }
                    Err(error) => {
                        let_error = Some(CompileError(format!("variable ${name}: {error}")));
                        break;
                    }
                }
            }
            if let Some(error) = let_error {
                self.unsupported.push(Unsupported {
                    context: context_text.clone(),
                    test: None,
                    tests: tests.len(),
                    reason: error.0,
                });
                blocked = Some(format!(
                    "follows rule `{context_text}`, which is not supported"
                ));
                continue;
            }
            let mut compiled_tests = Vec::new();
            for element in tests {
                let kind = if element.name.as_deref() == Some("report") {
                    TestKind::Report
                } else {
                    TestKind::Assert
                };
                let text = element.attribute("test").unwrap_or_default().to_owned();
                let test_context = with_fallback_namespaces(&rule_context, element);
                let expr = match parse_expr(&text, &test_context) {
                    Ok(expr) => expr,
                    Err(error) => {
                        self.unsupported.push(Unsupported {
                            context: context_text.clone(),
                            test: Some((kind, normalize(&text))),
                            tests: 1,
                            reason: error.0,
                        });
                        continue;
                    }
                };
                let subject = element
                    .attribute("subject")
                    .or_else(|| rule.attribute("subject"))
                    .and_then(|subject| parse_expr(subject, &test_context).ok());
                let diagnostics = element
                    .attribute("diagnostics")
                    .unwrap_or_default()
                    .split_whitespace()
                    .filter_map(|id| self.diagnostics.get(id))
                    .map(|diagnostic| message_parts(diagnostic, &test_context))
                    .collect();
                compiled_tests.push(Test {
                    kind,
                    text: normalize(&text),
                    expr,
                    role: element.attribute("role").map(str::to_owned),
                    id: element.attribute("id").map(str::to_owned),
                    message: message_parts(element, &test_context),
                    diagnostics,
                    subject,
                });
            }
            self.rules.push(Rule {
                pattern: index,
                context_text,
                context: compiled_context,
                lets: compiled_lets,
                tests: compiled_tests,
            });
        }
    }

    /// A rule's variables and tests, with `sch:extends` expanded.
    fn rule_children(&self, rule: &'s SchElement) -> (Vec<&'s SchElement>, Vec<&'s SchElement>) {
        let mut lets = Vec::new();
        let mut tests = Vec::new();
        self.rule_children_into(rule, &mut lets, &mut tests, 0);
        (lets, tests)
    }

    fn rule_children_into(
        &self,
        rule: &'s SchElement,
        lets: &mut Vec<&'s SchElement>,
        tests: &mut Vec<&'s SchElement>,
        depth: usize,
    ) {
        for child in &rule.children {
            let SchContent::Element(element) = child else {
                continue;
            };
            match element.name.as_deref() {
                Some("let") => lets.push(element),
                Some("assert" | "report") => tests.push(element),
                Some("extends") if depth < 20 => {
                    if let Some(base) = element
                        .attribute("rule")
                        .and_then(|id| self.abstract_rules.get(id))
                    {
                        self.rule_children_into(base, lets, tests, depth + 1);
                    }
                }
                _ => {}
            }
        }
    }
}

/// `context` plus the namespace declarations in scope on `element` for
/// prefixes no `sch:ns` declares.
fn with_fallback_namespaces(context: &StaticContext, element: &SchElement) -> StaticContext {
    let mut context = context.clone();
    for (prefix, uri) in &element.namespaces {
        context
            .namespaces
            .entry(prefix.clone())
            .or_insert_with(|| uri.clone());
    }
    context
}

fn message_parts(element: &SchElement, context: &StaticContext) -> Vec<Part> {
    let mut parts = Vec::new();
    message_parts_into(element, context, &mut parts);
    parts
}

fn message_parts_into(element: &SchElement, context: &StaticContext, parts: &mut Vec<Part>) {
    for child in &element.children {
        match child {
            SchContent::Text(text) => parts.push(Part::Text(text.clone())),
            SchContent::Element(inner) => match inner.name.as_deref() {
                Some("name") => parts.push(match inner.attribute("path") {
                    None => Part::Name(None),
                    Some(path) => match parse_expr(path, context) {
                        Ok(expr) => Part::Name(Some(expr)),
                        Err(_) => Part::Unsupported(path.to_owned()),
                    },
                }),
                Some("value-of") => {
                    let select = inner.attribute("select").unwrap_or_default();
                    parts.push(match parse_expr(select, context) {
                        Ok(expr) => Part::ValueOf(expr),
                        Err(_) => Part::Unsupported(select.to_owned()),
                    });
                }
                // `emph`, `dir`, `span`, and foreign markup: their text.
                _ => message_parts_into(inner, context, parts),
            },
        }
    }
}

fn normalize(text: &str) -> String {
    crate::xpath::collapse_whitespace(text)
}

fn dispatch(rules: &[Rule]) -> Dispatch {
    let mut dispatch = Dispatch::default();
    for (index, rule) in rules.iter().enumerate() {
        let mut by_name = Vec::new();
        let mut any_element = false;
        let mut attributes = false;
        let mut other = false;
        for alternative in &rule.context.alternatives {
            let Some(last) = alternative.steps.last() else {
                other = true; // `/`
                continue;
            };
            if last.step.axis == crate::xpath::Axis::Attribute {
                attributes = true;
                continue;
            }
            match &last.step.test {
                NodeTest::Name(name) | NodeTest::Element(Some(name)) => match &name.local {
                    Some(local) => by_name.push(local.clone()),
                    None => any_element = true,
                },
                NodeTest::Element(None) => any_element = true,
                NodeTest::AnyNode => {
                    any_element = true;
                    other = true;
                }
                NodeTest::Text
                | NodeTest::Comment
                | NodeTest::ProcessingInstruction(_)
                | NodeTest::Document => other = true,
                NodeTest::Attribute(_) => {}
            }
        }
        by_name.sort();
        by_name.dedup();
        if any_element {
            dispatch.any_element.push(index);
        } else {
            for name in by_name {
                dispatch
                    .elements_by_name
                    .entry(name)
                    .or_default()
                    .push(index);
            }
        }
        if attributes {
            dispatch.attributes.push(index);
        }
        if other {
            dispatch.other_nodes.push(index);
        }
    }
    dispatch
}

#[cfg(test)]
mod tests;
