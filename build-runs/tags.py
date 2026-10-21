import subprocess, re, sys, struct
# usage: tags.py <asmfile> ; emits (string,id) at every insert call into the tag table
M = 'C:/Users/tlori/FreeCity4'
exe = open(M + '/bin/SimCity4.exe', 'rb').read()
pe = struct.unpack_from('<I', exe, 0x3c)[0]
nsec = struct.unpack_from('<H', exe, pe + 6)[0]
optsz = struct.unpack_from('<H', exe, pe + 20)[0]
base = struct.unpack_from('<I', exe, pe + 52)[0]
secs = []
for i in range(nsec):
    o = pe + 24 + optsz + 40 * i
    vs, va, rs, ro = struct.unpack_from('<IIII', exe, o + 8)
    secs.append((va + base, max(vs, rs), ro))
def cstr(a):
    for va, sz, ro in secs:
        if va <= a < va + sz:
            off = ro + a - va
            return exe[off:exe.index(b'\0', off)].decode('latin1')
    return '?%x' % a
pairs, s, v = [], None, None
for line in open(sys.argv[1]):
    m = re.search(r'PUSH 0x([0-9a-f]{6,})', line)
    if m:
        s = cstr(int(m.group(1), 16)); continue
    m = re.search(r'MOV dword ptr \[EBP \+ -0x(28|4)\],(0x[0-9a-f]+)', line)
    if m:
        v = int(m.group(2), 16); continue
    if 'CALL 0x0090116d' in line or 'CALL 0x00902d3b' in line:
        pairs.append((s, v))
print(len(pairs))
print('; '.join('%s=%d' % p for p in pairs))
