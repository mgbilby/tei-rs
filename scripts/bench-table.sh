#!/usr/bin/env bash
# Times the tei-rs command-line tool against the reference validators with
# hyperfine on one representative sample per project, prints the two results
# tables as Markdown, and writes them, with the figures quoted around them,
# into BENCHMARKS.md and README.md (scripts/bench-docs.py; review the change
# with `git diff` before committing). The tables are: RELAX NG only (tei-rs
# --no-schematron against Jing), then RELAX NG plus the schemas' embedded
# Schematron rules (tei-rs against Jing followed by the rules compiled to XSLT
# by SchXslt and run on Saxon-HE, TEI's reference pipeline; the rules are
# compiled once per schema beforehand, which isn't timed). Run it on an
# otherwise idle machine.
#
# Usage: JING_JAR=/path/to/jing.jar scripts/bench-table.sh [RUNS] [FILTER]
#   RUNS    timed runs per tool and document, after 1 warm-up run (default 5)
#   FILTER  only rows whose project or document contains this text; the
#           documents are then left unchanged
# Set BENCH_WRITE=0 to only print the tables, and BENCH_MACHINE to describe
# the machine in BENCHMARKS.md (default: its CPU model and operating system).
# Set SCHEMATRON=0 to skip the second table. Saxon-HE and SchXslt are
# downloaded from Maven Central on first use (SHA-256 checked) and cached in
# ${XDG_CACHE_HOME:-~/.cache}/tei-rs-bench.
# Set XMLLINT=1 to add a column for libxml2's xmllint to the first table (one
# run each, capped at XMLLINT_LIMIT seconds, default 120, since it can be very
# slow on tei_all).
#
# Needs hyperfine, Java, python3 and curl; see CONTRIBUTING.md for getting
# jing.jar.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${JING_JAR:?set JING_JAR to the path of jing.jar}"
runs=${1:-5}
filter=${2:-}
limit=${XMLLINT_LIMIT:-120}
for tool in hyperfine java python3 curl; do
  command -v "$tool" >/dev/null || { echo "$tool is not installed" >&2; exit 2; }
done
[[ ${XMLLINT:-} == 1 ]] && { command -v xmllint >/dev/null || { echo "xmllint is not installed" >&2; exit 2; }; }

cargo build -q --release
bin=$PWD/target/release/tei-rs
b=tests/tei/bench
ta=tests/tei/schemas/tei_all-2.7.0.rng
pi=schemas/papyri.info/schemas/papyri.info.rng

# project | document | schema | files (space-separated), in alphabetical order by project
rows=(
  "84000|*The King of Samādhis Sūtra* (Toh 127)|$ta|$b/84000/055-001_toh127_king_of_samadhis.xml"
  "Akademieprotokolle (ÖAW)|Minutes of a general assembly, A_0005|$ta|$b/akademie/A_0005.xml"
  "Arthur Schnitzler|*Schnitzler-Briefe*: index of works|$ta|$b/schnitzler/indices_listbibl.xml"
  "Auden Musulin Papers|Stella Musulin, typescript memoir *The Years in Austria*|$ta|$b/amp/amp-transcript__0062.xml"
  "Beta maṣāḥǝft|Synaxarion manuscript ESam019|$ta|$b/betamasaheft/ESam019.xml"
  "Bodleian medieval manuscripts|Places authority file|$ta|$b/bodleian/places.xml"
  "Bullinger Digital|Thomas Erastus, letter of 23 July 1568|$ta|$b/bullinger/7538.xml"
  "CBETA|Taishō T1796, Yixing's commentary on the *Mahāvairocana Sūtra*|$ta|$b/cbeta/T39n1796.xml"
  "CLiGS textbox|Gutiérrez, *Juan Moreira*|$ta|$b/cligs/nh0053.xml"
  "conha19|Calcagno, *En busca del eslabón*|$ta|$b/conha19/nh0215.xml"
  "Coptic Scriptorium|John IV of Constantinople, *On Penitence and Abstinence* 1|$ta|$b/coptic/penitence.01.xml"
  "correspSearch (CMIF)|Georg Forster's correspondence|$ta|$b/cmif/forster_gesamt.xml"
  "CroALa|Marulić, *De institutione bene vivendi*|$ta|$b/croala/marul-mar-inst.xml"
  "CSEL (Open Greek and Latin)|Augustine, *De baptismo*|$ta|$b/csel/stoa0040.stoa033a.opp-lat1.xml"
  "DARIAH-ERIC lexical resources|Concordance entry *vera* (Grettis saga)|$ta|$b/lexicalresources/Corpus2Dictionary_vera.xml"
  "Deutsches Lyrik Korpus (DLK)|Lohenstein, *Venus*|$ta|$b/dlk/textgrid.poem.53041-Lohenstein-Venus.xml"
  "Deutsches Textarchiv (DTA)|Kant, *Kritik der reinen Vernunft* (1781)|$ta|$b/dta/kant_rvernunft_1781.xml"
  "DHARMA|*Kakawin Sutasoma*, critical edition|$ta|$b/dharma/DHARMA_CritEdKakavinSutasoma.xml"
  "Digital Mitford|Mitford, *Rienzi*|$ta|$b/mitford/Rienzi.xml"
  "DISCO|Prosopography of the sonnets' authors|$ta|$b/disco/prosopography.xml"
  "DraCor|GerDraCor: Goethe, *Faust II*|$ta|$b/gerdracor/goethe-faust-der-tragoedie-zweiter-teil.xml"
  "ELTeC|Brontë, *Wuthering Heights* (1847)|$ta|$b/eltec/ENG18471_Bronte.xml"
  "ENCPOS (École des chartes)|Opening session of the École des chartes, 5 May 1847|$ta|$b/encpos/ENC_1-1847-1865_PREV2.xml"
  "Faust-Edition|Faust II, manuscript 2 H, textual transcript|$ta|$b/faust/gsa_391098.xml"
  "Fihrist|Manchester, Persian MS 10|$ta|$b/fihrist/manchester_Persian_MS_10.xml"
  "First1KGreek|tlg0527.tlg001|$ta|$b/first1kgreek/tlg0527.tlg001.1st1K-grc1.xml"
  "Frankenstein Variorum|Collation spine C08|$ta|$b/frankenstein/spine_C08.xml"
  "FreeDict|Afrikaans–German dictionary|$ta|$b/freedict/afr-deu.xml"
  "FRUS|1981–1988, vol. XVI, South America|$ta|$b/frus/frus1981-88v16.xml"
  "Genji monogatari (nakamura196)|Kuronet transcription of *Kiritsubo*|$ta|$b/genji/kuronet_3a83785106e8af47fd5c465a5b3d2192.xml"
  "Geste|*Floovant* (Guessard, 1858)|$ta|$b/geste/ed_FloovG.xml"
  "Handrit.is|AM 377 fol.|$ta|$b/handrit/AM02-0377-en.xml"
  "I.Sicily|ISic000001|$ta|tests/tei/docs/isicily/ISic000001.xml"
  "Inscriptions of Israel/Palestine|jord0001|$ta|$b/iip/jord0001.xml"
  "IOSPE|Introduction to the Byzantine inscriptions|$ta|$b/iospe/introductions_byzantine.xml"
  "Legal Kraus|*Karl Kraus contra …*, vol. 2|$ta|$b/legalkraus/boehm_vol2.xml"
  "MiMoText roman18|Rousseau, *Julie ou la Nouvelle Héloïse*|$ta|$b/roman18/Rousseau_Julie.xml"
  "OBVIL Critique|Sainte-Beuve, *Nouveaux lundis*, vol. 8|$ta|$b/obvil/sainte-beuve_nouveaux-lundis-08.xml"
  "OpenArabicPE|*al-Manār*, vol. 29, no. 6|$ta|$b/openarabicpe/oclc_1588981-v_29-i_6.TEIP5.xml"
  "papyri.info|10 DDbDP/DCLP documents|$pi|$(echo $b/papyri/DDbDP/*.xml $b/papyri/DCLP/*.xml)"
  "papyri.info|one DDbDP document|$pi|$b/papyri/DDbDP/0_1.xml"
  "ParlaMint|UK Commons, 2017-09-07 (annotated)|$ta|$b/parlamint/ParlaMint-GB_2017-09-07-commons.ana.xml"
  "Patrologia Latina|Hilary of Poitiers, *Vita operaque*|$ta|$b/patrologia-latina/stoa0149b.stoa001.opp-lat2.xml"
  "Perseus|Caesar, *De bello Gallico*|$ta|$b/perseus/phi0448.phi001.perseus-lat2.xml"
  "Persian Digital Library|ʿUrfī, *Dīvān*|$ta|$b/pdl/orfi.divan.pdl.xml"
  "SARIT|*Aṣṭāṅgahṛdayasaṃhitā*|$ta|$b/sarit/astangahrdayasamhita.xml"
  "Syriaca.org|Nestorius, *The Book of Heraclides*|$ta|$b/syriaca/391.xml"
  "TCP|EEBO-TCP A01185|$ta|$b/tcp/A01185.xml"
  "Tolstoy Digital|*War and Peace*, variants to volume 1|$ta|$b/tolstoy/v13_198_541_Varianty_k_tomu_pervomu_Vojny_i_mira.xml"
  "U.S. Epigraphy|Princeton University Art Museum 3603-I56|$ta|$b/usep/NJ.Princ.PU.AM.G.3603-I56.xml"
  "VICAV|Dictionary of Baghdad Arabic|$ta|$b/vicav/dc_acm_baghdad_eng_001.xml"
)

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

schematron=${SCHEMATRON:-1}
if [[ $schematron == 1 ]]; then
  jars=${XDG_CACHE_HOME:-$HOME/.cache}/tei-rs-bench
  mkdir -p "$jars"
  fetch_jar() { # fetch_jar MAVEN_PATH SHA256 -> local path of the jar
    local file=$jars/${1##*/}
    if [[ ! -f $file ]] || [[ $(sha256sum "$file" | cut -d' ' -f1) != "$2" ]]; then
      echo "downloading ${1##*/}" >&2
      curl -fsSL --retry 3 -o "$file" "https://repo1.maven.org/maven2/$1"
      [[ $(sha256sum "$file" | cut -d' ' -f1) == "$2" ]] || { echo "checksum mismatch: $file" >&2; return 1; }
    fi
    echo "$file"
  }
  saxon=$(fetch_jar net/sf/saxon/Saxon-HE/12.5/Saxon-HE-12.5.jar 98c3a91e6e5aaf9b3e2b37601e04b214a6e67098493cdd8232fcb705fddcb674)
  resolver=$(fetch_jar org/xmlresolver/xmlresolver/5.2.2/xmlresolver-5.2.2.jar efc92bd7ed32b3e57095e0b3e872051ccfbbdcc980831ef33e89e38161a85222)
  resolver_data=$(fetch_jar org/xmlresolver/xmlresolver/5.2.2/xmlresolver-5.2.2-data.jar 173904bdbd783ba0fac92c5bcc05da5d09f0ce7eed24346666ea0a239461f9b4)
  schxslt=$(fetch_jar name/dmaus/schxslt/schxslt/1.10.1/schxslt-1.10.1.jar 4f4f21edab7b37f96ad59ae12a344d3510f1092ac46b6d81a4efa0120b73cb58)
  export SAXON_CP=$saxon:$resolver:$resolver_data JING_JAR
  python3 -I -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1]); z.extractall(sys.argv[2], [n for n in z.namelist() if n.startswith("xslt/")])' "$schxslt" "$tmp"
  # The reference pipeline, as one command: Jing, then the compiled rules.
  # reference.sh XSL|- SCHEMA SOURCE OUTPUT DOC...
  cat >"$tmp/reference.sh" <<'EOF'
#!/usr/bin/env bash
xsl=$1 schema=$2 source=$3 output=$4
shift 4
java -jar "$JING_JAR" "$schema" "$@"
status=$?
if [[ $xsl != - ]]; then
  java -cp "$SAXON_CP" net.sf.saxon.Transform -xsl:"$xsl" -s:"$source" -o:"$output" || exit 2
fi
exit $status
EOF
fi

# compiled_xsl SCHEMA -> the schema's embedded rules compiled to XSLT by
# SchXslt (once per schema), or "-" when it embeds none.
compiled_xsl() {
  local key
  key=$(printf '%s' "$1" | sha256sum | cut -c1-16)
  if [[ ! -e $tmp/$key.done ]]; then
    java -cp "$SAXON_CP" net.sf.saxon.Transform -xsl:scripts/schematron/extract.xsl -s:"$1" -o:"$tmp/$key.sch"
    if grep -q 'pattern' "$tmp/$key.sch"; then
      java -cp "$SAXON_CP" net.sf.saxon.Transform -xsl:"$tmp/xslt/2.0/compile-for-svrl.xsl" -s:"$tmp/$key.sch" -o:"$tmp/$key.xsl"
    fi
    touch "$tmp/$key.done"
  fi
  if [[ -e $tmp/$key.xsl ]]; then echo "$tmp/$key.xsl"; else echo -; fi
}

# mean_sd JSON INDEX -> "mean ± sd ms<TAB>mean" for the INDEXth command timed
mean_sd() {
  python3 -I - "$1" "$2" <<'EOF'
import json, sys
r = json.load(open(sys.argv[1]))["results"][int(sys.argv[2])]
r = r["summary"]["time_wall_clock"] if "summary" in r else r  # hyperfine 2 / 1
m, s = r["mean"] * 1000, (r["stddev"] or 0) * 1000
print(f"{m:.1f} ± {s:.1f} ms\t{m}")
EOF
}
size() {
  awk -v n="$1" 'BEGIN { if (n >= 1048576) printf "%.1f MB", n / 1048576; else printf "%d KB", (n + 512) / 1024 }'
}
# compare NAME CMD NAME CMD -> "reference | tei-rs | speed-up" (hyperfine)
compare() {
  # -i: documents that are invalid against the schema exit with status 1
  hyperfine -N -i --style none --warmup 1 --runs "$runs" --export-json "$tmp/r.json" \
    --command-name "$1" "$2" --command-name "$3" "$4" >/dev/null 2>&1
  local theirs theirs_ms ours ours_ms
  IFS=$'\t' read -r theirs theirs_ms < <(mean_sd "$tmp/r.json" 0)
  IFS=$'\t' read -r ours ours_ms < <(mean_sd "$tmp/r.json" 1)
  echo "$theirs | $ours | $(awk -v j="$theirs_ms" -v t="$ours_ms" 'BEGIN { printf "%.1f×", j / t }')"
}

header='| Project | Document | Size | Jing | tei-rs | Speed-up |'
rule='|---|---|---|---|---|---|'
if [[ ${XMLLINT:-} == 1 ]]; then
  header='| Project | Document | Size | xmllint | Jing | tei-rs | Speed-up |'
  rule='|---|---|---|---|---|---|---|'
fi
relaxng=("$header" "$rule")
with_rules=('| Project | Document | Size | Jing + SchXslt | tei-rs | Speed-up |' '|---|---|---|---|---|---|')

n=0
for row in "${rows[@]}"; do
  IFS='|' read -r project document schema files <<<"$row"
  [[ -n $filter && $project$document != *"$filter"* ]] && continue
  read -ra docs <<<"$files"
  for doc in "${docs[@]}"; do
    [[ -f $doc ]] || { echo "missing $doc; run scripts/fetch-samples.sh" >&2; exit 2; }
  done
  echo "timing $project: $document" >&2
  line="| $project | $document | $(size "$(cat "${docs[@]}" | wc -c)") |"
  if [[ ${XMLLINT:-} == 1 ]]; then
    start=$(date +%s%N)
    rc=0
    timeout "$limit" xmllint --noout --relaxng "$schema" "${docs[@]}" >/dev/null 2>&1 || rc=$?
    ms=$(( ($(date +%s%N) - start) / 1000000 ))
    if [[ $rc -eq 124 ]]; then line+=" > $limit s |"; else line+=" $ms ms (1 run) |"; fi
  fi
  relaxng+=("$line $(compare jing "java -jar $JING_JAR $schema ${docs[*]}" \
    tei-rs "$bin validate --no-schematron --schema $schema ${docs[*]}") |")

  if [[ $schematron == 1 ]]; then
    n=$((n + 1))
    xsl=$(compiled_xsl "$schema")
    # Saxon takes one source document, or a directory of them.
    if [[ ${#docs[@]} -eq 1 ]]; then
      source=${docs[0]} output=$tmp/out.svrl
    else
      source=$tmp/docs$n output=$tmp/out$n
      mkdir -p "$source" "$output"
      for doc in "${docs[@]}"; do ln -s "$PWD/$doc" "$source/"; done
    fi
    reference="bash $tmp/reference.sh $xsl $schema $source $output ${docs[*]}"
    rc=0
    $reference >/dev/null 2>"$tmp/err" || rc=$?
    if [[ $rc -ge 2 ]]; then
      echo "the Schematron reference failed on $project: $document:" >&2
      cat "$tmp/err" >&2
      exit 2
    fi
    with_rules+=("| $project | $document | $(size "$(cat "${docs[@]}" | wc -c)") | $(compare reference "$reference" \
      tei-rs "$bin validate --schema $schema ${docs[*]}") |")
  fi
done

printf '\n### RELAX NG\n\n'
printf '%s\n' "${relaxng[@]}"
if [[ $schematron == 1 ]]; then
  printf '\n### RELAX NG and Schematron\n\n'
  printf '%s\n' "${with_rules[@]}"
fi

# Write the results into the documents, unless only some rows were timed.
if [[ -n $filter || ${BENCH_WRITE:-1} == 0 ]]; then
  exit 0
fi
printf '%s\n' "${relaxng[@]}" >"$tmp/relaxng.md"
documents=(--relaxng "$tmp/relaxng.md")
if [[ $schematron == 1 ]]; then
  printf '%s\n' "${with_rules[@]}" >"$tmp/schematron.md"
  documents+=(--schematron "$tmp/schematron.md")
fi
machine=${BENCH_MACHINE:-}
if [[ -z $machine ]]; then
  cpu=$(lscpu 2>/dev/null | sed -n 's/^Model name: *//p' | head -1)
  [[ -n $cpu ]] || cpu=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -m)
  os=$( (. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME") || echo "$(uname -s) $(uname -r)")
  machine="$cpu, $os"
fi
commit="\`$(git rev-parse --short HEAD)\`"
if [[ -n $(git status --porcelain --untracked-files=no -- . ':!BENCHMARKS.md' ':!README.md') ]]; then
  commit+=" with uncommitted changes"
fi
if [[ $runs == 1 ]]; then timed="1 run"; else timed="$runs runs"; fi
info="Commit $commit, $(date +%Y-%m-%d): hyperfine $(hyperfine --version | cut -d' ' -f2), $timed after 1 warm-up run, on $machine."
python3 scripts/bench-docs.py --info "$info" "${documents[@]}"
echo "updated BENCHMARKS.md and README.md: review with 'git --no-pager diff --stat' (or 'git diff', q to leave), then commit" >&2

