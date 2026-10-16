# Network bridges (behaviour spec)

Status: M5 behaviour spec, written 2026-10-16 (re-specs17a lane). It is the follow-up that network-geometry.md §7.1
outlined (NG2). Every fact was checked on the Windows executable `bin/SimCity4.exe` (1.1.641, image base
`0x00400000`), and every address is a Windows address. Method names are the Mac build's. Where the Mac and Windows
symbol matcher did not pair a function, the name was given by role *[inferred]*: the Mac class has exactly one
method of that name with that role. Numeric details (float or double, operation order, constants, compare
directions) were checked against the instruction bytes. Implementers work from this spec, the public LGPL gzcom-dll
SDK headers and general knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags:

- *[compat]*: observable; a replacement must match it.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: consistent with all the evidence, but not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.

`r24(e)` is `e` rounded to float32 (foundations §18). The FPU runs at 24-bit precision, so every intermediate x87
result below is an `r24` value. Every float compare below was read from its flag test. "Greater" means strictly
greater, and a NaN never passes a "greater" or "less" test.

Scope. Everything the network tool does about bridges:

- the **bridge types**: the INI sections, the exemplar properties, and the specialisation files (§2, §3);
- the **water** bridge pass `IdentifyBridgeCells` and its span selection, span adjustment and type choice (§4–§6);
- the **land** bridge pass `IdentifyLandBridgeCells` (§7);
- the **elevations** (§8);
- the two **models** that turn a span into pieces: specialised and parametric (§9, §10);
- the **pieces**, their occupants and their **pillars** (support columns) (§11);
- the **cost** and the **failure codes** (§12, §13);
- the tool's bridge getters and setters (§1).

What is not here: the bridge occupant class (`cSC4NetworkBridgeOccupant`, `0x00604BB0` onwards) beyond the calls the
tool makes; the cable and suspension-wire geometry, which is visual (§11.4, NB2); the player's bridge menu
(`cSC4NetworkToolUI::AddBridgeEntry`, `0x00646260`).

Related specs (cited, not repeated):

- [network-geometry.md](network-geometry.md): the steps, cells and kinds (§2.2), the tool fields (§2.1), the
  placement parameters (§2.4), the pass order (§4), step heights and the vertex offset tables (§6.1), cell infos
  (§8.1), the height solver (§9), supports and flags (§10).
- [network-tools.md](network-tools.md): `PlaceNetwork` (§5), the cost (§8.1), the tool fields (§2).
- [network-manager.md](network-manager.md): network types (§3.1), network flags (§5).
- [terrain.md](terrain.md): the slot table (slots 12, 17, 18, 30, 34).
- [occupant-manager.md](occupant-manager.md): slot 40 `GetOccupantsByStandardCityCells` (§8.4), slot 48
  `ReleaseOccupantList` (§9.5).
- [foundations.md](foundations.md) §18: x87 precision.

---

## 1. Identity and slots

### 1.1 cSC4NetworkTool bridge slots (base vtable `0x00AA8748`)

These slots are in the SDK range (network-tools.md §14). The names are the Mac build's (Mac and Windows agree slot for
slot up to 50). The pops were read from each target's `ret`.

| Slot | Offset | Address | Method | Pop | Behaviour |
|---|---|---|---|---|---|
| 30 | +0x78 | `0x00626E80` | `GetPossibleBridges(&list)` | 4 | copies the candidate list `+0x1F4` (§6.3) into `list`; false if it is empty |
| 31 | +0x7C | `0x00623B70` | `GetPreferredBridge()` | 0 | returns `+0x200` |
| 32 | +0x80 | `0x00623B80` | `SetPreferredBridge(index)` | 4 | `+0x200 := index` |
| 33 | +0x84 | `0x00623B90` | `GetBridgeAltitudeOverride()` | 0 | returns `+0x204` in ST0 |
| 34 | +0x88 | `0x00623BA0` | `SetBridgeAltitudeOverride(h)` | 4 | `+0x204 := h` |
| 35 | +0x8C | `0x00624AF0` | `GetBridgeName(index, &string)` | 8 | the bridge type's name (`+0x15C`) |
| 36 | +0x90 | `0x00624130` | `GetMajorBridge()` | 0 | the **last** bridge record's bridge type index (`+0x0C`); −1 if there is no record |
| 37 | +0x94 | `0x00624180` | `GetMajorBridgeAltitude(&h)` | 4 | `h :=` last record `+0x10`; false if there is no record |
| 38 | +0x98 | `0x00624150` | `GetMajorBridgeReferenceAltitude(&h)` | 4 | `h :=` last record `+0x14`; false if there is no record |
| 39 | +0x9C | `0x006241B0` | `IsMajorBridgeLandBridge()` | 0 | last record's byte `+0x18`; false if there is no record |
| 40 | +0xA0 | `0x00624B20` | `GetBridgeBitmapKey(index, &a, &b)` | 12 | the bridge type's `+0x148` and `+0x14C`; false (outputs untouched) if either is 0 |

The "major" bridge is simply the last record appended in the run. `index` in slots 35 and 40 is an index into the
network type's bridge type vector, without a bounds check.

`Reset` sets `+0x200 := −1` (no preference, `0x00627006`) and `+0x204 := −1.0` (no override, `0x00627010`).

### 1.2 Internal functions

| Address | Mac name | Section |
|---|---|---|
| `0x0062F9D0` | `IdentifyWaterAndLandBridgeTypes` *[inferred]* | §2.5 |
| `0x0062FDB0` | (static step test) | §4.2 |
| `0x0062EB80` | `CheckBridgeStepEdgeTypes` *[inferred]* | §4.3 |
| `0x00632830` | `IdentifyBridgeCells` | §4 |
| `0x00630570` | `SelectBridgeStartAndEnd` *[inferred]* | §5.1 |
| `0x0062FED0` | `AdjustBridgeSpan` *[inferred]* | §5.2 |
| `0x0062F760` | `FindBestBridgeType` *[inferred]* | §6 |
| `0x0062ECD0` | `PadStepsToMakeMinBridgeSpan` *[inferred]* | §6.2 |
| `0x00632AD0` | `IdentifyLandBridgeCells` | §7 |
| `0x00632580` | `CreateLandBridge` *[inferred]* | §7.3 |
| `0x0062EA90` | `SelectBridgeElevation` *[inferred]* | §8.2 |
| `0x00632060` | `ConstructSpecializedBridge` *[inferred]* | §9 |
| `0x00631300` | `ConstructParametricBridge` *[inferred]* | §10 |
| `0x0062F610` | `InsertBridgePieces` | §11 |
| `0x0062EEB0` | `InsertBridgePiece` | §11.2 |
| `0x0062DF30` | `AddSuspendedBridgeStructures` | §11.4 |
| `0x0062E690`, `0x00630F70` | (suspension profile, cable plotting) | §11.4 |
| `0x006437A0` | `cSC4NetworkTypeInfo::LoadBridgeTypes` | §2.1 |
| `0x00642220` | `ReadBridgeTypeInfo` | §2.2 |
| `0x006436D0` | `ReadBridgeTypeSpecializations` | §3 |

---

## 2. Bridge types

### 2.1 The INI sections

`ReloadNetworkTypeInfo` (`0x00623310`) loads the bridge types of each network type with `LoadBridgeTypes(ini,
section, specialisation instance, pieces per step W)`. The INI is the network INI resource (type 0, group
`0x8A5971C5`, instance `0x8A5993B9`).

| Type | Section | Specialisation instance | W | Shipped bridge types |
|---|---|---|---|---|
| 0 road | `RoadBridgeExemplars` | `0x1000` | 1 | 9 |
| 1 rail | `RailBridgeExemplars` | `0x1001` | 1 | 5 |
| 2 highway | `HighwayBridgeExemplars` | `0x1002` | 2 | 11 |
| 3 street | `StreetBridgeExemplars` | `0x1003` | 1 | 0 |
| 4 pipe | `PipeBridgeExemplars` | `0x1004` | 1 | 0 |
| 5 power line | none | | | 0 |
| 6 avenue | `AvenueBridgeExemplars` | `0x1006` | 2 | 9 |
| 7 subway | `SubwayBridgeExemplars` | `0x1007` | 1 | 0 |
| 8 light rail | `LightRailBridgeExemplars` | `0x1008` | 1 | 1 |
| 9 monorail | `MonorailBridgeExemplars` | `0x1009` | 1 | 1 |
| 10 one-way road | `OneWayRoadBridgeExemplars` | `0x100A` | 1 | 0 |
| 11 dirt road | `DirtRoadBridgeExemplars` | `0x100B` | 1 | 0 |
| 12 ground highway | `GroundHighwayBridgeExemplars` | `0x100C` | 2 | 11 |

The specialisation instance is `0x1000 + type`. The section lookup lowercases the name (`0x0091D91C`), so the
shipped `[MonoRailBridgeExemplars]` matches. Entries are read in file order; the key is the text before `=` and the
value is the trimmed text after it.

Street, pipe, subway, one-way road and dirt road have no shipped section, so they never get a bridge *[compat]*:
`IdentifyBridgeCells` needs at least one bridge type (§4.1).

### 2.2 One entry: ReadBridgeTypeInfo (`0x00642220`)

An entry has the form `KEY = TYPE, GROUP, INSTANCE, NAME` (shipped: `1 = 0x6534284a, 0xa82ca30f, 0x00000001, level
road bridge`). It is read as follows:

1. Append a default bridge type record (`0x178` bytes, §2.3) to the network type's vector (type info `+0x30`)
   **before** parsing.
2. `+0x00 := ` the key as an unsigned number.
3. Split the value on `,`. Unless there are exactly 4 tokens, return, **leaving the default record in the vector**
   *[quirk]*. No shipped entry has another count.
4. Tokens 0–2 are the exemplar's type, group and instance (hex). Token 3 is the bridge's name, stored at `+0x15C`.
5. Look the exemplar up through the resource manager. If that fails, the record is removed again.
6. Read the properties (§2.3). If the exemplar has `0x482CD466`, the name is replaced by the localised string it
   names.

Every shipped bridge exemplar is type `0x6534284A`, group `0xA82CA30F`; there are 47, all in `SimCity_1.dat`.

### 2.3 The bridge type record (`0x178` bytes)

"Default" is the value when the property is absent. Integers are u32 unless stated.

| Offset | Property | Default | Meaning |
|---|---|---|---|
| `+0x00` | — | | the INI key |
| `+0x04` | `0x482CB458` | 2N + 2 | **minimum span** in steps |
| `+0x08` | `0x482CB467` | 100000 | **maximum span** in steps |
| `+0x0C` | `0x482CB45F` | 1 | W, pieces per step (lanes) |
| `+0x10` | `0x482CB442` | 0 | N, **ramp** steps at each end |
| `+0x14` | `0x482CB443`, `0x482CB444` | | N × W ramp pieces `{u32 instance, u8 rotation}`. The rotation count is truncated to a byte (`0x00642567`) *[quirk]* |
| `+0x20` | `0x482CB44D` | | N floats: ramp **pillar depths** (−1 = none) |
| `+0x2C` | `0x482CB445` | | the **deck** piece instance |
| `+0x30` | `0x482CB446` | | u8, the deck piece rotation |
| `+0x34` | `0x482CB447` | 0 | **pier spacing** in steps |
| `+0x38` | `0x482CB459` | `+0x34` | first-pier offset in steps |
| `+0x3C` | `0x482CD465` | 0 | float, the end-pier fraction (§10.2) |
| `+0x40` | `0x482CB448` | | M, steps per **pier** group |
| `+0x44` | `0x482CB449`, `0x482CB44A` | | M × W pier pieces `{instance, rotation}` |
| `+0x50` | `0x482CB44C` | | M floats: pier pillar depths |
| `+0x5C` | `0x482CB441` | | float, the deck height above **water** |
| `+0x60` | from `0x482CB44B` | | N + 1 floats: the cumulative ramp drops (§2.4) |
| `+0x6C` | from `0x482CB44B` | | N floats: the ramp model offsets (§2.4) |
| `+0x78` | | 0 | float, `+0x60[0]`: the total ramp rise, the deck height above **land** |
| `+0x7C` | | 0 | the last ramp index i ≥ 1 whose pillar depth is ≥ 0, else 0 |
| `+0x80` | `0x482CB44E` | | the pillar **footprint** for ramp and deck pieces: 4 (x, z) points, then the same rotated by 90°, 180° and 270° (π/2 = `0x00AA8D30`) |
| `+0x8C` | `0x482CB45E` | `+0x80`'s points | the same for **pier** pieces |
| `+0x98` | `0x482CB44F` | | bool: a **land** bridge type |
| `+0x99` | `0x482CB450` | | bool: a **water** bridge type |
| `+0x9C` | `0x482CB455` | 0 | float, suspension parameter between two pylons (§11.4) |
| `+0xA0` | `0x482CB456` | 0 | float, suspension parameter between a pylon and an end (§11.4) |
| `+0xA4` | `0x482CB457` | 0 | float, copied into each supported piece (§9.3) |
| `+0xA8` | `0x482CB45A` | 0 | minimum pier count (§10.2) |
| `+0xAC` | `0x482CB45B` | 100 | maximum pier count (§10.2) |
| `+0xB0`, `+0xB4` | `0x482CB45C`, `0x482CB45D` | 0 | the two cable model ids; both non-zero = a **cable** bridge (§10.4) |
| `+0xB8`–`+0xC4` | `0x482CB471`, `0x482CD460`, `0x482CD461`, `0x482CD462` | | floats for the cables; `+0xC4` is also added to a pier piece's model height (§10.4) |
| `+0xC8` | `0x482CD463` | 0.0 | minimum **clearance** (§6) |
| `+0xCC` | `0x482CD464` | 10000.0 | maximum clearance |
| `+0xD0` | `0x482CB468` | | float, the model bound offset (§11.2) |
| `+0xD4` | `0x482CB472` | | float, added to every piece's model height |
| `+0xD8`–`+0x144` | `0x482CB46B`, `0x482CB46C`, `0x482CB469`, `0x482CB46A`, `0x482CB46D`, `0x482CB46E` | | the suspension tower data (visual, §11.4). `+0xD8` is a byte: all of it present |
| `+0x148`, `+0x14C` | `0x482CB46F` | 0, 0 | the menu bitmap key (slot 40) |
| `+0x150`, `+0x154` | `0x482CB470` | 0, 0 | the **cost**: value 0 is the fixed cost, value 1 the cost per step (§12) |
| `+0x15C` | | | the name |
| `+0x16C` | | empty | the **length map**: step count → specialisation (§3) |

The shipped exemplars set none of `0x482CB467`, `0x482CD463`, `0x482CD464` and `0x482CB470`. So every shipped bridge
type has the maximum span 100000, the clearance range [0, 10000] and the cost (0, 0) *[compat]*. Properties
`0x482CB451`–`0x482CB454` are present on some exemplars but never read. Every shipped exemplar sets both
`0x482CB44F` and `0x482CB450` to true.

### 2.4 The ramp heights

`0x482CB44B` holds N pairs `(a_i, b_i)`. From the last ramp step back (`0x00642907`–`0x00642949`):

- `+0x60[N] := 0`;
- `+0x60[i] := r24((b_i − a_i) + +0x60[i + 1])`;
- `+0x6C[i] := r24(+0x60[i + 1] + b_i)`.

`+0x78 := +0x60[0]` (0 when N = 0). Ramp step i (0 at the bank) is drawn `+0x60[i]` below the deck at its outer edge
and `+0x60[i + 1]` below it at its inner edge (§10.3).

### 2.5 The water and land lists

`Init` (`0x00626B19`) calls `0x0062F9D0`, which fills two lists of bridge type indices for the tool's network type:
`+0x33C` holds those with `+0x99` (water), `+0x348` those with `+0x98` (land), in vector order.

Then `Init` sets the tool's **land-bridge flag** `+0x354` (`0x00626B24`–`0x00626B5E`): it clears it, and for each
bridge type it tests the **address** of the type's `+0x98` field instead of the byte stored there. The address is
never null, so `+0x354` is set exactly when the network type has at least one bridge type *[quirk]*. Every shipped
type sets `+0x98`, so reading the byte would give the same result: the quirk has no shipped effect.

### 2.6 Shipped values

The columns used by the tests (§16). Keys are the INI keys; "inst" is the exemplar instance.

| Network | Key | Inst | Name | Min span | +0x5C | N | M | W | +0x34 | +0x38 | +0x7C |
|---|---|---|---|---|---|---|---|---|---|---|---|
| road | 1 | `0x1` | level | 10 | 20 | 3 | 3 | 1 | 6 | 6 | 0 |
| road | 2 | `0x2` | raised | 16 | 0 | 7 | 1 | 1 | 5 | 5 | 4 |
| road | 3 | `0x3` | undertruss | 4 | 20 | 1 | 1 | 1 | 4 | 4 | 0 |
| road | 4 | `0x5` | wooden covered | 4 | 20 | 1 | 1 | 1 | 2 | 2 | 0 |
| road | 5 | `0x6` | medium suspension | 22 | 35 | 1 | 1 | 1 | 16 | 6 | 0 |
| road | 6 | `0x7` | stone arch | 4 | 20 | 1 | 1 | 1 | 4 | 4 | 0 |
| road | 7 | `0x8` | small steel arch | 10 | 20 | 1 | 1 | 1 | 4 | 4 | 0 |
| road | 8 | `0x9` | cantilever truss | 12 | 20 | 1 | 1 | 1 | 4 | 4 | 0 |
| road | 9 | `0xA` | overdeck truss | 4 | 20 | 1 | 1 | 1 | 6 | 6 | 0 |
| rail | 1 | `0x4` | overtruss | 4 | 15.5 | 1 | 1 | 1 | 3 | 3 | 0 |
| highway | 1 | `0x02000001` | level | 4 | 0 | 0 | 1 | 2 | 5 | 5 | 0 |
| highway | 2 | `0x02000002` | Golden Gate | 43 | 35.119 | 3 | 1 | 2 | 35 | 6 | |
| highway | 5 | `0x02000005` | cable stayed | 32 | 45 | 1 | 1 | 2 | 15 | 7 | |
| light rail, monorail | 1 | `0x08000001`, `0x0D000001` | | 4 | 0 | 0 | 1 | 1 | 3 | 3 | 0 |

Road key 1: ramp pairs all (0, 0), so `+0x78` = 0; pier depths [498, −1, −1]; ramp depths [256, −1, −1]; `+0xD0` = 15.
Road key 2: ramp pairs (−1.747, 0), (−4.958, 0), (−7.686, 0), (−7.741, 0), (−6.789, 0), (−4.893, 0), (−2.838, 0), so
`+0x78` ≈ 36.652; ramp depths [100, −1, −1, 100, 100, −1, −1]. Ground highway's types repeat highway's with `+0xD4` =
−15.5. Rail has 5 types (instances `0x4`, `0x03000002`–`0x03000005`), highway 11 (`0x02000001`–`0x0200000B`),
avenue 9 (`0x04000001`–`0x04000009`), ground highway 11 (`0x0A000001`–`0x0A00000B`).

---

## 3. Specialisations

`ReadBridgeTypeSpecializations` (`0x006436D0`) reads the text resource type `0x0A5BCF4B`, group `0xAA5BCF57`,
instance `0x1000 + type`, and parses it (`Parse`, `0x00643580`). Shipped: `0x1000` (road), `0x1001` (rail), `0x1002`
(highway), `0x1006` (avenue), `0x100C` (ground highway). A **specialisation** is a fixed piece list for one exact span
length of one bridge type; it replaces the parametric model for that length (§4.4).

### 3.1 Grammar

Tokens are separated by white space; `<`, `>`, `,`, `:`, `*`, `(` and `)` are tokens of their own (`NextToken`,
`0x0063DDF0`). At the top level:

- `stop` ends the parse;
- `section NAME … end` defines a **global** section (a piece list) by name (hash map, `0x00640E10`);
- `bridge KEY … end` opens the bridge type whose INI key is KEY. If there is none, the parse **fails**, and the rest of
  the file is ignored.

Inside `bridge` (`ParseBridge`, `0x00643330`):

- `section NAME … end`: a section visible only inside this bridge;
- `length L … end`: the specialisation for span length L. L must be in [min span, max span] and not already in the
  length map, or the whole parse fails. It creates `map[L]`;
- `end` closes the bridge. Any other token fails the parse.

A body (`ParseBridgeSpecialization`, `0x0063EA50`) is a list of lines:

- `height H` (integer or float): added at the end to every piece height and every step height of this body;
- `piece ITEM ITEM …` to the end of the line, where each ITEM may be prefixed by `COUNT *` to repeat it. An ITEM is:
  - `<INSTANCE, ROTATION[, HTDELTA[, YOFFSET]]>` optionally followed by `:OPT{,OPT}`, where OPT is a number (the pillar
    depth), `pylon` (support kind 1) or `end` (support kind 2) (`ParseBridgeInstance`, `0x00641310`);
  - a section name, or `rev(NAME)`;
  - inside a bridge only: `length(L)` or `rev(length(L))`, naming a length already defined.

### 3.2 What a body builds

- A **piece**: 20 bytes, `{float Y, u32 instance, u8 rotation, float depth (−1 when not given), u32 support kind}`.
- A **step height** list: it starts as [0]. After every W pieces it gets `last + HTDELTA` (the HTDELTA of the piece
  that completes the step).
- A piece's Y is `YOFFSET + ` the last step height at the time it is read.
- A reference (section or length) appends that list's pieces and heights, rebased on the current last height. `rev`
  appends them in reverse order, XORs each rotation with 2 and rebases against the referenced list's end height.

The resulting map entry holds the pieces and the heights (piece vector at entry `+0x00`, heights at `+0x0C`; the
entry is passed as `map node + 0x14`).

---
