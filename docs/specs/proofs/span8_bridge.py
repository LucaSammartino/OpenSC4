# span-render-8.md section 10: runs the shipped bridge code (cGZBuffer Init/Lock/colour map, context Init,
# SetColorTable, DrawPreclippedSpanList) in the emulator on a constructed type-2 fixture.
# The global heap alloc/free (0x005E55E0 / 0x005E5620) are replaced by a bump allocator; everything else is shipped code.
import sys, struct, random
import span8_emu as E
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_EAX, UC_X86_REG_ESP, UC_X86_REG_EIP
mu, DAT = E.mu, E.DAT
HEAP = [DAT + 0x200000]
ALLOCS = []
def hook(uc, addr, size, _):
    esp = uc.reg_read(UC_X86_REG_ESP); ret = E.dw(esp)
    if addr == 0x005E55E0:
        n = E.dw(esp + 4); p = HEAP[0]; HEAP[0] += (n + 15) & ~15; ALLOCS.append(n)
        uc.mem_write(p, b'\0' * n); uc.reg_write(UC_X86_REG_EAX, p)
    uc.reg_write(UC_X86_REG_ESP, esp + 4); uc.reg_write(UC_X86_REG_EIP, ret)   # cdecl: caller pops
mu.hook_add(UC_HOOK_CODE, hook, begin=0x005E55E0, end=0x005E55E0)
mu.hook_add(UC_HOOK_CODE, hook, begin=0x005E5620, end=0x005E5620)
def call(this, fa, args):
    mu.reg_write(UC_X86_REG_ECX, this); popped = E.call(fa, args); return popped, mu.reg_read(UC_X86_REG_EAX)
fails = []
def ok(cond, what):
    if not cond: fails.append(what)
def new_buffer(w, h, ctype, bpp):
    b = HEAP[0]; HEAP[0] += 0x100; mu.mem_write(b, b'\0' * 0x100)
    mu.mem_write(b, struct.pack('<I', 0x00AC1400)); mu.mem_write(b + 0x2C, struct.pack('<I', 1))
    if ctype in (1, 3):   # palette types: storage set-up 0x00826390 asks the framework for a palette service; lay out by hand
        st = (w * (bpp >> 3) + 3) & ~3; p = HEAP[0]; HEAP[0] += (st * h + 15) & ~15
        for off, v in ((0xC, ctype), (0x10, bpp), (0x1C, w), (0x20, h), (0x3C, p), (0x40, st)): mu.mem_write(b + off, struct.pack('<I', v))
        return b, 0x10, 1
    pop, r = call(b, 0x008269B0, struct.pack('<4I', w, h, ctype, bpp))          # cGZBuffer::Init (vfunc +0x0C)
    return b, pop, r & 0xFF
def new_ctx():
    c = HEAP[0]; HEAP[0] += 0x100; mu.mem_write(c, b'\0' * 0x100)
    call(c, 0x00830690, b'')                                                     # constructor (factory 0x0082B910)
    return c
def main():
    rng = random.Random(1)
    # 10.4 buffer Init: stride (w*bpp/8+3)&~3, storage stride*h
    for w, h, t, bpp in ((10, 3, 2, 8), (1, 1, 2, 8), (13, 5, 2, 8), (7, 2, 4, 16)):
        del ALLOCS[:]; b, pop, r = new_buffer(w, h, t, bpp); st = (w * (bpp >> 3) + 3) & ~3
        ok(pop == 0x10 and r == 1 and E.dw(b + 0x40) == st and ALLOCS == [st * h], 'buffer init %r' % ((w, h, t, bpp),))
    b, pop, r = new_buffer(0, 3, 2, 8); ok(r == 0, 'init w=0 must fail')
    # 10.4 colour mapping: 0 for type 2 in both buffer classes
    b, _, _ = new_buffer(4, 4, 2, 8)
    for _ in range(200):
        rgb = [rng.getrandbits(32) for _ in range(3)]
        pop, v = call(b, 0x00825DA0, struct.pack('<3I', *rgb)); ok(pop == 12 and v == 0, 'map 0x00825DA0')
    o = HEAP[0]; HEAP[0] += 0x100; mu.mem_write(o + 0x2C, struct.pack('<I', 2))
    for _ in range(50):
        pop, v = call(o, 0x0082AD30, struct.pack('<3I', *[rng.getrandbits(32) for _ in range(3)])); ok(pop == 12 and v == 0, 'map 0x0082AD30')
    # 10.3 context Init: renderer per colour type
    want = {2: 0x00AC188C, 4: 0x00AC182C, 5: 0x00AC1844, 6: 0x00AC182C, 7: 0x00AC185C, 8: 0x00AC1874, 9: 0x00AC1874}
    for t in range(0, 12):
        b, _, _ = new_buffer(8, 2, t, 8); c = new_ctx(); del ALLOCS[:]
        pop, r = call(c, 0x00830710, struct.pack('<I', b))
        if t in want:
            rend = E.dw(c + 0xB0)
            ok(pop == 4 and r & 0xFF == 1 and ALLOCS == [4] and E.dw(rend) == want[t], 'ctx init type %d' % t)
            rec = struct.unpack('<6I', E.rd(c + 0x78, 24))
            ok(rec == (E.dw(b + 0x3C), 8, 8, 2, t, 1), 'dst record type %d %r' % (t, rec))
            ok(E.rd(c + 0x75, 1) == b'\1' and E.dw(b + 0x44) == 0x8040 and struct.unpack('<H', E.rd(b + 0x38, 2))[0] == 1, 'lock type %d' % t)
        else:
            ok(pop == 4 and r & 0xFF == 0 and E.dw(c + 8) == 0 and E.dw(c + 0xB0) == 0, 'ctx init fail type %d' % t)
            ok(struct.unpack('<H', E.rd(b + 0x38, 2))[0] == 1, 'type %d left locked' % t)   # compat: lock not undone
    # 10.4/10.5 end to end on type 2: SetColorTable then DrawPreclippedSpanList (solid and alpha flag)
    for alpha in (0, 1):
        for _ in range(50):
            b, _, _ = new_buffer(8, 2, 2, 8); c = new_ctx(); call(c, 0x00830710, struct.pack('<I', b))
            px = E.dw(b + 0x3C); mu.mem_write(px, bytes(rng.getrandbits(8) | 1 for _ in range(16)))
            argb = rng.getrandbits(32); tab = HEAP[0]; HEAP[0] += 16; mu.mem_write(tab, struct.pack('<I', argb))
            pop, _ = call(c, 0x0082FF70, struct.pack('<3I', 0, 1, tab))
            ok(pop == 12 and E.dw(c + 0xB4) == argb and E.dw(c + 0xC4) == 0, 'SetColorTable')
            mu.mem_write(c + 0x41, bytes([alpha]))
            x0 = rng.randrange(8); x1 = rng.randint(x0, 8); y = rng.randrange(2)
            sp = HEAP[0]; HEAP[0] += 16; mu.mem_write(sp, struct.pack('<II', (y << 16) | x0, (y << 16) | x1))
            before = E.rd(px, 16); pop, _ = call(c, 0x008302E0, struct.pack('<II', sp, 1)); after = E.rd(px, 16)
            exp = bytearray(before)
            for x in range(x0, x1): exp[y * 8 + x] = 0
            ok(pop == 8 and after == bytes(exp), 'draw solid alpha=%d' % alpha)
    print('span8_bridge:', 'PASS' if not fails else 'FAIL %d: %s' % (len(fails), '; '.join(fails[:6])))
    return 1 if fails else 0
if __name__ == '__main__':
    sys.exit(main())
