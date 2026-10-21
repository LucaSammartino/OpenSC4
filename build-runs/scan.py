"""scan.py: (1) E8 calls to the colour-pair ctor 0x0044A490 with the imm8 pushes in the 24 bytes before;
(2) PUSH 4 / PUSH 6 (6A 04/6A 06) followed within 16 bytes by a CALL [reg+0x14] (FF 5x 14);
(3) count of CALL [reg+0x6C] (FF 5x 6C) sites in .text."""
import struct, sys
sys.argv = ['rd', 'imm', '0']
exec(open('C:/Users/tlori/FreeCity4/.claude/worktrees/re-span1555-bridge4/build-runs/rd.py').read().split("mode = sys.argv")[0])
text = [s for s in secs if s[0] <= 0x00831470 < s[0] + s[3]][0]
a0, sz, ro, rs = text
code = data[ro:ro + rs]
def pushes(i):
    w = code[max(0, i - 24):i]; out = []
    for j in range(len(w) - 1):
        if w[j] == 0x6A: out.append(w[j + 1])
    return out
ctor = 0x0044A490; hits = {}
for i in range(len(code) - 5):
    if code[i] == 0xE8:
        t = (a0 + i + 5 + struct.unpack_from('<i', code, i + 1)[0]) & 0xffffffff
        if t == ctor:
            hits.setdefault(tuple(pushes(i)[-2:]), []).append('%08x' % (a0 + i))
print('ctor calls', sum(len(v) for v in hits.values()))
for k, v in hits.items(): print('  pushes', k, len(v), ' '.join(v[:6]))
c14 = []
for i in range(len(code) - 2):
    if code[i] == 0x6A and code[i + 1] in (4, 6):
        w = code[i + 2:i + 18]
        for j in range(len(w) - 2):
            if w[j] == 0xFF and (w[j + 1] & 0xF8) == 0x50 and w[j + 2] == 0x14:
                c14.append('%08x/%d' % (a0 + i, code[i + 1])); break
print('push4/6 then call+0x14:', len(c14), ' '.join(c14[:40]))
n6c = sum(1 for i in range(len(code) - 2) if code[i] == 0xFF and (code[i + 1] & 0xF8) == 0x50 and code[i + 1] != 0x54 and code[i + 2] == 0x6C)
print('call [reg+0x6c] sites', n6c)
