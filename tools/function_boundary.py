"""Direct answering requirements from each Ghidra function's own body only.

Direct callee counts are distinct exported entries, partitioned by coverage's
game-function set (the remaining exported functions are library/compiler code).
Import/indirect counts are call *sites*. No callee effects are propagated here.
Slot displacements are candidates: a single non-stack register plus a nonnegative
dword-aligned offset, including zero. They do not prove a vtable's identity.
Globals are actual memory reads/writes in PE .data/.rdata, not LEA/immediates.
"""
import bisect
import csv
from dataclasses import dataclass, field
import json
from pathlib import Path
import re
import struct

CLASSES = ("leaf", "globals", "direct", "indirect", "import")
ADDRESS = re.compile(r"[0-9a-fA-F]{8}")
REGISTER = r"E(?:AX|BX|CX|DX|SI|DI|BP|SP)"
SLOT_REGISTER = r"E(?:AX|BX|CX|DX|SI|DI)"
MEMORY = re.compile(r"\[([^]]+)\]")
WRITE = re.compile(r"^(?:MOV\w*|SET\w*|F(?:I?ST\w*|NST\w*|SAVE|XSAVE)|"
                   r"XSAVE\w*|STMXCSR|SGDT|SIDT|POP)$")
READ_WRITE = {"ADD", "ADC", "SUB", "SBB", "INC", "DEC", "OR", "AND", "XOR",
              "NEG", "NOT", "XCHG", "XADD", "CMPXCHG", "CMPXCHG8B", "SHL",
              "SHR", "SAL", "SAR", "ROL", "ROR", "RCL", "RCR", "SHLD", "SHRD",
              "BTS", "BTR", "BTC"}


def data_sections(exe):
    """Half-open VA ranges from the PE32 .data/.rdata virtual section sizes."""
    data = Path(exe).read_bytes()
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    opt = pe + 24
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0" or struct.unpack_from("<H", data, opt)[0] != 0x10b:
        raise ValueError("boundary scan requires a PE32 image")
    base = struct.unpack_from("<I", data, opt + 28)[0]
    sec = opt + struct.unpack_from("<H", data, pe + 20)[0]
    result = []
    for i in range(struct.unpack_from("<H", data, pe + 6)[0]):
        pos = sec + i * 40
        if data[pos:pos + 8].rstrip(b"\0") in (b".data", b".rdata"):
            size, rva = struct.unpack_from("<II", data, pos + 8)
            result.append((base + rva, base + rva + size))
    return result


@dataclass
class Boundary:
    direct_game: set = field(default_factory=set)
    direct_library: set = field(default_factory=set)
    direct_other: set = field(default_factory=set)
    import_calls: int = 0
    import_names: set = field(default_factory=set)
    indirect_calls: int = 0
    indirect_slots: set = field(default_factory=set)
    global_reads: bool = False
    global_writes: bool = False

    def columns(self):
        result = {}
        for kind in ("game", "library", "other"):
            targets = getattr(self, "direct_" + kind)
            result["direct_" + kind + "_callees"] = len(targets)
            result["direct_" + kind + "_targets"] = json.dumps([f"{va:08x}" for va in sorted(targets)])
        result.update(import_calls=self.import_calls, import_names=json.dumps(sorted(self.import_names)),
                      indirect_calls=self.indirect_calls, indirect_slots=json.dumps(sorted(self.indirect_slots)),
                      global_reads=int(self.global_reads), global_writes=int(self.global_writes))
        result["boundary_class"] = ("import" if self.import_calls else "indirect" if self.indirect_calls else
                                    "direct" if self.direct_game or self.direct_library or self.direct_other else
                                    "globals" if self.global_reads or self.global_writes else "leaf")
        return result

    def instruction(self, ins, game, functions, iat, sections):
        ins = ins.strip().upper()
        parts = ins.split(None, 1)
        while parts and parts[0] in {"LOCK", "REP", "REPE", "REPNE", "REPZ", "REPNZ"}:
            parts = parts[1].split(None, 1) if len(parts) > 1 else []
        if len(parts) < 2:
            return
        mnemonic, operands = parts
        if mnemonic == "CALL":
            absolute = re.fullmatch(r"(?:DWORD PTR )?\[0X([0-9A-F]+)\]", operands)
            direct = re.fullmatch(r"0X([0-9A-F]+)", operands)
            if absolute and int(absolute[1], 16) in iat:
                self.import_calls += 1
                self.import_names.add(iat[int(absolute[1], 16)])
            elif direct:
                target = int(direct[1], 16)
                if target in iat:
                    self.import_calls += 1
                    self.import_names.add(iat[target])
                else:
                    targets = self.direct_game if target in game else (
                        self.direct_library if target in functions else self.direct_other)
                    targets.add(target)
            elif "[" in operands or re.fullmatch(REGISTER, operands):
                self.indirect_calls += 1
                slot = re.fullmatch(r"(?:DWORD PTR )?\[(" + SLOT_REGISTER +
                                    r")(?:\s*\+\s*(0X[0-9A-F]+|[0-9]+))?\]", operands)
                if slot:
                    displacement = int(slot[2], 16 if slot[2] and slot[2].startswith("0X") else 10) if slot[2] else 0
                    if displacement % 4 == 0:
                        self.indirect_slots.add(displacement)
        if mnemonic in {"LEA", "NOP"}:
            return
        for index, operand in enumerate(operands.split(",")):
            if re.search(r"\b(?:FS|GS):", operand):
                continue  # segment-relative offsets are not absolute image VAs
            addresses = [int(m, 16) for mem in MEMORY.findall(operand)
                         for m in re.findall(r"\b0X([0-9A-F]+)\b", mem)]
            if not any(lo <= va < hi for va in addresses for lo, hi in sections):
                continue
            writes = (index == 0 and (WRITE.fullmatch(mnemonic) or mnemonic in READ_WRITE)) or mnemonic == "XCHG"
            reads = not (index == 0 and WRITE.fullmatch(mnemonic))
            self.global_reads |= bool(reads)
            self.global_writes |= bool(writes)


def scan(export, game, iat, sections):
    """Attribute decoded instructions by inclusive, possibly discontiguous bodies.

    Deliberately do not consume calls.tsv: its resolved computed-call targets
    cannot turn a register/slot call into a direct call in the instruction body.
    """
    export = Path(export)
    ranges, functions = [], set()
    with (export / "function-details.tsv").open(encoding="utf-8", newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            if not ADDRESS.fullmatch(row["address"]):
                continue
            va = int(row["address"], 16)
            functions.add(va)
            for part in row["ranges"].split(";"):
                if "-" in part:
                    lo, hi = part.split("-")
                    ranges.append((int(lo, 16), int(hi, 16), va))
    result = {va: Boundary() for va in functions}
    ranges.sort()
    starts = [r[0] for r in ranges]
    # Track all overlapping bodies, if an export contains shared instruction ranges.
    prefix_end = []
    for lo, hi, va in ranges:
        prefix_end.append(max(hi, prefix_end[-1] if prefix_end else hi))
    with (export / "instructions.asm").open(encoding="utf-8", errors="replace") as stream:
        for line in stream:
            if line.startswith(";"):
                continue
            parts = line.rstrip("\r\n").split("\t")
            if len(parts) < 3 or not ADDRESS.fullmatch(parts[0]):
                continue
            address = int(parts[0], 16)
            index = bisect.bisect_right(starts, address) - 1
            owners = set()
            while index >= 0 and prefix_end[index] >= address:
                if address <= ranges[index][1]:
                    owners.add(ranges[index][2])
                index -= 1
            for va in owners:
                result[va].instruction(parts[2], game, functions, iat, sections)
    return result
