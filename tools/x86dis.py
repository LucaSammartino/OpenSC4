"""Disassemble a range of the SC4 Windows exe. Usage: python tools/x86dis.py <exe> <va-hex> <length>"""
import sys
import capstone
import pefile

pe = pefile.PE(sys.argv[1])
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
va = int(sys.argv[2], 16)
n = int(sys.argv[3], 0)
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
for ins in md.disasm(img[va - base:va - base + n], va):
    print(f"{ins.address:08x}  {ins.mnemonic:6s} {ins.op_str}")
