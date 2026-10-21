"""Normalise every cSpanRender1555 RenderBlt table slot and compare structurally.
usage: norm.py dump            -> writes build-runs/norm/<addr>.txt (normalised listing) + table.tsv
       norm.py col C REF       -> diff every row of column C against row REF (hunk summary)
       norm.py pair A B        -> unified diff of two normalised functions (by address)
       norm.py groups          -> identical-normalised-body groups over all slots
Normalisation: branch/call targets inside the function -> L<offset>, outside -> the absolute VA;
other operands verbatim (constants, masks, strides kept)."""
import struct, sys, os, re, difflib
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
EXE = 'C:/Users/tlori/FreeCity4/bin/SimCity4.exe'
FT = 'C:/Users/tlori/FreeCity4/ghidra/export/windows/functions.tsv'
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'norm')
d = open(EXE, 'rb').read()
pe = struct.unpack_from('<I', d, 0x3c)[0]
nsec = struct.unpack_from('<H', d, pe + 6)[0]
optsz = struct.unpack_from('<H', d, pe + 20)[0]
base = struct.unpack_from('<I', d, pe + 24 + 28)[0]
secs = []
o = pe + 24 + optsz
for i in range(nsec):
    vs, va, rs, ra = struct.unpack_from('<IIII', d, o + 8)
    secs.append((va, max(vs, rs), ra)); o += 40
def off(v):
    r = v - base
    for va, sz, ra in secs:
        if va <= r < va + sz: return ra + r - va
    raise ValueError(hex(v))
def dw(v): return struct.unpack_from('<I', d, off(v))[0]
sizes = {}
for ln in open(FT, encoding='utf-8'):
    f = ln.split('\t')
    if len(f) > 2 and re.fullmatch('[0-9a-f]{8}', f[0]): sizes[int(f[0], 16)] = int(f[2])
def table():
    rows = []
    for i in range(14):
        rp = dw(0x00AC1B44 + 4 * i)
        rows.append([dw(rp + 4 * j) for j in range(12)])
    return rows
md = Cs(CS_ARCH_X86, CS_MODE_32)
def norm(fa):
    sz = sizes[fa]; code = d[off(fa):off(fa) + sz]; out = []
    for ins in md.disasm(code, fa):
        ops = ins.op_str
        if ins.mnemonic.startswith('j') or ins.mnemonic == 'call':
            m = re.fullmatch(r'0x([0-9a-f]+)', ops)
            if m:
                t = int(m.group(1), 16)
                ops = 'L%x' % (t - fa) if fa <= t < fa + sz else hex(t)
        out.append((ins.mnemonic + ' ' + ops).strip())
    return out
def load(fa):
    p = os.path.join(OUT, '%08x.txt' % fa)
    if not os.path.exists(p):
        os.makedirs(OUT, exist_ok=True); open(p, 'w').write('\n'.join(norm(fa)) + '\n')
    return open(p).read().splitlines()
cmd = sys.argv[1]
T = table()
if cmd == 'dump':
    with open(os.path.join(OUT if os.path.isdir(OUT) else (os.makedirs(OUT) or OUT), 'table.tsv'), 'w') as f:
        for r, row in enumerate(T):
            for c, fa in enumerate(row):
                n = load(fa); f.write('%d\t%d\t%08x\t%d\t%d\n' % (r, c, fa, sizes.get(fa, -1), len(n)))
    print('ok', sum(len(r) for r in T), 'slots', len({a for r in T for a in r}), 'distinct')
elif cmd == 'col':
    c, ref = int(sys.argv[2]), int(sys.argv[3]); a = load(T[ref][c])
    for r in range(14):
        b = load(T[r][c]); sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
        hunks = [(t, i2 - i1, j2 - j1) for t, i1, i2, j1, j2 in sm.get_opcodes() if t != 'equal']
        print('row%2d %08x n=%3d same=%3d hunks=%s' % (r, T[r][c], len(b), sum(i2 - i1 for t, i1, i2, j1, j2 in sm.get_opcodes() if t == 'equal'), hunks[:6]))
elif cmd == 'pair':
    a, b = load(int(sys.argv[2], 16)), load(int(sys.argv[3], 16))
    for l in list(difflib.unified_diff(a, b, lineterm='', n=int(sys.argv[4]) if len(sys.argv) > 4 else 1))[2:]: print(l)
elif cmd == 'groups':
    g = {}
    for r, row in enumerate(T):
        for c, fa in enumerate(row): g.setdefault(tuple(load(fa)), []).append('r%dc%d' % (r, c))
    print(len(g), 'distinct normalised bodies')
    for k, v in g.items():
        if len(v) > 1: print(len(k), v)
