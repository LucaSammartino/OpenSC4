"""Differential proof: run every cSpanRender565 RenderBlt slot (14 rows x 12 columns, table 0x00AC1E28) under
unicorn on random inputs and compare the destination with the model of span-render-565.md section 7.
usage: python span565_blt.py [via] [trials] [row,col ...]   (via: call through RenderBlt instead of the slot)   one line per slot: PASS n / FAIL first-mismatch."""
import sys, struct, random
import span565_emu as E
mu, DAT, STK, RET, TAB, rd, M = E.mu, E.DAT, E.STK, E.RET, E.TAB, E.rd, E.M
BPP = [2, 2, 2, 2, 4, 4, 1]
KEYM = [0xFFF, 0x7FFF, 0x7FFF, 0xFFFF, 0xFFFFFF, 0xFFFFFF, 0xFF]   # caller's key mask per source (section 6.1)
KCM = KEYM[:6] + [0]   # bits of the texel the keyed body compares: the 8-bit keyed row compares none
# ---- model (spec section 7) -----------------------------------------------------------------
def to8888(f, s):            # (a, r, g, b) 8-bit per source format, section 7.4
    e5 = lambda v: ((v << 3) | (v >> 2)) & 255
    if f == 0: return tuple(((s >> k) & 15) * 17 for k in (12, 8, 4, 0))
    if f in (1, 2): return (255 if (f == 2 or s & 0x8000) else 0, e5((s >> 10) & 31), e5((s >> 5) & 31), e5(s & 31))
    if f == 3: return (255, e5((s >> 11) & 31), (((s >> 5) & 63) << 2 | ((s >> 5) & 63) >> 4), e5(s & 31))
    if f in (4, 5): return (255 if f == 5 else s >> 24, (s >> 16) & 255, (s >> 8) & 255, s & 255)
    return (s, 255, 255, 255)
def conv(f, s):              # section 7.3: source texel -> 565 (Blt, ConstantAlpha)
    if f == 0:
        r, g, b = (s >> 8) & 15, (s >> 4) & 15, s & 15
        return ((r << 1 | r >> 3) << 11) | ((g << 2 | g >> 2) << 5) | (b << 1 | b >> 3)
    if f in (1, 2):
        g = (s >> 5) & 31
        return ((s & 0x7C00) << 1) | ((g << 1 | g >> 4) << 5) | (s & 31)
    if f == 3: return s & 0xFFFF
    if f in (4, 5): return pack(0, (s >> 16) & 255, (s >> 8) & 255, s & 255)
    return 0xFFFF
def pack(a, r, g, b): return ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)
def blend(S, D, w):          # packed form of helper 0x00845450 (per-channel lerp when w <= 32)
    g = ((((S & 0x7E0) - (D & 0x7E0)) * w + (((D & 0x7E0) + 0x10) << 5)) & 0xFC00)
    rb = ((((S & 0xF81F) - (D & 0xF81F)) * w + 0x8010 + ((D & 0xF81F) << 5)) & 0x1F03E0)
    return ((g + rb) & M) >> 5 & 0xFFFF
def modul(f, s, col):
    a = to8888(f, s); m = ((col >> 24) & 255, (col >> 16) & 255, (col >> 8) & 255, col & 255)
    return tuple((a[i] * m[i] + 128) >> 8 for i in range(4))
def model(f, key, kind, s, D, col, keyv):
    if key and (s & KCM[f]) == keyv: return D
    k = kind & 3
    if k == 0: return conv(f, s)
    if k == 1: return blend(conv(f, s), D, col >> 27)
    m = modul(f, s, col)
    if k == 2: return pack(*m)
    return blend(pack(*m), D, m[0] >> 3)
# ---- harness ----------------------------------------------------------------------------------
def addr(mode, x0, x, y, u0, v0, du, dv, W, Hh):   # section 7.5 / 7.6
    if mode == 0: return x, y
    if mode == 1: return ((u0 + du * x) & M) >> 16, ((v0 + dv * y) & M) >> 16
    return (((x0 + u0) & M) % W + x - x0) % W, ((y + v0) & M) % Hh
def trial(r, c, rng):
    f, key, mode = r // 2, r & 1, c // 4
    bpp = BPP[f]; W, Hh = rng.randint(4, 40), rng.randint(1, 6); sstr = W * bpp + rng.choice((0, 4, 8))
    DW, DH, dstr = 64, 6, 64 * 2 + 8
    sp, dp, spans_a = DAT, DAT + 0x10000, DAT + 0x20000
    src = [rng.getrandbits(8 * bpp) for _ in range(W * Hh)]
    keyv = rng.getrandbits(32) & KEYM[f]
    if f == 6 and rng.random() < 0.5: keyv = 0
    for i in range(len(src)):
        if rng.random() < 0.25: src[i] = (src[i] & ~KEYM[f] & ((1 << 8 * bpp) - 1)) | keyv
    sbuf = bytearray(sstr * Hh)
    for y in range(Hh):
        for x in range(W): sbuf[y * sstr + x * bpp:y * sstr + (x + 1) * bpp] = src[y * W + x].to_bytes(bpp, 'little')
    dst = [rng.getrandbits(16) for _ in range(DW * DH)]
    dbuf = bytearray(dstr * DH)
    for i, v in enumerate(dst): struct.pack_into('<H', dbuf, (i // DW) * dstr + (i % DW) * 2, v)
    col = rng.choice((rng.getrandbits(32), 0xFFFFFFFF, 0x80808080, 0x00FFFFFF, 0xFF000000 | rng.getrandbits(24)))
    u0 = v0 = du = dv = 0
    if mode == 1:
        du, dv = rng.randint(0x4000, 0x20000), rng.randint(0x4000, 0x20000); u0, v0 = rng.getrandbits(16), rng.getrandbits(16)
    elif mode == 2: u0, v0 = rng.choice(((rng.randrange(W), rng.randrange(Hh)), (rng.getrandbits(32), rng.getrandbits(32))))
    spans = []
    for _ in range(rng.randint(1, 4)):
        if mode == 1:
            y = rng.randrange(DH)
            while ((v0 + dv * y) >> 16) >= Hh: y -= 1
            if y < 0: continue
            x1 = 0
            while x1 < DW and ((u0 + du * x1) >> 16) < W: x1 += 1
            if x1 == 0: continue
        else:
            y = rng.randrange(DH if mode else Hh); x1 = DW if mode == 2 else W
        a = rng.randrange(x1); b = rng.randint(a, x1)
        spans.append((a, b, y))
    if not spans: return None
    mu.mem_write(sp, bytes(sbuf)); mu.mem_write(dp, bytes(dbuf))
    mu.mem_write(spans_a, b''.join(struct.pack('<II', (y << 16) | a, (y << 16) | b) for a, b, y in spans))
    args = struct.pack('<II', spans_a, len(spans)) + struct.pack('<6I', dp, dstr, DW, DH, 5, 2) + \
        struct.pack('<6I', sp, sstr, W, Hh, 0, bpp) + struct.pack('<8I', u0, v0, du, dv, f, col, keyv, key)
    if VIA:   # through RenderBlt 0x0083E210(spans, n, &dst, &src, &b, kind): records by pointer, dispatch on b
        mu.mem_write(DAT + 0x38000, args[8:])
        popped = E.call(0x0083E210, args[:8] + struct.pack('<4I', DAT + 0x38000, DAT + 0x38018, DAT + 0x38030, c), 4000000)
        if popped != 0x18: return 'RenderBlt popped %d' % popped
    else: E.call(TAB[r][c], args, 4000000)
    out = rd(dp, len(dbuf)); exp = list(dst); used = {}
    for a, b, y in spans:
        for x in range(a, b):
            sx, sy = addr(mode, a, x, y, u0, v0, du, dv, W, Hh)
            used[y * DW + x] = src[sy * W + sx]
            exp[y * DW + x] = model(f, key, c, src[sy * W + sx], exp[y * DW + x], col, keyv)
    for i, e in enumerate(exp):
        g = struct.unpack_from('<H', out, (i // DW) * dstr + (i % DW) * 2)[0]
        if g != e:
            return 'x=%d y=%d s=%x D=%04x col=%08x key=%x exp=%04x got=%04x' % (
                i % DW, i // DW, used.get(i, -1), dst[i], col, keyv, e, g)
    return ''
VIA = False
def main():
    global VIA
    if len(sys.argv) > 1 and sys.argv[1] == 'via': VIA = True; del sys.argv[1]
    nt = int(sys.argv[1]) if len(sys.argv) > 1 else 50
    sel = [tuple(map(int, a.split(','))) for a in sys.argv[2:]] or [(r, c) for r in range(14) for c in range(12)]
    npass = 0
    for r, c in sel:
        rng = random.Random(r * 100 + c); msg = 'PASS'; ok = 0
        for t in range(nt):
            try: res = trial(r, c, rng)
            except Exception as e: res = 'EMU %s' % e
            if res is None: continue
            if res: msg = 'FAIL ' + res; break
            ok += 1
        npass += msg == 'PASS'
        print('r%d c%d %08x %s %d' % (r, c, TAB[r][c], msg, ok))
    print('passed %d/%d' % (npass, len(sel)))
if __name__ == '__main__': main()
