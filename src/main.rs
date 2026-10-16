//! `tei-rs` command-line tool.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use teixml::{Schema, SchemaStore};

const USAGE: &str = "\
Usage:
  tei-rs validate [--schema FILE] [--schemas DIR] [--no-schematron] <DOCUMENT.xml>...
  tei-rs schemas list  [--schemas DIR]
  tei-rs schemas check [--schemas DIR] [FILTER]
  tei-rs schemas sync  [--schemas DIR] [FILTER]
  tei-rs schemas add   [--schemas DIR] <URL> [--alias HREF]... [--mirror URL]...
  tei-rs schemas which [--schemas DIR] <DOCUMENT.xml>...

validate   Validates documents against a RELAX NG schema (.rng or .rnc) and
           the Schematron rules embedded in it, and prints one line per
           problem as `file:line:column: severity: message`. Schematron
           messages start with `Schematron assert failed:` or `Schematron
           report:`; their severity (error, warning or info) comes from the
           rule's role, and only errors make a document invalid. Schematron
           tests that use XPath tei-rs doesn't support are listed on stderr
           and skipped. Without --schema, each document is validated against
           the schema it declares in <?xml-model?>, taken from the local
           schema store, so no network access is needed. Each schema is
           compiled once per run.
  --no-schematron  Check RELAX NG only.

schemas    Manages the local schema store.
  list     Lists stored schemas and whether each has been downloaded.
  check    Compares each stored schema (and the files it includes) with the
           version published at its URL, without changing anything.
  sync     Like check, but downloads every outdated or missing schema.
  add      Downloads the schema at URL into the store. --alias records another
           href documents use for it; --mirror adds a fallback download URL.
  which    Prints the schema file each document would be validated against.
  FILTER   Only schemas whose URL contains this text.

--schemas DIR  The schema store (default: $TEI_RS_SCHEMAS, else a tei-rs
               folder in your user cache directory).

Exit status: 0 when everything is valid / current, 1 when a document is invalid
or a schema is outdated or missing, 2 on usage errors, unreadable input or
unreachable schema sources.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Command-line options shared by every command.
#[derive(Default)]
struct Options {
    schema: Option<PathBuf>,
    store: Option<PathBuf>,
    aliases: Vec<String>,
    mirrors: Vec<String>,
    no_schematron: bool,
    positional: Vec<String>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg.as_str() {
            "-s" | "--schema" => options.schema = Some(value("--schema")?.into()),
            "--schemas" => options.store = Some(value("--schemas")?.into()),
            "--alias" => options.aliases.push(value("--alias")?),
            "--mirror" => options.mirrors.push(value("--mirror")?),
            "--no-schematron" => options.no_schematron = true,
            flag if flag.starts_with('-') => return Err(format!("unknown option `{flag}`")),
            positional => options.positional.push(positional.to_owned()),
        }
    }
    Ok(options)
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    if args.is_empty()
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        println!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    }
    let (command, rest) = args.split_first().expect("not empty");
    match command.as_str() {
        "validate" => {
            let options = parse_options(rest)?;
            if options.positional.is_empty() {
                return Err("no documents given".into());
            }
            Ok(validate(&options))
        }
        "schemas" => {
            let Some((subcommand, rest)) = rest.split_first() else {
                return Err("`schemas` needs a subcommand: list, check, sync or add".into());
            };
            let options = parse_options(rest)?;
            let dir = options
                .store
                .clone()
                .unwrap_or_else(SchemaStore::default_dir);
            let mut store = SchemaStore::open(&dir).map_err(|error| error.to_string())?;
            match subcommand.as_str() {
                "list" => Ok(list(&store)),
                "which" => Ok(which(&store, &options.positional)),
                "check" | "sync" | "add" => sync_command(subcommand, &mut store, &options),
                other => Err(format!("unknown `schemas` subcommand `{other}`")),
            }
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn validate(options: &Options) -> ExitCode {
    let store = if options.schema.is_none() {
        let dir = options
            .store
            .clone()
            .unwrap_or_else(SchemaStore::default_dir);
        match SchemaStore::open(&dir) {
            Ok(store) => Some(store),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };

    let mut compiled: HashMap<PathBuf, Result<Schema, String>> = HashMap::new();
    let mut status = 0;
    for document in &options.positional {
        let document = Path::new(document);
        let xml = match std::fs::read_to_string(document) {
            Ok(xml) => xml,
            Err(error) => {
                eprintln!("{}: {error}", document.display());
                status = 2;
                continue;
            }
        };
        let schema_path = match (&options.schema, &store) {
            (Some(path), _) => path.clone(),
            (None, Some(store)) => {
                match teixml::store::schema_for_document(&xml, document, store) {
                    Ok(path) => path,
                    Err(error) => {
                        eprintln!("{}: {error}", document.display());
                        status = 2;
                        continue;
                    }
                }
            }
            (None, None) => unreachable!("a store is opened whenever --schema is absent"),
        };
        let schema = compiled
            .entry(schema_path.clone())
            .or_insert_with(|| load_schema(&schema_path, !options.no_schematron));
        let schema = match schema {
            Ok(schema) => schema,
            Err(error) => {
                eprintln!("{}: {}: {error}", document.display(), schema_path.display());
                status = 2;
                continue;
            }
        };
        match schema.validate_str_from(&xml, document) {
            Ok(diagnostics) => {
                // Buffered: a document can have tens of thousands of errors.
                let mut out = std::io::BufWriter::new(std::io::stdout().lock());
                for diagnostic in &diagnostics {
                    let _ = write!(out, "{}:", document.display());
                    if let Some(position) = diagnostic.position {
                        let _ = write!(out, "{position}:");
                    }
                    let _ = write!(out, " {}: {}", diagnostic.severity(), diagnostic.message());
                    if !diagnostic.path.is_empty() {
                        let _ = write!(out, " (in {})", diagnostic.path);
                    }
                    let _ = writeln!(out);
                }
                let _ = out.flush();
                if diagnostics.iter().any(|diagnostic| diagnostic.is_error()) {
                    status = status.max(1);
                }
            }
            Err(error) => {
                eprintln!("{}: {error}", document.display());
                status = 2;
            }
        }
    }
    ExitCode::from(status)
}

/// Compiles a schema and, when Schematron checking is on, its embedded
/// rules, warning on stderr about any tests that can't be checked.
fn load_schema(path: &Path, schematron: bool) -> Result<Schema, String> {
    #[cfg_attr(not(feature = "schematron"), allow(unused_mut))]
    let mut schema = Schema::from_file(path).map_err(|error| error.to_string())?;
    #[cfg(feature = "schematron")]
    {
        schema.set_schematron(schematron);
        if schematron {
            let rules = schema.schematron().map_err(|error| error.to_string())?;
            if !rules.unsupported().is_empty() {
                eprintln!(
                    "warning: {}: {} of {} Schematron tests can't be checked by tei-rs and are skipped:",
                    path.display(),
                    rules.unsupported_test_count(),
                    rules.test_count()
                );
                for entry in rules.unsupported() {
                    eprintln!("  {entry}");
                }
            }
        }
    }
    #[cfg(not(feature = "schematron"))]
    let _ = schematron;
    Ok(schema)
}

fn list(store: &SchemaStore) -> ExitCode {
    println!("schema store: {}", store.dir().display());
    if store.entries().is_empty() {
        println!("(empty — add schemas with `tei-rs schemas add <URL>`)");
    }
    for entry in store.entries() {
        let state = match &entry.file {
            Some(file) if store.dir().join(file).is_file() => format!(
                "stored, checked {}",
                entry.checked.as_deref().unwrap_or("never")
            ),
            _ => "not downloaded".to_owned(),
        };
        println!("{}  [{state}]", entry.url);
        for alias in &entry.aliases {
            println!("    alias  {alias}");
        }
        if let Some(source) = entry
            .fetched_from
            .as_ref()
            .filter(|source| **source != entry.url)
        {
            println!("    from   {source}");
        }
    }
    ExitCode::SUCCESS
}

fn which(store: &SchemaStore, documents: &[String]) -> ExitCode {
    let mut status = 0;
    for document in documents {
        let document = Path::new(document);
        let resolved = std::fs::read_to_string(document)
            .map_err(|error| error.to_string())
            .and_then(|xml| {
                teixml::store::schema_for_document(&xml, document, store)
                    .map_err(|error| error.to_string())
            });
        match resolved {
            Ok(schema) if documents.len() == 1 => println!("{}", schema.display()),
            Ok(schema) => println!("{}\t{}", document.display(), schema.display()),
            Err(error) => {
                eprintln!("{}: {error}", document.display());
                status = 2;
            }
        }
    }
    ExitCode::from(status)
}

#[cfg(feature = "sync")]
fn sync_command(
    subcommand: &str,
    store: &mut SchemaStore,
    options: &Options,
) -> Result<ExitCode, String> {
    use teixml::sync::Status;

    let reports = if subcommand == "add" {
        let [url] = options.positional.as_slice() else {
            return Err("`schemas add` needs exactly one URL".into());
        };
        vec![
            teixml::sync::add(store, url, &options.aliases, &options.mirrors)
                .map_err(|error| error.to_string())?,
        ]
    } else {
        if options.positional.len() > 1 {
            return Err(format!("`schemas {subcommand}` takes at most one FILTER"));
        }
        let filter = options.positional.first().map(String::as_str);
        teixml::sync::sync(store, filter, subcommand == "sync")
            .map_err(|error| error.to_string())?
    };

    let mut status = 0;
    for report in &reports {
        let (label, code) = match &report.status {
            Status::Current => ("current", 0),
            Status::Updated => ("updated", 0),
            Status::Outdated => ("OUTDATED", 1),
            Status::Missing => ("MISSING", 1),
            Status::Unreachable(_) => ("UNREACHABLE", 2),
        };
        status = status.max(code);
        let via = report
            .source
            .as_ref()
            .filter(|source| **source != report.url)
            .map(|source| format!(" (via {source})"))
            .unwrap_or_default();
        println!("{label:<12} {}{via}", report.url);
        if let Status::Unreachable(error) = &report.status {
            println!("             {error}");
        }
    }
    if reports.is_empty() {
        println!("no schemas matched in {}", store.dir().display());
    }
    Ok(ExitCode::from(status))
}

#[cfg(not(feature = "sync"))]
fn sync_command(_: &str, _: &mut SchemaStore, _: &Options) -> Result<ExitCode, String> {
    eprintln!(
        "error: schema downloads need the optional `sync` feature; \
         install with `cargo install teixml --features sync`"
    );
    Ok(ExitCode::from(2))
}
