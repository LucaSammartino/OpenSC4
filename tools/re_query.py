#!/usr/bin/env python3
"""Read-only queries over the Ghidra export of SimCity 4.exe, for RE lanes running in parallel (RE side only).

One process per query, no server, no lock: each table of ghidra/export/windows (and re/names_windows.tsv,
docs/abi/vtables.tsv) is parsed once into a pickle under build-runs/re-query-cache/ of the main checkout, written to a
temp file and os.replace'd, so concurrent first runs at worst build the same file twice. A cache file is rebuilt when
the size or mtime of a source changes. Nothing is written outside the cache directory.

Usage: python tools/re_query.py [--repo DIR] [--limit N] <command> ...
  func <addr|name>...          containing function: start+offset, size, name (source/confidence), owner class, proto
  asm <addr|name> [--max N]    the function's instructions (instructions.asm), call/jump targets and strings named;
                               an address outside every function prints N lines from there (capstone if undecoded)
  callers <addr|name>          direct calls to the function (calls.tsv): callsite, caller+offset
  callees <addr|name>          direct calls made by the function, in address order
  xrefs <addr|name>            data references to the address (data-references.tsv + string refs), with the
                               containing function of each referrer; --from lists the references made from inside
                               the function instead
  strings <regex>              strings matching the regex, with every referencing function
  search <name-regex>          functions (names_windows + Ghidra) and, with --all, labels/imports whose name matches
  vtable <class-regex|addr>    docs/abi/vtables.tsv: slots of the matching class vtables; for an address, the vtable
                               at that address or every vtable slot that points at that function
Addresses: hex with or without 0x (005e55e0, 0x5E55E0), FUN_xxxxxxxx, or SimCity 4.exe+0x... offsets.
Names: an exact name, else a unique regex match (several matches are listed instead).
"""
import argparse
import bisect
import hashlib
import os
import pickle
import re
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import coverage  # noqa: E402  (main_checkout, owner_of_name: the project's own rules)

REPO = Path(__file__).resolve().parents[1]
DEFAULT_MAIN = Path(r"C:\Users\tlori\FreeCity4")
EXPORT_REL = Path("ghidra/export/windows")
CACHE_VERSION = 1
IMAGE_BASE = 0x400000
ASM_STRIDE = 32  # one index entry per 32 instruction lines
HEX8 = re.compile(r"[0-9a-fA-F]{8}")


# ---------------------------------------------------------------- locations


def main_checkout(repo):
    """The main checkout of a worktree without spawning git: <repo>/.git is a 'gitdir: <main>/.git/worktrees/x' file."""
    g = repo / ".git"
    if g.is_file():
        m = re.match(r"gitdir:\s*(.+)", g.read_text(encoding="utf-8", errors="replace").strip())
        if m:
            p = Path(m.group(1))
            for parent in p.parents:
                if parent.name == ".git":
                    return parent.parent
    if g.is_dir():
        return repo
    return coverage.main_checkout(repo)


class Paths:
    def __init__(self, repo_arg):
        here = Path(repo_arg).resolve() if repo_arg else REPO
        roots = [here, main_checkout(here), DEFAULT_MAIN]
        self.export = next((r / EXPORT_REL for r in roots if (r / EXPORT_REL / "functions.tsv").exists()), None)
        if self.export is None:
            sys.exit("re_query: ghidra/export/windows not found in %s (tools/setup_ghidra.ps1 -Windows builds it)"
                     % ", ".join(map(str, roots)))
        # committed tables: the lane's own copy first (it may carry new names), else the main checkout's
        self.names = next((r / "re/names_windows.tsv" for r in roots if (r / "re/names_windows.tsv").exists()), None)
        self.vtables = next((r / "docs/abi/vtables.tsv" for r in roots if (r / "docs/abi/vtables.tsv").exists()), None)
        self.exe = next((r / "bin/SimCity4.exe" for r in roots if (r / "bin/SimCity4.exe").exists()), None)
        main = self.export.parents[2]
        self.cache = main / "build-runs" / "re-query-cache"
        owners = main / "build-runs" / "coverage-owners.tsv"
        self.owners = owners if owners.exists() else None


# ---------------------------------------------------------------- cache


def _stamp(paths):
    out = []
    for p in paths:
        st = os.stat(p)
        out.append((str(p), st.st_size, st.st_mtime_ns))
    return out


def cached(paths, table, sources, build):
    """The table built from sources, from the pickle cache when its stamp matches; rebuilt and replaced atomically."""
    sources = [s for s in sources if s is not None]
    stamp = _stamp(sources)
    f = paths.cache / f"{table}-{_path_key(stamp)}.pkl"
    try:
        with open(f, "rb") as fh:
            v, st, data = pickle.load(fh)
        if v == CACHE_VERSION and st == stamp:
            return data
    except (OSError, EOFError, pickle.UnpicklingError, ValueError, TypeError):
        pass
    data = build(*sources)
    try:
        paths.cache.mkdir(parents=True, exist_ok=True)
        fd, tmp = tempfile.mkstemp(prefix=f"{table}-", suffix=".tmp", dir=paths.cache)
        with os.fdopen(fd, "wb") as fh:
            pickle.dump((CACHE_VERSION, stamp, data), fh, protocol=pickle.HIGHEST_PROTOCOL)
        for _ in range(20):  # Windows: replace fails while another process has the target open; it is only a cache
            try:
                os.replace(tmp, f)
                break
            except PermissionError:
                time.sleep(0.05)
        else:
            os.unlink(tmp)
    except OSError:
        pass
    return data


def _path_key(stamp):
    """Sources from different checkouts (a lane's own names_windows.tsv) get their own cache file."""
    return hashlib.sha1("|".join(s[0].lower() for s in stamp).encode()).hexdigest()[:10]


def tsv_rows(path):
    with open(path, encoding="utf-8", errors="replace", newline="") as f:
        header = None
        for line in f:
            if not line.strip() or line.startswith("#"):
                continue
            col = line.rstrip("\r\n").split("\t")
            if header is None:
                header = col
                continue
            yield col


# ---------------------------------------------------------------- tables


def build_funcs(functions_tsv, details_tsv, names_tsv=None, owners_tsv=None):
    size, gname, thunk = {}, {}, set()
    for c in tsv_rows(functions_tsv):
        if not HEX8.fullmatch(c[0]):
            continue
        va = int(c[0], 16)
        size[va] = int(c[2])
        gname[va] = c[1]
        if c[3] == "true":
            thunk.add(va)
    rng, proto = [], {}
    for c in tsv_rows(details_tsv):
        if not HEX8.fullmatch(c[0]):
            continue
        va = int(c[0], 16)
        if not c[2].startswith("undefined "):
            proto[va] = c[2]
        for part in c[4].split(";"):
            if "-" in part:
                a, b = part.split("-")
                rng.append((int(a, 16), int(b, 16), va))
    rng.sort()
    names = {}
    if names_tsv:
        for c in tsv_rows(names_tsv):
            if HEX8.fullmatch(c[0]):
                names[int(c[0], 16)] = (c[1], c[2] if len(c) > 2 else "", c[3] if len(c) > 3 else "")
    owners = {}
    if owners_tsv:
        for c in tsv_rows(owners_tsv):
            if HEX8.fullmatch(c[0]) and len(c) > 3:
                owners[int(c[0], 16)] = (c[2], c[3])
    return {"size": size, "gname": gname, "thunk": thunk, "rng_lo": [r[0] for r in rng], "rng": rng,
            "proto": proto, "names": names, "owners": owners}


def build_calls(calls_tsv):
    by_caller, by_target = {}, {}
    for c in tsv_rows(calls_tsv):
        if len(c) < 4 or not HEX8.fullmatch(c[0]):
            continue
        caller, site = int(c[0], 16), int(c[1], 16)
        target = int(c[2], 16) if HEX8.fullmatch(c[2]) else c[2]
        kind = "call" if c[3] == "UNCONDITIONAL_CALL" else c[3].lower()
        by_caller.setdefault(caller, []).append((site, target, kind))
        by_target.setdefault(target, []).append((site, caller, kind))
    return {"by_caller": by_caller, "by_target": by_target}


def build_drefs(dref_tsv):
    to, frm = {}, {}
    for c in tsv_rows(dref_tsv):
        try:
            a, b = int(c[0], 16), int(c[1], 16)
        except (ValueError, IndexError):
            continue
        to.setdefault(b, []).append(a)
        frm.setdefault(a, []).append(b)
    return {"to": to, "from": frm}


def build_strings(str_tsv):
    strings = {}  # string address -> [value, [(function, from)]]
    for c in tsv_rows(str_tsv):
        if len(c) < 4 or not HEX8.fullmatch(c[2]):
            continue
        s = strings.setdefault(int(c[2], 16), ["\t".join(c[3:]), []])
        fn = int(c[0], 16) if HEX8.fullmatch(c[0]) else None
        s[1].append((fn, int(c[1], 16) if HEX8.fullmatch(c[1]) else None))
    return strings


def build_symbols(sym_tsv):
    """Non-default labels and imports (Ghidra's own analysis names); LAB_/DAT_/switch labels are dropped."""
    out = {}
    for c in tsv_rows(sym_tsv):
        if len(c) < 4 or c[2] == "DEFAULT" or c[1] == "Function" and c[2] != "IMPORTED":
            continue
        if c[3].startswith(("switchD_", "$L")):
            continue
        key = int(c[0], 16) if HEX8.fullmatch(c[0]) else c[0]
        out.setdefault(key, c[3])
    return out


def build_asm_index(asm_path):
    """Sorted (address, byte offset) of every ASM_STRIDE-th instruction line of instructions.asm."""
    addrs, offs = [], []
    n = 0
    with open(asm_path, "rb") as f:
        off = 0
        for line in f:
            if line[:1] != b";" and len(line) > 9:
                if n % ASM_STRIDE == 0:
                    addrs.append(int(line[:8], 16))
                    offs.append(off)
                n += 1
            off += len(line)
    return {"addrs": addrs, "offs": offs, "lines": n}


def build_vtables(vt_tsv):
    rows = []
    for c in tsv_rows(vt_tsv):
        if len(c) < 6:
            continue
        rows.append((c[0], c[1], c[2], c[3], c[4], c[5]))  # class, vtable, slot, offset, target, method
    return rows


class DB:
    def __init__(self, paths):
        self.p = paths
        self._t = {}

    def _get(self, table, sources, build):
        if table not in self._t:
            self._t[table] = cached(self.p, table, sources, build)
        return self._t[table]

    @property
    def f(self):
        e = self.p.export
        return self._get("funcs", [e / "functions.tsv", e / "function-details.tsv", self.p.names, self.p.owners],
                         build_funcs)

    @property
    def calls(self):
        return self._get("calls", [self.p.export / "calls.tsv"], build_calls)

    @property
    def drefs(self):
        return self._get("drefs", [self.p.export / "data-references.tsv"], build_drefs)

    @property
    def strings(self):
        return self._get("strings", [self.p.export / "string-references.tsv"], build_strings)

    @property
    def symbols(self):
        return self._get("symbols", [self.p.export / "symbols.tsv"], build_symbols)

    @property
    def asmidx(self):
        return self._get("asmidx", [self.p.export / "instructions.asm"], build_asm_index)

    @property
    def vtables(self):
        if self.p.vtables is None:
            sys.exit("re_query: docs/abi/vtables.tsv not found")
        return self._get("vtables", [self.p.vtables], build_vtables)

    # ------------------------------------------------ lookups

    def containing(self, va):
        f = self.f
        i = bisect.bisect_right(f["rng_lo"], va) - 1
        for k in (i, i - 1, i - 2):
            if k >= 0 and f["rng"][k][0] <= va <= f["rng"][k][1]:
                return f["rng"][k][2]
        return None

    def fname(self, va):
        f = self.f
        if va in f["names"]:
            return f["names"][va][0]
        return f["gname"].get(va)

    def label(self, va):
        """name+0xoff for any address: its function, else a symbol, else the bare address."""
        if isinstance(va, str):
            return self.symbols.get(va, va)
        s = self.containing(va)
        if s is not None:
            n = self.fname(s)
            return n if va == s else f"{n}+0x{va - s:x}"
        if va in self.f["size"] or va in self.f["names"]:
            return self.fname(va)
        n = self.symbols.get(va)
        return n if n else f"{va:08x}"

    def named(self, va):
        """'addr name' (or just 'addr' when nothing names it)."""
        if isinstance(va, str):
            return self.label(va)
        lab = self.label(va)
        return f"{va:08x}" if lab == f"{va:08x}" else f"{va:08x} {lab}"

    def owner(self, va):
        f = self.f
        if va in f["owners"]:
            o, rule = f["owners"][va]
            return f"{o} ({rule})"
        n = self.fname(va) or ""
        if n.startswith("FUN_") or n.startswith("thunk_FUN_"):
            return None
        o = coverage.owner_of_name(n)
        return None if o.startswith("fn:") else o

    def resolve(self, arg, want_function=True):
        """An address from hex / FUN_ / exe+offset / name; for names, exact match else unique regex match."""
        a = arg.strip()
        m = re.fullmatch(r"(?:.*\.exe\+|\(executable\)\+)0x([0-9a-fA-F]+)", a)
        if m:
            return IMAGE_BASE + int(m.group(1), 16)
        m = re.fullmatch(r"(?:FUN_|LAB_|DAT_|0x)?([0-9a-fA-F]{1,8})", a)
        if m and (a.lower().startswith(("0x", "fun_", "lab_", "dat_")) or len(m.group(1)) >= 6):
            return int(m.group(1), 16)
        f = self.f
        exact = sorted({va for va, n in f["names"].items() if n[0] == a} | {va for va, n in f["gname"].items() if n == a})
        if not exact and not want_function:
            exact = sorted(va for va, n in self.symbols.items() if n == a and isinstance(va, int))
        if len(exact) == 1:
            return exact[0]
        if exact:
            hits = exact
        else:
            try:
                rx = re.compile(a)
            except re.error as e:
                sys.exit(f"re_query: {a!r} is not an address, a name or a regex ({e})")
            hits = sorted({va for va, n in f["names"].items() if rx.search(n[0])}
                          | {va for va, n in f["gname"].items() if not n.startswith("FUN_") and rx.search(n)})
            if len(hits) == 1:
                return hits[0]
        if not hits:
            sys.exit(f"re_query: no function named {a!r}")
        print(f"re_query: {a!r} matches {len(hits)} functions; pick an address:")
        for va in hits[:20]:
            print(f"  {va:08x}  {self.fname(va)}")
        if len(hits) > 20:
            print(f"  ... {len(hits) - 20} more (use: search)")
        sys.exit(1)

    def func_of(self, arg):
        va = self.resolve(arg)
        s = self.containing(va)
        if s is None and va in self.f["size"]:
            s = va
        return va, s


# ---------------------------------------------------------------- commands


def regex(text, case):
    try:
        return re.compile(text, 0 if case else re.I)
    except re.error as e:
        sys.exit(f"re_query: bad regex {text!r}: {e}")


def more(n, limit):
    if n > limit:
        print(f"... {n - limit} more (--limit N)")


def cmd_func(db, args):
    f = db.f
    for a in args.targets:
        va, s = db.func_of(a)
        if s is None:
            line = f"{va:08x}  not inside a Ghidra function"
            if va in f["names"]:
                n = f["names"][va]
                line = f"{va:08x}  {n[0]}  [{n[1]}/{n[2]}]  (no Ghidra function here: asm prints from the address)"
            elif va in db.symbols:
                line += f"  [{db.symbols[va]}]"
            i = bisect.bisect_right(db.f["rng_lo"], va) - 1
            if i >= 0:
                lo, hi, fn = db.f["rng"][i]
                line += f"  prev={fn:08x} {db.fname(fn)} ends {hi:08x}"
            print(line)
            continue
        name = db.fname(s)
        src = f["names"].get(s)
        bits = [f"{s:08x}" + (f"+0x{va - s:x}" if va != s else ""), f"size={f['size'].get(s, 0)}", name]
        if src:
            bits.append(f"[{src[1]}/{src[2]}]" if src[1] else "")
        if src and f["gname"].get(s) and not f["gname"][s].startswith("FUN_") and f["gname"][s] != src[0]:
            bits.append(f"ghidra={f['gname'][s]}")
        if s in f["thunk"]:
            bits.append("thunk")
        o = db.owner(s)
        if o:
            bits.append(f"owner={o}")
        if s in f["proto"]:
            bits.append(f"proto={f['proto'][s]}")
        print("  ".join(b for b in bits if b))


def asm_lines(db, lo, hi, limit):
    """Lines of instructions.asm with lo <= address <= hi (at most limit)."""
    idx = db.asmidx
    i = bisect.bisect_right(idx["addrs"], lo) - 1
    off = idx["offs"][max(i, 0)]
    out = []
    with open(db.p.export / "instructions.asm", "rb") as fh:
        fh.seek(off)
        for raw in fh:
            if raw[:1] == b";":
                continue
            a = int(raw[:8], 16)
            if a < lo:
                continue
            if a > hi or len(out) >= limit:
                break
            col = raw.decode("utf-8", "replace").rstrip("\r\n").split("\t")
            out.append((a, col[1] if len(col) > 1 else "", col[2] if len(col) > 2 else ""))
    return out


IMM = re.compile(r"0x([0-9a-fA-F]{6,8})\b")


def annotate(db, text, here_fn):
    notes = []
    for m in IMM.finditer(text):
        v = int(m.group(1), 16)
        if v in db.f["size"]:
            if v != here_fn:
                notes.append(db.fname(v))
        elif v in db.strings:
            s = db.strings[v][0]
            notes.append('"' + (s[:60] + "..." if len(s) > 60 else s) + '"')
        elif v in db.symbols:
            notes.append(db.symbols[v])
        elif v >= 0x401000:
            s = db.containing(v)
            if s is not None and s != here_fn:
                notes.append(db.label(v))
    return ("  ; " + ", ".join(notes)) if notes else ""


def cmd_asm(db, args):
    va, s = db.func_of(args.target)
    limit = args.max
    if s is not None:
        rngs = [(lo, hi) for lo, hi, fn in db.f["rng"] if fn == s]
        print(f"; {s:08x} {db.fname(s)}  size={db.f['size'].get(s, 0)}  ranges=" +
              ",".join(f"{lo:08x}-{hi:08x}" for lo, hi in rngs))
        lines = []
        for lo, hi in rngs:
            lines += asm_lines(db, lo, hi, limit + 1 - len(lines))
    else:
        lines = asm_lines(db, va, 0xFFFFFFFF, limit + 1)
        if not lines or lines[0][0] != va:
            if db.p.exe is None:
                sys.exit(f"re_query: {va:08x} is not a decoded instruction and bin/SimCity4.exe is missing")
            import x86dis  # capstone over the exe: for bytes Ghidra did not decode
            print(f"; {va:08x} not decoded by Ghidra; capstone:")
            for a, mn, op in x86dis.disasm(str(db.p.exe), va, limit * 6)[:limit]:
                print(f"{a:08x}  {mn} {op}")
            return
        print(f"; {va:08x} is not inside a function; {limit} lines from there")
    for a, byts, text in lines[:limit]:
        print(f"{a:08x}  " + (f"{byts:<16s} " if args.bytes else "") + text + annotate(db, text, s))
    if len(lines) > limit:
        print(f"... truncated at --max {limit}")


def cmd_callers(db, args):
    va, s = db.func_of(args.target)
    target = s if s is not None else va
    rows = sorted(db.calls["by_target"].get(target, []))
    print(f"; {len(rows)} direct calls to {db.named(target)}")
    for site, caller, kind in rows[:args.limit]:
        print(f"{site:08x}  {db.label(site)}" + ("" if kind == "call" else f"  [{kind}]"))
    more(len(rows), args.limit)


def cmd_callees(db, args):
    va, s = db.func_of(args.target)
    if s is None:
        sys.exit(f"re_query: {va:08x} is not inside a function")
    rows = sorted(db.calls["by_caller"].get(s, []), key=lambda r: r[0])
    print(f"; {len(rows)} calls from {s:08x} {db.fname(s)}")
    for site, target, kind in rows[:args.limit]:
        print(f"{site:08x} -> {db.named(target)}" + ("" if kind == "call" else f"  [{kind}]"))
    more(len(rows), args.limit)


def cmd_xrefs(db, args):
    va = db.resolve(args.target, want_function=False)
    if args.frm:
        s = db.containing(va)
        if s is None:
            sys.exit(f"re_query: {va:08x} is not inside a function")
        rngs = [(lo, hi) for lo, hi, fn in db.f["rng"] if fn == s]
        rows = []
        for a, tos in db.drefs["from"].items():
            if any(lo <= a <= hi for lo, hi in rngs):
                rows += [(a, t) for t in tos]
        for addr, (val, refs) in db.strings.items():
            rows += [(fr, addr) for fn, fr in refs if fn == s and fr is not None and (fr, addr) not in rows]
        rows.sort()
        print(f"; {len(rows)} data references from {s:08x} {db.fname(s)}")
        for a, t in rows[:args.limit]:
            extra = f'  "{db.strings[t][0][:60]}"' if t in db.strings else ""
            print(f"{a:08x} -> {db.named(t)}{extra}")
        more(len(rows), args.limit)
        return
    rows = {(a, "data") for a in db.drefs["to"].get(va, [])}
    if va in db.strings:
        rows |= {(fr, "string") for fn, fr in db.strings[va][1] if fr is not None}
    rows = sorted(rows)
    head = f"; {len(rows)} data references to {db.named(va)}"
    if va in db.strings:
        head += f'  "{db.strings[va][0][:80]}"'
    print(head)
    for a, kind in rows[:args.limit]:
        print(f"{a:08x}  {db.label(a)}" + ("" if kind == "data" else "  [string-ref]"))
    more(len(rows), args.limit)


def cmd_strings(db, args):
    rx = regex(args.regex, args.case)
    hits = sorted((a, v) for a, v in db.strings.items() if rx.search(v[0]))
    for a, (val, refs) in hits[:args.limit]:
        fns = sorted({fn for fn, _ in refs if fn is not None})
        shown = ", ".join(f"{fn:08x} {db.fname(fn)}" for fn in fns[:6]) + (f", +{len(fns) - 6}" if len(fns) > 6 else "")
        print(f'{a:08x}  "{val[:100]}"  <- {shown}')
    more(len(hits), args.limit)


def cmd_search(db, args):
    rx = regex(args.regex, args.case)
    f = db.f
    hits = {}
    for va, n in f["names"].items():
        if rx.search(n[0]):
            hits[va] = (n[0], n[1])
    for va, n in f["gname"].items():
        if va not in hits and not n.startswith("FUN_") and rx.search(n):
            hits[va] = (n, "ghidra")
    if args.all:
        for va, n in db.symbols.items():
            if isinstance(va, int) and va not in hits and rx.search(n):
                hits[va] = (n, "label")
            elif isinstance(va, str) and rx.search(n):
                hits[va] = (n, "import")
    keys = sorted(hits, key=lambda k: (isinstance(k, str), k if isinstance(k, int) else 0, str(k)))
    for va in keys[:args.limit]:
        n, src = hits[va]
        addr = f"{va:08x}" if isinstance(va, int) else va
        size = f["size"].get(va) if isinstance(va, int) else None
        print(f"{addr}  " + (f"size={size}  " if size is not None else "") + f"{n}  [{src}]")
    more(len(keys), args.limit)


def cmd_vtable(db, args):
    rows = db.vtables
    a = args.target.strip()
    m = re.fullmatch(r"(?:0x)?([0-9a-fA-F]{6,8})", a)
    if m:
        va = int(m.group(1), 16)
        sel = [r for r in rows if r[1] and HEX8.fullmatch(r[1]) and int(r[1], 16) == va]
        if not sel:  # a function: every vtable slot that points at it
            sel = [r for r in rows if r[4] and HEX8.fullmatch(r[4]) and int(r[4], 16) == va]
            print(f"; {len(sel)} vtable slots point at {db.named(va)}")
            for r in sel[:args.limit]:
                print(f"{r[1]}  slot {r[2]:>3} {r[3]:>6}  {r[0]}")
            more(len(sel), args.limit)
            return
    else:
        rx = regex(a, True)
        classes = sorted({r[0] for r in rows if rx.search(re.sub(r"::vftable(_\d+)?$", "", r[0]))})
        exact = [c for c in classes if re.sub(r"::vftable(_\d+)?$", "", c) == a]
        if exact:
            classes = exact
        sel = [r for r in rows if r[0] in set(classes)]
        if len({r[1] for r in sel}) > 8 and not exact:
            print(f"; {a!r} matches {len(classes)} vtables; narrow it:")
            for c in classes[:args.limit]:
                print("  " + c)
            more(len(classes), args.limit)
            return
    cur = None
    shown = 0
    for r in sel:
        if (r[0], r[1]) != cur:
            cur = (r[0], r[1])
            print(f"; {r[0]} @ {r[1]}")
        if shown < args.limit:
            print(f"{r[2]:>3} {r[3]:>6}  {r[4]}  {r[5]}")
        shown += 1
    more(shown, args.limit)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="re_query.py", description=__doc__.split("\n")[0])
    ap.add_argument("--repo", help="checkout holding ghidra/export/windows (default: this one, its main checkout, "
                                   f"{DEFAULT_MAIN})")
    ap.add_argument("--limit", type=int, default=40, help="max lines for list commands (default 40)")
    common = argparse.ArgumentParser(add_help=False)  # --limit / --repo also accepted after the command
    common.add_argument("--limit", type=int, default=argparse.SUPPRESS)
    common.add_argument("--repo", default=argparse.SUPPRESS)
    sub = ap.add_subparsers(dest="cmd", required=True)
    _add = sub.add_parser
    sub.add_parser = lambda name, **kw: _add(name, parents=[common], **kw)
    p = sub.add_parser("func")
    p.add_argument("targets", nargs="+")
    p = sub.add_parser("asm")
    p.add_argument("target")
    p.add_argument("--max", type=int, default=400)
    p.add_argument("--bytes", action="store_true", help="also print the instruction bytes")
    for c in ("callers", "callees"):
        sub.add_parser(c).add_argument("target")
    p = sub.add_parser("xrefs")
    p.add_argument("target")
    p.add_argument("--from", dest="frm", action="store_true")
    for c in ("strings", "search"):
        p = sub.add_parser(c)
        p.add_argument("regex")
        p.add_argument("--case", action="store_true", help="case-sensitive")
        if c == "search":
            p.add_argument("--all", action="store_true", help="also Ghidra labels and imports")
    sub.add_parser("vtable").add_argument("target")
    sub.add_parser("warm", help="build every cache table now")
    args = ap.parse_args(argv)
    db = DB(Paths(args.repo))
    if args.cmd == "warm":
        for t in ("f", "calls", "drefs", "strings", "symbols", "asmidx", "vtables"):
            getattr(db, t)
        print(f"re_query: cache ready in {db.p.cache}")
        return 0
    {"func": cmd_func, "asm": cmd_asm, "callers": cmd_callers, "callees": cmd_callees, "xrefs": cmd_xrefs,
     "strings": cmd_strings, "search": cmd_search, "vtable": cmd_vtable}[args.cmd](db, args)
    return 0


if __name__ == "__main__":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except AttributeError:
        pass
    try:
        sys.exit(main())
    except BrokenPipeError:
        sys.exit(0)
