#!/usr/bin/env bash
# Records what the reference ISO Schematron implementation reports for every
# schema/document pair, into tests/tei/schematron-expected.tsv; the Rust test
# tests/schematron_agreement.rs checks tei-rs against it.
#
# The reference pipeline is the one TEI uses: the Schematron rules embedded in
# the .rng are extracted (scripts/schematron/extract.xsl), compiled to XSLT 2.0
# with SchXslt and run with Saxon-HE. Pairs: tei_all against every document in
# tests/tei/docs/ and tests/tei/bench/, plus every pair in
# tests/tei/expected-declared.tsv (each document against its own schema from
# schemas/, where that schema embeds any rules), plus the made-up documents in
# tests/tei/schematron/ (written to break as many rules as possible) against
# every schema in tests/tei/schemas/ and schemas/.
#
# Usage:
#   SAXON_CP=saxon-he.jar:xmlresolver.jar SCHXSLT_JAR=schxslt.jar \
#     scripts/record-schematron-expectations.sh
#
# Both come from Maven Central, e.g.
#   mvn dependency:get -Dartifact=net.sf.saxon:Saxon-HE:12.5
#   mvn dependency:get -Dartifact=name.dmaus.schxslt:schxslt:1.10.1
#   m2=~/.m2/repository
#   SAXON_CP=$m2/net/sf/saxon/Saxon-HE/12.5/Saxon-HE-12.5.jar:$m2/org/xmlresolver/xmlresolver/5.2.2/xmlresolver-5.2.2.jar
#   SCHXSLT_JAR=$m2/name/dmaus/schxslt/schxslt/1.10.1/schxslt-1.10.1.jar
#
# Output columns: schema, document, kind, role, location, context, test. kind is
# `assert` (a failed assert) or `report` (a successful report), one line per
# message; a document with no messages has one line of kind `none`, and one
# Saxon can't parse has kind `not-well-formed`. location is SchXslt's XPath
# for the context node (/Q{uri}local[n]/.../@Q{uri}local); context is the
# rule's context and test the assert/report test, whitespace normalised.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${SAXON_CP:?set SAXON_CP to the Saxon-HE classpath (Saxon-HE and xmlresolver jars)}"
: "${SCHXSLT_JAR:?set SCHXSLT_JAR to the path of schxslt.jar}"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(cd "$work" && unzip -q "$SCHXSLT_JAR" 'xslt/*')
saxon() { java -cp "$SAXON_CP" net.sf.saxon.Transform "$@"; }

# compiled SCHEMA -> path of the compiled validation XSLT (cached per schema),
# or nothing when the schema embeds no rules.
compiled() {
  local key
  key=$(printf '%s' "$1" | sha256sum | cut -c1-16)
  if [[ ! -e "$work/$key.done" ]]; then
    saxon -xsl:scripts/schematron/extract.xsl -s:"$1" -o:"$work/$key.sch"
    if grep -q 'pattern' "$work/$key.sch"; then
      saxon -xsl:"$work/xslt/2.0/compile-for-svrl.xsl" -s:"$work/$key.sch" -o:"$work/$key.xsl"
    fi
    touch "$work/$key.done"
  fi
  [[ -e "$work/$key.xsl" ]] && echo "$work/$key.xsl"
}

record() { # record SCHEMA DOC (schema relative to the repository root, doc to tests/tei)
  local schema=$1 doc=$2 xsl lines
  xsl=$(compiled "$schema") || return 0
  local source="tests/tei/$doc" dtd
  # Some TEI test files name a DTD that isn't shipped with them. Like the
  # RELAX NG validators, read them without it (from a copy without the
  # external identifier, which keeps every element and attribute).
  dtd=$(head -c 2000 "$source" | sed -nE 's/.*<!DOCTYPE [^ >]+ SYSTEM "([^"]*)".*/\1/p' | head -1)
  if [[ -n $dtd && ! -e $(dirname "$source")/$dtd ]]; then
    sed -E '1,20s/(<!DOCTYPE [^ >]+) SYSTEM "[^"]*"/\1/' "$source" >"$work/nodtd.xml"
    source=$work/nodtd.xml
  fi
  if ! saxon -xsl:"$xsl" -s:"$source" -o:"$work/out.svrl" 2>"$work/err"; then
    if grep -qi 'SXXP0003\|not well-formed\|XML parser' "$work/err"; then
      printf '%s\t%s\tnot-well-formed\t-\t-\t-\t-\n' "$schema" "$doc"
      return
    fi
    cat "$work/err" >&2
    return 1
  fi
  lines=$(saxon -xsl:scripts/schematron/svrl-to-tsv.xsl -s:"$work/out.svrl" | LC_ALL=C sort)
  if [[ -z "$lines" ]]; then
    printf '%s\t%s\tnone\t-\t-\t-\t-\n' "$schema" "$doc"
  else
    while IFS= read -r line; do printf '%s\t%s\t%s\n' "$schema" "$doc" "$line"; done <<<"$lines"
  fi
}

out=tests/tei/schematron-expected.tsv
{
  printf '# schema (relative to the repository root)\tdocument\tkind\trole\tlocation\tcontext\ttest\n'
  while IFS= read -r doc; do
    record tests/tei/schemas/tei_all-2.7.0.rng "${doc#tests/tei/}"
  done < <(find tests/tei/docs tests/tei/bench -name '*.xml' | LC_ALL=C sort)
  while IFS=$'\t' read -r schema doc _; do
    [[ $schema == \#* || ! -e $schema || ! -e tests/tei/$doc ]] && continue
    record "$schema" "$doc"
  done < tests/tei/expected-declared.tsv
  while IFS= read -r doc; do
    while IFS= read -r schema; do
      record "$schema" "${doc#tests/tei/}"
    done < <(find tests/tei/schemas schemas -name '*.rng' | LC_ALL=C sort)
  done < <(find tests/tei/schematron -name '*.xml' | LC_ALL=C sort)
} > "$out.tmp" 2> >(grep -v '^Picked up JAVA_TOOL_OPTIONS' >&2)
mv "$out.tmp" "$out"
echo "wrote $(grep -vc '^#' "$out") lines to $out"
