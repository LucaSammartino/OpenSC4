#!/usr/bin/env python3
"""Extract value-only x86 entry contracts for every exported Windows function.

Uses Ghidra's decoded instructions, not its guessed prototypes. No third-party
packages. Raw analysis belongs in --evidence, never in the engine-readable TSV.
See docs/abi/README.md for uncertainty, regeneration and validation commands.
"""
import argparse
import bisect
import csv
import json
import re
import struct
import time
from collections import Counter, defaultdict, deque
from dataclasses import dataclass, field
from pathlib import Path

from abi_returns import X87_PUSH, X87_POP1, X87_POP2, ret_label

ROOT = Path(__file__).resolve().parents[1]
COLUMNS = ('address size name convention this callee_pop stack_arg_dwords reg_args '
           'ret ret_confidence confidence callers notes').split()
REGS = ('eax', 'ecx', 'edx', 'ebx', 'esp', 'ebp', 'esi', 'edi')
RI = {r: i for i, r in enumerate(REGS)}
ALIASES = {r: r for r in REGS}
for full, small in zip(REGS, ('ax al ah', 'cx cl ch', 'dx dl dh', 'bx bl bh',
                              'sp', 'bp', 'si', 'di')):
    ALIASES.update((s, full) for s in small.split())
REG_RE = re.compile(r'\b(?:' + '|'.join(ALIASES) + r')\b')
MEM_RE = re.compile(r'\[([^]]+)\]')
PRESERVED = {'ebx', 'ebp', 'esi', 'edi'}
VOLATILE = {'eax', 'ecx', 'edx'}
ARG_REGS = {'eax', 'ecx', 'edx', 'ebx', 'esi', 'edi'}
X87_ZERO = {'fldz', 'fld1', 'fldpi', 'fldl2e', 'fldl2t', 'fldlg2', 'fldln2'}
X87_PAIR = {'fcompp', 'fucompp', 'fcomp', 'fcom', 'fucom', 'fucomp', 'faddp',
            'fsubp', 'fsubrp', 'fmulp', 'fdivp', 'fdivrp', 'fxch', 'fscale',
            'fprem', 'fprem1', 'fpatan', 'fyl2x', 'fyl2xp1'}
READ_ONLY = {'cmp', 'test', 'bt', 'push', 'call', 'jmp', 'prefetchnta', 'prefetcht0',
             'prefetcht1', 'prefetcht2', 'fld', 'fild', 'fbld', 'fldcw', 'ldmxcsr'}


def number(s):
    try:
        return int(s.strip(), 0)
    except ValueError:
        return None


def registers(op):
    return {ALIASES[m] for m in REG_RE.findall(op)}


def full_register(op):
    return op in REGS


def register_mask(op, reg):
    mask = 0
    for token in REG_RE.findall(op):
        if ALIASES[token] == reg:
            mask |= 15 if token in REGS else 2 if token.endswith('h') else 1 if token.endswith('l') else 3
    return mask


@dataclass(slots=True)
class Insn:
    address: int
    raw: bytes
    mnemonic: str
    operands: tuple

    @classmethod
    def parse(cls, line):
        addr, raw, asm = line.rstrip().split('\t', 2)
        parts = asm.lower().split(None, 1)
        # Ghidra spells string prefixes as part of the mnemonic field.
        if parts[0] in ('rep', 'repe', 'repne', 'lock') and len(parts) > 1:
            parts = parts[1].split(None, 1)
        return cls(int(addr, 16), bytes.fromhex(raw), parts[0],
                   tuple(p.strip() for p in parts[1].split(',')) if len(parts) > 1 else ())

    @property
    def end(self):
        return self.address + len(self.raw)

    @property
    def target(self):
        # Decode relative transfers from bytes; textual names/format are irrelevant.
        b = self.raw
        if len(b) == 5 and b[0] in (0xe8, 0xe9):
            return (self.end + struct.unpack_from('<i', b, 1)[0]) & 0xffffffff
        if len(b) == 2 and (b[0] == 0xeb or 0x70 <= b[0] <= 0x7f or 0xe0 <= b[0] <= 0xe3):
            return (self.end + struct.unpack_from('<b', b, 1)[0]) & 0xffffffff
        if len(b) == 6 and b[0] == 0x0f and 0x80 <= b[1] <= 0x8f:
            return (self.end + struct.unpack_from('<i', b, 2)[0]) & 0xffffffff
        return None

    @property
    def pop(self):
        b = self.raw.lstrip(b'\x66')
        if b[:1] in (b'\xc2', b'\xca') and len(b) >= 3:
            return struct.unpack_from('<H', b, 1)[0]
        if b[:1] in (b'\xc3', b'\xcb'):
            return 0
        return None


def access(ins):
    """GPR read/write sets. Partial writes do not kill the rest of an input GPR."""
    m, ops = ins.mnemonic, ins.operands
    reads, writes = set(), set()
    if not ops:
        if m in ('cdq', 'cwd'):
            return {'eax'}, {'edx'}
        if m.startswith(('stos', 'scas')):
            return {'eax', 'edi', 'ecx'}, {'edi', 'ecx'}
        if m.startswith(('movs', 'cmps')):
            return {'esi', 'edi', 'ecx'}, {'esi', 'edi', 'ecx'}
        if m.startswith('lods'):
            return {'esi', 'ecx'}, {'eax', 'esi', 'ecx'}
        return reads, writes
    for op in ops:
        reads |= registers(op)
    dst = ops[0]
    if m in ('mov', 'movzx', 'movsx', 'lea', 'pop') or m.startswith('set'):
        if '[' not in dst:
            reads -= registers(dst)
    if m not in READ_ONLY and not m.startswith(('j', 'f')) and '[' not in dst:
        if dst in ALIASES:
            writes.add(ALIASES[dst])
    if m in ('xor', 'sub', 'pxor') and len(ops) == 2 and ops[0] == ops[1]:
        reads -= registers(dst)
    if m in ('mul', 'div', 'idiv') or m == 'imul' and len(ops) == 1:
        reads.add('eax')
        if m in ('div', 'idiv'):
            reads.add('edx')
        writes |= {'eax', 'edx'}
    if m == 'xchg' and len(ops) == 2:
        writes |= {op for op in ops if full_register(op)}
    if m in ('shld', 'shrd'):
        writes |= {dst} if full_register(dst) else set()
    if m in ('fnstsw', 'fstsw') and dst == 'ax':
        writes.add('eax')
    if m == 'cmpxchg':
        reads.add('eax')
        writes.add('eax')
    if m in ('loop', 'loope', 'loopne', 'jecxz'):
        reads.add('ecx')
    # Stack saves / allocation are not evidence of semantic register arguments.
    if m == 'push' and dst in REGS:
        reads.discard(dst)
    return reads, writes


def memory_offset(op, aliases):
    """An address relative to entry ESP, or None for dynamic/unrelated addresses."""
    match = MEM_RE.search(op)
    if not match:
        return None
    text = match[1].replace(' ', '').replace('-', '+-')
    value, bases = 0, 0
    for term in text.split('+'):
        if not term:
            continue
        if term in RI:
            v = aliases[RI[term]]
            if v is None:
                return None
            value += v
            bases += 1
        else:
            v = number(term)
            if v is None:
                return None
            value += v
    return value if bases == 1 else None


def memory_width(op):
    for name, width in (('xmmword', 16), ('oword', 16), ('tbyte', 10), ('qword', 8),
                        ('dword', 4), ('word', 2), ('byte', 1)):
        if op.startswith(name):
            return width
    return 4


class Image:
    """The few PE facts needed to read jump-table words (no executable decoding)."""
    def __init__(self, path):
        self.data = path.read_bytes()
        pe = struct.unpack_from('<I', self.data, 0x3c)[0]
        if self.data[pe:pe + 4] != b'PE\0\0':
            raise ValueError('not a PE image')
        count, opt = struct.unpack_from('<H', self.data, pe + 6)[0], pe + 24
        self.base = struct.unpack_from('<I', self.data, opt + 28)[0]
        pos = opt + struct.unpack_from('<H', self.data, pe + 20)[0]
        self.sections = []
        for i in range(count):
            vs, va, size, raw = struct.unpack_from('<IIII', self.data, pos + 40 * i + 8)
            self.sections.append((self.base + va, size, raw))

    def word(self, va):
        for base, size, raw in self.sections:
            if base <= va and va + 4 <= base + size:
                return struct.unpack_from('<I', self.data, raw + va - base)[0]
        return None


@dataclass
class Function:
    address: int
    name: str
    ranges: list
    ghidra: str = 'unknown'
    insns: dict = field(default_factory=dict)
    edges: dict = field(default_factory=dict)
    tails: set = field(default_factory=set)
    pops: set = field(default_factory=set)
    notes: set = field(default_factory=set)
    live: set = field(default_factory=set)
    body_args: int = 0
    kinds: set = field(default_factory=set)

    @property
    def size(self):
        return sum(end - start + 1 for start, end in self.ranges)


def build_graph(f, instructions, starts, image=None):
    for start, end in f.ranges:
        va = start
        while va <= end:
            ins = instructions.get(va)
            if ins is None:
                f.notes.add('incomplete-decode')
                break
            f.insns[va] = ins
            va = ins.end
    for va, ins in f.insns.items():
        m, target = ins.mnemonic, ins.target
        edges = []
        if ins.pop is not None:
            f.pops.add(ins.pop)
        elif m in ('int3', 'ud2', 'hlt'):
            pass
        elif m == 'jmp' or m.startswith('j') or m.startswith('loop'):
            if target in f.insns:
                edges.append(target)
            elif target in starts:
                f.tails.add(target)
            elif target is not None:
                f.notes.add('unresolved-transfer')
            elif m == 'jmp':
                # Ghidra body ranges include switch destinations; only follow validated table entries.
                op = ins.operands[0] if ins.operands else ''
                table = re.search(r'0x([0-9a-f]+)', op)
                if image and table and '*0x4' in op.replace(' ', ''):
                    base = int(table[1], 16)
                    for index in range(4096):
                        dst = image.word(base + 4 * index)
                        if dst not in f.insns:
                            break
                        edges.append(dst)
                if not edges:
                    f.notes.add('indirect-transfer')
            if m != 'jmp' and ins.end in f.insns:
                edges.append(ins.end)
        elif ins.end in f.insns:
            edges.append(ins.end)
        else:
            f.notes.add('incomplete-flow')
        f.edges[va] = tuple(dict.fromkeys(edges))


def propagate_pops(functions):
    # Monotone union; handles chains and cycles without a recursion depth guess.
    reverse = defaultdict(set)
    for f in functions.values():
        for dst in f.tails:
            reverse[dst].add(f.address)
    work = deque(functions)
    queued = set(work)
    while work:
        addr = work.popleft()
        queued.discard(addr)
        for parent in reverse[addr]:
            f = functions[parent]
            old = len(f.pops)
            f.pops |= functions[addr].pops
            if len(f.pops) != old and parent not in queued:
                work.append(parent)
                queued.add(parent)


def stack_and_registers(f, functions):
    """Forward abstract interpretation: stack aliases, must-defined registers, may-live inputs.

    Merge by intersection / equal stack offsets, so loops cannot fabricate ever
    larger argument offsets. Unknown stack writes stay unknown, never zero.
    """
    initial = tuple(0 if r == 'esp' else None for r in REGS)
    states = {f.address: (0, initial)}
    work = deque([f.address])
    live, body = set(), 0
    updates = 0
    while work:
        va = work.popleft()
        ins = f.insns.get(va)
        if ins is None:
            continue
        defined, aliases = states[va]
        a = list(aliases)
        reads, writes = access(ins)
        m, ops = ins.mnemonic, ins.operands
        if m == 'call' or m == 'jmp' and ins.target in functions:
            callee = functions.get(ins.target)
            if callee:
                reads |= callee.live
        for r in reads & ARG_REGS:
            mask = 0
            for op in ops:
                mask |= register_mask(op, r)
            if m in ('call', 'mul', 'div', 'idiv', 'cdq', 'cwd') or not mask:
                mask = 15
            if defined >> (4 * RI[r]) & mask != mask:
                live.add(r)
        for index, op in enumerate(ops):
            if '[' not in op or m == 'lea':
                continue
            if index == 0 and (m.startswith(('mov', 'set', 'fst', 'fist', 'fnst')) or m == 'pop'):
                continue
            offset = memory_offset(op, aliases)
            if offset is not None and offset >= 4:
                body = max(body, (offset + memory_width(op) - 4 + 3) // 4)
            elif offset is None and registers(op) & {'esp', 'ebp'}:
                f.notes.add('dynamic-stack-read')
        old = dict(zip(REGS, aliases))
        for r in writes:
            a[RI[r]] = None
        if m == 'mov' and len(ops) == 2 and full_register(ops[0]):
            a[RI[ops[0]]] = old.get(ops[1])
        elif m == 'lea' and len(ops) == 2 and full_register(ops[0]):
            a[RI[ops[0]]] = memory_offset(ops[1], aliases)
        elif m in ('add', 'sub') and len(ops) == 2 and full_register(ops[0]):
            v = number(ops[1])
            base = old[ops[0]]
            a[RI[ops[0]]] = base + (v if m == 'add' else -v) if base is not None and v is not None else None
        if m == 'push':
            a[RI['esp']] = old['esp'] - 4 if old['esp'] is not None else None
        elif m == 'pop':
            a[RI['esp']] = old['esp'] + 4 if old['esp'] is not None else None
        elif m == 'leave':
            a[RI['esp']] = old['ebp'] + 4 if old['ebp'] is not None else None
            a[RI['ebp']] = None
        elif m == 'call':
            callee = functions.get(ins.target)
            pop = next(iter(callee.pops)) if callee and len(callee.pops) == 1 else None
            a[RI['esp']] = old['esp'] + pop if old['esp'] is not None and pop is not None else None
            if pop is None:
                f.notes.add('unknown-call-pop')
            writes |= VOLATILE
            for r in VOLATILE:
                a[RI[r]] = None
        write_bits = 0
        for r in writes:
            mask = register_mask(ops[0], r) if ops and ops[0] in ALIASES else 15
            if m == 'call' or not mask:
                mask = 15
            write_bits |= mask << (4 * RI[r])
        new_defined = defined | write_bits
        state = new_defined, tuple(a)
        for dst in f.edges.get(va, ()):
            before = states.get(dst)
            if before is None:
                merged = state
            else:
                merged = (before[0] & state[0], tuple(x if x == y else None
                          for x, y in zip(before[1], state[1])))
            if before != merged:
                states[dst] = merged
                work.append(dst)
                updates += 1
        if updates > max(1000, len(f.insns) * 40):
            f.notes.add('analysis-limit')
            break
    f.body_args = body
    f.live |= live


def caller_cleanup(call, instructions, max_insns=24):
    """First unambiguous straight-line cleanup; stop at another call/argument setup.

    Restoring nonvolatile registers is never argument cleanup. Multiple volatile
    pops and LEA ESP are equivalent to ADD ESP. A batch cleanup is retained as
    an upper bound and disagreement with body reads remains visible.
    """
    va, total = call.end, 0
    evidence = []
    for _ in range(max_insns):
        ins = instructions.get(va)
        if ins is None:
            break
        m, ops = ins.mnemonic, ins.operands
        amount = None
        if m == 'add' and len(ops) == 2 and ops[0] == 'esp':
            amount = number(ops[1])
        elif m == 'lea' and len(ops) == 2 and ops[0] == 'esp':
            amount = memory_offset(ops[1], tuple(0 if r == 'esp' else None for r in REGS))
        elif m == 'pop' and ops and ops[0] in VOLATILE:
            amount = 4
        elif m in ('push', 'call', 'ret', 'retn', 'leave', 'pop', 'enter') or m.startswith(('j', 'loop')):
            break
        elif 'esp' in access(ins)[1]:
            break
        if amount is not None:
            if amount <= 0 or amount % 4:
                break
            total += amount
            evidence.append(f'{ins.address:08x}')
            if m != 'pop':
                break
        va = ins.end
    return total // 4, evidence


def required_x87(ins):
    m = ins.mnemonic
    if m in X87_ZERO or m in ('fild', 'fbld'):
        return 0
    if m == 'fld':
        return 0 if ins.operands and '[' in ins.operands[0] else 1
    if m in ('fnstsw', 'fstsw', 'fnstcw', 'fstcw', 'fldcw', 'fnclex', 'fclex',
             'fninit', 'finit', 'fnop', 'fwait', 'wait'):
        return 0
    if m in X87_PAIR:
        return 2
    return 1 if m.startswith('f') else 0


def call_x87_used(call, f):
    work, seen = [(dst, 0) for dst in f.edges.get(call.address, ())], set()
    while work and len(seen) < 160:
        va, height = work.pop()
        if (va, height) in seen:
            continue
        seen.add((va, height))
        ins = f.insns[va]
        if ins.mnemonic == 'call' or ins.pop is not None:
            continue
        if required_x87(ins) > height:
            return True
        height += (1 if ins.mnemonic in X87_PUSH else -2 if ins.mnemonic in X87_POP2
                   else -1 if ins.mnemonic in X87_POP1 else 0)
        if not 0 <= height <= 8:
            continue
        work.extend((dst, height) for dst in f.edges.get(va, ()))
    return False


def returns(f, functions, vtable):
    """Forward x87 depth and backwards EAX/EDX provenance at every return."""
    depths, seen, work = defaultdict(set), set(), [(f.address, 0)]
    predecessors = defaultdict(list)
    for va, edges in f.edges.items():
        for dst in edges:
            predecessors[dst].append(va)
    while work and len(seen) < max(1000, len(f.insns) * 10):
        va, height = work.pop()
        if (va, height) in seen or va not in f.insns:
            continue
        seen.add((va, height))
        ins, m = f.insns[va], f.insns[va].mnemonic
        if ins.pop is not None:
            depths[va].add(height)
        elif m == 'call':
            vr = vtable.get(ins.target, [])
            height = int(any(r['ret'] == 'x87' for r in vr) or call_x87_used(ins, f))
        elif m in X87_PUSH:
            height += 1
        elif m in X87_POP1:
            height -= 1
        elif m in X87_POP2:
            height -= 2
        elif m == 'emms':
            height = 0
        if not 0 <= height <= 8:
            f.notes.add('x87-depth-uncertain')
            continue
        work.extend((dst, height) for dst in f.edges.get(va, ()))
    kinds = set()
    for va, heights in depths.items():
        if 1 in heights:
            kinds.add('x87')
        if heights == {1}:
            continue
        # State: location, whether EDX was assigned (or consumed), walk length.
        pending = [(p, None, 0) for p in predecessors[va]]
        visited = set()
        while pending:
            prev, edx, n = pending.pop()
            if (prev, edx) in visited:
                continue
            visited.add((prev, edx))
            ins = f.insns[prev]
            if n > 80:
                kinds.add('unknown')
                continue
            if ins.mnemonic == 'call':
                vr = vtable.get(ins.target, [])
                kinds |= {r['ret'] for r in vr if r['ret'] != 'x87'} or {'via-call'}
                continue
            reads, writes = access(ins)
            if ins.mnemonic not in ('pop', 'push', 'leave', 'nop') and not ins.mnemonic.startswith('f'):
                if edx is None and 'edx' in reads and 'edx' not in writes:
                    edx = 'consumed'
                elif edx is None and 'edx' in writes:
                    edx = 'written'
                # Partial EAX writes still supply a return channel, unlike a full live-in kill.
                eax_write = 'eax' in writes or (ins.operands and ins.operands[0] in ('al', 'ah', 'ax')
                                                and ins.mnemonic not in READ_ONLY)
                if eax_write:
                    kinds.add('edx:eax' if edx == 'written' else 'eax')
                    continue
            if not predecessors[prev]:
                kinds.add('unknown')
            pending.extend((p, edx, n + 1) for p in predecessors[prev])
    f.kinds = kinds


def convention(live, pops, args):
    if {'ecx', 'edx'} <= live:
        return 'fastcall'
    if 'ecx' in live and 'edx' not in live:
        return 'thiscall'
    if live:
        return 'unknown'
    if len(pops) == 1 and next(iter(pops)) > 0:
        return 'stdcall'
    if pops == {0} and args:
        return 'cdecl'
    return 'unknown'


def read_tsv(path):
    with path.open(encoding='utf-8-sig', newline='') as fh:
        return list(csv.DictReader(fh, delimiter='\t'))


def load_functions(export, names):
    functions = {}
    for row in read_tsv(export / 'function-details.tsv'):
        addr = int(row['address'], 16)
        ranges = [tuple(int(a, 16) for a in part.split('-'))
                  for part in row['ranges'].split(';') if part]
        functions[addr] = Function(addr, names.get(addr, row['name']), ranges, row['calling_convention'])
    return functions


def extract(export, exe, vtables, output, evidence, direct_contracts=()):
    started = time.perf_counter()
    names_path = ROOT / 're/names_windows.tsv'
    names = {int(r['va'], 16): r['name'] for r in read_tsv(names_path)} if names_path.exists() else {}
    functions = load_functions(export, names)
    exported_count = len(functions)
    vtable = defaultdict(list)
    for row in read_tsv(vtables):
        addr = int(row['target'], 16)
        vtable[addr].append(row)
        if addr not in functions:
            functions[addr] = Function(addr, names.get(addr, row['method']), [])
            functions[addr].notes.add('vtable-only;no-export-ranges')
    instructions = {}
    with (export / 'instructions.asm').open(encoding='utf-8') as fh:
        for line in fh:
            if line.startswith(';') or not line.strip():
                continue
            ins = Insn.parse(line)
            instructions[ins.address] = ins
    print(f'loaded functions={exported_count} vtable_only={len(functions)-exported_count} '
          f'instructions={len(instructions)}', flush=True)
    image = Image(exe) if exe else None
    for f in functions.values():
        build_graph(f, instructions, functions, image)
    propagate_pops(functions)
    # Register forwarding reaches a fixed point across both calls and tails.
    reverse = defaultdict(set)
    for f in functions.values():
        for ins in f.insns.values():
            if ins.mnemonic in ('call', 'jmp') and ins.target in functions:
                reverse[ins.target].add(f.address)
    work, queued = deque(functions), set(functions)
    while work:
        addr = work.popleft()
        queued.discard(addr)
        f = functions[addr]
        old = set(f.live)
        stack_and_registers(f, functions)
        if f.live != old:
            for parent in reverse[addr] - queued:
                work.append(parent)
                queued.add(parent)
    print('stack/register analysis complete', flush=True)
    calls, cleanups = defaultdict(list), defaultdict(list)
    for row in read_tsv(export / 'calls.tsv'):
        try:
            addr, site = int(row['target'], 16), int(row['callsite'], 16)
        except ValueError:
            continue
        ins = instructions.get(site)
        if ins is None or ins.mnemonic != 'call' or ins.target != addr:
            continue
        calls[addr].append(site)
        count, sites = caller_cleanup(ins, instructions)
        if count:
            cleanups[addr].append({'callsite': f'{site:08x}', 'dwords': count, 'cleanup_sites': sites})
    for f in functions.values():
        returns(f, functions, vtable)
    # Return forwarding also reaches a fixed point, bounded by the finite label vocabulary.
    for _ in range(20):
        changed = False
        for f in functions.values():
            before = len(f.kinds)
            for dst in f.tails:
                f.kinds |= functions[dst].kinds
            changed |= len(f.kinds) != before
        if not changed:
            break
    direct = {int(r['address'], 16): r for r in direct_contracts}
    rows, raw_evidence = [], {}
    for addr, f in sorted(functions.items()):
        body = f.body_args
        cleanup = max((r['dwords'] for r in cleanups[addr]), default=0)
        pop = next(iter(f.pops)) if len(f.pops) == 1 else None
        count = pop // 4 if pop else max(body, cleanup)
        conv = convention(f.live, f.pops, count)
        ret, ret_conf = ret_label(f.kinds, f.pops, ['indirect-jump'] if 'indirect-transfer' in f.notes else [])
        notes = set(f.notes)
        notes.add(f'body_stack_dwords={body};caller_cleanup_dwords={cleanup}')
        if len(f.pops) > 1:
            notes.add('inconsistent-pops=' + '/'.join(map(str, sorted(f.pops))))
        if pop is not None and pop % 4:
            notes.add('unaligned-pop')
        if not f.pops:
            notes.add('pop-unresolved')
        if body != (pop // 4 if pop else cleanup):
            notes.add('argument-sources-disagree')
        raw_ret, raw_pop, raw_conv = ret, pop, conv
        if addr in vtable:
            # Existing facts include reviewed return types unavailable from byte writes alone.
            vr = sorted(vtable[addr], key=lambda r: (r['ret_confidence'] != 'high', r['ret']))[0]
            values = {(r['callee_pop'], r['ret']) for r in vtable[addr]}
            if len(values) > 1:
                notes.add('vtable-conflict=' + '/'.join(sorted(f'{p or "?"}:{r}' for p, r in values)))
            ret, ret_conf = vr['ret'], vr['ret_confidence'] or 'low'
            vp = int(vr['callee_pop']) if vr['callee_pop'] else None
            if (raw_pop, raw_ret) != (vp, ret):
                notes.add(f'vtable-reviewed;inferred_pop={raw_pop if raw_pop is not None else "?"};inferred_ret={raw_ret}')
            if vp is not None:
                pop = vp
                count = pop // 4 if pop else max(body, cleanup)
            notes.add('source=vtables.tsv')
        if addr in direct:
            d = direct[addr]
            # Reviewed semantic types / unused parameters win; retain all contradictory observations.
            expected = (d['convention'], d['pop'], d['args'], d['ret'])
            observed = (conv, pop, count, ret)
            if observed != expected:
                notes.add('direct-reviewed;inferred=' + ':'.join(map(str, observed)))
                notes.add('direct-contract=' + ':'.join(map(str, expected)))
            conv, pop, count = d['convention'], d['pop'], d['args']
            if addr not in vtable and d['ret'] != 'discarded':
                ret, ret_conf = d['ret'], d.get('ret_confidence', 'high')
            notes.add('source=direct-calls.md')
            if d.get('note'):
                notes.add(d['note'])
        if f.ghidra != 'unknown' and conv != f.ghidra.lstrip('_'):
            notes.add('ghidra=' + f.ghidra.lstrip('_'))
        problematic = {'incomplete-decode', 'incomplete-flow', 'analysis-limit', 'dynamic-stack-read',
                       'unknown-call-pop', 'indirect-transfer', 'unresolved-transfer', 'x87-depth-uncertain'} & notes
        two_sources = ((pop is not None and pop > 0 and body == count and body > 0) or
                       (pop == 0 and body == cleanup == count and cleanup > 0))
        conf = ('exact' if two_sources and not problematic and len(f.pops) == 1
                else 'high' if pop is not None and not problematic
                else 'medium' if pop is not None else 'low')
        if conv == 'unknown' or len(f.pops) > 1 or 'argument-sources-disagree' in notes:
            conf = 'low'
        row = dict(zip(COLUMNS, [f'{addr:08x}', f.size, f.name, conv,
                   'ecx' if conv in ('thiscall', 'fastcall') else 'none',
                   '' if pop is None else pop, '' if pop is None else count,
                   ','.join(r for r in REGS if r in f.live) or 'none', ret, ret_conf,
                   conf, len(set(calls[addr])), ';'.join(sorted(notes))]))
        rows.append(row)
        raw_evidence[row['address']] = {'body_dwords': body, 'caller_cleanup_dwords': cleanup,
                    'cleanups': cleanups[addr], 'pops': sorted(f.pops), 'reg_args': sorted(f.live),
                    'inferred_convention': raw_conv, 'inferred_ret': raw_ret, 'ghidra': f.ghidra,
                    'notes': sorted(f.notes), 'ranges': f.ranges, 'instruction_count': len(f.insns)}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('w', encoding='utf-8', newline='') as fh:
        writer = csv.DictWriter(fh, COLUMNS, delimiter='\t', lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)
    elapsed = round(time.perf_counter() - started, 3)
    report = {'functions': exported_count, 'vtable_only': len(functions) - exported_count,
              'rows': len(rows), 'seconds': elapsed, 'conventions': dict(Counter(r['convention'] for r in rows)),
              'confidence': dict(Counter(r['confidence'] for r in rows)), 'functions_evidence': raw_evidence}
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(f'wrote rows={len(rows)} seconds={elapsed:.3f} conventions={report["conventions"]}', flush=True)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    from re_query import main_checkout
    main_root = main_checkout(ROOT)
    parser.add_argument('--export', type=Path, default=main_root / 'ghidra/export/windows')
    parser.add_argument('--exe', type=Path, default=main_root / 'bin/SimCity4.exe')
    parser.add_argument('--output', type=Path, default=ROOT / 'docs/abi/functions.tsv')
    parser.add_argument('--vtables', type=Path, default=ROOT / 'docs/abi/vtables.tsv')
    parser.add_argument('--evidence', type=Path, default=ROOT / 'build-runs/abi-functions/evidence.json')
    args = parser.parse_args()
    contracts = json.loads((ROOT / 'tools/abi_direct_contracts.json').read_text(encoding='utf-8'))
    extract(args.export, args.exe, args.vtables, args.output, args.evidence, contracts)


if __name__ == '__main__':
    main()
