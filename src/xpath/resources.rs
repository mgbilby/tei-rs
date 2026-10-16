//! The documents Schematron rules read with `doc()`, XSLT's `document()` and
//! `doc-available()`: resolving their URIs, reading them, and keeping each
//! one parsed once per validated document.
//!
//! Validation never touches the network: `http(s)` URIs are read from the
//! schema store, where `tei-rs schemas add URL` puts them. Files are read
//! only under the schema's directory, the validated document's directory
//! and the schema store (after resolving symbolic links and `..`), and only
//! regular files within [`MAX_RESOURCE_BYTES`]. Anything else is
//! unavailable: `doc-available()` is false for it, and `doc()` and
//! `document()` raise `FODC0002`.
//!
//! Documents read this way have their own trees. Nodes from different
//! documents are ordered by document, the validated one first and the
//! others in the order they were read, which is stable for a validation.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::XError;
use super::ast::{Expr, Step};
use super::eval::{Document, Evaluator, sorted_nodes};
use super::functions::Function;
use super::value::{Atomic, Item, Seq, XML_NS, XNode};
use crate::limits::{MAX_RESOURCE_BYTES, MAX_RESOURCES, MAX_RESOURCES_BYTES};
use crate::store::SchemaStore;

type Result<T> = std::result::Result<T, XError>;

/// The static base URI `doc()`, `document()` and `doc-available()` resolve
/// relative URIs against: the URI of the schema file the rule is in.
pub(crate) type StaticBase = Option<Arc<str>>;

/// What rules may read besides the schema's and the document's
/// directories: the schema store, which also maps `http(s)` URIs to files.
#[derive(Clone, Debug, Default)]
pub(crate) struct Access {
    pub store: Option<SchemaStore>,
}

/// The documents read while one document is validated.
pub(crate) struct Resources<'a> {
    store: Option<&'a SchemaStore>,
    /// Canonical directories whose files may be read.
    roots: Vec<PathBuf>,
    /// The validated document's canonical path, so reading it returns it.
    main: Option<PathBuf>,
    /// The documents read so far, in a list that only grows, so each stays
    /// where it is for as long as its nodes are in use.
    first: OnceCell<Box<Slot<'a>>>,
    last: Cell<Option<&'a Slot<'a>>>,
    /// Each file read (by canonical path), or why it couldn't be.
    by_path: RefCell<HashMap<PathBuf, Result<&'a Document<'a>>>>,
    count: Cell<usize>,
    bytes: Cell<u64>,
}

struct Slot<'a> {
    text: String,
    tree: OnceCell<roxmltree::Document<'a>>,
    document: OnceCell<Document<'a>>,
    next: OnceCell<Box<Slot<'a>>>,
}

impl<'a> Resources<'a> {
    /// `schema_dir` is the schema's directory; `document` the validated
    /// document's path.
    pub(crate) fn new(
        access: &'a Access,
        schema_dir: Option<&Path>,
        document: Option<&Path>,
    ) -> Self {
        let mut roots: Vec<PathBuf> = schema_dir
            .into_iter()
            .chain(document.and_then(Path::parent))
            .chain(access.store.as_ref().map(SchemaStore::dir))
            .map(|dir| {
                if dir.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    dir
                }
            })
            .filter_map(|dir| dir.canonicalize().ok())
            .collect();
        roots.dedup();
        Self {
            store: access.store.as_ref(),
            roots,
            main: document.and_then(|path| path.canonicalize().ok()),
            first: OnceCell::new(),
            last: Cell::new(None),
            by_path: RefCell::new(HashMap::new()),
            count: Cell::new(0),
            bytes: Cell::new(0),
        }
    }

    /// The document read for `tree`, if it is one of them.
    fn find(&self, tree: &roxmltree::Document<'a>) -> Option<&Document<'a>> {
        let mut slot = self.first.get();
        while let Some(current) = slot {
            if let Some(document) = current.document.get()
                && std::ptr::eq(document.doc, tree)
            {
                return Some(document);
            }
            slot = current.next.get();
        }
        None
    }

    /// The canonical path of the file an absolute URI names (a `file:`
    /// URI's path, or the stored copy of an `http(s)` URI), if rules may
    /// read it.
    fn locate(&self, uri: &str) -> Result<PathBuf> {
        let path = self
            .local_path(uri)?
            .canonicalize()
            .map_err(|error| cannot_read(uri, &error.to_string()))?;
        if self.main.as_ref() == Some(&path) || self.roots.iter().any(|root| path.starts_with(root))
        {
            return Ok(path);
        }
        let roots: Vec<String> = self
            .roots
            .iter()
            .map(|root| root.display().to_string())
            .collect();
        Err(cannot_read(
            uri,
            &format!(
                "Schematron rules only read files under the schema's directory, the \
                 document's directory and the schema store ({})",
                if roots.is_empty() {
                    "none here".to_owned()
                } else {
                    roots.join(", ")
                }
            ),
        ))
    }

    /// The local file an absolute URI names: a `file:` URI's path, or the
    /// stored copy of an `http(s)` URI.
    fn local_path(&self, uri: &str) -> Result<PathBuf> {
        if let Some(path) = file_path(uri) {
            return Ok(path);
        }
        if !matches!(scheme(uri), Some("http" | "https")) {
            return Err(cannot_read(
                uri,
                "only local files and the schema store are read",
            ));
        }
        let Some(store) = self.store else {
            return Err(cannot_read(
                uri,
                "validation never touches the network, and no schema store is set to read it from",
            ));
        };
        store
            .resolve(uri)
            .ok_or_else(|| XError::new("FODC0002", store.missing(uri)))
    }

    /// Reads and parses the document at an absolute URI (without a
    /// fragment), once; `None` when it is the validated document itself.
    fn load(&'a self, uri: &str) -> Result<Option<&'a Document<'a>>> {
        let path = self.locate(uri)?;
        if self.main.as_ref() == Some(&path) {
            return Ok(None);
        }
        if let Some(result) = self.by_path.borrow().get(&path) {
            return result.clone().map(Some);
        }
        let result = self.read(uri, &path);
        self.by_path.borrow_mut().insert(path, result.clone());
        result.map(Some)
    }

    fn read(&'a self, uri: &str, path: &Path) -> Result<&'a Document<'a>> {
        let metadata =
            std::fs::metadata(path).map_err(|error| cannot_read(uri, &error.to_string()))?;
        if !metadata.is_file() {
            return Err(cannot_read(uri, "not a regular file"));
        }
        if metadata.len() > MAX_RESOURCE_BYTES {
            return Err(cannot_read(
                uri,
                &format!("larger than {} MB", MAX_RESOURCE_BYTES / 1024 / 1024),
            ));
        }
        if self.count.get() >= MAX_RESOURCES
            || self.bytes.get() + metadata.len() > MAX_RESOURCES_BYTES
        {
            return Err(cannot_read(
                uri,
                &format!(
                    "the rules may read at most {MAX_RESOURCES} documents and {} MB per validated document",
                    MAX_RESOURCES_BYTES / 1024 / 1024
                ),
            ));
        }
        let text =
            std::fs::read_to_string(path).map_err(|error| cannot_read(uri, &error.to_string()))?;
        self.count.set(self.count.get() + 1);
        self.bytes.set(self.bytes.get() + text.len() as u64);
        if crate::limits::xml_nesting_exceeds(&text, crate::limits::MAX_DOCUMENT_DEPTH) {
            return Err(cannot_read(uri, "elements nested too deeply"));
        }
        let slot = self.append(Slot {
            text,
            tree: OnceCell::new(),
            document: OnceCell::new(),
            next: OnceCell::new(),
        });
        let tree = roxmltree::Document::parse_with_options(&slot.text, parsing_options())
            .map_err(|error| cannot_read(uri, &error.to_string()))?;
        let tree = slot.tree.get_or_init(|| tree);
        let mut document = Document::new(tree, Some(uri.to_owned()));
        document.rank = self.count.get() as u32;
        Ok(slot.document.get_or_init(|| document))
    }

    fn append(&'a self, slot: Slot<'a>) -> &'a Slot<'a> {
        let cell = match self.last.get() {
            Some(last) => &last.next,
            None => &self.first,
        };
        let slot: &'a Slot<'a> = cell.get_or_init(|| Box::new(slot));
        self.last.set(Some(slot));
        slot
    }
}

impl<'a> Evaluator<'_, 'a> {
    /// The document `node` belongs to.
    pub(crate) fn document_of(&self, node: XNode<'a>) -> &Document<'a> {
        let tree = tree(node);
        if std::ptr::eq(tree, self.doc.doc) {
            return self.doc;
        }
        self.resources
            .and_then(|resources| resources.find(tree))
            .unwrap_or(self.doc)
    }

    /// Whether `node` is in the validated document.
    pub(crate) fn in_main_document(&self, node: XNode<'a>) -> bool {
        std::ptr::eq(tree(node), self.doc.doc)
    }

    /// Position in document order, across documents.
    pub(crate) fn order(&self, node: XNode<'a>) -> (u32, u32, u32) {
        let (id, attribute) = node.order();
        (self.document_of(node).rank, id, attribute)
    }

    /// `nodes` in document order without duplicates.
    pub(crate) fn in_document_order(&self, mut nodes: Vec<XNode<'a>>) -> Seq<'a> {
        if nodes
            .windows(2)
            .all(|pair| std::ptr::eq(tree(pair[0]), tree(pair[1])))
        {
            return sorted_nodes(nodes);
        }
        nodes.sort_by_cached_key(|node| self.order(*node));
        nodes.dedup();
        nodes.into_iter().map(Item::Node).collect()
    }

    /// `fn:doc()`: the document node of the document at `uri`.
    pub(crate) fn doc(&self, uri: &str, base: Option<&str>) -> Result<XNode<'a>> {
        if uri.contains('#') {
            return Err(XError::new(
                "FODC0005",
                format!("doc(\"{uri}\"): a URI with a fragment identifier"),
            ));
        }
        let absolute = resolve_uri(uri, base).ok_or_else(|| no_base(uri))?;
        self.load(&absolute)
    }

    /// XSLT's `document()`: the documents (or, for a URI with a fragment
    /// identifier, the elements with that `xml:id`) that `items` name. A node
    /// is resolved against its own base URI, a string against the static
    /// base, unless `base_node` gives the base for all of them.
    pub(crate) fn xslt_document(
        &self,
        items: Seq<'a>,
        base_node: Option<XNode<'a>>,
        static_base: Option<&str>,
    ) -> Result<Seq<'a>> {
        let fixed_base = base_node.map(|node| self.base_uri(node));
        let mut found = Vec::new();
        for item in items {
            let (reference, base) = match item {
                Item::Node(node) => (
                    node.string_value(),
                    fixed_base.clone().unwrap_or_else(|| self.base_uri(node)),
                ),
                Item::Atomic(atomic) => (
                    atomic.to_string_value(),
                    fixed_base
                        .clone()
                        .unwrap_or_else(|| static_base.map(str::to_owned)),
                ),
            };
            let absolute =
                resolve_uri(&reference, base.as_deref()).ok_or_else(|| no_base(&reference))?;
            let (uri, fragment) = absolute.split_once('#').unwrap_or((&absolute, ""));
            let root = self.load(uri)?;
            if fragment.is_empty() {
                found.push(root);
            } else if let Some(element) = self.document_of(root).element_by_id(fragment) {
                found.push(XNode::Node(element));
            }
        }
        Ok(self.in_document_order(found))
    }

    /// `fn:doc-available()`: whether `uri` names a regular file within
    /// [`MAX_RESOURCE_BYTES`] that is well-formed XML and that rules may
    /// read. An `http(s)` URI is available only if the schema store holds
    /// it.
    pub(crate) fn doc_available(&self, uri: &str, base: Option<&str>) -> Result<bool> {
        let absolute = resolve_uri(uri, base).ok_or_else(|| no_base(uri))?;
        let Some(path) = self
            .resources
            .and_then(|resources| resources.locate(&absolute).ok())
        else {
            return Ok(false);
        };
        let Ok(metadata) = std::fs::metadata(&path) else {
            return Ok(false);
        };
        if !metadata.is_file() || metadata.len() > MAX_RESOURCE_BYTES {
            return Ok(false);
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Ok(false);
        };
        if crate::limits::xml_nesting_exceeds(&text, crate::limits::MAX_DOCUMENT_DEPTH) {
            return Ok(false);
        }
        Ok(roxmltree::Document::parse_with_options(&text, parsing_options()).is_ok())
    }

    fn load(&self, uri: &str) -> Result<XNode<'a>> {
        let resources = self
            .resources
            .ok_or_else(|| cannot_read(uri, "no documents can be read here"))?;
        Ok(match resources.load(uri)? {
            Some(document) => document.root(),
            None => self.doc.root(),
        })
    }

    /// A node's base URI: its document's URI, adjusted by any `xml:base`
    /// on it or its ancestors.
    fn base_uri(&self, node: XNode<'a>) -> Option<String> {
        let mut base = self.document_of(node).uri.clone();
        let Some(element) = node.element() else {
            return base;
        };
        let bases: Vec<&str> = element
            .ancestors()
            .filter_map(|ancestor| ancestor.attribute((XML_NS, "base")))
            .collect();
        for xml_base in bases.into_iter().rev() {
            base = resolve_uri(xml_base, base.as_deref()).or(base);
        }
        base
    }
}

fn tree<'a>(node: XNode<'a>) -> &'a roxmltree::Document<'a> {
    match node {
        XNode::Node(node) | XNode::Attr(node, _) => node.document(),
    }
}

fn parsing_options<'input>() -> roxmltree::ParsingOptions<'input> {
    roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    }
}

fn cannot_read(uri: &str, reason: &str) -> XError {
    XError::new("FODC0002", format!("cannot read `{uri}`: {reason}"))
}

fn no_base(uri: &str) -> XError {
    XError::new(
        "FODC0002",
        format!("`{uri}` is relative, and there is no base URI to resolve it against"),
    )
}

/// The `file:` URI of a local path, made absolute.
pub(crate) fn file_uri(path: &Path) -> String {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    format!("file:{}", path.to_string_lossy().replace('\\', "/"))
}

/// The static base URI for rules from the schema file at `uri` (a path, or
/// a URI with a scheme); `None` when there is no file.
pub(crate) fn static_base(uri: &str) -> Option<String> {
    if uri.is_empty() {
        None
    } else if uri.contains("://") || uri.starts_with("file:") {
        Some(uri.to_owned())
    } else {
        Some(file_uri(Path::new(uri)))
    }
}

/// A URI's scheme, if it has one. A single letter is a drive (`C:/…`).
fn scheme(uri: &str) -> Option<&str> {
    let scheme = &uri[..uri.find(':')?];
    (scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(scheme)
}

/// Resolves a URI reference against a base URI, as RFC 3986 does for
/// hierarchical URIs. With no base, only an absolute path resolves (to a
/// `file:` URI).
pub(crate) fn resolve_uri(reference: &str, base: Option<&str>) -> Option<String> {
    if scheme(reference).is_some() {
        return Some(reference.to_owned());
    }
    let Some(base) = base.and_then(|base| Some((base, scheme(base)?))) else {
        return reference
            .starts_with('/')
            .then(|| format!("file:{}", remove_dot_segments(reference)));
    };
    let (base, scheme) = base;
    let base = base.split('#').next().unwrap_or(base);
    if reference.is_empty() {
        return Some(base.to_owned());
    }
    if reference.starts_with('#') {
        return Some(format!("{base}{reference}"));
    }
    if reference.starts_with("//") {
        return Some(format!("{scheme}:{reference}"));
    }
    // The scheme and authority, then the path.
    let rest = &base[scheme.len() + 1..];
    let authority = match rest.strip_prefix("//") {
        Some(after) => after.find('/').map_or(rest.len(), |slash| slash + 2),
        None => 0,
    };
    let (prefix, base_path) = base.split_at(scheme.len() + 1 + authority);
    let path = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let base_path = base_path.split('?').next().unwrap_or_default();
        let directory = &base_path[..base_path.rfind('/').map_or(0, |slash| slash + 1)];
        if directory.is_empty() && authority > 0 {
            format!("/{reference}")
        } else {
            format!("{directory}{reference}")
        }
    };
    Some(format!("{prefix}{}", remove_dot_segments(&path)))
}

/// Removes `.` and `..` segments from a URI path (before any query or
/// fragment); `..` never climbs above the root.
fn remove_dot_segments(path: &str) -> String {
    let end = path.find(['?', '#']).unwrap_or(path.len());
    let (path, rest) = path.split_at(end);
    let segments: Vec<&str> = path.split('/').collect();
    let mut out: Vec<&str> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let last = index + 1 == segments.len();
        match *segment {
            "." => {}
            ".." => {
                if out.len() > 1 || out.first().is_some_and(|first| !first.is_empty()) {
                    out.pop();
                }
            }
            segment => {
                out.push(segment);
                continue;
            }
        }
        if last {
            out.push("");
        }
    }
    format!("{}{rest}", out.join("/"))
}

/// The path of a `file:` URI on this machine; `None` for other URIs.
fn file_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file:")?;
    let rest = match rest.strip_prefix("//") {
        Some(authority_and_path) => {
            let slash = authority_and_path.find('/')?;
            let host = &authority_and_path[..slash];
            if !host.is_empty() && host != "localhost" {
                return None;
            }
            &authority_and_path[slash..]
        }
        None => rest,
    };
    let path = percent_decode(rest);
    // `file:///C:/x` names `C:/x`.
    let path = match path.strip_prefix('/') {
        Some(drive) if cfg!(windows) && drive.as_bytes().get(1) == Some(&b':') => drive.to_owned(),
        _ => path,
    };
    Some(PathBuf::from(path))
}

/// Decodes `%XX` escapes; anything else is kept as it is.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                index += 3;
            }
            (byte, _) => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Adds the absolute URIs `expr` reads with `doc()`, `document()` or
/// `doc-available()` of a string literal.
pub(crate) fn literal_documents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Literal(_) | Expr::Var(_) | Expr::ContextItem | Expr::Root => {}
        Expr::Sequence(items) => items.iter().for_each(|item| literal_documents(item, out)),
        Expr::Filter(base, predicates) => {
            literal_documents(base, out);
            predicates
                .iter()
                .for_each(|item| literal_documents(item, out));
        }
        Expr::Step(Step { predicates, .. }) => predicates
            .iter()
            .for_each(|item| literal_documents(item, out)),
        Expr::Path(a, b)
        | Expr::Or(a, b)
        | Expr::And(a, b)
        | Expr::GeneralCompare(_, a, b)
        | Expr::ValueCompare(_, a, b)
        | Expr::NodeCompare(_, a, b)
        | Expr::Arithmetic(_, a, b)
        | Expr::Range(a, b)
        | Expr::Union(a, b)
        | Expr::Intersect(a, b)
        | Expr::Except(a, b)
        | Expr::Map(a, b)
        | Expr::Let(_, a, b)
        | Expr::For(_, a, b)
        | Expr::Quantified(_, _, a, b) => {
            literal_documents(a, out);
            literal_documents(b, out);
        }
        Expr::If(a, b, c) => {
            literal_documents(a, out);
            literal_documents(b, out);
            literal_documents(c, out);
        }
        Expr::Negate(a)
        | Expr::InstanceOf(a, _)
        | Expr::Cast(a, _, _)
        | Expr::Castable(a, _, _) => literal_documents(a, out),
        Expr::Call(function, args) => {
            if let (
                Function::Doc(base) | Function::Document(base) | Function::DocAvailable(base),
                Some(Expr::Literal(Atomic::Str(uri))),
            ) = (function, args.first())
                && let Some(uri) = resolve_uri(uri, base.as_deref())
            {
                out.push(uri.split('#').next().unwrap_or_default().to_owned());
            }
            args.iter().for_each(|arg| literal_documents(arg, out));
        }
    }
}

#[cfg(test)]
mod tests;
