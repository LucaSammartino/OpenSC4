"""Extract per-function matching features for the Windows exe and the Mac Rev A Intel reference.

Features per function:
  size      body bytes
  consts    set of "rare" 32-bit immediates (GZCOM IDs, magic numbers); image addresses and
            small values excluded because they differ between builds
  strings   set of referenced string values
  callees   ordered list of direct call targets (by address in that build)
  callers   set of direct callers

Output: re/symport/{win,mac}_features.pkl
Usage:  python tools/symport/features.py
"""
import collections
import csv
import os
import pickle
import re
import sys

import capstone
import pefile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
WIN_PKG = os.path.join(ROOT, "external", "extracted", "sc4-windows-analysis", "artifacts", "code", "SimCity4-deep")
MAC_PKG = os.path.join(ROOT, "external", "extracted", "sc4-mac-symbol-analysis", "artifacts", "code", "Mac-RevA-x86-named")
OUT = os.path.join(ROOT, "re", "symport")
EXE = os.path.join(ROOT, "bin", "SimCity4.exe")

csv.field_size_limit(1 << 30)


def tsv(path):
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        yield from csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE)


def is_rare_const(v, img_lo, img_hi):
    if v < 0x10000 or v >= 0xFFFF0000:
        return False
    if img_lo <= v < img_hi:
        return False
    return True


def parse_ranges(s):
    out = []
    for part in s.strip().strip(";").split(";"):
        if "-" in part:
            a, b = part.split("-")
            out.append((int(a, 16), int(b, 16)))
    return out


def common(pkg):
    funcs = {}
    for r in tsv(os.path.join(pkg, "functions.tsv")):
        a = int(r["address"], 16)
        funcs[a] = {"name": r["name"], "size": int(r["body_bytes"] or 0), "thunk": r["thunk"] == "true",
                    "consts": set(), "strings": set(), "callees": [], "callers": set(), "ranges": []}
    for r in tsv(os.path.join(pkg, "function-details.tsv")):
        a = int(r["address"], 16)
        if a in funcs:
            funcs[a]["ranges"] = parse_ranges(r["ranges"])
    for r in tsv(os.path.join(pkg, "calls.tsv")):
        try:
            c, s, t = int(r["caller"], 16), int(r["callsite"], 16), int(r["target"], 16)
        except ValueError:
            continue
        if c in funcs:
            funcs[c]["callees"].append((s, t))
        if t in funcs:
            funcs[t]["callers"].add(c)
    for f in funcs.values():
        f["callees"] = [t for _, t in sorted(f["callees"])]
    for r in tsv(os.path.join(pkg, "string-references.tsv")):
        try:
            a = int(r["function"], 16)
        except ValueError:
            continue
        v = r["value"]
        if a in funcs and v and len(v) >= 3:
            funcs[a]["strings"].add(v)
    return funcs


def windows():
    funcs = common(WIN_PKG)
    pe = pefile.PE(EXE)
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    lo, hi = base, base + len(img)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    md.detail = True
    for a, f in funcs.items():
        for s, e in f["ranges"]:
            code = img[s - base:e + 1 - base]
            for ins in md.disasm(code, s):
                for op in ins.operands:
                    if op.type == capstone.x86.X86_OP_IMM:
                        v = op.imm & 0xFFFFFFFF
                        if is_rare_const(v, lo, hi):
                            f["consts"].add(v)
    return funcs


IMM = re.compile(r"0x([0-9a-f]{5,8})\b")


def mac():
    funcs = common(MAC_PKG)
    starts = sorted((s, e, a) for a, f in funcs.items() for s, e in f["ranges"])
    lo, hi = 0x1000, 0x2000000  # Mac image is well below this; excludes code/data addresses
    import bisect
    keys = [s for s, _, _ in starts]
    with open(os.path.join(MAC_PKG, "instructions.asm"), encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith(";"):
                continue
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 3:
                continue
            addr = int(parts[0], 16)
            i = bisect.bisect_right(keys, addr) - 1
            if i < 0:
                continue
            s, e, owner = starts[i]
            if addr > e:
                continue
            asm = parts[2]
            # memory displacements like [EBX + 0x1234] are struct/PIC offsets, not IDs
            for m in IMM.finditer(asm):
                start = m.start()
                if "[" in asm[:start] and "]" in asm[start:]:
                    continue
                v = int(m.group(1), 16)
                if is_rare_const(v, lo, hi):
                    funcs[owner]["consts"].add(v)
    return funcs


def main():
    os.makedirs(OUT, exist_ok=True)
    which = sys.argv[1:] or ["win", "mac"]
    for w in which:
        f = windows() if w == "win" else mac()
        for v in f.values():
            v.pop("ranges", None)
        with open(os.path.join(OUT, f"{w}_features.pkl"), "wb") as fh:
            pickle.dump(f, fh)
        nc = sum(1 for v in f.values() if v["consts"])
        ns = sum(1 for v in f.values() if v["strings"])
        print(f"{w}: {len(f)} functions, {nc} with rare consts, {ns} with strings")


if __name__ == "__main__":
    main()
