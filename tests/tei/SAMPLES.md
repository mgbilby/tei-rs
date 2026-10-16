# Sample files

Every TEI document and schema the tests and benchmarks use, with a link to the
exact upstream version. The public repository includes the files whose licence
allows it ([`licences.tsv`](licences.tsv)); download the rest, or all of them,
into `tests/tei/` with

```sh
scripts/fetch-samples.sh            # all files (about 84 MB; verifies SHA-256)
scripts/fetch-samples.sh papyri     # only paths containing "papyri"
```

The machine-readable list (with hashes) is [`samples.tsv`](samples.tsv); licences
are summarised in [`SOURCES.md`](SOURCES.md). Without these files, `cargo test`
skips the comparisons with Jing and `cargo bench` has nothing to time.

## Deutsches Textarchiv

8 file(s), 8.2 MB. Licence: https://creativecommons.org/licenses/by-sa/4.0/deed.de. Source: [deutschestextarchiv.de](https://deutschestextarchiv.de).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/dta/fontane_stechlin_1899.xml` | 1.1 MB | [https://www.deutschestextarchiv.de/book/download_xml/fontane_stechlin_1899](https://www.deutschestextarchiv.de/book/download_xml/fontane_stechlin_1899) |
| `bench/dta/goethe_faust01_1808.xml` | 434 KB | [https://www.deutschestextarchiv.de/book/download_xml/goethe_faust01_1808](https://www.deutschestextarchiv.de/book/download_xml/goethe_faust01_1808) |
| `bench/dta/humboldt_kosmos01_1845.xml` | 1.0 MB | [https://www.deutschestextarchiv.de/book/download_xml/humboldt_kosmos01_1845](https://www.deutschestextarchiv.de/book/download_xml/humboldt_kosmos01_1845) |
| `bench/dta/kant_rvernunft_1781.xml` | 2.0 MB | [https://www.deutschestextarchiv.de/book/download_xml/kant_rvernunft_1781](https://www.deutschestextarchiv.de/book/download_xml/kant_rvernunft_1781) |
| `bench/dta/kleist_krug_1811.xml` | 292 KB | [https://www.deutschestextarchiv.de/book/download_xml/kleist_krug_1811](https://www.deutschestextarchiv.de/book/download_xml/kleist_krug_1811) |
| `bench/dta/lessing_nathan_1779.xml` | 460 KB | [https://www.deutschestextarchiv.de/book/download_xml/lessing_nathan_1779](https://www.deutschestextarchiv.de/book/download_xml/lessing_nathan_1779) |
| `bench/dta/marx_kapital01_1867.xml` | 2.4 MB | [https://www.deutschestextarchiv.de/book/download_xml/marx_kapital01_1867](https://www.deutschestextarchiv.de/book/download_xml/marx_kapital01_1867) |
| `bench/dta/schiller_raeuber_1781.xml` | 502 KB | [https://www.deutschestextarchiv.de/book/download_xml/schiller_raeuber_1781](https://www.deutschestextarchiv.de/book/download_xml/schiller_raeuber_1781) |

## papyri.info

45 file(s), 3.7 MB. Licence: CC BY 3.0. Source: [papyri/idp.data](https://github.com/papyri/idp.data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/papyri/APD/apd1006.xml` | 2 KB | [APD/apd1006.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APD/apd1006.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APD/apd1459.xml` | 2 KB | [APD/apd1459.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APD/apd1459.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APD/apd1857.xml` | 2 KB | [APD/apd1857.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APD/apd1857.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APD/apd524.xml` | 2 KB | [APD/apd524.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APD/apd524.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APD/apd999.xml` | 2 KB | [APD/apd999.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APD/apd999.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APIS/berenike_xml_berenike.apis.1.xml` | 3 KB | [APIS/berenike/xml/berenike.apis.1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APIS/berenike/xml/berenike.apis.1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APIS/gothenburg_xml_gothenburg.apis.1.xml` | 4 KB | [APIS/gothenburg/xml/gothenburg.apis.1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APIS/gothenburg/xml/gothenburg.apis.1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APIS/oxford-ipap_xml_oxford-ipap.apis.1.xml` | 3 KB | [APIS/oxford-ipap/xml/oxford-ipap.apis.1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APIS/oxford-ipap/xml/oxford-ipap.apis.1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APIS/sacramento_xml_sacramento.apis.6599.xml` | 3 KB | [APIS/sacramento/xml/sacramento.apis.6599.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APIS/sacramento/xml/sacramento.apis.6599.xml) @ `2ccd42bf1c74` |
| `bench/papyri/APIS/yale_xml_yale.apis.0000010000.xml` | 4 KB | [APIS/yale/xml/yale.apis.0000010000.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/APIS/yale/xml/yale.apis.0000010000.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Biblio/1_1.xml` | 1 KB | [Biblio/1/1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Biblio/1/1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Biblio/31_30001.xml` | 1 KB | [Biblio/31/30001.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Biblio/31/30001.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Biblio/52_51001.xml` | 1 KB | [Biblio/52/51001.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Biblio/52/51001.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Biblio/72_71001.xml` | 2 KB | [Biblio/72/71001.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Biblio/72/71001.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Biblio/98_97001.xml` | 1 KB | [Biblio/98/97001.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Biblio/98/97001.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DCLP/0_555.xml` | 5 KB | [DCLP/0/555.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DCLP/0/555.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DCLP/17_17453.xml` | 7 KB | [DCLP/17/17453.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DCLP/17/17453.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DCLP/39_39226.xml` | 4 KB | [DCLP/39/39226.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DCLP/39/39226.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DCLP/754_754102.xml` | 9 KB | [DCLP/754/754102.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DCLP/754/754102.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DCLP/999_999926.xml` | 7 KB | [DCLP/999/999926.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DCLP/999/999926.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DDbDP/0_1.xml` | 8 KB | [DDbDP/0/1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DDbDP/0/1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DDbDP/203_203161.xml` | 3 KB | [DDbDP/203/203161.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DDbDP/203/203161.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DDbDP/47_47000.xml` | 3 KB | [DDbDP/47/47000.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DDbDP/47/47000.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DDbDP/80_80000.xml` | 3 KB | [DDbDP/80/80000.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DDbDP/80/80000.xml) @ `2ccd42bf1c74` |
| `bench/papyri/DDbDP/999_999484.xml` | 5 KB | [DDbDP/999/999484.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/DDbDP/999/999484.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_meta_EpiDoc/HGV1_1.xml` | 5 KB | [HGV_meta_EpiDoc/HGV1/1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_meta_EpiDoc/HGV1/1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_meta_EpiDoc/HGV204_203161.xml` | 5 KB | [HGV_meta_EpiDoc/HGV204/203161.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_meta_EpiDoc/HGV204/203161.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_meta_EpiDoc/HGV47_46345.xml` | 4 KB | [HGV_meta_EpiDoc/HGV47/46345.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_meta_EpiDoc/HGV47/46345.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_meta_EpiDoc/HGV8_7125.xml` | 4 KB | [HGV_meta_EpiDoc/HGV8/7125.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_meta_EpiDoc/HGV8/7125.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_meta_EpiDoc/HGV999_998048.xml` | 3 KB | [HGV_meta_EpiDoc/HGV999/998048.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_meta_EpiDoc/HGV999/998048.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_metadata/XML_dump_Glossary.xml` | 1.0 MB | [HGV_metadata/XML_dump/Glossary.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_metadata/XML_dump/Glossary.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_metadata/XML_dump_HGVddbStubs.xml` | 1.4 MB | [HGV_metadata/XML_dump/HGVddbStubs.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_metadata/XML_dump/HGVddbStubs.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_metadata/XML_dump_HGVentry.xml` | 157 KB | [HGV_metadata/XML_dump/HGVentry.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_metadata/XML_dump/HGVentry.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_metadata/XML_dump_edNumbersFullxmlresult.xml` | 968 KB | [HGV_metadata/XML_dump/edNumbersFullxmlresult.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_metadata/XML_dump/edNumbersFullxmlresult.xml) @ `2ccd42bf1c74` |
| `bench/papyri/HGV_metadata/XML_dump_nomeList.xml` | 3 KB | [HGV_metadata/XML_dump/nomeList.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/HGV_metadata/XML_dump/nomeList.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Historical/aegyptus_103_69_1.xml` | 9 KB | [Historical/aegyptus/103/69_1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Historical/aegyptus/103/69_1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Historical/o.theb.taxes_1_102.xml` | 2 KB | [Historical/o.theb.taxes/1/102.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Historical/o.theb.taxes/1/102.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Historical/p.fuad.i.univ_10.xml` | 6 KB | [Historical/p.fuad.i.univ/10.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Historical/p.fuad.i.univ/10.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Historical/p.petr.2ed_1.xml` | 24 KB | [Historical/p.petr.2ed/1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Historical/p.petr.2ed/1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Historical/zrg_139_5.xml` | 11 KB | [Historical/zrg/139/5.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Historical/zrg/139/5.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Translations/0_53-1.xml` | 4 KB | [Translations/0/53-1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Translations/0/53-1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Translations/206_20622a-1.xml` | 5 KB | [Translations/206/20622a-1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Translations/206/20622a-1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Translations/43_43006-1.xml` | 2 KB | [Translations/43/43006-1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Translations/43/43006-1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Translations/754_754089-1.xml` | 13 KB | [Translations/754/754089-1.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Translations/754/754089-1.xml) @ `2ccd42bf1c74` |
| `bench/papyri/Translations/glossary.xml` | 87 KB | [Translations/glossary.xml](https://github.com/papyri/idp.data/blob/2ccd42bf1c7485d56c7511ac797a082533d8ecee/Translations/glossary.xml) @ `2ccd42bf1c74` |

## Text Creation Partnership

2 file(s), 375 KB. Licence: CC0 1.0. Source: [textcreationpartnership/A00001](https://github.com/textcreationpartnership/A00001), [textcreationpartnership/A01185](https://github.com/textcreationpartnership/A01185).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/tcp/A00001.xml` | 177 KB | [A00001.xml](https://github.com/textcreationpartnership/A00001/blob/8b2f32f2e3e603e1e7a668782a4981a5257d9043/A00001.xml) @ `8b2f32f2e3e6` |
| `bench/tcp/A01185.xml` | 198 KB | [A01185.xml](https://github.com/textcreationpartnership/A01185/blob/0aed25a06e11302be046cef079d89e205508f25b/A01185.xml) @ `0aed25a06e11` |

## Perseus

1 file(s), 676 KB. Licence: CC BY-SA 4.0. Source: [PerseusDL/canonical-latinLit](https://github.com/PerseusDL/canonical-latinLit).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/perseus/phi0448.phi001.perseus-lat2.xml` | 676 KB | [data/phi0448/phi001/phi0448.phi001.perseus-lat2.xml](https://github.com/PerseusDL/canonical-latinLit/blob/128a05afa9cc7179ab18f8d05f78a9f1dd50e415/data/phi0448/phi001/phi0448.phi001.perseus-lat2.xml) @ `128a05afa9cc` |

## First1KGreek

1 file(s), 589 KB. Licence: CC BY-SA 4.0. Source: [OpenGreekAndLatin/First1KGreek](https://github.com/OpenGreekAndLatin/First1KGreek).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/first1kgreek/tlg0527.tlg001.1st1K-grc1.xml` | 589 KB | [data/tlg0527/tlg001/tlg0527.tlg001.1st1K-grc1.xml](https://github.com/OpenGreekAndLatin/First1KGreek/blob/03776b39f4047c5cff06f5296fae4b2bae4b08fb/data/tlg0527/tlg001/tlg0527.tlg001.1st1K-grc1.xml) @ `03776b39f404` |

## I.Sicily

3 file(s), 40 KB. Licence: CC BY 4.0. Source: [ISicily/ISicily](https://github.com/ISicily/ISicily).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `docs/isicily/ISic000001.xml` | 15 KB | [inscriptions/ISic000001.xml](https://github.com/ISicily/ISicily/blob/572d7f5ee1cbb62087bfed3c46ffc7cab6cb386c/inscriptions/ISic000001.xml) @ `572d7f5ee1cb` |
| `docs/isicily/ISic003039.xml` | 13 KB | [inscriptions/ISic003039.xml](https://github.com/ISicily/ISicily/blob/572d7f5ee1cbb62087bfed3c46ffc7cab6cb386c/inscriptions/ISic003039.xml) @ `572d7f5ee1cb` |
| `docs/isicily/ISic030173.xml` | 12 KB | [inscriptions/ISic030173.xml](https://github.com/ISicily/ISicily/blob/572d7f5ee1cbb62087bfed3c46ffc7cab6cb386c/inscriptions/ISic030173.xml) @ `572d7f5ee1cb` |

## DraCor (GerDraCor)

6 file(s), 3.9 MB. Licence: CC0 1.0. Source: [dracor-org/gerdracor](https://github.com/dracor-org/gerdracor).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/gerdracor/goethe-faust-der-tragoedie-zweiter-teil.xml` | 630 KB | [tei/goethe-faust-der-tragoedie-zweiter-teil.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/goethe-faust-der-tragoedie-zweiter-teil.xml) @ `7a7a3147b57a` |
| `bench/gerdracor/kraus-die-letzten-tage-der-menschheit.xml` | 2.2 MB | [tei/kraus-die-letzten-tage-der-menschheit.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/kraus-die-letzten-tage-der-menschheit.xml) @ `7a7a3147b57a` |
| `bench/gerdracor/lessing-nathan-der-weise.xml` | 432 KB | [tei/lessing-nathan-der-weise.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/lessing-nathan-der-weise.xml) @ `7a7a3147b57a` |
| `docs/gerdracor/achat-ein-april-scherz.xml` | 100 KB | [tei/achat-ein-april-scherz.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/achat-ein-april-scherz.xml) @ `7a7a3147b57a` |
| `docs/gerdracor/alberti-brot.xml` | 299 KB | [tei/alberti-brot.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/alberti-brot.xml) @ `7a7a3147b57a` |
| `docs/gerdracor/ebner-eschenbach-das-gestaendnis.xml` | 284 KB | [tei/ebner-eschenbach-das-gestaendnis.xml](https://github.com/dracor-org/gerdracor/blob/7a7a3147b57aed70393facd12cacd69d05f58091/tei/ebner-eschenbach-das-gestaendnis.xml) @ `7a7a3147b57a` |

## ELTeC

2 file(s), 2.1 MB. Licence: CC BY 4.0 (markup). Source: [COST-ELTeC/ELTeC-eng](https://github.com/COST-ELTeC/ELTeC-eng).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/eltec/ENG18400_Trollope.xml` | 1.2 MB | [level1/ENG18400_Trollope.xml](https://github.com/COST-ELTeC/ELTeC-eng/blob/3db3b6738a836f0e1b276b6c9703e42f4297f978/level1/ENG18400_Trollope.xml) @ `3db3b6738a83` |
| `bench/eltec/ENG18471_Bronte.xml` | 896 KB | [level1/ENG18471_Bronte.xml](https://github.com/COST-ELTeC/ELTeC-eng/blob/3db3b6738a836f0e1b276b6c9703e42f4297f978/level1/ENG18471_Bronte.xml) @ `3db3b6738a83` |

## ParlaMint

2 file(s), 81 KB. Licence: CC BY 4.0. Source: [clarin-eric/ParlaMint](https://github.com/clarin-eric/ParlaMint).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/parlamint/ParlaMint-GB_2017-09-07-commons.ana.xml` | 74 KB | [Samples/ParlaMint-GB/2017/ParlaMint-GB_2017-09-07-commons.ana.xml](https://github.com/clarin-eric/ParlaMint/blob/d5e880fbbd173570ced8b5683bacc0043b326a90/Samples/ParlaMint-GB/2017/ParlaMint-GB_2017-09-07-commons.ana.xml) @ `d5e880fbbd17` |
| `bench/parlamint/ParlaMint-GB_2017-09-07-commons.xml` | 7 KB | [Samples/ParlaMint-GB/2017/ParlaMint-GB_2017-09-07-commons.xml](https://github.com/clarin-eric/ParlaMint/blob/d5e880fbbd173570ced8b5683bacc0043b326a90/Samples/ParlaMint-GB/2017/ParlaMint-GB_2017-09-07-commons.xml) @ `d5e880fbbd17` |

## SARIT

2 file(s), 2.9 MB. Licence: CC BY-SA 3.0. Source: [sarit/SARIT-corpus](https://github.com/sarit/SARIT-corpus).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/sarit/astangahrdayasamhita.xml` | 2.9 MB | [astangahrdayasamhita.xml](https://github.com/sarit/SARIT-corpus/blob/1eac9ee0b055c8d11147edac4a75c76008ccc363/astangahrdayasamhita.xml) @ `1eac9ee0b055` |
| `bench/sarit/astavakragita.xml` | 67 KB | [astavakragita.xml](https://github.com/sarit/SARIT-corpus/blob/1eac9ee0b055c8d11147edac4a75c76008ccc363/astavakragita.xml) @ `1eac9ee0b055` |

## Arthur Schnitzler

3 file(s), 2.0 MB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [arthur-schnitzler/schnitzler-briefe-data](https://github.com/arthur-schnitzler/schnitzler-briefe-data), [arthur-schnitzler/schnitzler-tagebuch-data](https://github.com/arthur-schnitzler/schnitzler-tagebuch-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/schnitzler/editions_L01903.xml` | 20 KB | [data/editions/L01903.xml](https://github.com/arthur-schnitzler/schnitzler-briefe-data/blob/e058a5110ea5382b531c419a05b7249b191ea1cd/data/editions/L01903.xml) @ `e058a5110ea5` |
| `bench/schnitzler/editions_entry__1879-07-08.xml` | 10 KB | [editions/entry__1879-07-08.xml](https://github.com/arthur-schnitzler/schnitzler-tagebuch-data/blob/e6b008a7e74ef3cb0f3a21cb3f5716b0f261e48d/editions/entry__1879-07-08.xml) @ `e6b008a7e74e` |
| `bench/schnitzler/indices_listbibl.xml` | 2.0 MB | [data/indices/listbibl.xml](https://github.com/arthur-schnitzler/schnitzler-briefe-data/blob/e058a5110ea5382b531c419a05b7249b191ea1cd/data/indices/listbibl.xml) @ `e058a5110ea5` |

## Beta maṣāḥǝft

3 file(s), 2.2 MB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [BetaMasaheft/Manuscripts](https://github.com/BetaMasaheft/Manuscripts).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/betamasaheft/EMML1529.xml` | 5 KB | [EMML/1001-2000/EMML1529.xml](https://github.com/BetaMasaheft/Manuscripts/blob/b7fc6d4dae8e1645f13c673b6683f3a94c5e556c/EMML/1001-2000/EMML1529.xml) @ `b7fc6d4dae8e` |
| `bench/betamasaheft/ESam019.xml` | 2.2 MB | [ES/ESam019.xml](https://github.com/BetaMasaheft/Manuscripts/blob/b7fc6d4dae8e1645f13c673b6683f3a94c5e556c/ES/ESam019.xml) @ `b7fc6d4dae8e` |
| `bench/betamasaheft/MASE002.xml` | 8 KB | [MearigoSellase/MASE002.xml](https://github.com/BetaMasaheft/Manuscripts/blob/b7fc6d4dae8e1645f13c673b6683f3a94c5e556c/MearigoSellase/MASE002.xml) @ `b7fc6d4dae8e` |

## CBETA

3 file(s), 3.1 MB. Licence: https://www.cbeta.org/copyright.php. Source: [cbeta-org/xml-p5](https://github.com/cbeta-org/xml-p5).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/cbeta/A114n1504.xml` | 114 KB | [A/A114/A114n1504.xml](https://github.com/cbeta-org/xml-p5/blob/dbdea41071e1e260ad84b72faefd4587333cf76d/A/A114/A114n1504.xml) @ `dbdea41071e1` |
| `bench/cbeta/T39n1796.xml` | 2.9 MB | [T/T39/T39n1796.xml](https://github.com/cbeta-org/xml-p5/blob/dbdea41071e1e260ad84b72faefd4587333cf76d/T/T39/T39n1796.xml) @ `dbdea41071e1` |
| `bench/cbeta/T44n1847.xml` | 114 KB | [T/T44/T44n1847.xml](https://github.com/cbeta-org/xml-p5/blob/dbdea41071e1e260ad84b72faefd4587333cf76d/T/T44/T44n1847.xml) @ `dbdea41071e1` |

## Coptic Scriptorium

3 file(s), 646 KB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [CopticScriptorium/corpora](https://github.com/CopticScriptorium/corpora).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/coptic/01_Matthew_04.xml` | 47 KB | [bohairic.nt/bohairic.nt_TEI/01_Matthew_04.xml](https://github.com/CopticScriptorium/corpora/blob/3ac067f1709a0012daf39ea8da2fac79980176a5/bohairic.nt/bohairic.nt_TEI/01_Matthew_04.xml) @ `3ac067f1709a` |
| `bench/coptic/04_Numeri_05.xml` | 68 KB | [bohairic.ot/bohairic.ot_TEI/04_Numeri_05.xml](https://github.com/CopticScriptorium/corpora/blob/3ac067f1709a0012daf39ea8da2fac79980176a5/bohairic.ot/bohairic.ot_TEI/04_Numeri_05.xml) @ `3ac067f1709a` |
| `bench/coptic/penitence.01.xml` | 532 KB | [john-constantinople/john.constantinople_TEI/penitence.01.xml](https://github.com/CopticScriptorium/corpora/blob/3ac067f1709a0012daf39ea8da2fac79980176a5/john-constantinople/john.constantinople_TEI/penitence.01.xml) @ `3ac067f1709a` |

## correspSearch (CMIF)

3 file(s), 1.4 MB. Licence: https://creativecommons.org/publicdomain/zero/1.0/. Source: [correspSearch/csStorage](https://github.com/correspSearch/csStorage).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/cmif/avhumboldt_Jugendbriefe.xml` | 233 KB | [avhumboldt/Jugendbriefe.xml](https://github.com/correspSearch/csStorage/blob/cb6437ab7f6232aab192c414d79c2268ccfd7e55/avhumboldt/Jugendbriefe.xml) @ `cb6437ab7f62` |
| `bench/cmif/feuerbach_1.xml` | 119 KB | [feuerbach/1.xml](https://github.com/correspSearch/csStorage/blob/cb6437ab7f6232aab192c414d79c2268ccfd7e55/feuerbach/1.xml) @ `cb6437ab7f62` |
| `bench/cmif/forster_gesamt.xml` | 1.1 MB | [forster/gesamt.xml](https://github.com/correspSearch/csStorage/blob/cb6437ab7f6232aab192c414d79c2268ccfd7e55/forster/gesamt.xml) @ `cb6437ab7f62` |

## FreeDict

3 file(s), 1.7 MB. Licence: https://www.gnu.org/licenses/gpl-2.0.html. Source: [freedict/fd-dictionaries](https://github.com/freedict/fd-dictionaries).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/freedict/afr-deu.xml` | 1.3 MB | [afr-deu/afr-deu.tei](https://github.com/freedict/fd-dictionaries/blob/5bdceeac8d0dba3298c1bebe734f60d54dad30f7/afr-deu/afr-deu.tei) @ `5bdceeac8d0d` |
| `bench/freedict/tur-deu.xml` | 299 KB | [tur-deu/tur-deu.tei](https://github.com/freedict/fd-dictionaries/blob/5bdceeac8d0dba3298c1bebe734f60d54dad30f7/tur-deu/tur-deu.tei) @ `5bdceeac8d0d` |
| `bench/freedict/wol-fra.xml` | 87 KB | [wol-fra/wol-fra.tei](https://github.com/freedict/fd-dictionaries/blob/5bdceeac8d0dba3298c1bebe734f60d54dad30f7/wol-fra/wol-fra.tei) @ `5bdceeac8d0d` |

## Legal Kraus

3 file(s), 1.9 MB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [karl-kraus/legalkraus-data](https://github.com/karl-kraus/legalkraus-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/legalkraus/D_000015-006-000.xml` | 12 KB | [data/editions/D_000015-006-000.xml](https://github.com/karl-kraus/legalkraus-data/blob/bc279c5e9a2b20be2777a5558f1348c18b2da79c/data/editions/D_000015-006-000.xml) @ `bc279c5e9a2b` |
| `bench/legalkraus/D_000134-018-000.xml` | 9 KB | [data/editions/D_000134-018-000.xml](https://github.com/karl-kraus/legalkraus-data/blob/bc279c5e9a2b20be2777a5558f1348c18b2da79c/data/editions/D_000134-018-000.xml) @ `bc279c5e9a2b` |
| `bench/legalkraus/boehm_vol2.xml` | 1.9 MB | [data/boehm_tei/boehm_vol2.xml](https://github.com/karl-kraus/legalkraus-data/blob/bc279c5e9a2b20be2777a5558f1348c18b2da79c/data/boehm_tei/boehm_vol2.xml) @ `bc279c5e9a2b` |

## Patrologia Latina

3 file(s), 3.0 MB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [OpenGreekAndLatin/patrologia_latina-dev](https://github.com/OpenGreekAndLatin/patrologia_latina-dev).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/patrologia-latina/stoa0040.stoa041a.opp-lat1.xml` | 44 KB | [data/stoa0040/stoa041a/stoa0040.stoa041a.opp-lat1.xml](https://github.com/OpenGreekAndLatin/patrologia_latina-dev/blob/8c651f859f821f0868755596f1a513946ca25350/data/stoa0040/stoa041a/stoa0040.stoa041a.opp-lat1.xml) @ `8c651f859f82` |
| `bench/patrologia-latina/stoa0149b.stoa001.opp-lat2.xml` | 2.8 MB | [data/stoa0149b/stoa001/stoa0149b.stoa001.opp-lat2.xml](https://github.com/OpenGreekAndLatin/patrologia_latina-dev/blob/8c651f859f821f0868755596f1a513946ca25350/data/stoa0149b/stoa001/stoa0149b.stoa001.opp-lat2.xml) @ `8c651f859f82` |
| `bench/patrologia-latina/tmp1186.tmp1186.opp-lat1.xml` | 148 KB | [data/tmp1186/tmp1186/tmp1186.tmp1186.opp-lat1.xml](https://github.com/OpenGreekAndLatin/patrologia_latina-dev/blob/8c651f859f821f0868755596f1a513946ca25350/data/tmp1186/tmp1186/tmp1186.tmp1186.opp-lat1.xml) @ `8c651f859f82` |

## Syriaca.org

3 file(s), 1.8 MB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [srophe/srophe-app-data](https://github.com/srophe/srophe-app-data), [srophe/syriac-corpus](https://github.com/srophe/syriac-corpus).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/syriaca/13.xml` | 43 KB | [data/persons/tei/13.xml](https://github.com/srophe/srophe-app-data/blob/fefe80f5aabebeef5ee3cef66d3d344cf573f173/data/persons/tei/13.xml) @ `fefe80f5aabe` |
| `bench/syriaca/391.xml` | 1.7 MB | [data/tei/391.xml](https://github.com/srophe/syriac-corpus/blob/833adc148cc356a6c70c16f81b22df9188df717a/data/tei/391.xml) @ `833adc148cc3` |
| `bench/syriaca/78.xml` | 44 KB | [data/places/tei/78.xml](https://github.com/srophe/srophe-app-data/blob/fefe80f5aabebeef5ee3cef66d3d344cf573f173/data/places/tei/78.xml) @ `fefe80f5aabe` |

## Tolstoy Digital

3 file(s), 2.2 MB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [tolstoydigital/TEI](https://github.com/tolstoydigital/TEI).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/tolstoy/v13_198_541_Varianty_k_tomu_pervomu_Vojny_i_mira.xml` | 2.2 MB | [texts/works/v13_198_541_Varianty_k_tomu_pervomu_Vojny_i_mira.xml](https://github.com/tolstoydigital/TEI/blob/91b92a6b6fe77abbf2302e5654c02dc27ecfdb54/texts/works/v13_198_541_Varianty_k_tomu_pervomu_Vojny_i_mira.xml) @ `91b92a6b6fe7` |
| `bench/tolstoy/v61_208_E_V_Liprandi.xml` | 14 KB | [texts/letters/v61_208_E_V_Liprandi.xml](https://github.com/tolstoydigital/TEI/blob/91b92a6b6fe77abbf2302e5654c02dc27ecfdb54/texts/letters/v61_208_E_V_Liprandi.xml) @ `91b92a6b6fe7` |
| `bench/tolstoy/v79_283_V_A_Molochnikovu.xml` | 10 KB | [texts/letters/v79_283_V_A_Molochnikovu.xml](https://github.com/tolstoydigital/TEI/blob/91b92a6b6fe77abbf2302e5654c02dc27ecfdb54/texts/letters/v79_283_V_A_Molochnikovu.xml) @ `91b92a6b6fe7` |

## 84000

3 file(s), 2.7 MB. Licence: https://creativecommons.org/licenses/by-nc-nd/3.0/ (CC BY-NC-ND 3.0, README and TEI header). Source: [84000/data-tei](https://github.com/84000/data-tei).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/84000/055-001_toh127_king_of_samadhis.xml` | 2.5 MB | [translations/kangyur/translations/055-001_toh127_king_of_samadhis.xml](https://github.com/84000/data-tei/blob/bdfc81c9aecdfaf93e084ab64a885577de773046/translations/kangyur/translations/055-001_toh127_king_of_samadhis.xml) @ `bdfc81c9aecd` |
| `bench/84000/061-010_toh190-the_sutra_of_the_prophecy_concerning_strivivarta.xml` | 208 KB | [translations/kangyur/translations/061-010_toh190-the_sutra_of_the_prophecy_concerning_strivivarta.xml](https://github.com/84000/data-tei/blob/bdfc81c9aecdfaf93e084ab64a885577de773046/translations/kangyur/translations/061-010_toh190-the_sutra_of_the_prophecy_concerning_strivivarta.xml) @ `bdfc81c9aecd` |
| `bench/84000/knowledgebase_purna.xml` | 14 KB | [knowledgebase/purna.xml](https://github.com/84000/data-tei/blob/bdfc81c9aecdfaf93e084ab64a885577de773046/knowledgebase/purna.xml) @ `bdfc81c9aecd` |

## Akademieprotokolle (ÖAW)

3 file(s), 1.1 MB. Licence: none stated (README, no LICENSE file, no availability in the TEI header). Source: [acdh-oeaw/akademie-data](https://github.com/acdh-oeaw/akademie-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/akademie/A_0005.xml` | 1009 KB | [data/editions/A_0005.xml](https://github.com/acdh-oeaw/akademie-data/blob/6fe4b3e63ba46e3eabb3d495331b0042a05799f1/data/editions/A_0005.xml) @ `6fe4b3e63ba4` |
| `bench/akademie/A_0099.xml` | 35 KB | [data/editions/A_0099.xml](https://github.com/acdh-oeaw/akademie-data/blob/6fe4b3e63ba46e3eabb3d495331b0042a05799f1/data/editions/A_0099.xml) @ `6fe4b3e63ba4` |
| `bench/akademie/C_0012.xml` | 118 KB | [data/editions/C_0012.xml](https://github.com/acdh-oeaw/akademie-data/blob/6fe4b3e63ba46e3eabb3d495331b0042a05799f1/data/editions/C_0012.xml) @ `6fe4b3e63ba4` |

## Auden Musulin Papers

3 file(s), 355 KB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [Auden-Musulin-Papers/amp-data](https://github.com/Auden-Musulin-Papers/amp-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/amp/amp-transcript__0005.xml` | 13 KB | [data/editions/amp-transcript__0005.xml](https://github.com/Auden-Musulin-Papers/amp-data/blob/289a52de61aef0b6354e3c8298173bf1f889feb2/data/editions/amp-transcript__0005.xml) @ `289a52de61ae` |
| `bench/amp/amp-transcript__0051.xml` | 17 KB | [data/editions/amp-transcript__0051.xml](https://github.com/Auden-Musulin-Papers/amp-data/blob/289a52de61aef0b6354e3c8298173bf1f889feb2/data/editions/amp-transcript__0051.xml) @ `289a52de61ae` |
| `bench/amp/amp-transcript__0062.xml` | 325 KB | [data/editions/amp-transcript__0062.xml](https://github.com/Auden-Musulin-Papers/amp-data/blob/289a52de61aef0b6354e3c8298173bf1f889feb2/data/editions/amp-transcript__0062.xml) @ `289a52de61ae` |

## Bodleian medieval manuscripts

3 file(s), 1.7 MB. Licence: none stated (README, no LICENSE file; TEI header availability gives access conditions only). Source: [bodleian/medieval-mss](https://github.com/bodleian/medieval-mss).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/bodleian/MS_Laud_Misc_541.xml` | 13 KB | [collections/Laud_Misc/MS_Laud_Misc_541.xml](https://github.com/bodleian/medieval-mss/blob/4f445a5b60bec0f1bebfa6d3a0efdf9bd7a27022/collections/Laud_Misc/MS_Laud_Misc_541.xml) @ `4f445a5b60be` |
| `bench/bodleian/Merton_College_MS_129.xml` | 19 KB | [collections/Merton/Merton_College_MS_129.xml](https://github.com/bodleian/medieval-mss/blob/4f445a5b60bec0f1bebfa6d3a0efdf9bd7a27022/collections/Merton/Merton_College_MS_129.xml) @ `4f445a5b60be` |
| `bench/bodleian/places.xml` | 1.6 MB | [places.xml](https://github.com/bodleian/medieval-mss/blob/4f445a5b60bec0f1bebfa6d3a0efdf9bd7a27022/places.xml) @ `4f445a5b60be` |

## Bullinger Digital

3 file(s), 1.6 MB. Licence: https://creativecommons.org/licenses/by-nc-nd/3.0/ch/ (CC BY-NC-ND 3.0 CH, README). Source: [stazh/bullinger-korpus-tei](https://github.com/stazh/bullinger-korpus-tei).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/bullinger/11521.xml` | 13 KB | [data/letters/11521.xml](https://github.com/stazh/bullinger-korpus-tei/blob/dc9fb5290c210023a6a9182e8e3102c1f6dff734/data/letters/11521.xml) @ `dc9fb5290c21` |
| `bench/bullinger/435.xml` | 30 KB | [data/letters/435.xml](https://github.com/stazh/bullinger-korpus-tei/blob/dc9fb5290c210023a6a9182e8e3102c1f6dff734/data/letters/435.xml) @ `dc9fb5290c21` |
| `bench/bullinger/7538.xml` | 1.5 MB | [data/letters/7538.xml](https://github.com/stazh/bullinger-korpus-tei/blob/dc9fb5290c210023a6a9182e8e3102c1f6dff734/data/letters/7538.xml) @ `dc9fb5290c21` |

## CLiGS textbox

3 file(s), 882 KB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [cligs/textbox](https://github.com/cligs/textbox).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/cligs/it0129.xml` | 26 KB | [italian/short19-20/tei/it0129.xml](https://github.com/cligs/textbox/blob/1b91e940475c8a7e866457a3953fed5edb0a29f1/italian/short19-20/tei/it0129.xml) @ `1b91e940475c` |
| `bench/cligs/nh0053.xml` | 626 KB | [spanish/novela-hispanoamericana/tei/nh0053.xml](https://github.com/cligs/textbox/blob/1b91e940475c8a7e866457a3953fed5edb0a29f1/spanish/novela-hispanoamericana/tei/nh0053.xml) @ `1b91e940475c` |
| `bench/cligs/rp0005.xml` | 230 KB | [portuguese/romances19/tei/rp0005.xml](https://github.com/cligs/textbox/blob/1b91e940475c8a7e866457a3953fed5edb0a29f1/portuguese/romances19/tei/rp0005.xml) @ `1b91e940475c` |

## conha19

3 file(s), 804 KB. Licence: https://creativecommons.org/publicdomain/mark/1.0/. Source: [cligs/conha19](https://github.com/cligs/conha19).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/conha19/nh0005.xml` | 150 KB | [tei/nh0005.xml](https://github.com/cligs/conha19/blob/d5def0a3b5900fd37172714d1b8b91bfce0a267c/tei/nh0005.xml) @ `d5def0a3b590` |
| `bench/conha19/nh0178.xml` | 134 KB | [tei/nh0178.xml](https://github.com/cligs/conha19/blob/d5def0a3b5900fd37172714d1b8b91bfce0a267c/tei/nh0178.xml) @ `d5def0a3b590` |
| `bench/conha19/nh0215.xml` | 520 KB | [tei/nh0215.xml](https://github.com/cligs/conha19/blob/d5def0a3b5900fd37172714d1b8b91bfce0a267c/tei/nh0215.xml) @ `d5def0a3b590` |

## CroALa

3 file(s), 1.8 MB. Licence: https://creativecommons.org/publicdomain/zero/1.0/. Source: [nevenjovanovic/croatiae-auctores-latini-textus](https://github.com/nevenjovanovic/croatiae-auctores-latini-textus).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/croala/begnius-s-corvatiae.xml` | 18 KB | [txts/begnius-s-corvatiae.xml](https://github.com/nevenjovanovic/croatiae-auctores-latini-textus/blob/729f0ee28de9d35c94009e877e500fae5e971299/txts/begnius-s-corvatiae.xml) @ `729f0ee28de9` |
| `bench/croala/marul-mar-inst.xml` | 1.8 MB | [txts/marul-mar-inst.xml](https://github.com/nevenjovanovic/croatiae-auctores-latini-textus/blob/729f0ee28de9d35c94009e877e500fae5e971299/txts/marul-mar-inst.xml) @ `729f0ee28de9` |
| `bench/croala/vrancic-a-epist-1540-03-07.xml` | 8 KB | [txts/vrancic-a-epist-1540-03-07.xml](https://github.com/nevenjovanovic/croatiae-auctores-latini-textus/blob/729f0ee28de9d35c94009e877e500fae5e971299/txts/vrancic-a-epist-1540-03-07.xml) @ `729f0ee28de9` |

## CSEL, Open Greek and Latin

3 file(s), 844 KB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [OpenGreekAndLatin/csel-dev](https://github.com/OpenGreekAndLatin/csel-dev).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/csel/stoa0040.stoa033a.opp-lat1.xml` | 770 KB | [data/stoa0040/stoa033a/stoa0040.stoa033a.opp-lat1.xml](https://github.com/OpenGreekAndLatin/csel-dev/blob/2813e3a52b4206df0324299e5110e5bd0d95cee4/data/stoa0040/stoa033a/stoa0040.stoa033a.opp-lat1.xml) @ `2813e3a52b42` |
| `bench/csel/stoa0117.stoa001.opp-lat1.xml` | 59 KB | [data/stoa0117/stoa001/stoa0117.stoa001.opp-lat1.xml](https://github.com/OpenGreekAndLatin/csel-dev/blob/2813e3a52b4206df0324299e5110e5bd0d95cee4/data/stoa0117/stoa001/stoa0117.stoa001.opp-lat1.xml) @ `2813e3a52b42` |
| `bench/csel/stoa0252.stoa006.opp-lat2.xml` | 15 KB | [data/stoa0252/stoa006/stoa0252.stoa006.opp-lat2.xml](https://github.com/OpenGreekAndLatin/csel-dev/blob/2813e3a52b4206df0324299e5110e5bd0d95cee4/data/stoa0252/stoa006/stoa0252.stoa006.opp-lat2.xml) @ `2813e3a52b42` |

## DARIAH-ERIC lexical resources

3 file(s), 2.5 MB. Licence: https://creativecommons.org/licenses/by/3.0/. Source: [DARIAH-ERIC/lexicalresources](https://github.com/DARIAH-ERIC/lexicalresources).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/lexicalresources/Corpus2Dictionary_bjarnar.xml` | 23 KB | [Resources/Corpus2Dictionary/Entries/bjarnar.xml](https://github.com/DARIAH-ERIC/lexicalresources/blob/b038b2b5b8ebb7fef785eab11bb030dce23cebaa/Resources/Corpus2Dictionary/Entries/bjarnar.xml) @ `b038b2b5b8eb` |
| `bench/lexicalresources/Corpus2Dictionary_vera.xml` | 2.4 MB | [Resources/Corpus2Dictionary/Entries/vera.xml](https://github.com/DARIAH-ERIC/lexicalresources/blob/b038b2b5b8ebb7fef785eab11bb030dce23cebaa/Resources/Corpus2Dictionary/Entries/vera.xml) @ `b038b2b5b8eb` |
| `bench/lexicalresources/Mueller-1878.tei.xml` | 22 KB | [Events/LexMC2017/Data/Etymology/Mueller-1878.tei.xml](https://github.com/DARIAH-ERIC/lexicalresources/blob/b038b2b5b8ebb7fef785eab11bb030dce23cebaa/Events/LexMC2017/Data/Etymology/Mueller-1878.tei.xml) @ `b038b2b5b8eb` |

## Deutsches Lyrik Korpus (DLK)

3 file(s), 271 KB. Licence: none stated (README, no LICENSE file, no availability in the TEI header). Source: [tnhaider/DLK](https://github.com/tnhaider/DLK).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/dlk/textgrid.poem.53041-Lohenstein-Venus.xml` | 263 KB | [DLK/tei/tei_plain/textgrid.poem.53041-Lohenstein%2C_Daniel_Casper_von-Venus.tei.xml](https://github.com/tnhaider/DLK/blob/ef0b62054a301a2d4d15eca586c4b052a18b13ca/DLK/tei/tei_plain/textgrid.poem.53041-Lohenstein%2C_Daniel_Casper_von-Venus.tei.xml) @ `ef0b62054a30` |
| `bench/dlk/textgrid.poem.55497-Goethe-77.xml` | 3 KB | [DLK/tei/tei_plain/textgrid.poem.55497-Goethe%2C_Johann_Wolfgang-77.tei.xml](https://github.com/tnhaider/DLK/blob/ef0b62054a301a2d4d15eca586c4b052a18b13ca/DLK/tei/tei_plain/textgrid.poem.55497-Goethe%2C_Johann_Wolfgang-77.tei.xml) @ `ef0b62054a30` |
| `bench/dlk/textgrid.poem.63671-Heyse-Kehraus.xml` | 5 KB | [DLK/tei/tei_plain/textgrid.poem.63671-Heyse%2C_Paul-Kehraus.tei.xml](https://github.com/tnhaider/DLK/blob/ef0b62054a301a2d4d15eca586c4b052a18b13ca/DLK/tei/tei_plain/textgrid.poem.63671-Heyse%2C_Paul-Kehraus.tei.xml) @ `ef0b62054a30` |

## DHARMA

3 file(s), 933 KB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [erc-dharma/tfa-pallava-epigraphy](https://github.com/erc-dharma/tfa-pallava-epigraphy), [erc-dharma/tfa-sii-epigraphy](https://github.com/erc-dharma/tfa-sii-epigraphy), [erc-dharma/tfd-nusantara-philology](https://github.com/erc-dharma/tfd-nusantara-philology).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/dharma/DHARMA_CritEdKakavinSutasoma.xml` | 914 KB | [editions/DHARMA_CritEdKakavinSutasoma.xml](https://github.com/erc-dharma/tfd-nusantara-philology/blob/3a30b0e78f5e0bbe007852afe6abe2ca2bec7d81/editions/DHARMA_CritEdKakavinSutasoma.xml) @ `3a30b0e78f5e` |
| `bench/dharma/DHARMA_INSPallava00220.xml` | 9 KB | [DHARMA_INSPallava00220.xml](https://github.com/erc-dharma/tfa-pallava-epigraphy/blob/1b1b17c4df4990aba2c28101372e669fc9182d10/DHARMA_INSPallava00220.xml) @ `1b1b17c4df49` |
| `bench/dharma/DHARMA_INStfaSIIv08p0i0127.xml` | 9 KB | [DHARMA_INStfaSIIv08p0i0127.xml](https://github.com/erc-dharma/tfa-sii-epigraphy/blob/dfa95be6729f162c0e7b70eea15c51da94520ff4/DHARMA_INStfaSIIv08p0i0127.xml) @ `dfa95be6729f` |

## Digital Mitford

3 file(s), 619 KB. Licence: https://creativecommons.org/licenses/by-sa/3.0/ (TEI header). Source: [ebeshero/Mitford](https://github.com/ebeshero/Mitford).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/mitford/1819-03-02-MWebb.xml` | 12 KB | [Box_2019-05-17_Letters/1819-03-02-MWebb.xml](https://github.com/ebeshero/Mitford/blob/74b5dc485ac9bc5c856043b027209904acaa83ea/Box_2019-05-17_Letters/1819-03-02-MWebb.xml) @ `74b5dc485ac9` |
| `bench/mitford/1822-04-12-Elford.xml` | 25 KB | [Box_2019-05-17_Letters/1822-04-12-Elford.xml](https://github.com/ebeshero/Mitford/blob/74b5dc485ac9bc5c856043b027209904acaa83ea/Box_2019-05-17_Letters/1822-04-12-Elford.xml) @ `74b5dc485ac9` |
| `bench/mitford/Rienzi.xml` | 582 KB | [Box-2018-05-23-All/Rienzi.xml](https://github.com/ebeshero/Mitford/blob/74b5dc485ac9bc5c856043b027209904acaa83ea/Box-2018-05-23-All/Rienzi.xml) @ `74b5dc485ac9` |

## DISCO

3 file(s), 1.5 MB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [pruizf/disco](https://github.com/pruizf/disco).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/disco/disco159g_0307.xml` | 10 KB | [tei/15th-17th/per-sonnet/disco159g_0307.xml](https://github.com/pruizf/disco/blob/b626fefad22b2c8a0ecec8903cb6a465ebadb541/tei/15th-17th/per-sonnet/disco159g_0307.xml) @ `b626fefad22b` |
| `bench/disco/disco311n.xml` | 9 KB | [tei/19th/per-author/disco311n.xml](https://github.com/pruizf/disco/blob/b626fefad22b2c8a0ecec8903cb6a465ebadb541/tei/19th/per-author/disco311n.xml) @ `b626fefad22b` |
| `bench/disco/prosopography.xml` | 1.5 MB | [ancillary-files/prosopography.xml](https://github.com/pruizf/disco/blob/b626fefad22b2c8a0ecec8903cb6a465ebadb541/ancillary-files/prosopography.xml) @ `b626fefad22b` |

## ENCPOS (École des chartes)

3 file(s), 150 KB. Licence: https://creativecommons.org/licenses/by-nc-nd/3.0/fr/. Source: [chartes/encpos](https://github.com/chartes/encpos).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/encpos/ENCPOS_1890_05.xml` | 21 KB | [data/ENCPOS_1890/ENCPOS_1890_05.xml](https://github.com/chartes/encpos/blob/3fcebbb67fe35801f524f77c25c0e3cd896697d6/data/ENCPOS_1890/ENCPOS_1890_05.xml) @ `3fcebbb67fe3` |
| `bench/encpos/ENCPOS_2014_24.xml` | 6 KB | [data/ENCPOS_2014/ENCPOS_2014_24.xml](https://github.com/chartes/encpos/blob/3fcebbb67fe35801f524f77c25c0e3cd896697d6/data/ENCPOS_2014/ENCPOS_2014_24.xml) @ `3fcebbb67fe3` |
| `bench/encpos/ENC_1-1847-1865_PREV2.xml` | 123 KB | [data/1-1847-1865/ENC_1-1847-1865_PREV2.xml](https://github.com/chartes/encpos/blob/3fcebbb67fe35801f524f77c25c0e3cd896697d6/data/1-1847-1865/ENC_1-1847-1865_PREV2.xml) @ `3fcebbb67fe3` |

## Faust-Edition

3 file(s), 1.2 MB. Licence: https://creativecommons.org/licenses/by-nc-sa/4.0/. Source: [faustedition/faust-xml](https://github.com/faustedition/faust-xml).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/faust/gsa_391098.xml` | 1.2 MB | [xml/transcript/gsa/391098/391098.xml](https://github.com/faustedition/faust-xml/blob/502eca65120dd6189ceaf41d4e5017775e6e4677/xml/transcript/gsa/391098/391098.xml) @ `502eca65120d` |
| `bench/faust/gsa_391098_0200.xml` | 30 KB | [xml/transcript/gsa/391098/0200.xml](https://github.com/faustedition/faust-xml/blob/502eca65120dd6189ceaf41d4e5017775e6e4677/xml/transcript/gsa/391098/0200.xml) @ `502eca65120d` |
| `bench/faust/ub_bonn_S_863_II_0043.xml` | 25 KB | [xml/transcript/ub_bonn/S_863_II/0043.xml](https://github.com/faustedition/faust-xml/blob/502eca65120dd6189ceaf41d4e5017775e6e4677/xml/transcript/ub_bonn/S_863_II/0043.xml) @ `502eca65120d` |

## Fihrist

3 file(s), 501 KB. Licence: none stated (README, no LICENSE file; TEI header availability gives access conditions only). Source: [bodleian/fihrist-mss](https://github.com/bodleian/fihrist-mss).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/fihrist/british-library_Uk_Delhi_Persian_671.xml` | 8 KB | [collections/british%20library/Uk_Delhi_Persian_671.xml](https://github.com/bodleian/fihrist-mss/blob/2825c76d062344ab3a8553dd8ffa73360113851f/collections/british%20library/Uk_Delhi_Persian_671.xml) @ `2825c76d0623` |
| `bench/fihrist/manchester_Persian_MS_10.xml` | 486 KB | [collections/the%20university%20of%20manchester/Persian_MS_10.xml](https://github.com/bodleian/fihrist-mss/blob/2825c76d062344ab3a8553dd8ffa73360113851f/collections/the%20university%20of%20manchester/Persian_MS_10.xml) @ `2825c76d0623` |
| `bench/fihrist/oxford_MS_Marsh_223.xml` | 7 KB | [collections/oxford%20university/MS_Marsh_223.xml](https://github.com/bodleian/fihrist-mss/blob/2825c76d062344ab3a8553dd8ffa73360113851f/collections/oxford%20university/MS_Marsh_223.xml) @ `2825c76d0623` |

## Frankenstein Variorum

3 file(s), 2.0 MB. Licence: https://creativecommons.org/licenses/by-sa/3.0/ (TEI header; repository Unlicense). Source: [FrankensteinVariorum/fv-data](https://github.com/FrankensteinVariorum/fv-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/frankenstein/f1818_vol_1_chapter_ii.xml` | 45 KB | [2023-variorum-chapters/f1818_vol_1_chapter_ii.xml](https://github.com/FrankensteinVariorum/fv-data/blob/2debfb706fbe0c1cc2c90b5a75d41b5591aef9c4/2023-variorum-chapters/f1818_vol_1_chapter_ii.xml) @ `2debfb706fbe` |
| `bench/frankenstein/fThomas_vol_2_chapter_vi.xml` | 27 KB | [2023-variorum-chapters/fThomas_vol_2_chapter_vi.xml](https://github.com/FrankensteinVariorum/fv-data/blob/2debfb706fbe0c1cc2c90b5a75d41b5591aef9c4/2023-variorum-chapters/fThomas_vol_2_chapter_vi.xml) @ `2debfb706fbe` |
| `bench/frankenstein/spine_C08.xml` | 2.0 MB | [2023-standoff_Spine/spine_C08.xml](https://github.com/FrankensteinVariorum/fv-data/blob/2debfb706fbe0c1cc2c90b5a75d41b5591aef9c4/2023-standoff_Spine/spine_C08.xml) @ `2debfb706fbe` |

## FRUS (Office of the Historian)

3 file(s), 2.5 MB. Licence: public domain (US government work; README). Source: [HistoryAtState/frus](https://github.com/HistoryAtState/frus).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/frus/frus1877app.xml` | 1.2 MB | [volumes/frus1877app.xml](https://github.com/HistoryAtState/frus/blob/6170e4b32c1224002db2fe0c5d8d910597cddc2e/volumes/frus1877app.xml) @ `6170e4b32c12` |
| `bench/frus/frus1961-63v22-24mSupp.xml` | 3 KB | [volumes/frus1961-63v22-24mSupp.xml](https://github.com/HistoryAtState/frus/blob/6170e4b32c1224002db2fe0c5d8d910597cddc2e/volumes/frus1961-63v22-24mSupp.xml) @ `6170e4b32c12` |
| `bench/frus/frus1981-88v16.xml` | 1.2 MB | [volumes/frus1981-88v16.xml](https://github.com/HistoryAtState/frus/blob/6170e4b32c1224002db2fe0c5d8d910597cddc2e/volumes/frus1981-88v16.xml) @ `6170e4b32c12` |

## Genji monogatari (nakamura196)

3 file(s), 1.8 MB. Licence: https://creativecommons.org/licenses/by/4.0/. Source: [nakamura196/genji](https://github.com/nakamura196/genji).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/genji/aozora_16.xml` | 18 KB | [scripts/aozora/all/data/16.xml](https://github.com/nakamura196/genji/blob/3f0476e8ecdca97b8a96a0e8a32fb505a1b64650/scripts/aozora/all/data/16.xml) @ `3f0476e8ecdc` |
| `bench/genji/aozora_23.xml` | 43 KB | [scripts/aozora/all/data/23.xml](https://github.com/nakamura196/genji/blob/3f0476e8ecdca97b8a96a0e8a32fb505a1b64650/scripts/aozora/all/data/23.xml) @ `3f0476e8ecdc` |
| `bench/genji/kuronet_3a83785106e8af47fd5c465a5b3d2192.xml` | 1.7 MB | [docs/kuronet/anno/3a83785106e8af47fd5c465a5b3d2192/tei.xml](https://github.com/nakamura196/genji/blob/3f0476e8ecdca97b8a96a0e8a32fb505a1b64650/docs/kuronet/anno/3a83785106e8af47fd5c465a5b3d2192/tei.xml) @ `3f0476e8ecdc` |

## Geste

3 file(s), 3.3 MB. Licence: https://creativecommons.org/publicdomain/mark/1.0/ (texts); https://creativecommons.org/licenses/by-sa/4.0/ (markup). Source: [Jean-Baptiste-Camps/Geste](https://github.com/Jean-Baptiste-Camps/Geste).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/geste/ed_FloovG.xml` | 2.9 MB | [xml_gold/ed_FloovG.xml](https://github.com/Jean-Baptiste-Camps/Geste/blob/71737a20632383cd5c88d9547ba15ea8feb1f641/xml_gold/ed_FloovG.xml) @ `71737a206323` |
| `bench/geste/ed_GarLorrMo.xml` | 141 KB | [xml_gold/ed_GarLorrMo.xml](https://github.com/Jean-Baptiste-Camps/Geste/blob/71737a20632383cd5c88d9547ba15ea8feb1f641/xml_gold/ed_GarLorrMo.xml) @ `71737a206323` |
| `bench/geste/transcr_Asprem_C.xml` | 226 KB | [xml_gold/transcr_Asprem_C.xml](https://github.com/Jean-Baptiste-Camps/Geste/blob/71737a20632383cd5c88d9547ba15ea8feb1f641/xml_gold/transcr_Asprem_C.xml) @ `71737a206323` |

## Handrit.is

3 file(s), 1.7 MB. Licence: none stated (README, no LICENSE file, TEI header availability). Source: [Handrit/Manuscripts](https://github.com/Handrit/Manuscripts).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/handrit/AM02-0377-en.xml` | 1.7 MB | [Manuscripts/Den%20Arnamagn%C3%A6anske%20Samling/AM/AM02/en/AM02-0377-en.xml](https://github.com/Handrit/Manuscripts/blob/8c9a167757755b3e991cb9e63e572a61a6eecdfc/Manuscripts/Den%20Arnamagn%C3%A6anske%20Samling/AM/AM02/en/AM02-0377-en.xml) @ `8c9a16775775` |
| `bench/handrit/Lbs04-2133-is.xml` | 7 KB | [Manuscripts/Landsb%C3%B3kasafn%20%C3%8Dslands%20-%20H%C3%A1sk%C3%B3lab%C3%B3kasafn/Lbs/Lbs04/is/Lbs04-2133-is.xml](https://github.com/Handrit/Manuscripts/blob/8c9a167757755b3e991cb9e63e572a61a6eecdfc/Manuscripts/Landsb%C3%B3kasafn%20%C3%8Dslands%20-%20H%C3%A1sk%C3%B3lab%C3%B3kasafn/Lbs/Lbs04/is/Lbs04-2133-is.xml) @ `8c9a16775775` |
| `bench/handrit/Rask066-en.xml` | 5 KB | [Manuscripts/Den%20Arnamagn%C3%A6anske%20Samling/Rask/en/Rask066-en.xml](https://github.com/Handrit/Manuscripts/blob/8c9a167757755b3e991cb9e63e572a61a6eecdfc/Manuscripts/Den%20Arnamagn%C3%A6anske%20Samling/Rask/en/Rask066-en.xml) @ `8c9a16775775` |

## Inscriptions of Israel/Palestine

3 file(s), 98 KB. Licence: none stated (README, no LICENSE file, TEI header availability). Source: [Brown-University-Library/iip-texts](https://github.com/Brown-University-Library/iip-texts).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/iip/jord0001.xml` | 80 KB | [epidoc-files/jord0001.xml](https://github.com/Brown-University-Library/iip-texts/blob/0b7dc8358ccdfd0c9391f049da4839fbd91c26e5/epidoc-files/jord0001.xml) @ `0b7dc8358ccd` |
| `bench/iip/masa0839.xml` | 10 KB | [epidoc-files/masa0839.xml](https://github.com/Brown-University-Library/iip-texts/blob/0b7dc8358ccdfd0c9391f049da4839fbd91c26e5/epidoc-files/masa0839.xml) @ `0b7dc8358ccd` |
| `bench/iip/naza0001.xml` | 7 KB | [epidoc-files/naza0001.xml](https://github.com/Brown-University-Library/iip-texts/blob/0b7dc8358ccdfd0c9391f049da4839fbd91c26e5/epidoc-files/naza0001.xml) @ `0b7dc8358ccd` |

## IOSPE

3 file(s), 764 KB. Licence: https://opensource.org/licenses/MIT (repository LICENSE; nothing in the TEI header). Source: [kingsdigitallab/iospe](https://github.com/kingsdigitallab/iospe).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/iospe/3.72.xml` | 20 KB | [kiln/webapps/ROOT/content/xml/tei/inscriptions/3.72.xml](https://github.com/kingsdigitallab/iospe/blob/94f678fa2641214d528bcdd303b0616217e0f02e/kiln/webapps/ROOT/content/xml/tei/inscriptions/3.72.xml) @ `94f678fa2641` |
| `bench/iospe/5.93.xml` | 14 KB | [kiln/webapps/ROOT/content/xml/tei/inscriptions/5.93.xml](https://github.com/kingsdigitallab/iospe/blob/94f678fa2641214d528bcdd303b0616217e0f02e/kiln/webapps/ROOT/content/xml/tei/inscriptions/5.93.xml) @ `94f678fa2641` |
| `bench/iospe/introductions_byzantine.xml` | 730 KB | [kiln/webapps/ROOT/content/xml/tei/introductions/byzantine.xml](https://github.com/kingsdigitallab/iospe/blob/94f678fa2641214d528bcdd303b0616217e0f02e/kiln/webapps/ROOT/content/xml/tei/introductions/byzantine.xml) @ `94f678fa2641` |

## MiMoText roman18

3 file(s), 2.3 MB. Licence: https://creativecommons.org/publicdomain/zero/1.0/. Source: [MiMoText/roman18](https://github.com/MiMoText/roman18).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/roman18/Arnaud_Bal.xml` | 135 KB | [XML-TEI/files/Arnaud_Bal.xml](https://github.com/MiMoText/roman18/blob/c5cbec512c7059ffe39bfdce071bc6e8f6eb6a66/XML-TEI/files/Arnaud_Bal.xml) @ `c5cbec512c70` |
| `bench/roman18/Rousseau_Julie.xml` | 2.1 MB | [XML-TEI/files/Rousseau_Julie.xml](https://github.com/MiMoText/roman18/blob/c5cbec512c7059ffe39bfdce071bc6e8f6eb6a66/XML-TEI/files/Rousseau_Julie.xml) @ `c5cbec512c70` |
| `bench/roman18/Rousseau_Reine.xml` | 46 KB | [XML-TEI/files/Rousseau_Reine.xml](https://github.com/MiMoText/roman18/blob/c5cbec512c7059ffe39bfdce071bc6e8f6eb6a66/XML-TEI/files/Rousseau_Reine.xml) @ `c5cbec512c70` |

## OBVIL Critique

3 file(s), 1.1 MB. Licence: https://creativecommons.org/licenses/by-nc-nd/3.0/fr/. Source: [OBVIL/critique](https://github.com/OBVIL/critique).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/obvil/la-bruyere_ouvrages-esprit_1694.xml` | 49 KB | [la-bruyere/la-bruyere_ouvrages-esprit_1694.xml](https://github.com/OBVIL/critique/blob/887de00dde6d804d3b0e08b7db5f0963c428e33b/la-bruyere/la-bruyere_ouvrages-esprit_1694.xml) @ `887de00dde6d` |
| `bench/obvil/lafargue_origines-du-romantisme.xml` | 104 KB | [lafargue/lafargue_origines-du-romantisme.xml](https://github.com/OBVIL/critique/blob/887de00dde6d804d3b0e08b7db5f0963c428e33b/lafargue/lafargue_origines-du-romantisme.xml) @ `887de00dde6d` |
| `bench/obvil/sainte-beuve_nouveaux-lundis-08.xml` | 976 KB | [sainte-beuve/sainte-beuve_nouveaux-lundis-08.xml](https://github.com/OBVIL/critique/blob/887de00dde6d804d3b0e08b7db5f0963c428e33b/sainte-beuve/sainte-beuve_nouveaux-lundis-08.xml) @ `887de00dde6d` |

## OpenArabicPE: al-Manār

3 file(s), 645 KB. Licence: https://creativecommons.org/licenses/by-sa/4.0/. Source: [OpenArabicPE/digital-manar](https://github.com/OpenArabicPE/digital-manar).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/openarabicpe/oclc_1588981-v_29-i_6.TEIP5.xml` | 417 KB | [tei/oclc_1588981-v_29-i_6.TEIP5.xml](https://github.com/OpenArabicPE/digital-manar/blob/83fb63adc72b92e78aeb23c976d660ccb6a60bfa/tei/oclc_1588981-v_29-i_6.TEIP5.xml) @ `83fb63adc72b` |
| `bench/openarabicpe/oclc_1588981-v_3-i_10.TEIP5.xml` | 94 KB | [tei/oclc_1588981-v_3-i_10.TEIP5.xml](https://github.com/OpenArabicPE/digital-manar/blob/83fb63adc72b92e78aeb23c976d660ccb6a60bfa/tei/oclc_1588981-v_3-i_10.TEIP5.xml) @ `83fb63adc72b` |
| `bench/openarabicpe/oclc_1588981-v_7-i_23.TEIP5.xml` | 135 KB | [tei/oclc_1588981-v_7-i_23.TEIP5.xml](https://github.com/OpenArabicPE/digital-manar/blob/83fb63adc72b92e78aeb23c976d660ccb6a60bfa/tei/oclc_1588981-v_7-i_23.TEIP5.xml) @ `83fb63adc72b` |

## Persian Digital Library

3 file(s), 1.9 MB. Licence: https://creativecommons.org/licenses/by-sa/4.0/ (README). Source: [PersDigUMD/PDL](https://github.com/PersDigUMD/PDL).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/pdl/khosrow.hashtbehesht.pdl.xml` | 108 KB | [data/khosro/8behesht/khosrow.hashtbehesht.pdl.xml](https://github.com/PersDigUMD/PDL/blob/1d9ec2450d62874f60dc5fd9e6c55c7048c5a36c/data/khosro/8behesht/khosrow.hashtbehesht.pdl.xml) @ `1d9ec2450d62` |
| `bench/pdl/orfi.divan.pdl.xml` | 1.8 MB | [data/orfi/divan/orfi.divan.pdl.xml](https://github.com/PersDigUMD/PDL/blob/1d9ec2450d62874f60dc5fd9e6c55c7048c5a36c/data/orfi/divan/orfi.divan.pdl.xml) @ `1d9ec2450d62` |
| `bench/pdl/razi.gowhareshq.pdl.xml` | 6 KB | [data/razi/gouhar-e-eshgh/razi.gowhareshq.pdl.xml](https://github.com/PersDigUMD/PDL/blob/1d9ec2450d62874f60dc5fd9e6c55c7048c5a36c/data/razi/gouhar-e-eshgh/razi.gowhareshq.pdl.xml) @ `1d9ec2450d62` |

## U.S. Epigraphy

3 file(s), 40 KB. Licence: none stated (README, no LICENSE file, TEI header availability). Source: [Brown-University-Library/usep-data](https://github.com/Brown-University-Library/usep-data).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/usep/NJ.Princ.PU.AM.G.3603-I56.xml` | 27 KB | [xml_inscriptions/transcribed/NJ.Princ.PU.AM.G.3603-I56.xml](https://github.com/Brown-University-Library/usep-data/blob/e009be2c3d5e8a31c7f8303e1ca196c0963697da/xml_inscriptions/transcribed/NJ.Princ.PU.AM.G.3603-I56.xml) @ `e009be2c3d5e` |
| `bench/usep/PA.Phil.UP.UM.L.MS5732.xml` | 7 KB | [xml_inscriptions/metadata_only/PA.Phil.UP.UM.L.MS5732.xml](https://github.com/Brown-University-Library/usep-data/blob/e009be2c3d5e8a31c7f8303e1ca196c0963697da/xml_inscriptions/metadata_only/PA.Phil.UP.UM.L.MS5732.xml) @ `e009be2c3d5e` |
| `bench/usep/VA.Rich.MFA.G.60.32.xml` | 5 KB | [xml_inscriptions/transcribed/VA.Rich.MFA.G.60.32.xml](https://github.com/Brown-University-Library/usep-data/blob/e009be2c3d5e8a31c7f8303e1ca196c0963697da/xml_inscriptions/transcribed/VA.Rich.MFA.G.60.32.xml) @ `e009be2c3d5e` |

## VICAV

3 file(s), 2.8 MB. Licence: https://creativecommons.org/licenses/by-nc-sa/3.0/ (TEI header; the repository README says CC BY-SA 4.0 unless otherwise noted). Source: [acdh-oeaw/vicav-content](https://github.com/acdh-oeaw/vicav-content).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `bench/vicav/dc_acm_baghdad_eng_001.xml` | 2.4 MB | [vicav_dicts/dc_acm_baghdad_eng_001.xml](https://github.com/acdh-oeaw/vicav-content/blob/66554f59b8f5ca4b59669039a00b29419fc1f30e/vicav_dicts/dc_acm_baghdad_eng_001.xml) @ `66554f59b8f5` |
| `bench/vicav/vicav_profile_palmyra.xml` | 6 KB | [vicav_profiles/vicav_profile_palmyra.xml](https://github.com/acdh-oeaw/vicav-content/blob/66554f59b8f5ca4b59669039a00b29419fc1f30e/vicav_profiles/vicav_profile_palmyra.xml) @ `66554f59b8f5` |
| `bench/vicav/vicav_sample_text_002.xml` | 434 KB | [vicav_samples/vicav_sample_text_002.xml](https://github.com/acdh-oeaw/vicav-content/blob/66554f59b8f5ca4b59669039a00b29419fc1f30e/vicav_samples/vicav_sample_text_002.xml) @ `66554f59b8f5` |

## TEI Consortium test suite

49 file(s), 834 KB. Licence: BSD-2-Clause / CC BY 3.0. Source: [TEIC/TEI](https://github.com/TEIC/TEI).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `docs/tei-c/aphrodisias.xml` | 9 KB | [P5/Test/aphrodisias.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/aphrodisias.xml) @ `3525937afae1` |
| `docs/tei-c/aphrodisias2.xml` | 9 KB | [P5/Test/aphrodisias2.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/aphrodisias2.xml) @ `3525937afae1` |
| `docs/tei-c/comenius.xml` | 3 KB | [P5/Test/comenius.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/comenius.xml) @ `3525937afae1` |
| `docs/tei-c/comenius2.xml` | 3 KB | [P5/Test/comenius2.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/comenius2.xml) @ `3525937afae1` |
| `docs/tei-c/detest.xml` | 24 KB | [P5/Test/detest.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/detest.xml) @ `3525937afae1` |
| `docs/tei-c/morerendition.xml` | 1 KB | [P5/Test/morerendition.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/morerendition.xml) @ `3525937afae1` |
| `docs/tei-c/relateditem.xml` | 4 KB | [P5/Test/relateditem.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/relateditem.xml) @ `3525937afae1` |
| `docs/tei-c/tei_lite.xml` | 177 KB | [P5/Test/tei_lite.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/tei_lite.xml) @ `3525937afae1` |
| `docs/tei-c/test-newodd.xml` | 15 KB | [P5/Test/test-newodd.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/test-newodd.xml) @ `3525937afae1` |
| `docs/tei-c/testAlien.xml` | 1 KB | [P5/Test/testAlien.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testAlien.xml) @ `3525937afae1` |
| `docs/tei-c/testTripReport.xml` | 1 KB | [P5/Test/testTripReport.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testTripReport.xml) @ `3525937afae1` |
| `docs/tei-c/testall.xml` | 58 KB | [P5/Test/testall.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testall.xml) @ `3525937afae1` |
| `docs/tei-c/testappinfo.xml` | 2 KB | [P5/Test/testappinfo.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testappinfo.xml) @ `3525937afae1` |
| `docs/tei-c/testbare.xml` | 1 KB | [P5/Test/testbare.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testbare.xml) @ `3525937afae1` |
| `docs/tei-c/testbasic.xml` | 27 KB | [P5/Test/testbasic.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testbasic.xml) @ `3525937afae1` |
| `docs/tei-c/testbibl.xml` | 3 KB | [P5/Test/testbibl.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testbibl.xml) @ `3525937afae1` |
| `docs/tei-c/testchinese.xml` | 33 KB | [P5/Test/testchinese.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testchinese.xml) @ `3525937afae1` |
| `docs/tei-c/testdocbook1.xml` | 4 KB | [P5/Test/testdocbook1.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testdocbook1.xml) @ `3525937afae1` |
| `docs/tei-c/testdocbook4.xml` | 26 KB | [P5/Test/testdocbook4.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testdocbook4.xml) @ `3525937afae1` |
| `docs/tei-c/testdocbook5.xml` | 1 KB | [P5/Test/testdocbook5.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testdocbook5.xml) @ `3525937afae1` |
| `docs/tei-c/testdrama.xml` | 18 KB | [P5/Test/testdrama.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testdrama.xml) @ `3525937afae1` |
| `docs/tei-c/testfand2.xml` | 1 KB | [P5/Test/testfand2.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testfand2.xml) @ `3525937afae1` |
| `docs/tei-c/testfand3.xml` | 1 KB | [P5/Test/testfand3.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testfand3.xml) @ `3525937afae1` |
| `docs/tei-c/testfand5.xml` | 1 KB | [P5/Test/testfand5.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testfand5.xml) @ `3525937afae1` |
| `docs/tei-c/testfrag.xml` | 8 KB | [P5/Test/testfrag.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testfrag.xml) @ `3525937afae1` |
| `docs/tei-c/testfsd.xml` | 4 KB | [P5/Test/testfsd.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testfsd.xml) @ `3525937afae1` |
| `docs/tei-c/testglobals1.xml` | 1 KB | [P5/Test/testglobals1.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testglobals1.xml) @ `3525937afae1` |
| `docs/tei-c/testgos.xml` | 15 KB | [P5/Test/testgos.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testgos.xml) @ `3525937afae1` |
| `docs/tei-c/testi18n.xml` | 9 KB | [P5/Test/testi18n.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testi18n.xml) @ `3525937afae1` |
| `docs/tei-c/testmathml.xml` | 24 KB | [P5/Test/testmathml.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testmathml.xml) @ `3525937afae1` |
| `docs/tei-c/testmeta2010.xml` | 1 KB | [P5/Test/testmeta2010.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testmeta2010.xml) @ `3525937afae1` |
| `docs/tei-c/testminimal.xml` | 1 KB | [P5/Test/testminimal.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testminimal.xml) @ `3525937afae1` |
| `docs/tei-c/testms.xml` | 102 KB | [P5/Test/testms.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testms.xml) @ `3525937afae1` |
| `docs/tei-c/testnames.xml` | 72 KB | [P5/Test/testnames.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testnames.xml) @ `3525937afae1` |
| `docs/tei-c/testndextra.xml` | 14 KB | [P5/Test/testndextra.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testndextra.xml) @ `3525937afae1` |
| `docs/tei-c/testoverlap.xml` | 3 KB | [P5/Test/testoverlap.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testoverlap.xml) @ `3525937afae1` |
| `docs/tei-c/testplace-kml.xml` | 2 KB | [P5/Test/testplace-kml.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testplace-kml.xml) @ `3525937afae1` |
| `docs/tei-c/testplace-nogml.xml` | 28 KB | [P5/Test/testplace-nogml.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testplace-nogml.xml) @ `3525937afae1` |
| `docs/tei-c/testplace.xml` | 27 KB | [P5/Test/testplace.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testplace.xml) @ `3525937afae1` |
| `docs/tei-c/testplus.xml` | 19 KB | [P5/Test/testplus.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testplus.xml) @ `3525937afae1` |
| `docs/tei-c/testrdf.xml` | 26 KB | [P5/Test/testrdf.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testrdf.xml) @ `3525937afae1` |
| `docs/tei-c/testspoken.xml` | 1 KB | [P5/Test/testspoken.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testspoken.xml) @ `3525937afae1` |
| `docs/tei-c/testtite1.xml` | 1 KB | [P5/Test/testtite1.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtite1.xml) @ `3525937afae1` |
| `docs/tei-c/testtite2.xml` | 1 KB | [P5/Test/testtite2.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtite2.xml) @ `3525937afae1` |
| `docs/tei-c/testtite3.xml` | 2 KB | [P5/Test/testtite3.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtite3.xml) @ `3525937afae1` |
| `docs/tei-c/testtranscr.xml` | 4 KB | [P5/Test/testtranscr.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtranscr.xml) @ `3525937afae1` |
| `docs/tei-c/testtranscr2.xml` | 4 KB | [P5/Test/testtranscr2.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtranscr2.xml) @ `3525937afae1` |
| `docs/tei-c/testtranscr4.xml` | 2 KB | [P5/Test/testtranscr4.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/testtranscr4.xml) @ `3525937afae1` |
| `docs/tei-c/torture.xml` | 43 KB | [P5/Test/torture.xml](https://github.com/TEIC/TEI/blob/3525937afae16b45fe581c41e5c0c1a8deac21a3/P5/Test/torture.xml) @ `3525937afae1` |

## TEI P5 2.7.0 tei_all

1 file(s), 735 KB. Licence: BSD-2-Clause / CC BY 3.0. Source: [TEIC/TEI-Simple](https://github.com/TEIC/TEI-Simple).

| File in `tests/tei/` | Size | Upstream (pinned commit) |
|---|---|---|
| `schemas/tei_all-2.7.0.rng` | 735 KB | [tei_all.rng](https://github.com/TEIC/TEI-Simple/blob/16095eb1b08ea0b640a960175c14589861d9107c/tei_all.rng) @ `16095eb1b08e` |
