"""Read-only reader for SimCity 4 DBPF packages (.dat/.sc4) and Exemplar/Cohort records.

Supports DBPF 1.0/1.1 with index version 7.0 (20-byte entries) and 7.1 (24-byte entries),
the DIR record (0xE86B1EEF) for compressed entries, QFS/RefPack decompression and the
binary Exemplar/Cohort format ('EQZB1###' / 'CQZB1###'). Never writes to the package.

CLI:
  python tools/dbpf.py list <file.dat> [type-hex]
  python tools/dbpf.py exemplar <file.dat> <type-hex> <group-hex> <instance-hex>
  python tools/dbpf.py find <instance-hex> <file.dat> [file2.dat ...]
"""
import struct
import sys

TYPE_DIR = 0xE86B1EEF
TYPE_EXEMPLAR = 0x6534284A
TYPE_COHORT = 0x05342861


def qfs_decompress(data):
    """Decompress a QFS/RefPack stream (with the 4-byte DBPF compressed-size prefix)."""
    pos = 0
    if len(data) >= 6 and data[4:6] == b"\x10\xfb":
        pos = 4
    if data[pos + 1] != 0xFB:
        raise ValueError("not a QFS stream")
    flags = data[pos]
    pos += 2
    size = (data[pos] << 16) | (data[pos + 1] << 8) | data[pos + 2]
    pos += 3
    if flags & 0x01:  # extra compressed-size field
        pos += 3
    out = bytearray()
    n = len(data)
    while pos < n:
        c0 = data[pos]
        if c0 < 0x80:  # 2-byte op
            c1 = data[pos + 1]
            plain = c0 & 3
            count = ((c0 >> 2) & 7) + 3
            offset = ((c0 & 0x60) << 3) + c1 + 1
            pos += 2
        elif c0 < 0xC0:  # 3-byte op
            c1, c2 = data[pos + 1], data[pos + 2]
            plain = (c1 >> 6) & 3
            count = (c0 & 0x3F) + 4
            offset = ((c1 & 0x3F) << 8) + c2 + 1
            pos += 3
        elif c0 < 0xE0:  # 4-byte op
            c1, c2, c3 = data[pos + 1], data[pos + 2], data[pos + 3]
            plain = c0 & 3
            count = ((c0 & 0x0C) << 6) + c3 + 5
            offset = ((c0 & 0x10) << 12) + (c1 << 8) + c2 + 1
            pos += 4
        elif c0 < 0xFC:  # literal run
            plain = ((c0 & 0x1F) << 2) + 4
            out += data[pos + 1:pos + 1 + plain]
            pos += 1 + plain
            continue
        else:  # end marker with 0..3 trailing literals
            plain = c0 & 3
            out += data[pos + 1:pos + 1 + plain]
            break
        out += data[pos:pos + plain]
        pos += plain
        start = len(out) - offset
        for i in range(count):  # byte-by-byte: copies may overlap
            out.append(out[start + i])
    if len(out) != size:
        raise ValueError(f"QFS size mismatch: {len(out)} != {size}")
    return bytes(out)


class DBPF:
    def __init__(self, path):
        self.path = path
        with open(path, "rb") as f:
            self.data = f.read()
        d = self.data
        if d[:4] != b"DBPF":
            raise ValueError(f"{path}: not a DBPF file")
        self.major, self.minor = struct.unpack_from("<II", d, 4)
        idx_major, count, idx_off, idx_size = struct.unpack_from("<IIII", d, 0x20)
        idx_minor = struct.unpack_from("<I", d, 0x3C)[0]
        esize = 24 if (idx_major == 7 and idx_minor >= 1) else 20
        if count and idx_size // count in (20, 24):
            esize = idx_size // count
        self.entries = []  # (type, group, instance, offset, size)
        for i in range(count):
            t, g, inst = struct.unpack_from("<III", d, idx_off + i * esize)
            off, size = struct.unpack_from("<II", d, idx_off + i * esize + esize - 8)
            self.entries.append((t, g, inst, off, size))
        self.compressed = set()
        for t, g, inst, off, size in self.entries:
            if t == TYPE_DIR:
                rec = 16 if esize == 20 else 20
                for j in range(size // rec):
                    ct, cg, ci = struct.unpack_from("<III", d, off + j * rec)
                    self.compressed.add((ct, cg, ci))

    def find(self, t=None, g=None, i=None):
        return [e for e in self.entries
                if (t is None or e[0] == t) and (g is None or e[1] == g) and (i is None or e[2] == i)]

    def read(self, entry):
        t, g, i, off, size = entry
        raw = self.data[off:off + size]
        if (t, g, i) in self.compressed:
            return qfs_decompress(raw)
        return raw

    def get(self, t, g, i):
        hits = self.find(t, g, i)
        return self.read(hits[-1]) if hits else None


# Exemplar property value types
_PTYPES = {
    0x0100: ("B", 1, "u8"), 0x0200: ("H", 2, "u16"), 0x0300: ("I", 4, "u32"),
    0x0700: ("i", 4, "s32"), 0x0800: ("q", 8, "s64"), 0x0900: ("f", 4, "f32"),
    0x0B00: ("?", 1, "bool"), 0x0C00: (None, 1, "string"),
}


def parse_exemplar(buf):
    """Return (parent_tgi, {prop_id: (type_name, values)}). Values is a list (strings: a str)."""
    magic = buf[:8]
    if magic[1:4] != b"QZB" or magic[4:5] != b"1":
        if magic[:4] in (b"EQZT", b"CQZT"):
            raise NotImplementedError("text exemplars are not supported")
        raise ValueError(f"not a binary exemplar: {magic!r}")
    parent = struct.unpack_from("<III", buf, 8)
    count = struct.unpack_from("<I", buf, 20)[0]
    pos = 24
    props = {}
    for _ in range(count):
        pid, ptype, key = struct.unpack_from("<IHH", buf, pos)
        pos += 8
        fmt, width, name = _PTYPES[ptype]
        if key == 0x80:
            pos += 1  # unused byte
            n = struct.unpack_from("<I", buf, pos)[0]
            pos += 4
        else:
            pos += 1
            n = 1
        if fmt is None:
            val = buf[pos:pos + n].decode("latin-1")
        else:
            val = list(struct.unpack_from("<" + fmt * n, buf, pos))
        pos += width * n
        props[pid] = (name, val)
    return parent, props


def _fmt_val(name, val):
    if name == "string":
        return repr(val)
    if name == "f32":
        return "[" + ", ".join(f"{v:g}" for v in val) + "]"
    if name in ("u32", "s32", "u16", "u8"):
        return "[" + ", ".join(f"{v} (0x{v & 0xFFFFFFFF:X})" for v in val) + "]"
    return str(val)


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 1
    cmd = argv[1]
    if cmd == "list":
        p = DBPF(argv[2])
        t = int(argv[3], 16) if len(argv) > 3 else None
        for e in p.find(t):
            c = "C" if e[:3] in p.compressed else " "
            print(f"{e[0]:08X} {e[1]:08X} {e[2]:08X} {e[4]:8d} {c}")
    elif cmd == "exemplar":
        p = DBPF(argv[2])
        t, g, i = (int(x, 16) for x in argv[3:6])
        buf = p.get(t, g, i)
        if buf is None:
            print("not found")
            return 1
        parent, props = parse_exemplar(buf)
        print("parent cohort: " + " ".join(f"{x:08X}" for x in parent))
        for pid, (name, val) in props.items():
            print(f"0x{pid:08X} {name:6s} {_fmt_val(name, val)}")
    elif cmd == "find":
        inst = int(argv[2], 16)
        for path in argv[3:]:
            try:
                pkg = DBPF(path)
            except ValueError:
                continue  # not a DBPF package
            for e in pkg.find(i=inst):
                print(f"{path}: {e[0]:08X} {e[1]:08X} {e[2]:08X} size={e[4]}")
    else:
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
