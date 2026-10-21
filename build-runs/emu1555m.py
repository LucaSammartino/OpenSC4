"""Differential check of the five cSpanRender1555 methods (spec sections 2-6) under unicorn.
usage: emu1555m.py [trials]"""
import sys, struct, random
import emu1555 as E
mu, DAT, STK, RET = E.mu, E.DAT, E.STK, E.RET
DW, DH, DSTR = 64, 6, 136
def call(fa, args):
    esp = STK + 0x80000; mu.mem_write(esp, struct.pack('<I', RET) + args)
    mu.reg_write(E.UC_X86_REG_ESP, esp); mu.emu_start(fa, RET, count=2000000)
M = 0xFFFFFFFF
def blend(S, D, w):   # packed form of 0x008313D9..0x00831421 (equals the per-channel lerp for w <= 32)
    sp, dp, sg, dg = S & 0x7C1F, D & 0x7C1F, S & 0x3E0, D & 0x3E0
    rb = (((sp - dp) * w + (dp << 5) + 0x4010) & M) >> 5
    g = (((sg - dg) * w + ((dg + 0x10) << 5)) & M) >> 5
    return (rb | 0x83E0) & (g | 0xFC1F) & 0xFFFF
def setup(rng):
    dst = [rng.getrandbits(16) for _ in range(DW * DH)]; buf = bytearray(DSTR * DH)
    for i, v in enumerate(dst): struct.pack_into('<H', buf, (i // DW) * DSTR + (i % DW) * 2, v)
    mu.mem_write(DAT + 0x10000, bytes(buf))
    spans = []
    for _ in range(rng.randint(1, 4)):
        y = rng.randrange(DH); a = rng.randrange(DW); spans.append((a, rng.randint(a, DW), y))
    mu.mem_write(DAT + 0x20000, b''.join(struct.pack('<II', (y << 16) | a, (y << 16) | b) for a, b, y in spans))
    mu.mem_write(DAT + 0x30000, struct.pack('<6I', DAT + 0x10000, DSTR, DW, DH, 6, 2))
    return dst, spans, struct.pack('<III', DAT + 0x20000, len(spans), DAT + 0x30000)
def check(exp):
    out = E.rd(DAT + 0x10000, DSTR * DH)
    for i, e in enumerate(exp):
        g = struct.unpack_from('<H', out, (i // DW) * DSTR + (i % DW) * 2)[0]
        if g != e: return 'x=%d y=%d exp=%04x got=%04x' % (i % DW, i // DW, e, g)
    return ''
def t_solid(rng):
    dst, spans, a = setup(rng); col = rng.getrandbits(32); call(0x008310B0, a + struct.pack('<I', col))
    for x0, x1, y in spans:
        for x in range(x0, x1): dst[y * DW + x] = col & 0xFFFF
    return check(dst)
def t_alpha(rng):
    dst, spans, a = setup(rng); src = rng.getrandbits(32); argb = rng.getrandbits(32)
    call(0x00831200, a + struct.pack('<II', src, argb)); w = ((argb >> 24) * 0x818180 & M) >> 26
    for x0, x1, y in spans:
        for x in range(x0, x1): dst[y * DW + x] = blend(src, dst[y * DW + x], w)
    return check(dst)
def grad(rng, alpha):
    dst, spans, a = setup(rng)
    g = [rng.getrandbits(24) for _ in range(4)] + [rng.randint(-0x40000, 0x40000) & M for _ in range(8)]
    mu.mem_write(DAT + 0x40000, struct.pack('<12I', *g)); call(0x008312D0 if alpha else 0x00831100, a + struct.pack('<I', DAT + 0x40000))
    for x0, x1, y in spans:
        V = [(g[i] + g[4 + i] * x0 + g[8 + i] * y) & M for i in range(4)]
        sx = lambda v: v - (1 << 32) if v >> 31 else v
        for k in range(x1 - x0):
            if not alpha:
                B = ((V[0] + k * g[4]) & M) >> 16 & 31; G = ((V[1] + k * (g[5] << 5)) & M) >> 21 & 31
                R = ((V[2] + k * (g[6] << 10)) & M) >> 26 & 31; dst[y * DW + x0 + k] = R << 10 | G << 5 | B
            else:
                B = ((V[0] >> 3) + k * (sx(g[4]) >> 3) & M) >> 16 & 31; G = (((V[1] << 2) + k * (g[5] << 2)) & M) >> 21 & 31
                R = (((V[2] << 7) + k * (g[6] << 7)) & M) >> 26 & 31; w = ((V[3] * 0x84 + k * (g[7] * 0x84)) & M) >> 26
                i = y * DW + x0 + k; dst[i] = blend(R << 10 | G << 5 | B, dst[i], w)
    return check(dst)
def t_blt(rng):   # dispatch only: RenderBlt must call rows[b.key + 2*b.fmt][kind] with the records by value
    r, c = rng.randrange(14), rng.randrange(12); hit = []
    h = mu.hook_add(E.UC_HOOK_CODE, lambda u, ad, sz, ud: hit.append(ad), begin=E.TAB[r][c], end=E.TAB[r][c])
    dst, spans, a = setup(rng)
    mu.mem_write(DAT + 0x50000, struct.pack('<6I', DAT, 4, 4, 4, 0, 2) + struct.pack('<8I', 0, 0, 0, 0, r // 2, 0, 0, r & 1))
    try: call(0x00831470, a + struct.pack('<III', DAT + 0x50000, DAT + 0x50018, c))
    except Exception: pass
    mu.hook_del(h); return '' if hit else 'slot r%d c%d not reached' % (r, c)
E.UC_HOOK_CODE = __import__('unicorn').UC_HOOK_CODE
nt = int(sys.argv[1]) if len(sys.argv) > 1 else 200
for name, fn in (('Solid', t_solid), ('Alpha', t_alpha), ('Gradient', lambda r: grad(r, 0)),
                 ('AlphaGradient', lambda r: grad(r, 1)), ('Blt dispatch', t_blt)):
    rng = random.Random(name); res = ''
    for t in range(nt):
        res = fn(rng)
        if res: break
    print(name, 'FAIL ' + res if res else 'PASS %d' % nt)
