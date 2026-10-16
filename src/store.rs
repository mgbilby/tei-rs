//! A local store of RELAX NG schemas, so documents can be validated against
//! the schema they declare (`<?xml-model href="…"?>`) without network access.
//!
//! The store is a directory holding `manifest.json` and the schema files.
//! Each manifest entry names a schema by its canonical URL, lists other hrefs
//! documents use for the same schema (`aliases`) and mirrors to fetch it from
//! when the canonical host is unavailable. Files are stored under a path
//! derived from the URL, so relative `include`s resolve inside the store.
//! Keeping the store up to date (the network side) is in [`crate::sync`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;

const MANIFEST: &str = "manifest.json";
const RELAX_NG_NS: &str = "http://relaxng.org/ns/structure/1.0";

/// One stored schema.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    /// Canonical URL: where the latest version of the schema is published.
    pub url: String,
    /// Other hrefs documents use to declare this schema (e.g. an `http://`
    /// variant or a relative path).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// URLs to fetch from, in order, when `url` itself can't be reached.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mirrors: Vec<String>,
    /// The stored copy, relative to the store directory; `None` until fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Files the schema pulls in with `include`/`externalRef`, relative to the
    /// store directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    /// SHA-256 of the stored file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Where the stored copy was fetched from (the URL or one of the mirrors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_from: Option<String>,
    /// When the stored copy was last fetched or confirmed current (UTC date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<String>,
}

impl Entry {
    /// Whether `href` names this schema.
    pub fn matches(&self, href: &str) -> bool {
        self.url == href || self.aliases.iter().any(|alias| alias == href)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    schemas: Vec<Entry>,
}

/// A schema store directory.
///
/// ```
/// use teixml::{Schema, SchemaStore};
///
/// # let dir = std::env::temp_dir().join("teixml-doc-store");
/// # std::fs::create_dir_all(dir.join("example.org"))?;
/// # std::fs::write(dir.join("example.org/tei.rnc"), "element TEI { empty }")?;
/// # std::fs::write(dir.join("manifest.json"), r#"{"schemas": [{
/// #   "url": "https://example.org/tei.rnc",
/// #   "aliases": ["http://example.org/tei.rnc"],
/// #   "file": "example.org/tei.rnc"
/// # }]}"#)?;
/// let store = SchemaStore::open(&dir)?; // or SchemaStore::default_dir()
/// for entry in store.entries() {
///     println!("{} ({:?})", entry.url, entry.file);
/// }
///
/// // Canonical URLs and aliases resolve to the stored copy.
/// let path = store.resolve("http://example.org/tei.rnc").expect("fetched");
/// let schema = Schema::from_file(path)?;
/// assert!(schema.validate_str("<TEI/>")?.is_empty());
/// assert!(store.resolve("https://example.org/other.rnc").is_none());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct SchemaStore {
    dir: PathBuf,
    manifest: Manifest,
}

impl SchemaStore {
    /// Opens the store in `dir`. A missing directory or manifest gives an
    /// empty store; nothing is written until [`SchemaStore::save`].
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let dir = dir.into();
        let path = dir.join(MANIFEST);
        let manifest = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|error| Error::Store(format!("{}: {error}", path.display())))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Manifest::default(),
            Err(source) => return Err(Error::Io { path, source }),
        };
        Ok(Self { dir, manifest })
    }

    /// The default store location: `$TEI_RS_SCHEMAS` if set, otherwise
    /// `tei-rs/schemas` in the user's cache directory.
    pub fn default_dir() -> PathBuf {
        if let Some(dir) = std::env::var_os("TEI_RS_SCHEMAS") {
            return dir.into();
        }
        let home = || std::env::var_os("HOME").map(PathBuf::from);
        let cache = if cfg!(target_os = "windows") {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            home().map(|home| home.join("Library/Caches"))
        } else {
            std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| home().map(|home| home.join(".cache")))
        };
        cache
            .unwrap_or_else(std::env::temp_dir)
            .join("tei-rs")
            .join("schemas")
    }

    /// The store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// All entries, in manifest order.
    pub fn entries(&self) -> &[Entry] {
        &self.manifest.schemas
    }

    #[cfg_attr(not(feature = "sync"), allow(dead_code))]
    pub(crate) fn entries_mut(&mut self) -> &mut Vec<Entry> {
        &mut self.manifest.schemas
    }

    /// The stored file for the schema `href` names, if it has been fetched.
    pub fn resolve(&self, href: &str) -> Option<PathBuf> {
        let entry = self
            .manifest
            .schemas
            .iter()
            .find(|entry| entry.matches(href))?;
        let file = entry.file.as_deref().filter(|file| is_store_path(file))?;
        let path = self.dir.join(file);
        path.is_file().then_some(path)
    }

    /// Writes `manifest.json`.
    pub fn save(&self) -> Result<(), Error> {
        std::fs::create_dir_all(&self.dir).map_err(|source| Error::Io {
            path: self.dir.clone(),
            source,
        })?;
        let path = self.dir.join(MANIFEST);
        let mut text = serde_json::to_string_pretty(&self.manifest)
            .map_err(|error| Error::Store(error.to_string()))?;
        text.push('\n');
        std::fs::write(&path, text).map_err(|source| Error::Io { path, source })
    }
}

/// The schema a document should be validated against: the `href` of its
/// first RELAX NG `<?xml-model?>` processing instruction, resolved against
/// the document's own path when it is relative and exists on disk, and
/// otherwise looked up in `store`.
///
/// ```
/// use std::path::Path;
/// use teixml::{Schema, SchemaStore, store};
///
/// # let dir = std::env::temp_dir().join("teixml-doc-schema-for-document");
/// # std::fs::create_dir_all(dir.join("example.org"))?;
/// # std::fs::write(dir.join("example.org/tei.rnc"), "element TEI { empty }")?;
/// # std::fs::write(dir.join("manifest.json"),
/// #     r#"{"schemas": [{"url": "https://example.org/tei.rnc", "file": "example.org/tei.rnc"}]}"#)?;
/// let store = SchemaStore::open(&dir)?;
/// let xml = r#"<?xml-model href="https://example.org/tei.rnc"?><TEI/>"#;
/// let path = store::schema_for_document(xml, Path::new("letters/1.xml"), &store)?;
/// let schema = Schema::from_file(path)?;
/// assert!(schema.validate_str(xml)?.is_empty());
///
/// // Undeclared or unknown schemas are an `Error::Store` saying what to do.
/// assert!(store::schema_for_document("<TEI/>", Path::new("2.xml"), &store).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn schema_for_document(
    xml: &str,
    document: &Path,
    store: &SchemaStore,
) -> Result<PathBuf, Error> {
    let hrefs = declared_schemas(xml);
    let Some(href) = hrefs.first() else {
        return Err(Error::Store(
            "the document declares no RELAX NG schema (<?xml-model?>); pass --schema".into(),
        ));
    };
    if !href.contains("://") {
        let local = document.parent().unwrap_or(Path::new("")).join(href);
        if local.is_file() {
            return Ok(local);
        }
    }
    store.resolve(href).ok_or_else(|| {
        let listed = store.entries().iter().any(|entry| entry.matches(href));
        Error::Store(if listed {
            format!(
                "schema `{href}` is listed in the schema store at {} but not downloaded yet; \
                 run `tei-rs schemas sync`",
                store.dir().display()
            )
        } else {
            format!(
                "schema `{href}` is not in the schema store at {}; run `tei-rs schemas add {href}`",
                store.dir().display()
            )
        })
    })
}

/// The hrefs of all `<?xml-model?>` processing instructions in `xml` that
/// declare a RELAX NG schema, in document order. A declaration without
/// `schematypens` counts when its href ends in `.rng` or `.rnc`.
///
/// ```
/// let xml = r#"<?xml version="1.0"?>
/// <?xml-model href="https://example.org/tei.rng"
///     schematypens="http://relaxng.org/ns/structure/1.0"?>
/// <?xml-model href="https://example.org/tei.sch"
///     schematypens="http://purl.oclc.org/dsdl/schematron"?>
/// <?xml-model href="local.rnc"?>
/// <TEI xmlns="http://www.tei-c.org/ns/1.0"/>"#;
/// assert_eq!(
///     teixml::store::declared_schemas(xml),
///     ["https://example.org/tei.rng", "local.rnc"]
/// );
/// ```
pub fn declared_schemas(xml: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    // Only the prolog (before the root element's start tag) declares schemas.
    let root = xml
        .match_indices('<')
        .map(|(index, _)| index)
        .find(|&index| !matches!(xml.as_bytes().get(index + 1), Some(b'?' | b'!')))
        .unwrap_or(xml.len());
    let mut rest = &xml[..root];
    while let Some(start) = rest.find("<?xml-model") {
        let after = &rest[start + "<?xml-model".len()..];
        let Some(end) = after.find("?>") else { break };
        let attributes = pseudo_attributes(&after[..end]);
        let get = |name: &str| {
            attributes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        if let Some(href) = get("href") {
            let relax_ng = match get("schematypens") {
                Some(ns) => ns == RELAX_NG_NS,
                None => href.ends_with(".rng") || href.ends_with(".rnc"),
            };
            if relax_ng {
                hrefs.push(href.to_owned());
            }
        }
        rest = &after[end..];
    }
    hrefs
}

/// Parses `name="value"` / `name='value'` pairs from a processing
/// instruction's content.
fn pseudo_attributes(text: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut rest = text;
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].trim().to_owned();
        let value_part = rest[eq + 1..].trim_start();
        let Some(quote) = value_part
            .chars()
            .next()
            .filter(|c| *c == '"' || *c == '\'')
        else {
            break;
        };
        let Some(close) = value_part[1..].find(quote) else {
            break;
        };
        pairs.push((name, value_part[1..1 + close].to_owned()));
        rest = &value_part[close + 2..];
    }
    pairs
}

/// Whether `path` is a plain store-relative path: `/`-separated names, none
/// of them empty, `.` or `..`, and nothing a file system could read as a
/// root, drive or other separator. Paths from the manifest are checked
/// before use, so an edited manifest can't point outside the store.
pub(crate) fn is_store_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':', '\0'])
        })
}

/// The store-relative path a schema fetched from `url` is saved under:
/// host and path, with anything that could escape the store removed.
#[cfg_attr(not(feature = "sync"), allow(dead_code))]
pub(crate) fn file_for_url(url: &str) -> String {
    let without_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let without_query = without_scheme.split(['?', '#']).next().unwrap_or_default();
    without_query
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .map(|part| part.replace([':', '\\', '<', '>', '"', '|', '?', '*', '\0'], "_"))
        .filter(|part| part != "." && part != "..")
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg_attr(not(feature = "sync"), allow(dead_code))]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_paths_stay_inside_the_store() {
        assert!(is_store_path("host/a/b.rng"));
        for bad in [
            "",
            "/etc/passwd",
            "a/../../b",
            "a//b",
            "./a",
            "C:/x",
            "a\\..\\b",
            "a/b\0",
        ] {
            assert!(!is_store_path(bad), "{bad:?}");
        }
        for url in [
            "https://h/a\\..\\..\\x.rng",
            "https://h:443/C:/x.rng",
            "https://h/%2e%2e/x.rng",
            "https://h/a/<b>|c?d",
        ] {
            assert!(
                is_store_path(&file_for_url(url)),
                "{url}: {}",
                file_for_url(url)
            );
        }
    }

    #[test]
    fn finds_relax_ng_declarations_only() {
        let xml = r#"<?xml version="1.0"?>
<?xml-model href="https://example.org/s.rng" type="application/xml"
    schematypens="http://relaxng.org/ns/structure/1.0"?>
<?xml-model href='https://example.org/s.sch' schematypens='http://purl.oclc.org/dsdl/schematron'?>
<?xml-model href="local.rnc"?>
<TEI xmlns="http://www.tei-c.org/ns/1.0"><?xml-model href="ignored.rng"?></TEI>"#;
        assert_eq!(
            declared_schemas(xml),
            ["https://example.org/s.rng", "local.rnc"]
        );
    }

    #[test]
    fn url_paths_stay_inside_the_store() {
        assert_eq!(
            file_for_url("https://tei-c.org/release/xml/tei_all.rng"),
            "tei-c.org/release/xml/tei_all.rng"
        );
        assert_eq!(
            file_for_url("https://evil.example/../../etc/passwd"),
            "evil.example/etc/passwd"
        );
        assert_eq!(
            file_for_url("http://host:8080/a.rng?x=1"),
            "host_8080/a.rng"
        );
    }
}
