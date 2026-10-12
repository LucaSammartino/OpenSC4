# Object layouts that original code may read directly

Interface facts (see [README.md](README.md)). They give field offsets of game objects whose memory the original code,
or code written in the same style, may read without going through a virtual call. A replacement object of one of
these kinds should use the same layout. If it doesn't, any direct read gets wrong data. The facts were read from the
target executable's constructor and accessor bodies; no logic is recorded here.

## Standard message (`cIGZMessage2Standard`, original class `cRZMessage2Standard`, 0x2C bytes)

Vtable `0x00ACEC00`. It has 38 slots: slot 37 is the scalar deleting destructor, and slots 0–36 follow the
compiled order in foundations §5.1.

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | vtable |
| +0x04 | 4 | reference count. Release destroys the object when the count is ≤ 1, otherwise it decrements. |
| +0x08 | 1 | flags byte (bits below), followed by 3 bytes of padding |
| +0x0C | 4 | type |
| +0x10 | 4 | data1 / void1 |
| +0x14 | 4 | data2 / void2 |
| +0x18 | 4 | data3 / void3 |
| +0x1C | 4 | data4 / void4 |
| +0x20 | 4 | string (`cIGZString*`, counted reference) |
| +0x24 | 4 | IGZUnknown (counted reference) |
| +0x28 | 4 | extra (`cIGZSerializable*`, counted reference) |

Flags byte at +0x08:

| Bit | Mask | Meaning | Set by | Cleared by |
|---|---|---|---|---|
| 0 | 0x01 | has type | SetType with a non-zero type | SetType(0) |
| 1 | 0x02 | has data1 | SetData1 or SetVoid1 (any value, even 0) | never |
| 2 | 0x04 | has data2 | SetData2 or SetVoid2 | never |
| 3 | 0x08 | has data3 | SetData3 or SetVoid3 | never |
| 4 | 0x10 | has data4 | SetData4 or SetVoid4 | never |
| 5 | 0x20 | has extra | SetExtra(non-null) | SetExtra(null) |
| 6 | 0x40 | has string | SetString(non-null) | SetString(null) |
| 7 | 0x80 | has IGZUnknown | SetIGZUnknown(non-null) | SetIGZUnknown(null) |

The pointer setters (string, IGZUnknown, extra) AddRef the new value and Release the old one. A new object is
all zeros apart from the vtable.
