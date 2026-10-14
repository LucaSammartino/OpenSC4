"""Map SimCity 4.exe addresses to the function that contains them (coordinator triage tool).

RE material: reads decomp/ (function headers "// ---- <addr>  <name>") and ghidra/export/windows/functions.tsv
(body sizes). Not for implementer lanes.

Usage:
  python tools/addr2func.py 0x5B9FEB 005e55e0 ...
  python tools/addr2func.py --log build-runs/run74.log      # every "SimCity 4.exe+0x..." / "(executable)+0x..."
                                                            # in the plugin log's exception and watchdog reports
"""
import bisect
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IMAGE_BASE = 0x400000
HEADER = re.compile(r"^// ---- ([0-9a-fA-F]{8})  (.+?)\s*$")


def load():
    sizes = {}
    tsv = os.path.join(ROOT, "ghidra", "export", "windows", "functions.tsv")
    with open(tsv, encoding="utf-8", errors="replace") as f:
        next(f)
        for line in f:
            col = line.rstrip("\n").split("\t")
            try:
                sizes[int(col[0], 16)] = (col[1], int(col[2]))
            except (ValueError, IndexError):
                pass
    names = {}
    for dirpath, _, files in os.walk(os.path.join(ROOT, "decomp")):
        for fn in files:
            if not fn.endswith(".c"):
                continue
            path = os.path.join(dirpath, fn)
            with open(path, encoding="utf-8", errors="replace") as f:
                for line in f:
                    m = HEADER.match(line)
                    if m:
                        a = int(m.group(1), 16)
                        # A named header wins over a FUN_ one (the same function can appear twice).
                        if a not in names or names[a][0].startswith("FUN_"):
                            names[a] = (m.group(2), os.path.relpath(path, ROOT))
    starts = sorted(set(sizes) | set(names))
    return starts, sizes, names


def describe(addr, starts, sizes, names):
    i = bisect.bisect_right(starts, addr) - 1
    if i < 0:
        return "0x%08X: before the first function" % addr
    start = starts[i]
    name, where = names.get(start, (sizes.get(start, ("?", 0))[0], ""))
    size = sizes.get(start, (None, 0))[1]
    inside = "" if size == 0 or addr - start < size else " (PAST its %d-byte body)" % size
    return "0x%08X = %s+0x%X%s%s" % (addr, name, addr - start, inside, ("  [" + where + "]") if where else "")


def from_log(path):
    pat = re.compile(r"(?:SimCity 4\.exe|\(executable\))\+0x([0-9A-Fa-f]+)")
    seen = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            for m in pat.finditer(line):
                a = IMAGE_BASE + int(m.group(1), 16)
                if a not in seen:
                    seen.append(a)
    return seen


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == "--log":
        addrs = from_log(argv[1])
    else:
        addrs = [int(a, 16) for a in argv]
    starts, sizes, names = load()
    for a in addrs:
        print(describe(a, starts, sizes, names))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
