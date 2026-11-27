#!/usr/bin/env python3
"""Builds crates/core/data/genre_tree.json from Wikidata (CC0): music genres, their English/Turkish names and
aliases, "subclass of" (P279) and "influenced by" (P737) links between them. Run by hand now and then:
    python3 -I tools/genre-tree/make.py crates/core/data/genre_tree.json
"""
import datetime, json, os, sys, time, urllib.error, urllib.parse, urllib.request

URL = "https://query.wikidata.org/sparql"
GENRE = "?g wdt:P31 wd:Q188451 ."


def ask(q):
    req = urllib.request.Request(URL + "?" + urllib.parse.urlencode({"query": q, "format": "json"}),
                                 headers={"User-Agent": "re-la-genre-tree/1.0", "Accept": "application/sparql-results+json"})
    for _ in range(6):
        try:
            with urllib.request.urlopen(req, timeout=300) as r:
                rows = json.load(r)["results"]["bindings"]
            print(f"{len(rows)} rows", file=sys.stderr, flush=True)
            time.sleep(61)  # the service may allow one query a minute
            return rows
        except urllib.error.HTTPError as e:
            if e.code != 429:
                raise
            print(f"429, waiting", file=sys.stderr, flush=True)
            time.sleep(int(e.headers.get("Retry-After") or 61) + 1)
    raise SystemExit("Wikidata kept refusing (429)")


def qid(b, k):
    return b[k]["value"].rsplit("/", 1)[1]


def main(out):
    names = {}
    for prop in ("rdfs:label", "skos:altLabel"):
        for b in ask(f'SELECT ?g ?l WHERE {{ {GENRE} ?g {prop} ?l FILTER(LANG(?l) IN ("en", "tr")) }}'):
            n = b["l"]["value"].strip().lower()
            if n and len(n) <= 60:
                names.setdefault(qid(b, "g"), []).append(n)
    links = {}
    for key, prop in (("p", "P279"), ("i", "P737")):
        for b in ask(f"SELECT ?g ?o WHERE {{ {GENRE} ?g wdt:{prop} ?o . ?o wdt:P31 wd:Q188451 . }}"):
            links.setdefault(key, []).append((qid(b, "g"), qid(b, "o")))
    linked = {q for pairs in links.values() for pair in pairs for q in pair}
    # a genre with no link to any other says nothing about similarity: left out to keep the file small
    keep = sorted(q for q in names if q in linked)
    at = {q: i for i, q in enumerate(keep)}
    g = [{"n": sorted(set(names[q]))} for q in keep]
    for key, pairs in links.items():
        for a, b in pairs:
            if a in at and b in at and a != b:
                g[at[a]].setdefault(key, []).append(at[b])
    for e in g:
        for key in ("p", "i"):
            if key in e:
                e[key] = sorted(set(e[key]))
    doc = {"license": "CC0 (Wikidata)", "made": datetime.date.today().isoformat(), "g": g}
    tmp = f"{out}.tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, separators=(",", ":"))
    os.replace(tmp, out)  # a failed write leaves the old catalog
    print(f"{len(g)} genres, {sum(len(e.get('p', [])) for e in g)} parent links, "
          f"{sum(len(e.get('i', [])) for e in g)} influence links")


if __name__ == "__main__":
    main(sys.argv[1])
