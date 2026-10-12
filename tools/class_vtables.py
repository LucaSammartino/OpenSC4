"""Locate a GZCOM class's vtables in the SC4 Windows exe from its CLSID.

GetGZCLSID is compiled as `mov eax, CLSID; ret`. A pointer to it sits in the
class's cIGZSerializable sub-vtable. Constructors/destructors install every
sub-vtable with `mov dword [reg+off], imm32`, so we find the code that installs
the vtable containing GetGZCLSID, then collect every other .rdata vtable
address installed within the same few instructions.

Usage: python tools/class_vtables.py <exe> <clsid-hex>
"""
import re
import struct
import sys

import capstone
import pefile

pe = pefile.PE(sys.argv[1])
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
clsid = int(sys.argv[2], 16)
text = next(s for s in pe.sections if s.Name.startswith(b".text"))
rdata = next(s for s in pe.sections if s.Name.startswith(b".rdata"))
tlo, thi = base + text.VirtualAddress, base + text.VirtualAddress + text.Misc_VirtualSize
rlo, rhi = base + rdata.VirtualAddress, base + rdata.VirtualAddress + rdata.Misc_VirtualSize
tbytes = img[text.VirtualAddress:text.VirtualAddress + text.Misc_VirtualSize]
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)


def u32(va):
    return struct.unpack_from("<I", img, va - base)[0]


def is_code(v):
    return tlo <= v < thi


def imm_refs(value):
    return [tlo + m.start() for m in re.finditer(re.escape(struct.pack("<I", value)), tbytes)]


def installed_vtables(site):
    """Disassemble around an imm32 site and return all .rdata vtable-looking imm32 stores."""
    out = set()
    for ins in md.disasm(img[site - 0x40 - base:site + 0x40 - base], site - 0x40):
        if ins.mnemonic == "mov" and ins.op_str.startswith("dword ptr ["):
            parts = ins.op_str.split(", ")
            try:
                v = int(parts[-1], 16)
            except ValueError:
                continue
            if rlo <= v < rhi and is_code(u32(v)):
                out.add(v)
    return out


getters = [tlo + m.start() for m in re.finditer(re.escape(b"\xb8" + struct.pack("<I", clsid) + b"\xc3"), tbytes)]
print("GetGZCLSID:", " ".join(f"{g:08x}" for g in getters))
vtables, sites = set(), set()
for g in getters:
    for m in re.finditer(re.escape(struct.pack("<I", g)), img):
        slot = base + m.start()
        # the vtable containing this slot starts at the nearest installed address at or below it
        a = slot
        while a > slot - 0x400:
            refs = imm_refs(a)
            if refs:
                sites.update(refs)
                break
            a -= 4
for s in sites:
    vtables |= installed_vtables(s)
print("installed at:", " ".join(f"{s:08x}" for s in sorted(sites)))
for vt in sorted(vtables):
    print(f"\n-- vtable {vt:08x}")
    i = 0
    while is_code(u32(vt + 4 * i)) and (i == 0 or not imm_refs(vt + 4 * i)) and i < 200:
        print(f"   [{i:3d}] +0x{i*4:03x}  {u32(vt + 4 * i):08x}")
        i += 1
