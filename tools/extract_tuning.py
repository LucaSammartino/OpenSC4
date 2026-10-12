"""Dump SC4's simulator tuning exemplars and Demand exemplars from the installed game (read-only).

Later packages override earlier ones (SimCity_1..5.dat, then EP1.dat), matching game load order
for the base data. Property IDs are labelled from re/prior_art_exemplar_props.md where known.

Output:
  re/tuning/simulators.md   every exemplar in the simulator-tuning group 0xE7E2C2DB
  re/tuning/demand.md       every exemplar in the Demand group 0xC7BB4816
  re/tuning/*.json          the same data, machine-readable
Usage: python tools/extract_tuning.py [game_dir]
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from dbpf import DBPF, TYPE_EXEMPLAR, parse_exemplar  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
GAME = sys.argv[1] if len(sys.argv) > 1 else r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition"
PACKS = [f"SimCity_{i}.dat" for i in range(1, 6)] + ["EP1.dat"]
GROUPS = {0xE7E2C2DB: "simulators", 0xC7BB4816: "demand"}
EXEMPLAR_NAME = 0x00000020


def prop_names():
    names = {}
    for line in open(os.path.join(ROOT, "re", "prior_art_exemplar_props.md"), encoding="utf-8"):
        m = re.match(r"\|\s*([^|]*)\|\s*(0x[0-9a-fA-F]{8})\s*\|\s*[^|]*\|\s*([^|]*)\|", line)
        if m:
            names[int(m.group(2), 16)] = (m.group(3).strip(), m.group(1).strip())
    return names


def main():
    names = prop_names()
    found = {g: {} for g in GROUPS}
    for p in PACKS:
        path = os.path.join(GAME, p)
        if not os.path.exists(path):
            continue
        db = DBPF(path)
        for e in db.find(TYPE_EXEMPLAR):
            if e[1] in GROUPS:
                try:
                    parent, props = parse_exemplar(db.read(e))
                except Exception as ex:  # text exemplars etc.
                    props, parent = {"error": str(ex)}, None
                found[e[1]][e[2]] = {"package": p, "props": props}
    out = os.path.join(ROOT, "re", "tuning")
    os.makedirs(out, exist_ok=True)
    for g, label in GROUPS.items():
        js, md = {}, [f"# SC4 {label} exemplars (group 0x{g:08X})\n",
                      f"Extracted read-only from {GAME}; later packages override earlier ones.\n"]
        for inst in sorted(found[g]):
            rec = found[g][inst]
            props = rec["props"]
            title = props.get(EXEMPLAR_NAME, ("", "?"))[1] if isinstance(props, dict) and EXEMPLAR_NAME in props else "?"
            md.append(f"\n## 0x{inst:08X} — {title}  ({rec['package']})\n")
            md.append("| Property | Name | Type | Value |\n|---|---|---|---|")
            jp = {}
            for pid, v in sorted(props.items()) if isinstance(props, dict) else []:
                if pid == "error":
                    continue
                t, val = v
                nm = names.get(pid, ("", ""))[0]
                sval = val if isinstance(val, str) else ", ".join(f"{x:g}" if isinstance(x, float) else str(x) for x in val)
                if len(sval) > 400:
                    sval = sval[:400] + " …"
                md.append(f"| 0x{pid:08X} | {nm} | {t} | {sval} |")
                jp[f"0x{pid:08X}"] = {"name": nm, "type": t, "value": val}
            js[f"0x{inst:08X}"] = {"title": title, "package": rec["package"], "props": jp}
        open(os.path.join(out, f"{label}.md"), "w", encoding="utf-8").write("\n".join(md) + "\n")
        json.dump(js, open(os.path.join(out, f"{label}.json"), "w", encoding="utf-8"), indent=1)
        print(f"{label}: {len(js)} exemplars")


if __name__ == "__main__":
    main()
