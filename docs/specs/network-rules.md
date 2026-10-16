# Network rules (behaviour spec)

Status: M5 behaviour spec, written 2026-10-16 (re-specs17b lane). Every fact was checked on the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`); every address is a Windows address. The Mac build's symbol
names were used to name the Windows functions. Numeric details (bit layouts, table contents, operation order) were
checked against the instruction bytes where the pseudocode was unclear (`0x00622B50`, `0x006340D0`, `0x0063B340`,
`0x00640320`, `0x006094B0`, `0x00633A60`, `0x006335E0`). Implementers work from this spec, the public LGPL gzcom-dll SDK
headers and general knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags:

- *[compat]*: observable; a replacement must match it.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: consistent with all the evidence, but not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.

Scope. The **piece choice** half of the network tool's solve (network-geometry.md §8 names it in outline):

- the rule resources: their instances, their file grammars, and when they are loaded (§2);
- the **tile rule files** of the per-type tile evaluators, the `#Transmogrify` expansion into rotated and mirrored
  variants (§3, §4), and how a rule matches a cell (§5);
- `SolveNetwork`'s rule stages: the padded sets, `SolveUsingEvaluator` and `ApplyNetworkRules` (§6);
- the **edge subtypes** (`AssignEdgeSubtypes`), which the rule files key on (§7);
- **multi-type intersections** (a cell with two network types) and the override matrix (§8);
- the **tile subsets**: the `RuleOverrides` pair patches and the `TileVerifiers` pattern checks (§9);
- the **highway intersection rules** (`cIntRule`): their file grammar, `Test`, `DoAutoComplete`,
  `IdentifyHighwayIntersections` and `InsertIsolatedHighwayIntersection` (§10);
- how a solved cell's piece id becomes an occupant exemplar (§11).

It answers network-geometry.md NG5 (§12).

What is not here: the drag, the validation passes, the height solver and the commit (network-geometry.md,
network-tools.md); bridge and tunnel pieces (the evaluator instances `0x1000` and up, network-geometry.md §7.1); one-way
conflict resolution (`ResolveOneWayConflicts`); the preview's multi-type separation (`SolveMultiTypeSeparation`,
`0x00633FA0`, which only draws). The shipped rule files are game data. This spec describes their **format** and uses
synthetic rules in its examples; it never reproduces the shipped rules.

Related specs (cited, not repeated):

- [network-geometry.md](network-geometry.md): the steps and cells (§3), the cell infos (§8.1), the solve outline (§8),
  the hash-set iteration order (§9.1).
- [network-tools.md](network-tools.md): `PlaceNetwork` (§5), the commit and `InsertNetworkOccupant` (§6.2),
  `PlaceNetworkOccupantById` (§4), the failure codes (§5.3).
- [network-manager.md](network-manager.md): network types (§3.1), network flags (§5), connection nodes (§7.7, §7.8).
- [occupant-manager.md](occupant-manager.md): the cell queries used by `cIntRule::Test`.

---

## 1. Vocabulary

### 1.1 Cells, sides, offsets

A **cell key** is `(z << 16) + x` (unsigned 32-bit). The four **sides** of a cell are numbered:

| Side | Name | Neighbour offset (dx, dz) |
|---|---|---|
| 0 | W | (−1, 0) |
| 1 | N | (0, −1) |
| 2 | E | (+1, 0) |
| 3 | S | (0, +1) |

The offset tables are `0x00AA89F8` (dx) and `0x00AA8A08` (dz). The opposite side of `s` is `(s + 2) & 3`.

### 1.2 Network types

Types 0–12: 0 Road, 1 Rail, 2 Highway, 3 Street, 4 Pipe, 5 PowerLine, 6 Avenue, 7 Subway, 8 LightRail, 9 Monorail,
10 OneWayRoad, 11 DirtRoad, 12 GroundHighway. The **type bit** of type `t` is `1 << t` (tables `0x00AA835C`,
`0x00AA89C4`, `0x00AA74F8`, `0x00AA8C10`, all equal). `0x1FFF` is the mask of all type bits. A cell info's flags
(`+0x08`, network-geometry.md §8.1) hold the type bits and other flags (`0x40000` bridge step, `0x80000` portal).

**Network names** (used by the rule files): the 13 names above, spelled `Road`, `Rail`, `Highway`, `Street`, `Pipe`,
`PowerLine`, `Avenue`, `Subway`, `LightRail`, `Monorail`, `OneWayRoad`, `DirtRoad`, `GroundHighway` (name table
`0x00AA845C`). `NetworkNameToID` (`0x0063C7A0`) compares case-insensitively after trimming. INI section names are
looked up lower-cased, so they are case-insensitive too.

**Elevated types**: Highway, LightRail, Monorail (bit mask `0x304`). **Street-level road types**: Road, Street,
Avenue, DirtRoad (bit mask `0x849`).

### 1.3 Edge words

Each cell info holds one **edge word** per type (`+0x18 + 4t`) and their union (`+0x4C`). Byte `i` of an edge word
(bits `8i`–`8i+7`) describes side `i`. So a word written in hex is `0xSSEENNWW`. In each byte:

| Bits | Meaning |
|---|---|
| 0–2 | the **kind** (below) |
| 3–7 | the **subtype** (or "minor type"), set by §7 or by rule files |

| Kind | Meaning |
|---|---|
| 0 | no connection |
| 2 | **orthogonal**: the network leaves through the middle of the side |
| 1 | **diagonal**, leaning to the corner that this side shares with side `(s + 1) & 3` |
| 3 | **diagonal**, leaning to the corner that this side shares with side `(s + 3) & 3` |
| 4 | the **inner side of a wide network** (avenue, highway): set by `AddGroupingInfo` (`0x006383A0`) on the sides between the two cells of a width-2 step |

So a diagonal through the north-west corner is W kind 1 plus N kind 3 (`0x00000301`), and a north–south straight is
`0x02000200`. A side "has a connection" when `byte & 3 ≠ 0`.

Helpers used below (all bytewise):

- `connmask(e)` = for each byte with bit 0 or bit 1 set, `0x03` in that byte, else `0x00`.
- `covers(eW, eL)` is true when `((eW ^ eL) & connmask(eL)) = 0`: on every side where `eL` has a connection, `eW` has
  the same low two bits.
- `D(e)` is an 8-bit mask: for each side `i` whose byte has `byte & 3 ≠ 0`, bits `2i` and `2i + 1` are set. (It is
  computed as byte 3 of `(((e & 0x03030303) + 0x03030303) & 0x04040404) × 0x00C30C30`.)
- `within(a, b)` (`0x00729EC0`) is true when, on every side where byte `a` is non-zero, the low nibble of `b` equals
  the low nibble of `a`. It is computed as `((((a + 0x0F0F0F0F) >> 4) & 0x01010101) × 0xF & (a ^ b)) = 0`, which is the
  rule above for bytes below `0x10` *[compat]*.

### 1.4 Rotation and flip of a piece

A piece's orientation is one **orientation byte**: bits 0–1 the rotation `r` (quarter turns), bit 7 the **flip**.
Rule files write it as two numbers, `rot` and `flip`; the byte is `rot | (flip ≠ 0 ? 0x80 : 0)`, with `rot` not
masked.

### 1.5 Metapositions

Rule conditions and outputs name a neighbour cell by a **metaposition** `m` (0–24), an offset (dx, dz) in the 5 × 5
square around the centre (tables `0x00B0B3D8` dx and `0x00B0B3F4` dz; the inverse grid `0x00B0B3BC`):

| m | (dx, dz) | m | (dx, dz) | m | (dx, dz) | m | (dx, dz) | m | (dx, dz) |
|---|---|---|---|---|---|---|---|---|---|
| 0 | (0, 0) | 5 | (1, 0) | 10 | (−2, −1) | 15 | (2, −2) | 20 | (1, 2) |
| 1 | (−1, 0) | 6 | (1, 1) | 11 | (−2, −2) | 16 | (2, −1) | 21 | (0, 2) |
| 2 | (−1, −1) | 7 | (0, 1) | 12 | (−1, −2) | 17 | (2, 0) | 22 | (−1, 2) |
| 3 | (0, −1) | 8 | (−1, 1) | 13 | (0, −2) | 18 | (2, 1) | 23 | (−2, 2) |
| 4 | (1, −1) | 9 | (−2, 0) | 14 | (1, −2) | 19 | (2, 2) | 24 | (−2, 1) |

Positions 1–8 are the inner ring and 9–24 the outer ring, each clockwise from the west.

---

## 2. Resources and loading

### 2.1 The rule resources

All rule files are INI or text resources of type `0x0A5BCF4B`, group `0xAA5BCF57`. `ReloadRules` (`0x006253D0`) loads
them once per process, from the first network tool `Init` (`0x00626A0D`), in this order:

1. The network INI (`0x8A5971C5` / `0x8A5993B9`) → `ReloadNetworkTypeInfo` (`0x00623310`). For each type: clear the
   record, set its ids (§2.2), load its two **tile evaluators** (§2.3), its bridge types, supports and placement
   parameters (network-geometry.md §2), then `LoadComplete`. After all types: the **tile verifiers** from the
   section `[TileVerifiers]` of the same INI (§9.2).
2. Instance `0x10000001` → the **multi-type intersection solutions** and the `[AllowedOverrides]` matrix (§8.1).
3. Instance `0x10000002` → the **tile conflict rules**, section `[RuleOverrides]` (§9.1).
4. Instance `0x10000000` → the **highway intersection rules** (§10.1), into the global rule map `0x00B452B0`.

A file that is missing leaves its tables empty; every consumer treats an empty table as "nothing applies".

### 2.2 The network type records

Type `t`'s record is at `0x00B452C8 + t × 0x114` (network-geometry.md §2.3). The fields this spec uses:

| Offset | Meaning |
|---|---|
| `+0x00` | the **edge-subtype scheme** (§7): 1 for Road, Street, Pipe, Subway, OneWayRoad, DirtRoad; 2 for Rail, LightRail, Monorail; 0 for the others |
| `+0x0C` | the **piece exemplar group** (§11) |
| `+0x24` | the vector of tile evaluators, in load order, ended by a null entry or the vector's end |
| `+0xFC` | the **override matrix row**: bit `u` set when this type may override type `u` (§8.1) |

Piece exemplar groups (`+0x0C`, written by `ReloadNetworkTypeInfo`, e.g. Road at `0x0062333B`):

| Type | Group | Type | Group |
|---|---|---|---|
| 0 Road | `0x2821ED93` | 7 Subway | `0x8A15F3F2` |
| 1 Rail | `0xE8347989` | 8 LightRail | `0x2B79DFFB` |
| 2 Highway | `0xA8434037` | 9 Monorail | `0xEBE084C2` |
| 3 Street | `0xA92A02EA` | 10 OneWayRoad | `0xCBE084CB` |
| 4 Pipe | `0xA951171C` | 11 DirtRoad | `0x6BE08658` |
| 5 PowerLine | `0x088E1962` | 12 GroundHighway | `0xEBE084D1` |
| 6 Avenue | `0xCB730FAC` | | |

### 2.3 Tile evaluator instances

Each type loads two tile rule files (instances of type `0x0A5BCF4B`, group `0xAA5BCF57`), first then second
(key table `0x00B0B418`–`0x00B0B52C`). PowerLine loads none (its tool places pieces itself, network-tools.md §9).

| Type | Instances (load order) | Type | Instances (load order) |
|---|---|---|---|
| 0 Road | `0x8`, `0x7` | 7 Subway | `0xC`, `0xB` |
| 1 Rail | `0x6`, `0x5` | 8 LightRail | `0x10`, `0xF` |
| 2 Highway | `0x2`, `0x1` | 9 Monorail | `0x16`, `0x15` |
| 3 Street | `0xA`, `0x9` | 10 OneWayRoad | `0x12`, `0x11` |
| 4 Pipe | `0x4`, `0x3` | 11 DirtRoad | `0x14`, `0x13` |
| 6 Avenue | `0xE`, `0xD` | 12 GroundHighway | `0x18`, `0x17` |

An evaluator is created and pushed onto the type's vector even when its file is missing or rejected; it then has no
rules and never matches. In the shipped data, four files (Avenue's two, LightRail's `0xF`, Monorail's `0x15`) start
with `#Transmogrify` (§4); the others carry their variants written out.

---

## 3. Tile rule files

`cSC4TileEvaluator::AddRulesFromResource` (`0x00623020`) reads one file into the evaluator's **rule map**.

### 3.1 Tokens

The file is read line by line (tokenizer `0x006210B0`, `0x00622620`):

- a line whose first character is `;` is a comment and is skipped;
- tokens are separated by commas; the end of a line also ends a token;
- empty tokens are skipped;
- numbers are read with `atol`, which skips leading blanks and stops at the first non-digit.

### 3.2 Header

The first token must be exactly `#Transmogrify`, which turns the expansion of §4 on, or any other token that starts
with `#` (for example `#RoadRules#`). Otherwise the file is **rejected**: no rules are read.

### 3.3 Rules

Rules follow one another. Each rule (`cSC4TileRule::Read`, `0x00622870`) is:

1. **Optional transform token**: `noflip` (transform mask `0x0F`) or `notrans` (mask `0x01`). Without it the mask is
   `0xFF`.
2. **Centre line**: the token `1`, then four edge tokens for sides W, N, E, S. If the token is not `1`, `Read` fails
   and the whole file stops loading; the rules read before stay in the map *[inferred]*.
3. Any number of **condition lines** `2, m, e0, e1, e2, e3`.
4. Any number of **output lines** `3, m, id, rot, flip`.

Lines 3 and 4 may come in any order. The rule ends at the end of the file, or at the first token that does not start
with a digit or whose value is neither 2 nor 3; that token starts the next rule. At the end of the file a rule is
kept only if it has at least one output; a rule ended by the next rule's token is kept even with no outputs (§5.3).

**Edge tokens** (`0x00622800`): a decimal number `v`, read as `w = v mod 256`; the byte is
`((w / 10) << 3) | (w mod 10)`. The tens digit is the subtype and the units digit is the kind. So `2` is `0x02`, `13`
is `0x0B` (subtype 1, kind 3), `27` is `0x17`. Centre tokens have no wildcard.

**Condition edge tokens**: as above, or `?`, which makes that byte a wildcard (value byte 0, mask byte 0). A
condition is `{m, value, mask}`: `m` by `atol`, `value` the four bytes, `mask` `0xFF` in each byte that is not a
wildcard.

**Output lines** (`0x00622790`): `m` by `atol`; `id` by `RZConvertToUint32A` (hex with a `0x` prefix, else decimal);
`rot` by `atol` (kept as a byte, not masked); `flip` non-zero sets bit 7 of the orientation byte. An output is
`{id (u32), m (byte), orientation (byte)}`.

### 3.4 The rule map

The rule map is an ordered multimap keyed by the rule's **centre edge word** (the four centre bytes). An insert goes
after every existing rule with an equal key. So the rules for one key are in file order, and files loaded into one
evaluator never mix: each evaluator has its own map.

---

## 4. The `#Transmogrify` expansion

After a `#Transmogrify` file is read, `0x00622B50` replaces the evaluator's map with the expanded one:

1. Visit the source rules in map order (key ascending, then insertion order).
2. For each source rule and each `t` in 0–7 whose bit is set in the rule's transform mask: build the variant
   **mirror if `t & 4`** (§4.2), then **rotate by `t & 3`** (§4.1).
3. Drop the variant if it equals an **earlier variant of the same source rule** (`0x006214A0`): the same centre key
   and the same set of conditions, compared as `(m, value)` pairs in both directions. Masks and outputs are not
   compared, so a mirrored variant whose outputs differ is still dropped when its pattern is the same.
4. Each kept variant gets transform mask `0x01` and is inserted into the new map after the existing equal keys.

So for one key the new map holds variants in source-rule order, then by `t`.

### 4.1 Rotation by `r` quarter turns (`0x00621380`, `0x00621180`)

| Part | Rule |
|---|---|
| edge byte | the byte of side `i` moves to side `(i + r) & 3`; its value is unchanged |
| metaposition | 0 stays 0; 1–8 → `((m − 1 + 2r) mod 8) + 1`; 9–24 → `((m − 9 + 4r) mod 16) + 9` |
| output orientation, not flipped | rotation := `(rot + r) & 3` |
| output orientation, flipped | rotation := entry `rot × 4 + r` of the table `0x00AA8328`, which is `(rot − r) & 3` for `rot` 0–3; bit 7 stays |

A rotation by 1 turns the offset (dx, dz) into (−dz, dx): metaposition 1 (−1, 0) becomes 3 (0, −1). The table rows are
`[0,3,2,1]`, `[1,0,3,2]`, `[2,1,0,3]`, `[3,2,1,0]`. A flipped output whose written `rot` is 4 or more reads past the
table *[quirk]*; the shipped files only use 0–3.

### 4.2 Mirror (`0x00621DF0`; edge bytes `0x00621210`)

The mirror swaps west and east:

| Part | Rule |
|---|---|
| metaposition | (dx, dz) → (−dx, dz) |
| edge bytes | the W and E bytes swap; N and S stay. Then each byte is transformed by kind: kind 1 ↔ kind 3 (subtype kept); kind 2: bit 3 kept, bits 4 and 5 swapped, **bits 6–7 dropped** *[quirk]*; kinds 0 and 4–7 unchanged. Bit 2 of the kind is kept. |
| condition | the value and the mask both go through the edge-byte rule |
| output orientation | rotation unchanged; bit 7 toggled |

Example: the north-west diagonal `0x00000301` (W kind 1, N kind 3) mirrors to `0x00030100` (N kind 1, E kind 3), the
north-east diagonal.

---

## 5. Matching a cell (`Evaluate`, `0x006220D0`)

### 5.1 The neighbourhood

The solver builds a **neighbourhood** (`0x00627080`) for the centre cell (x, z), the solving tool and type `t`:

- the centre key = the centre cell info's edge word for `t`, or 0 when the cell has no info;
- `edges(m)` = the edge word for `t` of the cell info at the metaposition's offset, or 0 when there is no info
  (`0x00623EE0`; cached per metaposition);
- `valid(m)` (`IsValidSolutionPos`, `0x00623F40`) is false when the target cell has an info whose flags carry
  **another** type bit (`flags & (typebit ^ 0x1FFF) ≠ 0`); otherwise true (also when the target has no info). The
  solver always builds the neighbourhood with this check on (`0x00634150` passes 0 for the override flag that would
  turn it off).

### 5.2 First match wins

For the rules whose key equals the centre key, in map order:

1. Every **condition** must hold: `(mask & (value ^ edges(m))) = 0`.
2. Every **output** must have `valid(m)`.

The first rule that passes is the match; `Evaluate` returns its output list. If none passes, the cell does not match
this evaluator.

### 5.3 Notes

- A rule with no outputs (§3.3) still matches. It produces nothing, so it **blocks** the later rules of the same
  evaluator for that pattern *[compat]*.
- A rule's centre key compares the whole edge word, subtypes included, so a cell whose subtypes were set by §7 only
  matches rules written with those subtypes.
- Conditions on metaposition 0 test the centre itself.
