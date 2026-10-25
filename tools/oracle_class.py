"""Static run-7 oracle classes, ported from the coordinator's reference scan.

Precedence: OS effects > indirect dispatch > writable globals > closed. These
are planning classes, not evidence of equivalence. The reference's KERNEL32
allowlist and writable-data bounds are deliberately preserved. Scan library
callees too, so a direct library call cannot hide an OS/global/indirect effect.
"""
import bisect
import csv
import re
import struct
from collections import defaultdict, deque
from pathlib import Path

CLASSES = ("closed", "globals", "vtable", "os")
DATA_LO, DATA_HI = 0xB07000, 0xBC0000
BENIGN_K32 = re.compile(
    r"^(Heap|Local|Global|Virtual|Interlocked|InitializeCriticalSection|EnterCriticalSection|"
    r"LeaveCriticalSection|DeleteCriticalSection|TlsGetValue|TlsSetValue|TlsAlloc|TlsFree|"
    r"GetLastError|SetLastError|GetCurrentThreadId|GetCurrentProcess|GetCurrentThread|"
    r"MultiByteToWideChar|WideCharToMultiByte|lstrlen|lstrcpy|lstrcmp|GetTickCount|"
    r"QueryPerformance|Sleep$|GetProcAddress|GetModuleHandle|RaiseException|RtlUnwind|IsBad|"
    r"GetStringType|LCMapString|CompareString|GetCPInfo|GetACP|GetOEMCP|FreeLibrary|LoadLibrary)")
WR = re.compile(r"^(MOV|MOVZX|ADD|SUB|INC|DEC|OR|AND|XOR|FSTP|FST|FISTP|FIST|MOVQ|MOVD|"
                r"MOVSS|MOVAPS|MOVUPS|SHL|SHR|SAR|NEG|NOT|XCHG|SETNZ|SETZ|CMPXCHG|STOSD|LOCK)\b")
ABS = re.compile(r"0x([0-9a-f]{6,8})\]", re.I)


def effect(name):
    dll, fn = name.split("!", 1)
    dll = dll.lower()
    if dll == "kernel32.dll" and BENIGN_K32.match(fn):
        return None
    return dll + ("-file" if dll == "kernel32.dll" else "")


def read_iat(exe):
    """VA -> dll!function from a PE32 import table; no third-party PE dependency."""
    data = Path(exe).read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    opt = pe + 24
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0" or struct.unpack_from("<H", data, opt)[0] != 0x10B:
        raise ValueError("oracle classification requires a PE32 image")
    base = struct.unpack_from("<I", data, opt + 28)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    sec = opt + struct.unpack_from("<H", data, pe + 20)[0]
    sections = [struct.unpack_from("<IIII", data, sec + 40 * i + 8) for i in range(nsec)]

    def offset(rva):
        for vsize, va, rawsize, raw in sections:
            if va <= rva < va + max(vsize, rawsize):
                return raw + rva - va
        if rva < struct.unpack_from("<I", data, opt + 60)[0]:
            return rva
        raise ValueError(f"unmapped PE RVA {rva:x}")

    def string(rva):
        start = offset(rva)
        return data[start:data.index(b"\0", start)].decode("ascii")

    imports = {}
    directory = struct.unpack_from("<I", data, opt + 104)[0]
    if not directory:
        return imports
    cursor = offset(directory)
    while True:
        original, stamp, chain, name, first = struct.unpack_from("<IIIII", data, cursor)
        if not any((original, stamp, chain, name, first)):
            break
        dll = string(name).lower()
        lookup = offset(original or first)
        index = 0
        while True:
            target = struct.unpack_from("<I", data, lookup + index * 4)[0]
            if not target:
                break
            fn = f"ordinal-{target & 0xffff}" if target & 0x80000000 else string(target + 2)
            imports[base + first + index * 4] = dll + "!" + fn
            index += 1
        cursor += 20
    return imports


def instruction_flags(ins, iat):
    flags = set()
    if ins.startswith("CALL"):
        match = re.fullmatch(r"CALL dword ptr \[0x([0-9a-f]+)\]", ins, re.I)
        if match:
            target = int(match[1], 16)
            if target in iat:
                boundary = effect(iat[target])
                if boundary:
                    flags.add("os:" + boundary)
            else:
                flags.add("indirect")
        elif re.match(r"CALL (dword ptr \[|E[A-Z]{2}$)", ins):
            flags.add("indirect")
        return flags
    parts = ins.split(" ", 1)
    if len(parts) == 2 and WR.match(parts[0]):
        match = ABS.search(parts[1].split(",")[0])
        if match and DATA_LO <= int(match[1], 16) < DATA_HI:
            flags.add("gwrite")
    if any(DATA_LO <= int(match[1], 16) < DATA_HI for match in ABS.finditer(ins)):
        flags.add("gread")
    return flags


def closure(flags, callees):
    """Monotone worklist handles recursion and library-to-library call chains."""
    result = {va: set(values) for va, values in flags.items()}
    callers = defaultdict(set)
    for caller, targets in callees.items():
        result.setdefault(caller, set())
        for target in targets:
            callers[target].add(caller)
    pending = deque(result)
    queued = set(result)
    while pending:
        target = pending.popleft()
        queued.discard(target)
        for caller in callers[target]:
            extra = result[target] - result[caller]
            if extra:
                result[caller].update(extra)
                if caller not in queued:
                    pending.append(caller)
                    queued.add(caller)
    return result


def classify(flags):
    if any(f.startswith("os:") for f in flags):
        return "os"
    if "indirect" in flags:
        return "vtable"
    if flags & {"gread", "gwrite"}:
        return "globals"
    return "closed"


def scan(export, iat):
    export = Path(export)
    ranges = []
    with (export / "function-details.tsv").open(encoding="utf-8", newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            if not re.fullmatch(r"[0-9a-fA-F]{8}", row["address"]):
                continue
            for part in row["ranges"].split(";"):
                if "-" in part:
                    lo, hi = part.split("-")
                    ranges.append((int(lo, 16), int(hi, 16), int(row["address"], 16)))
    ranges.sort()
    starts = [r[0] for r in ranges]
    flags = defaultdict(set)
    with (export / "instructions.asm").open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            if line.startswith(";"):
                continue
            parts = line.rstrip("\r\n").split("\t")
            if len(parts) < 3 or not re.fullmatch(r"[0-9a-fA-F]{8}", parts[0]):
                continue
            address = int(parts[0], 16)
            index = bisect.bisect_right(starts, address) - 1
            if index >= 0 and address <= ranges[index][1]:
                flags[ranges[index][2]].update(instruction_flags(parts[2], iat))
    callees = defaultdict(set)
    with (export / "calls.tsv").open(encoding="utf-8", newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            if all(re.fullmatch(r"[0-9a-fA-F]{8}", row[k]) for k in ("caller", "target")):
                callees[int(row["caller"], 16)].add(int(row["target"], 16))
    return closure(flags, callees)
