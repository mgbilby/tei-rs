# Security policy

## Reporting a vulnerability

Please report security problems privately, through GitHub's
[private vulnerability reporting](https://github.com/mgbilby/tei-rs/security/advisories/new)
for this repository, not in a public issue. Include what you ran and the
input that triggers the problem, if you can share it.

Only the latest release is supported with fixes.

## What teixml protects against

**Documents are untrusted input.** Validating any XML document, however
malformed or hostile, should end in a result or an error, never a crash,
a hang or unbounded memory use:

- External entities are never resolved, and entity expansion is bounded
  (both by the `roxmltree` parser).
- Elements may nest at most `MAX_DOCUMENT_DEPTH` (1,024) levels deep; deeper
  documents are rejected before parsing.
- Validation never touches the network.

**Schemas are trusted input**, like code: a schema decides which files are
read. A schema's `include`/`externalRef` read local files (never URLs), and
parts of a file read as a schema can appear in error messages. Only validate
against schemas you trust, such as published TEI and project schemas. Even
so, schemas are bounded too: patterns may nest at most 512 levels,
Schematron expressions 256, and a document's Schematron tests may run at most
100 million `for`/`some`/`every` iterations.

**Documents read by Schematron rules.** Rules can read other XML documents
with `doc()`, `document()` and `doc-available()`, and what they read can
appear in their messages. The URI may come from the validated document
itself (FreeDict's rules read `document(@ana)`), so these reads are limited
whatever the schema says:

- Only files under the schema's directory, the validated document's
  directory and the schema store are read, after resolving symbolic links
  and `..`. Anything else is unavailable: `doc-available()` is false, and
  `doc()`/`document()` raise an error naming the allowed directories.
- `http(s)` URIs are never fetched: they are read from the schema store,
  where `tei-rs schemas add URL` puts them.
- Only regular files up to 256 MB are read, and `doc()`/`document()` read at
  most 1,000 documents and 1 GB in all per validated document, with the same
  nesting and entity limits as the document itself.

**Schema downloads** (the optional `sync` feature) use HTTPS only, redirects
included, with the operating system's certificates; responses are limited
to 64 MB and 1,000 files per schema, and everything is written inside the
schema store directory.

A crash, hang, out-of-memory abort or file access outside these rules is a
vulnerability; please report it.
