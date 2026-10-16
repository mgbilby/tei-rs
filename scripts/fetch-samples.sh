#!/usr/bin/env bash
# Downloads the TEI sample documents and schema used by the tests and
# benchmarks into tests/tei/, from the exact upstream commits (or, for sources
# not on GitHub, the URLs) listed in tests/tei/samples.tsv, and verifies each
# file's SHA-256. Files already present with the right hash are skipped.
#
# Usage: scripts/fetch-samples.sh [FILTER]
#   FILTER  only paths containing this text (e.g. `papyri`, `bench/sarit`)
# Set TEI_SAMPLES_DIR to download somewhere other than tests/tei.
set -euo pipefail
cd "$(dirname "$0")/.."
filter=${1:-}
dest=${TEI_SAMPLES_DIR:-tests/tei}

if command -v sha256sum >/dev/null; then
  sha256() { sha256sum "$1" | cut -d' ' -f1; }
else
  sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

fetched=0 present=0 failed=0
while IFS=$'\t' read -r path _project repository commit upstream _licence hash _bytes; do
  [[ $path == \#* || -z $path ]] && continue
  [[ -n $filter && $path != *"$filter"* ]] && continue
  # Only plain relative paths, so the table can't write outside $dest.
  if [[ $path == /* || /$path/ == */../* || /$path/ == *//* || $path == *\\* ]]; then
    echo "skipped: unsafe path $path" >&2
    failed=$((failed + 1))
    continue
  fi
  target=$dest/$path
  if [[ -f $target && $(sha256 "$target") == "$hash" ]]; then
    present=$((present + 1))
    continue
  fi
  mkdir -p "$(dirname "$target")"
  if [[ $upstream == http://* || $upstream == https://* ]]; then
    url=$upstream
  else
    url="https://raw.githubusercontent.com/$repository/$commit/$upstream"
  fi
  if curl -fsSL --retry 3 -o "$target.part" "$url" && [[ $(sha256 "$target.part") == "$hash" ]]; then
    mv "$target.part" "$target"
    fetched=$((fetched + 1))
  else
    echo "failed: $path ($url)" >&2
    failed=$((failed + 1))
  fi
done < tests/tei/samples.tsv

echo "$fetched downloaded, $present already present, $failed failed"
[[ $failed -eq 0 ]]
