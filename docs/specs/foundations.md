# Spec: foundations (streams, save file, response curves, messages, random numbers)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and sample saves. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* marks a finding that is consistent with all evidence but not proven; *[quirk]* marks original
behaviour that is surprising and documented so that nobody "fixes" it by accident.

Other specs refer to this one for the meaning of `str`, `ref`, `reskey`, "response curve", "standard message"
and the random number generators.

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
| `WriteVariant` / `ReadVariant` | varies | §2.8 (partly known) |

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

### 2.8 Variants *[inferred, partial]*

`WriteVariant` writes a `uint32` type tag (bit 15 set = array) and then the value. Arrays are a `uint32` count
followed by the elements, each in its primitive encoding. The mapping from variant types to tags is not yet
documented (open question 3). No simulator record specified so far uses variants directly.

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
`0x6AAEE931`/`0x6AAEE932`, region information, a record `0x89D9B9EE/0x69D9B9F1/0` written by the city) are outside
this spec.

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
   `Write`, runs later, in step 5.)*
4. Send `kSC4MessageSave` `0x26C63344` and `kSC4MessagePostSave` `0x26C63345`.
5. Run the class-object store save (§3.5): the city, the 53 components and everything they reference.
6. Close the package, then rename it over the city file.

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
- Loading returns false when there are no points. A **missing property** removes the curve. Each subsystem spec
  says what an absent curve means for it.

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

Arithmetic: on the Windows build the expression `py + (input − px) · slope` is computed in x87 extended precision
from `float32` operands and rounded once to `float32` by the caller *[inferred]*. *[choice]* FreeCity4 computes
it in `f64` and rounds to `f32` once. Tests compare with a relative tolerance of 1e-6.

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
| `RandomSint32RangeGaussianFast(lo, hi)` | the average of four `NextRaw()` values (summed with carries, rounded, divided by 4) used like `RandomUint32Uniform(hi − lo)` *[inferred]* |

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
| a `cRZRandom` member of the subsystem | wall clock (`0xFFFFFFFF`) at construction | residential, weather, flora, power, aura, fire protection, zone manager; plumbing (created at `Init`) |
| a `cRZRandom` member | **0** (integer path deterministic, double path clock-seeded) | flammability |
| a file-local `cRZRandomSuper` per source file | local wall-clock seconds XOR a per-file constant | budget (the `RiskyMoney` disaster roll), crime, pollution effects, lots, construction, network lots |
| a `cRZRandomSuper` member | wall clock at `Init` | traffic simulator, traffic network map, prop developer (clock × 28) |
| a process-wide `cRZRandom` (lazy) | wall clock | neighbours deal propositions |
| a temporary `cRZRandom` | **cell coordinates** (`a · 65536 + b`) | lot developer desirability test: deterministic per cell *[inferred: which coordinate is `a`]* |
| a temporary `cRZRandom` | wall clock | civic building monthly simulation |
| the C runtime `rand()` | unknown (open question 5) | `std::random_shuffle` of power poles, pipes, props and building lists; landfill tiles |

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
3. **Variant tags** (§2.8): the type-tag table for `WriteVariant` and the records that use it.
4. **Other city-file records** (§3.2): `0x89D9B9EE/0x69D9B9F1`, region, thumbnail and terrain records, and
   unnamed message `0x26C63346`. Not needed for simulator replacement, but needed for standalone mode.
5. **C runtime `rand()`** seeding: whether the game calls `srand`, and with what. This affects the shuffles in §6.2.
6. **RNG vectors** (§7 items 13–14): confirm with a runtime capture from the original (force a seed and log the
   first outputs).
7. **Object `Init` order on load** (§3.7): confirmed only from the city's `Init` list. Which `Init`-time
   cross-subsystem calls depend on it is for each subsystem spec.
