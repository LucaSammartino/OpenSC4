"""Disassemble a range of the SC4 Windows exe. Usage: python tools/x86dis.py <exe> <va-hex> <length>"""
import sys


def disasm(exe, va, n):
    """(address, mnemonic, operands) for the instructions in [va, va + n) of the exe's mapped image."""
    import capstone
    import pefile

    pe = pefile.PE(exe, fast_load=True)
    base = pe.OPTIONAL_HEADER.ImageBase
    img = pe.get_memory_mapped_image()
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    return [(i.address, i.mnemonic, i.op_str) for i in md.disasm(img[va - base:va - base + n], va)]


if __name__ == "__main__":
    for a, m, o in disasm(sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3], 0)):
        print(f"{a:08x}  {m:6s} {o}")
