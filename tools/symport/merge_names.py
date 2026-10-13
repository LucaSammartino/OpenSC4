"""Merge every Windows naming source into re/names_windows.tsv (one name per address).

Priority (highest first):
  manual     re/symbols_windows.tsv and re/symbols/*.tsv (hand-verified, with evidence)
  auto       re/symport/matches.tsv (constant/string anchors, call-graph and vtable propagation)
  library    Function ID matches for statically linked CRT/STLport/Lua/libpng/zlib (0xC0000054's FID databases,
             attached by tools/setup_ghidra.ps1)
  folded     re/symport/folded.tsv: identical-code-folded bodies shared by several Mac methods

confidence: high (manual, auto stage A/C), medium (auto stage B), low (tiny bodies
<= 12 bytes matched automatically, which are often folded getters), folded.
"""
import collections
import csv
import os
import pickle

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
WIN_EXPORT = os.path.join(ROOT, "ghidra", "export", "windows")  # tools/setup_ghidra.ps1 -Windows
csv.field_size_limit(1 << 30)


def rows(path):
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        yield from csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE)


def main():
    win = pickle.load(open(os.path.join(ROOT, "re", "symport", "win_features.pkl"), "rb"))
    names = {}

    def put(a, name, source, conf, note=""):
        if a not in names and name:
            names[a] = (name.strip(), source, conf, note or "")

    manual = [os.path.join(ROOT, "re", "symbols_windows.tsv")]
    sd = os.path.join(ROOT, "re", "symbols")
    if os.path.isdir(sd):
        manual += sorted(os.path.join(sd, f) for f in os.listdir(sd) if f.endswith(".tsv"))
    for p in manual:
        for r in rows(p):
            try:
                a = int(r["va"], 16)
            except (ValueError, KeyError, TypeError):
                continue
            put(a, r.get("name", ""), "manual:" + os.path.basename(p), "high", r.get("evidence", ""))

    for r in rows(os.path.join(ROOT, "re", "symport", "matches.tsv")):
        a = int(r["win"], 16)
        st = r["stage"]
        conf = "high" if st in ("A", "C") else "medium"
        if win.get(a, {}).get("size", 99) <= 12:
            conf = "low"
        put(a, r["mac_name"], f"auto:{st}", conf, f"mac {r['mac']} score {r['score']} support {r['support']}")

    # Names the Function ID analyzer gave during auto-analysis (symbol source ANALYSIS, not a default name).
    for r in rows(os.path.join(WIN_EXPORT, "functions.tsv")):
        n = r["name"]
        if r.get("source") == "ANALYSIS" and n and not n.startswith(("FUN_", "thunk_FUN_", "entry")):
            put(int(r["address"], 16), "lib::" + n, "library", "medium", "Function ID")

    for r in rows(os.path.join(ROOT, "re", "symport", "folded.tsv")):
        a = int(r["win"], 16)
        ns = r["mac_names"].split(" | ")
        put(a, "icf::" + ns[0], "folded", "folded", f"{r['count']} names: {r['mac_names']}")

    out = os.path.join(ROOT, "re", "names_windows.tsv")
    with open(out, "w", encoding="utf-8") as fh:
        fh.write("va\tname\tsource\tconfidence\tnote\n")
        for a in sorted(names):
            n, s, c, note = names[a]
            fh.write(f"{a:08x}\t{n}\t{s}\t{c}\t{note.replace(chr(9), ' ')}\n")
    # Data labels: paired vtables, plus hand-verified addresses that are not function entries.
    labels = {}
    for r in rows(os.path.join(ROOT, "re", "symport", "vtable_pairs.tsv")):
        if r["class"].startswith("__ZTV"):
            continue
        sub = int(r["sub"])
        labels[int(r["win_vtable"], 16)] = (f"{r['class']}::vftable" + (f"_{sub}" if sub else ""),
                                            int(r["win_slots"]), f"paired with Mac {r['mac_vtable']}#{sub}, "
                                            f"{r['votes']} agreeing slots")
    for p in manual:
        for r in rows(p):
            try:
                a = int(r["va"], 16)
            except (ValueError, KeyError, TypeError):
                continue
            if a not in win and r.get("name"):
                labels.setdefault(a, (r["name"].split(" (")[0].strip(), 0, r.get("evidence") or ""))
    with open(os.path.join(ROOT, "re", "labels_windows.tsv"), "w", encoding="utf-8") as fh:
        fh.write("va\tname\tslots\tnote\n")
        for a in sorted(labels):
            n, k, note = labels[a]
            fh.write(f"{a:08x}\t{n}\t{k}\t{note.replace(chr(9), ' ')}\n")
    print(f"data labels: {len(labels)}")

    by = collections.Counter(v[1].split(":")[0] for v in names.values())
    conf = collections.Counter(v[2] for v in names.values())
    total = sum(1 for f in win.values() if not f["thunk"])
    print(f"named {len(names)} of {total} non-thunk Windows functions ({100*len(names)/total:.1f}%)")
    print("by source:", dict(by))
    print("by confidence:", dict(conf))


if __name__ == "__main__":
    main()
