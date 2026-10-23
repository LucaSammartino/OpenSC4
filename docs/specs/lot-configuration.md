# Spec: the lot configuration (`cSC4LotConfiguration`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from observing the original program and its data files. Implementers must work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words and tags (*[compat]*, *[choice]*, *[inferred]*, *[quirk]*) as in [foundations.md](foundations.md).

Scope: one **lot configuration** (a "lot template"): the object that the lot configuration manager builds from each
lot configuration exemplar, what it reads from the exemplar, the answers its 33 interface slots give to the growth,
zone, tract, lot and prop developers, and its quirks. The **manager** (component 9: the catalogue, its detail-level
rebuild, the filter cache, the family set, the per-city building information) is specified in
[prop-lotconfig.md](prop-lotconfig.md) Part B and is cited here, not repeated.

Related specs: [prop-lotconfig.md](prop-lotconfig.md) (§B3 catalogue and detail filter, §B4.3 building information,
§B5 manager queries), [lot-developer.md](lot-developer.md) (§17.1 the 0x34-byte object record, §6 construction,
§17.5 foundations, §17.6 object positions), [tract-developer.md](tract-developer.md) (§4.6–§4.12 configuration
choice), [zones.md](zones.md) (§7.2.1 lot size, §8.4 `CanCreateLot`, §8.4.2 constraint tiles),
[prop-developer.md](prop-developer.md) (optional props), [building-development.md](building-development.md) (§4.7
building types and families), [data-formats.md](data-formats.md) (§4.4 inheritance from cohorts, §5 getters),
[city-detail.md](city-detail.md) (§3.1 the detail filter, CD1), [foundations.md](foundations.md) (§18 x87 precision).

## 0. Summary

- A lot configuration is a 0xC0-byte object (class `0xA99062B4`) built **only** by the lot configuration manager:
  one per exemplar in group `0xA8FBD372`, at the first city `Init` and again after a detail change (prop-lotconfig
  §B3). It is never saved; a lot saves only the configuration ID (lot-records.md).
- `Load` (slot 31) resets the object, then reads about twenty exemplar properties through the exemplar's effective
  property list, so a value in the **parent cohort** counts (data-formats §4.4). 1897 of the 1908 shipped
  configurations inherit their slopes, retaining wall and foundation from one of 41 cohorts.
- The answers are plain field reads: size, growth stage, three slopes, water, three compatibility bit sets (zone
  type, wealth, purpose), the road-side bits, the construction flag, the object array and the ID lists. Building
  capacity, style (occupant groups) and the monopoly flag are **not** read from the exemplar: the manager fills them
  when a city has initialised (prop-lotconfig §B4.3).
- Slot 19 `IsCompatibleWithMonopolyPieceUse` always answers **true** and takes one argument (PL3, §5.4).
- No messages (it neither posts nor subscribes), no random draws, no cadence, no saved record; it is not a
  city component and has no `Init` or `Shutdown` slot of its own.

Implementation size: about 300 lines (`Load` 150, the slots 80, the editor `Save` 70).

## 1. Identity

| Item | Value |
|---|---|
| Class ID | `0xA99062B4` (class ID table); not registered with GZCOM for creation: the manager allocates it directly |
| Interface | `cISC4LotConfiguration`, IID `0x68EDBEBC`; vtable `0x00A8CEB8`, 34 slots (§8) |
| Size | 0xC0 bytes |
| Constructor | `0x004797A0`: every field zero except the defaults of §2 |
| Created by | lot configuration manager slot 5 (`0x00489910`: allocate, construct, AddRef, `Load(key, detail)`), slot 22 (`CreateGenericLotConfiguration`, editor) |
| Destructor | `0x00479C20` (calls `Shutdown` first); deleting destructor slot 33 `0x00479F50` |

### 1.1 Creating one: the named route (GLUE21-1 — PARTLY, answered 2026-10-16 by re-answers31)

**The premise is right and the conclusion is not.** `cSC4LotConfiguration` really is *not* registered with GZCOM
for creation — its class ID `0xA99062B4` exists for serialization refs (`re/class_id_table.tsv:330`) but there is
no factory entry, so a plugin cannot `GetClassObject` it. **But there is a named route to the
allocate-and-construct pair, and it is the manager, not GZCOM.**

The pair is the helper `FUN_00489910` (`0x00489910`), which is what the "Created by" row above describes:

| Step | Call | Address |
|---|---|---|
| allocate | `FUN_005e55e0(0xC0)` — the pooled allocator, exactly the 0xC0-byte size | `0x005E55E0` |
| construct | the constructor | `0x004797A0` |
| AddRef | `vtable+0x04` | — |
| load | `vtable+0x7C` (slot 31) `Load(key, detail)`, the detail byte from `*(DAT_00B43CD8 + 0xF16)` | — |
| on success | `FUN_00648f00` — insert into the manager's map/family-ID set | `0x00648F00` |
| on failure | `vtable+0x08` `Release`, return 0 | — |

The **public** routes into it are the manager's own slots, and the manager **is** GZCOM-creatable — CLSID
**`0x299062C0`** (`re/class_id_table.tsv:317`), `cISC4LotConfigurationManager`:

- slot 5 `AddLotConfiguration(key const&)` — `0x00489A60`
- slot 6 `AddLotConfiguration(cfg*, bool)` — `0x0048A690` (takes an already-built configuration)
- slot 22 `CreateGenericLotConfiguration(cfg*&, buildingType)` — `0x0048BCF0` (the editor's generic one)

**The change to make.** `enable = lotcfg` must not probe GZCOM for `cSC4LotConfiguration`. Either

1. **replace the manager** (CLSID `0x299062C0`) and have the replacement allocate its own
   `cISC4LotConfiguration`-shaped objects — 0xC0 bytes, vtable `0x00A8CEB8`, 34 slots (§8) — and answer slots
   5/6/22/8 out of its own catalogue; or
2. **keep the game's manager** and drive it through slots 5 and 22 with keys, letting the game allocate.

Either way the configuration **class** does have to be reimplemented as a type you allocate (nothing else can
make one), but it does **not** need a GZCOM registration — there is none to have. The previous behaviour
("`create_configuration` probes GZCOM and refuses when it does not answer") is what blocks `enable = lotcfg`;
probing GZCOM is the wrong lookup. Note also that `GetLotConfiguration` (slot 8, `0x0048A320`) returning null
is a *legitimate* answer when a key is unknown, so the game's null deref at `SimCity 4.exe+0x5BF49` (reading
`0x8C`) is most likely a key that never got registered, not a missing class.

`QueryInterface` (`0x004777B0`): IIDs `1` and `0x68EDBEBC` give the object (with a reference); anything else false.
Reference count at +0x04, starting at 0. `AddRef` is the shared `0x005F47F0`. `Release` (`0x00479CE0`): a count
above 1 is decremented and returned; otherwise the object resets itself (`Shutdown`, §3.1 step 3) and is deleted
(slot 33 with flag 1), returning 0.

## 2. State

| Offset | Type | Field | Default (constructor and reset) | Set by | Read by |
|---|---|---|---|---|---|
| +0x00 | ptr | vtable `0x00A8CEB8` | | | |
| +0x04 | u32 | reference count | 0 | | |
| +0x08 | u32 | **ID** = the instance of the exemplar key | 0 | `Load` | slot 3 |
| +0x0C | `cRZString` (0x14 bytes, layouts.md) | **name**, exemplar property `0x20` | empty | `Load` | slot 4 |
| +0x20, +0x21 | u8, u8 | **size** in cells: width (x), depth (z) | 0, 0 | `Load` | slot 7 |
| +0x24 | `vector<u32>` | **family list** (lot configuration families) | empty | `Load` | slots 5, 6 |
| +0x30 | f32 | maximum slope | 90.0 | `Load` | slot 8 |
| +0x34 | f32 | minimum slope | 0.0 | `Load` | slot 9 |
| +0x38 | f32 | maximum slope before foundation | 90.0 | `Load` | slot 10 |
| +0x3C | u8 | **growth stage** | 1 | `Load` | slot 11 |
| +0x3E | u16 | minimum building capacity | `0xFFFF` | manager (§4) | slot 12 |
| +0x40 | u16 | maximum building capacity | 0 | manager (§4) | slot 13 |
| +0x42 | u8 | water-compatible flag | 0 | `Load` | slot 14 |
| +0x44 | bit vector, 16 bits | compatible **zone types** | all false | `Load` | slot 15 |
| +0x58 | bit vector, 4 bits | compatible **wealths** | all false | `Load` | slot 16 |
| +0x6C | bit vector, 9 bits | compatible **purposes** | all false | `Load` | slot 17 |
| +0x80 | `vector<u32>` | **styles**: the occupant groups of the configuration's buildings, ascending | empty | manager (§4) | slot 18 |
| +0x8C | u8 | monopoly flag | 0 | manager (§4) | **nothing** (§5.4) |
| +0x8D | u8 | do-construction flag | 0 | `Load` | slot 20 |
| +0x8E | u8 | required road sides (bits 0–3) | 8 | `Load` | slot 21 |
| +0x90 | `vector<record>` | **object array** of 0x34-byte records (lot-developer §17.1) | empty | `Load` | slots 22–27 |
| +0x9C | `vector<u32>` | retaining wall types | empty | `Load` | slot 28 |
| +0xA8 | `vector<u32>` | fence types | empty | `Load` | slot 29 |
| +0xB4 | `vector<4 × s8>` | fence segment positions | empty | `Load` | slot 30 |

Vectors are STLport `(begin, end, capacity end)` triples; the bit vectors are STLport `vector<bool>` (start word
pointer, start bit offset, finish word pointer, finish bit offset, end of storage: 0x14 bytes). The manager writes
+0x3E, +0x40, +0x80 and +0x8C directly, not through a slot (§4; layouts proposal in the ABI rows file).

## 3. Loading (`Load(key, detail)`, slot 31, `0x00479F70`, pop 8, AL) *[compat]*

### 3.1 Steps

1. Ask the global resource manager (`0x00B43CA8`, slot 3) for the resource at `key` with the exemplar interface
   `0xA52160F5`. None → return **false** with the object untouched.
2. Take the exemplar's property holder (exemplar slot 7). Every property below is read through it, so inherited
   cohort properties count (data-formats §4.4).
3. **Reset** (`0x00479A10`): every field of §2 back to its default; the three bit vectors are cleared and resized
   to 16, 4 and 9 false bits; the object array is emptied (its ID vectors freed); the other vectors are emptied.
4. Name := property `0x20` (string getter `0x005FD570`; absent → stays empty). ID := the key's **instance**.
5. Version := property `0x88EDC789` (u8 getter `0x005FD3C0`). Absent or 0 → release the exemplar and return
   **false**. The name and ID stay set. *(Every shipped configuration has version 2.)*
6. Read the lot-level properties of §3.2 in the table's order.
7. Build the object array (§3.5) with the version and `detail`, then correct the object positions (§3.6).
8. Foundation object (§3.7).
9. Trim the object array's capacity to its size (no observable effect).
10. If the version is not 2: rewrite the configuration into the resource (resource manager slot 32 for the key's
    database segment, then this object's `Save`, slot 32, §7). *(Never happens with shipped data.)*
11. Release the exemplar and return **true**.

### 3.2 Lot-level properties

"Getter" names the typed helper (data-formats §5.1, §5.2): the scalar helpers convert any numeric stored type; the
vector helpers copy the stored bytes with no type check.

| Property | Name (community) | Getter | Field and rule | Shipped (effective, 1908) |
|---|---|---|---|---|
| `0x88EDC790` | LotConfigPropertySize | `vec<u8>` `0x005FFB40` | when the vector has **at least 2** values: width := v[0], depth := v[1]; otherwise the size stays (0, 0) | u8 × 2 in all; 54 distinct sizes, from (1, 1) to width 20 and depth 35 |
| `0x88EDC791` | LotConfigPropertyFamily | `vec<u32>` `0x005FF510` | family list := the values (absent → empty) | 1729 have one value; 1727 of them equal the building object's ID list; the two landmarks `0x97000001`, `0x97000002` use `0xC91F5897`; 179 absent |
| `0x88EDC792` | MaxSlopeBeforeLotFoundation | f32 `0x005FD4F0` | +0x38 (absent → 90.0) | 3.0 ×1454, 2.0 ×176, 0.0 ×110, 1.0 ×69, 90.0 ×53, 16.0 ×43, 20.0 ×2, 5.0 ×1 |
| `0xE99B068C` | MaxSlopeAllowed | f32 | +0x30 (absent → 90.0) | 25 ×730, 32 ×523, 20 ×234, 50 ×147, 16 ×133, 30 ×114, 40 ×14, 7 ×7, 5 ×4, 35 ×1, 12 ×1 |
| `0x699B08A4` | MinSlopeAllowed | f32 | +0x34 (absent → 0.0) | 0 ×1881, 5 ×26, 6 ×1 |
| `0x27812837` | GrowthStage | u8 | +0x3C (absent → 1) | 1 ×644, 2 ×328, 3 ×290, 4 ×133, 5 ×128, 6 ×128, 7 ×130, 8 ×127 |
| `0x89136440` | LotConfigPropertyWaterCompatibility | u8 | +0x42 := 1 when present and non-zero, else 0 | 1 in 8 (the three seaport piers, the ferry terminal and passenger ferry, the marina, the cruise ship, the lifeguard tower) |
| `0x88EDC793` | LotConfigPropertyZoneTypes | `vec<u8>` | each value `v < 16` sets zone bit `v`; larger values are ignored (§3.3) | see §3.3 |
| `0x88EDC795` | LotConfigPropertyWealthTypes | `vec<u8>` | each `v < 4` sets wealth bit `v` | 0 ×161, 1 ×290, 2 ×819, 3 ×658 configurations (10 have all of 1–3) |
| `0x88EDC796` | LotConfigPropertyPurposeTypes | `vec<u8>` | each `v < 9` sets purpose bit `v` | 0 ×172, 1 ×301, 2 ×583, 3 ×570, 4 ×1, 5 ×31, 6 ×89, 7 ×90, 8 ×71 |
| `0xE99B068D` | (construction animation) | u8 | +0x8D := 1 when present and non-zero | 1 in 9: the three landing strips and the six municipal and international airports |
| `0x88EDC798` | LotConfigPropertyRetainingWallTypes | `vec<u32>` | retaining wall list := the values | one value in 1907 (`0xC96D2134`–`0xC96D2137`); absent in the lifeguard tower |
| `0x88EDC797` | LotConfigPropertyFenceTypes | `vec<u32>` | fences, §3.4 | absent in all |
| `0x4A4A88F0` | (road requirements) | u8 | +0x8E := the value; then bit 3 is forced on (§3.3) | 8 ×1702, 12 ×108, 9 ×87, 13 ×6, 10 ×2, 15 ×2, 11 ×1 |
| `0x88FCD877` | (lot foundation type) | u32 `0x005FD480` | §3.7 | present in 1907 |

The exemplar type (`0x10`, 16 in every shipped configuration) is not checked. Of the effective values, the slopes,
the retaining wall and the foundation come from the parent cohort in 1897 configurations; the 11 configurations
without a parent carry them themselves. Hence the code defaults 90 / 0 / 90 never apply to shipped data.

### 3.3 The bit sets and the road sides

- **Zone, wealth and purpose.** The three lists are read with the byte-vector helper, so a value is one byte. Each
  value below the set's size (16, 4, 9) sets that bit; others are dropped without a message. A list that is absent
  leaves every bit false. A list that is present but empty also sets nothing, so a text exemplar's `Uint8:0:{}`
  gives no types (data-formats §4.3). Shipped zone lists, by zone type (SDK `cISC4ZoneManager::ZoneType`):
  0 none ×22, 1 R low ×152, 2 R medium ×244, 3 R high ×301, 4 C low ×419, 5 C medium ×887, 6 C high ×1152,
  7 agriculture ×79, 8 I medium ×177, 9 I high ×250, 10 military ×1, 11 airport ×10, 12 seaport ×3,
  15 plopped ×151; 13 and 14 never.
- **Purpose 9** (`Other` in the SDK enumeration) cannot be stored: the set has 9 bits (0–8).
- **Road sides.** +0x8E holds the property's byte. If bit 3 is clear after reading (the property absent leaves the
  default 8), bit 3 is set *[compat]*: every configuration requires a road on side 3. The game formats a debug
  line "Lot template 0x%08x should have a road required on the south edge" at that point and throws it away (no
  output). Bits 4–7 are kept but nothing reads them. Side numbering is in the configuration's own frame; side 3 is
  the "south" (front) edge; the developers rotate it by the lot's facing (tract-developer §4.10.3).

### 3.4 Fences *[quirk]*

With the fence list `v` present and at least 2 values long:

1. `n := v[0]`; the fence types are `v[2] … v[n+1]` (**index 1 is skipped**), read with no bound check.
2. Then, at index `i := n + 2`, if `i` is inside the list and `m := v[i]` is non-zero: `m` segment records follow,
   4 values each; each record keeps the **low byte** of its four values as four signed bytes. These reads are not
   bound-checked either.

`Save` writes `[n, types…, m, segment values…]` with the types from index **1** (§7), so a saved fence list reads
back shifted by one. No shipped configuration has fences, and **nothing in the game reads the fence lists through
slots 29 and 30** (LC1):
- the two getters (`0x004778B0`, `0x004778C0`) are referenced only from the vtable (`0x00A8CF2C`, `0x00A8CF30`), with
  no direct call;
- every virtual call through `+0x74` / `+0x78` in the place-lot tool, the lot developer, the zone, tract and prop
  developers and the BDS goes to another interface: those calls pass arguments or return a float, and slots 29/30 take
  none and return a pointer.

The only other code that touches `+0xA8` / `+0xB4` is the configuration's own `Load`/`Save` and the debug lot
configuration editor, which edits the vectors in place (`ToggleFence` `0x0047E890`, `0x00484700`). So no fence is
ever drawn or built from a configuration. *[choice]* FreeCity4 may read fences either way; it must not read past the
end of the list. Keeping them only for `Save` is enough.

### 3.5 The object array

The records are built from the properties `0x88EDC900 + k` (`0x00479D10`) as prop-lotconfig §B3.3 and
lot-developer §17.1 describe: the scan stops at the first missing `k` or at `0x88EDCE00` (at most **1280**
objects; the shipped international airport `0x0A512811` has exactly 1280), a property with fewer than 10 values is
skipped, and an object is kept when its type is 0 or its level-of-detail bits `(v1 >> 4) & 3` are below the
city detail. The record, in the SDK's names (`cSC4LotConfigurationObject`):

| Value | Record field | Rule |
|---|---|---|
| `v0` | +0x00 type | low byte |
| `v1` | +0x01 flags | low byte. Bits 4–5: detail level. Bit 0: **optional** (props; prop-developer.md) |
| `v2` | +0x02 rotation, +0x03 mirror | rotation := low byte (0–3 in shipped data); mirror := bit 31 |
| `v3`, `v4`, `v5` | +0x04, +0x08, +0x0C x, y, z | signed 16.16 fixed point: `float(int32 · 2⁻¹⁶)`, metres from the lot's corner |
| `v6`, `v7` | +0x18, +0x1C rect min x, min z | 16.16 |
| `v8`, `v9` | +0x10, +0x14 rect max x, max z | 16.16 |
| `v10` | +0x20 foundation / extra type | as is |
| `v11` | +0x24 object ID (lot tag) | as is when version ≥ 2; for version < 2 a process-wide serial (`0x00B2166C`, seeded from the GZCOM clock when 0, then incremented) and the ID list starts at `v11` |
| `v12 …` | +0x28 ID list | `vector<u32>` |

Object types and their readers:

| Type | Meaning | Shipped count (all details) | Read by |
|---|---|---|---|
| 0 | building; the ID list holds building families or types | 1908 (exactly one per configuration, one ID each, `v10` = 0) | slot 24; lot developer §6; manager §B4.3, §B5.3, §B5.6 |
| 1 | prop; IDs are prop types or families | 45,012 (5880 optional) | lot developer §6.5; prop developer |
| 2 | base or overlay texture; mirror flag used | 25,452 (624 mirrored) | slot 25; lot developer §6.6 |
| 3 | foundation | 0 in the properties; added by §3.7 | slot 26; lot developer §17.5 |
| 4 | flora; IDs are flora types or families | 1359 | lot developer §6.5 |
| 5 | water constraint | 43 | zones §8.4.2 |
| 6 | land constraint | 17 | zones §8.4.2 |
| 7 | network connection; positional IDs | 41 | lot developer `AddNetworkConnectionNodes`; slot 27 `HasObjectsOfType(7)` in `AddLotFoundation` |

Array order is property order (then the foundation object last), so "the first object of type `t`" is the first
such property.

### 3.6 Position correction (`0x00477BB0`) *[compat]*

Run once on the new array with the lot box `minX = minZ = 0`, `maxX = 16·width`, `maxZ = 16·depth` (the 16 is a
literal, not the city's cell size). With `c :=` the float constant `0x3D23D70B` (0.040000003) and the bounds
`loX := r24(minX + c)`, `hiX := r24(maxX − c)` (likewise for z), for every record:

- If `x < loX` (strict; NaN no): `d := r24(loX − x)`; else if `x > hiX` (strict; NaN no): `d := r24(hiX − x)`;
  else no change in x. On a change: `x := r24(x + d)`, rect min x `:= r24(minx + d)`, rect max x `:= r24(maxx + d)`.
- The same for z with `loZ`, `hiZ` and the z fields.

Only the centre is clamped; the footprint moves with it and may still stick out of the lot. Arithmetic is the x87
at 24-bit precision (foundations §18), so each operation rounds to float. The function's "anything moved" result is
ignored. No shipped object needs a correction (every shipped centre lies within `[0.04, 16·size − 0.04]`).

### 3.7 The foundation object

When property `0x88FCD877` is present (u32 getter), one record is **appended** after the correction (so it is
neither clamped nor filtered by detail level):

| Field | Value |
|---|---|
| type, flags, rotation, mirror | 3, 0, 0, 0 |
| x, y, z | 0.5, 0.0, 0.5 |
| rect max x, max z, min x, min z | 0.6, 0.6, 0.4, 0.4 (floats `0x3F19999A`, `0x3ECCCCCD`) |
| +0x20 | the property's value |
| +0x24 object ID | **not written**: whatever the stack held *[quirk]* |
| ID list | empty |

*[choice]* FreeCity4 writes 0 for that object ID; nothing reads a foundation object's ID. Shipped values:
`0x890B7314` ×889, `0x890B7315` ×640, `0x890B7313` ×371, and five port and ferry values (`0x19490000`,
`0x194A0000` … `0x19500000` range) ×7; absent in the lifeguard tower only. The lot developer reads it through slot
26 (lot-developer §17.5.4).

**PL23 raw-equivalence limit.** The +0x24 ID is not an exemplar-derived default: the loader fills the record
at `0x0047A59E`–`0x0047A609` without writing that dword, and the copy at `0x0047892B`–`0x00478931`
preserves it. Keep FreeCity4's zero choice and report any raw difference; no fixed native ID or whole-array
exactness follows from agreeing on the defined fields. The defined +0x20 foundation type remains mandatory.
See [prop-lotconfig.md §B3.3](prop-lotconfig.md#b33-which-lot-objects-are-kept-answers-cd1) and test 16 for the
comparison contract and conformance cases.

## 4. Building information (filled by the manager) *[compat]*

Four fields are not exemplar data. The lot configuration manager writes them for every configuration when it
receives `kSC4MessagePostCityInit` (prop-lotconfig §B4.3, `0x0048AD40`), from the building types of the building
object's ID list:

| Field | Value | Before the first city-init message |
|---|---|---|
| +0x3E / +0x40 | smallest / largest building profile capacity (profile +8, u16); **both 0** when no type was found or the configuration has no building object | `0xFFFF` / 0 |
| +0x80 | every occupant group of those types' exemplars, sorted ascending, no duplicates (the old contents are dropped first) | empty |
| +0x8C | 1 when at least one type's exemplar lacks the monopoly property `0x2781283F` or has it false | 0 |

A type whose exemplar does not load adds no group and sets +0x8C, but its capacity still counts (prop-lotconfig
§B4.3, PL11). A configuration rebuilt by a detail change (prop-lotconfig §B3.1) is new, so it has the defaults until the next
city-init message. The message comes after every city load, so growth never sees the defaults in a normal session.

## 5. Behaviour of the slots

### 5.1 Plain getters

| Slot | Method | Result |
|---|---|---|
| 3 | `GetID()` | +0x08 |
| 4 | `GetName(string&)` | assigns the name to the caller's string (its `operator=`, string slot 13) and returns what that returns: **a reference to the caller's string**, not a bool *[quirk]* |
| 6 | `GetFamilyIDArray()` | address of the family list |
| 7 | `GetSize(w&, d&)` | `w := width`, `d := depth` (zero-extended). No defined result; EAX happens to hold the depth |
| 8, 9, 10 | `GetMaximumSlope`, `GetMinimumSlope`, `GetMaximumSlopeBeforeFoundation` | +0x30, +0x34, +0x38 in ST0 |
| 11 | `GetGrowthStage()` | +0x3C |
| 12, 13 | `GetMin/MaxBuildingCapacity()` | +0x3E, +0x40 (u16) |
| 14 | `IsCompatibleWithWater()` | +0x42 |
| 20 | `IsDoConstruction()` | +0x8D |
| 22 | `GetLotConfigurationObjectCount()` | number of records |
| 23 | `GetLotConfigurationObjectArray()` | address of the object array |
| 28 | `GetRetainingWallIDArray()` | address of +0x9C |
| 29, 30 | `GetFenceIDArray()`, `GetFenceSegmentPositionArray()` | addresses of +0xA8, +0xB4 |

The array and list pointers point into the configuration; the caller borrows them.

### 5.2 Tests

| Slot | Method | Rule |
|---|---|---|
| 5 | `BelongsToFamily(f)` | `f` is in the family list (linear scan) |
| 15 | `IsCompatibleWithZoneType(t)` | zone bit `t` |
| 16 | `IsCompatibleWithWealthType(w)` | wealth bit `w` |
| 17 | `IsCompatibleWithPurposeType(p)` | purpose bit `p` |
| 18 | `IsCompatibleWithStyleType(g)` | `g` is in the style list (+0x80; linear scan) |
| 27 | `HasObjectsOfType(t)` | some record's type byte equals `t` (a `t` above 255 never matches) |

Slots 15–17 index the bit vector **without a bound check** *[quirk]*: an argument past the set's size reads a bit of
the storage word that `Load` never wrote, and a negative one reads before it. Shipped callers pass zone types 0–15,
wealths 0–3 and purposes 1–8. *[choice]* FreeCity4 answers false outside the set.

### 5.3 Objects and road sides

- **Slots 24, 25, 26** `GetPrimaryBuilding/Base/FoundationConfigurationObject()`: the first record of type 0, 2 or
  3 (`0x00477B10`), or null. Slot 25 therefore gives the first **texture**, whatever its role.
- **Slot 21** `GetRequiredRoads(vector<bool>& out)` (pop 4): if `out` does not have exactly 4 elements it is
  resized to 4 (STLport `resize`, new elements false); then `out[i] := bit i of +0x8E` for `i = 0 … 3`. No defined
  result. Consumers: tract-developer §4.10.3 (side `(facing + i + 1) mod 4`) and §4.12 row 6 (bits 0–2 decide
  "roads required"; bit 3 is ignored there).

### 5.4 Slot 19 `IsCompatibleWithMonopolyPieceUse` (answers PL3) *[compat]*

The slot (shared code at `0x004FC900`) returns **true** and pops **4 bytes**: it takes one dword argument and
ignores it. It does not read +0x8C. The SDK declares it without arguments: **SDK error**, a caller that follows the
header leaves 4 bytes on the stack. The Mac build's slot is a 10-byte "return 1" too, so the constant is the
original design, not an inlining accident *[inferred]*.

+0x8C is written by the manager (§4) and reset by `Load`, and **nothing reads it**: a scan of every byte read at
offset 0x8C finds none in lot, zone, tract or plop code, and slot 19 does not touch it. The monopoly-piece rule is
applied elsewhere, from the building exemplars' `0x2781283F`:

- the tract developer's `IsBuildingCompatible` (tract-developer.md, code 2);
- the plop tool's lot filter `cSC4LotConfigurationFilterMonopoly` (vtable `0x00A906C0`, 0x1C bytes, test
  `0x004BEDD0`): when its "allow monopoly" byte (+0x0C) is clear, a configuration is **excluded** when any type in
  its building object's expanded ID list (manager slot 27, then building development slot 14 for the key) has a
  true `0x2781283F`; a configuration without a building object is included. UI only.

No caller of slot 19 was found. *[choice]* FreeCity4 implements slot 19 as "return true, pop 4".

## 6. Lookups the other specs use

How the simulation finds and judges configurations. The manager queries are prop-lotconfig §B5; this table names
the configuration slots behind each decision.

| Decision | Calls | Where |
|---|---|---|
| Configuration of a lot, by ID | manager slot 8 | the lot's `Init` (lot-records.md), the lot developer, the port developers |
| Candidates for a growth developer (purpose, wealth) | manager slot 9 with a registered filter calling slots 17 and 16; then slot 11 (stage), 18 (style), 24, 12, 13 | tract-developer §4.6, §4.7 |
| Candidates for a zone (purpose, wealth, stage ≤ s) | manager slot 9 with an unregistered filter calling 17, 16, 11; then 24, 7 and the building family resolution below | zones §7.2.1 |
| Fit on a site | 7 (exact size), 15 (zone type at the site), 11 with 16 (stage against the existing lot), 8 and 9 (slope window, NaN passes), 21 (road sides) | tract-developer §4.12, §4.10.3 |
| Lot creation | 14 (water), 15 (zone), 8 (slope), 23 (constraint objects 5 and 6) | zones §8.4, §8.4.1, §8.4.2 |
| Construction | 24 (building object), 10 and 14 (foundation), 26, 27, 23 | lot-developer §6, §17.5 |
| Animated construction on plop | 20: the place-lot tool passes it as `animate` to `ConstructDefaultLot` (`0x004C1BB5`) | lot-developer §6.4; growth always animates |
| Optional props | 23 (type 1, flag bit 0) | prop-developer.md |
| Retaining wall type | 28: a uniform pick from the list with the wall generator `0x00B230C8` (`GetRetainingWallType` `0x005E2ED0`); empty or no configuration → 0 | lot-developer §7.2, building-development §4.9.2 |
| Water need | 11 against the developer's water cap | lot-developer §5.3 |

**Family resolution.** A configuration names buildings, props and flora by **type or family** in each object's ID
list; it holds no resolved types. The readers resolve them on demand:

- building IDs: building development slot 19 (`GetBuildingTypesByFamily` into a hash set: a family gives its
  members, a type gives itself) for the zone developer and the manager's building information; slot 18 (into a
  vector, which **assigns**) inside manager slot 27 (prop-lotconfig §B5.6, quirk 13); slot 20
  `IsBuildingInFamily` inside manager slot 17;
- prop IDs: prop manager slots 8 and 9; flora IDs: flora simulator slots 11 and 14 (manager slot 27);
- the configuration's own **family list** (+0x24) is a separate thing: it names lot configuration families. In
  shipped data it repeats the building family in 1727 of 1729 cases. It answers slot 5, the manager's
  by-family queries (slots 10 and 14) and the manager's family set (slot 21).

## 7. Saving (slot 32 `Save(key, segment)`, `0x00479580`, pop 8, AL) — editor only

Find the exemplar at `key` (or create a new one), write the configuration into its property holder
(`0x00478AC0`), then hand it to the resource manager for the segment. The writer sets: `0x10` = 16, `0x20` the
name, `0x88EDC789` = **2**, the size as two bytes, the family list (when non-empty), the three slopes, the stage,
water (only when set), the three bit sets as byte lists of their set indexes (only when any bit is set or the set is
not its default size), the construction flag (only when set), `0x88FCD877` from the first type-3 record (its +0x20,
or its first ID when +0x20 is 0), the wall list, the fence list (§3.4), `0x4A4A88F0` and the objects
(`0x004783A0`). Objects of type 3 are skipped; the others are numbered from `0x88EDC900` without gaps as
`[type, flags, rotation | mirror << 31, trunc(x·65536), trunc(y·65536), trunc(z·65536), min x, min z, max x,
max z (same conversion), +0x20, +0x24, IDs…]`. **Before writing, the writer sets y to 0 in the configuration
itself** for every object that is not a prop, and for a prop with a negative y *[quirk]*. Nothing in the simulation
calls slot 32; `Load` calls it for configurations of a version other than 2 (§3.1 step 10).

## 8. Interface slot table (`cISC4LotConfiguration`, vtable `0x00A8CEB8`) *[compat]*

| Slot | Offset | Method | Pop | Return | Address |
|---|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface(iid, void**)` | 8 | AL | `0x004777B0` |
| 1 | +0x04 | `AddRef()` | 0 | EAX | `0x005F47F0` (shared) |
| 2 | +0x08 | `Release()` | 0 | EAX | `0x00479CE0` |
| 3 | +0x0C | `GetID()` | 0 | EAX | `0x004777E0` (shared) |
| 4 | +0x10 | `GetName(cIGZString&)` | 4 | EAX string& | `0x004777F0` (tail call) |
| 5 | +0x14 | `BelongsToFamily(id)` | 4 | AL | `0x00477A90` |
| 6 | +0x18 | `GetFamilyIDArray()` | 0 | EAX | `0x00477810` (shared) |
| 7 | +0x1C | `GetSize(w&, d&)` | 8 | — | `0x00477820` |
| 8 | +0x20 | `GetMaximumSlope()` | 0 | ST0 | `0x00477840` (shared) |
| 9 | +0x24 | `GetMinimumSlope()` | 0 | ST0 | `0x00477850` (shared) |
| 10 | +0x28 | `GetMaximumSlopeBeforeFoundation()` | 0 | ST0 | `0x00477860` (shared) |
| 11 | +0x2C | `GetGrowthStage()` | 0 | AL | `0x00477870` |
| 12 | +0x30 | `GetMinBuildingCapacity()` | 0 | AX | `0x004798E0` |
| 13 | +0x34 | `GetMaxBuildingCapacity()` | 0 | AX | `0x004798F0` |
| 14 | +0x38 | `IsCompatibleWithWater()` | 0 | AL | `0x00477880` |
| 15 | +0x3C | `IsCompatibleWithZoneType(type)` | 4 | AL | `0x00477D00` |
| 16 | +0x40 | `IsCompatibleWithWealthType(wealth)` | 4 | AL | `0x00477D50` |
| 17 | +0x44 | `IsCompatibleWithPurposeType(purpose)` | 4 | AL | `0x00477DA0` |
| 18 | +0x48 | `IsCompatibleWithStyleType(group)` | 4 | AL | `0x00477AC0` |
| 19 | +0x4C | `IsCompatibleWithMonopolyPieceUse(unused)` | **4** | AL = 1 | `0x004FC900` (shared) |
| 20 | +0x50 | `IsDoConstruction()` | 0 | AL | `0x00479900` |
| 21 | +0x54 | `GetRequiredRoads(vector<bool>&)` | 4 | — | `0x00478200` |
| 22 | +0x58 | `GetLotConfigurationObjectCount()` | 0 | EAX | `0x00477AF0` |
| 23 | +0x5C | `GetLotConfigurationObjectArray()` | 0 | EAX | `0x00477890` (shared) |
| 24 | +0x60 | `GetPrimaryBuildingConfigurationObject()` | 0 | EAX | `0x00477DF0` |
| 25 | +0x64 | `GetPrimaryBaseConfigurationObject()` | 0 | EAX | `0x00477E00` |
| 26 | +0x68 | `GetPrimaryFoundationConfigurationObject()` | 0 | EAX | `0x00477E10` |
| 27 | +0x6C | `HasObjectsOfType(type)` | 4 | AL | `0x00477B60` |
| 28 | +0x70 | `GetRetainingWallIDArray()` | 0 | EAX | `0x004778A0` (shared) |
| 29 | +0x74 | `GetFenceIDArray()` | 0 | EAX | `0x004778B0` |
| 30 | +0x78 | `GetFenceSegmentPositionArray()` | 0 | EAX | `0x004778C0` |
| 31 | +0x7C | `Load(key const&, detail)` | 8 | AL | `0x00479F70` |
| 32 | +0x80 | `Save(key const&, segment)` | 8 | AL | `0x00479580` |
| 33 | +0x84 | deleting destructor (flags) | 4 | EAX | `0x00479F50` |

"Shared" = identical code folded with other classes' methods (the ABI table's `icf::` names). "—" = no defined
return value although the SDK declares bool.

**SDK errors:** slot 19 pops 4 (SDK: no argument); slot 4 returns the string reference, not a bool; slots 7 and 21
return nothing defined; slot 31's second argument is the city detail level (1–3), not a "prop array size"
(prop-lotconfig §B3.2); the vtable has a 34th slot (the deleting destructor) past the SDK's 33.

Proposed `docs/abi` rows (lane files under `build-runs/`): `re-specs15a-abi-rows.tsv` (33 replacements of the
`cISC4LotConfiguration` rows, slots 0–32), `re-specs15a-held-rows.tsv` (slot 33, past the SDK slot count) and
`re-specs15a-layouts.md` (a `layouts.md` section for the 0xC0-byte object).

## 9. Quirks (reproduced unless marked *[choice]*)

1. Slot 19 always answers true, takes and ignores one dword, and does not read the monopoly flag +0x8C, which
   nothing reads (§5.4).
2. `Load` forces road side 3 on; a property without bit 3 still requires the south edge (§3.3).
3. List values past a bit set's size are dropped; purpose 9 cannot be stored (§3.3).
4. Slots 15–17 do not bound-check their argument (§5.2; *[choice]* false outside).
5. Fences: `Load` skips index 1 and reads unchecked; `Save` writes the types from index 1 (§3.4; *[choice]* bounded
   reads).
6. The foundation object's ID is never written (§3.7; *[choice]* 0).
7. The position correction moves only centres below 0.04 m from the lot edge; the footprint moves with them and
   may stay outside (§3.6).
8. A size property with fewer than two values leaves the size (0, 0), and `Load` still succeeds; the zone
   developer's width test then divides by zero (zones §7.2.1 step 7).
9. Capacity defaults `0xFFFF` / 0 and an empty style list until the manager's city-init pass (§4); "no building
   type found" gives 0 / 0.
10. `GetName` returns a string reference; `GetSize` and `GetRequiredRoads` return nothing defined (§8).
11. `Save` changes the live configuration (y := 0) (§7).
12. A configuration whose version is not 2 is written back into its resource at load (§3.1 step 10).

## 10. Open questions

- ~~**LC1:** the fence lists (slots 29, 30): no simulation reader was found and no shipped configuration has fences.
  A caller search of the place-lot tool and the lot developer for vtable offsets +0x74 / +0x78 on a configuration
  would settle whether anything draws them. Low priority.~~ **ANSWERED 2026-10-16 (re-answers18):** nothing draws
  them. The getters' only references are the vtable entries `0x00A8CF2C`/`0x00A8CF30`. A scan of every
  `call [reg+0x74]` and `call [reg+0x78]` in the exe found none on a configuration: the lot-related ones take
  arguments or return floats. Only `Load`/`Save` and the debug configuration editor (`ToggleFence` `0x0047E890`,
  `0x00484700`, direct field access) touch the vectors. The §3.4 choice stands. §3.4.

## 11. Conformance tests

Exemplars below are in group `0xA8FBD372`; "version 2" means `0x88EDC789` = 2 (u8). Detail 3 unless stated.

1. **Defaults.** An exemplar with instance `0x1234`, name "T" and only version 2: `Load` true; ID `0x1234`; name
   "T"; size (0, 0); stage 1; slopes 90 / 0 / 90; water false; zone, wealth and purpose bits all false; family list
   empty; road bits 8 (`GetRequiredRoads` → [false, false, false, true]); object count 0; slots 24–26 null; slot 12
   `0xFFFF`, slot 13 0; slot 18(`0x2000`) false; slot 19(any) true.
2. **No version.** The same without `0x88EDC789`, or with value 0: `Load` false. With value 1: `Load` true (and a
   rewrite is requested, §3.1 step 10).
3. **Bit sets.** Zone list `{1, 2, 3, 200}` (u8): slot 15 true for 1, 2, 3, false for 0 and 4; 200 is dropped.
   Purpose list `{9}`: no purpose bit. Wealth list `{2}`: slot 16(2) true, slot 16(1) false.
4. **Roads.** `0x4A4A88F0` = 1 → stored 9; `GetRequiredRoads` on a 7-element vector → resized to
   [true, false, false, true]. Value 12 → [false, false, true, true].
5. **Inheritance.** A configuration with no slope properties whose parent cohort holds `0xE99B068C` = 25.0,
   `0x699B08A4` = 0.0, `0x88EDC792` = 3.0, `0x88EDC798` = `{0xC96D2136}`, `0x88FCD877` = `0x890B7314`: slots
   8 / 9 / 10 give 25 / 0 / 3; slot 28 gives `[0xC96D2136]`; the last object is the foundation object of §3.7 with
   +0x20 = `0x890B7314`, and slot 26 returns it.
6. **Shipped residential lot** `0x300004F0` "R$$5_1x1" (`SimCity_1.dat`): size (1, 1); stage 3; zones {1, 2, 3};
   wealth {2}; purpose {1}; family list `[0x1000000F]`; slopes 16 / 0 / 16; roads 8; walls `[0xC96D2136]`. Ten
   object properties of types 1, 4, 1, 1, 1, 1, 1, 0, 2, 2 with `v1` = 16, 32, 33, 32, 32, 32, 32, 0, 32, 0. Object
   count: detail 3 → 11 (the ten plus the foundation `0x890B7314`); detail 2 → 4 (prop with `v1` 16, building,
   last texture, foundation); detail 1 → 3 (building, last texture, foundation). The building record: x =
   `0x67BC4 · 2⁻¹⁶` = 6.48345947265625, z = `0x86A09 · 2⁻¹⁶` = 8.4141998291015625, rotation 2, object ID
   `0x09C6DA71`, IDs `[0x1000000F]`. The object with `v1` 33 is an optional prop (flag bit 0).
7. **Shipped civic lot** `0x03000001` "Uw1x1_Water Tower": zones {15}; wealth {0}; purpose {0}; slopes 50 / 0 / 0;
   at every detail level 3 objects: building (x 8.0, z 8.058380126953125, rotation 2), texture (`v1` 0), foundation.
8. **Clamp, low side** (24-bit arithmetic). Size (1, 1); an object with x = −1.0 (`v3` = −65536), rect min x −2.0,
   max x 0.0: `loX` = `0x3D23D70B`; `d` = 1.04 (`0x3F851EB8`); x becomes 0.039999962 (`0x3D23D700`, **not**
   `loX`: the 24-bit rounding of `d` loses the low bits); rect min x −0.96000004 (`0xBF75C290`), max x 1.04
   (`0x3F851EB8`). At 53-bit precision the same input would give x = `0x3D23D70B`. An object with x = 0.0 becomes
   exactly `0x3D23D70B`.
9. **Clamp, high side.** Size (1, 1); x = 16.5, rect 16.0 … 17.0: `hiX` = 15.96 (`0x417F5C29`); `d` = −0.54
   (`0xBF0A3D70`); x = 15.96 (`0x417F5C29`); rect min 15.46 (`0x41775C29`), max 16.46 (`0x4183AE14`). z = 8.0 is
   unchanged. An x of exactly `hiX` or a NaN x is unchanged.
10. **Foundation object is not clamped or filtered.** Size (1, 1) at detail 1 with `0x88FCD877` present: the last
    record is type 3 at (0.5, 0, 0.5) with rect max (0.6, 0.6), min (0.4, 0.4), whatever the detail.
11. **Fence asymmetry.** Fence list `[1, 0xAAAA, 0xBBBB, 0]`: fence types `[0xBBBB]` (index 2), then `m` = v[3] = 0,
    no segments. `Save` of a configuration with fence types `[0xBBBB]` and no segments writes `[1, 0xBBBB, 0]`,
    which reads back as fence types `[0]` and no segment count (index 3 is past the end).
12. **Building information.** A configuration whose building object lists family F with types A (capacity 30,
    groups {0x1000, 0x2001}) and B (capacity 12, group {0x2000}, monopoly true): after the city-init message slot
    12 = 12, slot 13 = 30, slot 18 true for 0x1000, 0x2000, 0x2001 and false for 0x2002, +0x8C = 1 (A lacks the
    property). With no type in F: slots 12 and 13 both 0.
13. **Primary objects.** Objects in order texture T1, building, texture T2, foundation: slot 25 → T1; slot 24 →
    the building; slot 26 → the foundation; slot 27(2) true, slot 27(7) false, slot 27(0x102) false.
14. **Monopoly slot.** Slot 19 called with one pushed dword returns true and leaves the stack balanced; with no
    pushed argument the caller's stack is off by 4 (the original's behaviour; a replacement must keep pop 4).
