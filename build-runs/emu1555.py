"""Differential check: run every cSpanRender1555 RenderBlt slot (14x12) under unicorn on random inputs and
compare the destination against the spec model (span-render-1555.md section 7).
usage: emu1555.py [trials] [row,col ...]   prints one line per slot: PASS n / FAIL first-mismatch."""
import os, sys, struct, random
H = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, os.path.join(H, 'pylib'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP
EXE = 'C:/Users/tlori/FreeCity4/bin/SimCity4.exe'
d = open(EXE, 'rb').read()
pe = struct.unpack_from('<I', d, 0x3c)[0]; nsec = struct.unpack_from('<H', d, pe + 6)[0]
optsz = struct.unpack_from('<H', d, pe + 20)[0]; base = struct.unpack_from('<I', d, pe + 52)[0]
isz = struct.unpack_from('<I', d, pe + 80)[0]
mu = Uc(UC_ARCH_X86, UC_MODE_32); mu.mem_map(base, (isz + 0xfff) & ~0xfff)
o = pe + 24 + optsz
for i in range(nsec):
    vs, va, rs, ra = struct.unpack_from('<IIII', d, o + 8); mu.mem_write(base + va, d[ra:ra + min(rs, vs or rs)]); o += 40
STK, DAT, RET = 0x10000000, 0x20000000, 0x30000000
mu.mem_map(STK, 0x100000); mu.mem_map(DAT, 0x400000); mu.mem_map(RET, 0x1000)
def rd(a, n): return bytes(mu.mem_read(a, n))
def dw(a): return struct.unpack('<I', rd(a, 4))[0]
TAB = [[dw(dw(0x00AC1B44 + 4 * r) + 4 * c) for c in range(12)] for r in range(14)]
BPP = [2, 2, 2, 2, 4, 4, 1]; KEYM = [0xFFF, 0x7FFF, 0x7FFF, 0xFFFF, 0xFFFFFF, 0xFFFFFF, 0xFF]
KCM = KEYM[:6] + [0]  # 8-bit keyed row: the texel contributes no bits, so only key == 0 hides
# ---- spec model -------------------------------------------------------------------------------
def conv(f, s):            # section 7.3: source texel -> 1555
    if f == 0:
        c = lambda v: ((v << 1) | (v >> 3)) & 31
        return (c((s >> 8) & 15) << 10) | (c((s >> 4) & 15) << 5) | c(s & 15) | (0x8000 if s & 0x8000 else 0)
    if f == 1: return s & 0xFFFF
    if f == 2: return (s & 0x7FFF) | 0x8000
    if f == 3: return (((s >> 11) & 31) << 10) | (((s >> 6) & 31) << 5) | (s & 31) | 0x8000
    if f in (4, 5):
        v = (((s >> 19) & 31) << 10) | (((s >> 11) & 31) << 5) | ((s >> 3) & 31)
        return v | (0x8000 if (f == 5 or s & 0x80000000) else 0)
    return 0x7FFF | ((s & 0x80) << 8)
def to8888(f, s):          # (a, r, g, b) 8-bit, hypothesis per source
    e5 = lambda v: ((v << 3) | (v >> 2)) & 255
    if f == 0: return tuple(((s >> k) & 15) * 17 for k in (12, 8, 4, 0))
    if f in (1, 2): return (255 if (f == 2 or s & 0x8000) else 0, e5((s >> 10) & 31), e5((s >> 5) & 31), e5(s & 31))
    if f == 3: return (255, e5((s >> 11) & 31), (((s >> 5) & 63) << 2 | ((s >> 5) & 63) >> 4), e5(s & 31))
    if f in (4, 5): return (255 if f == 5 else s >> 24, (s >> 16) & 255, (s >> 8) & 255, s & 255)
    return (s, 255, 255, 255)
def pack(a, r, g, b): return ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3) | (0x8000 if a >= 128 else 0)
def blend(S, D, w):
    ch = lambda sh: ((w * ((S >> sh) & 31) + (32 - w) * ((D >> sh) & 31) + 16) >> 5) << sh
    return ch(10) | ch(5) | ch(0) | 0x8000
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
    return blend(pack(*m) | 0x8000, D, m[0] >> 3)
# ---- harness ----------------------------------------------------------------------------------
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
    elif mode == 2: u0, v0 = rng.randrange(W), rng.randrange(Hh)
    spans = []
    for _ in range(rng.randint(1, 4)):
        y = rng.randrange(DH if mode else Hh)
        if mode == 1:
            y = rng.randrange(DH)
            while ((v0 + dv * y) >> 16) >= Hh: y -= 1
            if y < 0: continue
            x1 = 0
            while x1 < DW and ((u0 + du * x1) >> 16) < W: x1 += 1
            if x1 == 0: continue
        else: x1 = DW if mode == 2 else W
        a = rng.randrange(x1); b = rng.randint(a, x1)
        spans.append((a, b, y))
    if not spans: return None
    mu.mem_write(sp, bytes(sbuf)); mu.mem_write(dp, bytes(dbuf))
    mu.mem_write(spans_a, b''.join(struct.pack('<II', (y << 16) | a, (y << 16) | b) for a, b, y in spans))
    args = struct.pack('<II', spans_a, len(spans)) + struct.pack('<6I', dp, dstr, DW, DH, 6, 2) + \
        struct.pack('<6I', sp, sstr, W, Hh, 0, bpp) + struct.pack('<8I', u0, v0, du, dv, f, col, keyv, key)
    esp = STK + 0x80000; mu.mem_write(esp, struct.pack('<I', RET) + args); mu.reg_write(UC_X86_REG_ESP, esp)
    mu.emu_start(TAB[r][c], RET, count=2000000)
    out = rd(dp, len(dbuf)); exp = list(dst); used = {}
    for a, b, y in spans:
        for x in range(a, b):
            if mode == 0: sx, sy = x, y
            elif mode == 1: sx, sy = ((u0 + du * x) & 0xFFFFFFFF) >> 16, ((v0 + dv * y) & 0xFFFFFFFF) >> 16
            else: sx, sy = (x + u0) % W, (y + v0) % Hh
            used[y * DW + x] = src[sy * W + sx]
            exp[y * DW + x] = model(f, key, c, src[sy * W + sx], exp[y * DW + x], col, keyv)
    for i, e in enumerate(exp):
        g = struct.unpack_from('<H', out, (i // DW) * dstr + (i % DW) * 2)[0]
        if g != e:
            x, y = i % DW, i // DW
            return 'x=%d y=%d s=%x D=%04x col=%08x key=%x exp=%04x got=%04x' % (x, y, used.get(i, -1), dst[i], col, keyv, e, g)
    return ''
def main():
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
