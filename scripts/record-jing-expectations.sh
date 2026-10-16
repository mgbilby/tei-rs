#!/usr/bin/env bash
# Regenerates Jing's verdict for every schema/document pair: tests/tei/docs/ ->
# tests/tei/expected.tsv and tests/tei/bench/ -> tests/tei/expected-bench.tsv,
# both against tests/tei/schemas/*.rng; and every document against the schema
# it declares itself, taken from the schema store in schemas/ ->
# tests/tei/expected-declared.tsv (documents whose schema isn't stored are
# skipped; run `tei-rs schemas sync --schemas schemas` first to include them).
# The Rust tests in tests/jing_agreement.rs check tei-rs against them.
#
# Usage: JING_JAR=/path/to/jing.jar scripts/record-jing-expectations.sh
#
# A copy of jing.jar ships with the TEI sources:
#   git clone --depth 1 --filter=blob:none --sparse https://github.com/TEIC/TEI
#   git -C TEI sparse-checkout set P5/Utilities/lib   # -> TEI/P5/Utilities/lib/jing.jar
set -euo pipefail
cd "$(dirname "$0")/.."
: "${JING_JAR:?set JING_JAR to the path of jing.jar}"
record() { # record DOC_DIR OUT
local dir=$1 out=$2
{
  printf '# schema\tdocument\tverdict\tjing_error_count\tjing_first_error_line\n'
  for schema in tests/tei/schemas/*.rng; do
    while IFS= read -r doc; do
      log=$(java -jar "$JING_JAR" "$schema" "$doc" 2>&1 | grep -E ':[0-9]+:[0-9]+: (error|fatal)' || true)
      if grep -q ': fatal' <<<"$log"; then
        printf '%s\t%s\tnot-well-formed\t-\t-\n' "${schema#tests/tei/}" "${doc#tests/tei/}"
      elif [[ -z "$log" ]]; then
        printf '%s\t%s\tvalid\t0\t-\n' "${schema#tests/tei/}" "${doc#tests/tei/}"
      else
        count=$(grep -c . <<<"$log")
        first=$(head -1 <<<"$log" | sed -E 's/^.*:([0-9]+):[0-9]+: .*$/\1/')
        printf '%s\t%s\tinvalid\t%s\t%s\n' "${schema#tests/tei/}" "${doc#tests/tei/}" "$count" "$first"
      fi
    done < <(find "$dir" -name '*.xml' | sort)
  done
} > "$out"
echo "wrote $(grep -vc '^#' "$out") expectations to $out"
}
verdict() { # verdict SCHEMA DOC -> "verdict<TAB>count<TAB>first line"
  local log compact=()
  [[ $1 == *.rnc ]] && compact=(-c)
  log=$(java -jar "$JING_JAR" "${compact[@]}" "$1" "$2" 2>&1 | grep -E ':[0-9]+:[0-9]+: (error|fatal)' || true)
  if grep -q ': fatal' <<<"$log"; then
    printf 'not-well-formed\t-\t-'
  elif [[ -z "$log" ]]; then
    printf 'valid\t0\t-'
  else
    printf 'invalid\t%s\t%s' "$(grep -c . <<<"$log")" "$(head -1 <<<"$log" | sed -E 's/^.*:([0-9]+):[0-9]+: .*$/\1/')"
  fi
}

record tests/tei/docs tests/tei/expected.tsv
record tests/tei/bench tests/tei/expected-bench.tsv

cargo build -q --release
out=tests/tei/expected-declared.tsv
{
  printf '# schema (relative to the repository root)\tdocument\tverdict\tjing_error_count\tjing_first_error_line\n'
  while IFS= read -r doc; do
    schema=$(target/release/tei-rs schemas which --schemas schemas "$doc" 2>/dev/null) || continue
    printf '%s\t%s\t%s\n' "$schema" "${doc#tests/tei/}" "$(verdict "$schema" "$doc")"
  done < <(find tests/tei/docs tests/tei/bench -name '*.xml' | sort)
  # Samples that don't declare their schema, paired by hand.
  pm=schemas/raw.githubusercontent.com/clarin-eric/ParlaMint/main/Schema
  for pair in "$pm/ParlaMint-TEI.rng tests/tei/bench/parlamint/ParlaMint-GB_2017-09-07-commons.xml" \
              "$pm/ParlaMint-TEI.ana.rng tests/tei/bench/parlamint/ParlaMint-GB_2017-09-07-commons.ana.xml"; do
    read -r schema doc <<<"$pair"
    printf '%s\t%s\t%s\n' "$schema" "${doc#tests/tei/}" "$(verdict "$schema" "$doc")"
  done
  # FreeDict dictionaries and correspSearch CMIF files don't declare a schema
  # either; pair them with the format's own.
  gh=schemas/raw.githubusercontent.com
  for doc in tests/tei/bench/freedict/*.xml; do
    schema=$gh/freedict/fd-dictionaries/master/shared/freedict-P5.rng
    printf '%s\t%s\t%s\n' "$schema" "${doc#tests/tei/}" "$(verdict "$schema" "$doc")"
  done
  for doc in tests/tei/bench/cmif/*.xml; do
    schema=$gh/TEI-Correspondence-SIG/CMIF/master/schema/cmi-customization.rng
    printf '%s\t%s\t%s\n' "$schema" "${doc#tests/tei/}" "$(verdict "$schema" "$doc")"
  done
  # FRUS volumes don't declare one either; the project's schema is frus.rnc.
  for doc in tests/tei/bench/frus/*.xml; do
    schema=$gh/HistoryAtState/frus/master/schema/frus.rnc
    printf '%s\t%s\t%s\n' "$schema" "${doc#tests/tei/}" "$(verdict "$schema" "$doc")"
  done
} > "$out"
echo "wrote $(grep -vc '^#' "$out") expectations to $out"
