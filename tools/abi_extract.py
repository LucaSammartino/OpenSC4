"""Extract interface (ABI) facts from the SC4 Windows exe's vtables.

For every named vtable in re/labels_windows.tsv, each slot's target function is analysed for:
  - callee_pop: bytes popped by `ret N` (exact; __thiscall/__stdcall argument size, excluding `this`)
  - ret: the return channel: x87 (float/double in ST0), edx:eax (64-bit), eax, void?, or unknown
These are functional interface facts (no code). Output: docs/abi/vtables.tsv.

Usage: python tools/abi_extract.py bin/SimCity4.exe [--outdir DIR] [--check VA ...]
"""
import csv
import os
import struct
import sys

import capstone
import pefile
from capstone import x86_const as X
from abi_returns import X87_PUSH, X87_POP1, X87_POP2, ret_label

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

pe = pefile.PE(sys.argv[1])
BASE = pe.OPTIONAL_HEADER.ImageBase
IMG = pe.get_memory_mapped_image()
TEXT = [(BASE + s.VirtualAddress, BASE + s.VirtualAddress + s.Misc_VirtualSize)
        for s in pe.sections if s.Characteristics & 0x20000000]
MD = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
MD.detail = True


def in_text(va):
    return any(a <= va < b for a, b in TEXT)


def u32(va):
    return struct.unpack_from("<I", IMG, va - BASE)[0]


_insn_cache = {}


def insn_at(va):
    if va not in _insn_cache:
        it = MD.disasm(IMG[va - BASE: va - BASE + 16], va, 1)
        _insn_cache[va] = next(it, None)
    return _insn_cache[va]


EAX_REGS = {X.X86_REG_EAX, X.X86_REG_AX, X.X86_REG_AL}
EDX_REGS = {X.X86_REG_EDX, X.X86_REG_DX, X.X86_REG_DL}
# x87 stack effects, for the backward walk from a `ret` (net = pushes - pops seen so far, walking back).
# A value is returned in ST0 only if, walking back, a push makes the net count positive, or an arithmetic
# instruction modifies ST0 while no later pop is pending (net == 0). A popping store (fstp/fistp) after the
# last load consumes the value: such a function returns nothing in ST0 (run 74 audit, 2026-10-14).
# no effect on the stack and not producing a result: stores without pop, compares, control/status words
X87_NO_RESULT = ("fst", "fist", "fnstsw", "fstsw", "fcom", "fucom", "fcomi", "fucomi", "ficom", "fnstcw",
                 "fstcw", "fldcw", "fnclex", "fclex", "ffree", "fwait", "wait", "fxch", "fxam", "ftst",
                 "fnsave", "fsave", "frstor", "fnstenv", "fstenv", "fldenv", "fninit", "finit", "fincstp",
                 "fdecstp", "fnop", "fxsave", "fxrstor")
NEUTRAL = ("pop", "leave", "nop", "push")  # frame teardown / harmless before ret


def writes(insn, regs):
    try:
        _, w = insn.regs_access()
    except capstone.CsError:
        return False
    return any(r in regs for r in w)


def is_esp_adjust(insn):
    if insn.mnemonic in ("add", "sub", "lea", "mov") and insn.operands:
        op0 = insn.operands[0]
        return op0.type == X.X86_OP_REG and op0.reg in (X.X86_REG_ESP, X.X86_REG_EBP)
    return False


class Func:
    def __init__(self, entry):
        self.entry = entry
        self.insns = {}        # va -> insn
        self.rets = []         # (va, popbytes)
        self.preds = {}        # target va -> [source va] for jumps
        self.tail = []         # tail-jump targets
        self.indirect_jump = False
        self.indirect_ops = []
        self.calls_before_ret = []
        self.truncated = False


FUNC_STARTS = set()
_func_cache = {}


def analyse(entry, budget=6000):
    if entry in _func_cache:
        return _func_cache[entry]
    f = Func(entry)
    _func_cache[entry] = f
    work = [entry]
    seen = set()
    while work:
        va = work.pop()
        while True:
            if va in seen:
                break
            if len(seen) > budget or not in_text(va):
                f.truncated = True
                break
            seen.add(va)
            ins = insn_at(va)
            if ins is None or ins.mnemonic == "int3":
                break
            f.insns[va] = ins
            m = ins.mnemonic
            if m in ("ret", "retn"):
                pop = ins.operands[0].imm if ins.operands else 0
                f.rets.append((va, pop))
                break
            if m == "jmp":
                op = ins.operands[0]
                if op.type == X.X86_OP_IMM:
                    t = op.imm
                    if t != entry and t in FUNC_STARTS:
                        f.tail.append(t)
                        break
                    f.preds.setdefault(t, []).append(va)
                    va = t
                    continue
                f.indirect_jump = True  # switch table or tail call through pointer
                f.indirect_ops.append(ins.op_str)
                # follow a jump table if it is [reg*4 + table]
                if op.type == X.X86_OP_MEM and op.mem.scale == 4 and op.mem.base == 0 and op.mem.disp:
                    tbl = op.mem.disp & 0xFFFFFFFF
                    for k in range(64):
                        try:
                            t = u32(tbl + 4 * k)
                        except struct.error:
                            break
                        if not in_text(t) or abs(t - entry) > 0x4000:
                            break
                        f.preds.setdefault(t, []).append(va)
                        work.append(t)
                break
            if m.startswith("j") or m in ("loop", "loope", "loopne", "jecxz"):
                op = ins.operands[0]
                if op.type == X.X86_OP_IMM:
                    f.preds.setdefault(op.imm, []).append(va)
                    work.append(op.imm)
            va = ins.address + ins.size
    return f


def reads(insn, regs):
    try:
        r, _ = insn.regs_access()
    except capstone.CsError:
        return False
    return any(x in regs for x in r)


def classify_from(f, start_va, depth=0, edx_state=None):
    """Scan backwards from just before start_va for the instruction that sets EAX (or EDX:EAX).

    x87 results are decided separately by x87_ret_depths (a forward walk); this scan skips x87
    instructions, except that FNSTSW AX writes AX and is not a return value.
    edx_state: None = EDX not touched yet (scanning backwards), "consumed" = read before the ret, so
    not a return value, "written" = written and not read afterwards, a candidate high half.
    """
    kinds = set()
    va = start_va
    for _ in range(60):
        prev = None
        for back in range(1, 16):
            p = f.insns.get(va - back)
            if p is not None and p.address + p.size == va:
                prev = p
                break
        jump_preds = f.preds.get(va, [])
        if depth < 4:
            for s in jump_preds:
                kinds |= classify_from(f, s, depth + 1, edx_state)
        if prev is None or prev.mnemonic in ("jmp", "ret", "retn"):
            return kinds or {"unknown"}
        ins = prev
        m = ins.mnemonic
        va = ins.address
        if m == "call":
            op = ins.operands[0]
            if op.type == X.X86_OP_IMM and depth < 4 and in_text(op.imm):
                _, ck, _ = summarize(op.imm, depth + 1)
                kinds |= {("callee:" + k) for k in ck if not k.endswith("x87")}
            else:
                kinds.add("via-call")
            return kinds
        if m.startswith("f") or m in ("wait", "emms"):
            continue
        if m.startswith("j"):
            continue
        if edx_state is None and reads(ins, EDX_REGS) and not writes(ins, EDX_REGS):
            edx_state = "consumed"
        elif edx_state is None and writes(ins, EDX_REGS):
            edx_state = "written"
        if writes(ins, EAX_REGS) and m not in NEUTRAL:
            kinds.add("edx:eax" if edx_state == "written" else "eax")
            return kinds
    kinds.add("unknown")
    return kinds


X87_STACKLESS = ("fnstsw", "fstsw", "fnstcw", "fstcw", "fldcw", "fnclex", "fclex", "fwait", "wait", "fninit",
                 "finit", "fnstenv", "fstenv", "fldenv", "fnsave", "fsave", "frstor", "fxsave", "fxrstor", "fnop")


_ST_RE = __import__("re").compile(r"st\((\d)\)")
X87_IMPLICIT2 = ("fcompp", "fucompp", "fcomp", "fcom", "fucom", "fucomp", "faddp", "fsubp", "fsubrp", "fmulp",
                 "fdivp", "fdivrp", "fxch", "fscale", "fprem", "fprem1", "fpatan", "fyl2x", "fyl2xp1")
X87_IMPLICIT0 = ("fldz", "fld1", "fldpi", "fldl2e", "fldl2t", "fldlg2", "fldln2")


def x87_required(ins):
    """How many values the x87 stack must hold for this instruction to be valid."""
    m = ins.mnemonic
    regs = [int(x) for x in _ST_RE.findall(ins.op_str)]
    if regs:
        return max(regs) + 1  # st(i) must exist, also for fld st(i)
    if "ptr" in ins.op_str:
        return 0 if m in X87_PUSH else 1
    if m in X87_IMPLICIT0:
        return 0
    if m in X87_IMPLICIT2:
        return 2
    return 1


def call_returns_x87(f, call_ins, depth):
    """Does the code after this call use an x87 value the call left in ST0?

    The x87 stack is empty at every call (calling convention), so a callee that returns in ST0 leaves
    exactly one value. Simulate the following code assuming the call left nothing: the first x87
    instruction that needs more values than that proves the call returned one. A call followed by a
    ret passes its result through: then the callee's own label decides (direct calls only).
    """
    va = call_ins.address + call_ins.size
    h = 0
    for _ in range(60):
        ins = f.insns.get(va)
        if ins is None:
            return False
        m = ins.mnemonic
        if m in ("ret", "retn"):
            if h:
                return False
            op = call_ins.operands[0]
            if op.type == X.X86_OP_IMM and depth < 4 and in_text(op.imm):
                _, ck, _ = summarize(op.imm, depth + 1)
                return ret_label(ck, {0})[0] == "x87"
            return False
        if m == "call":
            return False
        if m == "jmp":
            op = ins.operands[0]
            if op.type == X.X86_OP_IMM and op.imm in f.insns:
                va = op.imm
                continue
            return False
        if m.startswith("f") and m not in X87_STACKLESS:
            if x87_required(ins) > h:
                return True
            if m in X87_POP2:
                h -= 2
            elif m in X87_POP1:
                h -= 1
            elif m in X87_PUSH:
                h += 1
            if h == 0 and m in X87_POP1 + X87_POP2:
                return False  # everything this code pushed is gone again: the call left nothing it used
        va = ins.address + ins.size
    return False


def x87_ret_depths(f, depth=0):
    """Forward walk from the entry with the x87 stack depth (0 at entry, the __thiscall convention).

    Returns {ret va: set of depths reaching it}. Depth 1 at a ret = returns in ST0; 0 = nothing.
    At a call the depth is 0 (the convention), then 1 if the call returns in ST0 (call_returns_x87).
    Loops are cut by (va, depth) pairs; a depth outside 0..8 marks a path the walk cannot follow.
    """
    out = {}
    seen = set()
    work = [(f.entry, 0)]
    while work and len(seen) < 20000:
        va, d = work.pop()
        while (va, d) not in seen:
            seen.add((va, d))
            ins = f.insns.get(va)
            if ins is None:
                break
            m = ins.mnemonic
            if m in ("ret", "retn"):
                out.setdefault(va, set()).add(d)
                break
            if m.startswith("f"):
                if m in X87_POP2:
                    d -= 2
                elif m in X87_POP1:
                    d -= 1
                elif m in X87_PUSH:
                    d += 1
                if d < 0 or d > 8:
                    out.setdefault(va, set()).add(d)
                    break
            elif m == "emms":
                d = 0
            elif m == "call":
                d = 1 if call_returns_x87(f, ins, depth) else 0
            elif m == "jmp":
                op = ins.operands[0]
                if op.type == X.X86_OP_IMM and op.imm in f.insns:
                    va = op.imm
                    continue
                break  # tail jump or indirect: handled by summarize / notes
            elif m.startswith("j") or m in ("loop", "loope", "loopne", "jecxz"):
                op = ins.operands[0]
                if op.type == X.X86_OP_IMM and op.imm in f.insns:
                    work.append((op.imm, d))
            va = ins.address + ins.size
    return out


def near(f, va, n=4):
    out = []
    p = va
    for _ in range(n):
        q = None
        for back in range(1, 16):
            c = f.insns.get(p - back)
            if c is not None and c.address + c.size == p:
                q = c
                break
        if q is None:
            break
        out.append(q)
        p = q.address
    return out


_sum_cache = {}
_sum_active = set()


def summarize(entry, depth=0):
    if entry in _sum_cache:
        pops, kinds, notes = _sum_cache[entry]
        return set(pops), set(kinds), list(notes)  # callers extend notes
    if entry in _sum_active:  # recursion (directly or through a call chain): no answer yet
        return set(), {"unknown"}, []
    _sum_active.add(entry)
    try:
        r = _summarize(entry, depth)
    finally:
        _sum_active.discard(entry)
    if depth == 0:
        _sum_cache[entry] = (set(r[0]), set(r[1]), list(r[2]))
    return r


def _summarize(entry, depth=0):
    f = analyse(entry)
    pops = {p for _, p in f.rets}
    kinds = set()
    depths = x87_ret_depths(f, depth)
    for rva, _ in f.rets:
        ds = depths.get(rva, {0})
        if 1 in ds:
            kinds.add("x87")
        if ds - {1}:
            kinds |= classify_from(f, rva)
        if ds - {0, 1}:
            kinds.add("x87-depth:" + "/".join(str(x) for x in sorted(ds - {0, 1})))
    for t in f.tail:
        if depth < 4:
            tp, tk, _ = summarize(t, depth + 1)
            pops |= tp
            kinds |= tk
    notes = []
    if f.truncated:
        notes.append("truncated")
    if f.indirect_jump:
        notes.append("indirect-jump " + "/".join(f.indirect_ops[:2]))
    if f.tail:
        notes.append("tail->" + ",".join(f"{t:08x}" for t in f.tail))
    return pops, kinds, notes


def main():
    names = {}
    with open(os.path.join(ROOT, "re", "names_windows.tsv"), encoding="utf-8") as fh:
        for row in csv.reader(fh, delimiter="\t"):
            if row and row[0] != "va":
                try:
                    names[int(row[0], 16)] = row[1]
                except ValueError:
                    pass
    FUNC_STARTS.update(names)
    vtables = []
    with open(os.path.join(ROOT, "re", "labels_windows.tsv"), encoding="utf-8") as fh:
        for row in csv.reader(fh, delimiter="\t"):
            if len(row) >= 3 and "::vftable" in row[1]:
                vtables.append((int(row[0], 16), row[1], int(row[2] or 0)))
    # vtable targets are function starts too
    for va, _, n in vtables:
        for s in range(n):
            t = u32(va + 4 * s)
            if in_text(t):
                FUNC_STARTS.add(t)

    if "--check" in sys.argv:
        for a in sys.argv[sys.argv.index("--check") + 1:]:
            t = int(a, 16)
            pops, kinds, notes = summarize(t)
            print(f"{t:08x} pops={sorted(pops)} kinds={sorted(kinds)} -> {ret_label(kinds, pops)} {notes}")
        return

    outdir = os.path.join(ROOT, "docs", "abi")
    if "--outdir" in sys.argv:
        outdir = sys.argv[sys.argv.index("--outdir") + 1]
    os.makedirs(outdir, exist_ok=True)
    rows = 0
    with open(os.path.join(outdir, "vtables.tsv"), "w", newline="", encoding="utf-8") as out:
        w = csv.writer(out, delimiter="\t", lineterminator="\n")
        w.writerow(["class", "vtable", "slot", "offset", "target", "method", "callee_pop", "arg_dwords",
                    "ret", "ret_confidence", "notes"])
        for va, label, n in sorted(vtables):
            for s in range(n):
                t = u32(va + 4 * s)
                if not in_text(t):
                    continue
                if names.get(t) == "_purecall":
                    w.writerow([label, f"{va:08x}", s, f"+0x{4 * s:X}", f"{t:08x}", "_purecall", "", "", "pure", "", ""])
                    rows += 1
                    continue
                pops, kinds, notes = summarize(t)
                label_ret, conf = ret_label(kinds, pops, notes)
                if len(pops) > 1:
                    notes.append("inconsistent-pops")
                pop = min(pops) if pops else ""
                w.writerow([label, f"{va:08x}", s, f"+0x{4 * s:X}", f"{t:08x}", names.get(t, ""),
                            pop, (pop // 4) if pop != "" else "", label_ret, conf, ";".join(notes)])
                rows += 1
    print(f"wrote {rows} slot rows for {len(vtables)} vtables to docs/abi/vtables.tsv")

    # Interfaces the engine implements or calls: curated vtable list, slot count auto-detected when 0.
    starts = {va for va, _, _ in vtables}
    with open(os.path.join(ROOT, "re", "abi_interfaces.tsv"), encoding="utf-8") as fh:
        ifaces = [r for r in csv.DictReader(fh, delimiter="\t")]
    starts |= {int(r["vtable"], 16) for r in ifaces if r["vtable"]}
    n_rows = 0
    with open(os.path.join(outdir, "interfaces.tsv"), "w", newline="", encoding="utf-8") as out:
        w = csv.writer(out, delimiter="\t", lineterminator="\n")
        w.writerow(["interface", "game_class", "vtable", "slot", "offset", "target", "method", "callee_pop",
                    "arg_dwords", "ret", "ret_confidence", "notes"])
        for r in ifaces:
            va = int(r["vtable"], 16)
            n = int(r["slots"] or 0)
            if n == 0:
                while in_text(u32(va + 4 * n)) and (n == 0 or va + 4 * n not in starts) and n < 400:
                    n += 1
            for s in range(n):
                t = u32(va + 4 * s)
                if not in_text(t):
                    continue
                if names.get(t) == "_purecall":
                    w.writerow([r["interface"], r["class"], f"{va:08x}", s, f"+0x{4 * s:X}", f"{t:08x}",
                                "_purecall", "", "", "pure", "", ""])
                    continue
                pops, kinds, notes = summarize(t)
                label_ret, conf = ret_label(kinds, pops, notes)
                if len(pops) > 1:
                    notes.append("inconsistent-pops")
                pop = min(pops) if pops else ""
                w.writerow([r["interface"], r["class"], f"{va:08x}", s, f"+0x{4 * s:X}", f"{t:08x}",
                            names.get(t, ""), pop, (pop // 4) if pop != "" else "", label_ret, conf,
                            ";".join(notes)])
                n_rows += 1
    print(f"wrote {n_rows} slot rows for {len(ifaces)} interfaces to docs/abi/interfaces.tsv")


if __name__ == "__main__":
    main()
