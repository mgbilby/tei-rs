#!/usr/bin/env bash
# End-to-end timing of the `tei-rs` CLI against Jing on the bundled samples.
# Both are timed as command-line runs (process start, schema load, parse,
# validate), best of N runs per file. tei-rs runs with --no-schematron, so
# both check RELAX NG only.
#
# Usage: JING_JAR=/path/to/jing.jar scripts/bench-vs-jing.sh [RUNS] [FILTER]
# Without JING_JAR only tei-rs is timed. FILTER keeps paths containing it.
set -euo pipefail
cd "$(dirname "$0")/.."
runs=${1:-3}
filter=${2:-}
schema=tests/tei/schemas/tei_all-2.7.0.rng
cargo build -q --release
bin=target/release/tei-rs

best_ms() { # best_ms CMD... -> fastest wall time in ms over $runs runs
  local best=
  for _ in $(seq "$runs"); do
    local start end
    start=$(date +%s%N)
    "$@" >/dev/null 2>&1 || true
    end=$(date +%s%N)
    local ms=$(( (end - start) / 1000000 ))
    if [[ -z $best || $ms -lt $best ]]; then best=$ms; fi
  done
  echo "$best"
}

printf '%-56s %8s %10s %10s\n' document KiB jing tei-rs
find tests/tei/bench tests/tei/docs -name '*.xml' | grep -F -- "$filter" | sort | while read -r doc; do
  kib=$(( $(wc -c <"$doc") / 1024 ))
  ours=$(best_ms "$bin" validate --no-schematron --schema "$schema" "$doc")
  if [[ -n ${JING_JAR:-} ]]; then
    theirs="$(best_ms java -jar "$JING_JAR" "$schema" "$doc")ms"
  else
    theirs=-
  fi
  printf '%-56s %8s %10s %8sms\n' "${doc#tests/tei/}" "$kib" "$theirs" "$ours"
done
