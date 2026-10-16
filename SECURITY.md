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
read. A schema's `include`/`externalRef` and its Schematron rules'
`doc()`/`doc-available()` read local files (never URLs), and parts of a file
read as a schema can appear in error messages. Only validate against schemas
you trust, such as published TEI and project schemas. Even so, schemas are
bounded too: patterns may nest at most 512 levels, Schematron expressions
256, and a document's Schematron tests may run at most 100 million
`for`/`some`/`every` iterations; `doc-available()` reads only regular files
up to 256 MB.

**Schema downloads** (the optional `sync` feature) use HTTPS only, redirects
included, with the operating system's certificates; responses are limited
to 64 MB and 1,000 files per schema, and everything is written inside the
schema store directory.

A crash, hang, out-of-memory abort or file access outside these rules is a
vulnerability; please report it.
