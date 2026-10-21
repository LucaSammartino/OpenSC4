"""Boundary proof for cSpanRender565 (span-render-565.md section 1.1) under unicorn:
vtable 0x00AC1844 entries, each slot called through its entry with an unmapped `this` (ECX never read),
bytes popped, EBX/EBP/ESI/EDI preserved, the two conformance examples, slot 0 with flags 0.
usage: python span565_bridge.py"""
import struct
import span565_emu as E
from unicorn.x86_const import (UC_X86_REG_ECX, UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_EBP,
                               UC_X86_REG_ESI, UC_X86_REG_EDI)
mu, DAT = E.mu, E.DAT
VT = [0x0082FF00, 0x0083DE50, 0x0083DEA0, 0x0083DFB0, 0x0083E070, 0x0083E210]
POPS = [4, 0x10, 0x10, 0x14, 0x10, 0x18]
KEEP = [(UC_X86_REG_EBX, 0x11111111), (UC_X86_REG_EBP, 0x22222222), (UC_X86_REG_ESI, 0x33333333),
        (UC_X86_REG_EDI, 0x44444444)]
fails = []

def call(slot, args, this=0xDEAD0000):
    fa = E.dw(0x00AC1844 + 4 * slot)
    if fa != VT[slot]: fails.append('slot %d entry %08x' % (slot, fa))
    mu.reg_write(UC_X86_REG_ECX, this)
    for r, v in KEEP: mu.reg_write(r, v)
    p = E.call(fa, args)
    if p != POPS[slot]: fails.append('slot %d popped %d' % (slot, p))
    for r, v in KEEP:
        if mu.reg_read(r) != v: fails.append('slot %d clobbers reg %d' % (slot, r))
    return mu.reg_read(UC_X86_REG_EAX)

def surface(stride, words):
    mu.mem_write(DAT + 0x10000, b'\xAA' * 0x100)
    mu.mem_write(DAT + 0x30000, struct.pack('<6I', DAT + 0x10000, stride, 5, 4, 5, 2))
    mu.mem_write(DAT + 0x20000, b''.join(struct.pack('<II', a, b) for a, b in words))

# (1) RenderSolid: three pixels of row 1, stride 12
surface(12, [(0x00010002, 0x00010005)])
call(1, struct.pack('<4I', DAT + 0x20000, 1, DAT + 0x30000, 0x1234F81F))
out = E.rd(DAT + 0x10000, 0x40)
want = bytearray(b'\xAA' * 0x40); want[16:22] = struct.pack('<3H', 0xF81F, 0xF81F, 0xF81F)
if out != bytes(want): fails.append('example 1 %s' % out.hex())
# (2) RenderBlt: A8888 texel 0x00FF8040, plain, no key, one pixel at (0, 0)
surface(12, [(0, 1)])
mu.mem_write(DAT + 0x40000, struct.pack('<I', 0x00FF8040))
mu.mem_write(DAT + 0x50000, struct.pack('<6I', DAT + 0x40000, 4, 1, 1, 9, 4) + struct.pack('<8I', 0, 0, 0, 0, 4, 0, 0, 0))
call(5, struct.pack('<6I', DAT + 0x20000, 1, DAT + 0x30000, DAT + 0x50000, DAT + 0x50018, 0))
g = struct.unpack('<H', E.rd(DAT + 0x10000, 2))[0]
if g != 0xFC08: fails.append('example 2 got %04x' % g)
# slots 2-4 with zero-length spans: only the boundary (entry, pops, registers) is checked here
surface(12, [(0x00010002, 0x00010002)])
mu.mem_write(DAT + 0x60000, bytes(0x30))
call(2, struct.pack('<4I', DAT + 0x20000, 1, DAT + 0x30000, DAT + 0x60000))
call(3, struct.pack('<5I', DAT + 0x20000, 1, DAT + 0x30000, 0x1234, 0x80FFFFFF))
call(4, struct.pack('<4I', DAT + 0x20000, 1, DAT + 0x30000, DAT + 0x60000))
# slot 0, flags 0: vtable reset to 0x00AC17AC, EAX = this, nothing freed
mu.mem_write(DAT + 0x70000, struct.pack('<I', 0x00AC1844))
eax = call(0, struct.pack('<I', 0), this=DAT + 0x70000)
if eax != DAT + 0x70000 or E.dw(DAT + 0x70000) != 0x00AC17AC: fails.append('slot 0 eax %08x' % eax)
print('PASS bridge 6/6 slots, 2 examples' if not fails else 'FAIL ' + '; '.join(fails))
