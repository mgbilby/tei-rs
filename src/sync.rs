//! Keeping a [`SchemaStore`] up to date: fetches each schema from its
//! canonical URL (or a mirror), compares it, and every file it `include`s,
//! with the stored copy, and optionally writes the new version.

use std::path::{Component, Path};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Error;
use crate::store::{Entry, SchemaStore, file_for_url, sha256_hex};

/// Largest schema file accepted, in bytes.
const MAX_SCHEMA_BYTES: u64 = 64 * 1024 * 1024;
/// The most files one schema may consist of, counting its includes.
const MAX_SCHEMA_FILES: usize = 1000;

/// How a stored schema compares with the published one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The stored copy matches the published schema.
    Current,
    /// The published schema differs from the stored copy.
    Outdated,
    /// Listed in the manifest but never fetched.
    Missing,
    /// Was outdated or missing and has now been replaced (sync only).
    Updated,
    /// Neither the URL nor any mirror could be fetched.
    Unreachable(String),
}

/// The outcome for one manifest entry.
#[derive(Clone, Debug)]
pub struct Report {
    /// The schema's canonical URL.
    pub url: String,
    /// What was found.
    pub status: Status,
    /// Where the published version was fetched from, if anywhere.
    pub source: Option<String>,
}

/// Compares every schema in `store` (or only those whose URL contains
/// `filter`) with its published version. With `apply`, outdated and missing
/// schemas are downloaded and the manifest is saved.
///
/// ```no_run
/// use teixml::{SchemaStore, sync};
///
/// let mut store = SchemaStore::open(SchemaStore::default_dir())?;
/// // Check only the TEI schemas, without writing anything.
/// for report in sync::sync(&mut store, Some("tei-c.org"), false)? {
///     println!("{}: {:?}", report.url, report.status);
/// }
/// # Ok::<(), teixml::Error>(())
/// ```
pub fn sync(
    store: &mut SchemaStore,
    filter: Option<&str>,
    apply: bool,
) -> Result<Vec<Report>, Error> {
    let agent = agent();
    let mut reports = Vec::new();
    let mut changed = false;
    let indices: Vec<usize> = (0..store.entries().len())
        .filter(|&index| filter.is_none_or(|filter| store.entries()[index].url.contains(filter)))
        .collect();
    for index in indices {
        let entry = store.entries()[index].clone();
        let (report, updated) = sync_entry(&agent, store.dir(), &entry, apply);
        if let Some(updated) = updated {
            store.entries_mut()[index] = updated;
            changed = true;
        }
        reports.push(report);
    }
    if apply && changed {
        store.save()?;
    }
    Ok(reports)
}

/// Adds the schema published at `url` to `store` (with optional alias hrefs
/// and mirrors), downloads it with its dependencies, and saves the manifest.
/// If `url` is already listed, its aliases and mirrors are extended instead.
///
/// ```no_run
/// use teixml::{SchemaStore, sync};
///
/// let mut store = SchemaStore::open(SchemaStore::default_dir())?;
/// let report = sync::add(
///     &mut store,
///     "https://example.org/schema/project.rng",
///     &["http://example.org/schema/project.rng".to_owned()],
///     &[],
/// )?;
/// println!("{:?} from {:?}", report.status, report.source);
/// # Ok::<(), teixml::Error>(())
/// ```
pub fn add(
    store: &mut SchemaStore,
    url: &str,
    aliases: &[String],
    mirrors: &[String],
) -> Result<Report, Error> {
    let index = match store.entries().iter().position(|entry| entry.url == url) {
        Some(index) => index,
        None => {
            store.entries_mut().push(Entry {
                url: url.to_owned(),
                ..Entry::default()
            });
            store.entries().len() - 1
        }
    };
    let entry = &mut store.entries_mut()[index];
    for alias in aliases {
        if !entry.aliases.contains(alias) {
            entry.aliases.push(alias.clone());
        }
    }
    for mirror in mirrors {
        if !entry.mirrors.contains(mirror) {
            entry.mirrors.push(mirror.clone());
        }
    }
    let entry = entry.clone();
    let (report, updated) = sync_entry(&agent(), store.dir(), &entry, true);
    if let Some(updated) = updated {
        store.entries_mut()[index] = updated;
    }
    store.save()?;
    Ok(report)
}

/// A downloaded schema file and everything it includes.
struct Fetched {
    source: String,
    /// (store-relative path, contents); the first is the schema itself.
    files: Vec<(String, Vec<u8>)>,
}

fn sync_entry(
    agent: &ureq::Agent,
    dir: &Path,
    entry: &Entry,
    apply: bool,
) -> (Report, Option<Entry>) {
    let fetched = match fetch_with_dependencies(agent, entry) {
        Ok(fetched) => fetched,
        Err(error) => {
            let report = Report {
                url: entry.url.clone(),
                status: Status::Unreachable(error),
                source: None,
            };
            return (report, None);
        }
    };
    let stored_matches = |(path, bytes): &(String, Vec<u8>)| {
        std::fs::read(dir.join(path)).is_ok_and(|stored| &stored == bytes)
    };
    let main_present = entry
        .file
        .as_deref()
        .filter(|file| crate::store::is_store_path(file))
        .is_some_and(|file| dir.join(file).is_file());
    let status = if !main_present {
        Status::Missing
    } else if entry.file.as_deref() == Some(fetched.files[0].0.as_str())
        && fetched.files.iter().all(stored_matches)
    {
        Status::Current
    } else {
        Status::Outdated
    };

    let mut updated = None;
    let mut final_status = status.clone();
    if apply {
        let mut entry = entry.clone();
        if status != Status::Current {
            for (path, bytes) in &fetched.files {
                debug_assert!(crate::store::is_store_path(path), "{path}");
                let target = dir.join(path);
                let written = target
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&target, bytes));
                if let Err(error) = written {
                    let report = Report {
                        url: entry.url.clone(),
                        status: Status::Unreachable(format!("{}: {error}", target.display())),
                        source: Some(fetched.source.clone()),
                    };
                    return (report, None);
                }
            }
            entry.file = Some(fetched.files[0].0.clone());
            entry.dependencies = fetched.files[1..]
                .iter()
                .map(|(path, _)| path.clone())
                .collect();
            entry.sha256 = Some(sha256_hex(&fetched.files[0].1));
            final_status = Status::Updated;
        }
        entry.fetched_from = Some(fetched.source.clone());
        entry.checked = Some(today());
        updated = Some(entry);
    }
    let report = Report {
        url: entry.url.clone(),
        status: final_status,
        source: Some(fetched.source),
    };
    (report, updated)
}

/// Fetches the schema from its URL or, failing that, each mirror in turn,
/// then every file it includes (relative to wherever it was fetched from).
fn fetch_with_dependencies(agent: &ureq::Agent, entry: &Entry) -> Result<Fetched, String> {
    let main_file = file_for_url(&entry.url);
    let mut errors = Vec::new();
    for source in std::iter::once(&entry.url).chain(&entry.mirrors) {
        let result = (|| {
            let mut files = vec![(main_file.clone(), get(agent, source)?)];
            let mut index = 0;
            // Breadth-first over includes; each file's hrefs are relative to
            // that file, both at the source and inside the store.
            let mut sources = vec![source.clone()];
            while index < files.len() {
                for href in schema_references(&files[index].1) {
                    let Some(path) = join_relative(&files[index].0, &href) else {
                        return Err(format!("refusing to follow `{href}`"));
                    };
                    if files.iter().any(|(existing, _)| *existing == path) {
                        continue;
                    }
                    if files.len() == MAX_SCHEMA_FILES {
                        return Err(format!("more than {MAX_SCHEMA_FILES} included files"));
                    }
                    let url = join_url(&sources[index], &href);
                    files.push((path, get(agent, &url)?));
                    sources.push(url);
                }
                index += 1;
            }
            Ok(Fetched {
                source: source.clone(),
                files,
            })
        })();
        match result {
            Ok(fetched) => return Ok(fetched),
            Err(error) => errors.push(format!("{source}: {error}")),
        }
    }
    Err(errors.join("; "))
}

fn agent() -> ureq::Agent {
    // The operating system's certificate store, so corporate or other
    // intercepting proxies (configured via HTTPS_PROXY) keep working.
    // HTTPS only, redirects included.
    ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(120)))
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn get(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let mut response = agent.get(url).call().map_err(|error| error.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_SCHEMA_BYTES)
        .read_to_vec()
        .map_err(|error| error.to_string())
}

/// Relative hrefs of `include`/`externalRef` in an XML-syntax schema, or of
/// `include`/`external` in a compact-syntax one. Absolute URLs are left out:
/// they are separate schemas.
fn schema_references(bytes: &[u8]) -> Vec<String> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let mut hrefs = Vec::new();
    if crate::limits::xml_nesting_exceeds(text, crate::limits::MAX_SCHEMA_DEPTH) {
        return hrefs;
    }
    if let Ok(doc) = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    ) {
        for node in doc.descendants() {
            if matches!(node.tag_name().name(), "include" | "externalRef")
                && node.tag_name().namespace() == Some("http://relaxng.org/ns/structure/1.0")
                && let Some(href) = node.attribute("href")
            {
                hrefs.push(href.to_owned());
            }
        }
    } else {
        hrefs = compact_references(text);
    }
    hrefs.retain(|href| !href.contains("://"));
    hrefs.dedup();
    hrefs
}

/// The literals following the `include` and `external` keywords of a
/// compact-syntax schema. Comments, documentation and other literals are
/// skipped, so the words only count where they are keywords.
fn compact_references(text: &str) -> Vec<String> {
    #[derive(PartialEq)]
    enum Token<'a> {
        Word(&'a str),
        Literal(&'a str),
        Other,
    }
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            quote @ (b'"' | b'\'') => {
                let delimiter: &[u8] = if bytes[i..].starts_with(&[quote; 3]) {
                    &bytes[i..i + 3]
                } else {
                    &bytes[i..i + 1]
                };
                let start = i + delimiter.len();
                let end = bytes[start..]
                    .windows(delimiter.len())
                    .position(|window| window == delimiter)
                    .map_or(bytes.len(), |offset| start + offset);
                tokens.push(Token::Literal(&text[start..end]));
                i = (end + delimiter.len()).min(bytes.len());
            }
            b if b.is_ascii_alphanumeric() || b == b'_' || b == b'\\' => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric()
                        || matches!(bytes[i], b'_' | b'-' | b'.' | b':' | b'\\'))
                {
                    i += 1;
                }
                tokens.push(Token::Word(&text[start..i]));
            }
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                tokens.push(Token::Other);
                i += 1;
            }
        }
    }
    let mut hrefs = Vec::new();
    for pair in tokens.windows(2) {
        if let [Token::Word("include" | "external"), Token::Literal(href)] = pair {
            hrefs.push((*href).to_owned());
        }
    }
    hrefs
}

/// Resolves a relative href against a store-relative file path, refusing
/// anything that would leave the store.
fn join_relative(file: &str, href: &str) -> Option<String> {
    if href.contains(['\\', ':']) {
        return None;
    }
    let mut parts: Vec<&str> = file.split('/').collect();
    parts.pop();
    for component in Path::new(href).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            Component::ParentDir => {
                // Never pop the host directory.
                if parts.len() <= 1 {
                    return None;
                }
                parts.pop();
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let path = parts.join("/");
    crate::store::is_store_path(&path).then_some(path)
}

/// Resolves a relative href (no scheme, no leading `/`) against `base`.
fn join_url(base: &str, href: &str) -> String {
    let (origin, path) = match base.split_once("://") {
        Some((scheme, rest)) => {
            let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
            (format!("{scheme}://{host}"), path)
        }
        None => (String::new(), base),
    };
    let mut segments: Vec<&str> = path.split('/').collect();
    segments.pop(); // the file name
    for part in href.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            part => segments.push(part),
        }
    }
    format!("{origin}/{}", segments.join("/"))
}

/// Today's date (UTC) as YYYY-MM-DD.
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() / 86_400) as i64;
    // Howard Hinnant's days-to-civil algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_includes_stay_inside_the_store() {
        assert_eq!(
            join_relative("host/Schema/ParlaMint-TEI.rng", "ParlaMint.rng").as_deref(),
            Some("host/Schema/ParlaMint.rng")
        );
        assert_eq!(
            join_relative("host/a/b/main.rng", "../lib/x.rng").as_deref(),
            Some("host/a/lib/x.rng")
        );
        assert_eq!(join_relative("host/main.rng", "../../etc/passwd"), None);
        assert_eq!(join_relative("host/main.rng", "/etc/passwd"), None);
        assert_eq!(join_relative("host/main.rng", "..\\..\\x.rng"), None);
        assert_eq!(join_relative("host/main.rng", "C:/x.rng"), None);
    }

    #[test]
    fn urls_join_like_browsers_do_for_simple_paths() {
        assert_eq!(
            join_url("https://example.org/schema/main.rng", "../lib/x.rng"),
            "https://example.org/lib/x.rng"
        );
        assert_eq!(
            join_url("https://example.org/schema/main.rng", "x.rng"),
            "https://example.org/schema/x.rng"
        );
    }

    #[test]
    fn finds_includes_in_both_syntaxes() {
        let rng = br#"<grammar xmlns="http://relaxng.org/ns/structure/1.0">
            <include href="a.rng"/><externalRef href="https://x.org/abs.rng"/><start><externalRef href="b.rng"/></start></grammar>"#;
        assert_eq!(schema_references(rng), ["a.rng", "b.rng"]);
        let rnc = br#"include "common.rnc" start = external "part.rnc""#;
        assert_eq!(schema_references(rnc), ["common.rnc", "part.rnc"]);
    }

    #[test]
    fn ignores_the_keywords_in_comments_and_literals() {
        let rnc = br#"# include "comment.rnc"
            ## documentation that may include "quoted.rnc" text
            start = element p { attribute rend { "include" }, text }
            include 'real.rnc'
            x = external """triple.rnc""""#;
        assert_eq!(schema_references(rnc), ["real.rnc", "triple.rnc"]);
    }

    #[test]
    fn formats_dates() {
        assert_eq!(today().len(), 10);
    }
}
