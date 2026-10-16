//! Local validation benchmark over the sample documents bundled in
//! `tests/tei/bench/` and `tests/tei/docs/`. No network access or extra
//! tools are needed.
//!
//! ```text
//! cargo bench                      # all samples, 5 runs each
//! TEI_BENCH_RUNS=10 cargo bench    # more runs
//! cargo bench -- kraus             # only files whose path contains "kraus"
//! TEI_BENCH_SCHEMAS=declared cargo bench
//! TEI_BENCH_SCHEMATRON=1 cargo bench  # also check embedded Schematron rules
//! ```
//!
//! By default every document is validated against TEI P5 `tei_all`
//! (`tests/tei/schemas/`). With `TEI_BENCH_SCHEMAS=declared`, each document
//! is validated against the schema it declares, from the schema store in
//! `schemas/`; documents whose schema isn't stored are skipped.
//!
//! Validation is RELAX NG only, as Jing's is, unless `TEI_BENCH_SCHEMATRON=1`
//! (with the `schematron` feature) adds the schemas' Schematron rules.
//!
//! Reports schema compile times, then per document the fastest and median
//! validation time (parsing included) and throughput.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn main() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let fixtures = repository.join("tests/tei");
    let runs: usize = std::env::var("TEI_BENCH_RUNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&runs| runs > 0)
        .unwrap_or(5);
    // `cargo bench` passes `--bench`; anything else is a path filter.
    let filter: Option<String> = std::env::args().skip(1).find(|arg| !arg.starts_with("--"));

    let declared = std::env::var("TEI_BENCH_SCHEMAS").is_ok_and(|mode| mode == "declared");
    let store = declared.then(|| {
        teixml::SchemaStore::open(repository.join("schemas")).expect("schema store opens")
    });
    let default_schema = fixtures.join("schemas/tei_all-2.7.0.rng");
    // Each schema is compiled once and reused for every document.
    let mut schemas: HashMap<PathBuf, teixml::Schema> = HashMap::new();
    let schema_for = |xml: &str, document: &Path| -> Option<PathBuf> {
        match &store {
            Some(store) => teixml::store::schema_for_document(xml, document, store).ok(),
            None => Some(default_schema.clone()),
        }
    };

    let mut documents: Vec<PathBuf> = ["bench", "docs"]
        .iter()
        .flat_map(|dir| xml_files(&fixtures.join(dir)))
        .filter(|path| {
            filter
                .as_deref()
                .is_none_or(|filter| path.to_string_lossy().contains(filter))
        })
        .collect();
    documents.sort_by_key(|path| std::cmp::Reverse(file_size(path)));
    let fetch_hint = "run scripts/fetch-samples.sh to download the sample documents";
    if documents.is_empty() {
        println!("no sample documents under tests/tei/; {fetch_hint}");
        return;
    }
    if store.is_none() && !default_schema.is_file() {
        println!("{} is missing; {fetch_hint}", default_schema.display());
        return;
    }

    println!(
        "\n{:<52} {:>9} {:>10} {:>10} {:>8}  verdict",
        "document", "size", "fastest", "median", "MB/s"
    );
    let mut total = Duration::ZERO;
    let mut measured = 0;
    let mut skipped = 0;
    for path in &documents {
        let xml = std::fs::read_to_string(path).expect("readable sample");
        let Some(schema_path) = schema_for(&xml, path) else {
            skipped += 1;
            continue;
        };
        let schema = compiled(&mut schemas, &schema_path, &repository);
        measured += 1;
        let mut times = Vec::with_capacity(runs);
        let mut verdict = String::new();
        for _ in 0..runs {
            let started = Instant::now();
            let outcome = schema.validate_str(&xml);
            times.push(started.elapsed());
            verdict = match outcome {
                Ok(diagnostics) if !diagnostics.iter().any(teixml::Diagnostic::is_error) => {
                    "valid".into()
                }
                Ok(diagnostics) => format!(
                    "invalid ({} errors)",
                    diagnostics.iter().filter(|d| d.is_error()).count()
                ),
                Err(error) => format!("error: {error}"),
            };
        }
        times.sort();
        let fastest = times[0];
        let median = times[times.len() / 2];
        total += median;
        let name = path
            .strip_prefix(&fixtures)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        println!(
            "{:<52} {:>8}K {:>10} {:>10} {:>8.2}  {verdict}",
            truncate(&name, 52),
            xml.len() / 1024,
            ms(fastest),
            ms(median),
            xml.len() as f64 / 1_048_576.0 / fastest.as_secs_f64(),
        );
    }
    println!(
        "\n{measured} documents, {runs} runs each, sum of medians {}",
        ms(total)
    );
    if skipped > 0 {
        println!(
            "{skipped} documents skipped: their declared schema is not in schemas/ \
             (run `tei-rs schemas sync --schemas schemas`)"
        );
    }
}

/// The compiled schema at `path`, compiling (and reporting the time) on
/// first use.
fn compiled<'a>(
    schemas: &'a mut HashMap<PathBuf, teixml::Schema>,
    path: &Path,
    repository: &Path,
) -> &'a teixml::Schema {
    schemas.entry(path.to_owned()).or_insert_with(|| {
        let started = Instant::now();
        #[cfg_attr(not(feature = "schematron"), allow(unused_mut))]
        let mut schema = teixml::Schema::from_file(path).expect("schema compiles");
        #[cfg(feature = "schematron")]
        {
            let schematron = std::env::var("TEI_BENCH_SCHEMATRON").is_ok_and(|value| value == "1");
            schema.set_schematron(schematron);
            if schematron {
                schema.schematron().expect("Schematron rules compile");
            }
        }
        println!(
            "schema {} compiled in {}",
            path.strip_prefix(repository).unwrap_or(path).display(),
            ms(started.elapsed())
        );
        schema
    })
}

fn xml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(xml_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "xml") {
            files.push(path);
        }
    }
    files
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn ms(duration: Duration) -> String {
    format!("{:.1}ms", duration.as_secs_f64() * 1000.0)
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.to_owned()
    } else {
        let tail: String = text.chars().rev().take(width - 1).collect();
        format!("…{}", tail.chars().rev().collect::<String>())
    }
}
