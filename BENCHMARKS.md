# Benchmarks

Timings of the `tei-rs` command-line tool against the reference validators:
[Jing](https://relaxng.org/jclark/jing.html) for RELAX NG, and for the
Schematron rules TEI schemas embed, the pipeline TEI itself uses (the rules
compiled to XSLT by [SchXslt](https://github.com/schxslt/schxslt) and run on
[Saxon-HE](https://www.saxonica.com/)).

## Results

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) by
`scripts/bench-table.sh`, which also writes the tables and their figures
here: mean ± standard deviation of the timed runs, after 1 warm-up run. Each
table says when, on what and at which commit it was measured. Every tool is
timed as command-line runs (process start, schema compile, parse, validate);
the Java tools' times include JVM start-up.

One representative sample from each project, in alphabetical order by
project. Every document is validated against TEI P5 `tei_all`, except the
two papyri.info rows, which use the project's own papyri.info schema. Many of
the documents are invalid against `tei_all`, some with thousands of errors
(Geste, Tolstoy Digital, Schnitzler); every tool reports every error.

### RELAX NG

tei-rs with `--no-schematron` against Jing, which checks RELAX NG (and IDs)
only.

<!-- bench:relaxng -->
_Commit `1fd4075`, 2026-10-16: hyperfine 2.0.0, 5 runs after 1 warm-up run, on
a dedicated 2018 Mac Intel machine running Ubuntu 26._

| Project | Document | Size | Jing | tei-rs | Speed-up |
|---|---|---|---|---|---|
| 84000 | *The King of Samādhis Sūtra* (Toh 127) | 2.5 MB | 776.1 ± 15.9 ms | 81.4 ± 0.8 ms | 9.5× |
| Akademieprotokolle (ÖAW) | Minutes of a general assembly, A_0005 | 1009 KB | 439.6 ± 9.3 ms | 62.0 ± 0.2 ms | 7.1× |
| Arthur Schnitzler | *Schnitzler-Briefe*: index of works | 2.0 MB | 963.2 ± 29.0 ms | 86.1 ± 0.5 ms | 11.2× |
| Auden Musulin Papers | Stella Musulin, typescript memoir *The Years in Austria* | 325 KB | 396.4 ± 6.0 ms | 51.6 ± 0.7 ms | 7.7× |
| Beta maṣāḥǝft | Synaxarion manuscript ESam019 | 2.2 MB | 1681.8 ± 8.6 ms | 69.6 ± 0.2 ms | 24.2× |
| Bodleian medieval manuscripts | Places authority file | 1.6 MB | 437.6 ± 4.3 ms | 69.6 ± 0.4 ms | 6.3× |
| Bullinger Digital | Thomas Erastus, letter of 23 July 1568 | 1.5 MB | 399.1 ± 6.9 ms | 79.4 ± 1.0 ms | 5.0× |
| CBETA | Taishō T1796, Yixing's commentary on the *Mahāvairocana Sūtra* | 2.9 MB | 777.1 ± 8.9 ms | 76.3 ± 0.3 ms | 10.2× |
| CLiGS textbox | Gutiérrez, *Juan Moreira* | 626 KB | 319.0 ± 4.1 ms | 47.2 ± 1.4 ms | 6.8× |
| conha19 | Calcagno, *En busca del eslabón* | 520 KB | 323.8 ± 2.5 ms | 46.3 ± 0.2 ms | 7.0× |
| Coptic Scriptorium | John IV of Constantinople, *On Penitence and Abstinence* 1 | 532 KB | 342.5 ± 3.0 ms | 58.2 ± 0.5 ms | 5.9× |
| correspSearch (CMIF) | Georg Forster's correspondence | 1.1 MB | 817.8 ± 11.3 ms | 54.0 ± 0.3 ms | 15.1× |
| CroALa | Marulić, *De institutione bene vivendi* | 1.8 MB | 319.2 ± 4.7 ms | 52.5 ± 0.7 ms | 6.1× |
| CSEL (Open Greek and Latin) | Augustine, *De baptismo* | 770 KB | 314.7 ± 5.7 ms | 55.1 ± 1.1 ms | 5.7× |
| DARIAH-ERIC lexical resources | Concordance entry *vera* (Grettis saga) | 2.4 MB | 442.2 ± 5.8 ms | 93.7 ± 0.3 ms | 4.7× |
| Deutsches Lyrik Korpus (DLK) | Lohenstein, *Venus* | 263 KB | 267.9 ± 2.8 ms | 45.5 ± 0.5 ms | 5.9× |
| Deutsches Textarchiv (DTA) | Kant, *Kritik der reinen Vernunft* (1781) | 2.0 MB | 359.1 ± 5.7 ms | 77.2 ± 1.2 ms | 4.7× |
| DHARMA | *Kakawin Sutasoma*, critical edition | 914 KB | 323.1 ± 1.6 ms | 54.8 ± 1.1 ms | 5.9× |
| Digital Mitford | Mitford, *Rienzi* | 582 KB | 335.9 ± 4.8 ms | 55.1 ± 0.5 ms | 6.1× |
| DISCO | Prosopography of the sonnets' authors | 1.5 MB | 1339.0 ± 11.3 ms | 85.3 ± 0.8 ms | 15.7× |
| DraCor | GerDraCor: Goethe, *Faust II* | 630 KB | 374.8 ± 2.7 ms | 53.4 ± 1.3 ms | 7.0× |
| ELTeC | Brontë, *Wuthering Heights* (1847) | 896 KB | 321.6 ± 3.0 ms | 47.7 ± 0.6 ms | 6.7× |
| ENCPOS (École des chartes) | Opening session of the École des chartes, 5 May 1847 | 123 KB | 325.5 ± 5.3 ms | 43.1 ± 0.9 ms | 7.5× |
| Faust-Edition | Faust II, manuscript 2 H, textual transcript | 1.2 MB | 620.2 ± 10.0 ms | 72.1 ± 1.1 ms | 8.6× |
| Fihrist | Manchester, Persian MS 10 | 486 KB | 431.4 ± 4.1 ms | 51.4 ± 0.9 ms | 8.4× |
| First1KGreek | tlg0527.tlg001 | 589 KB | 351.2 ± 5.1 ms | 50.8 ± 0.9 ms | 6.9× |
| Frankenstein Variorum | Collation spine C08 | 2.0 MB | 1249.8 ± 24.6 ms | 71.8 ± 0.4 ms | 17.4× |
| FreeDict | Afrikaans–German dictionary | 1.3 MB | 338.6 ± 6.5 ms | 72.5 ± 0.8 ms | 4.7× |
| FRUS | 1981–1988, vol. XVI, South America | 1.2 MB | 439.4 ± 4.0 ms | 55.0 ± 0.9 ms | 8.0× |
| Genji monogatari (nakamura196) | Kuronet transcription of *Kiritsubo* | 1.7 MB | 656.8 ± 18.8 ms | 83.4 ± 0.6 ms | 7.9× |
| Geste | *Floovant* (Guessard, 1858) | 2.9 MB | 2286.7 ± 32.4 ms | 169.9 ± 0.8 ms | 13.5× |
| Handrit.is | AM 377 fol. | 1.7 MB | 392.5 ± 7.4 ms | 71.5 ± 0.3 ms | 5.5× |
| I.Sicily | ISic000001 | 15 KB | 324.3 ± 2.8 ms | 44.2 ± 1.1 ms | 7.3× |
| Inscriptions of Israel/Palestine | jord0001 | 80 KB | 346.1 ± 5.3 ms | 44.6 ± 0.7 ms | 7.8× |
| IOSPE | Introduction to the Byzantine inscriptions | 730 KB | 291.2 ± 3.1 ms | 49.0 ± 0.8 ms | 5.9× |
| Legal Kraus | *Karl Kraus contra …*, vol. 2 | 1.9 MB | 601.0 ± 11.5 ms | 73.0 ± 0.6 ms | 8.2× |
| MiMoText roman18 | Rousseau, *Julie ou la Nouvelle Héloïse* | 2.1 MB | 340.4 ± 8.6 ms | 61.2 ± 0.8 ms | 5.6× |
| OBVIL Critique | Sainte-Beuve, *Nouveaux lundis*, vol. 8 | 976 KB | 325.2 ± 4.1 ms | 48.6 ± 0.9 ms | 6.7× |
| OpenArabicPE | *al-Manār*, vol. 29, no. 6 | 417 KB | 353.4 ± 1.5 ms | 49.7 ± 0.8 ms | 7.1× |
| papyri.info | 10 DDbDP/DCLP documents | 54 KB | 347.3 ± 2.5 ms | 68.4 ± 0.8 ms | 5.1× |
| papyri.info | one DDbDP document | 8 KB | 306.8 ± 1.5 ms | 41.8 ± 1.2 ms | 7.3× |
| ParlaMint | UK Commons, 2017-09-07 (annotated) | 74 KB | 428.9 ± 4.9 ms | 44.7 ± 0.7 ms | 9.6× |
| Patrologia Latina | Hilary of Poitiers, *Vita operaque* | 2.8 MB | 376.4 ± 5.3 ms | 82.4 ± 0.6 ms | 4.6× |
| Perseus | Caesar, *De bello Gallico* | 676 KB | 349.6 ± 6.6 ms | 50.1 ± 1.0 ms | 7.0× |
| Persian Digital Library | ʿUrfī, *Dīvān* | 1.8 MB | 440.3 ± 4.6 ms | 64.9 ± 0.7 ms | 6.8× |
| SARIT | *Aṣṭāṅgahṛdayasaṃhitā* | 2.9 MB | 409.6 ± 7.4 ms | 93.1 ± 1.1 ms | 4.4× |
| Syriaca.org | Nestorius, *The Book of Heraclides* | 1.7 MB | 317.3 ± 2.6 ms | 58.6 ± 2.3 ms | 5.4× |
| TCP | EEBO-TCP A01185 | 198 KB | 308.5 ± 4.4 ms | 45.8 ± 0.7 ms | 6.7× |
| Tolstoy Digital | *War and Peace*, variants to volume 1 | 2.2 MB | 952.3 ± 11.4 ms | 76.9 ± 0.3 ms | 12.4× |
| U.S. Epigraphy | Princeton University Art Museum 3603-I56 | 27 KB | 320.4 ± 2.5 ms | 44.0 ± 1.0 ms | 7.3× |
| VICAV | Dictionary of Baghdad Arabic | 2.4 MB | 498.2 ± 9.3 ms | 85.8 ± 0.4 ms | 5.8× |
| **Median** | | | 374.8 ms | 58.2 ms | 7.0× |

tei-rs is faster than Jing on every sample: 4.4–24.2×, 7.0× at the median. The
largest speed-ups are for Beta maṣāḥǝft, Frankenstein Variorum and DISCO; the
smallest for DARIAH-ERIC lexical resources, Patrologia Latina and SARIT.
tei-rs's timings vary less from run to run: σ of at most 2.3 ms, against
Jing's 1.5–32 ms.
<!-- /bench:relaxng -->

On small documents most of the gap is JVM start-up; on large ones the
validator caches derivatives per content model (see
[`src/relaxng/PROVENANCE.md`](src/relaxng/PROVENANCE.md)), so repeated element
types cost a lookup rather than a fresh derivation.

### RELAX NG and Schematron

tei-rs with its default settings, which check the Schematron rules embedded
in the schema after RELAX NG, against TEI's reference pipeline: Jing, then
the same rules compiled to XSLT by SchXslt 1.10.1 and run on Saxon-HE 12.5.
The reference's rules are compiled once per schema before timing starts, so
its timings leave that step out; tei-rs's include compiling them. Both
report the same Schematron messages (`tests/schematron_agreement.rs`).
`tei_all` embeds 47 Schematron tests, of which tei-rs evaluates 45 (the
other 2 need `current-date()`; tei-rs lists them as unchecked), and the
papyri.info schema 79, all evaluated.

<!-- bench:schematron -->
_Commit `1fd4075`, 2026-10-16: hyperfine 2.0.0, 5 runs after 1 warm-up run, on
a dedicated 2018 Mac Intel machine running Ubuntu 26._

| Project | Document | Size | Jing + SchXslt | tei-rs | Speed-up |
|---|---|---|---|---|---|
| 84000 | *The King of Samādhis Sūtra* (Toh 127) | 2.5 MB | 2102.8 ± 48.6 ms | 96.4 ± 0.7 ms | 21.8× |
| Akademieprotokolle (ÖAW) | Minutes of a general assembly, A_0005 | 1009 KB | 1945.4 ± 47.4 ms | 86.9 ± 0.3 ms | 22.4× |
| Arthur Schnitzler | *Schnitzler-Briefe*: index of works | 2.0 MB | 2533.9 ± 66.0 ms | 99.6 ± 0.2 ms | 25.4× |
| Auden Musulin Papers | Stella Musulin, typescript memoir *The Years in Austria* | 325 KB | 1441.0 ± 9.1 ms | 59.9 ± 0.6 ms | 24.0× |
| Beta maṣāḥǝft | Synaxarion manuscript ESam019 | 2.2 MB | 2882.4 ± 30.3 ms | 79.8 ± 0.7 ms | 36.1× |
| Bodleian medieval manuscripts | Places authority file | 1.6 MB | 1761.0 ± 26.0 ms | 82.3 ± 0.5 ms | 21.4× |
| Bullinger Digital | Thomas Erastus, letter of 23 July 1568 | 1.5 MB | 1509.9 ± 10.6 ms | 88.3 ± 1.1 ms | 17.1× |
| CBETA | Taishō T1796, Yixing's commentary on the *Mahāvairocana Sūtra* | 2.9 MB | 2226.3 ± 9.7 ms | 95.0 ± 0.2 ms | 23.4× |
| CLiGS textbox | Gutiérrez, *Juan Moreira* | 626 KB | 1299.1 ± 7.1 ms | 54.8 ± 0.8 ms | 23.7× |
| conha19 | Calcagno, *En busca del eslabón* | 520 KB | 1313.0 ± 13.4 ms | 54.1 ± 0.3 ms | 24.3× |
| Coptic Scriptorium | John IV of Constantinople, *On Penitence and Abstinence* 1 | 532 KB | 1463.4 ± 18.3 ms | 68.8 ± 0.3 ms | 21.3× |
| correspSearch (CMIF) | Georg Forster's correspondence | 1.1 MB | 1881.5 ± 34.2 ms | 63.7 ± 0.7 ms | 29.5× |
| CroALa | Marulić, *De institutione bene vivendi* | 1.8 MB | 1365.1 ± 16.3 ms | 61.4 ± 0.3 ms | 22.2× |
| CSEL (Open Greek and Latin) | Augustine, *De baptismo* | 770 KB | 1406.1 ± 8.0 ms | 64.4 ± 0.7 ms | 21.8× |
| DARIAH-ERIC lexical resources | Concordance entry *vera* (Grettis saga) | 2.4 MB | 1961.7 ± 38.7 ms | 113.2 ± 1.2 ms | 17.3× |
| Deutsches Lyrik Korpus (DLK) | Lohenstein, *Venus* | 263 KB | 1252.3 ± 7.2 ms | 54.0 ± 0.9 ms | 23.2× |
| Deutsches Textarchiv (DTA) | Kant, *Kritik der reinen Vernunft* (1781) | 2.0 MB | 1601.1 ± 13.3 ms | 91.6 ± 0.4 ms | 17.5× |
| DHARMA | *Kakawin Sutasoma*, critical edition | 914 KB | 1457.8 ± 13.9 ms | 64.4 ± 1.1 ms | 22.6× |
| Digital Mitford | Mitford, *Rienzi* | 582 KB | 1381.7 ± 9.7 ms | 64.8 ± 1.1 ms | 21.3× |
| DISCO | Prosopography of the sonnets' authors | 1.5 MB | 2468.8 ± 30.1 ms | 98.3 ± 0.9 ms | 25.1× |
| DraCor | GerDraCor: Goethe, *Faust II* | 630 KB | 1472.3 ± 14.7 ms | 64.8 ± 0.9 ms | 22.7× |
| ELTeC | Brontë, *Wuthering Heights* (1847) | 896 KB | 1325.7 ± 6.8 ms | 56.9 ± 1.1 ms | 23.3× |
| ENCPOS (École des chartes) | Opening session of the École des chartes, 5 May 1847 | 123 KB | 1280.6 ± 12.2 ms | 50.8 ± 0.7 ms | 25.2× |
| Faust-Edition | Faust II, manuscript 2 H, textual transcript | 1.2 MB | 1831.5 ± 25.7 ms | 101.4 ± 0.6 ms | 18.1× |
| Fihrist | Manchester, Persian MS 10 | 486 KB | 1508.4 ± 15.5 ms | 59.4 ± 0.2 ms | 25.4× |
| First1KGreek | tlg0527.tlg001 | 589 KB | 1438.3 ± 6.2 ms | 59.4 ± 2.4 ms | 24.2× |
| Frankenstein Variorum | Collation spine C08 | 2.0 MB | 2580.1 ± 37.0 ms | 83.2 ± 0.8 ms | 31.0× |
| FreeDict | Afrikaans–German dictionary | 1.3 MB | 1593.9 ± 13.6 ms | 84.5 ± 0.3 ms | 18.9× |
| FRUS | 1981–1988, vol. XVI, South America | 1.2 MB | 1527.4 ± 6.9 ms | 63.3 ± 0.2 ms | 24.1× |
| Genji monogatari (nakamura196) | Kuronet transcription of *Kiritsubo* | 1.7 MB | 2785.6 ± 95.9 ms | 101.7 ± 0.3 ms | 27.4× |
| Geste | *Floovant* (Guessard, 1858) | 2.9 MB | 3737.8 ± 38.8 ms | 188.4 ± 0.3 ms | 19.8× |
| Handrit.is | AM 377 fol. | 1.7 MB | 1596.2 ± 28.0 ms | 82.7 ± 0.8 ms | 19.3× |
| I.Sicily | ISic000001 | 15 KB | 1284.7 ± 14.0 ms | 51.8 ± 1.2 ms | 24.8× |
| Inscriptions of Israel/Palestine | jord0001 | 80 KB | 1329.3 ± 9.9 ms | 53.2 ± 0.7 ms | 25.0× |
| IOSPE | Introduction to the Byzantine inscriptions | 730 KB | 1452.8 ± 15.0 ms | 56.9 ± 0.9 ms | 25.5× |
| Legal Kraus | *Karl Kraus contra …*, vol. 2 | 1.9 MB | 1742.6 ± 20.6 ms | 84.6 ± 0.4 ms | 20.6× |
| MiMoText roman18 | Rousseau, *Julie ou la Nouvelle Héloïse* | 2.1 MB | 1403.4 ± 16.1 ms | 69.0 ± 1.0 ms | 20.3× |
| OBVIL Critique | Sainte-Beuve, *Nouveaux lundis*, vol. 8 | 976 KB | 1315.8 ± 8.8 ms | 57.5 ± 0.3 ms | 22.9× |
| OpenArabicPE | *al-Manār*, vol. 29, no. 6 | 417 KB | 1356.0 ± 3.0 ms | 58.5 ± 0.6 ms | 23.2× |
| papyri.info | 10 DDbDP/DCLP documents | 54 KB | 1550.2 ± 16.5 ms | 82.0 ± 4.2 ms | 18.9× |
| papyri.info | one DDbDP document | 8 KB | 1400.9 ± 12.1 ms | 52.2 ± 0.8 ms | 26.8× |
| ParlaMint | UK Commons, 2017-09-07 (annotated) | 74 KB | 1419.6 ± 15.5 ms | 52.4 ± 1.0 ms | 27.1× |
| Patrologia Latina | Hilary of Poitiers, *Vita operaque* | 2.8 MB | 1763.7 ± 12.2 ms | 102.3 ± 0.8 ms | 17.2× |
| Perseus | Caesar, *De bello Gallico* | 676 KB | 1487.7 ± 10.8 ms | 57.5 ± 1.2 ms | 25.9× |
| Persian Digital Library | ʿUrfī, *Dīvān* | 1.8 MB | 1608.4 ± 11.1 ms | 76.7 ± 1.1 ms | 21.0× |
| SARIT | *Aṣṭāṅgahṛdayasaṃhitā* | 2.9 MB | 2050.7 ± 23.6 ms | 116.0 ± 1.0 ms | 17.7× |
| Syriaca.org | Nestorius, *The Book of Heraclides* | 1.7 MB | 1382.2 ± 13.9 ms | 67.9 ± 0.2 ms | 20.4× |
| TCP | EEBO-TCP A01185 | 198 KB | 1302.1 ± 12.2 ms | 53.9 ± 0.3 ms | 24.2× |
| Tolstoy Digital | *War and Peace*, variants to volume 1 | 2.2 MB | 2345.1 ± 34.6 ms | 87.1 ± 0.6 ms | 26.9× |
| U.S. Epigraphy | Princeton University Art Museum 3603-I56 | 27 KB | 1290.4 ± 3.6 ms | 51.7 ± 0.9 ms | 25.0× |
| VICAV | Dictionary of Baghdad Arabic | 2.4 MB | 1861.9 ± 22.1 ms | 103.1 ± 0.4 ms | 18.1× |
| **Median** | | | 1508.4 ms | 67.9 ms | 23.2× |

With Schematron, tei-rs is faster than the reference pipeline on every sample:
17.1–36.1×, 23.2× at the median. Checking the rules adds a median of 10 ms to
tei-rs (most for Faust-Edition: 29 ms), against a median of 1.1 s for the
reference's second JVM and XSLT run.
<!-- /bench:schematron -->

`TEI_BENCH_SCHEMATRON=1 cargo bench` times validation with the rules
in-process.

## Reproducing

The sample documents are listed, with links to their pinned upstream
versions, in [`tests/tei/SAMPLES.md`](tests/tei/SAMPLES.md). Download them
first (about 84 MB, SHA-256 verified):

```sh
scripts/fetch-samples.sh
```

Then:

```sh
cargo bench                              # every sample against tei_all, 5 runs each
TEI_BENCH_SCHEMAS=declared cargo bench   # each document against its own schema from schemas/
TEI_BENCH_RUNS=10 cargo bench -- sarit   # more runs, only paths containing "sarit"
```

`cargo bench` measures validation inside one process (schema compiled once,
parse and validate timed per document). The table above compares the tools
as command-line runs with hyperfine (Linux: `sudo apt install hyperfine
default-jre-headless`; macOS: `brew install hyperfine openjdk`), and
`scripts/bench-table.sh` produces both tables in one go and writes them,
with the figures quoted around them, into this file and the README (it
downloads Saxon-HE and SchXslt from Maven Central on first use, checks their
SHA-256 and caches them). Review the change with `git diff`, then commit it:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/TEIC/TEI ../TEI
git -C ../TEI sparse-checkout set P5/Utilities/lib
export JING_JAR=$(cd ../TEI/P5/Utilities/lib && pwd)/jing.jar

scripts/bench-table.sh               # 5 runs per tool and document, after 1 warm-up run
scripts/bench-table.sh 10 SARIT      # 10 runs, only rows mentioning "SARIT"; prints, writes nothing
SCHEMATRON=0 scripts/bench-table.sh  # the RELAX NG table only
BENCH_MACHINE="a dedicated 2018 Mac Intel machine running Ubuntu 26" scripts/bench-table.sh
XMLLINT=1 scripts/bench-table.sh     # add a column for libxml2's xmllint
```

For a single document:

```sh
TA=tests/tei/schemas/tei_all-2.7.0.rng
# -i: documents that are invalid against tei_all exit with status 1
hyperfine -N -i --warmup 1 --runs 5 \
  --command-name Jing   "java -jar $JING_JAR $TA tests/tei/bench/sarit/astangahrdayasamhita.xml" \
  --command-name tei-rs "target/release/tei-rs validate --no-schematron --schema $TA tests/tei/bench/sarit/astangahrdayasamhita.xml"
```

`scripts/bench-vs-jing.sh 3` prints a best-of-3 comparison for every sample,
not just one per project.
