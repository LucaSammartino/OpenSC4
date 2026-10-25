"""Print a small values-only summary from this lane's game logs; never read images."""
import argparse
import csv
import re
import sys
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("log", type=Path)
args = parser.parse_args()
latest = {}
mutations = []
for line in args.log.read_text(encoding="utf-8", errors="replace").splitlines():
    if "inplace summary " in line:
        row = dict(re.findall(r"([a-z_]+)=([a-z0-9-]+)", line))
        latest[row["name"], row["source"]] = row
    if "inplace mutation " in line or "inplace diff " in line or "inplace outside " in line:
        # These records contain specified output words/region-relative offsets only.
        mutations.append(line[line.index("inplace "):])
fields = "name,rva,bytes,installed,source,calls,sampled,compared,exact,float_only,different,outside,refused,nested,mutation,detected,max_ulp".split(",")
writer = csv.DictWriter(sys.stdout, fields, extrasaction="ignore", lineterminator="\n")
writer.writeheader()
for key in sorted(latest):
    writer.writerow(latest[key])
print("# Located differences and mutations")
for line in mutations:
    print("# " + line)
