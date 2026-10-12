# Spec: foundations (streams, save file, response curves, messages, random numbers, variants, property holders, regional records, agent order)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and sample saves. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* marks a finding that is consistent with all evidence but not proven; *[quirk]* marks original
behaviour that is surprising and documented so that nobody "fixes" it by accident.

Other specs refer to this one for the meaning of `str`, `ref`, `reskey`, "response curve", "standard message"
and the random number generators, and (§9–§13) for variants, the "property holder record", the regional-city
record, the C runtime `rand()` and the order of agents within a level.

## 1. Two contexts

- **Plug-in mode** (milestones M1–M5). The original game owns the save file, the streams and the message server.
  Our replacement objects receive `cIGZOStream` / `cIGZIStream` pointers in `Write` / `Read` and must produce and
  accept exactly the bytes described here. Read-side quirks (§2.9) are what our `Read` code will observe.
- **Standalone mode** (M6+). FreeCity4 implements the streams, the save container and the message server itself.
  It must write byte-identical data. On the read side, the quirks are *[choice]*: we may treat them as errors.

## 2. Serialization streams

### 2.1 Model

During a save or load, every serializable object reads or writes through a **segment stream** opened on one
record of the save package (§3). The stream implements `cIGZOStream` (write) or `cIGZIStream` (read) and also the
segment interfaces `cISC4DBSegmentOStream` (IID `0x6999A8FB`) / `cISC4DBSegmentIStream` (IID `0x4999A8F5`), which
add resource keys, object references and variants. Code that needs references must obtain the segment interface
with `QueryInterface`. A plain `cIGZIStream` cannot read references.

A stream has a position inside its record and a single **good** flag:

- The flag is set by a successful `Open`.
- Every `Set*`/`Get*`/`Write*`/`Read*`/`Skip` call does nothing and returns false if the flag is already clear.
  Otherwise it performs the operation and then sets the flag to the operation's result. The error state is
  therefore **sticky**: after the first failure every later call fails.
- `GetError()` returns 0 while the flag is set and a non-zero value (1) after a failure.
- `Flush()` does nothing. `SetUserData` / `GetUserData` are not used by the simulation.

### 2.2 Primitive encodings *[compat]*

All multi-byte values are **little-endian** (confirmed on sample saves). There is no alignment and no padding.

| Method (write / read) | Bytes | Encoding |
|---|---|---|
| `SetSint8` / `GetSint8`, `SetUint8` / `GetUint8` | 1 | two's complement / unsigned |
| `SetSint16` / `GetSint16`, `SetUint16` / `GetUint16` | 2 | LE |
| `SetSint32` / `GetSint32`, `SetUint32` / `GetUint32` | 4 | LE |
| `SetSint64` / `GetSint64`, `SetUint64` / `GetUint64` | 8 | LE |
| `SetFloat32` / `GetFloat32` | 4 | IEEE-754 binary32, LE bit pattern, written unchanged |
| `SetFloat64` / `GetFloat64` | 8 | IEEE-754 binary64, LE bit pattern, written unchanged |
| `SetVoid(p, n)` / `GetVoid(p, n)` | n | raw bytes, no length prefix |
| `SetGZStr` / `GetGZStr` | 4 + n | §2.3 |
| `SetRZCharStr` / `GetRZCharStr` | 4 + n | §2.4 |
| `WriteResKey` / `ReadResKey` | 12 | §2.5 |
| `SetGZSerializable` / `GetGZSerializable` | payload | inline object, §2.6 |
| `WriteGZSerializable` / `ReadGZSerializable` | 4 or 8 | object reference, §2.7 |
| `WriteVariant` / `ReadVariant` | 8 + value | §9 |

- **Booleans** have no method of their own. By convention they are written with `SetUint8` as 0 or 1
  *[compat]*. *[choice]* Readers treat any non-zero byte as true.
- Signed and unsigned variants of a width produce identical bytes.
- **Reads are atomic.** A read that needs *n* bytes when fewer remain in the record fails, consumes nothing and
  leaves the destination unchanged. Callers that read into uninitialised variables must initialise them first.

### 2.3 Strings (`SetGZStr` / `GetGZStr`)

**Write** *[compat]*:

```
uint32  header = byteLength          (top byte 0 = 8-bit text)
byte[byteLength]  the string's bytes, no terminator
```

- The bytes are the `cIGZString`'s own bytes, unchanged. The game's internal text encoding is **UTF-8**
  *[inferred]*: its conversion routines treat encoding code 8 (UTF-8) as the native string format. Sample saves
  contain ASCII only (open question 2).
- Writers must keep strings at most **200,000 bytes** and must not put NUL bytes in them *[compat]* (see the read
  quirks below).

**Read**: read the 32-bit header; `encoding = header >> 24`, `count = header & 0xFFFFFF`.

| Case | Behaviour |
|---|---|
| `encoding = 0x00` | read `count` bytes; they are the string |
| `encoding = 0x10` | read `count` UTF-16 code units (2 × `count` bytes, LE) and convert them to UTF-8 |
| `count > 200,000` (either encoding) | the read fails (header consumed, stream enters the error state) |
| `count = 0` | empty string, success |
| any other encoding byte | *[quirk]* legacy format, below |

Quirks of the original reader:

1. *[quirk]* If at least 2 bytes were read and the **second byte is 0**, the data is decoded as UTF-16 even when
   the header said 8-bit. For an 8-bit string this decodes memory beyond the string. Hence the "no NUL" rule.
2. *[quirk]* If fewer than 4 bytes remain for the header, the string becomes the remaining 0–3 bytes and the read
   **succeeds** (the stream stays good).
3. *[quirk]* Legacy format (encoding byte not 0 or 0x10): the reader steps back over the header and appends bytes
   one at a time, as many as the record's total size, ignoring failed reads. It returns success. No writer in
   the shipped game produces this.

*[choice]* In standalone mode, quirks 2 and 3 are read errors, and quirk 1 does not exist (the header decides).

### 2.4 C strings (`SetRZCharStr` / `GetRZCharStr`)

- `SetRZCharStr(s)` writes exactly what `SetGZStr` would write for the string `s` (without its terminator).
- `GetRZCharStr(buf, maxBytes)` reads a string as in §2.3 and copies `min(length + 1, maxBytes)` bytes, including
  the terminating NUL if it fits. If the string is truncated, the buffer is **not** NUL-terminated *[compat]*.
  Its result is the stream's good flag.

### 2.5 Resource keys

`WriteResKey(key)` writes three `uint32`: **type, group, instance**, in that order. `ReadResKey` reads them back.
It fails if any of the three reads fails.

**Persistent-resource references.** Some records refer to a game resource (for example an exemplar) instead of a
saved object. The convention is three `uint32` (type, group, instance) of the resource; a missing resource is
written as three zeros. On load the key is resolved through the resource manager, requesting the interface the
reader needs. A zero key does not resolve. Subsystem specs say where this form is used.

### 2.6 Inline nested objects (`SetGZSerializable` / `GetGZSerializable`)

`SetGZSerializable(obj)` simply calls `obj.Write(stream)` on the same stream. `GetGZSerializable(obj)` calls
`obj.Read(stream)`. There is no length, class ID or marker. The reader must already hold an object of the right
class. The stream's good flag becomes the method's result. **This is not the reference mechanism.** Despite the
similar name, objects shared or owned across records use §2.7.

### 2.7 Object references

A **reference** (`ref` in other specs) is written with `cISC4DBSegmentOStream::WriteGZSerializable(obj)`:

```
uint32  objectID          0 = null reference, nothing follows
uint32  classID           only if objectID ≠ 0: obj->GetGZCLSID()
```

The referenced object itself is **not** written here. Writing a reference registers the object with the save's
**class-object store** (§3.3), which writes each registered object exactly once into the record for its class,
after all other records. The object's data therefore lives in a different record from the reference.

**Object IDs** *[compat]*: any non-zero 32-bit value that is the same for every reference to the same object
during one save and different for different objects of the same class. The original uses the object's memory
address, so the IDs in sample saves look like heap pointers and are unique across all classes. *[choice]*
FreeCity4 numbers objects 1, 2, 3, … in registration order. IDs mean nothing after the save is loaded.

**Reading** (`cISC4DBSegmentIStream::ReadGZSerializable(ppObj, loadNow)`):

1. Read `objectID`. If it is 0, the result is a null pointer and success.
2. Read `classID`. Resolve `(classID, objectID)` in the store's registry:
   - already known → the same object (`AddRef`);
   - `classID` not listed in the save's class index (§3.3) → **failure** (the stream enters the error state);
   - otherwise create a new instance with `GZCOM()->GetClassObject(classID, GZIID_cIGZSerializable, &p)`. If
     creation fails → failure. If it succeeds, register it under `(classID, objectID)`.
3. The new object's `Read` is **not** called here. It is called when the loader reaches the object's entry in its
   class record (§3.5). The shipped game never sets `loadNow`; when set, the object's entry would be located and
   read immediately *[inferred]*.

Usual helper semantics *[compat]*: callers ask for an interface. After a successful reference read the result
is `QueryInterface`d to the requested IID. A null reference gives a null pointer and success. A non-null object
that lacks the interface gives failure, but the stream itself stays good.

*[compat]* **SDK discrepancy.** The shipped `cISC4DBSegmentIStream::ReadGZSerializable` takes **two** arguments
(`cIGZSerializable**`, `bool loadNow`), but the gzcom-dll header declares only the pointer. With MSVC
`__thiscall` the callee removes its arguments from the stack, so calling it through the SDK declaration corrupts
the stack. The plug-in shim must declare the extra `bool` (pass false).

Consequences for every `Read` implementation *[compat]*:

- A referenced object may not have been read yet. It may even be read later in the load, or never if its entry is
  missing. `Read` must only store the pointer. Work that depends on other objects' state belongs in `Init` or a
  later initialisation message.
- Cycles (A refers to B, B refers to A) are allowed and common, for example a department and its line items.

### 2.8 Variants

`WriteVariant` writes a `uint32` **marker** (0, or `0xA9BEE9AC` for an object-valued variant) and then a
`uint32` **disk type tag** (bit 15 set = array) and the value. Arrays are a `uint32` count followed by the
elements, each in its primitive encoding. The disk tags are **not** the in-memory `cIGZVariant` type numbers.
The full format is §9. Variants appear in saves inside property holders (§10).

### 2.9 Error behaviour summary

| Situation | Result |
|---|---|
| read past the end of the record | that call fails, nothing consumed, destination unchanged, stream error (sticky) |
| any call after an error | returns false, does nothing |
| string longer than 200,000 | failure |
| short read of a string header | *[quirk]* success with 0–3 garbage bytes |
| reference to a class not in the save | failure |
| reference whose class cannot be created (for example a missing plugin) | failure |
| `Skip(n)` with fewer than n bytes left | failure. Some bytes may already be consumed *[inferred]* |

### 2.10 Version convention

The framework itself has no version field. By convention every serializable object starts its payload with a
`uint16` version and its `Read` rejects versions it does not know (each subsystem spec lists its own). Records
opened through the game's "auto stream" helpers (the root record in §3.2) also start with a `uint16` version
written by the helper.

## 3. City save file

### 3.1 Container

A `.sc4` city is a **DBPF 1.0** package with index version 7.0 (20-byte index entries). The container format is
the same as the game's `.dat` files (see `tools/dbpf.py` for a reader). Record data may be QFS (RefPack)
compressed. Compressed records are listed in the directory record (type/group `0xE86B1EEF`, instance
`0x286B1F03`).

Compression rule of the original writer *[compat for byte-identical output, otherwise choice]*: a record is
compressed when it is at least 50 and at most 16,000,000 bytes long and compression shrinks it below 80 % of its
size. Readers must accept both compressed and uncompressed records.

### 3.2 Records used by the simulation

| Type | Group | Instance | Contents |
|---|---|---|---|
| `0x499B23FE` | `0x299B2D1B` | 0 | class-object **index** (§3.3) |
| *classID* | `0x299B2D1B` | 0 | **class record**: every saved object of that class (§3.3) |
| `0x49DD6F4C` | `0x49DD6F4E` | 1 | **root record**: `uint16 1` (stream helper version), `uint16 2` (app version), `ref` to the city (IID `0x66BA4DB5`), 12 bytes |
| `0xEA3FF4F5` | `0xEA3FF506` | `0xEA3FF4F5` | app load/save version: a bare `uint32` = **65** (`0x41`), no version prefix |
| `0x0B7D45E0` | `0x0B7D45E1` | `0x0B7D45E2` | module version: `uint16 3`, then four `uint32` (the executable's version parts) *[inferred]* |
| any | `0xAB85E27D` | any | **retained records** (§3.8) |

**This is the CLSID-as-type convention.** A subsystem's data is stored as one entry in the class record whose
**type is the subsystem's class ID**, group `0x299B2D1B`, instance 0. For example the simulator lives in
`0x2990C1E5/0x299B2D1B/0` and the budget simulator in `0xE990BE01/0x299B2D1B/0`. Subsidiary objects, such as
departments, line items and demand nodes, live in their own class records. A subsystem record is **never
located by its key alone**. It is reached through a reference from the city (§3.6). The key only follows from the
class ID.

Other records in a city file (region thumbnails and views, terrain data, city name strings `0x2026960B` in groups
`0x6AAEE931`/`0x6AAEE932`, a record `0x89D9B9EE/0x69D9B9F1/0` written by the city) are outside this spec, except
the **regional-city record** `0xCA027EDB/0xCA027EE1/0`, which is specified in §11.

### 3.3 Class-object store

**Class record** `(classID, 0x299B2D1B, 0)`: a sequence of entries, nothing else:

```
uint32  entrySize        bytes from the start of this field to the end of the payload (12 + payload length)
uint32  crc              §3.4
uint32  objectID
byte[]  payload          exactly what the object's Write produced
```

**Index record** `(0x499B23FE, 0x299B2D1B, 0)`:

```
uint32  version = 1
uint32  classCount
int32   entriesWritten                       total entries in all class records
classCount × { uint32 classID; uint32 objectCount }      ascending classID
```

`objectCount` is the number of distinct objects of that class registered during the save. Sample saves were
checked: the counts match the number of entries in each class record, and every class record parses into whole
entries with no trailing bytes.

### 3.4 Entry checksum *[compat]*

`crc` = CRC-32 with polynomial `0x04C11DB7`, **not reflected** (most significant bit first), initial value
`0xFFFFFFFF`, **no final XOR** (the "CRC-32/MPEG-2" parameter set). It covers the bytes from `objectID` onwards,
`min(entrySize − 8, 250000)` bytes. Larger payloads are only checksummed over their first 250,000 bytes (the
`objectID` included). Check value: CRC of ASCII `"123456789"` = `0x0376E6E7`. All entries of two sample saves
verify under this rule, including entries above 250,000 bytes.

### 3.5 Writing and loading the store

**Saving** (after all ordinary records are written) *[compat]*:

```
while some registered object has not been written:
    for each class C, ascending class ID, with unwritten objects:
        open record (C, 0x299B2D1B, 0) for appending
        for each unwritten object O of C, ascending object ID:
            p := position; write uint32 0, uint32 0, uint32 ID(O)
            ok := O.Write(stream)
            if ok:  patch entrySize and crc at p; continue after the payload
            else:   move back to p (the entry is discarded; later entries overwrite it)
            mark O written
            if the stream is in error: abort the save
        close the record
write the index record (from the start of the record)
```

- Writing an object may register more objects through references. They are written in a later pass and appended
  to their class records. Entries in a class record are therefore sorted by ID within each pass, but not overall.
- When a class is registered for the first time in a save, any existing record with its key is deleted first.
  This only matters when writing into an existing package. The game always writes a fresh file (§3.7).
- *[quirk]* A discarded entry still counts in the index's `objectCount`, and the loader would then miss an entry.
  **Our `Write` must never return false except for a stream error** *[choice]*.
- If no object was ever registered, nothing is written (no index).

**Loading** runs once, right after the package is opened and **before any other record is read**:

```
read the index; version must be 1, otherwise the load fails
for each class C in the index, ascending class ID:
    open (C, 0x299B2D1B, 0); if missing, skip the class
    if objectCount > 9,999,999: the class fails
    repeat objectCount times:
        p := position; read entrySize, crc
        verify crc over min(entrySize − 8, 250000) bytes from p + 8   (position unchanged)
        on mismatch: the class fails, stop reading it
        read objectID; resolve (C, objectID) as in §2.7 (create if new)
        if resolution failed: move to p + entrySize and continue
        call O.Read(stream)          (its result is ignored)
        if position ≠ p + entrySize: move to p + entrySize
    the class fails if the number of objects registered for C ≠ objectCount
```

- **Entry framing protects neighbours.** If `Read` reads too little or too much (within the record), the loader
  re-synchronises at the next entry. But a `Read` that puts the stream into the **error state** makes every later
  entry of that class fail, because the error is sticky. `Read` must not read past its own payload *[compat]*.
- Objects are read in ascending class ID, so a subsystem is usually read **before or after** the objects it refers
  to, never in a guaranteed order (§2.7).
- *[quirk]* The original's overall result reflects only the last class whose record could be opened. A failure in
  an earlier class (for example objects of an uninstalled plugin class) does not fail the load. The game aborts
  the load when this result is false. *[choice]* In standalone mode FreeCity4 reports per-class failures and
  fails the load only if a class that the city references fails.
- The original sanity check on `entrySize` is weak. *[choice]* We reject an entry whose `p + entrySize` exceeds the
  record size, or whose `entrySize` is below 12.

### 3.6 The city and its subsystems

The **city** object (class `0x8990C372`) is stored in the class-object store like any other object. The root
record (§3.2) refers to it. The city's payload (version 12, accepted on read: 8..12) holds the city's own fields
and then **53 references** to its components, in this fixed order *[compat]*. On read each reference is
resolved and `QueryInterface`d to the IID shown. The class IDs are those found in sample saves.

| # | Component | IID requested on read | Class (sample saves) |
|---|---|---|---|
| 1 | history warehouse | `0x69F4A803` | `0x89EFA536` |
| 2 | demolition utility | `0x26DDE2EA` | `0x0990C38A` |
| 3 | dispatch manager | `0x694028EC` | `0x4990BD93` |
| 4 | occupant manager | `0xC6BB8334` | `0x098F964D` |
| 5 | prop manager | `0x28326F3E` | `0xA990BD10` |
| 6 | zone manager | `0x67652433` | `0x298F9B2D` |
| 7 | zone developer | `0xE818ED2B` | `0x498F9B01` |
| 8 | lot manager | `0x681905BD` | `0x498F9B01` (the zone developer object) |
| 9 | lot configuration manager | `0xC8EDBEC0` | `0x299062C0` |
| 10 | prop developer | `0xE9A1F4B6` | `0x89C48F47` |
| 11 | network lot manager | `0x89A5BA3F` | `0xE9A6CD3B` |
| 12 | landfill developer | `0x8A09D502` | `0xEA09D4AE` |
| 13 | airport developer | `0xEA20548D` | `0xAA2049A1` |
| 14 | seaport developer | `0xCA22FE3E` | `0xEA230A28` |
| 15 | park manager | `0xAA3FE6BC` | `0xAA3FE6BC` |
| 16 | lot developer | `0x4818E5D2` | `0xA990BFE0` |
| 17 | tract developer | `0xA818ED1B` | `0x2990C142` |
| 18 | **simulator** | `0x8695664E` | `0x2990C1E5` |
| 19 | aura simulator | `0xC773E880` | `0x0990BDD8` |
| 20 | budget simulator | `0x6772BD23` | `0xE990BE01` |
| 21 | building development simulator | `0x07813EAE` | `0x8990BE7A` |
| 22 | commercial simulator | `0x077ABEB2` | `0x8990C080` |
| 23 | crime simulator | `0x67659555` | `0x8990C09A` |
| 24 | demand simulator | `0x47967E16` | `0x2990BFF2` |
| 25 | fire protection simulator | `0x88F54C7B` | `0x2990C05A` |
| 26 | flammability simulator | `0x48F41617` | `0xE990C045` |
| 27 | flora simulator | `0x06C66177` | `0xC990C034` |
| 28 | industrial simulator | `0x477A9E7D` | `0xA990C022` |
| 29 | land value simulator | `0xE7781D8C` | `0xE990C18E` |
| 30 | neighbours simulator | `0x0773FEF0` | `0xC990C0A4` |
| 31 | ordinance simulator | `0x41193C3A` | `0xA990C08E` |
| 32 | plumbing simulator | `0x47782CCC` | `0x0990C075` |
| 33 | police simulator | `0x67659895` | `0x6990C04F` |
| 34 | pollution simulator | `0x877181EC` | `0x8990C065` |
| 35 | power simulator | `0xE9E7BFF5` | `0x0990C005` |
| 36 | residential simulator | `0x077AC1EE` | `0x4990C013` |
| 37 | traffic simulator | `0x68027F37` | `0x6990C1AA` |
| 38 | weather simulator | `0xC6C66245` | `0x2990C1BC` |
| 39 | civic building simulator | `0x8A41322F` | `0x0A413481` |
| 40 | automata script system | `0x8A1B6647` | `0x4A1B664E` |
| 41 | automata controller manager | `0xCA14C9FC` | `0xCA14CA03` |
| 42 | traffic network map | `0xCA0F81B9` | `0x6A0F82B2` |
| 43–46 | vehicle, pedestrian, aircraft, watercraft managers | `0x098F9BF4` (all four) | `0xA98F9D3B`, `0xC98F9D17`, `0xC98F9D4A`, `0xA98F9D28` |
| 47 | advisor system | `0x4A0DD617` | `0x4A0DD616` |
| 48 | tutorial system | `0xAA3836F6` | `0xAA3836FB` |
| 49 | terrain | `0xC70BD37E` | `0xE98F9525` |
| 50 | 24-hour clock | `0x681BD53A` | `0xA7E7F929` |
| 51 | city detail manager (city version ≥ 9) | `0x6AD4B9F1` | `0xEAD4BA36` |
| 52 | city situation manager (version ≥ 11) | `0x6BB1437A` | `0x0BB14381` |
| 53 | network manager (version ≥ 12) | `0xA9764C8C` | `0xC990BD46` |

A `uint8` follows the references. The city's other fields are a later city spec's business. The fire-protection
slot class `0x2990C05A` is the class the ID table also names `cSC4ProtectionSimulator`.

**What a replacement subsystem must do to interoperate** *[compat]*:

1. Register a class factory for its class ID so that `GZCOM()->GetClassObject(classID, iid, &p)` creates it, both
   for new cities (the city asks for its own interface) and on load (the store asks for `GZIID_cIGZSerializable`,
   `0xE4FDA3D4`).
2. Answer `QueryInterface` for `cIGZSerializable` and for the IID in the table above.
3. Return its class ID from `GetGZCLSID()`. That value becomes the record type.
4. Accept `Read` on a fresh, un-initialised instance. `Read` happens **before** `Init` (§3.7).
5. Write only through the stream it is given, following its own subsystem spec's layout.

### 3.7 Save and load sequences

**Save** (`cSC4App::SaveCity`): the game writes a **new** package to a temporary file and then replaces the city
file with it (keeping the old one until the rename succeeds). In order:

1. Create the package. Send `kSC4MessagePreSave` `0x26C63343` (void1 = package segment, void2 and void3 = file
   path strings, data4 = the caller's flag), then `0x26C63346` with the same data (name unknown).
2. Write the app version record (65) and the module version record.
3. Write the root record. Its reference registers the city. *(The city's `Write`, and through it every subsystem's
   `Write`, runs later, in step 5.)* The same helper then writes the current city's regional-city record (§11.1).
4. Send `kSC4MessageSave` `0x26C63344` and `kSC4MessagePostSave` `0x26C63345`.
5. Run the class-object store save (§3.5): the city, the 53 components and everything they reference.
6. Close the package, then rename it over the city file.
7. If the save succeeded: rewrite the regional-city record inside the file of every adjacent city (§11.1).

Subscribers to PreSave/Save may write their own records into the package (void1). They run **before** any
object's `Write`.

**Load** (`cSC4App::LoadCity`):

1. Open the package. The app version record must hold 62..65. If the record is missing it counts as 65. If it is
   present but unreadable the load is rejected.
2. Send `kSC4MessagePreLoad` `0x26C63340` (void1 = segment, void2 = path).
3. **Load the class-object store** (§3.5). Every saved object, including the city and all subsystems, is created
   and `Read` here, in ascending class ID order. If this step reports failure, the load fails.
4. Read the root record: `uint16` must be 1, then `uint16` must be 2, then the city reference.
5. Send `kSC4MessageLoad` `0x26C63341` and `kSC4MessagePostLoad` `0x26C63342`. **Send** (synchronously)
   `kSC4MessagePreCityInit` `0x26D31EC0`, run the city's `Init` (which initialises its components, the simulator
   first), then **post** (queued) `kSC4MessagePostAppServicesInit` `0x2B96B3EA`, `kSC4MessagePostCityInit`
   `0x26D31EC1`, `kSC4MessagePostAppServicesInitComplete` `0xAB96B05F` and `kSC4MessagePostCityInitComplete`
   `0xEA8AE29A`. These city messages carry void1 = IGZUnknown = the city.

The city's `Init` initialises components in this order (relevant for `Init`-time dependencies) *[inferred]*:
simulator, history warehouse, terrain, surface water, effects manager, automata script system, automata controller
manager, occupant manager, zone manager, lot configuration manager, zone developer, landfill / airport / seaport
developers, park manager, lot manager, 24-hour clock, prop manager, network manager, dispatch manager, prop
developer, network lot manager, **budget, ordinance, power, plumbing, neighbours, building development**, tract
developer, lot developer, **demand, aura, commercial, crime, fire protection, flammability, flora, industrial, land
value, police, traffic, pollution, residential, weather, civic building**, disaster layer, demolition utility,
traffic network map, vehicle / pedestrian / aircraft / watercraft managers, advisor system, city situation
manager, tutorial system, MySim agent simulator, city detail manager.

### 3.8 Retained records

When a city is loaded, every record in group **`0xAB85E27D`** (up to 10,000,000 bytes each) is kept in memory
and written back unchanged on the next save *[compat]*. *[choice]* FreeCity4 may use this group for its own
extension data. The original game will carry it through a save.

### 3.9 Unknowns

See §8, questions 1–5.

## 4. Response curves

Tuning properties named "… curve" (for example "Funding To Efficiency Response Curve") hold a flat `float32`
array `x0, y0, x1, y1, …` that the game evaluates as a **response curve**. One shared helper builds and evaluates
them (`cSC4ResponseCurve`). More than a hundred call sites use it, including the budget, demand, traffic,
neighbours and service simulators.

### 4.1 Loading from a property

- The property's values are taken as `float32` and paired up: `n = count / 2` points. An odd trailing value is
  ignored.
- *[compat]* Curve properties must be `float32` arrays. The original reinterprets the raw bytes of other value
  types as floats, and misbehaves for single-value properties. *[choice]* FreeCity4 converts integer arrays to
  float, and treats non-numeric or single-value properties as absent.
- Loading a **present** property first discards the curve's old segments, then builds the new ones (§4.2). It
  returns false when there are no points; the curve is then empty (§4.3, zero points).
- **A missing property: the behaviour is per caller** *[compat]* (corrects the earlier blanket rule "a missing
  property removes the curve"; see Errata, M3 item 1). The shared loaders never delete a curve object. What a
  missing exemplar or property does depends on what the caller does around the load, and there are four
  patterns. Each subsystem spec must say which pattern applies to each of its curves, and what an absent or empty
  curve means for it.

  | Pattern | What the caller does | Exemplar or property missing | Users (verified call sites) |
  |---|---|---|---|
  | **keep** | loads in place with the "by exemplar key and property ID" loader, which fetches the exemplar, looks up the property and loads only if both exist | the curve keeps its **previous** segments (empty on the first load); the loader returns false | budget, traffic, pollution's per-occupant curves |
  | **default** | first rebuilds the curve from a code-default point list, then loads as in "keep" | the curve holds the **code default** | crime, fire protection, police (curves with a code default; protection §6), power's five tuning curves (power §8). Plumbing's curve `0x6A410BF2` is a variant: "keep", then (0,0)(100,100) if the curve is still empty after the load (plumbing spec) |
  | **empty** | first empties the curve (or uses a fresh temporary one), then loads as in "keep" | the curve is **empty** | pollution's tuning curves (pollution §9), pollution's temporary curves in `SimulationBegin` / `EndOfCycle` |
  | **delete** | if the property is present, loads it (creating the curve object when needed); otherwise deletes its curve object. Aura and flammability always delete first and then create a new object when the property is present | the curve is **absent** (no object) and the caller takes its "no curve" path | land value (landvalue §8), aura (all curves), flammability (`0x29B4013D`), demand (`0x4A1F38B5`, `0x4A1F38B6`); power's two per-building curves `0x8A1E07EE` / `0x0911E2E7` follow it per call, with a fresh temporary curve (power §8) |

  "Keep" differs from the others only when tuning is re-read (another `Init` after `Shutdown`, or a tuning-reload
  message) after the property disappeared, which shipped content never does. A present property with no points
  always gives an empty curve. *[choice]* FreeCity4 implements the pattern stated per curve, and treats an empty
  curve like an absent one only where the subsystem spec says so.

### 4.2 Construction *[compat]*

The point list is turned into an ordered list of **segments**. For each input point *i*, the predecessor is
input point *i − 1*. For *i = 0* the predecessor is `(−FLT_MAX, y0)`.

```
for i in 0 .. n−1:
    (x, y)   := point i
    (px, py) := predecessor (input point i−1, even if it was dropped)
    dy := y − py ;  dx := x − px                     (float32)
    if |dy| < 1e-10:            keep segment {x, y, px, py, slope = 0}
    else if dy · 1e-10 < dx:    keep segment {x, y, px, py, slope = dy / dx}     (slope stored as float32)
    else:                       drop the point
```

- Flat points are **always kept**, whatever their x.
- A point with the same x as its predecessor and a **higher** y is dropped. One with a **lower** y is kept with
  slope −∞, but §4.3 never selects it. Either way the curve makes a **vertical step** at that x.
- The predecessor is always the previous *input* point. After a dropped point, the next segment interpolates
  from the dropped point, not from the last kept one (example 7 in §4.4).
- Points are expected in ascending x. Out-of-order points that survive (only flat ones can) make the search in
  §4.3 operate on unsorted data. *[choice]* That case is undefined. Conformance tests do not cover it.

### 4.3 Evaluation *[compat]*

With `m` kept segments and `last = m − 1`:

```
k := the smallest index in 0 .. last whose x > input; k := last if there is none
if k = 0:                                   return y of segment 0
if k < last or input < x of segment last:   return py_k + (input − px_k) · slope_k
otherwise:                                  return y of segment last
```

- **Clamped at both ends.** Below the first x the output is `y0`, and from the last x on it is `y_last`.
- **Exact at the points.** At an interior point `x_i` the search selects the next segment, whose start is
  `(x_i, y_i)`, so the result is exactly `y_i`.
- **One point**: constant `y0`.
- **Zero points**: the original reads garbage (the helper is undefined). *[choice]* Return 0.0. Callers should
  treat an empty curve as absent.

Arithmetic: on the Windows build the expression `py + (input − px) · slope` is evaluated on the x87 at 24-bit
precision (§18): the subtraction, the product and the sum are each rounded to a 24-bit significand, i.e. three
`float32` operations. *[compat]* FreeCity4 evaluates it as three `f32` operations in that order. Tests compare
with a relative tolerance of 1e-6 (corrected: the earlier text assumed one rounding at the end).

### 4.4 Worked examples (shipped tuning values, `re/tuning/simulators.md`)

1. **Police funding → efficiency** `0x68DDAE93` = (0,0) (100,100) (120,110):
   f(−10) = 0, f(0) = 0, f(50) = 50, f(100) = 100, f(110) = 105, f(150) = 110 (clamped).
2. **Health quotient → life expectancy** `0x6941A67F` = (0,50) (20,55) (40,60) (60,65) (200,100):
   f(10) = 52.5, f(60) = 65, f(100) = 65 + 40 × 0.25 = 75, f(250) = 100.
3. **Crime, R$ jobless ratio → criminality** `0x8A0CBAA7` = (−1,0.1) (−0.7,0.07) (0,0) (0.7,−0.07) (1,−0.1):
   f(−2) = 0.1, f(−0.85) = 0.085, f(0.35) = −0.035, f(1.5) = −0.1 (all as float32).
4. **Jailbreak** `0xEA614F63` = (0,0) (80,0) (90,10) (100,100): f(50) = 0, f(85) = 5, f(95) = 55, f(100) = 100.
5. **Air pollution → HQ decay** `0x6941B022` = (0,0) (512,0.1) (1024,50): f(256) = 0.05, f(768) ≈ 25.05,
   f(2000) = 50.
6. **Land value → simoleons** `0x4AC519B8` = (0,0) (255,0): flat, so 0 everywhere. **Neighbour garbage cost**
   `0x4B61FFC9` = (0,1e−5) (0.0075,0.0075) (1,0.075): f(0) = 1e−5, f(0.5) ≈ 0.0409950, f(2) = 0.075.
7. Synthetic step (not shipped): (0,0) (5,10) (5,20) (10,30). The third point is dropped. The fourth interpolates
   from (5,20) with slope 2. f(4.999) ≈ 9.998, f(5) = 20, f(7.5) = 25.
8. Synthetic drop (not shipped): (0,0) (5,20) (5,10) (10,0): f(4.9) = 19.6, f(5) = 10, f(7.5) = 5.
9. Synthetic out-of-order x (not shipped): (0,0) (10,10) (5,20) (20,40). (5,20) is dropped, and the last segment
   starts from it: f(5) = 5, f(10) ≈ 26.667 (a jump), f(15) ≈ 33.333, f(20) = 40.

## 5. Message objects

### 5.1 The standard message (`cRZMessage2Standard`)

The simulation uses one concrete message class for clock messages, notifications and agent calls. Its class ID
is **`0x46DF45E7`** and it implements:

| Interface | IID |
|---|---|
| `cIGZMessage2Standard` | `0x65297976` |
| `cIGZMessage2` | `0xA52294B4` *[compat]* (confirmed: the standard message class answers `QueryInterface` for `1`, `0x65297976` and `0xA52294B4`) |
| `cIGZUnknown` | `1` |

You get one from `GZCOM()->GetClassObject(0x46DF45E7, 0x65297976, &p)`, from the message server's
`CreateMessage(0x46DF45E7, …)`, or from `Create(iid, &p)` on an existing message, which makes a new standard
message with the same **type**.

| Field | Initial value | Setter effect | `GetHas…` |
|---|---|---|---|
| type | 0 | `SetType(t)` | (internal "has type" bit = `t ≠ 0`) |
| data1 … data4 | 0 | `SetDataN(v)` and `SetVoidN(p)` write the **same** 32-bit slot | true once set, never cleared |
| string | none | `SetString(s)` stores a **reference** (`AddRef`), not a copy. Null clears it. | true while set |
| IGZUnknown | none | `SetIGZUnknown(o)` stores a reference. Null clears it. | true while set |
| extra | none | `SetExtra(s)` stores a reference to a `cIGZSerializable`. Null clears it. | true while set |

- `GetDataN` and `GetVoidN` return the slot unchanged. On the 32-bit game a pointer and an integer share it.
- `GetString(iid, &p)` and `GetExtra(iid, &p)` are really `QueryInterface(iid, &p)` on the stored object. They
  return false when nothing is stored. *[compat]* The SDK names the first parameter `dwMaxLen`. It is an
  interface ID (use `GZIID_cIGZString` `0x089B7DC8`).
- `operator==` and `operator<` compare **types** only (unsigned).
- Reference counting: a new message starts at count 0. The creator `AddRef`s it. `Release` destroys it when the
  count was 1 (or 0) and otherwise decrements. Destruction releases the string, IGZUnknown and extra references.

**Conventions used by specs** *[compat]*: "message (type T, data1 = a, data2 = b, data3 = c)" means a fresh
standard message with `SetType(T)` and `SetData1/2/3`. Unset fields stay 0. "void1 = X" means `SetVoid1(X)`.
"IGZUnknown = X" means `SetIGZUnknown(X)`.

**Delivery** (message server, `cIGZMessageServer2`):

- `MessageSend(m)`: synchronous. It calls `DoMessage(m)` on every target subscribed to `m`'s type, then on every
  target subscribed to type **0** (wildcard), each in subscription order. The targets are snapshotted before
  delivery. It returns true.
- `MessagePost(m, highPriority)`: queues `m` (the queue holds a reference) at the back, or at the front if
  `highPriority`. Queued messages are sent on a later message-server tick.
- The simulator's own pattern *[compat]*: create, `AddRef`, set type and data1..3, send or post, `Release`.

*[compat]* **SDK discrepancies.** The gzcom-dll `cRZMessage2Standard` implementation differs from the original. It
copies strings, does not implement `GetExtra`/`SetExtra`, and its `GetHasData2..4` are wrong because of a bit test
error. Our shim must implement its own message class or use the game's (through `GetClassObject`). It must not use
the SDK's implementation.

### 5.2 Shared IDs used by several simulators (resolved in M3)

These IDs appear in several subsystem specs. Their meaning is defined once here.

| ID | Kind | Meaning | Evidence |
|---|---|---|---|
| `0x029244DB` | occupant property, bool | **Preview occupant** *[inferred name]*. The lot-placement tool (the building model shown under the cursor before a lot is placed) and the toll-booth tool set it to true on the temporary occupants they create. Grid simulators ignore insert / remove / change messages for such occupants. Aura, crime, flammability and plumbing test only that the property **exists**; pollution tests that its value is **true**. The model maker also creates no shadow for them. | setters in the place-lot and toll-booth tools; readers in every grid simulator's occupant handler |
| `0xC53D10AA` | message | **Exemplar changed** *[inferred name]*. Posted (`MessagePost`, normal priority) by the exemplar resource code when an exemplar resource is written out, and by the exemplar debug view when it updates one. data1 = the exemplar's group, data2 = its instance. Each simulator re-reads its tuning when (data1, data2) equals its tuning exemplar's group and instance. Shipped play never writes exemplars, so the message is not expected in a normal session; FreeCity4 must still handle it. | senders: exemplar factory write path, exemplar resource view |
| `0xC6CDBF1A` / `0xC6CDBF1B` | message | **PreViewLevelChange / PostViewLevelChange** (names from the game's own message-name table). Sent synchronously (`MessageSend`) by the 3D view when its view level changes, Pre first and then Post. void1 points at the **old** level and void2 at the **new** level (u32 values, valid only during delivery). View levels: **1 = surface**, **2 = subway**, **3 = water pipes** *[inferred: the per-occupant view level is 2 for subway occupants and 3 for pipe occupants, otherwise 1]*. | sender: the 3D view's "set current view level"; readers: pollution (effects shown only when the new level is 1), landfill developer, pipe-burst disaster |
| `0x8A4260A6` | department ID | **Parks and Recreation**. The park manager creates this department budget at `Init` when it does not exist and names it with LTEXT group `0x6A3FF01C`, instance `0xCA49375E` ("Parks and Recreation" in `SimCityLocale.dat`). The cruise-ship reward building's exemplar also charges this department (budget-department property `0xEA54D283`), which is where the earlier "cruise ship" label came from. | park manager `Init`; exemplar `RW43x30_Cruise_Ship_Building_03BC` |

**Lot state and lot condition** (`cISC4Lot` compiled slots 33/34 `GetState`/`SetState` and 35/36
`GetCondition`/`SetCondition`) are two different bytes:

- State 4 is **abandoned**: the lot developer's "abandon building" sets state 4.
- The condition is updated by the lot developer's per-lot update. A lot in state 3 gets a condition in 0..3, never
  4. A lot in state 4 (abandoned) moves through conditions 1 → 2 → 3 → 4: it gets 1 at once, and then one more
  step each time its "days since last state change" exceeds the lot developer's threshold. `SetCondition` on an
  abandoned lot restarts that day counter, so each step takes the full threshold. Other states keep their
  condition.
- So **condition 4 means an abandoned lot at its last decay stage** *[inferred from the update rule; the enum has
  no names in the binary]*. Crime's arson mask (crime §7) and flammability's abandonment multiplier `0x29B40134` (flammability §6) apply
  only to such lots. Those specs read the **condition**, not the state.

## 6. Random numbers

### 6.1 Generators

The game has two generator classes. Both are exact algorithms that our engine must reproduce bit for bit
*[compat]* (§6.4 says why).

**`cRZRandom`** holds two 32-bit states, `lcg` and `mul` (both set to the seed), plus a cached-Gaussian flag:

| Operation | Definition |
|---|---|
| `Seed(s)` | `lcg := mul := s`. If `s = 0xFFFFFFFF`, s is first derived from the wall clock (non-deterministic). |
| `NextRaw()` (`RandomUint32Uniform()`) | `t := lcg × 0x41C64E6D + 0x3039` computed in **64 bits**; `lcg := t mod 2^32`; result = `(t >> 16) mod 2^32`. The result's high 16 bits come from the upper half of the 64-bit product. |
| `RandomUint32Uniform(n)` | `n = 0` → 0, no state change. Otherwise `(NextRaw() × n) >> 32` (64-bit product). |
| `RandomSint32RangeUniform(lo, hi)` | `lo + RandomUint32Uniform(hi − lo)` (unsigned difference). |
| `RandomDoubleUniform()` | if `mul = 0`, `mul :=` the process clock. Then `mul := mul × 0x278DDE6D mod 2^32`; result = `int32(mul) / 2^32 + 0.5`, in [0, 1). |
| `RandomDoubleRangeUniform(a, b)` | `a + RandomDoubleUniform() × (b − a)` |
| `RandomDoubleGaussian()` | Marsaglia polar method on two `2·RandomDoubleUniform() − 1` draws, repeated until `0 < s < 1`. It returns one value and caches the other for the next call. |
| `RandomDoubleRangeGaussian(a, b)` | `g := Gaussian + 3`, clamped to [0, 6) (≥ 6 gives 0.9999833 after scaling), mapped `a + (b − a) · g / 6` |
| `RandomDoubleRangeGaussianFast(a, b)` | `a + (u1 + u2 + u3) · (b − a) / 3` with three `RandomDoubleUniform` draws |
| `RandomSint32RangeGaussianFast(lo, hi)` | always exactly 4 `NextRaw()` draws `r1..r4`: `a = (r1 + r2 + r3 + r4 + 2) >> 2` with the sum in 64 bits, then `lo + ((a × (u32)(hi − lo)) >> 32)`. It draws 4 times even when the range is empty. Test: seed 1, `(0, 25)` gives 10, then 7 (plumbing spec) |

The integer methods use only `lcg`. The double methods use only `mul`. A generator seeded with 0 has a
deterministic `lcg`, but its double methods fall back to the clock.

**`cRZRandomSuper`** is a Mersenne Twister (MT19937: N = 624, M = 397, matrix `0x9908B0DF`, standard tempering)
with a **non-standard seeding**:

```
x := seed | 1          (seed 0xFFFFFFFF → derived from the wall clock first)
for i in 0 .. 623:
    state[i] := (x & 0xFFFF0000) | ((x · 69069 mod 2^32) >> 16)
    x := (x · 0x1C587629 + 0x10DCE) mod 2^32           (= two steps of x·69069 + 1)
regenerate all 624 words (standard MT twist); next output is word 0
```

| Operation | Definition |
|---|---|
| `RandomUint32Uniform()` | next tempered word |
| `RandomUint32Uniform(n)` | `(next × n) >> 32` |
| `RandomSint32RangeUniform(lo, hi)` | `lo + ((next × (hi − lo)) >> 32)` |
| `RandomDoubleUniform()` | `int32(next) / 2^32 + 0.5` |
| `RandomDoubleRangeUniform(a, b)` | `a + RandomDoubleUniform() × (b − a)` |
| `RandomUint32WeightedChoice(n, w[])` | n < 2 → 0. Otherwise, with `S` = sum of the positive weights: if `S = 0` → `RandomUint32Uniform(n)`. Otherwise `r := RandomDoubleUniform() × S`, walk the positive weights subtracting until `r < w[i]`, return i (n − 1 if none). |

### 6.2 Which generator the simulation uses

There is **no** single simulation RNG. Each subsystem uses its own instance:

| Generator instance | Seed | Used by (examples) |
|---|---|---|
| a `cRZRandom` member of the subsystem | wall clock (`0xFFFFFFFF`) at construction | residential, weather, flora, power, aura, fire protection, police, zone manager, advisor system (advisors.md §9), each MySim (mysims.md); plumbing (created at `Init`) |
| a `cRZRandom` member | **0** (integer path deterministic, double path clock-seeded) | flammability |
| a file-local `cRZRandomSuper` per source file | local wall-clock seconds XOR a per-file constant | budget (the `RiskyMoney` disaster roll), crime, pollution effects, lots, construction, network lots |
| a `cRZRandomSuper` member | wall clock at `Init` | traffic simulator, traffic network map, prop developer (clock × 28) |
| a process-wide `cRZRandom` (lazy) | wall clock | neighbours deal propositions |
| a temporary `cRZRandom` | **cell coordinates** (`a · 65536 + b`) | lot developer desirability test: deterministic per cell *[inferred: which coordinate is `a`]* |
| a temporary `cRZRandom` | wall clock | civic building monthly simulation |
| the C runtime `rand()` | state 1 at process start, **never seeded** by the game (§12) | `std::random_shuffle` of power poles, pipes, props and lot lists; landfill tiles; the random cell iterators (§12.5) |

- **No RNG state is saved** in any record specified so far (simulator, budget, demand), and seeding never reads
  the save. Loading the same save twice in the original gives different random streams *[compat]*: a saved city
  has no defined random future.
- *[compat]* Each subsystem spec must name the generator instance and the exact method (and therefore the number
  of state advances) at every random draw.

### 6.3 Seeding in FreeCity4 *[choice]*

- The core takes an explicit seed per generator instance from the host (`fc4.h`). The shim seeds from the wall
  clock like the original, unless a test or replay asks for fixed seeds.
- Generators are separate objects per subsystem, exactly as in §6.2. Merging them would change the sequence each
  subsystem sees.

### 6.4 Differential testing *[choice]*

Because the original seeds from the clock, exact differential runs need the observer plugin to **force seeds**
in the original (hooking the generators' seed operation) and log each subsystem's draws. With both sides seeded
identically and the algorithms of §6.1, the draw sequences must match exactly. Without forced seeds, compare only
RNG-free quantities (clock, budget arithmetic) or distributions (growth statistics, milestone M4).

## 7. Conformance tests (must exist in `engine/conformance`)

1. **Primitives**: each `Set*` writes the bytes of §2.2 (for example `SetUint32(0x11223344)` →
   `44 33 22 11`; `SetFloat32(1.0)` → `00 00 80 3F`; `SetFloat64(1.0)` → `00 00 00 00 00 00 F0 3F`); round trip
   for edge values (0, −1, min/max, NaN bit patterns, −0.0).
2. **Sticky error**: on a 6-byte record, `GetUint32` succeeds, the next `GetUint32` fails without consuming, the
   destination is unchanged, `GetError() ≠ 0`, and a following `GetUint8` also fails.
3. **Strings**: `"Fire Dept."` ↔ `0A 00 00 00 46 69 72 65 20 44 65 70 74 2E`; empty string ↔ `00 00 00 00`; a
   UTF-16 header `0x10000002` followed by `41 00 E9 00` reads as UTF-8 `"Aé"`; count 200,001 fails;
   `GetRZCharStr` truncation without terminator.
4. **Resource key**: `{type 1, group 2, instance 3}` ↔ `01 00 00 00 02 00 00 00 03 00 00 00`.
5. **References**: null ↔ `00 00 00 00`; object (ID 5, class `0x8990C12D`) ↔ `05 00 00 00 2D C1 90 89`; two
   references to the same object resolve to one instance; a cycle A↔B round-trips; an unknown class ID fails.
6. **CRC**: `"123456789"` → `0x0376E6E7`; empty input → `0xFFFFFFFF`; bytes `01 00 00 00 01 00` → `0xF34B70D6`.
7. **Class record**: an object with ID 1 whose `Write` emits `01 00` gives the entry
   `0E 00 00 00 D6 70 4B F3 01 00 00 00 01 00`. The index for that store alone is
   `01 00 00 00 01 00 00 00 01 00 00 00` + class ID + `01 00 00 00`. Payloads over 250,000 bytes use the truncated
   CRC.
8. **Store save order**: classes ascending, objects ascending within a pass, objects registered during a pass
   appended in a later pass; one object referenced twice is written once.
9. **Store load**: forward and backward references resolve. A `Read` that under-reads or over-reads within its
   entry does not disturb the next entry. A CRC mismatch stops that class only. An unregistered class's
   entries are skipped. Write → load → write is byte-identical (with IDs renumbered consistently).
10. **City references**: a stub city with 53 slots writes and reads them in the order of §3.6. The lot manager
    slot may refer to the same object as the zone developer.
11. **Response curves**: every example in §4.4 (tolerance 1e-6 relative), plus one point (constant), odd value
    count (last value ignored), and zero points (0.0, curve treated as absent).
12. **Messages**: `SetVoid1(p)` then `GetData1()` returns p; `GetHasData1..4` false until set; `SetString(null)`
    clears; `Create` copies the type only; `==` compares type only; reference counting frees at the last
    `Release`.
13. **`cRZRandom`**: seed 1 → `RandomUint32Uniform()` gives `0x000041C6`, `0x59D4967E`, `0xBF022781` (lcg states
    `0x41C67EA6`, `0x967EB0E7`, `0x2781E494`). Seed 1 → `RandomDoubleUniform()` gives 0.6545084968674928,
    0.48173871100880206, 0.851256174268201 (mul states `0x278DDE6D`, `0xFB533A69`, `0x59EBECB5`).
    `RandomUint32Uniform(0)` returns 0 without advancing.
14. **`cRZRandomSuper`**: seed 1 → first words of the initial table `0x00000001`, `0x1C59C359`; first outputs
    `0xD3F6B9E5`, `0xEF284157`, `0x0D3639C7`. Seed 4357 → `0xAAE64AC3`, `0x5E7C47DA`, `0xB98E8121`. These vectors
    were computed from §6.1 and must be confirmed against the original (open question 6).
15. **Weighted choice**: all weights ≤ 0 falls back to uniform; weights {0, 1, 0} always pick 1.

## 8. Open questions

1. **Load result quirk** (§3.5): does the shipped game really accept a save in which an early class fails, as the
   analysis suggests? A save with a deliberately corrupted early class record would settle it.
2. **Text encoding** of `SetGZStr` for non-ASCII city, mayor and department names. A save with an accented city
   name would confirm UTF-8 (or reveal a code page).
3. ~~**Variant tags** (§2.8)~~: answered in §9 (tag table) and §10 (property holders, the records that use them).
4. **Other city-file records** (§3.2): `0x89D9B9EE/0x69D9B9F1`, region, thumbnail and terrain records, and
   unnamed message `0x26C63346`. Not needed for simulator replacement, but needed for standalone mode.
5. ~~**C runtime `rand()`** seeding~~: answered in §12 (never seeded by the game; algorithm and users listed).
6. **RNG vectors** (§7 items 13–14): confirm with a runtime capture from the original (force a seed and log the
   first outputs).
7. **Object `Init` order on load** (§3.7): confirmed only from the city's `Init` list. Which `Init`-time
   cross-subsystem calls depend on it is for each subsystem spec.

### Addendum: message vtable slot order *[compat]* (verified against the game)

The standard message class's vtable matches the gzcom-dll `cIGZMessage2Standard` declaration order exactly. The order is: QueryInterface, AddRef, Release, Create, GetType (`+0x10`), SetType, `==`, `<`. Then, for data slots 1-4, `GetHasDataN`/`GetDataN`/`GetVoidN`/`SetDataN`/`SetVoidN`; data 1 starts at `+0x20`. Then String ×3, IGZUnknown ×3 (`GetIGZUnknown` at `+0x80`) and Extra ×3. Game agents read messages through `+0x10`, `+0x20`, `+0x24`, `+0x3C` and `+0x80`.

## 9. Variants (`cIGZVariant`)

Sources: the gzcom-dll `cIGZVariant` header (in-memory type numbers), the original reader and writer, and the
bytes of property holders in sample saves (§10.6), which confirm the marker, the tag width and the tag values
for `Uint8`, `Uint32`, `Sint32`, `Sint64`, `Float32` and the `Uint16`, `Uint32` and `Float32` arrays.

### 9.1 In-memory types *[compat]*

A variant holds one value or an array of values of one type. `GetType()` returns a 16-bit **memory type**; bit 15
(`0x8000`) marks an array, whose length is `GetCount()`. The numbers are those of the SDK header and are what
command parameter sets (`cIGZCommandParameterSet`), messages, Lua glue and property values see:

| Memory type | Name | Memory type | Name |
|---|---|---|---|
| `0x00` | Empty | `0x09` | Sint64 |
| `0x01` | Bool | `0x0A` | Float32 |
| `0x02` | Uint8 | `0x0B` | Float64 |
| `0x03` | Sint8 | `0x0C` | Char |
| `0x04` | Uint16 | `0x0D` | RZUnicodeChar (UTF-16 unit) |
| `0x05` | Sint16 | `0x0E` | RZChar (8-bit char) |
| `0x06` | Uint32 | `0x0F` | VoidPtr |
| `0x07` | Sint32 | `0x10` | IGZUnknown (an object reference) |
| `0x08` | Uint64 | `0x8000 \| t` | array of `t`; `0x8000` alone = raw bytes ("void array") |

A **string** value is an `RZChar` array (`0x800E`) whose count is the string length (no terminator) *[inferred]*.
Exemplar string properties load as this type.

### 9.2 On-disk encoding (`WriteVariant` / `ReadVariant`) *[compat]*

```
uint32  marker          0xA9BEE9AC if the memory type is exactly IGZUnknown (0x0010), otherwise 0
if marker = 0xA9BEE9AC:
    ref   object        (§2.7) the held object QueryInterface'd to cIGZSerializable; null if none or not serializable
else:
    uint32  diskTag     from the table below (array bit 0x8000 kept)
    value               scalar: one element.   diskTag 0x800C: a string (§2.3).
                        other arrays: uint32 count, then count elements.  diskTag 0: nothing.
```

The **disk tag is a different numbering** from the memory type. It is the property type numbering of exemplar
files (`diskTag << 8` = the exemplar's type code: `0x0100` Uint8, `0x0300` Uint32, `0x0900` Float32, `0x0B00`
Bool, `0x0C00` string, ...).

| Memory type | Disk tag | Element bytes | Read back |
|---|---|---|---|
| Empty `0x00` | `0x00` | none | yes (empty variant) |
| Bool `0x01` | `0x0B` | 1 (0 or 1) | yes; any non-zero byte is true |
| Uint8 `0x02` | `0x01` | 1 | yes |
| Sint8 `0x03` | `0x05` | 1 | yes |
| Uint16 `0x04` | `0x02` | 2 | yes |
| Sint16 `0x05` | `0x06` | 2 | yes |
| Uint32 `0x06` | `0x03` | 4 | yes |
| Sint32 `0x07` | `0x07` | 4 | yes |
| Uint64 `0x08` | `0x04` | 8 | yes |
| Sint64 `0x09` | `0x08` | 8 | yes |
| Float32 `0x0A` | `0x09` | 4 (IEEE bits) | yes |
| Float64 `0x0B` | `0x0A` | 8 (IEEE bits) | yes |
| Char `0x0C` | `0x11` | 1 | **no** *[quirk]* |
| RZUnicodeChar `0x0D` | `0x10` | 2 | **no** *[quirk]* |
| RZChar `0x0E` | `0x0C` | 1 | yes |
| VoidPtr `0x0F` | `0x12` | write fails | — |
| IGZUnknown `0x10` | (marker path) | `ref` | yes |
| void array `0x8000` | `0x8000` | count raw bytes | yes |
| Bool array `0x8001` | `0x800B` | 1 each | yes |
| Uint8 … Float64 arrays `0x8002`–`0x800B` | `0x8001`, `0x8005`, `0x8002`, `0x8006`, `0x8003`, `0x8007`, `0x8004`, `0x8008`, `0x8009`, `0x800A` (same order as the scalar rows) | as the scalar | yes |
| Char array `0x800C` | `0x8011` | 1 each | yes |
| RZUnicodeChar array `0x800D` | `0x8010` | 2 each | yes |
| RZChar array (string) `0x800E` | `0x800C` | §2.3 string: `uint32` header + bytes | yes |
| VoidPtr array `0x800F`, IGZUnknown array `0x8010` | `0x8012`, `0x800D` | write fails | — |

Details *[compat]*:

- **String arrays** (`0x800C`) have no separate count: the §2.3 header is the count. For 8-bit text the bytes are
  therefore identical to "count, then count bytes". The reader applies §2.3 completely (including the UTF-16
  header `0x10` and its quirks) and stores the result as an `RZChar` array.
- **Write failures**: for the unsupported types the marker and the disk tag are written first, then the call fails
  and the stream enters the error state (§2.1). Writers must not store `VoidPtr` values or arrays of `VoidPtr` or
  `IGZUnknown` in anything that is saved.
- *[quirk]* `Char` and `RZUnicodeChar` **scalars** are written (tags `0x11`, `0x10`) but the reader does not accept
  these tags. Reading one fails. Writers should store such values as `Uint8` / `Uint16` *[choice]*.
- **Reader tag set**: scalar tags `0x00`–`0x0C`; array tags `0x8000`–`0x800C`, `0x8010`, `0x8011`. Any other tag
  fails (stream error).
- **Marker on read**: only `0xA9BEE9AC` selects the object path; *[quirk]* **any** other value, not just 0, is
  treated as 0. *[choice]* Standalone readers may reject markers other than 0 and `0xA9BEE9AC`.
- **Object path on read**: read the reference (§2.7, without `loadNow`). A null reference succeeds and leaves the
  destination variant **unchanged** *[quirk]*. Otherwise the variant becomes an `IGZUnknown` holding the object.
  Every reader in the game reads into a **freshly constructed (Empty)** variant, so a null reference in practice
  yields an Empty value. A property holder then drops it (§10.2).
- **Array counts** have no upper limit in the original. *[choice]* FreeCity4 rejects a count whose elements
  would extend past the end of the record.
- The array path stores the **elements only**. There is no terminator, alignment or padding.
- `WriteVariant`/`ReadVariant` exist only on the segment streams (§2.1). The `<<` / `>>` helpers that property code
  uses ask the stream for the segment interface. *[quirk]* On a plain stream they write or read **nothing** and
  do not fail.

### 9.3 Worked examples

| Value | Bytes |
|---|---|
| Uint32 7 | `00 00 00 00` `03 00 00 00` `07 00 00 00` |
| Float32 6.0 (sample save) | `00 00 00 00` `09 00 00 00` `00 00 C0 40` |
| Sint64 50 (sample save) | `00 00 00 00` `08 00 00 00` `32 00 00 00 00 00 00 00` |
| Bool true | `00 00 00 00` `0B 00 00 00` `01` |
| Uint16 array {0x5C, 0x253} | `00 00 00 00` `02 80 00 00` `02 00 00 00` `5C 00 53 02` |
| Float32 array {100.0} (sample save) | `00 00 00 00` `09 80 00 00` `01 00 00 00` `00 00 C8 42` |
| string "Abc" | `00 00 00 00` `0C 80 00 00` `03 00 00 00` `41 62 63` |
| Empty | `00 00 00 00` `00 00 00 00` |
| IGZUnknown, null | `AC E9 BE A9` `00 00 00 00` |
| IGZUnknown, object ID 5 of class `0x8990C12D` | `AC E9 BE A9` `05 00 00 00` `2D C1 90 89` |

## 10. Properties and property holders in saves

### 10.1 Property (`cISCProperty`) *[compat]*

A property is a 32-bit **property ID** plus a variant value (§9). `Write(stream)`:

```
uint32   propertyID
variant  value         (§9.2)
```

`Read` reads the same fields into the property. Both return "the stream is not in the error state" (§2.1).

### 10.2 Plain holder (`cISCPropertyHolder`, the "property holder record") *[compat]*

In memory a holder keeps its properties **sorted by ascending unsigned property ID**, at most one per ID.
Adding a property whose ID is already present **replaces** that property in place. Adding never fails. The
`bool` argument of the SDK's `AddProperty(cISCProperty*, bool)` has no effect.

```
uint16  version = 2
uint32  count
count × {
    uint32    key          the ID under which the holder files the property (= its propertyID in every sample)
    property  (§10.1)      uint32 propertyID; variant
}
```

- **Write**: entries in the holder's order (ascending ID). The property ID therefore appears **twice** in each
  entry. The result is "stream not in error".
- **Read**:
  1. Read `version`. If it is not 2, return false. The stream is **not** put into the error state, and
     only the 2 version bytes have been consumed.
  2. Read `count`. **Remove all** existing properties.
  3. Repeat `count` times: read `key` (it is **ignored**), then read a property into a **new** property whose
     value starts Empty. If the property read succeeded, its value is not Empty, and it is not an `IGZUnknown`
     value holding a non-null object, add it under its **own** `propertyID`. A duplicate ID in the stream
     therefore replaces the earlier entry.
  4. Return "stream not in error". There is no upper limit on `count`. After a failed property read every later
     read also fails (§2.1), so the loop ends quickly in effect.
- *[quirk]* **Every** object-valued property is written (§9.2) but **dropped** on read *[compat]*:
  - with a non-null object, the value reads back as `IGZUnknown` holding the object, and the rule above drops it;
  - with a null object (marker plus null reference), the variant stays Empty (§9.2), and the "not Empty" rule
    drops it.

  A holder therefore **never** comes out of a load with an `IGZUnknown` property. The holder's test "not an
  `IGZUnknown` with a non-null object" can only matter for variants that are already `IGZUnknown`/null before the
  read, and no reader produces those. On save, both kinds are written (marker `0xA9BEE9AC` + reference, null =
  reference 0). No sample save contains either kind. (Corrects round 1, which said null-object entries survive.)
- *[choice]* FreeCity4 writers use `key = propertyID`. Standalone readers may reject `count` values that cannot fit
  in the remaining record.

### 10.3 Exemplar holder (`cISCExemplarPropertyHolder`) *[compat]*

An exemplar holder is a plain holder with an optional **parent exemplar** (the "default exemplar"). `HasProperty`
and `GetProperty` look at the holder's **own** properties first and then at the parent's. Only the own
properties are saved: inherited values are **not** copied into the save, only the parent's key is.

```
uint16  version = 2
...     plain holder record (§10.2)
uint32  group          parent exemplar key, in the order GROUP, TYPE, INSTANCE
uint32  type           (note: not the type, group, instance order of §2.5)
uint32  instance       all three zero = no parent
```

- **Read**: `version` must be 2 and the plain holder's `Read` must succeed. Otherwise return false (nothing after
  the failing part is read). Read the key. If all three fields are zero, the parent becomes none. Otherwise the
  holder asks the resource manager for the resource with that key, requesting `cISCResExemplar` (`0xA52160F5`).
  If it is found, it becomes the parent. If it is missing, the parent is **left unchanged** (none for a freshly
  constructed holder) and the read still succeeds. Return "stream not in error".
- Consequence *[compat]*: the parent is resolved against the **current** data files and plugins at load time.
  Changing an exemplar changes the inherited values of every saved object that refers to it. Removing it leaves
  the saved object with its own properties only.
- **Write side extra** *[inferred]*: if the parent has a non-zero property `0x6A871B82` (the same property the
  game's missing-plugin reporting checks, believed to be a plugin-pack ID), the writer also stores the parent
  exemplar itself as a resource in the save package, under the parent's own key. The city package is a resource
  source while the city is loaded, so the saved copy resolves even without the plugin. No sample save contains
  one. *[choice]* FreeCity4 writes the same in standalone mode once exemplar writing exists, and never in plug-in
  mode (the original writer does it there).

### 10.4 Which records use which holder

| Record | Holder | Spec |
|---|---|---|
| ordinance | exemplar holder | ordinance §9.3 |
| neighbour connection, neighbour deal (in the city's neighbours objects and in regional-city records) | plain holder | neighbors §4.3, §5.6; §11 |
| occupants, lots and many other saved objects | one of the two | their own specs |

The budget's locally funded objects refer to holders through `ref` (§2.7), not inline.

### 10.5 Duplicates, order and the parent *[compat]*

- A writer emits at most one entry per ID, in ascending unsigned ID order. A reader tolerates duplicates (the last
  one wins) and any order (it re-sorts).
- Saving loses nothing that a lookup can observe, except dropped object-valued properties (§10.2) and parent
  values that change in the data files (§10.3).

### 10.6 Worked examples

(a) **Ordinance holder without own properties** (sample save, Commuter Shuttle). The parent is group
`0xA9C2C209`, type `0x6534284A` (exemplar), instance `0xA2BF1DE5`:

```
02 00                    exemplar-holder version 2
02 00 00 00 00 00        plain holder: version 2, count 0
09 C2 C2 A9 4A 28 34 65 E5 1D BF A2
```

(b) **Neighbour-connection holder** (sample save, 8 properties). The first and the last entries:

```
02 00 08 00 00 00
00 41 0B 8A  00 41 0B 8A  00 00 00 00  02 80 00 00  0C 00 00 00  (12 × uint16: 00 00 … 5C 00 00 00 00 00 53 02)
…            (IDs 0xAA9EA81A … 0xAA9EA81F: Uint32 arrays of 7 zeros, tag 03 80 00 00)
77 70 94 CA  77 70 94 CA  00 00 00 00  09 00 00 00  00 00 C0 40        (0xCA947077 = Float32 6.0)
```

(c) **Synthetic** plain holder {`0x00000010` = Uint8 41, `0x80000001` = Float32 1.5}. The order is by unsigned ID:

```
02 00 02 00 00 00
10 00 00 00 10 00 00 00 00 00 00 00 01 00 00 00 29
01 00 00 80 01 00 00 80 00 00 00 00 09 00 00 00 00 00 C0 3F
```

The same properties as an exemplar holder with the parent of (a) add `02 00` in front and the 12 key bytes of (a)
at the end (57 bytes in total).

## 11. Regional-city record

### 11.1 Where it lives *[compat]*

The region stores **nothing per city outside the city files**. A region folder holds `region.ini`, `config.bmp`
and one `.sc4` file per city tile. Each city file contains one **regional-city record**, type `0xCA027EDB`,
group `0xCA027EE1`, instance 0. It is opened with the auto-stream helper, so its payload starts with
`uint16 1` (§2.10). The region's regional-city objects (`cISC4RegionalCity`) are created with
`GZCOM()->GetClassObject(0x6A037C24, 0x69D2BE1D)` (the IID is the one the reader requests; the SDK names no IID).
If that fails, they are constructed directly. Their `Read` is then called on this record. The objects are
built from these records:

- when the region loads, from every city file in the folder;
- by `cISC4Region::ReloadCity`, which re-reads one city's file (the neighbours simulator does this for every
  adjacent established city in its `Init`, neighbors §14.1);
- by moves in the region view.

It is written:

1. while the current city is saved: into the new package, right after the root record (§3.7 step 3);
2. after a **successful** save of the current city: into the existing file of **every adjacent city**,
   established or not, that has a save path. Only this record is replaced in the neighbour's file (§3.7 step 7).
   This is how deals and connections reach the neighbours' files;
3. when a city is moved in the region view.

### 11.2 Layout (version 13) *[compat]*

All values are little-endian. `str` is §2.3, `f32`/`f64` are IEEE.

```
u16  1                            auto-stream helper version
u16  version = 13
s32  posX, posZ                   region cell of the city's corner (GetPosition)
s32  sizeX, sizeZ                 size in region cells (GetCitySize)
s32  population, commercialJobs, industrialJobs
f32  workforcePercentage          a fraction (0.48 = 48 %)
s8   mayorRating
u8   difficulty
u8   moved                        set when the city is moved in the region; cleared by its next TerrainReconciliation (§11.3)
u32  serialNumber
s32  originalLanguage, originalCountry, lastLanguage, lastCountry
u32  birthDate
u8   established
str  cityName, s1, mayorName, s2, utilityAdvisorName, description, s3      (s1..s3: unknown, empty in samples)
f32  worldLongitude, worldLatitude, worldBaseElevation
s32  tutorialID                   −1 = none
u32  n;  n × { u32 populationType; s32 value }     population by type (ascending type)
u32  n;  n × { u32 populationType; s32 value }     extrapolated population
u32  n;  n × { u32 populationType; s32 value }     potential population
u32  12; 12 × f32                 tax rates (percent)
u32  n;  n × connection record    (neighbors §4.3, including its plain holder §10.2)
u32  n;  n × { u32 a; s32 x; s32 z; u8 flag }       pending connection changes (meaning partly unknown)
f64  budget
f64  income
s32  n;  n × { s32 commodity; f64 produced; f64 demanded; f64 imported; f64 exported; f64 costPerUnit }
                                  ascending commodity: 1 power, 2 water, 3 garbage
s32  n;  n × deal record          "other" deal list O (neighbors §5.6)
s32  n;  n × deal record          "local" deal list L
u32  n;  n × { u32 poiType; u32 m; m × { f32, f32, f32 } }     points of interest: type 0 airports, 1 seaports
```

Example (sample save, the first 43 bytes): `01 00 0D 00 06 00 00 00 05 00 00 00 02 00 00 00 02 00 00 00 04 0A
00 00 29 01 00 00 8C 02 00 00 8F C2 F5 3E 33 02 00 FB CB E3 53` gives helper 1, version 13, position (6, 5),
size (2, 2), population 2564, 297 commercial jobs, 652 industrial jobs, workforce 0.48, mayor rating 51,
difficulty 2, moved 0 and serial `0x53E3CBFB`. All 85 city files of the test region parse completely under
this layout (versions 9, 10 and 13, with the version rules below). None contains a deal.

**Read** *[compat]*:

- `version ≥ 14` → fail. Older versions: without `workforcePercentage` below 10 (0.5 is used); version 10 stores
  it as a **percentage** (multiplied by 0.01 on read); without `difficulty` below 13 (2 is used); without the
  points of interest below 12.
- `difficulty` is applied only when `established` is set.
- Limits: each population map ≤ `0x20000000` entries, tax rates ≤ 12, connections, changes and commodities each
  ≤ `0x100000`. Exceeding a limit makes the read return false. Deal counts are not limited.
- `tutorialID` is passed through the tutorial registry's ID mapping.
- If the city is not established, `cityName` is replaced by the localized default name. Then the city-name and
  mayor-name string records of the same package (§3.2, groups `0x6AAEE931` / `0x6AAEE932`, current language),
  when present, override `cityName` and `mayorName`.
- Connections are read into new **original** connection objects, constructed directly.
- Deals: each deal is read into a new **original** deal object. Deals that read successfully are appended to **O**.
  This holds for **both** saved lists, the first and then the second. Every deal read gets `SetFlags(Silent, true)`.
- The save-file path is not stored. The region sets it from the file name.

### 11.3 The two deal lists *[compat]*

- **O ("other")**: deals loaded from disk that no simulator has claimed yet, plus the deals of the record's city
  with third cities.
- **L ("local")**: the deals between the record's city and the **current** city (for the current city's own
  record: all of its deals). `GetLocalDeals(out)` replaces `out` with a copy of L (references).

**Claiming** (`extract(src, dst, removeClaimed)`): does nothing unless a current city with a neighbours
simulator exists. Let `cur` be the current city's regional record and `this` the record being updated. For each
deal `d` of `src`, in list order:

```
A := region city at d's city-side region cell;  B := region city at d's other-side region cell
if A or B is missing:   remove d from src (always, whatever removeClaimed says)
else if (this = cur and (A = cur or B = cur)) or (this ≠ cur and {A, B} = {cur, this}):
    append d to dst (a reference, AddRef);  if removeClaimed: remove d from src
```

| Method | Behaviour |
|---|---|
| `UpdateLocalDeals()` | `extract(O, L, true)`. L is **not** cleared first. Then `SetFlags(Silent, false)` on every deal that is **still in O** (third-party deals become visible to `UpdateImportExport`). *[inferred]* Under neighbors §5.4, clearing Silent may post a status report for those deals. Returns true. |
| `SetLocalDeals(list)` | `L := empty`; `extract(list, L, false)`; `UpdateImportExport()`. Returns true. L holds **references to the caller's objects**. *[quirk]* The caller's list loses every deal with a missing region city. The neighbours simulator passes **its own deal list**, so such deals disappear from the simulator. |
| `GetLocalDeals(out)` | `out := L` (copy of the references). |
| `UpdateImportExport()` | For commodities 1–3: `imported := 0`, `exported := 0`. Then for every deal of O, then of L: skip it if its other-side city is missing or it has any flag of `0xBC` (Cancelled, NoConnection, CannotAfford, NoLandfill, Silent). The commodity is E0/E1 → 1, E2/E3 → 2, E4/E5 → 3; other types are skipped. For a **buy** type (E0, E2, E4): if the other side is `this`, `exported[c] += amount`, else `imported[c] += amount`. For a **sell** type: if the other side is `this`, `imported[c] += amount`, else `exported[c] += amount`. Produced, demanded and cost per unit are not touched. |

**Messages** handled by every regional-city object *[compat]*:

| Message | Action |
|---|---|
| PostLoad `0x26C63342`; PostCityShutdown `0x26D31EC3` for the current city | Move all of L into O: repeatedly take the **last** deal of L and append it to O. L's order is therefore **reversed** at the end of O. |
| PostCityInit `0x26D31EC1`, only on the current city's record | Refresh the cached statistics from the city (`UpdateCityCache`). |
| PreSave `0x26C63343`, only on the current city's record | `UpdateCityCache`, then recompute the potential and extrapolated populations. |
| TerrainReconciliation `0xCA63D517`, only on the current city's record | Rebuild symmetric connections; when `moved` is set, existing connections are first marked for removal and incoming ones treated as new; then `moved` is cleared (details: a region spec). |

`Write` writes O, then L (§11.2).

### 11.4 Life of a deal across the region

1. **Load** of city X: the records come from disk, with every deal in O and Silent. PostLoad moves any L back
   into O.
2. **Neighbours `Init`** (neighbors §14.1): `ReloadCity` each adjacent established city; `R.UpdateLocalDeals()`
   (X's own record claims every deal involving X); for each adjacent city C: `C.UpdateLocalDeals()` (claims the
   X–C deals), then the simulator clones C's L into its own list (as seen from X).
3. **Push** (neighbors §9.2): for the adjacent established cities, then X's record: `SetLocalDeals(simulator
   list)` and/or `UpdateImportExport()`. The original visits the adjacent cities in **reverse** of the
   `GetAdjacentCities` order. That order is not observable, because each city's result depends only on the list.
   `SetLocalDeals` runs `UpdateImportExport` itself, so with both flags set the totals are computed twice
   (harmless).
4. **Save** of X: X's record is written into X's file, with L = the simulator's live deal objects, written with
   their own `Write`. Each adjacent city's record is rewritten into its file: O = its deals with third cities,
   L = its deals with X (again the simulator's objects).
5. Later, loading neighbour C: C's simulator rebuilds its X–C deals from X's record (step 2 with roles
   swapped). The copy in C's own file is used for C's import/export totals.

### 11.5 Consequences for FreeCity4

- Plug-in mode *[compat]*: the original region objects hold our deal objects (through `SetLocalDeals`) and write
  them with our `Write`. Our deals must produce neighbors §5.6 exactly and must tolerate `SetFlags` / `Clone`
  calls from region code at any time. FreeCity4 does not replace the regional-city class in plug-in mode
  *[choice]*, although the GZCOM creation path (§11.1) would allow it. It uses the region objects only through
  `cISC4RegionalCity`.
- Standalone mode: FreeCity4 implements §11.2–§11.4 itself, including the PostLoad reversal and the
  `SetLocalDeals` pruning, so that saves and the deal life cycle match.

## 12. The C runtime `rand()`

### 12.1 Algorithm *[compat]*

The executable links the multithreaded C runtime statically. `rand()` keeps a **per-thread** 32-bit state `s`.
A thread's state is set to **1** when the runtime first creates that thread's data.

```
srand(x):  s := x
rand():    s := (s × 214013 + 2531011) mod 2^32;   return (s >> 16) & 0x7FFF          (0 … 32767)
```

From state 1: 41, 18467, 6334, 26500, 19169, 15724, 11478, 29358, 26962, 24464. After `srand(12345)`: 7584,
19164, 25795.

### 12.2 Seeding *[compat]*

- **The game never calls `srand`.** The only call site is Lua's `math.randomseed(n)` (`srand(trunc(n))`), and
  `math.random` draws from the same generator. No shipped Lua script uses `math.random` or `math.randomseed`. All
  Lua resources in the shipped `.dat` files were searched. Plugin scripts may use them.
- The main thread's sequence therefore starts at state 1 when the process starts. It is **shared** by every
  caller on that thread, including non-simulation ones: radio-station playlist shuffles and idle handling,
  audio ambience and automata audio events, helicopter AI and others. The values that a subsystem gets depend on
  every earlier `rand()` call in the process: menu time, music settings, the cities played before. In practice
  they are **not reproducible** from a save. This is consistent with §6.2 (no RNG state is saved).
- Whether any `rand()` caller runs on a thread other than the simulation's is unknown (§15 question 4).
  Such a caller would have its own state, starting at 1.

### 12.3 `random_shuffle` *[compat]*

The library's `std::random_shuffle(first, last)` without a generator argument does:

```
for i in 1 .. n−1:   j := rand() mod (i + 1);   swap(a[i], a[j])
```

It makes exactly `n − 1` calls (none for `n ≤ 1`). The modulo is of non-negative values. For `n > 32768` the
later positions are never fully mixed: the original's behaviour, kept. Example: `a = [0, 1, …, 9]` from state 1
→ `[8, 1, 9, 2, 0, 5, 7, 3, 4, 6]`. The same algorithm is used on vectors and deques.

### 12.4 Random cell iterators *[compat, summary]*

The random cell iterator of weather §6.7 has two modes: in prime-step mode a reset makes one `rand()` call
(`start := rand() mod count`); in table mode, at each reset with shuffling requested, the index table is
`random_shuffle`d (§12.3). The table is refilled in natural order first whenever its size changed. A **linear**
variant of the same utility iterates indices `0 … n−1` instead of a rectangle. It uses the same prime table, caps
`n` at 130,444 in prime-step mode, and is empty for `n = 0`. Each subsystem spec that uses an iterator must say
which mode it uses and when it resets.

### 12.5 Users in the simulation (Windows build call sites)

| User | Use |
|---|---|
| power simulator (pole funding), plumbing simulator (pipe funding) | `random_shuffle` of a 32-bit list |
| prop developer (message handler) | `random_shuffle` of a 32-bit list |
| lot developer (message handler) | `random_shuffle` of an `int` deque |
| landfill zones (monthly landfill developer update) | `rand()` directly: one call per redrawn cell whose tile has all four surrounding heaps active, 0–4 per heap whose active state changes (landfill.md §4.8) |
| random cell iterator | flammability (daily), flora (init, monthly), pollution (init, daily, visual effects), power grid update, tract developer (lot listing, decline), weather (init, weekly); also obliterate-city, volcano lava, helicopters, audio ambience |
| linear iterator | construction occupants (props, crews), traffic simulator (accident check), one unnamed simulation function |

Names come from the symbol port and may be imprecise *[inferred]*.

### 12.6 FreeCity4 *[choice]*

- The game's runtime state cannot be reached through public interfaces, so FreeCity4 must not try to use it.
  The core provides a `CrtRand` generator implementing §12.1, plus `random_shuffle` per §12.3.
- Each replaced subsystem owns one `CrtRand` instance, seeded with 1 by default and overridable by the host. This
  matches a fresh process in which nothing else drew numbers. Subsystem specs must state the number of `rand()`
  calls for each operation, so that differential tests can align sequences once the observer forces the
  original's state.

## 13. Order of agents within a level (month level)

### 13.1 The original rule

The original simulator keeps the agents of each level in a **set ordered by the unsigned value of the
`cIGZMessageTarget2` pointer** passed to `AddAgent`. Adding the same pointer twice is a no-op. Delivery walks the
set in ascending order (simulator spec §5). The order is therefore fixed by heap addresses, not by the program.
FreeCity4's simulator runs agents in **registration order** (simulator spec §4).

### 13.2 Observation

Month-level (level 6) registrations, from a logged session with the FreeCity4 simulator in the test region
(registration order, and the rank of each agent's target pointer):

| Reg. | Agent | Addr. rank | Reg. | Agent | Addr. rank |
|---|---|---|---|---|---|
| 1 | terrain | 10 | 14 | lot developer | 18 |
| 2 | landfill developer | 20 | 15 | aura | 6 |
| 3 | airport/seaport developer | 21 | 16 | commercial | 25 |
| 4 | airport/seaport developer | 22 | 17 | crime | 19 |
| 5 | park manager | 7 | 18 | fire protection | 11 |
| 6 | **budget** | **4** | 19 | flora | 15 |
| 7 | **ordinance** | **24** | 20 | land value | 26 |
| 8 | power | 1 | 21 | police protection | 17 |
| 9 | plumbing | 5 | 22 | traffic | 8 |
| 10 | **neighbours** | **23** | 23 | pollution | 3 |
| 11 | demand | 16 | 24 | residential | 2 |
| 12 | building development | 14 | 25 | weather | 13 |
| 13 | tract developer | 12 | 26 | civic buildings | 9 |

In all three logged sessions the budget simulator's pointer was far below the neighbours' and the ordinance
simulator's (for example `0x0A9AAF70` against `0x17787798` and `0x17787818`). The budget is created before
both in a new and in a loaded city (city slot 20 against slots 30 and 31, §3.6).

### 13.3 Dependencies and the requirement

The budget's month agent books `GetOrdinanceMonthlyIncome/Expense` and the neighbour deal totals (budget §5.1,
§5.5). The ordinance simulator recomputes those totals in **its** month agent (ordinance §5). The neighbours
simulator activates, renews (re-prices), cancels and removes deals in **its** month agent (neighbors §7.3).

*[compat]* **Within the month level, the budget simulator's agent runs before the ordinance simulator's and the
neighbours simulator's agents.** Both the observed original order and FreeCity4's registration order give this,
because the city initialises the budget before the ordinance and neighbours simulators (§3.7) and each registers
its month agent in `Init`. Consequences, which tests must check:

- The settlement on the 1st of month M+1 books the ordinance totals computed on the 1st of month M (the
  previous run). An ordinance enacted during month M has its enactment amount folded into the totals on the 1st
  of M+1, and the budget books it on the 1st of **M+2**. The same one-month lag applies to repeal amounts and
  monthly ordinance costs.
- Neighbour deals are booked as they stood **before** this month's `simulate_deals`. That is the state left by
  the previous month-end run and by the re-runs that status reports triggered in between. A deal activated or
  renewed at this month-end is booked from the next settlement on. A deal removed at this month-end is still
  booked at this one if it had none of the excluded flags at that moment.
- A replacement budget, ordinance or neighbours simulator must register its month agent **in `Init`** (not lazily
  later), and a FreeCity4 city must keep the §3.7 `Init` order, so that registration order keeps the budget
  first.

No other ordering requirement is derived:

- **Ordinance vs neighbours**: their order differs between the original (neighbours first) and FreeCity4
  (ordinance first). Neither reads the other's month results.
- **Power, plumbing vs neighbours**: power and plumbing come first in both orders.
- *[compat]* **Landfill developer** (landfill.md §3.3): its month agent runs **after** pollution's and the
  budget's, and **before** the neighbours' and the ordinance simulator's. It feeds pollution's `landfilled` counter
  and line item 30, so the order decides whether those are seen this month or next. Evidence: in all eight logged
  level-6 registration sets (test sessions run3–run9 and the current plugin log, 24 agents each) the landfill
  developer's address rank is 19–20, pollution 3–14, budget 1–11, neighbours 21–22, ordinance 22–23. Registration
  order puts the landfill developer second, so FreeCity4 must order it explicitly (for example by registering its
  month agent after the city's other `Init` calls, or by a priority).
- What the eight logs confirm: the tail of the set is stable. Landfill developer (rank 19–20), port developers
  (20–21), neighbours (21–22) and ordinance (22–23) always come in that order, after every agent ranked 1–18;
  commercial (23–24) and land value (19–24) are at the end too. The ranks of the other 18 agents vary between
  sessions (for example pollution 3–14, budget 1–11, aura 2–18), so their mutual order stays open
  (§15 questions 5 and 7).
- Other pairs that differ between the two orders (for example residential, pollution and power before the budget
  in the original) were not analysed (§15 question 5). *[choice]* Until they are, FreeCity4 keeps registration
  order.

Day-level agents run before month-level ones on the 1st of a month (simulator spec §5); this is independent of
the order within a level.

## 14. Conformance tests for §9–§13 (must exist in `engine/conformance`)

16. **Variant encoding**: every row of §9.3 encodes to and decodes from the given bytes. Every readable tag of
    §9.2 round-trips, including arrays of length 0. A Bool byte `02` reads as true.
17. **Variant failures**: writing a `VoidPtr` emits `00 00 00 00 12 00 00 00` and then fails (sticky error).
    Reading tag `0x11` or `0x10` (scalar) fails. Marker `0x00000001` is read like 0. A null object reference
    leaves a pre-set Uint32 variant unchanged. A string variant with header `0x10000002` and bytes `41 00 E9 00`
    reads as "Aé" (§2.3).
18. **Plain holder**: example (c) of §10.6 round-trips. Adding properties in the order `0x80000001`, `0x10` writes
    them in the order `0x10`, `0x80000001`. A stream with duplicate IDs keeps the last value. An entry whose key
    differs from its property ID is filed under the property ID. Empty entries, object-valued entries and
    null-object entries (`AC E9 BE A9 00 00 00 00`) are all dropped. Version 3 returns false with only 2 bytes consumed and the stream still
    good.
19. **Exemplar holder**: example (a) of §10.6 round-trips. A stub resource manager resolves the parent, and a
    lookup of a property only the parent has succeeds. An unknown parent key still loads the own properties and
    leaves the parent unset. A key of zeros gives no parent. The key is read as group, type, instance.
20. **Regional-city record**: a synthetic version-13 record with two connections, two commodities, one deal in
    each list and two points of interest round-trips byte for byte. The header example of §11.2 decodes to the
    stated values. Version 10 reads the workforce as a percentage, version 9 has no workforce, version 12 has no
    difficulty, and version 14 fails. Both saved deal lists load into O, and every loaded deal is Silent.
21. **Deal claiming**: with region cells for cities X (current), C and D, and deals X–C, C–D, X–D and one with a
    missing cell: `C.UpdateLocalDeals()` moves X–C to L, keeps C–D in O with Silent cleared, and drops the
    broken deal. `X.SetLocalDeals(list)` keeps X–C and X–D and removes the broken deal **from the caller's
    list**. PostLoad moves L = [d1, d2] to the end of O as [d2, d1].
22. **Import/export**: a buy-power deal of amount 100 from X (city side) with C (other side): on C's record
    `exported[1] = 100`, on X's record `imported[1] = 100`. The same deal flagged Silent or Cancelled counts
    nothing. A deal of an unknown type counts nothing.
23. **`rand()`**: the sequences of §12.1. The shuffle example of §12.3 makes exactly 9 calls. A prime-step
    iterator reset makes exactly one call.
24. **Month order**: with stub budget and ordinance simulators registered in §3.7 order, an ordinance enacted on
    15 January with enactment cost 100 is charged at the 1 March settlement, not at the 1 February one. With
    registration order reversed the test detects the difference.

## 16. Interface slot order *[compat]* (overloads)

Original code calls our objects through vtable **slots**, so each interface's compiled slot order must match the
game exactly. A mismatched argument count under `__thiscall` corrupts the caller's stack.

How overloads end up in the vtable: the game was built with an MSVC-compatible compiler, which **groups overloaded
virtual methods and reverses each group**. The public SDK headers list overloads in source order. Compiled with
MSVC, they therefore produce the game's layout: it was verified in-game for every overload pair in
`cISC4Simulator` and `cISC4BudgetSimulator`. Two cautions:
- The verified tables in the interface specs describe the **compiled** layout (slot N = method). Compare against
  them, not against header text order.
- **Non-adjacent overloads** are the real risk. The compiler pulls a later overload up next to the first one, which
  shifts every method in between. An implementation should declare its interfaces with **unique method names**
  matching the spec slot tables, so the layout no longer depends on compiler grouping rules. It should also test each
  slot through the vtable with the game's argument counts.
- Verified compiled tables of framework and shared-service interfaces are in §17. They also list the places where
  the SDK **signature** itself is wrong (argument count or return type), independent of order.

## 15. Open questions for §9–§13

1. The meaning of the regional-city record's unknown strings `s1`–`s3`, of the connection-change entries, and the
   exact meaning of the twelve tax-rate slots.
2. Who keeps the regional record's `budget` and `income` current. The neighbours simulator's affordability checks
   read them, and `UpdateCityCache` does not set them.
3. Whether the original ever stores object-valued (`IGZUnknown`) or `Char` / `RZUnicodeChar` scalar properties in
   a saved holder. If it does, those values are lost or break the load (§9.2, §10.2).
4. Which thread runs the radio-station and audio callers of `rand()`. This decides whether they share the
   simulation thread's state (§12.2).
5. Month-level pairs other than budget/ordinance/neighbours whose order differs between the original's address
   order and registration order (residential, pollution and power before the budget; commercial and land value
   last). Each subsystem spec should check whether its month agent reads another one's month results.
6. Whether clearing Silent in `UpdateLocalDeals` really posts status reports (§11.3). That depends on the order of
   the check inside `SetFlags`, which a runtime capture would settle.
7. *(partly answered)* The landfill developer's position relative to pollution, budget, neighbours and ordinance
   is confirmed in all eight logged sessions (§13.3, now *[compat]*). Still open: the positions of the other
   month agents relative to each other where they vary between sessions, and whether any of those pairs exchange
   month results.

## 17. Compiled slot tables of game-framework interfaces *[compat]*

Method (applies to every table here and in the subsystem specs). Each interface's SDK header was laid out by the
MSVC rule of §16: every overload group sits at the position of its first declaration, and its members are in
reverse declaration order (rule re-checked with MSVC 2022, including a group whose members are not adjacent).
Each slot was then compared with the original Windows vtable named below. The callee's `ret N` operand gives
its stack-argument bytes (`int64`/`double` = 8, everything else 4; a by-value class return adds a hidden 4-byte
pointer). When an overload pair has the same byte count, its members were told apart by what the body does with
the argument. "Non-adjacent" marks overloads whose SDK declarations are separated by other methods: under MSVC
the later one moves up and shifts every method in between. An implementation that gives overloads unique names
(as §16 recommends) must place each one at the slot listed here.

**SDK signature errors** found by this check are listed with each interface. Calling them through the SDK
declaration corrupts the stack or returns garbage. The tables list only the overloaded and the wrong methods.
Every other slot matches the SDK in both order and argument size.

### 17.1 `cIGZVariant` (original vtable `0x00AA6788`, 127 interface slots)

| Slots | Group (compiled order) |
|---|---|
| 9/10, 12/13, 15/16, 18/19, 21/22, 24/25, 27/28, 30/31, 33/34, 36/37, 39/40, 42/43, 45/46, 48/49, 51/52 | `GetValX()` (no argument, returns the value) **then** `bool GetValX(X&)`, for X = Bool, Uint8, Sint8, Uint16, Sint16, Uint32, Sint32, Uint64, Sint64, Float32, Float64, Char, RZUnicodeChar, RZChar, VoidPtr. Example: `+0x84` = `int64 GetValSint64()`, `+0x88` = `bool GetValSint64(int64&)`, `+0x8C` = `SetValSint64` |
| 59/60 | `cIGZUnknown* GetValUnknown()`, then `bool GetValUnknown(uint32 riid, void** ppv)` |
| 62/63 … 90/91 | `AsX() const` (value), then `X& AsX()` (reference), for the 15 types in the order above |
| 92/93 … 122/123 | `RefX(X* p, uint32 count)`, then `X* RefX()`, for the 15 types and `RefVoid` |
| 124/125/126 | `RefIGZUnknown(cIGZUnknown*)`, `bool RefIGZUnknown(uint32 riid, void** ppv)`, `cIGZUnknown* RefIGZUnknown()` |

SDK errors: slot 8 is `bool CanGetVal(uint16 type) const` (one argument, always true in the original), not
`CanGetVal()`. Slot 60 takes `(riid, void**)`, not `(cIGZUnknown**)`.

### 17.2 `cISCPropertyHolder` (original vtable `0x00AA6CD8`, 22 interface slots)

| Slot | Method |
|---|---|
| 5 | `bool GetProperty(uint32 id, void* buffer, uint32& length)` |
| 6 | `bool GetProperty(uint32 id, uint32 riid, void** ppv)` |
| 7 | `bool GetProperty(uint32 id, cIGZString& value)` |
| 8 | `bool GetProperty(uint32 id, uint32& value)` |
| 9 | `cISCProperty* GetProperty(uint32 id)` |
| 10 | `bool AddProperty(uint32 id, void* data, uint32 length, bool)` |
| 11 | `bool AddProperty(uint32 id, cIGZUnknown* value, bool)`: **SDK error**, the SDK says `int32 lValue`. The original stores an object value |
| 12 | `bool AddProperty(uint32 id, cIGZString const& value, bool)`: **SDK error**, the SDK omits the `bool` (the callee pops 12 bytes) |
| 13 | `bool AddProperty(uint32 id, uint32 value, bool)` |
| 14 | `bool AddProperty(uint32 id, cIGZVariant const* value, bool)` |
| 15 | `bool AddProperty(cISCProperty* property, bool)` |
| 17 | `bool RemoveProperty(uint32 id, bool)`: **SDK error**, the `bool` is missing |
| 18 | `bool RemoveAllProperties(bool)`: **SDK error**, the `bool` is missing |
| 19 | `bool EnumProperties(bool (*filter)(cISCProperty*, void*), void (*fn)(cISCProperty*, void*), void* data) const`: **SDK error**, the SDK has 2 parameters |
| 20 | `bool EnumProperties(void (*fn)(cISCProperty*, void*), void* data) const` |

The `bool` arguments' meaning is not established. The callers seen pass 0 (false). The exemplar's property-holder
vtable (`0x00AAF038`) has the same layout.

### 17.3 `cIGZString` (cRZString vtable `0x00A80810`, 28 slots)

| Slot | Method |
|---|---|
| 3 / 4 | `FromChar(char const*, uint32 len)` / `FromChar(char const*)` |
| 8 / 9 / 10 | `IsEqual(char const*, uint32 len, bool cs)` / `IsEqual(cIGZString const&, bool cs)` / `IsEqual(cIGZString const*, bool cs)` |
| 11 / 12 | `CompareTo(char const*, uint32, bool)` / `CompareTo(cIGZString const&, bool)` |
| 16 / 17 | `Append(cIGZString const&)` / `Append(char const*, uint32)` |
| 18 / 19 | `Insert(uint32, cIGZString const&)` / `Insert(uint32, char const*, uint32)` |
| 20 / 21 | `Replace(uint32, cIGZString const&)` / `Replace(uint32, char const*, uint32)` |
| 23 / 24 | `Find(cIGZString const&, uint32, bool)` / `Find(char const*, uint32, bool)` (same size: told apart by the body, slot 23 calls `ToChar` on its argument) |
| 25 / 26 | `RFind(cIGZString const&, uint32, bool)` / `RFind(char const*, uint32, bool)` (same check) |
| 27 | `Sprintf(char const*, ...)`: variadic, so **cdecl**, caller cleans |

No SDK errors.

### 17.4 `cIGZDate` (cGZDate vtable `0x00ACEA38`, 46 slots; the date the simulator returns from `GetSimDate()`)

| Slot | Method |
|---|---|
| 3 | `bool Set(char const* monthName, uint32 day, uint32 year)` (compares month names) |
| 4 | `bool Set(uint32 dayNumber)` |
| 5 | `bool Set(cRZTime const&)` |
| 6 | `bool Set(uint32 month, uint32 day, uint32 year)` |
| 7 | `bool Set(uint32 day, uint32 year)` |
| 16 / 17 | `FirstDayOfMonth(uint32 month)` / `FirstDayOfMonth()` |
| 33 | `operator-(int32 days)` (**non-adjacent**) |
| 34 | `operator-(cIGZDate const&)` (**non-adjacent**) |
| 35 | `operator+(int32 days)` (**non-adjacent**) |
| 36 | `operator+(cIGZDate const&)` (**non-adjacent**) |

In the SDK the two `operator-` and the two `operator+` are interleaved (`-`, `+`, `-`, `+`). MSVC compiles them as
`-(int)`, `-(date)`, `+(int)`, `+(date)` at slots 33–36. Slots 37–40 (`++`, `--`, `+=`, `-=`) follow unchanged.

**SDK error (return type)**: slots 33–36 do **not** return `cIGZDate&`, and they do not modify the date. They
return a **uint32 day number**: `DayNumber() − n`, `DayNumber() − other.DayNumber()`, `DayNumber() + n`,
`DayNumber() + other.DayNumber()`. A FreeCity4 `cIGZDate` must return these integers. Callers must not treat the
result as a pointer.

### 17.5 `cISCResExemplar` (vtable `0x00AAF0D0`, 14 slots)

`AsISCPropertyHolder()` and `AsISCPropertyHolder() const` (slots 6/7) share one body. Their order is
unobservable. No errors.

### 17.6 `cIGZPersistResourceManager` (vtable `0x00AD89D0`, 44 slots)

| Slot | Method |
|---|---|
| 5 / 6 | `GetNewResource(uint32 type, uint32 riid, void**, uint32, cIGZUnknown*)` / `GetNewResource(cGZPersistResourceKey const&, uint32 riid, void**, uint32, cIGZUnknown*)` *(same size; by rule)* |
| 7 / 8 | `RegisterResource(cIGZPersistResource*)` / `RegisterResource(cGZPersistResourceKey const&, cIGZPersistResource&)` |
| 11 / 12 | `Save(cIGZPersistResourceKeyList*, cIGZPersistDBSegment*)` / `Save(cGZPersistResourceKey const&, cIGZPersistDBSegment*)` (checked: slot 12 calls `GetResource` with its key) |
| 21 | `FindObjectFactory(uint32 type, cIGZPersistResourceFactory**)` |
| 22 | `FindObjectFactory(cGZPersistResourceKey const& key, cIGZPersistResourceFactory**)`: **SDK error**, the SDK has a single `factory**` parameter. The original forwards `key.type` to slot 21 |
| 23 | `FindObjectFactory(cIGZPersistResource*, cIGZPersistResourceFactory**)` |
| 31 / 32 | `FindDBSegment(uint32, cIGZPersistDBSegment**)` / `FindDBSegment(cGZPersistResourceKey const&, cIGZPersistDBSegment**)` *(same size; by rule)* |
| 35 / 36 | `bool EnumerateDBSegments(callback*, cIGZPersistDBSegment*)` / `uint32 EnumerateDBSegments(cIGZPersistDBSegment**, uint32*)` *(same size; by rule)* |

`GetResource` (slot 3), which the simulation uses for exemplars and LTEXT, is not overloaded.

### 17.7 `cISC4GameDataRegistry` (vtable `0x00A8CBF8`, 29 slots)

| Slot | Method |
|---|---|
| 3 | `AddChildEntry(void* key, char const* name, callCallback (input/output parameter sets), cEntry* parent, flags, void* context)` |
| 4 | `AddChildEntry(void* key, char const* name, exchangeCallback (variant, set), cEntry* parent, flags, void* context)` |
| 5 | `AddChildEntry(cIGZUnknown* key, char const* name, cEntry* parent, flags, void* context)` |
| 6 / 7 | `RemoveChildEntry(void*)` / `RemoveChildEntry(cIGZUnknown*)` |
| 8 / 9 | `GetEntryID(void*)` / `GetEntryID(cIGZUnknown*)` |
| 24 / 25 | `AddNotificationTarget(cIGZMessageTarget2*)` / `AddNotificationTarget(cISC4GameDataRegistryNotificationTarget*)` |
| 26 / 27 | `RemoveNotificationTarget(cIGZMessageTarget2*)` / `RemoveNotificationTarget(cISC4GameDataRegistryNotificationTarget*)` |

Slots 3–9 were confirmed by the budget's own calls (budget §4.4). Slots 24–27 were placed by the rule only (same
size).

### 17.8 Shared simulation services

**`cISC4LotManager`** (zone developer's lot-manager vtable `0x00AB37C0`, 28 slots)

| Slot | Method |
|---|---|
| 5 / 6 | `GetLot(cS3DVector3 const&)` / `GetLot(int32 x, int32 z, bool)` |
| 7 | `GetLotsInCellRect(list, x1, z1, x2, z2, bool)`: not overloaded (meaning in budget Errata round 2, item 5) |
| 8 / 9 | `GetLotsSurroundingLot(int32 x, int32 z, list, int32 flags)` / `GetLotsSurroundingLot(cISC4Lot const*, list, int32 flags)` |
| 15 | `GetSafeLotIterator()` (the master lot list, budget §5.2) |
| 22 | `CreateLotIfPossible(int32 x, int32 z, int32 w, int32 h, int32, cISC4Lot*& lot, bool, bool, cISC4LotConfiguration*)`: **SDK error**, the `cISC4Lot*&` out-parameter is missing (36 bytes, not 32) |

**`cISC4Lot`** (vtable `0x00AB19B0`, 70 slots)

| Slot | Method |
|---|---|
| 50 / 51 | `GetCapacity(DeveloperType, bool forceCalculation)` / `GetCapacity(WealthType)` |
| 61 | `GetOccupancy() const`: **SDK error**, the method returns an `SC4Percentage` **by value**. MSVC passes a hidden result pointer, so the callee pops 4 bytes and writes the float through that pointer. Declare it as returning a one-float class type (or call it with a pointer argument). The SDK's `float` return is wrong |

**`cISC4BuildingDevelopmentSimulator`** (vtable `0x00AB0AF8`, 59 slots)

| Slot | Method |
|---|---|
| 6 / 7 / 8 | `AddNewBuildingOccupant(uint32, int32, int32, int32)` / `(uint32, float*, int32)` / `(cISC4BuildingOccupant*)` |
| 9 | `int64 GetCostOfOccupantDemolition(cISC4FoundationOccupant*, uint32)` (**non-adjacent**: declared after `DemolishFoundationOccupant`. **SDK error**: the SDK says `(void)`, but the method takes 8 bytes) |
| 10 | `int64 GetCostOfOccupantDemolition(cISC4BuildingOccupant*, uint32)` (non-adjacent) |
| 11 / 12 | `GetAllBuildingTypes(SC4Vector<uint32>&)` / `GetAllBuildingTypes(uint32* array, uint32& count)` |
| 18 / 19 | `GetBuildingTypesByFamily(SC4Vector<uint32>&, family)` / `GetBuildingTypesByFamily(hash set&, family)` (checked by body). The original's set is an STLport `hash_set`, **not** `std::unordered_set` as in the SDK, so never pass one |
| 23 / 24 | `AddNewFoundationOccupant(uint32, int32, float*, float, float, int, float)` / `(cISC4FoundationOccupant*)` |
| 32 / 33 | `GetDeveloper(DeveloperType)` / `GetDeveloper(PurposeType, WealthType)` |
| 37 / 38 | `GetExemplarName(type)` / `GetBuildingName(type)` (not overloaded; budget Errata round 2, item 2) |
| 42 / 43 | `GetBuildingMapKey(PurposeType, WealthType)` / `GetBuildingMapKey(uint32 type)` |

Because slot 9 moves up, everything from `GetAllBuildingTypes` to `DemolishFoundationOccupant` (header order)
sits one slot later than its header line count suggests.

**`cISC4HistoryWarehouse`** (vtable `0x00A8CCB4`, 14 slots)

| Slot | Method |
|---|---|
| 9 | `Coalesce(uint32 type, cIGZDate const& start, cIGZDate const& end, int32)`: **SDK error**, the SDK has one date (16 bytes, not 12) |
| 10 / 11 | `Dispose(uint32 type)` / `Dispose(uint32 type, cIGZDate const& start, cIGZDate const& end)` |

`RecordHistoryData` (12) and `SetHistoryDataQueueLength` (13), which the budget uses, are not overloaded.

**`cISC4DisasterManager`** (earthquake manager vtable `0x00A9E090`, 12 slots)

| Slot | Method |
|---|---|
| 6 | `cISC4DisasterInstance* AttemptDisasterStart(int32 x, int32 z)` (the budget's RiskyMoney call, `+0x18`) |
| 7 | `cISC4DisasterInstance* AttemptDisasterStart()` (returns null for the earthquake) |

For the earthquake, the two integers are world coordinates (budget §4.3), whatever the SDK's parameter names say.

## 18. Arithmetic precision (x87) *[compat]*

The Windows build does all floating-point arithmetic on the x87 FPU (no SSE). The precision-control field of the
x87 control word therefore decides how every intermediate result is rounded. This section says which setting is in
effect, on which threads, and what that means for the specs.

### 18.1 What the game sets

| When | Where | Effect on the control word |
|---|---|---|
| Process start (C runtime init, before `WinMain`) | CRT `_cinit` → `_fpmath(1)` → `_setdefaultprecision` = `_controlfp(_PC_53, _MCW_PC)` | precision **53-bit**; rounding to nearest; all exceptions masked (Windows default) |
| Application start-up, first statement of `cSC4App::PreFrameWorkInit` (`0x00452360`), main thread | the RZ framework's "set FPU precision" helper (`0x008929E0`) with mode 0 | precision **24-bit** (PC = 00); rounding and exception masks unchanged |
| Every Lua call (all `cGZLua5Thread` / `cGZLua5` wrappers, the Lua thread library) and the exemplar expression evaluator (`0x0067B920`) | the same helper, guarded by the matching "get precision" helper (`0x00892A0A`) | if the current precision is 24-bit: switch to **53-bit** for the call, then switch back to 24-bit |
| Direct3D 7 renderer | `SetCooperativeLevel` with `DDSCL_NORMAL` (windowed) or `DDSCL_FULLSCREEN \| DDSCL_EXCLUSIVE`; **neither** `DDSCL_FPUPRESERVE` nor `DDSCL_FPUSETUP` | Direct3D 7 may set the FPU to single precision itself *[inferred from the DirectX 7 documentation]*. That is the same setting the game already chose, so it changes nothing |
| Truncating conversions (`_ftol` `0x009EEF04`, and inline `fistp` sequences) | save the control word, set rounding to "chop", convert, restore | rounding control only, and only for the conversion |

The helper's modes are 0 = 24-bit, 1 = 53-bit, 2 = 64-bit. Nothing in the game calls it with mode 2, and nothing
else writes the precision field. The C library's x87 math helpers (`pow`, `exp`, `log`, …) keep the caller's
precision (they only force round-to-nearest and masked exceptions while they run).

### 18.2 Threads

- **Main thread: 24-bit precision** from `PreFrameWorkInit` on, for the rest of the session. The simulator, its
  agents (simulator.md), every grid simulator, the budget and all message handlers run on this thread, so **the
  whole simulation runs at 24-bit precision** *[inferred: the simulator is driven by the framework's idle and
  timer callbacks on the main thread, and the game creates no simulation thread]*. The only exceptions are code inside Lua calls and the exemplar
  expression evaluator (53-bit), which do not touch simulation state.
- **Worker threads** created by the game (`cRZThread`: background model loader, HTTP client, name lookup,
  file-change notification; and the sound driver's thread) start with the Windows default for new threads
  (53-bit). None of them runs simulation code *[inferred: no simulator, agent or grid code was found reachable
  from their entry points]*.

### 18.3 What 24-bit precision means

Under 24-bit precision control, every x87 `fadd`/`fsub`/`fmul`/`fdiv`/`fsqrt` (and their integer-operand forms)
rounds its **exact** result to a 24-bit significand, round-to-nearest-even. That is the same as one IEEE `float32`
operation, with three differences that matter for exact reproduction:

1. **Operands are not rounded first.** A `double` or `int32` operand loaded from memory (`fld qword`, `fild`) keeps
   its full value in the register. `x × 0.1` with a `double` constant rounds once, which can differ from
   `float32(x) × float32(0.1)`. Likewise `int32` values above 2^24 enter exactly.
2. **Exponent range.** Results keep the 15-bit x87 exponent while in registers, so intermediate overflow and
   underflow do not happen where `float32` would overflow or go denormal. Shipped values never get near those
   limits.
3. **Loads, stores, compares and conversions are not affected.** Storing a result to a `double` keeps the
   24-bit value exactly. Storing to a `float` does not round again. Comparisons and `_ftol` truncation see the
   24-bit register value.

There is no "extended precision" intermediate anywhere in the simulation. Specs that said a value "is computed in
x87 extended precision" before truncation or rounding mean, for the Windows build: **each operation is rounded
to 24 bits as above**. *[compat]* FreeCity4 reproduces this by doing every simulation arithmetic step as one
`float32` operation (rounding each step), with the operand caveat of point 1: where a spec names a `double`
constant or a large integer operand, that operand is used **unrounded** for that one operation (compute in `f64`
and round the result to `f32`; for the four basic operations this gives the correctly rounded result, because
`f64` has more than twice the significand bits of `f32`).

The Mac build (GCC, SSE or x87 at 64-bit) differs; this section applies only to the Windows build.

## Errata / clarifications (round 2)

1. **Null object values in property holders** (§9.2 vs §10.2, §14 test 18) *[compat]*. Both statements were about
   different things, and §10.2 / test 18 were wrong; they are corrected above. `ReadVariant` on a null reference
   leaves its destination unchanged, but the holder reader always reads into a **fresh, Empty** property, so the
   entry arrives Empty and is dropped by the "not Empty" rule. Non-null object entries arrive as `IGZUnknown`
   with an object and are dropped by the other rule. Result: **on load, every object-valued entry (null or not)
   is dropped; on save, both are written** (marker `0xA9BEE9AC` plus the reference, 0 for null). Loading such a
   holder and saving it again therefore removes those entries.

2. **Stream error handling** (implementer's question 7): confirmed. The game's streams expose only
   `GetError()` (compiled slot 18 of `cIGZIStream`) and no `SetError`. Serializable
   objects never put a stream into the error state themselves. A `Read` returns "`GetError() == 0` after
   reading everything", or false for a rejected version (with the stream untouched). All partial-failure
   behaviour comes from the stream's own sticky flag (§2.1, §2.9).

3. **Overload order** (§16): the deployed `FreeCity4.dll` of 14:50 (built with MSVC from SDK-order declarations)
   was compared slot by slot with the original budget vtable. Every slot with a readable `ret` matches, including
   18/19 and 27/28. The MSVC rule therefore held in practice, and the crash attributed to overload order must
   have had another cause.

4. **Variant overloads**: §17.1 shows that `cIGZVariant` follows the same rule (for example `GetValSint64(int64&)`
   is `+0x88`, not `+0x84`). It also lists two SDK signature errors there (`CanGetVal`, `GetValUnknown`).

5. **SDK signature errors** found in round 2 (details in §17 and the subsystem specs): `cISCPropertyHolder`
   (`AddProperty` ×2, `RemoveProperty`, `RemoveAllProperties`, `EnumProperties`), `cIGZVariant` (`CanGetVal`,
   `GetValUnknown`), `cIGZDate` (`operator±` return a day number), `cIGZPersistResourceManager`
   (`FindObjectFactory(key, …)`), `cISC4LotManager::CreateLotIfPossible`, `cISC4Lot::GetOccupancy` (by-value
   return), `cISC4BuildingDevelopmentSimulator::GetCostOfOccupantDemolition` (foundation overload),
   `cISC4HistoryWarehouse::Coalesce`, and `cISC4RegionalCity` (neighbors spec: setters take `double`,
   `GetWorldBaseElevation(float)` is a setter).

## Errata / clarifications (M3)

1. **Missing curve properties** (§4.1). The earlier text said that a missing property removes the curve. That is
   true only for callers that delete their own curve object (land value, aura, flammability, demand). The common
   "load by exemplar key and property ID" loader returns false and leaves the curve unchanged, so its callers end
   up with the previous, the code-default or an empty curve, depending on what they did before the load. §4.1 now
   lists the four patterns; every subsystem spec states its pattern per curve.
2. **Fire and police curves** (§4.1): they use the "default" pattern, not "keep" (protection §6). Police also
   has its own clock-seeded `cRZRandom` (§6.2).
3. **Plumbing curve, Gaussian-fast draw** (§4.1, §6.1): plumbing is not plain "default" (see the §4.1 table), and
   `RandomSint32RangeGaussianFast` is now exact.
4. **Power curves** (§4.1): power's tuning curves use the "default" pattern, not "keep" (code defaults are rebuilt
   before every load, power §8). Its per-building curves are read into a fresh temporary curve only when the
   property exists, so they behave like "delete" for each call.
5. **SDK signature errors found in M3 (power)**, each checked against the exe (callee-pop and epilogue):
   - `cISC4PowerSimulator` slots 45 / 46 `GetCurrentPlantEfficiencyPercent` / `GetCurrentPlantConditionPercent`
     (`0x006EB5F0`, `0x006EC820`): the SDK declares `float f(occupant)`. The game returns an `SC4Percentage` **by
     value through a hidden pointer** passed as the first stack argument; the callee pops 8 (pointer +
     occupant), returns that pointer in EAX and leaves nothing in ST0.
   - `cISC4PowerSimulator` slots 32 / 33 / 34 `GetOccupantPowerConsumed` / `…Produced` / `…MaxPowerProduced`
     (`0x006EB0D0` pop 8, `0x006EB420` pop 12, `0x006EB110` pop 8): declared `void`, but they return a **bool in AL**
     (upper EAX undefined).
   - `cISC4PowerSimulator` overloads compiled in reverse declaration order (§16): slot 16 is
     `GetMonthlyConsumption(x, z)` (`0x006EBA10`, pop 8) and slot 17 is `GetMonthlyConsumption()` (pop 0); slot 23
     is `GetPowerPlantCount(type)` (`0x006EFA00`, pop 4) and slot 24 is `GetPowerPlantCount()` (pop 0).
   - `cISC4NetworkTool` slot 10 `UpdateOnZoneChange`: the SDK declares no parameters. The game takes
     `(SC4Rect by value, uint32)` and the callee pops 20 (base `0x0062C900` `ret 0x14`; power-line tool override
     `0x006522F0`, pop 20).
