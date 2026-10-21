import re, sys
# usage: sets.py bf.txt ; reconstructs BuildFirstSets set contents (sets keyed by global address)
regs, pushes, sets, out = {}, [], {}, []
v = None
for line in open(sys.argv[1]):
    m = re.search(r'MOV (E[A-Z]+),0x([0-9a-f]{6,})$', line.strip())
    if m:
        regs[m.group(1)] = m.group(2)
    m = re.search(r'MOV dword ptr \[EBP \+ -0x4\],0x([0-9a-f]+)', line)
    if m:
        v = int(m.group(1), 16)
    m = re.search(r'MOV ECX,(E[A-Z]+)$', line.strip())
    if m:
        regs['ECX'] = regs.get(m.group(1))
    m = re.search(r'PUSH (0x[0-9a-f]+|E[A-Z]+)', line)
    if m:
        x = m.group(1)
        pushes.append(x[2:] if x.startswith('0x') else regs.get(x, x))
    if 'CALL 0x008fef47' in line:
        sets.setdefault(regs['ECX'], []).append(v); pushes = []
    if 'CALL 0x008ff187' in line:
        out.append('APPEND pushes=%s ecx=%s' % (pushes[-3:], regs.get('ECX')))
        pushes = []
for k, vals in sets.items():
    print(k, sorted(vals))
print('\n'.join(out))
