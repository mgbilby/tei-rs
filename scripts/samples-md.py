#!/usr/bin/env python3
"""Regenerates tests/tei/SAMPLES.md from tests/tei/samples.tsv.

Usage: scripts/samples-md.py   (run from anywhere; no network access)
"""
import collections, os
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
tsv = os.path.join(root, 'tests/tei/samples.tsv')
out = os.path.join(root, 'tests/tei/SAMPLES.md')
rows = [l.rstrip('\n').split('\t') for l in open(tsv) if not l.startswith('#') and l.strip()]
by = collections.OrderedDict()
order = ['Deutsches Textarchiv','papyri.info','Text Creation Partnership','Perseus','First1KGreek','I.Sicily','DraCor (GerDraCor)','ELTeC','ParlaMint','SARIT','Arthur Schnitzler','Beta maṣāḥǝft','CBETA','Coptic Scriptorium','correspSearch (CMIF)','FreeDict','Legal Kraus','Patrologia Latina','Syriaca.org','Tolstoy Digital','84000','Akademieprotokolle (ÖAW)','Auden Musulin Papers','Bodleian medieval manuscripts','Bullinger Digital','CLiGS textbox','conha19','CroALa','CSEL, Open Greek and Latin','DARIAH-ERIC lexical resources','Deutsches Lyrik Korpus (DLK)','DHARMA','Digital Mitford','DISCO','ENCPOS (École des chartes)','Faust-Edition','Fihrist','Frankenstein Variorum','FRUS (Office of the Historian)','Genji monogatari (nakamura196)','Geste','Handrit.is','Inscriptions of Israel/Palestine','IOSPE','MiMoText roman18','OBVIL Critique','OpenArabicPE: al-Manār','Persian Digital Library','U.S. Epigraphy','VICAV','TEI Consortium test suite','TEI P5 2.7.0 tei_all']
for p in order: by[p] = []
for r in rows: by.setdefault(r[1], []).append(r)
def size(n):
    n = int(n); return f'{n/1048576:.1f} MB' if n >= 1048576 else f'{max(1, round(n/1024))} KB'
L = ['# Sample files', '',
 'Every TEI document and schema the tests and benchmarks use, with a link to the',
 'exact upstream version. The public repository includes the files whose licence',
 'allows it ([`licences.tsv`](licences.tsv)); download the rest, or all of them,',
 'into `tests/tei/` with', '', '```sh', 'scripts/fetch-samples.sh            # all files (about TOTAL_MB MB; verifies SHA-256)',
 'scripts/fetch-samples.sh papyri     # only paths containing "papyri"', '```', '',
 'The machine-readable list (with hashes) is [`samples.tsv`](samples.tsv); licences',
 'are summarised in [`SOURCES.md`](SOURCES.md). Without these files, `cargo test`',
 'skips the comparisons with Jing and `cargo bench` has nothing to time.', '']
total = 0
for project, rs in by.items():
    if not rs: continue
    repo, commit, lic = rs[0][2], rs[0][3], rs[0][5]
    nbytes = sum(int(r[7]) for r in rs); total += nbytes
    repos = sorted({r[2] for r in rs})
    sources = ', '.join(f'[{x}](https://{x})' if '.' in x.split('/')[0] else f'[{x}](https://github.com/{x})' for x in repos)
    L += [f'## {project}', '', f'{len(rs)} file(s), {size(nbytes)}. Licence: {lic}. Source: {sources}.', '',
          '| File in `tests/tei/` | Size | Upstream (pinned commit) |', '|---|---|---|']
    for r in sorted(rs):
        if r[4].startswith(('http://', 'https://')):
            L.append(f'| `{r[0]}` | {size(r[7])} | [{r[4]}]({r[4]}) |')
        else:
            url = f'https://github.com/{r[2]}/blob/{r[3]}/{r[4]}'
            L.append(f'| `{r[0]}` | {size(r[7])} | [{r[4]}]({url}) @ `{r[3][:12]}` |')
    L.append('')
open(out, 'w').write('\n'.join(L).rstrip().replace('TOTAL_MB', str(round(total/1048576))) + '\n')
print(len(rows), 'files', round(total/1048576, 1), 'MB')
