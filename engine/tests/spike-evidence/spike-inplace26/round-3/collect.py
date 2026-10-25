"""Print final round-3 counters/locations from a raw game log; write nothing."""
import csv
import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent
NAMES = {row["name"] for row in csv.DictReader((ROOT / "functions.csv").open())}
FIELDS = "installed calls sampled compared exact float_only different outside refused nested mutation detected max_ulp".split()


def collect(path):
    summaries, mutations = {}, {}
    for line in Path(path).read_text(errors="replace").splitlines():
        if "inplace summary " not in line and "inplace mutation " not in line:
            continue
        row = dict(re.findall(r"([a-z_]+)=([^\s]+)", line))
        if row.get("name") not in NAMES:
            continue
        key = (row["name"], row["source"])
        if "inplace summary " in line:
            summaries[key] = [row[field] for field in FIELDS]
        else:
            mutations[key] = [row[field] for field in ("window", "byte", "xor", "detected")]
    return summaries, mutations


if __name__ == "__main__":
    summaries, mutations = collect(sys.argv[1])
    print("name,source," + ",".join(FIELDS))
    for key, values in sorted(summaries.items()):
        print(",".join([*key, *values]))
    print("MUTATIONS name,source,window,byte,xor,detected")
    for key, values in sorted(mutations.items()):
        print(",".join([*key, *values]))
