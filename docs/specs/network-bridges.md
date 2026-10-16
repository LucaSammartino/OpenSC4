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
| `+0x20` | `0x482CB44D` | | N floats: ramp **model depths** (−1 = never a column, §11.3) |
| `+0x2C` | `0x482CB445` | | the **deck** piece instance |
| `+0x30` | `0x482CB446` | | u8, the deck piece rotation |
| `+0x34` | `0x482CB447` | 0 | **pier spacing** in steps |
| `+0x38` | `0x482CB459` | `+0x34` | first-pier offset in steps |
| `+0x3C` | `0x482CD465` | 0 | float, the end-pier fraction (§10.2) |
| `+0x40` | `0x482CB448` | | M, steps per **pier** group |
| `+0x44` | `0x482CB449`, `0x482CB44A` | | M × W pier pieces `{instance, rotation}` |
| `+0x50` | `0x482CB44C` | | M floats: pier model depths |
| `+0x5C` | `0x482CB441` | | float, the deck height above **water** |
| `+0x60` | from `0x482CB44B` | | N + 1 floats: the cumulative ramp drops (§2.4) |
| `+0x6C` | from `0x482CB44B` | | N floats: the ramp model offsets (§2.4) |
| `+0x78` | | 0 | float, `+0x60[0]`: the total ramp rise, the deck height above **land** |
| `+0x7C` | | 0 | the last ramp index i ≥ 1 whose model depth is ≥ 0, else 0 |
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
  - `<INSTANCE, ROTATION[, HTDELTA[, YOFFSET]]>` optionally followed by `:OPT{,OPT}`, where OPT is a number (the model
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

### 3.3 Shipped coverage

Lengths with a specialisation (keys as in §2.6); every other length of every type is parametric:

| Network | Key: lengths |
|---|---|
| road (`0x1000`) | 3: 4–16; 6, 9: 4–255; 7: 10–255; 8: 12–255 |
| rail (`0x1001`) | 3: 10–255; 4: 12–255; 5: 4–255 |
| highway (`0x1002`) and ground highway (`0x100C`) | 3: 43–111; 6: 32–255; 8, 11: 4–255; 9: 10–255; 10: 30–255 |
| avenue (`0x1006`) | 1: 30–255; 2: 43–111; 3, 9: 4–255; 6: 10–255; 7: 32–255 |

The water pass never builds a span shorter than 5 steps (§4.4), so a length-4 specialisation is reached only by a
land bridge (§7.3).

---

## 4. The water pass: IdentifyBridgeCells (`0x00632830`)

### 4.1 Preconditions

`IdentifyBridgeCells(&v, type)` (pop 8) is pass 5 of `PlaceNetwork` (network-geometry.md §4). It does nothing unless:

- `AllowBridges` (slot 61) is true;
- `v ≥ 4` and `v ≤ +0x74`, the **straight** step count (`0x0063284C`–`0x00632858`). network-geometry.md §7 words
  this as "dragged count ≤ straight count"; the test reads the valid count `v`, which at this point is at most the
  dragged count;
- the network type has at least one bridge type (type info `+0x30` not empty).

### 4.2 The step test (`0x0062FDB0`)

A step is **clear** when the occupant manager finds no occupant passing the tool's OR filter (`+0x364`) in the
rectangle of the step's first and last cells, grown by one cell on every side: occupant manager slot 40 with
`maxCount` −1 (occupant-manager.md §8.4). A failed query also counts as clear. The list is released with slot 48.

The OR filter is made in `Init` (`0x00626AB9`–`0x00626AE9`). It passes a network occupant (type `0xC772BF98`) whose
network-flag test `HasAnyNetworkFlag(mask)` (network occupant slot 9, `0x0061F560`) is true. The mask is `0x1F4F`
(road, rail, highway, street, avenue, light rail, monorail, one-way road, dirt road and ground highway) for those
types, and only the type's own bit (`1 << type`) for pipe, power line and subway.

### 4.3 The edge test: CheckBridgeStepEdgeTypes(step, type) (`0x0062EB80`)

For every cell of the step that already has a cell info (network-geometry.md §8.1), the step **fails** when:

- the cell info's type mask (`+0x08`) has a bit of the low 13 other than `1 << type`: another network is there; or
- the cell info's edge word for this type (`+0x18 + 4 × type`) has a bit outside the **allowed** mask below.

| Pen direction | one-cell step | first cell | middle cells | last cell |
|---|---|---|---|---|
| 0 or 4 (x) | `0x00020002` | `0x04020002` | `0x04020402` | `0x00020402` |
| 2 or 6 (z) | `0x02000200` | `0x02040200` | `0x02040204` | `0x02000204` |

"First" and "last" are the step's first and last cell indexes (network-geometry.md §2.2). Cells without a cell info
pass. In words: an existing piece of the same network is allowed only where it already runs straight along the drag
and, inside a wide step, joins its neighbour in the step.

### 4.4 The loop

`i` runs from 2 while `i < v`. For each `i`:

1. Skip `i` unless steps `i`, `i − 1` and `i − 2` are all kind 0 (regular).
2. Skip `i` unless step `i` is clear (§4.2), at least one of its cells has water (terrain slot 34), and
   `CheckBridgeStepEdgeTypes(i)` passes.
3. `SelectBridgeStartAndEnd(i, &start, &end, &index, &refElev, type)` (§5.1). If it fails, or `index < 0`: **cut** the
   run, `v := i`, and stop (`0x00632ABB`).
4. If `end − start ≥ 4` and `end − start + 1 ≥` the bridge type's minimum span (`0x006329C1`–`0x006329D4`):
   1. mark steps `start … end` kind 1;
   2. append a bridge record (§4.5) with the network type, start, end, index, reference elevation `refElev` at
      `+0x14` and the land byte `+0x18 := 0`;
   3. `SelectBridgeElevation` on it (§8.2);
   4. look `end − start + 1` up in the bridge type's length map `+0x16C` (`0x00632A75`). Found: the **specialised**
      model with the entry (§9). Not found: the **parametric** model (§10).
5. Otherwise nothing is recorded and the steps stay regular. The water steps are then refused later by
   `CheckForGoodTerrain` with `0xF0000001` (network-geometry.md §4.5) *[compat]*.

On exit `v` is written back: unchanged after a full loop, `i` after a cut. `PlaceNetwork` turns a lowered `v` into
the failure code `0xF0000003` (§13).

A recorded bridge covers its steps with kind 1, so `i = end + 1` and `i = end + 2` fail step 1. A wet step there
is skipped, not bridged, and `CheckForGoodTerrain` refuses it later: a second crossing must leave at least two dry
regular steps after the first bridge's last step.

### 4.5 The bridge record (`0x28` bytes, tool vector `+0xD8`)

| Offset | Meaning |
|---|---|
| `+0x00` | network type |
| `+0x04`, `+0x08` | start and end step (inclusive) |
| `+0x0C` | bridge type index (into type info `+0x30`) |
| `+0x10` | the **deck** elevation (§8.2; land: §7.3) |
| `+0x14` | the **reference** elevation (§5.2; land: the same as `+0x10`) |
| `+0x18` | byte: 1 = land bridge (§7.3), 0 = water bridge |
| `+0x1C`–`+0x24` | vector of piece records (`0x50` bytes each, §9.2) |

The vector is cleared at the start of each run (network-tools.md §5.1 step 2).

---

## 5. The water span

### 5.1 SelectBridgeStartAndEnd(i, &start, &end, &index, &refElev, type) (`0x00630570`, pop 24)

1. `start := i − 1`. Fail unless `start ≥ 0`, step `start` is kind 0 and passes the edge test. (Step `start` is not
   tested for occupants.)
2. `end := i + 1`. Fail unless `end < +0x78 − 1`.
3. Loop: fail unless step `end` is kind 0, clear and passes the edge test. If none of its cells has water, it is the
   far bank: go to 4. Otherwise `end := end + 1`, and fail if `end ≥ +0x78 − 1`.
4. Fail if `end ≥ +0x74` (the straight count).
5. `AdjustBridgeSpan(&start, &end, &clearance, &refElev, type)` (§5.2).
6. `index := FindBestBridgeType(&start, &end, water = 1, clearance, type)` (§6). Succeed when `index ≥ 0`.

So the first span is the wet run plus one dry step on each side.

### 5.2 AdjustBridgeSpan(&start, &end, &clearance, &elev, type) (`0x0062FED0`, pop 20)

It finds the banks' heights, chooses the reference elevation, then widens the span until it meets the banks.

Two thresholds come from the placement parameter p+0x00 (the maximum slope, **in radians**, network-geometry.md
§2.4): `c14 := r24(p0 × 14.0)` (`0x00A98F48`) and `c4 := r24(p0 × 4.0)` (`0x00A8E17C`). Road: p0 = 35° ≈ 0.61087, so
c14 ≈ 8.5521 and c4 ≈ 2.4435. Multiplying an angle by a plain constant suggests that a rise per cell was meant
*[quirk; inferred intent]*.

Two step measures:

- `lowOf(step)`: the largest, over the step's cells, of the cell's **lowest** terrain corner (terrain slot 17), but at
  least 0;
- `surfaceOf(step)`: the largest, over the step's cells, of the cell's surface level (terrain slot 18: the corner
  altitude plus any water depth), but at least 0.

**Back bank.** `A := 0`, `B := 0`, `low := 1`. For `k := start` down while `k > 1`:

1. If steps `k`, `k − 1` and `k − 2` are not all kind 0, or step `k` is not clear, or fails the edge test:
   `low := k` and stop.
2. `d := surfaceOf(k − 1) − lowOf(k)`. If `d > c14`: `A := surfaceOf(k − 1)`. If `d > c4`: `B := surfaceOf(k − 1)`.

Every step that qualifies overwrites A or B, so the **farthest** qualifying step wins.

**Far bank.** `A2 := 0`, `B2 := 0`, `far := +0x78 − 2`, `done := false`. For `j := end` while `j < +0x78 − 2`:

1. If steps `j`, `j + 1` and `j + 2` are not all kind 0, or step `j` is not clear, or fails the edge test:
   `far := j` and stop.
2. If `done`, go on with the next `j` (the tests in 1 still run).
3. `d := surfaceOf(j + 1) − lowOf(j)`. If `d > c14`: `A2 := surfaceOf(j + 1)`. If `d > c4`: `B2 := surfaceOf(j + 1)`;
   otherwise `done := true`.

**Elevation.** `elev := min(B, B2, max(A, A2))`. If the override `+0x204 ≥ 0.0`, `elev := +0x204` instead
(`0x0063030D`).

**Widen.** While `start > low` and `surfaceOf(start) < elev`: `start := start − 1`. While `end < far` and
`surfaceOf(end) < elev`: `end := end + 1`.

**Floor.** If `elev < 0.01` (a double, `0x00A92688`): `elev :=` the surface level (slot 18) of the first cell of step
`start`.

**Clearance.** `clearance := 0`. For each step `k` from `start` to `end − 1`: if the step has a wet cell, take the
first one, `c`, and `clearance := max(clearance, elev − surface(c))`.

On flat banks (every `d ≤ c4`), `elev` is 0, so the floor rule puts it at the near bank's surface, and the span is
not widened.

---

## 6. FindBestBridgeType(&start, &end, water, h, type) (`0x0062F760`, pop 20)

### 6.1 The choice

The candidate list is the tool's water list `+0x33C` when `water` is true, else the land list `+0x348` (§2.5).
`n := end − start + 1`. A bridge type is a candidate unless:

- a preferred bridge is set (`+0x200 ≥ 0`) and this is not it;
- `h < +0xC8` or `h > +0xCC` (the clearance range, both ends inclusive);
- `+0x08 < n` (too long);
- `+0x04 > n` (too short) and `PadStepsToMakeMinBridgeSpan(+0x04, …)` (§6.2) fails on **copies** of start and end.

No candidate: return −1. Otherwise score each candidate with `d := n − minSpan`, and `d := d − 100000` when `d < 0`.
The smallest `|d|` wins; the **first** candidate in list order wins a tie. If the winner's `d < 0`, run
`PadStepsToMakeMinBridgeSpan(minSpan, &start, &end, type)` on the **real** start and end (its result is ignored).
The tool's candidate list `+0x1F4` is replaced by the candidates, and the winner's index is returned.

So an exact fit wins, then the longest bridge type that fits without padding, and padding is a last resort.

### 6.2 PadStepsToMakeMinBridgeSpan(minLen, &start, &end, type) (`0x0062ECD0`)

If `end − start + 1 ≥ minLen`: true. Otherwise alternate one step back, then one step forward, while steps are still
needed:

- back: `start := start − 1`; it is allowed while `start > 1`, steps `start` and `start − 1` are kind 0 and step
  `start` passes the edge test; otherwise undo it and stop going back;
- forward: `end := end + 1`; it is allowed while `end < v`, step `end` is kind 0 and passes the edge test; otherwise
  undo it and stop going forward.

If both directions are blocked while steps are still needed: false. At the end: true when `end < +0x74` and the span
is at least `minLen`. Padding does not test for occupants.

### 6.3 The candidate list

`+0x1F4`–`+0x1FC` is a vector of bridge type indices. `GetPossibleBridges` (slot 30) copies it; the player's bridge
menu uses it with `SetPreferredBridge` (slot 32). It holds the candidates of the **last** `FindBestBridgeType` call in
the run.

---

## 7. The land pass: IdentifyLandBridgeCells (`0x00632AD0`)

### 7.1 Preconditions

`IdentifyLandBridgeCells(v, type)` (pop 8, `v` by value) is pass 7 of `PlaceNetwork`, run only when `+0x354` is set
(§2.5). It needs `AllowBridges`, and `limit := min(+0x74, v) − 1 ≥ 4`. It never changes `v` and sets no failure code.

### 7.2 The scan

For each step it reads two terrain vertex altitudes (terrain slot 12) at the **first** cell of the step: vertex
`z × +0x248 + x` plus `+0x250[d]` and plus `+0x270[d]`, with `d := pen & 3` (network-geometry.md §6.1). So `d` is 0
for pen 0 and 4 and 2 for pen 2 and 6: the scan always reads the cell's +x edge (x drags) or +z edge (z drags),
whichever way the drag runs *[quirk]*. `hi` and `lo` are the larger and smaller of the two.

State: `inSpan := false`, `A := 0`, `B := 0`, `spanStart`, `D`, `E`, `C`. For `i := 1` while `i < limit`:

1. If step `i` is not kind 0 or fails the edge test (§4.3): `inSpan := false`, next `i`. (A and B are not updated.)
2. Not in a span:
   - if `hi < A`: the ground **drops**. Start a span: `spanStart := i − 1`, `D := B`, `E := hi`, `C := FLT_MAX`,
     `inSpan := true`; next `i`;
   - otherwise `A := lo`, `B := hi`; next `i`.
3. In a span, if `hi < D` (still below the deck):
   - if `B < E` or `hi < E`: `C := min(C, lo)`; next `i`;
   - otherwise: `spanStart := spanStart + 1`, `D := E`, then the check (4).
4. In a span with `hi ≥ D`, and after 3's second branch: the **check**. If `i − spanStart + 1 ≥ 4`, `D − C > +0x2CC`
   and `|D − hi| ≤ +0x2D0`: `CreateLandBridge(spanStart, i, D, type)` (§7.3). In every case `inSpan := false`.

`A` and `B` are the low and high altitudes of the last step seen outside a span. `D` is the near bank's height (the
deck), `C` the lowest ground under the span. The first step of a span never lowers `C` (it is not in the span yet
when `C` is reset), so `C` comes from the second step on. `+0x2CC` is p+0x3C (`0xC804DD95`, the maximum height above
the graded terrain: 55 for road, one-way and dirt road, 30 for the others) and `+0x2D0` is p+0x40 (`0xC804DD96`,
read at `0x0063CD5B`; 20 for every shipped type). A land bridge therefore needs a valley deeper than 55 m (road) or
30 m, at least 4 steps from bank to bank, and a far bank within 20 m of the near bank's height.

### 7.3 CreateLandBridge(start, end, elev, type) (`0x00632580`, pop 16)

1. `index := FindBestBridgeType(&start, &end, water = 0, h = 0.0, type)`. If `index < 0`: return false.
2. Query the occupant manager (slot 40, `maxCount` −1) with **no filter** over the rectangle spanned by the first
   cell of step `start` and the last cell of step `end`, grown by one cell on every side. If the query succeeds and
   finds anything: clear the candidate list `+0x1F4`, release the list and return false. So any occupant at all
   blocks a land bridge, unlike the water pass's filtered test *[compat]*.
3. Mark steps `start … end` kind 1. Append a record: network type `+0x35C`, start, end, index, `+0x10 := elev`,
   `+0x14 := elev`, `+0x18 := 1`.
4. As in §4.4 step 4.4: the length map, then the specialised or the parametric model. `SelectBridgeElevation` is
   **not** run: the deck is at the near bank's height `D`, without the type's `+0x5C` or `+0x78`.

The return value is not used by the scan.

---

## 8. Elevations

### 8.1 The two values

A record has a **reference** elevation (`+0x14`), the bank height chosen by `AdjustBridgeSpan` (§5.2), and a
**deck** elevation (`+0x10`), the height the models build at. Both are returned to the UI by slots 37 and 38.

### 8.2 SelectBridgeElevation(&record) (`0x0062EA90`, pop 4)

1. `+0x10 :=` the override `+0x204` (−1.0 when none).
2. For each step `k` from `start` to `end − 1` (the end step is excluded): compute, for each cell `c` of the step,
   `surface(c) + +0x5C` if the cell has water (terrain slot 34), else `surface(c) + +0x78` (surface = terrain slot 18;
   `+0x5C`, `+0x78` of the bridge type). Only the **last** cell's value is kept (the earlier ones are discarded,
   `0x0062EAF0`) *[quirk]*. If that value `> +0x10`: `+0x10 :=` it.

So the deck is the highest of the override and, per step, the last cell's surface plus the water or land height. For a
one-cell-wide network the quirk changes nothing. For two-wide networks the first cell of each step is ignored.

---

## 9. The specialised model: ConstructSpecializedBridge(&record, &entry) (`0x00632060`, pop 8)

### 9.1 Orientation

Both models orient pieces the same way. From the pen direction (`+0x7C`):

- `R := (pen / 2 + 3) & 3`: pen 0 (−x) → 3, pen 2 (−z) → 0, pen 4 (+x) → 1, pen 6 (+z) → 2;
- the cells of a step are taken first to last when `R` is 0 or 1, else last to first; the first cell taken is
  **lane** 0;
- the **corner base** `t := [1, 0, 3, 2][R]` (the tables at `0x00AA8960` and `0x00AA8970` are equal).

A piece record's four corner heights are indexed 0–3 (piece `+0x14 + 4 × corner`, the terrain corner order of
terrain.md slot 17). Corners `t` and `(t + 1) & 3` are one edge across the drag, `(t − 1) & 3` and `(t − 2) & 3` the
other.

### 9.2 The piece record (`0x50` bytes, record vector `+0x1C`)

| Offset | Meaning |
|---|---|
| `+0x00` | kind: 0 ramp, 1 pier, 2 deck (the specialised model writes 0 for every piece) |
| `+0x04`, `+0x08` | the cell x, z |
| `+0x0C` | the piece (exemplar instance) |
| `+0x10` | u8 rotation, 0–3 |
| `+0x14`–`+0x20` | the four corner heights |
| `+0x24` | the model height Y |
| `+0x28` | the model depth below the origin (−1.0 = never a column, §11.3) |
| `+0x2C`–`+0x38` | four suspension weights (1.0 when supported, §11.4) |
| `+0x3C` | bridge type `+0xA4` when supported |
| `+0x40` | byte: support flags (§9.3) |
| `+0x44`–`+0x4C` | vector of cable pieces (§11.4) |

### 9.3 The walk

The entry holds the piece list and the step heights `h[]` (§3.2). For each step `s` from `start` to `end`, with
`k := s − start`, and each cell of the step in lane order, take the **next** piece `P` of the list (one counter over
the whole bridge; the list's length is not checked against the cell count) and append a record:

- kind 0, the cell, `P`'s instance and depth;
- rotation `(P.rotation + R) & 3`;
- `Y := r24(r24(elev + P.Y) + +0xD4)`, `elev` = record `+0x10`;
- corners `t` and `t + 1`: `r24(h[k] + elev)`; corners `t − 1` and `t − 2`: `r24(h[k + 1] + elev)`.

**Support.** A piece is supported when its support kind is not 0 or a suspended section is open (below). A
supported piece gets the weights 1.0, `+0x3C := +0xA4`, and the flags:

- 2 if `R` is 1 or 3, else 1 (the drag axis);
- `| 4` if its support kind is 1 (`pylon`);
- only when W > 1: `| 8` or `| 0x10` by lane. Lane 0 gets 8 when `R` is 0 or 3, else 0x10; the other lane gets the
  other value.

**Sections.** After each step, let `m` be the last non-zero support kind among its pieces (0 if none). If `m ≠ 0`:

- if a section is open (it started at step `k0` with kind `m0`): the suspension profile (§11.4) runs over the records
  of steps `k0 … k`: pylon to pylon with `+0x9C` (mode 1), pylon to end with `+0xA0` (mode 2), end to pylon with
  `+0xA0` (mode 0); nothing runs from an end to an end. Then, if `m` is an end, the section closes; otherwise a new one
  opens at `k` with kind `m`;
- if no section is open: one opens at `k` with kind `m`.

---

## 10. The parametric model: ConstructParametricBridge(&record) (`0x00631300`, pop 4)

### 10.1 The layout

A parametric bridge has **ramps** at both ends (N steps each), **piers** in groups of M steps, and **deck** pieces
between them. A pier group is a centre step (pier piece 0) with the pier pieces 1 … M − 1 on the following steps and,
mirrored, on the preceding steps.

### 10.2 Pier positions

With `u34 := +0x34` (spacing), `u38 := +0x38` (first-pier offset), `x := +0x7C`:

1. `first := start + u38 + x`, `last := end − u38 − x`. `next := −1`, `placed := −1`, `spacing := 100000`.
2. If M = 0: no piers. Go to §10.3.
3. If `first > last`: if the minimum pier count `+0xA8` is not 0, `first := last := (start + end) / 2` (truncated),
   but `next` stays −1, so **no pier** is placed *[quirk]*. Go to §10.3.
4. `span := last − first`. Loop (at most two passes):
   1. `count := span / u34` (unsigned). `count` is the number of gaps between piers.
   2. If `count > +0xAC` and `+0xAC ≠ 1`: `count := +0xAC − 1` (0 if `+0xAC` is 0).
   3. If `count < +0xA8` and `+0xA8 ≠ 1`: `count := +0xA8 − 1`.
   4. `count := min(count, span / 2)`.
   5. If `count < 1`: `spacing := 100000`, stop.
   6. `spacing := span / count`. On the second pass, stop.
   7. First pass only: `f := r24(r24((end − start + 1) / r24(2 × +0x3C + count)) × +0x3C)` and
      `kk := floor(f) − (x + u38)` (the floor is done with the 12582912 rounding trick, `0x00631448`–`0x0063147A`;
      it is exact for these magnitudes). If `kk ≥ 1`: `first += kk`, `last −= kk`, `span −= 2 × kk`, next pass.
      Otherwise stop.
5. `next := first`.

`+0x3C` (0.6 on the suspension types) moves the outer piers to about `0.6 / (1.2 + count)` of the length from each
end: the towers of a suspension bridge.

### 10.3 The walk

For each step `s` from `start` to `end`, each lane `l` (cells in lane order, §9.1), in this order:

1. `s < start + N`: **ramp** piece `i := (s − start) × W + l`; depth `+0x20[i / W]`.
2. `s > end − N`: mirrored ramp `i := (end − s + 1) × W − l − 1`; rotation `+ 2`; depth `+0x20[i / W]`.
3. `placed ≥ 0` and `s − placed < M`: **pier** piece `(s − placed) × W + l`; depth `+0x50[s − placed]`.
4. `next < 1`: **deck**.
5. `d := next − s`:
   - `d ≤ 0`: pier piece `l` (the centre); depth `+0x50[0]`; `placed := next`; `next := next + spacing`, and
     `next := −1` if it is then past `last`;
   - `0 < d < M`: mirrored pier piece `d × W + l`, rotation `+ 2`, depth `+0x50[d]`;
   - `d ≥ M`: **deck**.

A deck piece is `+0x2C` with rotation `(+0x30 + 2l) & 3`, depth −1. Finally every rotation becomes
`(rotation + R) & 3`.

Heights (`elev` = record `+0x10`):

- deck: `Y := r24(elev + +0xD4)`; pier: `Y := r24(r24(elev + +0xD4) + +0xC4)`; all four corners `elev`;
- ramp `i`, `u := i / W`: `Y := r24(r24(elev − +0x6C[u]) + +0xD4)`. With `t` from §9.1 (for a mirrored ramp, `t` is
  taken with `(R − 2) & 3` instead of `R`): corners `t` and `t + 1` get `r24(elev − +0x60[u])`, corners `t − 1` and
  `t − 2` get `r24(elev − +0x60[u + 1])`.

### 10.4 Support flags and cables

A piece is supported when a pier has been placed or is pending (`placed ≠ −1` or `next ≠ −1`) **and** both cable
models (`+0xB0`, `+0xB4`) are set. Then it gets the weights 1.0, `+0x3C := +0xA4`, and the flags: 2 if `R` is 1 or 3,
else 1; `| 4` on the step `s = placed`; with W > 1, `| 8` or `| 0x10` by lane (lane 0 gets 8 when `R` is 0 or 3, else
0x10). Otherwise the flags are 0. Then, for cable bridges, the suspension profile (§11.4) runs between the end and
the first pier, between piers and from the last pier to the end, and the cables are plotted (`0x00630F70`).

---

## 11. Pieces and pillars

### 11.1 InsertBridgePieces(arg) (`0x0062F610`, pop 4)

For each bridge record in order, stopping at the first one whose `end ≥ +0x78`: for each piece record, look up the
cell's info (key `(z << 16) + x`; it must exist), set its byte `+0x51 := 1` (a bridge cell), and call
`InsertBridgePiece(piece, cellInfo, bridgeType, arg)`.

### 11.2 InsertBridgePiece (`0x0062EEB0`)

1. Create the occupant with `CreateNetworkOccupant` (slot 54, network-geometry.md §10.1), class `0x49CC1BCD` (the
   bridge occupant).
2. Set its exemplar with the piece and rotation (`0x00624860`), with the model offset `dy := Y − ` the occupant's
   height, then occupant slot 30.
3. Flags: clear `0x2000000` and `0x4000000`, set `0x8000000`. If the occupant is usable (`0x10000000`), clear
   `0x4000`, else set it (network-manager.md §5).
4. If the piece's exemplar lacks property `0xABE1B5A3`: occupant slot 68 `:=` the lowest corner of the cell
   (`0x0062DB90`) `− +0xD0`.

### 11.3 Pillars

A piece's depth is how far its own model reaches below the occupant's origin. A support column is added only to fill
the gap between the bottom of the model and the ground (`0x0062F012`–`0x0062F378`):

1. Nothing when the depth is < 0.
2. `top :=` the occupant's position height (occupant query `0x0062F02C`–`0x0062F03B`) `− depth` *[inferred: the
   position height is the piece's model height Y]*.
3. `bottom :=` the lowest of the terrain altitudes (slot 12) at the cell info's four vertex ids.
4. If `top < bottom`, the model already reaches the ground: no column.
5. The column's four corners are the footprint `+0x8C` for a pier piece (kind 1), else `+0x80`, in the variant for the
   piece's rotation, around the occupant's position, from `bottom` up to `top`.
6. If the mean of the four bottom heights (× 0.25, `0x00A8821C`) is below the water level (terrain slot 30), the
   column is split at `water − 0.01` into an underwater part and the part above, each added with
   `AddSupportColumnToOccupant` (`0x00643B70`); otherwise one column is added.

So the road level bridge's pier centre (depth 498) and first ramp (depth 256) get a column only where the ground is
more than 498 m or 256 m below them; the pieces with depth −1 never get one.

### 11.4 Suspension and cables

After the columns, `InsertBridgePiece` calls `AddSuspendedBridgeStructures` (`0x0062DF30`) when the piece's support
flags (`+0x40`) are not 0, then adds the piece's cable pieces (`+0x44`, filled by the cable plotting `0x00630F70`). The suspension profile (`0x0062E690`) turns the piece positions between two supports into the
per-corner weights `+0x2C`–`+0x38`, with the parameter given (`+0x9C` or `+0xA0`) and an end mode (0, 1 or 2). These
are drawing data: nothing in the simulation reads them *[inferred]*. Their exact formulas are left open (NB2).

---

## 12. Cost

`GetCostOfSolution` (slot 20, network-tools.md §8.1) adds, per bridge record, in integers (`0x00625740`–`0x0062577F`):

`(end − start) × (bridgeType+0x154 + tool+0x320) + bridgeType+0x150`

The bridge type is found from the record's own network type (`+0x00`) and index (`+0x0C`). `end − start` is one less
than the step count. Land and water bridges cost the same way. Every shipped bridge type has `+0x150` = `+0x154` = 0
(§2.3), so a bridge costs `(end − start) × +0x320`: 100 per step for road, rail and their kin, 1000 for highway and
avenue (network-geometry.md §2.4). network-tools.md §8.1 calls these "bridge record `+0x154`, `+0x150`"; they are
**bridge type** fields.

## 13. Failure codes

| Code | Set by | When |
|---|---|---|
| `0xF0000003` | `PlaceNetwork` after §4 | `IdentifyBridgeCells` lowered `v`: a wet step had no acceptable span (far bank not found before `v − 1` or the straight count, an occupied or crooked step on the way) or no bridge type fit |
| `0xF0000001` | `CheckForGoodTerrain` (network-geometry.md §4.5) | a wet step that the water pass skipped (a span shorter than 5 steps or than the type's minimum, a wet step blocked by a neighbouring bridge, or a run that is not straight) |

The land pass sets no code. A land bridge that cannot be built (no bridge type, an occupant in the way) just leaves
its steps regular, and the height solver then treats them as ordinary terrain.

---

## 14. Answers given here

- **NG2** (network-geometry.md §17): this spec. MATCHES the outline of network-geometry.md §7.1, with these
  corrections: the water pass's straight test reads `v`, not the dragged count (§4.1); the step test is an occupant
  query through the OR filter (§4.2); the minimum-span test is `end − start ≥ 4` and `n ≥` the minimum (§4.4); the
  parametric model reads `+0x34`–`+0x7C` and `+0xA8`–`+0xD4`, not `+0xF8`–`+0x120` (those are suspension tower data).
- **NG4**: §7 and §2.5. `+0x354` is set by an address test (`0x00626B53`) of bridge type `+0x98` (property
  `0x482CB44F`, land); every shipped type sets the property, so there is no shipped effect. The pass itself: §7.

## 15. Quirks (summary)

1. A malformed INI entry leaves a default bridge type in the vector (§2.2).
2. The ramp rotation count is truncated to a byte (§2.3).
3. `Init` tests the address of `+0x98`, not its value (§2.5).
4. `AdjustBridgeSpan` multiplies the slope angle in radians by 14 and 4 (§5.2).
5. `SelectBridgeElevation` keeps only the last cell of each step (§8.2).
6. The land scan reads the +x or +z edge whatever the drag direction (§7.2).
7. With the piers squeezed out (`first > last`) and a minimum pier count, the centre is computed but no pier is
   placed (§10.2).
8. `InsertBridgePieces` stops at the first record past `v` instead of skipping it (§11.1).
9. The specialised walk does not check the piece list's length against the cells (§9.3); the shipped files were not
   checked for a mismatch.

## 16. Open questions

- **NB1.** The rule that the network solver applies to bridge cells (`RemoveBridgePiecesFromSetOfCellsToBeSolved`,
  `0x00638B40`, and `CopyBridgeElevationConstraintsIntoCellInfo`): how the corner heights of §9 and §10 constrain the
  height solver (network-geometry.md §9). Not read.
- **NB2.** The suspension profile (`0x0062E690`), the cable plotting (`0x00630F70`) and `AddSuspendedBridgeStructures`
  in detail. Drawing only *[inferred]*.
- **NB3.** The occupant position height used for the column top (§11.3): inferred to be the piece's model height Y
  set in §11.2, not checked.

## 17. Conformance tests

A "city" is a fresh 64 × 64 city. Road = type 0. Cells are (x, z). "River" means wet cells (terrain slot 34 true) whose
surface level (slot 18) is the water level. Unless stated, the banks are flat and dry at the water level's height
(every `d ≤ c4` in §5.2), nothing is built, no preferred bridge and no override.

1. **Loading** (§2.1, §2.6). Road has 9 bridge types with keys 1–9 in INI order; key 4 is exemplar `0x5`. Street,
   pipe, subway, one-way road and dirt road have 0; power line has none. Every shipped type: max span 100000,
   clearance [0, 10000], cost (0, 0).
2. **Default minimum span** (§2.3). An exemplar with N = 3 ramp steps and no `0x482CB458`: minimum span 8.
3. **Malformed entry** (§2.2). Adding `10 = 0x6534284a, 0xa82ca30f` (2 tokens) to the road section gives a tenth
   road bridge type: key 10 and every field at its default.
4. **Ramp heights** (§2.4). Road key 2: `+0x78` ≈ 36.652 (the sum of the seven `b − a`), `+0x60[7]` = 0, `+0x7C` = 4.
5. **Specialisation grammar** (§3.1). A file `bridge 99 length 10 end end` where no bridge type has key 99: the parse
   fails and nothing after it is read. `bridge 1 length 9 … end` for road key 1 (minimum span 10): the whole parse
   fails.
6. **Land flag** (§2.5). Road: `+0x354` = 1. Street (no bridge types): 0. A test bridge type with `0x482CB44F` false
   as the only road type: still 1 (the address test).
7. **Edge mask** (§4.3). Pen 4, a one-cell step whose cell has a road piece with edge word `0x00020002`: passes. The
   same with `0x00000200` (a z-direction road): fails. A cell with a rail piece (type mask bit 1) on a road drag:
   fails.
8. **Water pass, simple** (§4.4, §5.1). Road, straight drag (0, 10) → (30, 10), pen 4, a river at x = 10 … 19:
   `i = 10` is the first wet step; start 9, end 20 (first dry step after the river); flat banks, so the reference
   elevation is the bank's surface and the span is not widened; `n = 12`. The winner is key 8 (cantilever truss,
   index 7, `d = 0`). One record, steps 9 … 20 kind 1, `+0x18` = 0, `v` unchanged. Road key 8 has specialisations
   for lengths 12 … 255, so the specialised model builds it.
9. **Tie break** (§6.1). As test 8 but `n = 11` (river x = 10 … 18): keys 1 and 7 both have `d = 1`; key 1 (index 0)
   wins, and the parametric model builds it (key 1 has no specialisation).
10. **Preferred bridge** (§6.1). As test 8 with `SetPreferredBridge(4)` (key 5, minimum 22): padding needs 10 more
    steps; with room on both sides it pads alternately back and forward to start 4, end 25 (22 steps). With the drag
    (7, 10) → (30, 10) instead (start 2, end 13, `v` = 24), the back side is blocked at once (`start` must stay > 1)
    and the forward side takes all ten: start 2, end 23.
11. **Short river** (§4.4). Road, a river at x = 10 … 11 (2 wet steps): start 9, end 12, `end − start = 3 < 4`: no
    record, `v` unchanged; then `CheckForGoodTerrain` refuses the run with `0xF0000001`.
12. **No far bank** (§5.1). Road, (0, 10) → (15, 10) with a river from x = 10 to past the end: the end reaches
    `+0x78 − 1`; `v := 10`; code `0xF0000003`.
13. **Crooked drag** (§4.1). The drag of network-geometry.md test 11 (not straight): the water pass does nothing;
    `0xF0000001`.
14. **Deck elevation** (§5.2, §8.2). As test 9 (road key 1: `+0x5C` = 20, `+0x78` = 0), water level 250 m, flat banks
    at 250 m: reference elevation `+0x14` = 250, deck `+0x10` = 270. With `SetBridgeAltitudeOverride(300)`: the
    reference is 300; the flat banks never reach it, so the span widens to start 1 and end `v − 2`; the deck is 300.
15. **Wide step quirk** (§8.2). Avenue, a two-cell step whose first cell is wet with surface 250 and last cell dry at
    terrain 255, `+0x5C` = 30, `+0x78` = 0: the step gives 255, not 280.
16. **Steep banks** (§5.2). Road (c14 ≈ 8.5521, c4 ≈ 2.4435), water level 250. The step `start` (the last dry step)
    slopes from 250 at the river to 262; the land behind it is flat at 262. At `k = start`: `d` = 262 − 250 = 12 > c14,
    so A = B = 262; further back `d` = 0. The far bank is the mirror image: A2 = B2 = 262. `elev` = 262, no widening,
    clearance 12. With a rise of 5 m instead (`d` = 5, between c4 and c14): B = B2 = 255, A = A2 = 0, `elev` =
    min(255, 255, 0) = 0, so the floor rule gives the surface of step `start`'s first cell, 255; clearance 5.
17. **Cost** (§12). Road, the bridge of test 8 (start 9, end 20) on flat land: the bridge term is 11 × 100 = 1100;
    highway with the same steps: 11 × 1000 = 11000.
18. **Parametric layout** (§10). A record for road key 1 (N = 3, M = 3, W = 1, `+0x34` = `+0x38` = 6, `+0x7C` = 0, `+0x3C` = 0,
    pier depths [498, −1, −1], ramp depths [256, −1, −1]), start 5, end 24, pen 4 (R = 1): first 11, last 18,
    count 1, spacing 7, kk = −6. Pieces by step: 5, 6, 7 ramp 0, 1, 2; 8 deck; 9, 10 mirrored pier 2, 1; 11 pier 0;
    12, 13 pier 1, 2; 14, 15 deck; 16, 17 mirrored pier 2, 1; 18 pier 0; 19, 20 pier 1, 2; 21 deck; 22, 23, 24
    mirrored ramp 2, 1, 0. Depth ≥ 0 only at steps 5 and 24 (256) and 11 and 18 (498); every other piece has −1. No support flags
    (no cable models).
19. **Pier fraction** (§10.2). Road key 5 (medium suspension: `+0x34` = 16, `+0x38` = 6, `+0x7C` = 0, `+0x3C` = 0.6,
    `+0xA8` = 2, `+0xAC` = 47, M = 1), start 10, end 49 (40 steps): first 16, last 43, span 27; count 27 / 16 = 1
    (the minimum rule gives 2 − 1 = 1); spacing 27. First pass: f = r24(r24(40 / 2.2) × 0.6) ≈ 10.909091, kk =
    10 − 6 = 4: first 20, last 39, span 19. Second pass: count 1, spacing 19. Piers (the towers) at steps 20 and 39;
    every other step between the ramps (steps 10 and 49) is deck.
20. **Squeezed piers** (§10.2). A bridge type with M = 1, `+0x38` = 6, `+0x7C` = 0, `+0xA8` = 2, start 0, end 9:
    first 6 > last 3, so both become 4, and no pier piece is placed (quirk 7).
21. **Orientation** (§9.1). Pen 0 → R 3, cells last to first, t = 2; pen 6 → R 2, t = 3; a deck piece with `+0x30` =
    1 on pen 4 has rotation 2.
22. **Land bridge** (§7). Road, straight drag (0, 10) → (20, 10), pen 4, no occupants. The two +x edge altitudes of
    each step's cell are equal: 100 for steps 0 … 4, then 60, 40, 30, 30, 30 for steps 5 … 9, then 100 for steps
    10 … 20. Step 5 has `hi` 60 < A = 100: spanStart 4, D = 100, E = 60. Steps 6 … 9 have `hi < E`, so they only lower
    C to 30. Step 10 has `hi` 100 ≥ D: length 10 − 4 + 1 = 7 ≥ 4, D − C = 70 > 55, |D − hi| = 0 ≤ 20:
    `CreateLandBridge(4, 10, 100, 0)`. `FindBestBridgeType` (land list, n = 7, h = 0) picks key 3 (index 2, `d` = 3,
    first of the four types with minimum 4). With steps 6 … 9 at 50 instead (D − C = 50 ≤ 55): no bridge. With any
    occupant inside the rectangle (cells (3, 9) to (11, 11)): no bridge, `+0x1F4` cleared.
23. **Flat valley floor** (§7.2). As test 22, but steps 5 … 9 all at 30. Step 6 has `hi` = E, so it takes the
    second branch: spanStart 5, D := 30, length 2: the span ends. Steps 7 and 9 start new spans (A and B were not
    updated inside the span, so A is still 100); steps 8 and 10 end them with length 2 and 3. No land bridge.
24. **Land record** (§7.3). The bridge of test 22: `+0x18` = 1, `+0x10` = `+0x14` = 100 (no `+0x5C` or `+0x78`
    added); `IsMajorBridgeLandBridge` is true and `GetMajorBridgeAltitude` gives 100.
25. **Getters** (§1.1). With no bridge: `GetMajorBridge` = −1, `GetMajorBridgeAltitude` false.
    `GetBridgeBitmapKey(0, …)` for road: true, (`0x6A386D26`, 1).
