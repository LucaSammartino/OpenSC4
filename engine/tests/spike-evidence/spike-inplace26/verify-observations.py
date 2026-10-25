"""Check values-only CSV evidence against final counters in the named game logs."""
import argparse
import csv
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("log_dir", type=Path)
args = parser.parse_args()
folder = Path(__file__).resolve().parent
sizes = {r["name"]: int(r["body_bytes"]) for r in csv.DictReader((folder / "functions.csv").open())}
for round_id in (2, 3):
    pair = {}
    mutations = []
    for source in ("natural", "fixture"):
        path = folder / f"{source}-{round_id}.csv"
        if not path.exists():
            continue
        label = f"inplace26-{source}-{round_id}"
        latest = {}
        for line in (args.log_dir / f"{label}.log").read_text(errors="replace").splitlines():
            if "inplace summary " in line:
                row = dict(re.findall(r"([a-z_]+)=([a-z0-9-]+)", line))
                if row["source"] == source:
                    latest[row["name"]] = row
            if "inplace mutation " in line:
                mutations.append(line[line.index("inplace mutation "):])
        saved = list(csv.DictReader(path.open()))
        assert len(saved) == len(latest) == 20
        for row in saved:
            for key, value in row.items():
                assert value == latest[row["name"]][key], (label, row["name"], key, value)
        pair[source] = saved
        fields = "calls sampled compared exact float_only different outside refused mutation detected".split()
        print(label, json.dumps({k: sum(int(r[k]) for r in saved) for k in fields}))
    if len(pair) != 2:
        continue
    by_name = {}
    for rows in pair.values():
        for row in rows:
            by_name.setdefault(row["name"], []).append(row)
    exact, strict, outside, floats = [], [], [], []
    for name, rows in by_name.items():
        total = lambda key: sum(int(r[key]) for r in rows)
        if total("outside"):
            outside.append(name)
        if total("float_only"):
            floats.append(name)
        if total("compared") and not any(total(k) for k in ("outside", "different", "float_only")):
            exact.append(name)
            if not total("refused"):
                strict.append(name)
    print("round", round_id, json.dumps(dict(
        observed_exact_confined=exact, no_refusal_exact_confined=strict,
        observed_body_bytes=sum(sizes[n] for n in exact),
        no_refusal_body_bytes=sum(sizes[n] for n in strict),
        outside_signals=outside, float_only=floats)))
print("CSV evidence matches final game counters.")
