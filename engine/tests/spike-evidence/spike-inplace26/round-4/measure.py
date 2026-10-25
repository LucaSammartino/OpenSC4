# SPDX-License-Identifier: GPL-3.0-or-later
"""Recompute round-4 evidence from this lane's value-only game results."""
import argparse
import csv
import io
import json
import re
from pathlib import Path


def measure(root, label):
    verdict = (root / "game-results" / (label + ".verdict")).read_text(encoding="utf-8")
    log = (root / "game-results" / (label + ".log")).read_text(encoding="utf-8", errors="replace")
    rows = {}
    for line in log.splitlines():
        if "inplace summary " not in line:
            continue
        row = dict(word.split("=", 1) for word in line.split("inplace summary ")[1].split())
        for key in row.keys() - {"name", "rva", "source"}:
            row[key] = int(row[key])
        rows[(row["name"], row["source"])] = row
    totals = {}
    for row in rows.values():
        target = totals.setdefault(row["source"], {})
        for key, value in row.items():
            if key not in ("name", "rva", "bytes", "installed", "source", "max_ulp"):
                target[key] = target.get(key, 0) + value
        target["max_ulp"] = max(target.get("max_ulp", 0), row["max_ulp"])
    fingerprints = re.findall(r"dll sha256 ([0-9a-f]+)", verdict)
    summary = {"label": label, "verdict": verdict.splitlines()[0],
               "dll": fingerprints[0] if fingerprints else "see verdict",
               "rows": len(rows), "installed": sum(r["installed"] for r in rows.values() if r["source"] == "natural"),
               "totals": totals}
    summary["mutations_located"] = len(re.findall(r"inplace mutation name=\S+ source=\S+ window=\d+ (?:byte|word)=\d+ xor=1 detected=1", log))
    summary["uninstalled"] = sorted({r["name"] for r in rows.values() if not r["installed"]})
    summary["uncompared"] = sorted({r["name"] for r in rows.values()} - {r["name"] for r in rows.values() if r["compared"]})
    return summary, rows


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("label")
    parser.add_argument("--raw", type=Path, default=Path("build-runs"))
    parser.add_argument("--rows", action="store_true")
    args = parser.parse_args()
    summary, rows = measure(args.raw, args.label)
    if args.rows:
        output = io.StringIO(newline="")
        writer = csv.DictWriter(output, fieldnames=next(iter(rows.values())).keys(), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows[key] for key in sorted(rows))
        print(output.getvalue(), end="")
    else:
        print(json.dumps(summary, indent=2))
