"""rd.py VA [count] [d|b|imm] : dump dwords/bytes at VA from bin/SimCity4.exe; imm VALUE: find occurrences of dword."""
import struct, sys
EXE = 'C:/Users/tlori/FreeCity4/bin/SimCity4.exe'
data = open(EXE, 'rb').read()
pe = struct.unpack_from('<I', data, 0x3C)[0]
nsec = struct.unpack_from('<H', data, pe + 6)[0]
optsz = struct.unpack_from('<H', data, pe + 20)[0]
base = struct.unpack_from('<I', data, pe + 24 + 28)[0]
secs = []
for i in range(nsec):
    o = pe + 24 + optsz + 40 * i
    vs, va, rs, ro = struct.unpack_from('<IIII', data, o + 8)
    secs.append((va + base, max(vs, rs), ro, rs))
def off(v):
    for a, sz, ro, rs in secs:
        if a <= v < a + rs:
            return ro + v - a
    raise SystemExit('unmapped %x' % v)
def va(o):
    for a, sz, ro, rs in secs:
        if ro <= o < ro + rs:
            return a + o - ro
mode = sys.argv[3] if len(sys.argv) > 3 else 'd'
if sys.argv[1] == 'imm':
    pat = struct.pack('<I', int(sys.argv[2], 16)); i = 0; hits = []
    while True:
        i = data.find(pat, i)
        if i < 0: break
        hits.append('%08x' % va(i)); i += 1
    print(len(hits), ' '.join(hits[:40]))
else:
    v = int(sys.argv[1], 16); n = int(sys.argv[2]) if len(sys.argv) > 2 else 8
    o = off(v)
    if mode == 'b':
        print(data[o:o + n].hex(' '))
    else:
        print(' '.join('%08x' % x for x in struct.unpack_from('<%dI' % n, data, o)))
