"""Extract vtables from both builds for slot-aligned symbol porting.

Windows: a vtable start is an .rdata address installed by `mov dword ptr [..], imm32` whose first
entry is a code pointer; it runs until a non-code entry or the next installed start.

Mac (GCC): every __ZTV symbol; entries come from data-references. A GCC vtable group is
[offset_to_top][typeinfo][slots...] repeated for each secondary base; we split sub-tables at
entries that are not function pointers. Each slot is normalized:
  * `__ZThn<N>_...` / `__ZTv...` thunks -> the demangled target name + resolved target address
  * the D1 (complete) destructor is dropped so the D0 (deleting) one lines up with MSVC/ICC's
    single scalar-deleting destructor
  * runs of same-named methods (overloads) are reversed to mirror MSVC slot order

Output: re/symport/vtables.pkl = {"win": {start: [addr...]}, "mac": [(ztv_name, [(name, addr)...])]}
"""
import bisect
import csv
import os
import pickle
import re
import struct

import capstone
import pefile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MAC_PKG = os.path.join(ROOT, "ghidra", "export", "mac")  # tools/setup_ghidra.ps1 -Mac
OUT = os.path.join(ROOT, "re", "symport")
EXE = os.path.join(ROOT, "bin", "SimCity4.exe")
csv.field_size_limit(1 << 30)


def tsv(path):
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        yield from csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE)


def win_vtables():
    pe = pefile.PE(EXE)
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    text = next(s for s in pe.sections if s.Name.startswith(b".text"))
    tlo, thi = base + text.VirtualAddress, base + text.VirtualAddress + text.Misc_VirtualSize
    rd = next(s for s in pe.sections if s.Name.startswith(b".rdata"))
    rlo, rhi = base + rd.VirtualAddress, base + rd.VirtualAddress + rd.Misc_VirtualSize

    def u32(va):
        return struct.unpack_from("<I", img, va - base)[0]

    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    starts = set()
    code = img[text.VirtualAddress:text.VirtualAddress + text.Misc_VirtualSize]
    # C7 /0 with imm32: mov dword ptr [mem], imm32. Scan for candidate imm32 values in .rdata.
    for m in re.finditer(rb"\xc7", code):
        off = m.start()
        for ins in md.disasm(code[off:off + 11], tlo + off, count=1):
            if ins.mnemonic == "mov" and ins.op_str.startswith("dword ptr ["):
                try:
                    v = int(ins.op_str.rsplit(", ", 1)[1], 16)
                except ValueError:
                    continue
                if rlo <= v < rhi and tlo <= u32(v) < thi:
                    starts.add(v)
    ordered = sorted(starts)
    out = {}
    for i, s in enumerate(ordered):
        nxt = ordered[i + 1] if i + 1 < len(ordered) else rhi
        slots, a = [], s
        while a < nxt and tlo <= u32(a) < thi:
            slots.append(u32(a))
            a += 4
        out[s] = slots
    return out


THUNK = re.compile(r"^__ZTh[nv]\d+_N?(?:K)?(\d+)(\w+?)(\d+)(\w+)")


def mac_vtables():
    names, addrs = {}, []
    for r in tsv(os.path.join(MAC_PKG, "functions.tsv")):
        a = int(r["address"], 16)
        names[a] = r["name"]
    ztv = []
    for r in tsv(os.path.join(MAC_PKG, "symbols.tsv")):
        n = r["name"]
        if n.startswith("__ZTV") and not r["address"].startswith("EXTERNAL"):
            ztv.append((int(r["address"], 16), n))
    ztv.sort()
    zstarts = [a for a, _ in ztv]
    entries = {}
    for r in tsv(os.path.join(MAC_PKG, "data-references.tsv")):
        try:
            f, t = int(r["from"], 16), int(r["to"], 16)
        except ValueError:
            continue
        i = bisect.bisect_right(zstarts, f) - 1
        if i >= 0 and f - zstarts[i] < 0x2000:
            entries[f] = t
    # resolve "non-virtual thunk" functions to their targets via their single call/jump
    thunk_target = {}
    for r in tsv(os.path.join(MAC_PKG, "calls.tsv")):
        try:
            c, t = int(r["caller"], 16), int(r["target"], 16)
        except ValueError:
            continue
        n = names.get(c, "")
        if n.startswith("__ZTh") or n.startswith("non-virtual thunk") or n.startswith("virtual thunk"):
            thunk_target.setdefault(c, t)

    def norm(a):
        n = names.get(a)
        if n is None:
            return ("?", a)
        if a in thunk_target and thunk_target[a] in names:
            return (names[thunk_target[a]], thunk_target[a])
        return (n, a)

    groups = []
    for i, (s, zname) in enumerate(ztv):
        end = ztv[i + 1][0] if i + 1 < len(ztv) else s + 0x2000
        a, cur, subs = s, [], []
        while a < end:
            t = entries.get(a)
            if t is not None and t in names:
                cur.append(norm(t))
            elif t is None and cur and a + 4 < end and entries.get(a + 4) in names:
                # a zero/unknown slot (e.g. __cxa_pure_virtual resolved oddly) inside a table
                cur.append(("?", 0))
            else:
                if cur:
                    subs.append(cur)
                cur = []
            a += 4
        if cur:
            subs.append(cur)
        for k, sub in enumerate(subs):
            groups.append((f"{zname}#{k}", fix_abi(sub)))
    return groups


def short(n):
    return n.split("(")[0]


def fix_abi(sub):
    out, i = [], 0
    # drop D1 (complete-object) dtor when followed by another dtor of the same class
    while i < len(sub):
        n, a = sub[i]
        if "~" in n and i + 1 < len(sub) and "~" in sub[i + 1][0] and short(sub[i + 1][0]) == short(n):
            out.append(sub[i + 1])
            i += 2
            continue
        out.append((n, a))
        i += 1
    # reverse runs of same-named overloads (MSVC/ICC slot order)
    res, i = [], 0
    while i < len(out):
        j = i
        while j + 1 < len(out) and short(out[j + 1][0]) == short(out[i][0]) and out[i][0] != "?" \
                and out[j + 1][1] != out[i][1]:
            j += 1
        run = out[i:j + 1]
        res.extend(reversed(run) if len(run) > 1 else run)
        i = j + 1
    return res


def main():
    os.makedirs(OUT, exist_ok=True)
    w = win_vtables()
    m = mac_vtables()
    with open(os.path.join(OUT, "vtables.pkl"), "wb") as fh:
        pickle.dump({"win": w, "mac": m}, fh)
    print(f"windows vtables: {len(w)} ({sum(len(v) for v in w.values())} slots)")
    print(f"mac sub-vtables: {len(m)} ({sum(len(v) for _, v in m)} slots)")


if __name__ == "__main__":
    main()
