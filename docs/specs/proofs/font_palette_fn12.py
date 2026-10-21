"""FN12 proof (drawing-fonts.md section 9): runs the shipped cGZBuffer colour split (+0x7C, 0x00825F80) and compose
(+0x78, 0x00825DA0) from bin/SimCity4.exe under unicorn, checks them against the spec rules for colour types 4..9 and
others, then rebuilds the font palette (0x0089838C arithmetic) from the emulated calls and prints edge vectors.
Needs unicorn and pefile (pip install, or a pylib/ dir next to this file or in <worktree>/build-runs/).
usage: python font_palette_fn12.py [path-to-SimCity4.exe]"""
import os, sys, struct
H = os.path.dirname(os.path.abspath(__file__))
for p in (os.path.join(H, 'pylib'), os.path.join(H, '..', '..', '..', 'build-runs', 'pylib')):
    if os.path.isdir(p): sys.path.insert(0, p)
import pefile
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EAX

EXE = sys.argv[1] if len(sys.argv) > 1 else 'C:/Users/tlori/FreeCity4/bin/SimCity4.exe'
pe = pefile.PE(EXE, fast_load=True)
BASE = pe.OPTIONAL_HEADER.ImageBase
mu = Uc(UC_ARCH_X86, UC_MODE_32)
LO, HI = 0x00825000, 0x00827000
mu.mem_map(LO, HI - LO); mu.mem_write(LO, pe.get_data(LO - BASE, HI - LO))
OBJ, STK, RET = 0x10000000, 0x20000000, 0x30000000
for a in (OBJ, STK, RET): mu.mem_map(a, 0x10000)
SPLIT, COMPOSE = 0x00825F80, 0x00825DA0

def call(fa, typ, args):
    mu.mem_write(OBJ + 0xC, struct.pack('<I', typ))
    esp = STK + 0x8000
    mu.mem_write(esp, struct.pack('<I', RET) + b''.join(struct.pack('<I', x & 0xFFFFFFFF) for x in args))
    mu.reg_write(UC_X86_REG_ESP, esp); mu.reg_write(UC_X86_REG_ECX, OBJ)
    mu.emu_start(fa, RET, count=1000)
    return mu.reg_read(UC_X86_REG_EAX), mu.reg_read(UC_X86_REG_ESP) - esp - 4

OUT = STK + 0x100
def split(typ, c):
    mu.mem_write(OUT, b'\xAA\xBB\xCC')                       # sentinels: untouched outputs stay visible
    ok, pop = call(SPLIT, typ, [c, OUT, OUT + 1, OUT + 2]); assert pop == 16
    return ok & 0xFF, tuple(mu.mem_read(OUT, 3))

def compose(typ, r, g, b):
    v, pop = call(COMPOSE, typ, [r, g, b]); assert pop == 12
    return v

def spec_split(typ, c):
    if typ in (4, 6): return 1, ((c >> 10 & 31) * 33 >> 2 & 255, (c >> 5 & 31) * 33 >> 2 & 255, (c & 31) * 33 >> 2)
    if typ == 5: return 1, ((c >> 11 & 31) * 33 >> 2 & 255, (c >> 5 & 63) * 65 >> 4 & 255, (c & 31) * 33 >> 2)
    if typ == 7: return 1, ((c >> 8 & 15) * 17, (c >> 4 & 15) * 17, (c & 15) * 17)
    if typ in (8, 9): return 1, (c >> 16 & 255, c >> 8 & 255, c & 255)
    return 0, (0xAA, 0xBB, 0xCC)

def spec_compose(typ, r, g, b):
    r, g, b = r & 255, g & 255, b & 255
    if typ in (4, 6): return 0x8000 | (r & 0xF8) << 7 | (g & 0xF8) << 2 | b >> 3
    if typ == 5: return (r & 0xF8) << 8 | (g & 0xFC) << 3 | b >> 3
    if typ == 7: return 0xF000 | (r & 0xF0) << 4 | (g & 0xF0) | b >> 4
    if typ in (8, 9): return 0xFF000000 | r << 16 | g << 8 | b
    return 0

bad = 0
for typ in (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10):
    for c in list(range(0, 0x10000, 7)) + [0xFFFF, 0x12345678, 0xFFFFFFFF, 0x80FF00FF]:
        bad += split(typ, c) != spec_split(typ, c)
    for r, g, b in [(0, 0, 0), (255, 255, 255), (0x17, 0x88, 0xF9), (0x107, 0x2FF, 0x380)] + \
                   [(i, 255 - i, i * 7) for i in range(0, 256, 5)]:
        bad += compose(typ, r, g, b) != spec_compose(typ, r, g, b)
print('split/compose mismatches vs spec:', bad)

def palette(typ, fg, bg):
    """0x0089838C: split(fg & 0xFFFF), split(bg & 0xFFFF), entries 8..119 composed, 0..7 = bg, 120..127 = fg."""
    _, F = split(typ, fg & 0xFFFF); _, B = split(typ, bg & 0xFFFF)
    pal = [bg & 0xFFFFFFFF] * 8
    for k in range(15):
        ch = [(B[i] + int((F[i] - B[i]) * k / 16)) & 255 for i in range(3)]   # IDIV truncates toward zero
        pal += [compose(typ, *ch)] * 8
    assert len(pal) == 128                                     # k = 14 fills 120..127, then fg overwrites them
    return pal[:120] + [fg & 0xFFFFFFFF] * 8

for typ, fg, bg in [(7, 0xFFFF, 0xF000), (7, 0xF000, 0xFFFF), (7, 0xF84C, 0x0000),
                    (9, 0xFFFFFFFF, 0xFF000000), (9, 0x00FF0000, 0x00000000), (9, 0xFF123456, 0xFF000000),
                    (0, 0xFFFF, 0)]:
    p = palette(typ, fg, bg)
    print(f'type {typ} fg {fg:08X} bg {bg:08X}: ' + ' '.join(f'{p[i]:08X}' for i in (0, 8, 16, 24, 72, 112, 120)))
sys.exit(1 if bad else 0)
