# cSC4PropOccupant (behaviour spec)

Status: prop-occupant spec, written 2026-10-16 (re-answers27 lane). Every fact was checked on the Windows
executable `bin/SimCity4.exe` (1.1.641, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`); every address is a Windows address.
Numeric details (float or double, operation order, widths) were checked against the instruction bytes.
Implementers work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags: *[compat]* is required for compatibility with the original game and its plugins; *[choice]* is a behaviour
we chose where the original was arbitrary or undefined; *[inferred]* is consistent with all evidence but not
proven; *[quirk]* is surprising original behaviour that is kept on purpose.

`r24(e)` is `e` rounded to float32 (foundations §18).

**Scope.** The in-world **prop occupant**: the object the game places, moves, draws, saves and removes — as
opposed to the prop *catalogue* and *manager* (prop-lotconfig.md Part A) and the prop *developer*
(prop-developer.md), which decide **which** prop to place. This spec gives the class identity, the object
layout, the complete interface slot tables **with return channels**, the spatial record, the drawing and
visual-effect attach, and the save record.

**What is not here:** the prop catalogue build (prop-lotconfig.md §A3), `CreateProp` / `AddLotProp` / `AddStreetProp`
callers (prop-lotconfig.md §A6–§A7, network-manager.md §8.6, lot-developer.md §15.5), the prop developer's block
update (prop-developer.md §4), the lot-configuration object record (lot-records.md §17.1 / lot-developer.md §17.1).

**Shared material cited and not repeated here:**

- [foundations.md](foundations.md) §2.3 (`gzstr`), §2.7 (references), §3.3 (the class-object store), §9.2 (variants),
  §10.2–§10.3 (plain and exemplar property holders), §18 (x87 precision).
- [lot-records.md](lot-records.md) §2.3 (the occupant base record), §10.1 (the prop save record), §10.2 (the
  visual-effect save record), §14 (references and resolution on load).
- [prop-lotconfig.md](prop-lotconfig.md) §A2 (the process-wide prop catalogue), §A6.1 (`CreateProp`), §A7 (add /
  remove), §A8 (persistence of the manager).
- [network-tools.md](network-tools.md) §13.1–§13.2 (the prop and visual-effect slots as other specs use them).
- [graphics-formats.md](graphics-formats.md) §4.2 (exemplar model keys), §3.9 (ANIM drivers).
- [occupant-manager.md](occupant-manager.md) (insertion, the spatial index, the flags byte).

---

## 0. Summary

`cSC4PropOccupant` (class ID `0x2977AA47`) is one in-world prop. It is a small object — **0x68 bytes** for the
plain case — with six interface pointers across its first `0x28` bytes and its real data in `+0x3C`…`+0x64`. It
carries a **prop type index** (`+0x5C`, a u16 into a process-wide ID table), a **bounding box**, an orientation,
a state, a time-of-day range, a random chance, a timer bitfield and a **lot tag**.

It implements `cISC4PropOccupant` (vtable `0x00AA4900`), `cIGZMessageTarget2` (`0x00AA48F0`), `cISC4Occupant`
(`0x00AA4868`), `cIGZSerializable` (`0x00AA484C`) and a `cSCExemplarPropertyHolder` (`0x00AA47E8`).

Three things in this spec correct or complete other documents and matter to implementers:

1. **The field offsets are from the object base, not from `+4`** (§2.1; network-tools.md §13.1 said `+4`).
2. **The return channels differ across the getters.** `docs/abi/interfaces.tsv` labels all 24 slots
   `eax-or-void`; 15 are unambiguous, and the two that look alike — `GetState` and `GetRandomChance` — are
   `eax` and **`al`** respectively (§4.1). `GetTimeOfDayRange` is `void`, not `st0` (§8 item 3).
3. **The interface is exactly 24 slots.** A first draft of this spec claimed the vtable ran on past 23; it does
   not (§4.2). What follows in the image is a pool/factory method table and then the burnt-building subclass's
   vtables.

Implementation size: about 400 lines for the class, its record read/write and the slot bodies, plus whatever the
drawing side needs.

---

## 1. Identity

| Item | Value |
|---|---|
| Class name string | `cSC4PropOccupant` (the ASCII literal in the exe; `re/class_id_table.tsv`) |
| Class ID | `0x2977AA47`, registered in the exe's class-id table at **`0x00B08C48`** (the next four entries are the subclasses: `0x00B08C50` burnt building, `0x00B08C58` terrain decal, `0x00B08C60` visual effect, `0x00B08C68` automata controller) |
| Occupant type u32 | `0xA823821E`, stored at `+0x2C` by the constructor (§2.2) |
| Primary interface | `cISC4PropOccupant`, IID `0xE9793A65`, vtable `0x00AA4900` at `+0x00` |
| Other IIDs answered | `0x2977AA47` (the class ID itself) → `this`; `0x452294AA` (`cIGZMessageTarget2`) → `this + 4` |
| Object size | **0x68 bytes** (prop-lotconfig.md §A6.1 step 3) |
| Constructor | `0x005EE050` (§2.2) |
| `cIGZSerializable` | vtable `0x00AA484C` at `+0x0C`; `Write` `0x005ECE90`, `Read` `0x005EF360`, `GetGZCLSID` `0x005EE0E0` |
| Record version | written 6, accepted **any** (`Read` does not check the version; lot-records.md §10.1) |
| Registered with GZCOM | yes — the class store creates it by `GetClassObject(classID, cIGZSerializable)` (lot-records.md §1) |

### 1.1 `QueryInterface` (`0x005EC960`)

`__thiscall`, one IID and one `void**`, callee pops 8, returns `al`. The body (measured):

| IID | Returns | Note |
|---|---|---|
| `0xE9793A65` (`cISC4PropOccupant`) | `this` | with a new reference |
| `0x2977AA47` (the class ID) | `this` | the class ID is accepted as an IID |
| `0x452294AA` (`cIGZMessageTarget2`) | `this + 4` | the message-target sub-object (§2.1) |
| anything else | passed to `FUN_005E89C0` | the `cSC4Occupant` base at `this + 8` (docs/abi/direct-calls.md) |

Every successful branch calls the vtable's `AddRef` (`*vtable[1]`) before returning 1.

### 1.2 Subclasses

Three prop subclasses share the record shape but differ in size and extra fields. Only the plain class and the
visual effect occur in saves.

| Class | Class ID | Size | Notes |
|---|---|---|---|
| `cSC4PropOccupant` | `0x2977AA47` | `0x68` | this spec |
| `cSC4PropOccupantTerrainDecal` | `0x2977AA48` | `0xA0` | `QueryInterface` `0x005ECB10`; primary IID `0xE982078A` or the class ID; occupant sub-object at `+0xC`; vtable `0x00AA4790` (lot-developer.md §7.2 / LD14) |
| `cSC4PropOccupantVisualEffect` | `0x2977AA49` | `0x94` | §6 |
| `cSC4PropOccupantBurntBuilding` | `0xC9EFB016` | `0xD8` | `QueryInterface` answers `cISC4PropOccupant` with the **base**; occupant sub-object at `+8`; `SetBoundsFromOccupant` `0x005EEB70` (lot-developer.md §7.2) |
| `cSC4PropOccupantAutomataController` | `0x2A4017A0` | — | occurs in no save; layout not specified (lot-records.md §16 Q6) |

The three non-plain sizes are what `CreateProp` allocates when GZCOM has no registered class for the profile's
occupant class ID (prop-lotconfig.md §A6.1 step 3).

---

## 2. Object layout

### 2.1 Where the offsets are measured from *[correction]*

**All field offsets in this spec are from the object base, which is the `cISC4PropOccupant` interface pointer.**

Three independent measurements agree, all of them `__thiscall` with `ECX = this`:

| Function | Bytes | Effect |
|---|---|---|
| `GetState` `0x005ED560` | `0F B6 41 63 C3` | `movzx eax, byte [ecx+0x63]; ret` |
| `GetRandomChance` `0x005ED570` | `8A 41 62 C3` | `mov al, byte [ecx+0x62]; ret` |
| `GetPropType` `0x005ED280` | `0F B7 41 5C …` | `movzx eax, word [ecx+0x5C]; …` |

`QueryInterface` (`0x005EC960`) returns `in_ECX` unchanged for `0xE9793A65`, so `this` **is** the
`cISC4PropOccupant` pointer, and the constructor (`0x005EE050`) writes the `cISC4PropOccupant` vtable
`0x00AA4900` to `+0x00`. network-tools.md §13.1 says *"The interface pointer is the prop object +4. Field offsets
below are from that pointer."* That sentence is **wrong**: the offsets it lists (`+0x5C`, `+0x5E`–`+0x63`) are
the ones the code addresses from `ECX`, i.e. from the object base. `this + 4` is a *different* interface —
`cIGZMessageTarget2` (§2.3). network-tools.md §13.1's **offsets** stand; only the stated base is wrong.

### 2.2 Fields (constructor `0x005EE050`)

The constructor sets two temporary vtables at `+0x00`/`+0x04`, calls `FUN_005E9220`, then installs the real
vtables and zeroes the data. Measured writes, in decompiler order:

| Offset | Width | Value written | Field |
|---|---|---|---|
| `+0x00` | 4 | `0x00AA4900` | `cISC4PropOccupant` vtable |
| `+0x04` | 4 | `0x00AA48F0` | `cIGZMessageTarget2` vtable |
| `+0x08` | 4 | `0x00AA4868` | `cISC4Occupant` vtable |
| `+0x0C` | 4 | `0x00AA484C` | `cIGZSerializable` vtable |
| `+0x10` | 4 | `0x00AA47E8` | `cSCExemplarPropertyHolder` vtable |
| `+0x14` | 4 | (base init) | **reference count** |
| `+0x24` | 4 | `0x00AA47D0` | sixth sub-object (§2.4) |
| `+0x2C` | 4 | `0xA823821E` | occupant type |
| `+0x3C`…`+0x53` | 6 × f32 | 0 | **bounding box** (min x,y,z, max x,y,z), written by `SetBoundingBox` (slot 4) |
| `+0x54` | 4 | 0 | **owned sub-object pointer** (the date-range record; slots 15/16) |
| `+0x58` | 4 | `0xFFFFFFFF` | **lot tag** (none) |
| `+0x5C` | 2 | 0 | **prop type index** (u16) |
| `+0x5E` | 1 | 0 | time-of-day start (tenths of an hour) |
| `+0x5F` | 1 | 0 | time-of-day end |
| `+0x60` | 1 | 0 | **timer bits** (low nibble = a 4-flag set) |
| `+0x61` | 1 | 0 | **orientation** (slots 9/10; `SetOrientation` clamps it to 0…3) |
| `+0x62` | 1 | **100** | **random chance** (percent) |
| `+0x63` | 1 | 0 | **state** |
| `+0x64` | 1 | 0 | **prop usage** (1 city, 2 lot, 3 building, 4 street) |

The default chance of **100** and the default lot tag of **`0xFFFFFFFF`** are the two constructor values an
implementer is most likely to get wrong; both are observable in a freshly created prop.

The bounding box is **not** a constructor field — it is zeroed there and written by `SetBoundingBox` (slot 4,
`0x00458C60`) during `CreateProp` (prop-lotconfig.md §A6.1 step 5). The **reference count lives at `+0x14`**,
not at `+0x04`: `AddRef`/`Release` (slots 1/2) are this-adjusting thunks that do `add ecx, 8` before jumping to
the shared `0x005E89B0` / `0x005E8DF0`, so an implementer who refcounts `this` directly hits the wrong object.

The seven dwords the constructor zeroes at `+0x3C`…`+0x54` are therefore exactly the AABB (6 floats) plus the
sub-object pointer — a useful check that the layout is right.

### 2.3 Sub-object map

| Offset | Interface | Vtable | How it is reached |
|---|---|---|---|
| `+0x00` | `cISC4PropOccupant` | `0x00AA4900` | the object base; QI `0xE9793A65` / `0x2977AA47` |
| `+0x04` | `cIGZMessageTarget2` | `0x00AA48F0` (4 slots) | QI `0x452294AA`; slot 3 is `DoMessage` `0x005ECFB0` |
| `+0x08` | `cISC4Occupant` | `0x00AA4868` (34 slots) | `AsOccupant` (prop slot 3) returns `this + 8` |
| `+0x0C` | `cIGZSerializable` | `0x00AA484C` | `Write` / `Read` / `GetGZCLSID` |
| `+0x10` | `cSCExemplarPropertyHolder` | `0x00AA47E8` | occupant slot 6 `AsPropertyHolder` |
| `+0x24` | unnamed | `0x00AA47D0` (5 slots + a null) | not identified *[inferred: a small auxiliary interface]* |

The `cIGZMessageTarget2` vtable `0x00AA48F0` is exactly four slots — `QueryInterface` `0x005EE0F0`, `AddRef`
`0x0064DD70`, `Release` `0x0064DD80`, `DoMessage` `0x005ECFB0` — which is why the next vtable in the image,
`0x00AA4900`, starts immediately after it.

### 2.4 The sixth sub-object (`0x00AA47D0` at `+0x24`) *[inferred]*

Observed slots (read from the image): `0` `0x005EE140`, `1` `0x0064DE00`, `2` `0x0064DE10`, `3` `0x00458E60`,
`4` `0x0060CE80`, `5` **null**. Slots 0–2 are the usual `QueryInterface` / `AddRef` / `Release` thunk triple
seen on every other sub-object here. The interface identity is **not** established; nothing in `docs/`
references `0x00AA47D0`. What would settle it: reading `0x005EE140`'s IID table, or finding the SDK header for
the class.

---

## 3. Creation and destruction

Cited, not repeated in detail:

- **`CreateProp(id, prop&)`** — prop manager slot 15, `0x004A3040`, pop 8, `al` (prop-lotconfig.md §A6.1). Picks
  the occupant class from the prop profile `+0x00` (defaulting to `0x2977AA47`), asks GZCOM for `cISC4PropOccupant`
  `0xE9793A65`, falls back to one of the three built-in sizes, then `SetPropType(id)`, then attaches the exemplar
  and sizes the box. **The caller owns one reference.**
- **`CreateGenericProp(key, prop&)`** — slot 16, `0x004A24D0` (prop-lotconfig.md §A6.2): same class pick, no
  prop type, no catalogue lookup.
- **`AddLotProp` / `AddCityProp` / `AddStreetProp` / `AddBuildingProp`** — prop-lotconfig.md §A7.1. **Usage** is
  written here: 1 city, 2 lot, 3 building, 4 street (field `+0x64`-side, §2.2 `prop_usage` in the record).
- **`CreateLotProp(lot, objectIndex, propID)`** — lot developer slot 63, `0x006C6230` (lot-developer.md §15.5):
  `AddLotProp` then **`SetLotTag(obj's tag at +0x24)`** (occupant slot 28). Returns the occupant **borrowed** in
  EAX even though the SDK declares `bool` — see the return-channel table.
- **`CreateProp(lot, i, id)`** — network lot manager slot 8, `0x0060CF80` (network-manager.md §8.6): `AddStreetProp`
  then `SetLotTag` (slot 28) and, when the lot flips, `SetFlag(0x80)` (slot 29).
- **Removal** — prop manager slot 29 `RemovePropA` `0x004A27B0`, and the single-prop path `0x004A2690` (hide the
  placeable → `RemoveOccupant` → occupant `Shutdown`). Demolition.md §4.2–§4.6 does the same for type `0xA823821E`.

**On load**, the class store creates the object through `GZCOM()->GetClassObject(classID, cIGZSerializable)` and
calls `Read` (lot-records.md §1). The occupant base's `Init` (`0x005E8C90`) posts `CreateOccupant 0x99EF1140` only
for occupants **not** loaded from a save (lot-records.md §2.4), so a loaded prop does not announce itself as new.

**On save**, every prop that some other saved object still references is written into the class record
`(0x2977AA47, 0x299B2D1B, 0)` (lot-records.md §14).

---

## 4. Interface slot tables

### 4.1 `cISC4PropOccupant` (vtable `0x00AA4900`)

Compiled slots; the SDK's own index is the compiled slot minus 3 (network-tools.md §13.1). `pop` is what the
**callee** pops after `this`; `args` is the number of stack dwords beyond `this`.

**Return channel** is the part implementers get wrong. `docs/abi/interfaces.tsv` currently labels every one of
these 24 rows `eax-or-void`; **15 of the 24 are unambiguous** and two of them are `al`, not `eax`:

- `eax` — a genuine value in EAX (a pointer, a u32, or a byte the code **zero-extends** with `movzx eax`).
- `al` — only AL is defined; the upper 24 bits of EAX are **undefined**. Do not widen to a bool.
- `void` — no defined return value. `void` does **not** mean "x87 empty by accident": check both.
- `via-call` — a tail call; the value is whatever the callee returns.
- `st0` — x87 (used by **neither** of the two time-of-day slots; see slot 14).

| Slot | Off | Address | Method | pop | args | Return | Notes |
|---|---|---|---|---|---|---|---|
| 0 | `+0x0` | `0x005EC960` | `QueryInterface` | 8 | 2 | **`al`** | §1.1 |
| 1 | `+0x4` | `0x006491C0` | `AddRef` | 0 | 0 | `via-call` | thunk `add ecx,8` → `0x005E89B0` (EAX = refcount) |
| 2 | `+0x8` | `0x006491D0` | `Release` | 0 | 0 | `via-call` | thunk `add ecx,8` → `0x005E8DF0` (EAX = refcount) |
| 3 | `+0xC` | `0x006C2040` | `AsOccupant` | 0 | 0 | **`eax`** | `this + 8`, null stays null |
| 4 | `+0x10` | `0x00458C60` | `SetBoundingBox` | 8 | 2 | **`void`** | 2 args are `const float[3]*` min and max; writes the AABB at `+0x3C` |
| 5 | `+0x14` | `0x004E13E0` | (stub) | 0 | 0 | **`eax`** | `xor eax,eax; ret` — always returns 0 |
| 6 | `+0x18` | `0x00648F10` | (no-op stub) | 4 | 1 | **`void`** | `ret 4` alone; the argument is ignored |
| 7 | `+0x1C` | `0x005ED280` | `GetPropType` | 0 | 0 | **`eax`** | §5.1; `movzx eax, word [ecx+0x5C]` then `*[0x00B231A4 + idx*4]` |
| 8 | `+0x20` | `0x005EF4C0` | `SetPropType` | 4 | 1 | `eax-or-void` | calls `GetPropTypeIndex` `0x005EF2E0` (returns the interned index in AX) then `mov [esi+0x5C], ax`. EAX holds that index at return but nothing constructs it as a return value — implement as `void` unless a caller proves otherwise |
| 9 | `+0x24` | `0x005EC9D0` | `GetOrientation` | 0 | 0 | **`eax`** | `movsx` of the byte `+0x61` |
| 10 | `+0x28` | `0x005EC9E0` | `SetOrientation` | 4 | 1 | **`al`** | clamps the argument to 0…3 into `+0x61`; always returns 1 |
| 11 | `+0x2C` | `0x005ED290` | `SetState(s)` | 4 | 1 | **`void`** | network-tools.md §13.1 |
| 12 | `+0x30` | `0x005ED560` | `GetState` | 0 | 0 | **`eax`** | `movzx eax, byte [ecx+0x63]` |
| 13 | `+0x34` | `0x005ED400` | `SetTimeOfDayRange(a, b)` | 8 | 2 | **`void`** | the two args are **by-value floats**; `fld` each, `fmul 10.0`, store a byte to `+0x5E`/`+0x5F`. **x87 empty at return** |
| 14 | `+0x38` | `0x005ECA10` | `GetTimeOfDayRange(&a, &b)` | 8 | 2 | **`void`** | two `float*` out-params: `movzx` the byte, `fild`, `fmul 0.1`, `fstp` through the pointer. **ST(0) is EMPTY at return** — see §8 |
| 15 | `+0x3C` | `0x005EEFF0` | `SetSimulatorDateRange` | 16 | 4 | **`void`** | prop-developer.md §4.6; drives the 0x28-byte sub-record at `+0x54` |
| 16 | `+0x40` | `0x005EE230` | `TickSimulatorDate` | 4 | 1 | **`void`** | prop-developer.md §3.3 |
| 17 | `+0x44` | `0x007C2740` | `SetRandomChance(c)` | 4 | 1 | **`void`** | stores the byte at `+0x62` |
| 18 | `+0x48` | `0x005ED570` | `GetRandomChance` | 0 | 0 | **`al`** | `mov al, byte [ecx+0x62]` — upper EAX **undefined** |
| 19 | `+0x4C` | `0x005ED580` | `GetPropTimerBitfield` | 0 | 0 | **`eax`** | `movzx eax, byte [ecx+0x60]` |
| 20 | `+0x50` | `0x005ED590` | `GetPropTimerState(mask)` | 4 | 1 | **`eax`** | `(+0x60 & mask) != 0`, produced as a real 0/1 (`sbb`/`neg`) |
| 21 | `+0x54` | `0x005ECA50` | `SetPropTimerState(mask, on)` | 8 | 2 | **`void`** | network-tools.md §13.1 |
| 22 | `+0x58` | `0x00477120` | `GetPropUsage` | 0 | 0 | **`al`** | the byte `+0x64`; identical-code-folded with flora / landfill |
| 23 | `+0x5C` | `0x005ECAE0` | `SetPropUsage` | 4 | 1 | **`void`** | stores the byte `+0x64` |

**This is the whole interface: 24 slots, `+0x00`…`+0x5F`.** Slots 0–23 are exactly the rows in
`docs/abi/interfaces.tsv`; there are no further slots. The methods pair up over the fields: `+0x5C` (7/8),
`+0x61` (9/10), `+0x63` (11/12), `+0x5E`/`+0x5F` (13/14), the `+0x54` sub-record (15/16), `+0x62` (17/18),
`+0x60` (19/20/21), `+0x64` (22/23).

### 4.2 What is *not* part of this interface *[correction]*

An earlier draft of this spec said the vtable continued past slot 23 and called slots 24–47 "undocumented
`cISC4PropOccupant` slots". **That was wrong** and would mis-size the interface. The truth, from four
independent lines of evidence:

1. `re/labels_windows.tsv` line 322 records `00aa4900 cSC4PropOccupant::vftable 24`, paired with the Mac symbol
   `__ZTV16cSC4PropOccupant#0`. `re/abi_interfaces.tsv` agrees (`cISC4PropOccupant … 00aa4900 24`).
2. The derived class `cSC4PropOccupantBurntBuilding` has its primary vtable at `0x00AA4B30`, also **24** slots
   (`re/labels_windows.tsv` line 326), with slots 1–23 byte-identical to this one and only slot 0 overridden.
   A derived class cannot truncate a vtable.
3. Nothing stores `0x00AA4960` as a vptr. The constructor stores `0x00AA47D0`, `0x00AA47E8`, `0x00AA484C`,
   `0x00AA4868`, `0x00AA48F0`, `0x00AA4900`; the derived constructor stores from `0x00AA49B0`. `0x00AA4960` is
   written into a **global** (`mov [0x00B0A8E4], 0x00AA4960` at `0x00A7370C`).
4. The target of what an earlier draft called "slot 24" (`0x005E16E0`) reads `[this+4]` as a pool pointer and
   **writes through it**. With `this` = the `+0x00` interface that would clobber the `cIGZMessageTarget2` vptr.

So the words after `0x00AA4900` + 24 dwords are:

| Address | What it is |
|---|---|
| `0x00AA4960` (20 dwords) | a **pool / factory method table**, five groups of `[AllocNode, FreeNode, CreatePool, FreePool]`, one per node type |
| `0x00AA49B0`, `0x00AA49FC`, `0x00AA4A00`, `0x00AA4A18`, `0x00AA4A7C`, `0x00AA4A98`, `0x00AA4B20`, `0x00AA4B30` | the extra vtables of `cSC4PropOccupantBurntBuilding` |
| `0x00AA4B90` | the burnt-building vbtable `[0xFFFFFFFC, 0x68]` |

The five allocator groups are worth recording even though they are not interface slots, because they explain the
"repeating four-slot" pattern and they encode the **object sizes** of §1.2:

| Group | AllocNode | FreeNode | CreatePool (stride) | FreePool | stride = the class size |
|---|---|---|---|---|---|
| 0 | `0x005E16E0` | `0x00477660` | `0x004771D0` | `0x005E5770` | `0x68` plain prop |
| 1 | `0x005E16E0` | `0x005EEE50` | `0x005ECD10` | `0x005DEB90` | `0xA0` terrain decal |
| 2 | `0x005E16E0` | `0x005EEE90` | `0x005ECD50` | `0x005DEB90` | `0x94` visual effect |
| 3 | `0x005E16E0` | `0x005EEED0` | `0x005ECD90` | `0x005DEB90` | `0xD8` burnt building |
| 4 | `0x005E16E0` | `0x005EEF10` | `0x005ECDD0` | `0x005DEB90` | `0x88` |

`AllocNode` pops the pool's free list (`[this+4]+8`) and returns the node in **EAX**; `FreeNode` pushes it back
masked and is **void**; `CreatePool` builds a free list of N nodes of the given stride and returns the pool in
**EAX**; `FreePool` frees it and is **void**. The strides independently confirm the object sizes of §1.2 for
three of the four prop classes (the fifth, `0x88`, is not a prop class this spec covers).

### 4.3 `cIGZMessageTarget2` (vtable `0x00AA48F0`, sub-object `+0x04`)

| Slot | Address | Method | Return |
|---|---|---|---|
| 0 | `0x005EE0F0` | `QueryInterface` | `al` |
| 1 | `0x0064DD70` | `AddRef` | `via-call` |
| 2 | `0x0064DD80` | `Release` | `via-call` |
| 3 | `0x005ECFB0` | `DoMessage` | — |

`DoMessage` handles the animation tick `0x86AD10EE`: it advances each animated model instance by the message's
millisecond count (data3, float32) times the simulator's time dilation (`cISC4Simulator` slot 32)
(graphics-formats.md §3.9).

### 4.4 `cISC4Occupant` (vtable `0x00AA4868`, sub-object `+0x08`)

34 slots. The ones prop code uses (prop-developer.md §8.4, prop-lotconfig.md §A7, city-detail.md §3.2):

| Slot | Method | Address |
|---|---|---|
| 3 | `Init` | `0x005EC940` |
| 5 | `IsInitialized` | `0x00458A40` |
| 6 | `AsPropertyHolder` | `0x006C2040` (folded) |
| 8 | `GetPosition` | `0x00458BA0` |
| 9 | `SetPosition` | `0x00458BD0` |
| 11 | `GetBoundingCityCells` | (vtables.tsv) |
| 27 | `GetLotTag` | `0x005ECAC0` |
| 28 | `SetLotTag` | `0x004302C0` (folded with flora) |
| 29 | `SetFlag` | `0x005E8B70` |
| 30 | `SetAllFlags` | `0x005E8B90` |
| 31 | `ClearFlag` | `0x005E8BA0` |
| 32 | `IsFlagSet` | `0x005E8BC0` |

The **lot tag** is the field the prop developer's PV2 rule reads: `GetLotTag` (slot 27) is false when the prop has
no tag, and the tag value is `+0x58` (constructor default `0xFFFFFFFF`).

---

## 5. The spatial record

### 5.1 Prop type: an index, not an ID *[important]*

The prop does **not** store its exemplar key. It stores a **u16 index** at `+0x5C` into a process-wide table of
prop type IDs.

`GetPropType` (slot 7, `0x005ED280`) is four instructions:

```
movzx eax, word ptr [ecx+0x5C]      ; the index
mov   ecx, dword ptr [0x00B231A4]   ; the table
mov   eax, dword ptr [ecx+eax*4]    ; table[index], a u32 ID
ret
```

So `0x00B231A4` holds a **pointer to an array of u32 prop type IDs**, and the prop's identity is
`table[+0x5C]`. `GetPropTypeIndex` (`0x005EF2E0`) maps the other way and **appends** an unknown ID to the table
on read (lot-records.md §10.1 field 3), so a save that names a type the catalogue does not know still round-trips:
the type is interned on load.

Consequences an implementer must reproduce:

1. The index is **process-global**, not per-prop. Two props of the same type share an index.
2. Because `GetPropTypeIndex` appends, the table grows across a session. A save written after loading unknown
   types is still correct — the u32 type ID is what is saved (§7), not the index.
3. `0x00B21770` is a **different** count: the element count of the catalogue's **type map** (`0x00B21760` + `0x10`),
   i.e. how many prop types the catalogue holds (prop-lotconfig.md §A2, the PL2 answer). Do not confuse it with
   the ID table at `0x00B231A4`.

### 5.2 Position, orientation and terrain

- **Position** is the occupant sub-object's (occupant slot 8 `GetPosition` `0x00458BA0`). For a prop it is the
  centre of the bounding box written by `SetBoundingBox` (slot 4).
- **Bounding box** is set by `CreateProp` from the origin to the exemplar's `OccupantSize`, or to `(1, 1, 1)`
  when the prop has no size — including the PL9 raw-data-pointer size quirk (prop-lotconfig.md §A6.1 step 5).
- **Orientation** is the byte at `+0x61`, 0…3 (slots 9 `GetOrientation` / 10 `SetOrientation`; save record
  field 5). `SetOrientation` clamps its argument into 0…3, so a caller cannot store 4 or a negative value.
- **Height / terrain coupling.** A prop does not carry its own altitude in the record beyond the box. The box's
  `min.y` / `max.y` are the height extents. The terrain coupling that matters is on the **lot**: lot flags lock
  the terrain flat at the lot's altitude (lot-records.md §3.2 field 3), and the network manager's terrain rule
  destroys occupants whose corner is below the terrain or more than 0.01 above it (network-manager.md).
  *[inferred]* A prop's y is not re-solved against terrain after placement; what would settle it is a
  terraforming brush over a prop and a check that its box does not move.

### 5.3 The lot tag

`+0x58`, u32, default `0xFFFFFFFF` = none. It is the **configuration object's ID** (`cSC4LotConfigurationObject`
`+0x24`) of the object that caused the prop to be placed (lot-developer.md §6.5 `CreateLotProp`, and
network-manager.md §8.6 `CreateProp`, both call `SetLotTag` right after creating the prop). It is saved with the
prop (§7) and it is what the city detail manager's reduce (`0x0045D690`) and the prop developer's block update
(prop-developer.md §4.3 step 2.3.2) match against.

**PV2** (the unmatched-tag rule) is specified in prop-developer.md §10 quirk 1. The corpus count is now made:
**40,830 of 147,162 tagged lot props over the 421 distinct saves under `Regions\` are unmatched (27.7 %)**, so
the rule fires on more than a quarter of the tagged lot props in the corpus. The tool is `re/prop_tag_corpus.py`;
the full figures and its validation are in QUESTIONS.md under PV2. This spec only fixes *where the tag lives*
(`+0x58`, default `0xFFFFFFFF`) and how it is saved (§7).

---

## 6. Drawing and the visual-effect sub-object

### 6.1 How a prop is drawn

Cited, not repeated: graphics-formats.md §4.2 picks the model from the occupant's exemplar through
`cSC4ModelMakerUtility::GetModelInstanceID` (`0x00496830`) / `GetAllModelInstanceIDs` (`0x00496C90`), using
RKT0–RKT5 (`0x27812820`…`0x27812825`), the zoom mask `0x0ABFC024`, occupant flag `0x80` selecting `0x2781292x`,
and the state from `GetOccupantOrientationAndState` (`0x00496670`). Frames are built by `GetModelFrame`
(`0x00806C40`) and drawn by `RenderMesh` (`0x007D4A80`).

Animated instances advance on the animation tick `0x86AD10EE` in `DoMessage` (`0x005ECFB0`, §4.3), and `SetState`
also sets the frame to the state (`0x005ED3AC`).

*[gap]* The occupant → **placeable** → 3D object step is not specified anywhere in `docs/` yet: occupant slots
20/21 `GetOrCreatePlaceableObject` (`0x005E91C0`) / `SetPlaceableObject` (`0x0078DAF0`) appear only in
`docs/abi/vtables.tsv`. An implementer who must actually render needs that seam; it is listed in §10.

### 6.2 `cSC4PropOccupantVisualEffect` (`0x2977AA49`)

The visual effect is a **separate prop class**, not a member of the plain prop. Its behaviour is fully specified
in network-tools.md §13.2 (the NM5 answer) and its record in lot-records.md §10.2; both stand.

What this spec adds is the **relationship to the occupant**:

- Its `QueryInterface` (`0x005ECC10`) returns the **object base** for IIDs `0x09820792` and `0x2977AA49`;
  `AsOccupant` (slot 3) is `this + 0xC` and `AsPropOccupant` (slot 4) is `this + 4` — so the embedded prop
  occupant is at `+4` and the occupant at `+0xC` (network-tools.md §13.2).
- The effect object lives at `+0x6C`; the stored name at `+0x74`/`+0x78`; the stored effect ID at `+0x84`;
  `+0x88`/`+0x8C` are zeroed on an effect-type change; `started` is `+0x90`; the constructor `0x005F0210` sets
  `+0x91 := 1`.
- When the occupant's flag 2 changes (`SetAllFlags`, `0x005ED9D0`) the prop starts or stops its effect.
- Potholes are the live example: created through GZCOM class `0x2977AA49` with IID `0x09820792`, named
  `"pothole"` / `"pothole_rail"` (network-manager.md §8.8).
- *[quirk]* Its `Write` emits an **empty name and 0** whenever the effect is live (lot-records.md §10.2).

---

## 7. Save record

The record is specified field by field in lot-records.md §10.1 and validated over the corpus there; **that
layout is authoritative and is not repeated**. The facts an implementer of *this* class needs:

| # | Field | Type | Notes |
|---|---|---|---|
| 1 | version | u16 | 6 written |
| 2 | occupant base | | lot-records.md §2.3 (type `0xA823821E`) |
| 3 | prop type ID | u32 | the **ID**, not the `+0x5C` index (§5.1) |
| 4 | bounds | box | 6 × f32 |
| 5 | orientation | s8 | 0…3 |
| 6 | state | u8 | 0…2 |
| 7 | time of day start, end | 2 × u8 | tenths of an hour; (0, 0) = always |
| 8 | has date range | bool | |
| 9 | date range | 4 × u32 | only when #8 |
| 10 | chance | u8 | 1…100 |
| 11 | prop usage | u8 | 1, 2, 4 |
| 12 | **lot tag** | u32 | `0xFFFFFFFF` = none |
| 13 | timer bits | u8 | only when version > 5, else `0x0F` |

*[quirk]* `Read` does **not** check the version: any version is read with this layout (lot-records.md §10.1).

Storage: the class-object store, record key `(0x2977AA47, 0x299B2D1B, 0)`, a sequence of
`{ u32 entrySize, u32 crc, u32 objectID, byte[] payload }` entries (foundations §3.3). Each payload is exactly
what this `Write` produced. The CRC is CRC-32/MPEG-2 over `min(entrySize − 8, 250000)` bytes from `objectID`
(foundations §3.4).

---

## 8. Corrections to other documents

1. **network-tools.md §13.1** — *"The interface pointer is the prop object +4. Field offsets below are from that
   pointer."* Wrong base. The offsets in that table are from the **object base** (§2.1, three measured
   instructions). `this + 4` is `cIGZMessageTarget2`. The offsets themselves are right; only the sentence is.
2. **`docs/abi/interfaces.tsv` return channels** for `cISC4PropOccupant` — **every one of the 24 rows is
   labelled `eax-or-void` / `medium`**, which is the known failure mode exactly. 15 of the 24 are unambiguous.
   The rows proposed for `build-runs/pending-abi/re-answers27-rows.tsv` carry the measured channels for the
   slots other specs actually call; the full set is §4.1. The load-bearing ones:

   | Slot | Address | Row says | Correct | Evidence |
   |---|---|---|---|---|
   | 0 | `0x005EC960` | `eax-or-void` | `al` | `mov al, 1; ret` on each success path |
   | 3 | `0x006C2040` | `eax-or-void` | `eax` | returns `this + 8` |
   | 5 | `0x004E13E0` | `eax-or-void` | `eax` | `33 C0 C3` = `xor eax, eax; ret` — a real 0 |
   | 6 | `0x00648F10` | `eax-or-void` | `void` | `ret 4` alone; a true no-op |
   | 7 | `0x005ED280` | `eax-or-void` | `eax` | `movzx eax, word [ecx+0x5C]; … mov eax, [ecx+eax*4]; ret` |
   | 9 | `0x005EC9D0` | `eax-or-void` | `eax` | `movsx` of `byte [ecx+0x61]` |
   | 10 | `0x005EC9E0` | `eax-or-void` | `al` | always `mov al, 1` |
   | 12 | `0x005ED560` | `eax-or-void` | `eax` | `movzx eax, byte [ecx+0x63]; ret` |
   | 14 | `0x005ECA10` | `eax-or-void` | `void` | see item 5 |
   | 18 | `0x005ED570` | `eax-or-void` | **`al`** | `8A 41 62 C3` = `mov al, byte [ecx+0x62]; ret` — upper EAX undefined |
   | 19 | `0x005ED580` | `eax-or-void` | `eax` | `movzx eax, byte [ecx+0x60]` |
   | 20 | `0x005ED590` | `eax-or-void` | `eax` | a real 0/1 via `sbb`/`neg` |
   | 22 | `0x00477120` | `eax-or-void` | **`al`** | `mov al, …` of `byte [ecx+0x64]` |

   Slots 5 and 6 sit next to each other and have **opposite** channels (a value return and a no-op); slots 12
   and 19 use `movzx eax` while 18 and 22 use `mov al`. Same-looking getters, different channels.
3. **`docs/sdk-errata-draft.md`** lists `cISC4PropOccupant` slot 14 `GetTimeOfDayRange` as *"x87 (high)"*, noted
   as "not checked by a spec". It **has now been checked and the claim is wrong**: the body is
   `movzx` the byte → `fild` → `fmul 0.1` → `fstp dword ptr [out]`, twice, then `pop ecx; ret`. Both `fstp`s
   pop, so **ST(0) is empty at return and the channel is `void`**. Slot 13 `SetTimeOfDayRange` is the same:
   x87 is used internally and the stack is empty at return. A "uses x87 ⇒ returns in ST(0)" rule is wrong here.
   Its two arguments are also **by-value floats**, not `float&`.
4. **`docs/abi/interfaces.tsv` names** — several `cISC4PropOccupant` rows carry identical-code-folded names from
   unrelated classes (slot 2 `icf::cSC4ConstructionOccupant::Release`, slot 3
   `icf::cSC4DBSegment::AsIGZPersistDBSegment`, slot 5 `icf::cSC43DRender::GetProjectionMatrixEntries`, slot 6
   `cSC4NetworkTool::DebugDumpEdgeTypes`, slot 10 `icf::cSC4PropOccupant::SetOrientation` as a folded body,
   slot 17 `icf::cSC43DRender::EnableTransparentBackground`). These are decompiler name guesses. Behaviourally
   slot 3 is `AsOccupant`, slot 5 is a constant-0 stub, slot 6 is a no-op, slot 10 is `SetOrientation`, and
   slot 17 is `SetRandomChance`.
5. **`docs/abi/interfaces.tsv` coverage is complete** — the table stops at slot 23 because the interface stops
   at slot 23. An earlier draft of *this* spec claimed otherwise; see §4.2.

---

## 9. Quirks (reproduced unless marked *[choice]*)

1. The constructor's default random chance is **100**, and the default lot tag is **`0xFFFFFFFF`** (§2.2).
2. The prop's identity is a **u16 index** into a process-global table, not the exemplar key (§5.1); an unknown
   type is interned by `GetPropTypeIndex` (`0x005EF2E0`) on read, so the table grows.
3. `GetRandomChance` returns **AL only** (§4.1): the upper bits of EAX are undefined, so a caller must not treat
   it as a clean 32-bit bool. `[choice]` FreeCity4 zero-extends.
4. `QueryInterface` answers the **class ID** `0x2977AA47` as an IID (§1.1), and passes anything else to the
   `cSC4Occupant` base rather than returning false directly.
5. `Read` does not check the record version; a version ≤ 5 record silently gets timer bits `0x0F`
   (lot-records.md §10.1).
6. `SetState` posts `0xE9C86DDB` only when the occupant is already in the city, and switches the model state only
   when the exemplar has property `0x4A70D491` (network-tools.md §13.1) *[inferred: the model-state call]*.
7. `SetPropTimerState` draws exactly once, on the 0…0xE ↔ 0xF transition of the low nibble, from the prop's own
   generator `0x00B231B0` (network-tools.md §13.1; foundations §6).
8. The visual-effect prop's `Write` drops the effect name and the trailing u32 while the effect is live
   (lot-records.md §10.2).
9. **Adjacent slots, opposite return channels**: slot 5 (`0x004E13E0`) is `xor eax,eax; ret` — a genuine 0 in
   EAX — while slot 6 (`0x00648F10`) is `ret 4` alone, a true no-op returning nothing (§4.1).
10. **`AddRef` / `Release` are this-adjusting thunks**: slots 1 and 2 do `add ecx, 8` before jumping to the
    shared `0x005E89B0` / `0x005E8DF0`. The reference count lives at `this + 0x14`, so a naive `AddRef` on `this`
    touches the wrong object (§2.2).
11. `GetRandomChance` returns **AL only** (quirk 3) while `GetState` and `GetPropTimerBitfield` return clean
    zero-extended **EAX** — three getters that look identical in the header, three different channels.
12. `GetTimeOfDayRange` and `SetTimeOfDayRange` use x87 internally but leave **ST(0) empty**; neither returns
    in `st0` (§8 item 3).
13. `GetPropUsage` `0x00477120` and `SetLotTag` `0x004302C0` are identical-code-folded across the prop, flora and
    landfill occupants — one body, three vtables.

---

## 10. Open questions

- **PO1.** ~~The behaviour, arguments and return channels of `cISC4PropOccupant` **slots 24 and above** (§4.2).~~
  **ANSWERED 2026-10-16 (re-answers27): DIFFERS from its premise.** There are no such slots. The interface is
  exactly 24 slots (`re/labels_windows.tsv` line 322 pairs `00aa4900` with the Mac `__ZTV16cSC4PropOccupant#0`
  at 24; the burnt-building subclass's primary vtable `0x00AA4B30` is also 24 with slots 1–23 identical; and
  nothing stores `0x00AA4960` as a vptr). What follows in the image is documented in §4.2. **A follow-up that
  survives:** what the pool/factory method table at `0x00AA4960` is used for, and which class owns the fifth
  allocator group (stride `0x88`).
- **PO1b.** Which class the fifth allocator group at `0x00AA4960` serves (stride `0x88`, §4.2). None of the four
  prop classes is `0x88` bytes.
- **PO2.** The identity of the sixth sub-object at `+0x24` (vtable `0x00AA47D0`, §2.4).
- **PO3.** The occupant → placeable → 3D object seam (§6.1): occupant slots 20/21 and `CreateGeometry`.
- **PO4.** ~~The remaining unnamed fields: `+0x61`, `+0x64`, and the seven zeroed dwords at `+0x3C`…`+0x54` (§2.2).~~
  **ANSWERED 2026-10-16 (re-answers27), in the main.** `+0x61` = orientation, `+0x64` = prop usage,
  `+0x3C`…`+0x53` = the bounding box, `+0x54` = an owned sub-object pointer, `+0x14` = the reference count.
  **What still has no meaning:** the 0x28-byte sub-record at `+0x54` that slots 15/16 drive (through a manager
  at `0x00B43D5C`).
- **PO5.** Whether a prop's box y is ever re-solved against terrain after placement (§5.2) *[inferred]*.
- **PO6.** The SDK names of `cISC4PropOccupant` slots 5 and 6 (§4.1). Their **behaviour is settled** — slot 5
  is a stub returning 0 in EAX (`xor eax,eax; ret`), slot 6 a true no-op (`ret 4`) — but the ABI rows carry
  identical-code-folded names from unrelated classes and nothing indicates the SDK's names. A plugin that calls
  them, or an SDK header, would settle the names.

---

## 11. Conformance tests

Each test names its section. A "city" is a fresh 64 × 64 city unless stated otherwise. Prop type IDs below are
the shipped road/lot prop exemplars; substitute any catalogue entry whose `OccupantSize` is a Float32 array of
three.

1. **Identity** (§1). A newly created plain prop: `QueryInterface(0xE9793A65)` and `QueryInterface(0x2977AA47)`
   both return the object base with a new reference; `QueryInterface(0x452294AA)` returns base `+4`; an unknown
   IID reaches the occupant base and comes back false.
2. **Constructor defaults** (§2.2). A fresh prop has state 0, chance **100**, time-of-day range (0, 0), timer
   bits 0, lot tag **`0xFFFFFFFF`**, prop type index 0 and occupant type `0xA823821E`.
3. **Object size** (§1). `CreateProp` for a profile with occupant class `0x2977AA47` allocates `0x68` bytes;
   `0x2977AA48` allocates `0xA0`; `0x2977AA49` allocates `0x94`.
4. **AsOccupant, the vtable extent and the refcount** (§2.2, §2.3, §4.2, quirk 10). Slot 3 returns `this + 8`
   and returns null for a null `this`. The `cISC4PropOccupant` vtable has exactly **24** entries
   (`+0x00`…`+0x5F`): the word at `0x00AA4900 + 24*4` is not part of this interface (it belongs to the
   pool/factory table at `0x00AA4960`). `AddRef` twice from 0 and `Release` once: the count lives at `+0x14` and
   the value returned by slot 1 must agree with it; a naive refcount on `+0x04` must **not**.
5. **GetPropType round trip** (§5.1). `SetPropType(id)` then `GetPropType()` returns `id`. Two props set to the
   same `id` report the same value and store the **same** `+0x5C` index (a global table).
6. **Unknown type is interned** (§5.1). Reading a save whose prop names a type absent from the catalogue
   succeeds; afterwards `GetPropType` returns that ID and the process-wide table has grown by one.
7. **GetState is a clean EAX return** (§4.1). `SetState(2)` then `GetState()` returns 2 in EAX with the upper
   bytes zero (`movzx`), not merely AL.
8. **GetRandomChance is an AL return** (§4.1, quirk 3). `SetRandomChance(37)` then `GetRandomChance()` returns 37
   in AL. A caller that reads EAX as a 32-bit value must mask to `0xFF`.
9. **Bounded box** (§5.2). `CreateProp` on a prop whose `OccupantSize` is (2, 3, 4) gives a box (0,0,0)–(2,3,4);
   a prop with no size gives (0,0,0)–(1,1,1).
10. **Orientation** (§5.2). `SetOrientation(3)` then `GetOrientation()` returns 3; the save record field 5 is 3.
11. **Time of day** (§4.1). `SetTimeOfDayRange(18, 8)` stores (180, 80) in `+0x5E`/`+0x5F`; `GetTimeOfDayRange`
    returns (18.0, 8.0). `SetTimeOfDayRange(0, 0)` registers no clock entry.
12. **Timer bits** (§4.1). `GetPropTimerState(0x2)` is false; `SetPropTimerState(0x2, true)` makes it true and
    leaves the other bits alone; `GetPropTimerBitfield()` reports the mask.
13. **Timer transition draw** (quirk 7). With chance 100 and the low nibble going to `0xF`, `SetState(0)` is
    called. With chance 0 it is not. Setting bits that leave the nibble below `0xF` never draws.
14. **Lot tag default** (§5.3). A fresh prop's `GetLotTag` (occupant slot 27) is false; `SetLotTag(0x1234)` makes
    it true and returns `0x1234`.
15. **Usage** (§3). `AddLotProp` writes usage 2; `AddCityProp` writes 1; `AddStreetProp` writes 4;
    `AddBuildingProp` writes 3. `GetPropUsage` / `SetPropUsage` round-trip each.
16. **Save / load round trip** (§7). Create a prop with type, box, orientation 2, state 1, range (180, 80),
    chance 50, usage 2, lot tag `0xABCD1234`, timer bits 7. Save and reload: every field is identical, and the
    record's version field is 6.
17. **Version ≤ 5 read** (quirk 5). A record whose version is 5 (with field 13 omitted) reads successfully and
    comes back with timer bits `0x0F`.
18. **Unknown type survives a round trip** (§5.1, quirk 2). Save a prop whose type is not in the catalogue,
    reload, save again: the type ID is unchanged in the record.
19. **Class-object store framing** (§7). In a save with N props, the record `(0x2977AA47, 0x299B2D1B, 0)` parses
    into exactly N entries with no trailing bytes, and each entry's CRC verifies under CRC-32/MPEG-2.
20. **Loaded props do not announce themselves** (§3). After a load, no `CreateOccupant 0x99EF1140` is posted for
    the loaded props (lot-records.md §2.4).
21. **Visual-effect attach** (§6.2). Create a `0x2977AA49` prop, `SetEffectType(name)`, `StartEffect(true)`:
    the call returns true only when an effect object exists at `+0x6C`; `StopEffect(true)` returns true only when
    one existed and was started.
22. **Visual-effect record quirk** (§6.2). A live effect prop saves an **empty** name and 0; after reload the
    name comes back from the exemplar *[inferred]*.
23. **State and the model** (quirk 6). `SetState(s)` posts `0xE9C86DDB` when the occupant is in the city, and does
    nothing when `s` already equals `+0x63`.
24. **PV2 tag matching** (§5.3). A lot prop whose tag equals a configuration object's ID is counted normally; one
    whose tag names nothing leaves the block's slot count one lower (prop-developer.md §4.3 step 2.3.2).
25. **Removal** (§3). `RemovePropA` then `Shutdown` removes the prop from the occupant manager; a second removal
    is a no-op.
