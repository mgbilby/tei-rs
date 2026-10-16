# Provenance

This module (`teixml::relaxng`) is derived from [`relax-ng`](https://crates.io/crates/relax-ng)
0.2.0 by CASOON, taken from <https://github.com/casoon/relax-ng> at commit
`b3516023d12f2e0287a5d6e4f679afddf2b9ad6a` (2026-08-28). That repository was
archived on 2026-09-24; the crate's development moved to
<https://github.com/casoon/barrierlab> (`crates/relax-ng`), whose copy was
identical to 0.2.0 when this was vendored.

The original code is MIT-licensed, Copyright (c) 2026 CASOON — see
[`LICENSE`](LICENSE). Its README is kept as
[`UPSTREAM-README.md`](UPSTREAM-README.md), and the vendored RELAX NG test
suite keeps its BSD-3-Clause notice (see
[`THIRD-PARTY-NOTICES.md`](../../THIRD-PARTY-NOTICES.md)).

Changes made in teixml (all MIT-licensed, like the original):

- Moved into the `teixml` crate as the `relaxng` module: `src/` became
  `src/relaxng/` (`lib.rs` → `mod.rs`, `crate::` paths → `crate::relaxng::`),
  `tests/spectest.rs` became `tests/relaxng_spec.rs`, the conformance suite
  stays at `tests/corpus/relaxng/`, and `examples/validate.rs` became
  `examples/relaxng_engine.rs`.
- `simplify::choice` now flattens nested choices and removes duplicate
  alternatives. Without this, an element reachable through more than one
  `define` (common in TEI) made the working pattern grow with every sibling,
  and large TEI documents took minutes or never finished. Regression test:
  `tests/regression_duplicate_choice.rs`.
- Whitespace-only text is handled as in James Clark's derivative algorithm
  (RELAX NG §6.2.7, as Jing does): dropped when it is one of several
  children, and matched-or-ignored when it is an element's only child.
  Previously the engine committed to whichever branch accepted the
  whitespace, so ParlaMint's `choice(group(orgName+, ref*), text)` rejected
  indented `<orgName>` children. Regression test:
  `tests/regression_whitespace_choice.rs`.
- XSD `pattern` facets are compiled once per process and cached
  (`datatypes/xsd.rs`), instead of on every value checked. Schemas with many
  facets, like papyri.info's, spent most of their validation time
  recompiling regexes. Each cached regex is shared behind an `Arc` rather
  than cloned, because a cloned `regex::Regex` starts with an empty pool of
  match caches and rebuilds them on its first match: with TEI's
  `teidata.word` (`[^\p{C}\p{Z}]+`), the Patrologia Latina sample (16,000
  `rend` attributes) spent two thirds of its instructions doing that.
- `Context` (base URI, namespace bindings, inherited `datatypeLibrary`/`ns`)
  is shared behind an `Arc` with copy-on-write (`grammar.rs`), so cloning
  patterns during validation no longer copies namespace maps.
- The validator derives over hash-consed patterns (`interned.rs`) instead
  of owned `Pattern` trees: each distinct node is stored once and named by
  a small id, `ref`s are replaced by their definitions when interned, and
  derivatives by a child element (and by text, where the text can't
  matter), nullability, element-content lookups and attribute-name checks
  are cached per pattern for the whole document. Attribute derivatives
  skip every branch that has no attribute of that name. Results are
  unchanged; on large TEI documents against `tei_all` validation went from
  seconds to well under a second, since each derivative was rebuilding
  and cloning large parts of the schema.
- The compact-syntax parser skips follow annotations (`pattern >> s:ns [
  ... ]`, RNC Annex C.1), which CBETA's schema uses to attach Schematron
  rules; they were a parse error.
- A prefixed `name` (attribute or `<name>` element) now takes its namespace
  from the prefix even when an `ns` attribute is also present, as §4.10
  requires; the prefix used to be kept in the local name, so Syriaca.org's
  compiled schemas were rejected as invalid.
- Diagnostics name elements and attributes by their local name, as Jing
  does (`div`, not `{http://www.tei-c.org/ns/1.0}div`), and `xml:` for
  attributes in the XML namespace. `ExpandedName`'s own `Display` is
  unchanged.
- "Missing required content" is reported where the element ends
  (`Element::end_location`, defaulting to `location`), which is where it is
  detected and where Jing reports it, so diagnostics come out in document
  order.
- An element's derivative by one of its attributes is also cached for the
  document, by attribute name and value, unless some datatype reachable in
  the content depends on the namespace bindings in scope. For that,
  `DatatypeLibrary` has a new method, `is_context_dependent` (true by
  default; false for the built-in library, and for every XSD type except
  `QName` and `NOTATION`), after Jing's `Datatype.isContextDependent`.
  Attribute values repeat a lot in TEI (`rend="italic"`), and each
  derivative rebuilt the element's attribute group and re-ran the value's
  datatype check. Regression test:
  `repeated_attribute_values_are_checked_in_their_own_context` in
  `validator.rs`.
- The §7.2–§7.4 checks on `group`/`interleave` operands no longer walk
  each operand's whole closure of definitions: what each definition
  contains (attributes, elements, text, data) is summarized once, which
  settles §7.2 and the `text` rule of §7.4, and attribute and element names
  are only collected for operands that share that kind of name with
  another, skipping `ref`s that can't contribute any and visiting each
  definition once. Errors are unchanged. Compiling `tei_all` took 337
  million instructions, 194 million of them in these checks; now 200 and
  56. Unit test: `group_restrictions_see_through_shared_definitions` in
  `simplify.rs`.
- Less work per document element: its namespace bindings are collected
  only when a value is actually checked against a datatype, its children
  are walked without first being collected into a `Vec`, and its name is
  built once and handed to its own validation instead of being asked for
  again.
- Schemas nested more than 512 levels deep (elements in the XML syntax,
  brackets in the compact syntax) are rejected with a parse error before
  parsing, since parsing and simplification recurse once per level and
  deeper input could overflow the stack.
- Comments that pointed to the upstream project's `plan/` notes, which
  aren't part of this repository, say so instead.
- Problems with an element or its attributes are located just past its
  start tag's `>`, and text that isn't allowed where the text ends (at the
  next child's start tag, or the end tag), which is where Jing reports them
  (`Element::start_location` and `Element::end_tag_location`, both optional).
- ID, IDREF and IDREFS checking from the RELAX NG DTD Compatibility
  specification (`idtypes.rs`), as Jing does it: compiling a schema works
  out which attributes have an ID-type (from the DTD Compatibility or XSD
  datatypes `ID`, `IDREF`, `IDREFS`) and rejects schemas that give them
  inconsistently, with Jing's messages; `Schema::id_types` exposes the
  result for checking documents. New `ValidationErrorKind` variants
  `DuplicateId`, `FirstIdOccurrence` and `UnresolvedIdref`.
- The DTD Compatibility datatype library
  (`http://relaxng.org/ns/compatibility/datatypes/1.0`) is registered by
  default (`datatypes/compatibility.rs`); schemas using it were rejected.

- `ValidationError::path` is an `ElementPath`: names shared between errors
  through reference-counted links, built only for elements that have an
  error, instead of a fresh `Vec` of owned names per error.
- An attribute name that only one `attribute` pattern in the content can
  match has its derivative cached once for every value that pattern
  accepts, so attributes with unique values (`xml:id`) no longer each
  rederive the whole attribute group.
- Documentation only: runnable examples for `SchemaResolver`,
  `DatatypeLibrary`, `DatatypeRegistry`, `IdTypes`, `ElementPath` and
  `ExpandedName`.
