"""Emulate SetGradientTransform 0x0082FDB0 per channel under PC24 vs PC53 (round-nearest-even)."""
import struct, math, sys
K = 16711680.0
def f32(v): return struct.unpack('<f', struct.pack('<f', v))[0]
def r24(v): return f32(v)  # 24-bit significand rounding, nearest-even (no exponent issues here)
def rint(v): return int(round(v)) if abs(v - math.floor(v) - 0.5) > 1e-12 else (int(math.floor(v)) + (int(math.floor(v)) & 1))
def chan(p, pdx, pdy, x0, y0, pc):
    rnd = r24 if pc == 24 else (lambda v: v)
    x0, y0 = f32(x0), f32(y0)
    dx = rint(f32(rnd(f32(pdx) * K))); dy = rint(f32(rnd(f32(pdy) * K)))
    s = rint(f32(rnd(f32(p) * K))) + 0x8000
    a = rnd(dx * x0); b = rnd(dy * y0); sm = rnd(a + b); d = rnd(s - sm)
    return s, dx, dy, int(d)  # int() truncates toward zero
for args in [(1.0, 0, 0, 0, 0), (0.5, 0.01, 0.02, 100.25, 37.75), (0.75, -0.003, 0.0007, 1023.3, 767.9), (0.2, 0.1, 0.1, 333.3, 222.2)]:
    print(args, 'PC24', chan(*args, pc=24), 'PC53', chan(*args, pc=53))
