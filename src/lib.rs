#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Validate TEI XML documents against RELAX NG schemas, in pure Rust.
//!
//! With the `schematron` feature (on by default), the ISO Schematron rules
//! a schema embeds in either syntax, as TEI ODD-generated schemas do, are
//! checked too; see [`Schema::set_schematron`] and the `schematron` module.
//!
//! ```no_run
//! let schema = teixml::Schema::from_file("tei_all.rng")?;
//! let xml = std::fs::read_to_string("edition.xml")?;
//! for diagnostic in schema.validate_str(&xml)? {
//!     println!("{}: {diagnostic}", diagnostic.severity());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod document;
mod limits;
pub mod relaxng;
#[cfg(feature = "schematron")]
pub mod schematron;
pub mod store;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "schematron")]
mod xpath;

use std::fmt;
use std::path::{Path, PathBuf};

use crate::relaxng::{
    DatatypeRegistry, ResolveError, SchemaResolver, SchemaSource, SchemaSyntax, ValidationErrorKind,
};

pub use document::Position;
pub use limits::MAX_DOCUMENT_DEPTH;
pub use relaxng::ElementPath;
pub use store::SchemaStore;

/// Runs `work` on its own thread with a large stack. Compiling schemas and
/// validating recurse once per level of nesting; this makes input at the
/// nesting limits fit whatever stack the caller has (debug builds especially
/// use large frames). The stack is reserved, not committed, so this costs
/// little; if the thread can't be created, `work` runs on the caller's.
fn on_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    const STACK: usize = 256 * 1024 * 1024;
    let work = std::sync::Mutex::new(Some(work));
    let run = || {
        (work
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .expect("runs once"))()
    };
    std::thread::scope(|scope| {
        match std::thread::Builder::new()
            .name("teixml".into())
            .stack_size(STACK)
            .spawn_scoped(scope, run)
        {
            Ok(thread) => thread
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic)),
            Err(_) => run(),
        }
    })
}

/// A compiled RELAX NG schema, ready to validate documents.
///
/// With the `schematron` feature, validation also checks the Schematron
/// rules the schema embeds, unless turned off with `set_schematron(false)`.
/// The rules are compiled on first use, once per schema.
///
/// ```
/// use teixml::{relaxng::SchemaSyntax, Schema};
///
/// let schema = Schema::from_text(
///     "element TEI { element text { text } }",
///     "",
///     SchemaSyntax::Compact,
/// )?;
/// assert!(schema.validate_str("<TEI><text>Hello</text></TEI>")?.is_empty());
///
/// let diagnostics = schema.validate_str("<TEI><body/></TEI>")?;
/// assert!(diagnostics.iter().any(|d| d.is_error()));
/// # Ok::<(), teixml::Error>(())
/// ```
pub struct Schema {
    inner: crate::relaxng::Schema,
    registry: DatatypeRegistry,
    #[cfg(feature = "schematron")]
    schematron: EmbeddedSchematron,
}

/// The schema's own text, from which its Schematron rules are compiled when
/// they are first needed (so RELAX NG-only validation never pays for them).
#[cfg(feature = "schematron")]
struct EmbeddedSchematron {
    enabled: bool,
    source: SchemaSource,
    compiled: std::sync::OnceLock<Result<schematron::Schematron, String>>,
    /// What the rules may read besides the schema's and the document's
    /// directories.
    access: xpath::Access,
}

impl Schema {
    /// Reads and compiles a schema file. The syntax is chosen from the
    /// extension: `.rnc` is the compact syntax, anything else the XML syntax.
    /// `include`/`externalRef` are resolved relative to the including file.
    ///
    /// ```
    /// # let dir = std::env::temp_dir().join("teixml-doc-from-file");
    /// # std::fs::create_dir_all(&dir)?;
    /// # std::fs::write(dir.join("text.rnc"), "body = element text { text }")?;
    /// # std::fs::write(dir.join("doc.rnc"), "include \"text.rnc\"\nstart = element TEI { body }")?;
    /// // doc.rnc includes text.rnc from the same directory.
    /// let schema = teixml::Schema::from_file(dir.join("doc.rnc"))?;
    /// assert!(schema.validate_str("<TEI><text/></TEI>")?.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_text(&text, &path.to_string_lossy(), syntax_for(path))
    }

    /// Compiles a schema from text. `base_uri` is the path that relative
    /// `include`/`externalRef` hrefs are resolved against.
    ///
    /// ```
    /// use teixml::{relaxng::SchemaSyntax, Schema};
    ///
    /// let rng = r#"<element name="p" xmlns="http://relaxng.org/ns/structure/1.0"
    ///     datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
    ///   <attribute name="n"><data type="integer"/></attribute>
    ///   <text/>
    /// </element>"#;
    /// let schema = Schema::from_text(rng, "schemas/p.rng", SchemaSyntax::Xml)?;
    /// assert!(schema.validate_str(r#"<p n="1">text</p>"#)?.is_empty());
    /// assert!(!schema.validate_str(r#"<p n="one">text</p>"#)?.is_empty());
    /// # Ok::<(), teixml::Error>(())
    /// ```
    pub fn from_text(text: &str, base_uri: &str, syntax: SchemaSyntax) -> Result<Self, Error> {
        let source = SchemaSource::new(text, base_uri, syntax);
        let inner = on_large_stack(|| crate::relaxng::Schema::compile(&source, &FileResolver))
            .map_err(|error| Error::Schema(error.to_string()))?;
        Ok(Self {
            inner,
            registry: DatatypeRegistry::new(),
            #[cfg(feature = "schematron")]
            schematron: EmbeddedSchematron {
                enabled: true,
                source,
                compiled: std::sync::OnceLock::new(),
                access: xpath::Access::default(),
            },
        })
    }

    /// Turns checking of the schema's embedded Schematron rules on (the
    /// default) or off. Off, validation is RELAX NG only.
    ///
    /// ```
    /// # let schema_text = "element TEI { empty }";
    /// let mut schema =
    ///     teixml::Schema::from_text(schema_text, "", teixml::relaxng::SchemaSyntax::Compact)?;
    /// schema.set_schematron(false);
    /// assert!(!schema.schematron_enabled());
    /// # Ok::<(), teixml::Error>(())
    /// ```
    #[cfg(feature = "schematron")]
    pub fn set_schematron(&mut self, enabled: bool) {
        self.schematron.enabled = enabled;
    }

    /// Gives the schema's Schematron rules a schema store to read from.
    /// Rules may read other XML documents with `doc()` and XSLT's
    /// `document()`: files under the schema's directory, the validated
    /// document's directory and, once this is set, the store directory.
    /// Validation never touches the network, so documents named by
    /// `http(s)` URIs are read from the store, where
    /// `sync::add` (`tei-rs schemas add URL`) puts them;
    /// [`Schematron::remote_documents`](schematron::Schematron::remote_documents)
    /// lists the ones a schema names. A document that can't be read makes
    /// the rules that read it report an evaluation error, and
    /// `doc-available()` false.
    ///
    /// ```
    /// use teixml::{relaxng::SchemaSyntax, DiagnosticKind, Schema, SchemaStore};
    ///
    /// # let dir = std::env::temp_dir().join("teixml-doc-set-store");
    /// # std::fs::create_dir_all(dir.join("example.org"))?;
    /// # std::fs::write(dir.join("example.org/kinds.xml"), "<kinds><kind>poem</kind></kinds>")?;
    /// # std::fs::write(dir.join("manifest.json"), r#"{"schemas": [{
    /// #   "url": "https://example.org/kinds.xml", "file": "example.org/kinds.xml"
    /// # }]}"#)?;
    /// let rng = r#"<element name="text" xmlns="http://relaxng.org/ns/structure/1.0"
    ///     xmlns:sch="http://purl.oclc.org/dsdl/schematron">
    ///   <sch:pattern>
    ///     <sch:rule context="text">
    ///       <sch:let name="kinds" value="doc('https://example.org/kinds.xml')//kind"/>
    ///       <sch:assert test="@type = $kinds">unknown type <sch:value-of select="@type"/></sch:assert>
    ///     </sch:rule>
    ///   </sch:pattern>
    ///   <attribute name="type"/>
    /// </element>"#;
    /// let mut schema = Schema::from_text(rng, "text.rng", SchemaSyntax::Xml)?;
    /// let store = SchemaStore::open(&dir)?; // holds https://example.org/kinds.xml
    /// assert_eq!(schema.schematron()?.remote_documents(), ["https://example.org/kinds.xml"]);
    /// assert!(store.resolve("https://example.org/kinds.xml").is_some());
    /// schema.set_store(&store);
    ///
    /// assert!(schema.validate_str(r#"<text type="poem"/>"#)?.is_empty());
    /// let diagnostics = schema.validate_str(r#"<text type="novel"/>"#)?;
    /// assert_eq!(diagnostics[0].message().to_string(), "Schematron assert failed: unknown type novel");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(feature = "schematron")]
    pub fn set_store(&mut self, store: &SchemaStore) {
        self.schematron.access.store = Some(store.clone());
    }

    /// Whether validation checks the embedded Schematron rules.
    #[cfg(feature = "schematron")]
    pub fn schematron_enabled(&self) -> bool {
        self.schematron.enabled
    }

    /// The schema's embedded Schematron rules, compiled on the first call.
    /// Use it to see how many tests there are and which ones are
    /// [unsupported](schematron::Schematron::unsupported) and skipped.
    /// Fails if the schema, or a file it includes, can't be read again.
    ///
    /// ```
    /// use teixml::{relaxng::SchemaSyntax, DiagnosticKind, Schema, Severity};
    ///
    /// let rng = r#"<element name="list" xmlns="http://relaxng.org/ns/structure/1.0"
    ///     xmlns:sch="http://purl.oclc.org/dsdl/schematron">
    ///   <sch:pattern>
    ///     <sch:rule context="list">
    ///       <sch:assert test="item">a list needs at least one item</sch:assert>
    ///       <sch:report test="count(item) gt 3" role="warning">a long list</sch:report>
    ///     </sch:rule>
    ///   </sch:pattern>
    ///   <zeroOrMore><element name="item"><text/></element></zeroOrMore>
    /// </element>"#;
    /// let schema = Schema::from_text(rng, "list.rng", SchemaSyntax::Xml)?;
    ///
    /// let rules = schema.schematron()?;
    /// assert_eq!(rules.test_count(), 2);
    /// assert!(rules.unsupported().is_empty());
    ///
    /// // Valid RELAX NG, but the assert fails.
    /// let diagnostics = schema.validate_str("<list/>")?;
    /// assert_eq!(diagnostics.len(), 1);
    /// assert_eq!(diagnostics[0].severity(), Severity::Error);
    /// match &diagnostics[0].kind {
    ///     DiagnosticKind::Schematron(message) => {
    ///         assert_eq!(message.text, "a list needs at least one item");
    ///     }
    ///     other => panic!("unexpected {other}"),
    /// }
    /// # Ok::<(), teixml::Error>(())
    /// ```
    #[cfg(feature = "schematron")]
    pub fn schematron(&self) -> Result<&schematron::Schematron, Error> {
        let compiled = self.schematron.compiled.get_or_init(|| {
            on_large_stack(|| {
                schematron::Schematron::from_relaxng(&self.schematron.source, &|href, base| {
                    FileResolver
                        .resolve(href, base)
                        .map_err(|error| error.to_string())
                })
            })
        });
        compiled
            .as_ref()
            .map_err(|error| Error::Schema(format!("Schematron rules: {error}")))
    }

    /// Validates an XML document given as text. Returns every problem found,
    /// RELAX NG errors first, then Schematron messages; the document is
    /// valid when none of them [is an error](Diagnostic::is_error).
    ///
    /// ```
    /// use teixml::{relaxng::SchemaSyntax, Schema};
    ///
    /// let schema = Schema::from_text(
    ///     "element TEI { element text { element p { text }+ } }",
    ///     "",
    ///     SchemaSyntax::Compact,
    /// )?;
    /// let xml = "<TEI>\n  <text>\n    <div/>\n  </text>\n</TEI>";
    /// for diagnostic in schema.validate_str(xml)? {
    ///     // Prints e.g. `3:10 error: unexpected element ... at /TEI/text`.
    ///     if let Some(position) = diagnostic.position {
    ///         print!("{}:{} ", position.line, position.column);
    ///     }
    ///     println!("{}: {} at {}", diagnostic.severity(), diagnostic.message(), diagnostic.path);
    /// }
    /// # Ok::<(), teixml::Error>(())
    /// ```
    pub fn validate_str(&self, xml: &str) -> Result<Vec<Diagnostic>, Error> {
        self.validate(xml, None)
    }

    /// Like [`validate_str`](Self::validate_str), for a document that was
    /// read from `path`. The file isn't read again: Schematron rules see the
    /// path as the document's `document-uri()`.
    ///
    /// ```
    /// # let dir = std::env::temp_dir().join("teixml-doc-validate-str-from");
    /// # std::fs::create_dir_all(&dir)?;
    /// # let path = dir.join("letter.xml");
    /// # std::fs::write(&path, "<TEI/>")?;
    /// # let schema = teixml::Schema::from_text("element TEI { empty }", "", teixml::relaxng::SchemaSyntax::Compact)?;
    /// let xml = std::fs::read_to_string(&path)?;
    /// // ... inspect or transform `xml` ...
    /// let diagnostics = schema.validate_str_from(&xml, &path)?;
    /// # assert!(diagnostics.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn validate_str_from(
        &self,
        xml: &str,
        path: impl AsRef<Path>,
    ) -> Result<Vec<Diagnostic>, Error> {
        self.validate(xml, Some(path.as_ref()))
    }

    /// Reads and validates an XML file.
    ///
    /// ```
    /// # let dir = std::env::temp_dir().join("teixml-doc-validate-file");
    /// # std::fs::create_dir_all(&dir)?;
    /// # std::fs::write(dir.join("letter.xml"), "<TEI/>")?;
    /// # let schema = teixml::Schema::from_text("element TEI { empty }", "", teixml::relaxng::SchemaSyntax::Compact)?;
    /// let diagnostics = schema.validate_file(dir.join("letter.xml"))?;
    /// let valid = !diagnostics.iter().any(|d| d.is_error());
    /// # assert!(valid);
    ///
    /// // A missing file is an `Error::Io`, not a diagnostic.
    /// assert!(matches!(
    ///     schema.validate_file(dir.join("missing.xml")),
    ///     Err(teixml::Error::Io { .. })
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn validate_file(&self, path: impl AsRef<Path>) -> Result<Vec<Diagnostic>, Error> {
        let path = path.as_ref();
        let xml = std::fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        self.validate(&xml, Some(path))
    }

    fn validate(&self, xml: &str, path: Option<&Path>) -> Result<Vec<Diagnostic>, Error> {
        on_large_stack(|| self.validate_here(xml, path))
    }

    fn validate_here(&self, xml: &str, path: Option<&Path>) -> Result<Vec<Diagnostic>, Error> {
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        };
        if limits::xml_nesting_exceeds(xml, MAX_DOCUMENT_DEPTH) {
            return Err(Error::Xml(format!(
                "elements nested more than {MAX_DOCUMENT_DEPTH} levels deep"
            )));
        }
        let doc = roxmltree::Document::parse_with_options(xml, options)
            .map_err(|error| Error::Xml(error.to_string()))?;
        let root = document::Node::root(&doc);
        let errors = self
            .inner
            .validate(&self.registry, &root)
            .map_err(|error| Error::Schema(error.to_string()))?;
        let mut found: Vec<(Option<usize>, ValidationErrorKind, ElementPath)> = errors
            .into_iter()
            .map(|error| {
                (
                    error.location().copied(),
                    error.kind().clone(),
                    error.path().clone(),
                )
            })
            .collect();
        let ids = self.inner.id_types();
        if !ids.is_empty() {
            // Duplicate IDs go in document order among the other errors,
            // after any at the same position; unresolved references last.
            let (duplicates, unresolved) = document::id_errors(&doc, ids);
            let mut merged = Vec::with_capacity(found.len() + duplicates.len() + unresolved.len());
            let mut found_iter = found.into_iter().peekable();
            let mut duplicates = duplicates.into_iter().peekable();
            while let Some((offset, ..)) = duplicates.peek() {
                let offset = *offset;
                while found_iter
                    .peek()
                    .is_some_and(|(at, ..)| at.is_none_or(|at| at <= offset))
                {
                    merged.extend(found_iter.next());
                }
                // A duplicate and its first occurrence stay together.
                merged.extend(
                    duplicates
                        .next()
                        .map(|(at, kind, path)| (Some(at), kind, path)),
                );
                merged.extend(
                    duplicates
                        .next()
                        .map(|(at, kind, path)| (Some(at), kind, path)),
                );
            }
            merged.extend(found_iter);
            merged.extend(
                unresolved
                    .into_iter()
                    .map(|(at, kind, path)| (Some(at), kind, path)),
            );
            found = merged;
        }
        // Positions are looked up in a line index, built only if needed.
        let lines = std::cell::OnceCell::new();
        let diagnostics: Vec<Diagnostic> = found
            .into_iter()
            .map(|(offset, kind, path)| Diagnostic {
                position: offset.map(|offset| {
                    lines
                        .get_or_init(|| document::LineIndex::new(xml))
                        .position(offset)
                }),
                kind: DiagnosticKind::RelaxNg(kind),
                path,
            })
            .collect();
        #[cfg(feature = "schematron")]
        let diagnostics = self.check_schematron(&doc, xml, &lines, path, diagnostics)?;
        #[cfg(not(feature = "schematron"))]
        let _ = path;
        Ok(diagnostics)
    }

    #[cfg(feature = "schematron")]
    fn check_schematron<'x>(
        &self,
        doc: &roxmltree::Document<'_>,
        xml: &'x str,
        lines: &std::cell::OnceCell<document::LineIndex<'x>>,
        path: Option<&Path>,
        mut diagnostics: Vec<Diagnostic>,
    ) -> Result<Vec<Diagnostic>, Error> {
        if !self.schematron.enabled {
            return Ok(diagnostics);
        }
        let rules = self.schematron()?;
        if rules.is_empty() {
            return Ok(diagnostics);
        }
        for (node, message) in rules.check(doc, path, &self.schematron.access) {
            let lines = lines.get_or_init(|| document::LineIndex::new(xml));
            diagnostics.push(Diagnostic::schematron(lines, node, message));
        }
        Ok(diagnostics)
    }
}

/// One validation problem in a document.
///
/// ```
/// use teixml::{relaxng::SchemaSyntax, Position, Schema, Severity};
///
/// let schema = Schema::from_text(
///     "element TEI { element text { empty } }",
///     "",
///     SchemaSyntax::Compact,
/// )?;
/// let diagnostics = schema.validate_str("<TEI>\n<text><div/></text>\n</TEI>")?;
/// let first = &diagnostics[0];
/// assert_eq!(first.severity(), Severity::Error);
/// assert_eq!(first.position, Some(Position { line: 2, column: 13 }));
/// // The element the problem was found in.
/// assert_eq!(first.path.to_string(), "/TEI/text");
/// assert_eq!(first.message().to_string(), "unexpected element `div`");
/// // Displays as all of the above together.
/// println!("{first}");
/// # Ok::<(), teixml::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Where in the document the problem was found.
    pub position: Option<Position>,
    /// What went wrong.
    pub kind: DiagnosticKind,
    /// The elements from the root down to the one the problem was reported
    /// on; displays as `/TEI/text/body`.
    pub path: ElementPath,
}

/// What a diagnostic is about, and where it comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// The document doesn't match the RELAX NG schema.
    RelaxNg(ValidationErrorKind),
    /// A Schematron `assert` failed or a `report` fired, or a rule could
    /// not be evaluated.
    #[cfg(feature = "schematron")]
    Schematron(schematron::Message),
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RelaxNg(kind) => write!(f, "{kind}"),
            #[cfg(feature = "schematron")]
            Self::Schematron(message) => write!(f, "{message}"),
        }
    }
}

/// How serious a diagnostic is. RELAX NG errors are always errors;
/// Schematron messages take theirs from the test's `role`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The document is invalid.
    Error,
    /// Worth a look; the document is still valid.
    Warning,
    /// For information; the document is still valid.
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        })
    }
}

impl Diagnostic {
    /// How serious this diagnostic is.
    pub fn severity(&self) -> Severity {
        match &self.kind {
            DiagnosticKind::RelaxNg(_) => Severity::Error,
            #[cfg(feature = "schematron")]
            DiagnosticKind::Schematron(message) => message.severity,
        }
    }

    /// Whether this diagnostic makes the document invalid.
    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }

    #[cfg(feature = "schematron")]
    fn schematron(
        lines: &document::LineIndex<'_>,
        node: xpath::XNode<'_>,
        message: schematron::Message,
    ) -> Self {
        let path = node.element().map(document::path).unwrap_or_default();
        Self {
            position: Some(lines.position(node.source_offset())),
            kind: DiagnosticKind::Schematron(message),
            path,
        }
    }
}

impl Diagnostic {
    /// What went wrong, as a sentence: the kind, plus a hint where it helps.
    pub fn message(&self) -> impl fmt::Display + '_ {
        Message(self)
    }
}

/// [`Diagnostic::message`].
struct Message<'d>(&'d Diagnostic);

impl fmt::Display for Message<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let diagnostic = self.0;
        write!(f, "{}", diagnostic.kind)?;
        // A root element in no namespace is usually a missing `xmlns`.
        if let DiagnosticKind::RelaxNg(ValidationErrorKind::UnexpectedElement(name)) =
            &diagnostic.kind
            && diagnostic.path.is_empty()
            && name.namespace.as_deref().unwrap_or_default().is_empty()
        {
            f.write_str(" (in no namespace)")?;
        }
        Ok(())
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(position) = self.position {
            write!(f, "{position}: ")?;
        }
        write!(f, "{}", self.message())?;
        if !self.path.is_empty() {
            write!(f, " (in {})", self.path)?;
        }
        Ok(())
    }
}

/// Why a schema or document couldn't be processed at all.
#[derive(Debug)]
pub enum Error {
    /// A file couldn't be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The schema is not a usable RELAX NG schema.
    Schema(String),
    /// The document is not well-formed XML.
    Xml(String),
    /// The schema store is unusable, or lacks a schema a document needs.
    Store(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Schema(message) => write!(f, "schema error: {message}"),
            Self::Xml(message) => write!(f, "XML error: {message}"),
            Self::Store(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn syntax_for(path: &Path) -> SchemaSyntax {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rnc") => SchemaSyntax::Compact,
        _ => SchemaSyntax::Xml,
    }
}

/// Resolves `include`/`externalRef` hrefs as file paths relative to the
/// including schema. URLs are refused: validation never touches the network.
struct FileResolver;

impl SchemaResolver for FileResolver {
    fn resolve(&self, href: &str, base_uri: &str) -> Result<SchemaSource, ResolveError> {
        if href.contains("://") {
            return Err(ResolveError::new(format!(
                "refusing to fetch `{href}`: only local files are supported"
            )));
        }
        let path = Path::new(base_uri)
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(href);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| ResolveError::new(format!("{}: {error}", path.display())))?;
        Ok(SchemaSource::new(
            text,
            path.to_string_lossy(),
            syntax_for(&path),
        ))
    }
}

/// Compiles and runs the examples in README.md as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
