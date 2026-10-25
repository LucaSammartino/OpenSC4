# GZ object-graph persistence: binary reader, binary writer, surveyor (behaviour spec)

Scope: the owners `cGZPersistBinaryReader` (34 functions, 2,136 bytes), `cGZPersistBinaryReader::cResourceInfo`
(3, 129), `cGZPersistBinaryWriter` (24, 1,507) and `cGZPersistSurveyorWriter` (30, 1,530) as tools/coverage.py
assigns them. These are the framework's serializers for a **graph of persistent objects**: a root object, the
objects it refers to inside the same resource, and the other resources it depends on. The text writer of the same
family, `cGZPersistASCIIWriter`, is in sc-resources.md §8; it shares the surveyor and the graph model described
here.

Related: sc-resources.md §8 (the ASCII writer and its line format), persistence.md §4 (persistent resources),
persistence-manager.md (the resource manager slots used on load), app-directors.md §6 (the resource-manager
director that registers these classes), output-streams.md §1.3 and §5 (the stream slots), foundations.md §2.2
(primitive encodings).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function of the four owners was read in the decompile; the vtables `0x00ADA340`,
`0x00ADA210` and `0x00AD9FF0`, the reader's `ReadRoot`, the writer's `WriteRoot` and the factories of the director
were read in the disassembly. Written 2026-10-24 (re-persistgraph7).

## 1. Classes and reachability *[compat]*

| Class | Class ID (director factory) | Object | Vtable | Interfaces |
|---|---|---|---|---|
| surveyor writer | `0xBB194C78` (`0x00975113` → constructor `0x0098158E`) | `0x40` bytes | `0x00AD9FF0` | 1, `0x1B0AAB0E` (writer), `0x9B194C66` (surveyor) |
| binary reader | `0x5B101E6B` (`0x00975129` → `0x00983D5B`) | `0x30` | `0x00ADA340` | 1, `0xFB0C4344` (reader) |
| binary writer | `0xBB100367` (`0x0097513F` → `0x009832DF`) | `0x30` | `0x00ADA210` | as the ASCII writer: 1, `0x1B0AAB0E` |
| ASCII writer | `0x3B0AB2B4` (`0x00975155` → `0x00983314`) | `0x30` | `0x00ADA288` | sc-resources.md §8 |

The class IDs of the binary reader, the binary writer and the ASCII writer occur in the executable only in the
director's registration table: no shipped code creates them, so they are reached only through COM (plugins). The
surveyor is created by the two writers' `WriteRoot`.

Each class keeps a plain reference count at `+4` (`AddRef` is the shared `0x00968B60`); `Release` destroys the
object through its destructor slot (reader 28, writer 29, surveyor 37) when the count reaches 0.

## 2. The graph model

- An **object** is a persistent serializable referent (IID `0xBB11A53E`). Its slot 4 returns its type information
  (name string, class ID, version), slot 5 writes its body to a writer, slot 3 reads its body from a reader, and
  slot 6 returns the resource that owns it.
- The **root** is the object passed to `WriteRoot` / `ReadRoot`. An object referenced through `WriteObjectRef`
  whose owning resource is the root belongs to the graph; one owned by another resource makes that resource a
  **dependency** instead (only the whole resource is recorded, never the object inside it).
- Objects written with `WriteObject` are **inline**: they are written in place and are not part of the object list.

## 3. The surveyor (`cGZPersistSurveyorWriter`) *[compat]*

A writer that writes nothing and only collects the graph.

| Offset | Field |
|---|---|
| `+0x08` | the output stream (held) |
| `+0x0C` | the root (held) |
| `+0x10` / `+0x1C` | the object list and its index map (object → index) |
| `+0x28` / `+0x34` | the dependency list and its index map (resource → index) |

| Slot | Address | Behaviour |
|---|---|---|
| 3 `WriteRoot(root, stream)` | `0x00981943` | `Clear`; root := `root`; `AddObjectRef(root)`; then, for i = 0 while i < the (growing) object count, object i's slot 5 with the surveyor as writer; finally stream := `stream` |
| 4–11, 22, 23 | `0x0096E960` (shared) | scalar and string writes: nothing |
| 12–21 | `0x0093E47C` (shared) | array writes: nothing |
| 24 `WriteObject(name, obj)` | `0x009811B5` | a non-null object writes its body into the surveyor (its slot 5), so inline objects are walked too |
| 26 `WriteObjectRef(name, ref)` | `0x0098199B` | null: nothing; owner := `ref`'s slot 6; owner = root → `AddObjectRef(ref)`, else `AddDepResource(owner)` |
| 27 `WriteTypeInfo` | `0x00949B73` (shared) | nothing |
| 28 `OutputStream` | `0x009812D4` | the stream, or a shared `cRZNullOStream` made on first use (freed at exit) |
| 29 `Clear` | `0x0098178F` | root := null; every listed object and dependency released; both lists and maps emptied |
| 30 | `0x009CEDE1` (shared) | the root |
| 31 / 32 / 33 | `0x00981205` / `0x0098131E` / `0x009817C7` | object count; object i (no range check); index of an object (−1 when absent) |
| 34 / 35 / 36 | `0x0098120F` / `0x0098132B` / `0x009817F2` | the same for dependencies |
| 37 | `0x00981773` | destructor (body `0x009815CA`) |

`AddObjectRef(obj)` (`0x0098188D`) and `AddDepResource(res)` (`0x009818E8`): when not yet in the map, append it
(held) and map it to its index; a repeat does nothing. Slot 25 is the ASCII writer's shared adaptor (`0x009811C8`,
sc-resources.md §8). The walk is breadth-first in discovery order: object 0 is the root, then objects in the order
their first reference was met.

The remaining functions are container code: `0x00981219`, `0x00981246` (free a map subtree), `0x00981338`,
`0x00981361` (clear a map), `0x0098138A`, `0x009813A7` (copy a held-pointer range), `0x009813D6`, `0x009813ED`
(destroy a map), `0x0098160D`, `0x009816C0` (grow and insert), `0x0098181D`, `0x00981855` (append to the object
list or the dependency list; the binary reader uses them too).

## 4. The binary format *[compat]*

All values are little-endian stream primitives (foundations.md §2.2); field names are not written.

```
u32 depCount
depCount × { u32 group, u32 instance, u32 type }      the dependency's resource key
u32 objCount                                          ≥ 1 when written; object 0 is the root
objCount × u32 classID                                each object's class ID from its type information
objCount × object body                                each object's slot 5, in list order
```

Inside a body the fields are written by the object's own code through the writer:

| Writer slot (reader slot) | Encoding |
|---|---|
| 4 bool (4) | u8 (writer: the byte as given; reader: ≠ 0) |
| 5, 6 s8 / u8 (5, 6) | u8 |
| 7, 8 s16 / u16 (7, 8) | u16 |
| 9, 10 s32 / u32 (9, 10) | u32 |
| 11, 12 f32 / f64 (11, 12) | f32 / f64 |
| 13 bool array (13) | count × u8 (reader: each ≠ 0) |
| 14, 15 s8 / u8 arrays (14, 15) | the bytes raw (`SetVoid` / `GetVoid`) |
| 16–21 arrays (16–21) | count × the element |
| 22 C string | `SetRZCharStr` (output-streams.md §1.5) |
| 23 string (22) | `SetGZStr` / `GetGZStr` |
| — (23 `SkipBytes(name, n)`) | the reader skips n bytes |
| 24 inline object (24) | u32 class ID (0 = null), then the object's body |
| 26 reference (25) | u8 non-null; if non-null: u8 external, u32 index (into the object list, or into the dependency list when external) |
| 27 type information (26) | `GZStr` name, u32 class ID, u32 version |

Array counts are not written: both sides pass the count themselves.

## 5. The binary writer (`cGZPersistBinaryWriter`) *[compat]*

`WriteRoot(root, stream)` (`0x009833CC`): creates a surveyor (class `0xBB194C78`, IID `0x9B194C66`; the result is
not checked), pushes it on the writer's stack of surveyors, surveys the graph with it (§3 slot 3), then writes §4:
the dependency keys (each taken from the dependency's key: group, instance, type), the object count and class IDs,
and each object's body with the binary writer itself. It then flushes the stream (slot 3), calls slot 10 of each
dependency in order, and pops and releases the surveyor.

- Every field write goes to the top surveyor's stream (slot 28 `OutputStream`, shared with the ASCII writer,
  `0x00982F1C`; it does not test for an empty stack). The scalar float writes use the stream's `SetFloat32` /
  `SetFloat64` (`0x0040CEF0`, `0x004A7560`).
- `WriteObject(name, obj)` (`0x00981FAA`): null → u32 0; else u32 the class ID, then `obj`'s slot 5.
- `WriteObjectRef(name, ref)` (`0x00982E70`, slot 26): u8 (ref ≠ null); for a non-null ref, external := its owner
  (slot 6) is not the surveyor's root; u8 external; u32 the index from the surveyor (the object's, or the owner
  dependency's). A reference to an object the survey did not reach writes index −1 *[inferred: the surveyor's map
  miss]*.
- `WriteTypeInfo(info)` (`0x00982006`): name (`SetGZStr`), class ID, version.
- No write result is checked; there is no rollback.

The destructor body (`0x00983291`) and deleting destructor (`0x009832F8`) release the stack.

## 6. The binary reader (`cGZPersistBinaryReader`) *[compat]*

The reader keeps a stack of `cResourceInfo` records (`0x00982B59` copy, `0x009831C8` constructor, `0x009831F0`
destructor): the dependency list (`+0x00`), the object list (`+0x0C`) and the input stream (`+0x18`, held).

`ReadRoot(root, stream)` (`0x00983A4F`):

1. Push a record for `stream`. Take the resource manager. If the manager does not hold the root's resource key
   (manager slot 10), register the root under it (slot 8) for the duration of the read, so that dependencies that
   refer back to it find it.
2. Read the dependency count; for each, group, instance, type: type 0 → a null dependency; otherwise the manager's
   `GetResource(key, IID 0x9B11BF70)` (slot 3); a failure gives a null dependency. Both cases format a message
   ("Null dependent resource …", "Cannot load dependent resource …") that is not shown or logged.
3. Read the object count. Object 0 is the root itself: its class ID is read and ignored (also when the count is
   0). Each further class ID creates an object through GZCOM (IID `0xBB11A53E`); class 0 or a failed creation gives
   a null entry.
4. Each non-null object, in order, reads its body (its slot 3, with the reader).
5. If the root was registered in step 1, unregister it (manager slot 9). Pop the record.

Field reads (each takes the field name and ignores it):

- Scalars return the value. The value is read into a local that holds the reader's own address beforehand, so a
  failed read returns that address (or its top byte for 8-bit reads) *[quirk]*. *[choice]* FreeCity4 returns 0.
- `ReadObject(name, iid, out)` (`0x00981D0F`): u32 class ID; 0 → `out` := null. Otherwise the object is created
  (IID `0x5B0AAA74`), reads its body, is asked for `iid` into `out`, and is released. A failed creation is not
  checked *[quirk]*; *[choice]* FreeCity4 sets `out` to null.
- `ReadObjectRef(name, iid, out)` (`0x00982DBB`): u8 non-null (0 → `out` := null); u8 external; u32 index;
  `out` := null, then the entry of the top record's object list (or dependency list when external) at that index,
  when non-null, is asked for `iid`. The index is not range-checked *[quirk]*; *[choice]* FreeCity4 treats an
  out-of-range index as null.
- `ReadTypeInfo(info)` (`0x0098320F`): name, class ID, version into `info` (copy `0x00981D78`); returns `info`.
- `InputStream` (slot 27, `0x00982E64`): the top record's stream (no empty-stack test).

The rest is container code: `0x00981AB6` (step a record-stack iterator), `0x009833AB`, `0x009839A3`, `0x009839EA`,
`0x00983A13` (destroy the stack), `0x00983A41` (destructor body), `0x00983D74` (deleting destructor).

## 7. Quirks

- A dependency is recorded as a whole resource: a reference into another resource reads back as that resource.
- A truncated object count (0) still consumes one class ID for the root.
- Failed scalar reads return the reader's address.

## 8. Function inventory

`cGZPersistBinaryReader`: `0x009819C8`, `0x009819F1`, `0x00981A03`, `0x00981A1A`, `0x00981A2D`, `0x00981AB6`,
`0x00981ADA`, `0x00981AFA`, `0x00981B15`, `0x00981B30`, `0x00981B4C`, `0x00981B68`, `0x00981B83`, `0x00981B9E`,
`0x00981BB9`, `0x00981BD5`, `0x00981C0D`, `0x00981C38`, `0x00981C63`, `0x00981C8E`, `0x00981CB9`, `0x00981CE4`,
`0x00981D0F`, `0x00981D78`, `0x00982DBB`, `0x00982E64`, `0x0098320F`, `0x009833AB`, `0x009839A3`, `0x009839EA`,
`0x00983A13`, `0x00983A41`, `0x00983A4F`, `0x00983D74`. `cResourceInfo`: `0x00982B59`, `0x009831C8`, `0x009831F0`.

`cGZPersistBinaryWriter`: `0x00981A52`, `0x00981A69`, `0x00981A7C`, `0x00981D99`, `0x00981DAC`, `0x00981DBF`,
`0x00981DD2`, `0x00981DE5`, `0x00981DF8`, `0x00981E0B`, `0x00981E23`, `0x00981E3D`, `0x00981E70`, `0x00981EA5`,
`0x00981EDA`, `0x00981F0F`, `0x00981F44`, `0x00981F76`, `0x00981FAA`, `0x00982006`, `0x00982E70`, `0x00983291`,
`0x009832F8`, `0x009833CC`.

`cGZPersistSurveyorWriter`: `0x0093E47C`, `0x0098116D`, `0x009811A0`, `0x009811B5`, `0x00981205`, `0x0098120F`,
`0x00981219`, `0x00981246`, `0x009812D4`, `0x0098131E`, `0x0098132B`, `0x00981338`, `0x00981361`, `0x0098138A`,
`0x009813A7`, `0x009813D6`, `0x009813ED`, `0x009815CA`, `0x0098160D`, `0x009816C0`, `0x00981773`, `0x0098178F`,
`0x009817C7`, `0x009817F2`, `0x0098181D`, `0x00981855`, `0x0098188D`, `0x009818E8`, `0x00981943`, `0x0098199B`.
