# Small framework owners 2: service accessors, allocators, messages, locks, log back ends, cursors (behaviour spec)

Scope: 31 small framework owners as tools/coverage.py assigns them: `RZ` (14 functions), the allocators
(`cRZFastAllocatorGeneralThreadSafe` 2, `cGZAllocatorService` 14, `cGZDefaultAllocator` 5, `cRZStackAllocator` 2),
the messages (`cRZMessage2Standard` 28, `cRZAutoMsg2Notification` 7, `cGZMouseMessage` 18), the reference-counting
bases (`cRZUnknown` 11, `cRZSystemService` 14, `cRZAutoRefCount` 18, `cRZUnknownAutoPtr` 3), threads and locks
(`cRZCriticalSection` 10, `ThreadPlatformData` 2, `cRZThread` 3, `cGZMessageQueue` 2), the log back ends
(`nGZLog::cOutputBackEnd` 6, `nGZLog::cDefaultOutputBackEnd` 5), cursors and keys (`cRZCursor` 2,
`cRZSystemCursorW95` 3, `cRZAutoCursor` 2, `cRZCursorResource` 3, `cRZCursorResourceFactory` 4, `cRZKeyboard` 2),
and resources and app bases (`cGZResourceCOMDirectorW32` 8, `cGZDBRecordBitmapRes` 3, `cGZDBRecordPackedFile` 2,
`cRZStringResource` 6, `cGZApp` 11, `cRZVersion` 5, `cRZExceptionNotification` 4): 219 functions, 9,424 bytes.

Related: foundations.md §5.1 (the standard message's behaviour, which this spec does not repeat), framework-gz-services.md
§4 (message server, system-service reference count), system-service-pointer.md (`cRZSysServPtr`), gz-framework-core.md
§7 (the log service), widget-winmgr.md (the mouse, key and canvas-event messages), persistence.md §6 and
persist-buffer-factory.md (buffer resources), output-streams.md (record streams), dates.md (the allocator lock),
application.md (`cSC4App` on top of `cGZApp`).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the decompile; the message vtable `0x00ACEC00`, the allocator
bodies `0x005B9FC0`, `0x00505190`, `0x005051C0`, `0x0090CEA3`, the lock bodies `0x00930AC0`–`0x00930BC3`, the
back-end vtables `0x00AD21B8`, `0x00AD2164`, the cursor-factory tables `0x00AD9818`–`0x00AD9844` and the
`cGZApp` tables `0x00AC3190`–`0x00AC3218` were read in the disassembly. Slot names follow the public gzcom-dll SDK
headers where the game's order matches them. Written 2026-10-24 (re-fwmisc7b).

## 1. Cached service accessors (`RZ::`) *[compat]*

Each accessor returns a framework system service, looked up once through `cRZSysServPtr` (system-service-pointer.md)
and then cached in a global. The temporary pointer's reference is released after the lookup, so the cache holds a
**borrowed** pointer: valid while the service lives, never cleared (a null result is retried on the next call).

| Accessor | Address | Cache | Service ID | IID |
|---|---|---|---|---|
| `MS` (message server) | `0x00913B97` | `0x00B628B0` | `0x64063387` | `0xC1085722` |
| `MS2` (message server 2) | `0x00913BC3` | `0x00B628B4` | `0x04FA845B` | `0x652294C7` |
| `RDD` (realtime debug display) | `0x00913BEE` | `0x00B628B8` | `0x8414A2C2` | `0x6054A2FB` |
| `GS` (graphic system) | `0x00913C1A` | `0x00B628BC` | `0xC416025C` | `0x0073283C` |
| `WM` (window manager) | `0x00913C46` | `0x00B628C0` | `0xA417445E` | `0x000005A4` |
| `WCM` (window control manager) | `0x00913C72` | `0x00B628C4` | `0xC2C2EB0F` | `0x22C2EB1F` |
| `SS` (sound system) | `0x00913C9E` | `0x00B628C8` | `0x23CECBA0` | `0x106077D2` |
| `SM` (sound manager) | `0x00913CCA` | `0x00B628CC` | `0x1AF85BFB` | `0x3AF85BB7` |
| `HM` (Hit manager) | `0x00913CF6` | `0x00B628D0` | `0x1B2CE671` | `0xBB1BD9A8` |
| `FO` (font system) | `0x00913D22` | `0x00B628D4` | `0x20673960` | `0x00673960` |
| `FS` (file system) | `0x00913D4E` | `0x00B628D8` | `0x2073215D` | `0xE0203660` |
| `LM` (language manager) | `0x00913D7A` | `0x00B628DC` | `0x441E5070` | `0x0054B7D5` |

`ADir(out)` (`0x00913DA6`) and `DDir(out)` (`0x00913DDE`) copy the C string returned by the file system's slot 4,
respectively slot 7, into `out` (the application and data directories, by FreeCity4's names *[inferred]*).

## 2. Allocators *[compat]*

### 2.1 The general fast allocator

Nine fixed-size pools for blocks of 8, 16, 32, …, 2,048 bytes (pool `i` holds `8 << i`), each with its own vtable
(`0x00AA3FD4`, `0x00A942B8`, `0x00AA3FE4`, … `0x00AA4044`) and an empty circular chunk list; a table of the nine pools
at `+0x6C`. Built by `0x005E5CB0`; the thread-safe variant (`0x005E5E30`) puts a critical section (§5.1) first and
the allocator at `+8`.

- **Allocate(n)** (`0x005B9FC0`): `total = n + 4`. The first pool whose size is at least `total` gives the block and
  its header word is `(i << 24) | total`. Above 2,048 the block comes from the C heap with header
  `total | 0x80000000`. The caller gets the address after the header.
- **Deallocate(p)** (`0x00505190`): null is ignored. The header's top byte below 9 returns the block to that pool;
  anything else frees it to the C heap.
- **Reallocate(p, n)** (`0x0090CEA3`): `n = 0` deallocates `p` and returns null. Else a new block of `n`; if `p` is
  non-null, `min(old usable size, n)` bytes are copied (usable size = the header's size field minus 4) and `p` is
  deallocated. Returns the new block.
- The thread-safe forms (`0x005E55B0`, `0x005051C0`, `0x0090CF2B`) take the lock (its slot 1) around the same bodies
  and release it (slot 2).
- `0x0090D056` frees a pool's chunk list (every chunk after the head, then the head, through the pool's slot 3).
  Destruction: `0x005E58F0` (audio.md cites it), thread-safe `0x005E5A10` (then the lock).

### 2.2 The allocator service (`cGZAllocatorService`)

A system service (service ID `0x3AE4BEA3`, priority 9,000,000, IID `0x3AE4BEAB`) owning a thread-safe fast allocator
at `+0x20`. Its allocator face: Allocate `0x009305FC` → `0x005E55B0`, Deallocate `0x00930604` → `0x005051C0`,
Reallocate `0x0093060C` → `0x0090CF2B`. `QueryInterface` (`0x0093054D`, thunks `0x009305C4`, `0x009305DC`) answers
`0x3AE4BEAB` with this, else the system service's. Constructor `0x00930630`, destructor body `0x0093059D`, deleting
destructor `0x00930614` (thunk `0x009305F4`).

Users fetch it through the framework (`GetSystemService(0x3AE4BEA3, 0x3AE4BEAB, &p)`) and may cache it: the advisor
HQ's list free `0x00413FE0` (owner `cRZAutoRefCount`) destroys a list, then returns its head node with slot 4
(Deallocate) of the service cached at `0x00B207E4`.

### 2.3 The default allocator (`cGZDefaultAllocator`, IID `0x3AE4BEAA`)

Unlocked: Allocate `0x0093057F`, Deallocate `0x00930589`, Reallocate `0x00930593` are the §2.1 bodies on the global
fast allocator at `0x00B629F8`. `QueryInterface` `0x00930528` (IIDs `0x3AE4BEAA` and 1); deleting destructor
`0x0093050B`.

### 2.4 The stack allocator (`cRZStackAllocator<T*, N, 1>::New(n)`)

An object holding `N` inline pointer slots after an 8-byte header (`+0` count, initially `0xFFFFFFFF`; `+4` heap
block). The first `New(n)` records `n`; `n < N` returns the inline slots, else a heap block of `n * 4` bytes stored at
`+4` (freed by the owner). **A later `New` returns a fresh heap block that is not recorded** *[quirk]*. Instances:
`N = 10` `0x0048EE50`, `N = 8` `0x005ECCC0`.

## 3. Messages

### 3.1 The standard message (`cRZMessage2Standard`, vtable `0x00ACEC00`) *[compat]*

Behaviour as foundations.md §5.1. The layout and slot map (SDK order, verified):

| Offset | Field | Presence bit (`+8`) |
|---|---|---|
| `+4` | reference count | |
| `+0x0C` | type | 0 (set iff the type is non-zero) |
| `+0x10` … `+0x1C` | data1 … data4 (shared with void1 … void4) | 1 … 4 |
| `+0x20` | string (a referenced object) | 6 (`0x40`) |
| `+0x24` | IGZUnknown (a referenced object) | 7 (`0x80`) |
| `+0x28` | extra (a referenced object) | 5 (`0x20`) |

| Slots | Addresses |
|---|---|
| 0–2 QueryInterface (IIDs 1, `0x65297976`, `0xA52294B4`), AddRef, Release | `0x00913292`, `0x00968B60`, `0x009132C1` |
| 3 `Create(iid, out)`: a new standard message with this one's type; a failed QueryInterface destroys it | `0x009134FF` |
| 4–7 GetType, SetType, `==`, `<` (types, unsigned) | `0x009CEDE1`, `0x009132DB`, `0x0091337A`, `0x00913394` |
| 8–12, 13–17, 18–22, 23–27 per data slot: GetHas, GetData = GetVoid, SetData = SetVoid | has `0x009132F3`, `0x009132FE`, `0x0091330A`, `0x00913316`; get `0x0099BE66`, `0x009D4714`, `0x00995D93`, `0x009D8539`; set `0x00913322`, `0x00913330`, `0x0091333E`, `0x0091334C` |
| 28–30 GetHasString, GetString (QueryInterface on the string), SetString | `0x0091335A`, `0x00913438`, `0x00913452` |
| 31–33 GetHasIGZUnknown, GetIGZUnknown, SetIGZUnknown | `0x00913366`, `0x009D858F`, `0x0091346B` |
| 34–36 GetHasExtra, GetExtra (QueryInterface on the extra), SetExtra | `0x0091336E`, `0x00913484`, `0x009134A1` |
| 37 deleting destructor | `0x009134BA` (body `0x009133AB`: releases extra, IGZUnknown, string) |

The three setters of referenced objects AddRef the new object and Release the old one. audio.md §3.2 calls slot 33's
field "void1 / target"; it is the IGZUnknown reference, not a data slot.

Messages are allocated by the framework `operator new` (`0x009133DA`) and freed by `0x00913409`: the §2.1 allocator
at `0x00B627E0` under the lock `0x00B62870`. Constructor `0x009134D6`; `0x009135E8` (owner `cRZAutoMsg2Notification`)
allocates 44 bytes and constructs one. Subclass destructors: the zoning message's `0x004DCC30` (zones.md: frees its
cell region, then the base body) and its deleting form `0x004DCC10`.

### 3.2 Automatic notification lists (`cRZAutoMsg2Notification`) *[compat]*

A set of message types (`std::set<u32>`, constructor `0x00913603`, destructor `0x009135FE`).

- `AddMessageType(t)` (`0x0091360F`): inserts; true when `t` was not in the set.
- `AddMessageTypes(types, n)` (`0x00913628`): inserts each; true only when every one was new.
- `AddAllNotifications(target, server)` (`0x00913556`, command-system.md) and `RemoveAllNotifications(target, server)`
  (`0x0091359F`): a null server means the message server 2 service; for each type in ascending order, the server's
  add (slot 5), respectively remove (slot 6), of `(target, type)`.

### 3.3 Pooled input messages (`cGZMouseMessage` owner) *[compat]*

Three small message classes are recycled through free lists instead of the heap. Their deleting destructors push the
object on the class's free list; `Create(iid, out)` pops one (refilling the pool through `0x005A61B0` when empty),
resets it, and QueryInterfaces it; a failed query destroys it (back to the list) and returns false.

| Message type | IID | Vtable | Free list | Create (slot 3) | QueryInterface | Destroy | Fields (initial) |
|---|---|---|---|---|---|---|---|
| `0xFA0C6390` mouse | `0x1A0C615F` | `0x00ADA858` | `0x00B17AC8` | `0x00989B90` | `0x00989A30` | slot 13 `0x00989B20` | `+0xC` −1, `+0x10`/`+0x14` point, `+0x18`, `+0x1C` (0) |
| `0x7A104750` key | `0x1A104742` | `0x00ADA890` | `0x00B17AD8` | `0x00989C40` | `0x00989AB0` | slot 13 `0x00989B40` | `+0xC` −1, `+0x10`, `+0x14` (0) |
| `0x1A10630A` canvas event | `0xDA1062FA` | `0x00ADA82C` | `0x00B17AB8` | `0x00989CE0` | `0x00989970` | slot 10 `0x00989B00` | `+0xC` −1 |

All three answer IIDs 1 and `0xA52294B4` too; slots 4–7 are GetType, SetType, `==`, `<`. Common bodies: count at
`+4`, Release `0x00989AE0` (mouse, key) and `0x009899A0` (canvas event): a count above 1 is decremented, else the
object is destroyed; SetType `0x00989A60`;
`==` / `<` on the type (`0x009899C0`, `0x00989A70`); constructor `0x00989B60`. Mouse: `Initialize(a, &point, c, d)`
(`0x00989A00`) stores `+0xC` = a, the point, `+0x1C` = c, `+0x18` = d; `Position(out)` (`0x009899E0`). Key:
`0x00989A90` stores four fields `+0xC` … `+0x18`. Their meaning is widget-winmgr.md's dispatch.

## 4. Reference-counting bases *[compat]*

- **`cRZUnknown`** (vtable `0x00ACE638`, count at `+4` starting at 0): `QueryInterface` `0x009457C6`, `Release`
  `0x0090D997` (a count of 1 or less destroys through slot 3 and returns 0), `RemoveRef` `0x0090D981` (decrements a
  positive count, never destroys), constructor `0x0090DA1E`, destructor `0x0090D990`, deleting destructor
  `0x0090DA2B`. The same owner holds: the deleting destructor of the bare reference count (vtable `0x00ACE624`,
  `0x0090DA01`); the library stream object with vtable `0x00ACE650` (`QueryInterface` `0x0090D9AE`: IIDs `0x12E` and
  1; `Release` `0x0090D9D3`: count at `+8`, destroys through slot 18); `0x0090D9EA` (calls own slot 14, then slot 6
  with the argument); and `0x007EFEE0`, a copy-construct loop over 32-byte elements.
- **`cRZSystemService`** (framework-gz-services.md §4.1): constructor `(id, priority)` `0x0091DE0F` sets both the
  service and tick priority; `0x0091DDCF` takes `(id, priority, tickPriority)`; both start not running. Thunks
  `0x0091DD9B` (QueryInterface), `0x0091DDA3` (Release), `0x0091DDAB` (deleting destructor `0x0091DDB3`). Three
  bodies are other classes': `0x0098E790` is the graphic system's own slot 13 (service table `0x00ADB17C`;
  gz-graphics.md §5): when its boolean argument differs from the current state it changes the state byte `+0xA2`
  under the object's lock and resets the display objects around the change; `0x009CD0AA` and `0x009CCF6B` are the
  drag-and-drop manager's destructor and list clear (widget-dragdrop.md).
- **`cRZAutoRefCount<T>::operator=`**: assigning a pointer AddRefs the new one, stores it, then Releases the old one;
  assigning the same pointer does nothing (`0x00886EF0` COM form; `0x0054DB10`, `0x00997392` copy forms that also
  test self-assignment; `0x004201D0` through a virtual base). The owner also holds folded bodies of other classes:
  `0x0047AB30` (the lot-configuration chooser's member `+0x15C`), `0x00547AD0` (the fire crew's target fire `+0x290`:
  the old target's slot 12 gets −w, the new one's +w, with w = 1 when the crew state `+0x278` is 2, else 3 when the
  flag argument is set, else 2), `0x005B1C80` (a particles description's release: count `+0x204`, destroyed at 0),
  `0x00610880` (fill a range of reference pointers), `0x0064B5C0` / `0x0064CF40` (copy and erase in a vector of
  12-byte elements), `0x00683E20` (a resource key table entry view's destructor body), `0x00776F50` (move a rectangle
  so its top-left is the argument point), `0x00413FE0` (§2.2).
- **`cRZUnknownAutoPtr<cRZSparseMatrix<WinGridCellData>>`**: `Release` `0x0095A087` (count `+8`; destroys and frees
  at 1 or less); destructor `0x0095A0AA`: when it owns (`+0xC`) the matrix `+4`, destroys and frees it (the matrix's
  map free `0x00959F09`).

## 5. Threads and locks

### 5.1 The critical section (`cRZCriticalSection`, vtable `0x00AD1EF4`) *[compat]*

Owns a heap block (28 bytes: a Win32 `CRITICAL_SECTION` and a nesting count at `+0x18`), constructed by `0x00930B91`
(block zeroed by `0x00930AA8`). Slots: 0 Release `0x00930B20` (destroys), 1 Lock `0x00930B35` (enters, count + 1),
2 Unlock `0x00930B6D` (count − 1, leaves, returns the count), 3 TryLock `0x00930B48`, 4 IsValid `0x009DAA18`
(always true), 5 IsLocked `0x00930B2E` (the count), 6 deleting destructor `0x00930BC3` (body `0x00930B02`: deletes
the section, frees the block), 7 `GetCriticalSectionHandle(out)` `0x00930B82`.

**TryLock**: `TryEnterCriticalSection` is looked up at startup (`0x00930AC0`, skipped on Windows 9x) into
`0x00B62A88`. When present and it fails, TryLock returns 0. Otherwise TryLock **enters again** (blocking), adds 1 to
the count and returns it, so a successful TryLock owns the section twice for one count *[quirk: one Unlock leaves it
still owned]*; without the import it is a blocking Lock. *[choice]* a replacement enters once.

### 5.2 Threads and the message queue

- `ThreadPlatformData` (48 bytes, constructor `0x00920755`): handle/ID fields `+0`…`+8` zero, `+0xC` the marker
  `0x12345678`, a name string `+0x10`, count `+0x24`, a critical section `+0x28`. `AddRef` (`0x009204CB`) increments
  the count under that section and returns it.
- `cRZThread` (vtable `0x00ACFB90`): the constructor (`0x009207A4`) allocates and AddRefs the platform data (an
  allocation failure is not tested before AddRef *[quirk]*). `GetCurrentThreadId(out)` `0x009206F6`; sleep
  `0x009205A0` (framework-rz-files.md, microseconds).
- `cGZMessageQueue` (vtable `0x00AD1C60`, constructor `0x0092F7CD`): an empty list (head made by `0x0092F04B`), a
  critical section `+8`, a thread signal `+0x10` (initially clear), `+0x18` = 0.

## 6. Log back ends (`nGZLog`) *[compat]*

Back ends receive the log service's output (gz-framework-core.md §7). Base `cOutputBackEnd` (vtable `0x00AD21B8`;
constructor `0x0093574F`): a map from group name to level (`+4`), count `+0x10`, default level `+0x14` =
`0x7FFFFFFF`, stream `+0x18` (null), name `+0x1C`. Slots: 0 QueryInterface (IIDs 1, `0x49B84AFB`), 2 Release, 3
accepts `(group, level)` when `level <` the group's level (slot 5: its map entry, else the default), 6
`DumpOutputLevels(stream)`, 7 output (pure in the base), 8 `SetName` (`0x00935717`). Destructor body `0x009356F1`
(name, stream release, map `0x009345F8`), deleting `0x0093577B`.

`DumpOutputLevels` (`0x009355DA`) writes, with the log service's level descriptions:

```
Backend <name>:\n
-------------\n
All unspecified groups showing <default level>\n
Group '<group>' showing <level>\n        (one line per group, in name order)
-------------\n
```

The default back end (`cDefaultOutputBackEnd`, vtable `0x00AD2164`):

- Default constructor `0x009352E7`: the framework's standard output stream (framework slot 31), the name
  `BackEnd<n>` from a counter starting at 0 (`0x00B63314`), no timestamp, flush interval 1. The named constructor
  `0x00935361` (gz-framework-core.md §7.3) copies the template `00:00:00 ` into the prefix when its timestamp flag is
  set. Deleting destructor `0x0093533F`.
- Output (`0x009353CC`, slot 7, `(group, level, text)`): nothing without a stream. With the timestamp, the prefix's
  digits are rewritten with the hour, minute and second (two digits each by `0x009352C0`). The line is
  `prefix + group + ": " + text`, written as one string when shorter than 3,968 bytes, else as four writes. The level
  is not printed. A counter is incremented and the stream is flushed when its previous value is a multiple of the
  flush interval (every line by default).

## 7. Cursors and keys *[compat]*

- `cRZCursor` (vtable `0x00AC52D4`, 48 bytes; constructor `0x0089229E`): count `+4`, a byte `+8` from the graphic
  system's slot 32 (when a framework exists; the system is kept at `+0xC`), a name `+0x10`, hot spot and size fields
  `+0x24`…`+0x2A` zero, handle `+0x2C` null. Destructor `0x00892911` (cited by other specs).
- `cRZSystemCursorW95(id)` (`0x00892694`, vtable `0x00AC5310`): loads a Windows system cursor: 1 arrow, 2 cross,
  3 I-beam, 5 no, 6 size-all, 7 size NE-SW, 8 size N-S, 9 size NW-SE, 10 size W-E, 11 up arrow, 15 wait, 16 help,
  17 hand; any other ID leaves no cursor. `SetHandle` `0x00892762`; the deleting destructor (`0x0089276C`) drops the
  handle without destroying it (system cursors are shared).
- `cRZAutoCursor(id)` (`0x00891D9A`): asks the window manager's cursor manager (window manager slot 32) to push the
  cursor (slot 12) and remembers whether it did; the destructor (`0x00891DCA`) pops it (slot 13) only then.
- `cRZKeyboard::IsKeyDownNowStatic(vk)` (`0x00891AE4`): the high bit of `GetAsyncKeyState`, with the left button
  (`vk` 1) read as the right one when the mouse buttons are swapped. `IsToggleKeySetStatic(vk)` (`0x00891B85`): the
  low bit of `GetKeyState`.
- The cursor resource (type `0x686AA4B0`, interface `0x88696706` at `+0x18`, the cursor at `+0x1C`):
  `QueryInterface` (`0x0097DC7C`, thunk `0x0097DD6D`) asks the cursor first, then answers `0x88696706` (false for a
  null out pointer), then the persistent resource's.
- The cursor resource factory (interface `0x08696779` at `+0x10`; constructor `0x0097E214`): `QueryInterface`
  `0x0097DCF6` (thunk `0x0097E234`); `CreateInstance(type, iid, out)` (`0x0097E23C`) builds a 32-byte cursor
  resource for type `0x686AA4B0` only; `SerialRead(resource, record)` (`0x0097DD25`) reads into a resource of that
  type through its `0x88696706` face; the face's slot 3 (`0x0097E292`) reads a cursor image from a record into a
  temporary resource (the buffer-resource reader `0x0097DDAB`) and on success hands its object out as IID
  `0x834C471E`.

## 8. Resource director, records, app bases *[compat]*

- `cGZResourceCOMDirectorW32` (director ID `0xA431A334`; constructor `0x00974FD4`, `GetDirectorID` `0x00975031`)
  registers four classes: `0x656DDC30` the persist buffer-resource factory (`0x00974F87`; 28 bytes, constructor
  `0x0097806D`, persist-buffer-factory.md), `0x22B9DA43` the buffer resource factory (`0x00974F71`, persistence.md
  §7), `0x286A98A8` a cursor (`0x00974FA8`), `0xC8696797` the cursor resource factory (`0x00974FBE`).
- `cGZDBRecordBitmapRes`: `GetSize(out)` `0x0097CD36` (the record's slot 52; true); destructor `0x0097CD49` frees
  its buffer `+0x40`, then the serial record. The constructor `0x0097CCFF` is persistence.md's.
- `cGZDBRecordPackedFile`: constructor `0x0096D859` (`(segment, …)`: the serial record base, the segment at `+0x40`);
  `0x0096F33D` is the write record's destructor: an open spill file (`+0x54`) is closed and deleted from disk, the
  file object destroyed; the buffer `+0x44` freed; then the serial record.
- `cRZStringResource` thunks: AddRef `0x0098416A`, Release `0x00984172`, QueryInterface `0x0098417A`, deleting
  destructor `0x00984192` (audio.md: `0x0098407D`, `0x009841D0`).
- `cGZApp` (constructor `0x0087B144`: a system service with ID `0x66` and priority −2,000,000, the app name at `+0x20`,
  the app face at `+0x1C`): `FrameWork` `0x0087B10B` (the framework); `AddApplicationService(s)` `0x0087B110` (the
  framework's AddSystemService); `AsIGZSystemService` `0x0044B490` (face − `0x1C`); thunks `0x0087B11C`, `0x0087B134`
  (QueryInterface), `0x0087B124`, `0x0087B13C` (Release), `0x0087B12C` (deleting destructor `0x0087B1A7`, body
  `0x0087B180`).
- `cRZVersion` (constructor `0x008915C1`: the current application path, empty version data; destructor `0x008915EC`
  frees the data, deleting `0x0089171E`); `GetSystemMemory` `0x0089157C` (sl-library.md); `GetUserName(out,
  default)` (`0x008919B1`): the Windows user name (up to 512 bytes) and true; on failure `default` (or `?` when null)
  and false.
- `cRZExceptionNotification` (IID `0xA4B30150`; `QueryInterface` `0x00892A2C`, `Release` `0x00892A51`, deleting
  destructor `0x00892AEB`): `Notify(module, line)` (`0x00892A68`) shows a Yes/No warning box "Exception about to be
  thrown, accept?" with `Module: <module or "not specified">\nLine: <line>` and returns true for Yes.

## 9. Function inventory

`RZ`: `0x00913B97`, `0x00913BC3`, `0x00913BEE`, `0x00913C1A`, `0x00913C46`, `0x00913C72`, `0x00913C9E`, `0x00913CCA`,
`0x00913CF6`, `0x00913D22`, `0x00913D4E`, `0x00913D7A`, `0x00913DA6`, `0x00913DDE`.
`cRZFastAllocatorGeneralThreadSafe`: `0x005E5CB0`, `0x005E5E30`.
`cGZAllocatorService`: `0x005E58F0`, `0x005E5A10`, `0x0090CF2B`, `0x0090D056`, `0x0093054D`, `0x0093059D`,
`0x009305C4`, `0x009305DC`, `0x009305F4`, `0x009305FC`, `0x00930604`, `0x0093060C`, `0x00930614`, `0x00930630`.
`cGZDefaultAllocator`: `0x0093050B`, `0x00930528`, `0x0093057F`, `0x00930589`, `0x00930593`.
`cRZStackAllocator`: `0x0048EE50`, `0x005ECCC0`.
`cRZMessage2Standard`: `0x004DCC10`, `0x004DCC30`, `0x00913292`, `0x009132C1`, `0x009132DB`, `0x009132F3`,
`0x009132FE`, `0x0091330A`, `0x00913316`, `0x00913322`, `0x00913330`, `0x0091333E`, `0x0091334C`, `0x0091335A`,
`0x00913366`, `0x0091336E`, `0x0091337A`, `0x00913394`, `0x009133DA`, `0x00913438`, `0x00913452`, `0x0091346B`,
`0x00913484`, `0x009134A1`, `0x009134BA`, `0x009134D6`, `0x009134FF`, `0x00995D93`.
`cRZAutoMsg2Notification`: `0x00913556`, `0x0091359F`, `0x009135E8`, `0x009135FE`, `0x00913603`, `0x0091360F`,
`0x00913628`.
`cGZMouseMessage`: `0x00989970`, `0x009899A0`, `0x009899C0`, `0x009899E0`, `0x00989A00`, `0x00989A30`, `0x00989A60`,
`0x00989A70`, `0x00989A90`, `0x00989AB0`, `0x00989AE0`, `0x00989B00`, `0x00989B20`, `0x00989B40`, `0x00989B60`,
`0x00989B90`, `0x00989C40`, `0x00989CE0`.
`cRZUnknown`: `0x007EFEE0`, `0x0090D981`, `0x0090D990`, `0x0090D997`, `0x0090D9AE`, `0x0090D9D3`, `0x0090D9EA`,
`0x0090DA01`, `0x0090DA1E`, `0x0090DA2B`, `0x009457C6`.
`cRZSystemService`: `0x0091DD67`, `0x0091DD89`, `0x0091DD9B`, `0x0091DDA3`, `0x0091DDAB`, `0x0091DDB3`, `0x0091DDCF`,
`0x0091DE0F`, `0x009297A0`, `0x00929A31`, `0x00932B83`, `0x0098E790`, `0x009CCF6B`, `0x009CD0AA`.
`cRZAutoRefCount`: `0x00413FE0`, `0x004201D0`, `0x0047AB30`, `0x0048EEA0`, `0x005447B0`, `0x00547AD0`, `0x0054DB10`,
`0x00562DA0`, `0x005B1C80`, `0x005EA080`, `0x00610880`, `0x0064B5C0`, `0x0064CF40`, `0x00683E20`, `0x00769400`,
`0x00776F50`, `0x00886EF0`, `0x00997392`.
`cRZUnknownAutoPtr`: `0x00959F09`, `0x0095A087`, `0x0095A0AA`.
`cRZCriticalSection`: `0x00930AA8`, `0x00930B02`, `0x00930B20`, `0x00930B2E`, `0x00930B48`, `0x00930B6D`,
`0x00930B82`, `0x00930B91`, `0x00930BC3`, `0x009DAA18`.
`ThreadPlatformData`: `0x009204CB`, `0x00920755`. `cRZThread`: `0x009205A0`, `0x009206F6`, `0x009207A4`.
`cGZMessageQueue`: `0x0092F04B`, `0x0092F7CD`.
`nGZLog::cOutputBackEnd`: `0x009345F8`, `0x009355DA`, `0x009356F1`, `0x00935717`, `0x0093574F`, `0x0093577B`.
`nGZLog::cDefaultOutputBackEnd`: `0x009352C0`, `0x009352E7`, `0x0093533F`, `0x00935361`, `0x009353CC`.
`cRZCursor`: `0x0089229E`, `0x00892911`. `cRZSystemCursorW95`: `0x00892694`, `0x00892762`, `0x0089276C`.
`cRZAutoCursor`: `0x00891D9A`, `0x00891DCA`. `cRZKeyboard`: `0x00891AE4`, `0x00891B85`.
`cRZCursorResource`: `0x0097DC7C`, `0x0097DD6D`, `0x0097E292`.
`cRZCursorResourceFactory`: `0x0097DCF6`, `0x0097DD25`, `0x0097E234`, `0x0097E23C`.
`cGZResourceCOMDirectorW32`: `0x00974F71`, `0x00974F87`, `0x00974FA8`, `0x00974FBE`, `0x00974FD4`, `0x00975031`,
`0x0097806D`, `0x0097E214`.
`cGZDBRecordBitmapRes`: `0x0097CCFF`, `0x0097CD36`, `0x0097CD49`. `cGZDBRecordPackedFile`: `0x0096D859`,
`0x0096F33D`.
`cRZStringResource`: `0x0098407D`, `0x0098416A`, `0x00984172`, `0x0098417A`, `0x00984192`, `0x009841D0`.
`cGZApp`: `0x0044B490`, `0x0087B10B`, `0x0087B110`, `0x0087B11C`, `0x0087B124`, `0x0087B12C`, `0x0087B134`,
`0x0087B13C`, `0x0087B144`, `0x0087B180`, `0x0087B1A7`.
`cRZVersion`: `0x0089157C`, `0x008915C1`, `0x008915EC`, `0x0089171E`, `0x008919B1`.
`cRZExceptionNotification`: `0x00892A2C`, `0x00892A51`, `0x00892A68`, `0x00892AEB`.
