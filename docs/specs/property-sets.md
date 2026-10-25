# GZ property sets and their serializers (behaviour spec)

Scope: the owners `cGZPropertySetT` (14 functions, 791 bytes), `cGZPropertySetSerializerT` (1, 206),
`cGZPropertySetSerializerBinaryT` (1, 479), `cGZPropertySetSerializerXmlT` (14, 2,592) and
`cPropertySetUint32XmlReader` (9, 1,222) as tools/coverage.py assigns them. A **property set** maps a u32 key to a
typed value; the Hit music system stores its track definitions, register sets and channel definitions in them
(hit-music.md §4.2–§4.3). The serializers read a set from a record (binary or XML) and write it as XML.

Related: hit-music.md §4 (the users; "the standard binary property-set serializer" there is §3 here),
sound-framework.md §3.2 (`CRC24`), foundations.md §2.2 (primitive encodings), output-streams.md §1.3 / §5 (stream
slots), persistence.md §3 (the serial record).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the five owners was read in the decompile; the vtables `0x00AD27D4`,
`0x00AD442C`, `0x00AD43FC`, `0x00AD22C4`, the XML serializer's constructor (`0x009406CB`), its start and end
handlers and the string literals they use were read in the disassembly. Written 2026-10-24 (re-persistgraph7).

## 1. Values *[compat]*

A value is a small polymorphic object ("Any" value). The property set uses five of its slots: 0 destroy in place
(argument 0), 1 assign from another value (the property set's `Value` uses it), 3 copy-construct into given
storage, 4 the value's type ID, 5 its size in bytes. The types the serializers know:

| Type ID | Value | Vtable |
|---|---|---|
| `0x0B8BEA18` | string (`AnyString`) | |
| `0x0C264712` | s32 (`AnySint32`) | `0x00AD4468` (copy-construct `0x00940DE8`) |
| `0xABC78708` | f32 (`AnyFloat32`) | `0x00AD2BA4` |
| `0xCBA908E1` | bool (`AnyBoolean`) | `0x00AD4444` |
| `0xEB61E4F7` | u32 (`AnyUint32`) | `0x00AD25FC` |

## 2. The property set (`cGZPropertySetT`, IID `0xEBB10CDD`) *[compat]*

The set holds an ordered map (u32 key → a byte block) at `+0x08` (count `+0x0C`) and an iteration cursor at
`+0x14`. Each block holds one value constructed in it (slot 3), so the stored values are copies.

| Slot | Address | Behaviour |
|---|---|---|
| 0 `QueryInterface` | `0x00939254` | IIDs 1 and `0xEBB10CDD` |
| 2 `Release` | `0x00939279` | a count below 2 destroys the set (slot 11) and returns 0; else decrement |
| 3 `SetValue(key&, value&)` | `0x009393D4` | absent key: `InsertNewValue`; present: the old value is destroyed in place, the block resized to the new value's size (new bytes 0) and the value copied in; always true |
| 4 `ClearValue(key&)` | `0x0093934A` | present: destroy the value, erase the entry; returns whether it was present |
| 5 `ClearAll` | `0x00939040` | destroy every value, then empty the map |
| 6 `FirstKeyAndValue(&key, &value)` | `0x00939290` | either pointer null → false (cursor untouched); cursor := first entry; empty → false; else the key and a borrowed pointer to the stored value |
| 7 `NextKeyAndValue(&key, &value)` | `0x009392C7` | the cursor advances first (also with null pointers; never test-guarded); null pointers → false; at the end → false; else as slot 6 |
| 8 `Value(key&, out&)` | `0x00939306` | absent → false; else `out` assigns from the stored value (its slot 1) and that result is returned |
| 9 `HasValue(key&)` | `0x0093932E` | present |
| 10 | `0x009CEDE1` (shared) | the entry count |
| 11 | `0x00939607` | destructor (body `0x009393BD`: `ClearAll`, then the map) |

- `InsertNewValue(key, value)` (`0x00939431`): a block of the value's size is inserted under the key and the value
  copied into it; it returns the new entry, or null when the key was already present (nothing changes).
- Constructor (`0x00939237`): count 0, empty map, cursor null.
- The map assignment (`0x00939DA1`, used when a holder copies its set, caller `0x0093A011`) copies the blocks
  **byte for byte**, without the values' copy method *[quirk]*: a value that owns memory (a string) ends up shared
  by both sets. *[choice]* FreeCity4 copies values with their copy method.

## 3. Reading a record (`cGZPropertySetSerializerT::Read(stream, set)`, `0x009408A5`) *[compat]*

The entry point the Hit resources call (`0x00940E51`, `0x00941670`). It asks the stream for the serial record
interface (`0xC56B8F08`), reads the first u32, and rewinds the record to position 0 (record slot 55); any failure →
false. A first u32 of `0x2BEA4862` or `0xCBE750E0` selects the binary serializer (§4, a 12-byte object, vtable
`0x00AD40F8`), anything else the XML serializer (§5, `0x54` bytes). The serializer's `Read(stream, set)` result is
returned. Allocation failures are not checked.

## 4. The binary form (`cGZPropertySetSerializerBinaryT::Read`, `0x009409E0`) *[compat]*

```
u32 magic = 0x2BEA4862
u16 version            (stored in the serializer; must be 0 or 1)
u32 count
count × { u32 type, u32 key, value }
```

| Type | Value |
|---|---|
| `0x0B8BEA18` | `GZStr` (foundations.md §2.3) |
| `0xABC78708` | f32 |
| `0xCBA908E1` | u8, ≠ 0 is true |
| `0xEB61E4F7` | u32 |
| other | nothing is read and nothing is stored; the loop goes on with the next entry *[quirk: the stream is then misaligned]* |

Each value is stored with the set's `SetValue`. A wrong magic, a version ≥ 2 or any failed read returns false; the
entries read before stay in the set. A record whose first word is `0xCBE750E0` reaches this reader through §3 and
always fails the magic test *[quirk]*.

## 5. The XML form (`cGZPropertySetSerializerXmlT`) *[compat]*

Identity: `QueryInterface` (`0x00940722`, thunk `0x00940763`): IID 1 and `0xEBE701F3` → the serializer;
`0x4B6B9C9B` → the XML handler interface at `+4`. `Release` (`0x0094070E`, thunk `0x0094076B`): when the count
(`+8`) drops below 1 the serializer is destroyed (slot 5; deleting destructor `0x009407AF`, body `0x009407CB`
destroying the three strings). AddRef thunk of the handler interface: `0x00935EAD`. The constructor (`0x009406CB`)
zeroes the count, the end-element counter (`+0x0C`) and the "root seen" flag (`+0x10`), and makes the key text
(`+0x14`), type text (`+0x28`) and value text (`+0x3C`) strings; the target set is `+0x50`.

### 5.1 Writing (`Write(stream, set)`, `0x0094286B`)

Output through the stream's `SetVoid` (slot 17):

```
<?xml version="1.0"?>\n
<cGZPropertySetUint32>\n
<AnyUint32 key="0x0000abcd" type="0xeb61e4f7">123</AnyUint32>\n        one line per entry, in key order
</cGZPropertySetUint32>\n
```

- The root element name is `cGZPropertySetUint32` (`GetPropertySetTypeString`, `0x00942D50`). Each entry's element
  is `AnyString`, `AnySint32`, `AnyFloat32`, `AnyBoolean` or `AnyUint32` by the value's type; `key` and `type` are
  `0x%08x`. Values: strings raw (no XML escaping *[quirk]*); s32 and u32 with `%d` (a u32 at or above 2³¹ is written
  negative *[quirk]*); f32 with `%f`; bool as `true` / `false`.
- An empty set writes nothing and returns false (the first `FirstKeyAndValue` fails) *[quirk]*. A value of any other
  type stops the write with false after the entries already written. Any failed write returns false.

### 5.2 Reading (`Read(stream, set)`, `0x00941679`)

Creates an XML parser (class `0x6B6B01D1`, IID `0x6B6B01CD`) and gives it the handler interface (parser slot 8);
asks the stream for the serial record to get the record's size; reads that many bytes with `GetVoid`; sets the
target set and parses the bytes as one final chunk (parser slot 3). The parse result is returned; any earlier
failure returns false.

The handler (vtable `0x00AD43FC`):

- **Start element** (`0x00942D87`): the first element must be named exactly `cGZPropertySetUint32`, or the handler
  fails; it sets the "root seen" flag. Later elements: the attribute `key` is copied to the key text and `type` to
  the type text (other attributes are ignored); the element's name is not checked. The value text is emptied.
- **Character data** (`0x00940773`): a chunk made only of tab, LF, CR and space is ignored; any other chunk is
  appended whole to the value text.
- **End element** (`0x00940C5B`): the counter is decremented and the element is skipped only when it reaches 0;
  it starts at 0 and nothing increments it, so every end element is processed *[quirk]*. key :=
  `RZAtoUint32HexA(key text)`, type := `RZAtoUint32HexA(type text)`; by type: string → the value text;
  s32 / u32 → `RZConvertToUint32A(value text)`; f32 → `RZAtofA`; bool → true only when the value text is
  exactly `true` (a case-sensitive `CompareTo`, rz-strings.md §4); other → the handler fails. The value is stored
  with `SetValue`.
- Consequences *[quirk]*: the root's own end element stores the last entry a second time (same key, type and text,
  so no change); a document with no entries fails at the root's end (type 0).

## 6. The Hit XML reader (`cPropertySetUint32XmlReader`, class `0xABA06EF4`) *[compat]*

An XML handler that fills a property set (`+0x0C`, held) from elements with `Name` and `Value` attributes
(hit-music.md §1 registers the class).

- `QueryInterface` (`0x00935E48`, thunk `0x00935EA5`): IIDs 1 and `0xCBA0711A` → the interface at `+4`;
  `0x4B6B9C9B` (XML handler) → the object. `Release` (`0x00935E89`, thunk `0x00935EB5`): a count (`+8`) of exactly
  1 destroys the object (slot 12; deleting destructor `0x00935EDE`, body `0x00935EFA`: the text string, the
  attribute map, the set released); otherwise decrement (a count of 0 goes negative).
- **Start element** (`0x00943809`): empties the character text (`+0x1C`); every attribute pair is stored in the
  attribute map (`+0x10`, name → value; the scan of `Name` / `Value` here has no further effect).
- **Character data** (`0x00940868`): the chunk is appended as a C string; the length argument is not used
  *[quirk: the chunk must be NUL-terminated]*.
- **End element** (`0x009425A2`): value := the character text, name := ""; then the attribute map's `Name` entry, if
  any, replaces the name and its `Value` entry replaces the value. By element name:

  | Element | Stored (key = `CRC24(name) \| 0xFF000000`) |
  |---|---|
  | `PropertyUint32String` | when name or value is non-empty: the value as a string |
  | `PropertyUint32Uint32` | the value must be non-empty and start with a decimal digit, else the handler **fails**; when name or value is non-empty: `RZConvertToUint32A(value)` as a u32 |
  | `PropertyUint32Float32` | the same digit test; the value through a variant's `GetValFloat32` as an f32 |
  | `PropertyStringString`, any other | nothing |

  The attribute map is cleared after every end element that does not fail.

## 7. Function inventory

`cGZPropertySetT`: `0x00939040`, `0x00939237`, `0x00939254`, `0x00939279`, `0x00939290`, `0x009392C7`,
`0x00939306`, `0x0093932E`, `0x0093934A`, `0x009393BD`, `0x009393D4`, `0x00939431`, `0x00939607`, `0x00939DA1`.
`cGZPropertySetSerializerT`: `0x009408A5`. `cGZPropertySetSerializerBinaryT`: `0x009409E0`.
`cGZPropertySetSerializerXmlT`: `0x00935EAD`, `0x0094070E`, `0x00940722`, `0x00940763`, `0x0094076B`,
`0x00940773`, `0x009407AF`, `0x009407CB`, `0x00940C5B`, `0x00940DE8`, `0x00941679`, `0x0094286B`, `0x00942D50`,
`0x00942D87`. `cPropertySetUint32XmlReader`: `0x00935E48`, `0x00935E89`, `0x00935EA5`, `0x00935EB5`, `0x00935EDE`,
`0x00935EFA`, `0x00940868`, `0x009425A2`, `0x00943809`.
