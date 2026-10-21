"""Differential proof of the five cSpanRender565 methods (span-render-565.md sections 2-6) under unicorn:
random surfaces, spans, colours and gradient records (full 32-bit range) against the spec model; also checks the
bytes each method pops. usage: python span565_methods.py [trials]"""
import sys, struct, random
import span565_emu as E
from span565_blt import blend
mu, DAT, M, TAB = E.mu, E.DAT, E.M, E.TAB
DW, DH, DSTR = 64, 6, 136
def setup(rng):
    dst = [rng.getrandbits(16) for _ in range(DW * DH)]; buf = bytearray(DSTR * DH)
    for i, v in enumerate(dst): struct.pack_into('<H', buf, (i // DW) * DSTR + (i % DW) * 2, v)
    mu.mem_write(DAT + 0x10000, bytes(buf))
    spans = []
    for _ in range(rng.randint(1, 4)):
        y = rng.randrange(DH); a = rng.randrange(DW); spans.append((a, rng.randint(a, DW), y))
    mu.mem_write(DAT + 0x20000, b''.join(struct.pack('<II', (y << 16) | a, (y << 16) | b) for a, b, y in spans))
    mu.mem_write(DAT + 0x30000, struct.pack('<6I', DAT + 0x10000, DSTR, DW, DH, 5, 2))
    return dst, spans, struct.pack('<III', DAT + 0x20000, len(spans), DAT + 0x30000)
def check(exp, popped, want):
    if popped != want: return 'popped %d want %d' % (popped, want)
    out = E.rd(DAT + 0x10000, DSTR * DH)
    for i, e in enumerate(exp):
        g = struct.unpack_from('<H', out, (i // DW) * DSTR + (i % DW) * 2)[0]
        if g != e: return 'x=%d y=%d exp=%04x got=%04x' % (i % DW, i // DW, e, g)
    return ''
def t_solid(rng):
    dst, spans, a = setup(rng); col = rng.getrandbits(32); p = E.call(0x0083DE50, a + struct.pack('<I', col))
    for x0, x1, y in spans:
        for x in range(x0, x1): dst[y * DW + x] = col & 0xFFFF
    return check(dst, p, 0x10)
def weight(a8): return ((a8 * 0x818180) & M) >> 26
def t_alpha(rng):
    dst, spans, a = setup(rng); src = rng.getrandbits(32); argb = rng.getrandbits(32)
    p = E.call(0x0083DFB0, a + struct.pack('<II', src, argb)); w = weight(argb >> 24)
    for x0, x1, y in spans:
        for x in range(x0, x1): dst[y * DW + x] = blend(src & 0xFFFF, dst[y * DW + x], w)
    return check(dst, p, 0x14)
def gval(rng):
    return rng.choice((rng.getrandbits(32), rng.getrandbits(24), rng.randint(-0x40000, 0x40000) & M))
def grad(rng, alpha):
    dst, spans, a = setup(rng); g = [gval(rng) for _ in range(12)]
    mu.mem_write(DAT + 0x40000, struct.pack('<12I', *g))
    p = E.call(0x0083E070 if alpha else 0x0083DEA0, a + struct.pack('<I', DAT + 0x40000))
    sar3 = lambda v: ((v - (1 << 32) if v >> 31 else v) >> 3) & M
    for x0, x1, y in spans:
        V = [(g[i] + g[4 + i] * x0 + g[8 + i] * y) & M for i in range(4)]
        for k in range(x1 - x0):
            b = (((V[0] >> 3) + k * sar3(g[4])) & M) >> 16 & 0x1F          # B: shifted start, truncated step
            gg = (((V[1] + k * g[5]) << 3) & M) >> 21 & 0x3F               # G: bits 18-23 of the exact value
            r = (((V[2] + k * g[6]) << 8) & M) >> 27                       # R: bits 19-23 of the exact value
            S = r << 11 | gg << 5 | b; i = y * DW + x0 + k
            if not alpha: dst[i] = S
            else: dst[i] = blend(S, dst[i], (((V[3] + k * g[7]) * 0x84) & M) >> 26)
    return check(dst, p, 0x10)
def t_blt(rng):   # dispatch: RenderBlt must reach rows[b.key + 2*b.fmt][kind], records copied by value
    r, c = rng.randrange(14), rng.randrange(12); hit = []
    h = mu.hook_add(E.UC_HOOK_CODE, lambda u, ad, sz, ud: hit.append(ad), begin=TAB[r][c], end=TAB[r][c])
    dst, spans, a = setup(rng)
    mu.mem_write(DAT + 0x50000, struct.pack('<6I', DAT, 4, 4, 4, 0, 2) + struct.pack('<8I', 0, 0, 0, 0, r // 2, 0, 0, r & 1))
    try: E.call(0x0083E210, a + struct.pack('<III', DAT + 0x50000, DAT + 0x50018, c), 100)
    except Exception: pass
    mu.hook_del(h); return '' if hit else 'slot r%d c%d not reached' % (r, c)
def main():
    nt = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    print('weight thresholds', ' '.join('%d:%d' % (w, next(a for a in range(256) if weight(a) == w))
                                       for w in range(33) if any(weight(a) == w for a in range(256))))
    for name, fn in (('Solid', t_solid), ('Alpha', t_alpha), ('Gradient', lambda r: grad(r, 0)),
                     ('AlphaGradient', lambda r: grad(r, 1)), ('Blt dispatch', t_blt)):
        rng = random.Random(name); res = ''
        for t in range(nt):
            res = fn(rng)
            if res: break
        print(name, 'FAIL ' + res if res else 'PASS %d' % nt)
if __name__ == '__main__': main()
