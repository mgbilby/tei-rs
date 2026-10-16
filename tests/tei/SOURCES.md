# Test and benchmark data: sources and licences

Everything under `tests/tei/` is third-party data, used unmodified for testing
and benchmarking only. Each file keeps its own licence; none of it is covered
by this repository's code licence. Sources were fetched on 2026-10-15 and, for the projects from Arthur Schnitzler
onwards below, on 2026-10-16. Rows marked **NC/ND** or **NC** have non-commercial
or no-derivatives licences, and rows marked **no licence stated** have none
(all rights reserved by default); `licences.tsv` keeps all of them out of the
public release (`fetch-only`).

| Path | Project | Source (repository @ commit) | Licence |
|---|---|---|---|
| `schemas/tei_all-2.7.0.rng` | TEI P5 2.7.0 (2014), TEI Consortium | github.com/TEIC/TEI-Simple @ `16095eb1b08e` (`tei_all.rng`) | BSD-2-Clause or CC BY 3.0 (TEI dual licence) |
| `docs/tei-c/` | TEI Consortium test suite | github.com/TEIC/TEI @ `3525937afae1`, `P5/Test/` | BSD-2-Clause or CC BY 3.0 |
| `docs/gerdracor/`, `bench/gerdracor/` | DraCor — German Drama Corpus | github.com/dracor-org/gerdracor @ `7a7a3147b57a`, `tei/` | CC0 1.0 |
| `docs/isicily/` | I.Sicily — Inscriptions of Sicily | github.com/ISicily/ISicily @ `572d7f5ee1cb`, `inscriptions/` | CC BY 4.0 |
| `bench/papyri/<collection>/` | papyri.info / IDP: 5 files from each of its 9 XML collections (APD, APIS, Biblio, DCLP, DDbDP, HGV_meta_EpiDoc, HGV_metadata, Historical, Translations), each from a different subdirectory where the collection has them | github.com/papyri/idp.data @ `2ccd42bf1c74` (file name = path inside the collection, `/` → `_`) | CC BY 3.0, attribution to the respective projects |
| `bench/tcp/` | Text Creation Partnership (EEBO-TCP) | github.com/textcreationpartnership/A00001 @ `8b2f32f2e3e6`, …/A01185 @ `0aed25a06e11` | CC0 1.0 |
| `bench/perseus/` | Perseus Digital Library (Caesar, *De bello Gallico*) | github.com/PerseusDL/canonical-latinLit @ `128a05afa9cc` | CC BY-SA 4.0 |
| `bench/first1kgreek/` | First1KGreek, Open Greek and Latin | github.com/OpenGreekAndLatin/First1KGreek @ `03776b39f404` | CC BY-SA 4.0 |
| `bench/eltec/` | ELTeC — European Literary Text Collection (English) | github.com/COST-ELTeC/ELTeC-eng @ `3db3b6738a83`, `level1/` | Texts public domain; markup CC BY 4.0 |
| `bench/parlamint/` | ParlaMint (UK Parliament sample) | github.com/clarin-eric/ParlaMint @ `d5e880fbbd17`, `Samples/ParlaMint-GB/2017/` | CC BY 4.0 |
| `bench/dta/` | Deutsches Textarchiv (DTA), Berlin-Brandenburg Academy of Sciences: 8 texts (Goethe, Lessing, Schiller, Kant, Humboldt, Marx, Fontane, Kleist) | deutschestextarchiv.de, `book/download_xml/<id>` (URLs and SHA-256 in `samples.tsv`) | CC BY-SA 4.0 (per each file's TEI header) |
| `bench/sarit/` | SARIT — Search and Retrieval of Indic Texts | github.com/sarit/SARIT-corpus @ `1eac9ee0b055` | CC BY-SA 3.0 (per each file's TEI header) |
| `bench/schnitzler/` | Arthur Schnitzler: *Briefwechsel mit Autorinnen und Autoren* and *Tagebuch* (ACDH-CH, Austrian Academy of Sciences) | github.com/arthur-schnitzler/schnitzler-briefe-data @ `e058a5110ea5`, schnitzler-tagebuch-data @ `e6b008a7e74e` (file name = path inside the repository, `/` → `_`) | CC BY 4.0 (per each file's TEI header) |
| `bench/betamasaheft/` | Beta maṣāḥǝft: Manuscripts of Ethiopia and Eritrea (Universität Hamburg) | github.com/BetaMasaheft/Manuscripts @ `b7fc6d4dae8e` | CC BY-SA 4.0 (per each file's TEI header) |
| `bench/cbeta/` | CBETA — Chinese Buddhist Electronic Text Association | github.com/cbeta-org/xml-p5 @ `dbdea41071e1` | **Non-commercial use only**, "when distributed with this header intact" (per each file's TEI header; see cbeta.org/copyright.php) |
| `bench/coptic/` | Coptic Scriptorium | github.com/CopticScriptorium/corpora @ `3ac067f1709a` | CC BY 4.0 (repository README and TEI headers) |
| `bench/cmif/` | correspSearch: letter metadata in the Correspondence Metadata Interchange Format (CMIF) | github.com/correspSearch/csStorage @ `cb6437ab7f62` (file name = path inside the repository, `/` → `_`) | per file: `forster_gesamt.xml` and `feuerbach_1.xml` CC0 1.0, `avhumboldt_Jugendbriefe.xml` CC BY 4.0 |
| `bench/freedict/` | FreeDict bilingual dictionaries (`.tei` files, stored here as `.xml`) | github.com/freedict/fd-dictionaries @ `5bdceeac8d0d` | GPL (version per each file's TEI header) |
| `bench/legalkraus/` | Legal Kraus: Karl Kraus's legal papers (ACDH-CH) | github.com/karl-kraus/legalkraus-data @ `bc279c5e9a2b` | CC BY 4.0 (per each file's TEI header) |
| `bench/patrologia-latina/` | Patrologia Latina, Open Greek and Latin | github.com/OpenGreekAndLatin/patrologia_latina-dev @ `8c651f859f82` | CC BY-SA 4.0 (per each file's TEI header) |
| `bench/syriaca/` | Syriaca.org: Digital Syriac Corpus and the Syriaca.org gazetteer and prosopography | github.com/srophe/syriac-corpus @ `833adc148cc3`, srophe/srophe-app-data @ `fefe80f5aabe` | CC BY 4.0 (syriac-corpus), CC BY 3.0 (app data), per each file's TEI header |
| `bench/tolstoy/` | Tolstoy Digital: L. N. Tolstoy's complete works in 90 volumes | github.com/tolstoydigital/TEI @ `91b92a6b6fe7` | CC BY-SA (stated in Russian in each file's TEI header) |
| `bench/84000/` | 84000: Translating the Words of the Buddha (Kangyur translations and knowledge base) | github.com/84000/data-tei @ `bdfc81c9aecd` | **NC/ND**: CC BY-NC-ND 3.0 (repository README and each translation's TEI header) |
| `bench/akademie/` | Akademieprotokolle: minutes of the Austrian Academy of Sciences (ACDH-CH) | github.com/acdh-oeaw/akademie-data @ `6fe4b3e63ba4` | **no licence stated** (README, no LICENSE file, no `availability` in the TEI headers) |
| `bench/amp/` | Auden Musulin Papers: W. H. Auden's letters to Stella Musulin (ACDH-CH) | github.com/Auden-Musulin-Papers/amp-data @ `289a52de61ae` | CC BY 4.0 (per each file's TEI header) |
| `bench/bodleian/` | Medieval Manuscripts in Oxford Libraries (Bodleian Libraries) | github.com/bodleian/medieval-mss @ `4f445a5b60be` | **no licence stated** (README, no LICENSE file; the TEI headers give access conditions only) |
| `bench/bullinger/` | Bullinger Digital: Heinrich Bullinger's correspondence (Staatsarchiv Zürich, University of Zurich) | github.com/stazh/bullinger-korpus-tei @ `dc9fb5290c21` | **NC/ND**: CC BY-NC-ND 3.0 CH (repository README) |
| `bench/cligs/` | CLiGS textbox: Spanish, Portuguese and Italian novels and stories (Computational Literary Genre Stylistics, Würzburg) | github.com/cligs/textbox @ `1b91e940475c` | CC BY-SA 4.0 (per each file's TEI header) |
| `bench/conha19/` | conha19: corpus of 19th-century Spanish-American novels (CLiGS) | github.com/cligs/conha19 @ `d5def0a3b590` | Public Domain Mark 1.0 (repository README and each file's TEI header) |
| `bench/croala/` | Croatiae auctores Latini (CroALa), University of Zagreb | github.com/nevenjovanovic/croatiae-auctores-latini-textus @ `729f0ee28de9` | per file: `marul-mar-inst.xml` CC0 1.0 (TEI header), the others CC BY 4.0 (repository LICENSE.md) |
| `bench/csel/` | Corpus Scriptorum Ecclesiasticorum Latinorum (CSEL), Open Greek and Latin | github.com/OpenGreekAndLatin/csel-dev @ `2813e3a52b42` | CC BY-SA 4.0 (per each file's TEI header) |
| `bench/dharma/` | DHARMA: The Domestication of "Hindu" Asceticism and the Religious Making of South and Southeast Asia (ERC): inscriptions and critical editions | github.com/erc-dharma/tfd-nusantara-philology @ `3a30b0e78f5e`, github.com/erc-dharma/tfa-sii-epigraphy @ `dfa95be6729f`, github.com/erc-dharma/tfa-pallava-epigraphy @ `1b1b17c4df49` | per file: `DHARMA_CritEdKakavinSutasoma.xml` CC BY 4.0, the inscriptions CC BY-SA 4.0 (TEI headers; repository READMEs say CC BY 4.0) |
| `bench/disco/` | DISCO: Diachronic Spanish Sonnet Corpus | github.com/pruizf/disco @ `b626fefad22b` | CC BY 4.0 (repository LICENSE and README) |
| `bench/dlk/` | Deutsches Lyrik Korpus (German Poetry Corpus), poems from TextGrid | github.com/tnhaider/DLK @ `ef0b62054a30` | **no licence stated** (README, no LICENSE file, no `availability` in the TEI headers) |
| `bench/encpos/` | ENCPOS: *Positions de thèses* of the École nationale des chartes | github.com/chartes/encpos @ `3fcebbb67fe3` | **NC/ND**: CC BY-NC-ND 3.0 FR (per each file's TEI header) |
| `bench/faust/` | Faust-Edition: historical-critical edition of Goethe's *Faust* | github.com/faustedition/faust-xml @ `502eca65120d` | **NC**: CC BY-NC-SA 4.0 (per each file's TEI header) |
| `bench/fihrist/` | Fihrist: union catalogue of manuscripts from the Islamicate world | github.com/bodleian/fihrist-mss @ `2825c76d0623` | **no licence stated** (README, no LICENSE file; the TEI headers give access conditions only) |
| `bench/frankenstein/` | Frankenstein Variorum | github.com/FrankensteinVariorum/fv-data @ `2debfb706fbe` | CC BY-SA 3.0 (per each file's TEI header; the repository's LICENSE is the Unlicense) |
| `bench/frus/` | Foreign Relations of the United States (FRUS), Office of the Historian, U.S. Department of State | github.com/HistoryAtState/frus @ `6170e4b32c12` | public domain (U.S. government work; repository README) |
| `bench/genji/` | TEI editions of the *Tale of Genji* (Satoru Nakamura) | github.com/nakamura196/genji @ `3f0476e8ecdc` | per file: the Kuronet transcription CC BY 4.0, the Aozora Bunko texts CC0 1.0 (TEI headers) |
| `bench/geste/` | Geste: a corpus of Old French *chansons de geste* | github.com/Jean-Baptiste-Camps/Geste @ `71737a206323` | texts public domain (Public Domain Mark 1.0), markup CC BY-SA 4.0 (README and each file's TEI header) |
| `bench/handrit/` | Handrit.is: Icelandic and Old Norse manuscript catalogue | github.com/Handrit/Manuscripts @ `8c9a16775775` | **no licence stated** (README, no LICENSE file, TEI header `availability`) |
| `bench/iip/` | Inscriptions of Israel/Palestine (Brown University) | github.com/Brown-University-Library/iip-texts @ `0b7dc8358ccd` | **no licence stated** (README, no LICENSE file, no `availability` in the TEI headers) |
| `bench/iospe/` | IOSPE: Ancient Inscriptions of the Northern Black Sea (King's Digital Lab) | github.com/kingsdigitallab/iospe @ `94f678fa2641` | MIT (repository LICENSE; nothing in the TEI headers) |
| `bench/lexicalresources/` | DARIAH-ERIC lexical resources: TEI Lex-0 dictionaries and training data | github.com/DARIAH-ERIC/lexicalresources @ `b038b2b5b8eb` | per file: the Corpus2Dictionary entries CC BY 3.0, `Mueller-1878.tei.xml` CC BY 4.0 (TEI headers) |
| `bench/mitford/` | Digital Mitford: The Mary Russell Mitford Archive | github.com/ebeshero/Mitford @ `74b5dc485ac9` | CC BY-SA 3.0 (per each file's TEI header; the repository's LICENSE is AGPL-3.0) |
| `bench/obvil/` | OBVIL (Sorbonne Université): French literary criticism | github.com/OBVIL/critique @ `887de00dde6d` | **NC/ND**: CC BY-NC-ND 3.0 FR (per each file's TEI header) |
| `bench/openarabicpe/` | OpenArabicPE: Digital *al-Manār* | github.com/OpenArabicPE/digital-manar @ `83fb63adc72b` | CC BY-SA 4.0 (repository LICENSE.md and each file's TEI header) |
| `bench/pdl/` | Persian Digital Library (University of Maryland) | github.com/PersDigUMD/PDL @ `1d9ec2450d62` | CC BY-SA 4.0 (repository README) |
| `bench/roman18/` | MiMoText roman18: French novels 1751–1800 (Trier) | github.com/MiMoText/roman18 @ `c5cbec512c70` | CC0 1.0 (README and each file's TEI header) |
| `bench/usep/` | U.S. Epigraphy Project (Brown University) | github.com/Brown-University-Library/usep-data @ `e009be2c3d5e` | **no licence stated** (README, no LICENSE file, no `availability` in the TEI headers) |
| `bench/vicav/` | VICAV: Vienna Corpus of Arabic Varieties (ACDH-CH) | github.com/acdh-oeaw/vicav-content @ `66554f59b8f5` | **NC**: CC BY-NC-SA 3.0 (each file's TEI header; the repository README says CC BY-SA 4.0 unless otherwise noted) |

`expected.tsv` was produced by running Jing on these files
(`scripts/record-jing-expectations.sh`).

Notes on the papyri.info collections: `Biblio` files are standalone TEI
`<bibl>` records, not `<TEI>` documents, so `tei_all` rejects them by design.
`HGV_metadata` holds the HGV database's raw XML exports (not TEI), and some of
them are not well-formed XML; they are kept as negative / robustness cases.
