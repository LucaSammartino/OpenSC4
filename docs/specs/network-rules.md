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
- what a solved cell's piece id selects (§11).

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
| `+0x0C` | the exemplar group of the type's **prebuilt-model pieces** (§11) |
| `+0x24` | the vector of tile evaluators, in load order, ended by a null entry or the vector's end |
| `+0xFC` | the **override matrix row**: bit `u` set when this type may override type `u` (§8.1) |

Prebuilt-piece exemplar groups (`+0x0C`, written by `ReloadNetworkTypeInfo`, e.g. Road at `0x0062333B`):

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

---

## 6. The rule stages of `SolveNetwork`

### 6.1 Order (`SolveNetwork(type, resolve)`, `0x00635CB0`)

`S` is the tool's set of cells to be solved (`+0x84`, built by `BuildSetOfCellsToBeSolved`, network-geometry.md §8).
Sets are hash sets of cell keys; "for each key" means their iteration order (network-geometry.md §9.1).

1. `P := BuildPaddedNetworkSet(S, 2)` (§6.2).
2. `AssignEdgeSubtypes(P, R, reset = true)` (§7). It fills `R`, the cells whose subtypes could not be assigned, and
   takes them out of `P`.
3. Take the cells of `R` out of `S`.
4. `SolveMultiTypeIntersections(S, out, type)` (§8.2). Take every cell of `out` out of `S` and `P`.
5. For each type `u` = 0 … 12 (every type, not only the tool's), for each tile evaluator of `u` in order until a null
   entry: `SolveUsingEvaluator(P, S, evaluator, u)` (§6.3).
6. Put the cells of `R` back into `S`.
7. The result (network-geometry.md §8 step 3.5): when not `resolve` and `S` is not empty, the unsolved-cell rule;
   otherwise `AdjustTileSubsets(solved, resolve)` (§9.1), then `VerifyTileSubsets(solved)` (§9.2); either failing
   gives `+0x78 − 1`; when both pass, the unsolved-cell rule on what is left in `S` (`+0x78` when `S` is empty).

A cell is **solved** when it has a record in the tool's **solved list** `+0x98`: 12-byte records
`{piece id (u32), orientation byte, cell key (u32)}`. The cell info's `+0xB8` holds the record's index.

### 6.2 BuildPaddedNetworkSet(src, out, pad) (`0x00634480`)

For each key of `src`: every cell of the square of half-width `pad` around it (clamped to the city,
`GetPaddedRectAroundCell` *[inferred]*) that has a cell info and is **not fixed** (`+0x53 = 0`) goes into `out`. The
source cell itself goes in even when it is fixed.

### 6.3 SolveUsingEvaluator(P, S, evaluator, u) (`0x006358C0`)

`W` := a new empty hash set (100 initial buckets). `ApplyNetworkRules(P, S, W, evaluator, u)`. Then every key of `W`
goes back into `P`. So each evaluator gets one pass over the open cells; a cell claimed by an output (below) leaves `P`
for good, for every later evaluator and type.

### 6.4 ApplyNetworkRules(P, S, W, evaluator, ·, u) (`0x006340D0`, pop 24)

The fifth argument (the tool's `+0x48`) is not used. While `S` is not empty and `P` is not empty:

1. `c` := the first key of `P` in iteration order.
2. `c` **qualifies** when all of these hold:
   1. `c` has a cell info;
   2. its type bits are exactly `u`'s: `(flags & 0x1FFF) = 1 << u`;
   3. it is not a connection node (`+0x56 = 0`), **or** all of: the node occupant's `GetNetworkFlag() & 0x1FFF`
      equals the cell's **whole** flags word (`+0x08`), and either the occupant's slot 13 (`0x00619110`,
      network-manager.md §7.8) returns false, or, with the two edge words `(a, b)` it returns, the cell's edge word
      `e` for `u` has no subtype or kind bit above 2 in any byte (`e & 0xF8F8F8F8 = 0`), `within(a, e)` and
      `within(e, b)` (§1.3);
   4. `Evaluate` (§5) finds a rule.
3. If `c` does not qualify: move `c` from `P` to `W`.
4. Otherwise, for each output of the rule, in order: `target` := `c` moved by the output's metaposition.
   1. If `target = c`, note "self claimed".
   2. If `target` is in `P`, remove it from `P`; else if it is in `W`, remove it from `W`; else **skip this output**.
   3. If `target` is in `S`: remove it from `S`, append `{id, orientation, target}` to the solved list and set the
      target's `+0xB8` to the record's index.
5. If "self claimed" was not noted: move `c` from `P` to `W`.

Consequences *[compat]*:

- An output may claim a **padding** cell (in `P` but not in `S`): it is removed from `P` and gets no record.
- A cell already claimed by an earlier output (no longer in `P` or `W`) is never claimed twice; the output is skipped.
- Condition 2.3 compares the occupant's masked type bits with the cell's whole flags word, so a connection-node cell
  that carries a non-type flag (for example `0x40000`) never qualifies *[quirk]*.

---

## 7. Edge subtypes (`AssignEdgeSubtypes(P, R, reset)`, `0x006359B0`)

The rule files key on subtypes (the tens digit of an edge token). Before the rules run, the solver writes them:

1. `Q := BuildPaddedNetworkSet(P, 1)`.
2. If `reset`: for each cell of `Q` whose flags word has at most one bit set, and that is in `P`: clear the subtypes
   of its edge word for its type (`ResetEdgeMinorTypes`, `0x00621440`: keep bits 0–2 of each byte), then recompute
   the union `+0x4C`.
3. For each cell `c` of `Q` whose flags word has exactly one bit set (so a bridge-step cell, with `0x40000`, is
   skipped *[compat]*): `t` := its type; the scheme := record `+0x00` of `t` (§2.2). Scheme 1 runs §7.1, scheme 2
   runs §7.2, other schemes nothing. Each gets `inP` = whether `c` is in `P`. If the scheme returns false: remove `c`
   from `P` and add it to `R`.

A cell of `Q` that is not in `P` can still land in `R`; step 6 of §6.1 then adds it to `S`, where it counts as an
unsolved cell *[compat]*.

In both schemes, "set byte `i` of `c`" changes `c`'s edge word for `t` and recomputes `c`'s union.

### 7.1 Scheme 1: diagonal stubs (`0x00633A60`; Road, Street, Pipe, Subway, OneWayRoad, DirtRoad)

Let `e` be `c`'s edge word for `t`. Classify its sides by `byte & 7` (`0x00620DF0`): **orthogonal** sides (kind 2)
and **diagonal** sides (kind 1 or 3), each in side order. A cell is **purely diagonal** (`0x00623C90`) when its edge
word for `t` has no orthogonal side and one or two diagonal sides.

If `c` has no orthogonal side or no diagonal side, return true. Otherwise compute the **through marks** `M[0..3]`
(below), then for each diagonal side `s` with `M[s] = 0`:

1. `n` := the neighbour across `s`. No cell info: next side.
2. `pd` := `n` is purely diagonal.
3. If `inP`: if `c` is fixed and `pd`, return **false**. Set `c`'s byte `s` subtype to `pd` (0 or 1, bits 3–7
   replaced). If not `pd`, also set its kind to 2 (`(byte & 0xF8) | 2`): the diagonal stub becomes an orthogonal side.
4. If `n` is in `P`: the same on `n`'s byte `(s + 2) & 3`. Otherwise, if `pd` and `n` is fixed, return **false**.

Return true.

**Through marks** (`0x006335E0`). `M` starts all 0. For each side `s` = 0 … 3 with `M[s] = 0`:

1. `k := byte(s) & 3`. If `k = 1`, `s2 := (s + 1) & 3`; if `k = 3`, `s2 := (s + 3) & 3`; otherwise next side.
2. `k2 := byte(s2) & 3`. If `k + k2 ≠ 4`, next side. (The two sides form one diagonal through their shared corner.)
3. Walk twice, first with `(dir, kind) = (s, k)`, then with `(s2, k2)`; stop at the first failed walk. A walk: step
   to the neighbour across `dir`. No cell info: **fail**. Purely diagonal: **success**. Otherwise turn,
   `dir := (dir + 1) & 3` when `kind = 3`, else `(dir − 1) & 3`; `k' :=` that neighbour's `byte(dir) & 3`; if
   `k' + kind = 4`, continue from it with `kind := k'`, else **fail**.
4. If both walks succeed: `M[s] := M[s2] := 1`.

### 7.2 Scheme 2: switches and curves (`0x00633C10`; Rail, LightRail, Monorail)

For each side `i` = 0 … 3 (re-reading `c`'s edge word each time): `b := byte(i)`, `k := b & 3`; skip when `k = 0`.
Let `opp = (i + 2) & 3`, `left = (i + 3) & 3`, `right = (i + 1) & 3`; `kind(j) = byte(j) & 3`.

- **`k = 2`**: `f := (kind(opp) ≠ 0 ? 1 : 0) | (kind(left) = 1 ? 2 : 0) | (kind(right) = 3 ? 4 : 0)`. If `f` is 0 or
  1, skip the side; otherwise the subtype is `f` (2–7).
- **`k` = 1 or 3**: skip the side unless `kind(opp) ≠ 0`, and also skip it when `kind(opp) = k` and
  `k + kind(left) = 4`. Then `p := right` when `k = 1`, else `left`; the subtype is 2 when `kind(p) = 2` or
  `kind(p) + k = 4`, else 1.

The new byte is `(subtype << 3) | k` (bit 2 is dropped). Then:

1. If `inP`: if `c` is fixed, return **false**; set `c`'s byte `i`.
2. `n` := the neighbour across `i`. If `n` is in `P`: set `n`'s byte `opp` to the same value (when `n` has a cell
   info). Otherwise, if `n` has a cell info and is fixed, return **false**.

Return true.

---

## 8. Multi-type intersections

### 8.1 The solutions file (instance `0x10000001`)

**Solutions.** For each pair of types `a < b` (by index), `ReadAllMultiTypeIntersectionSolutions` (`0x00641D30`) reads
the section `[<NameA><NameB>IntersectionSolutions]`, for example `[RoadRailIntersectionSolutions]`, through
`0x0063E340` and the entry callback `0x0063D080`:

- the entry's key is ignored (keys may repeat);
- the value is split on commas (`RZParseDelimitedText`) and must give **exactly 5** fields, else the entry is ignored:
  `edgesA, edgesB, pieceId, rot, flip`, each read with `RZConvertToUint32A` (hex with `0x`, else decimal). The edge
  words are written whole, in hex (`0xSSEENNWW`, §1.3), not with the per-side tokens of §3;
- the pair's table maps `(edgesA, edgesB)` to `{pieceId, rot | (flip ≠ 0 ? 0x80 : 0)}`; a later entry with the same
  key overwrites the earlier one.

**Override matrix.** In the section `[AllowedOverrides]`, each entry's key is ignored and its value is
`NameA,NameB` (`SC4NetworkLoadOverrideMatrix`). Both names are converted with `NetworkNameToID` and the bits are set
**symmetrically**: row A gets bit B and row B gets bit A. The 13 rows are copied into the type records (`+0xFC`).

### 8.2 SolveMultiTypeIntersections(S, out, primary) (`0x00634820`)

`primary` is the tool's type. `prio` is the table `0x00AA8A18`:

| Type | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| prio | 3 | 5 | 6 | 2 | 1 | 0 | 4 | 1 | 7 | 7 | 3 | 0 | 5 |

`typeOf(bits)` (`0x00623E60`) is the lowest type 0–11 whose bit is in `bits`, and **12 when none is**, so the
GroundHighway bit and every non-type flag bit read as type 12 *[quirk]*. `beats(j, i)` is true when
`prio[i] < prio[j]`, or the priorities are equal, `i ≠ primary` and `j = primary`. `elevated(x)` is `x & 0x304 ≠ 0`
for a set of type bits. "Remove type `i`" means: clear its bit in the flags, clear its edge word, recompute the union,
and when `i` = 10 clear the one-way byte `+0x50`.

For each key `c` of `S`, with `f` its flags word, `lo` the lowest set bit of `f` and `rest = f − lo`:

1. `rest = 0`: next cell.
2. **Three or more bits** (`rest` has two or more bits): `A := typeOf(lo)`, `B :=` typeOf(the lowest bit of `rest`),
   `C :=` typeOf(`rest` without its lowest bit). For `i` in (A, B, C), then `j` in (A, B, C): if `beats(j, i)`,
   `covers(e[j], e[i])` and `elevated(bit i) = elevated(bit j)`, **remove type `i`** and start again at step 1 for
   this cell with the new flags. The edge words compared are those read when the cell's analysis started. If no pair
   qualifies, next cell.
3. **Two bits**: `A := typeOf(lo)`, `B := typeOf(rest)`. A connection node (`+0x56`): next cell.
   1. Winner and loser: if `beats(B, A)`, `W = B`, `L = A` and the winner's bits are `rest`; otherwise `W = A`,
      `L = B` and the winner's bits are `lo`. `eW := e[W]`, `eL := e[L]`.
   2. **Covered**: if `covers(eW, eL)`: when row `W` of the override matrix has bit `L`, clear `e[L]`, recompute the
      union, set the flags to **the winner's bits only**, and clear `+0x50` if `L` = 10. In both cases, next cell; no
      piece comes from this stage, and the cell stays in `S` for the evaluators.
   3. **Crossing**: if `elevated(W's bits) = elevated(L's bits)`, clear every byte of `eL` whose low two bits equal
      `eW`'s and are non-zero (the shared arms belong to the winner); store `eL` and recompute the union; if `L` = 10,
      `+0x50 &= D(eL)`. Then, whatever the elevation, `+0x50 &= ~D(U)`, where `U` is the OR of the edge words of
      Road, Street, Avenue and DirtRoad.
   4. Look up the table of the pair `(min(W, L), max(W, L))` with the key (edge word of the lower-index type, edge
      word of the higher-index type), using the updated `eL`. Not found: next cell (it stays in `S`).
   5. Found: set `+0xB8` and append `{pieceId, orientation, c}` to the solved list; append `c` to `out`; append
      `(x, z, f)` to the tool's potential-intersection list `+0x08` (the list that
      `QueryUserForPotentialIntersections` walks, §10.7).

`+0x50` is the cell's one-way direction byte (two bits per side).

---

## 9. Tile subsets

### 9.1 Conflict patches: `[RuleOverrides]` and AdjustTileSubsets

**File** (instance `0x10000002`, section `[RuleOverrides]`, entry callback `0x0063D1E0`). Each entry's key and value
are both six numbers `idA, rotA, flipA, idB, rotB, flipB`, each parsed with the pattern
`" %i , %i , %d , %i , %i , %d"`, which must yield 6 fields (`%i` accepts `0x` hex). The key is the **pattern**, the
value the **replacement**; an orientation is `rot | (flip ≠ 0 ? 0x80 : 0)`. Entries go into an ordered map
(`0x00B466D0`) keyed by `idA`, several per `idA` allowed, in file order *[inferred]*.

A pattern describes two pieces side by side, **B east of A** (B on A's side 2), in a normalised frame.

**PatchTilePair(A, B, s)** (`0x006337E0`), where B is the piece on A's side `s` and the patterns are those with
`idA = A.id`:

1. Normalise both orientations to the frame where B is east: an unflipped orientation's rotation becomes
   `(rot + 2 − s) & 3`; a flipped one's `(rot + s + 2) & 3`, keeping bit 7. Call them `nA`, `nB`.
2. For each pattern in map order with `idB = B.id`:
   1. If the flip bits of the pattern's A orientation and `nA` differ, compare against `nA ^ 0x82` and `nB ^ 0x82`
      (the pair mirrored north–south, B still east); otherwise against `nA` and `nB`.
   2. It matches when the A orientation equals, and either `B.id = 0` or the B orientation equals.
   3. On a match: if the replacement's `idA` is 0, return **2** (a conflict that cannot be patched). Otherwise A and B
      take the replacement's ids and orientations (`^ 0x82` again when step 2.1 mirrored), the orientations are
      de-normalised (unflipped `(rot + s + 2) & 3`, flipped `(rot + 2 − s) & 3`), and the result is **1**.
3. No match: **0**.

**AdjustTileSubsets(solved, resolve)** (`0x00634D70`):

1. An empty conflict table: return true.
2. `budget := max(100, 8 × the solved count)`.
3. A **pass**: `changed := false`; for each record A of the solved list, in index order:
   1. `tries := 0`. For each side `s` = 0 … 3, with `n` the cell info across `s` (none: next side):
      1. B := `n`'s solved record when `n.+0xB8 ≥ 0`. Otherwise, when `n` has an occupant, an **existing** B =
         `{occupant PieceId (slot 23), occupant orientation (slot 26), n's key}`. Otherwise next side.
      2. If `n` is fixed (`+0x53`) or a connection node (`+0x56`): B becomes an existing copy with **id 0** (so only
         patterns with `idB = 0` match it).
      3. `r := PatchTilePair(A, B, s)`. If `r = 0` and B is existing, try the reverse once:
         `r := PatchTilePair(B, A, (s + 2) & 3)` with the patterns of `B.id`.
      4. `r = 1`: `budget −= 1`; below 0, return **false**. `changed := true`. An existing B is appended to the
         solved list (and `n.+0xB8` set), so it is placed again with its new piece. `tries += 1`; if `tries > 100`, go
         to the next record; otherwise restart the sides of A from 0.
      5. `r = 2`: if not `resolve`, return **false**. Otherwise put A's cell back into `S`, decrement `+0xB8` of
         every later record's cell, remove A from the list, and **start the pass again from the first record** with
         `changed := false`. A's own `+0xB8` is left as it was *[quirk]*.
4. At the end of a pass: if `changed`, run another pass; otherwise return true.

### 9.2 Pattern checks: `[TileVerifiers]` and VerifyTileSubsets

**File** (the network INI of §2.1 step 1, section `[TileVerifiers]`, entry callback `LoadVerifiersCallback`
`0x0063F7A0`). Each entry adds one **verifier** for the piece id in its key (hex, `RZAtoUint32HexA`). The value is a
list of groups of four comma-separated tokens `m, id, rot, flip`: `m` by `atol`, `id` in hex, `rot` by `atol`, and
`flip` non-zero adds `0x80` to the orientation. Each group is a **requirement** `{id, orientation, m}`. Several
entries for one piece id give several verifiers, any one of which may pass.

**VerifyTileSubsets(solved)** (`0x00635290`): with no verifiers, return true. For each record A, in index order:

1. No verifier for `A.id`: keep A.
2. A verifier **passes** when every requirement holds:
   1. `(dx, dz)` := the offset of `m`, then turned by A's orientation: when rotation bit 0 is set,
      `(dx, dz) := (−dz, dx)`; when rotation bit 1 is set, `(dx, dz) := (−dx, −dz)`; when flipped, `dx := −dx`.
   2. The cell `(A.x + dx, A.z + dz)` lies inside the city (both coordinates below the city size, compared unsigned)
      and has a cell info.
   3. Its piece: its solved record, else its occupant's PieceId and orientation, else id 0 and orientation 0.
   4. The relative orientation: when the flip bits of A's and the piece's orientations differ,
      `(o + oA) & 0x83`, otherwise `(o − oA + 8) & 0x83` (byte arithmetic). The piece id must equal the requirement's,
      and the relative orientation must equal the requirement's orientation.
3. If no verifier of `A.id` passes: put A's cell back into `S`, decrement `+0xB8` of every later record's cell, remove
   A from the list (the next record moves into its index) and note a failure.

Return true when nothing was removed.

---

## 10. Highway intersection rules (`nSC4Networks::cIntRule`)

These rules describe the **prebuilt intersection pieces** (interchanges, highway–road crossings and the like): a
pattern of existing network cells that the piece replaces, the cells it newly occupies and the models it places.

### 10.1 The file (instance `0x10000000`)

`ReadHighwayIntersectionRules` (`0x00641B60`) enumerates every section of the INI (INI slot 14) and hands each to
`ReadIntersectionRule` (`0x00641A50`):

- a section whose name starts with `Ordering` is an **ordering** section (§10.8);
- a section named `HighwayIntersectionInfo_<hex id>` is a **rule** with that id (read with `RZAtoUint32HexA`). The
  rule is created in the global rule map `0x00B452B0` (a hash map keyed by id; entry = `{next, id, rule}`, the rule at
  entry `+8`). Its id is stored, its ring links (§10.8) are set to its own id, its `CellLayout` and `ConsLayout` line
  lists are cleared, its keys are read (`ReadIntersectionEntries`, `0x00640320`), and then the two ASCII layouts are
  parsed (§10.3) when their line lists are not empty;
- other sections are ignored.

**Keys** (compared case-insensitively, `cRZString::IsEqual`), in the order the reader tests them; every key may
repeat, and a later one overwrites (lists append):

| Key | Value | Effect (rule field) |
|---|---|---|
| `Piece` | `x, z, rot, flip, 0xid` (two floats, then `" , %i , %i , 0x%x"`) | append a **model** `{id, rot byte, flip byte, x (f32), z (f32)}` (`+0x04`, 16-byte entries) |
| `StepOffsets` | `a,b` (`%d,%d`, must give 2) | `+0x10 := a`, `+0x14 := b` |
| `CellLayout` | one text line | append to the cell layout lines |
| `ConsLayout` | one text line | append to the constraint layout lines |
| `AutoTileBase` | number | `+0xC8` |
| `AutoPathBase` | number | `+0xCC` |
| `CheckType` | a check-type definition (§10.2) | add to the tag map `+0x64` |
| `CopyFrom` | hex id | if that rule exists: copy it whole into this one (`0x0063E060`), then restore this rule's id and reset its ring links to its own id; otherwise nothing |
| `Transpose` | (any) | transpose the rule (§10.4) |
| `Rotate` | `n` | rotate the rule by `n` quarter turns (§10.4) |
| `Translate` | `a,b` (`%d,%d`) | translate the rule by (a, b) (§10.4) |
| `ReplacementIntersection` | `a,b` (`%d,%d`, the count is not checked) | `+0x48 := 1`; `+0x40 := a`, `+0x44 := b` |
| `ConvertQueryID` | number | `+0x78`: the string id of the conversion question (§10.7) |
| `PlaceQueryID` | number | `+0x7C` |
| `HandleOffset` | `a,b` (`%d,%d`, must give 2) | `+0x94 := a`, `+0x98 := b` |
| `PreviewEffect` | text | `+0x80` (a string) |
| `Costs` | `" %d "` | `+0xB4` |
| `AutoPlace` | (any) | `+0xE9 := 1`, whatever the value |
| `OneWayDir` | number | `+0xE8 := atol(value)` |

The reader also builds the strings `PieceType` and `ResetPieces` but never compares a key with them; such keys are
ignored *[compat]*. Keys are applied in file order, so `CopyFrom` must come before the keys that adjust the copy, and
`Rotate`/`Transpose`/`Translate` act on everything read or copied before them.

Other rule fields: `+0xE4` the **type mask** (the OR of the type bits named by the check types), the tag lists and
bounds of §10.3.

### 10.2 Check types (`CheckType`)

The value is parsed by a small tokenizer (`cParser`, `0x0063DDF0`, `0x0063F1F0`); `#` starts a comment that runs to
the end of the value and `;` ends it. The grammar:

```
<tag> - <Network>:<edges>[,<mask>] [<Network>:<edges>[,<mask>]] [check] [multiok] [optional]
```

- `<tag>` is one character; longer tags fail with "Bad map tag".
- At most two network checks; a third fails with "Too many network checks in one rule".
- `<Network>` is a network name (§1.2); `<edges>` and `<mask>` are hex edge words; the mask defaults to `0xFFFFFFFF`.
- `check` clears the tag's **occupies** flag; `optional` sets **optional** and clears **occupies**; `multiok` sets
  **multi-ok**. By default a tag occupies its cell, is not optional and is not multi-ok.

A check-type record (24 bytes): `+0` type 1, `+1` type 2 (−1 when absent), `+4` edges 1, `+8` mask 1, `+0xC`
edges 2, `+0x10` mask 2, `+0x14` multi-ok, `+0x15` occupies, `+0x16` optional. Each named type's bit is ORed into the
rule's `+0xE4`.

### 10.3 The ASCII layouts

**Cell layout** (`ParseCellsFromASCIIArt`, `0x00609FC0`). Each `CellLayout` line is one row `z` (from 0, in order)
and each character one column `x` (from 0):

| Character | Meaning |
|---|---|
| space, `.` | nothing |
| a letter or digit with a check type | a **tagged cell** `(x, z, tag)` (list `+0x58`, 12-byte entries); if its type occupies, it also extends the **bounds** and joins the **occupied cells** (`+0x4C`) |
| a letter or digit without a check type | nothing |
| `+` | a **new cell** (list `+0x28`): a cell the piece takes that holds no network yet; it also joins the occupied cells and extends the bounds |
| `<` | marks this row as the centre row |
| `^` | marks this column as the centre column |

The bounds start at `minX = minZ = 1000`, `maxX = maxZ = −1000` (`+0x18`, `+0x1C`, `+0x20`, `+0x24`). Afterwards every
coordinate (bounds, occupied, new and tagged cells) is shifted by minus the centre (column, row). Without a `<` or
`^` the centre stays −1, so the cells shift by +1 *[quirk]*. The parser then builds the tile grid (`+0xB8`, width
`+0xC4`) and its map (`+0xD4`) that the preview uses.

**Constraint layout** (`ParseConstraintsFromASCIIArt`, `0x0060A560`) maps characters to constraint codes:
`-` 1, `|` 2, `/` 3, `\` 4, `F` 5, `R` 6, `#` 7, `X` 8; `<` and `^` set its centre (`+0xB0`, `+0xAC`). The grid is
`+0x9C` (width `+0xA8`). Its consumer is outside this spec (§14).

### 10.4 Transforms

All three keep the rule internally consistent; the cell lists hold integer offsets from the anchor.

**Translate(a, b)** (`0x00608D20`): add (a, b) to every new, occupied and tagged cell, to every model position, to the
handle offset (`+0x94`, `+0x98`) and to the bounds; subtract (a, b) from the constraint centre (`+0xAC`, `+0xB0`).

**Transpose** (`0x006091F0`), the mirror across the line x = z:

- swap x and z in every cell list, model position, the handle offset and the bounds (min with min, max with max);
- each check type's edges: swap bytes W ↔ N and E ↔ S, and map each byte whose **whole value** is 1, 2 or 3 to
  `4 − value` (1 ↔ 3); a byte with subtype bits is left as it is (`0x00608C50`) *[quirk]*. Masks: the same byte swap
  only;
- each model: flip toggled; rotation `+1` when it becomes flipped, `−1` when it becomes unflipped (mod 4); the tile
  map entries likewise;
- the constraint and tile grids are transposed (`0x00608F20`);
- `OneWayDir := (−d − 2) & 7`.

**Rotate(n)** (`0x006094B0`), `n & 3` quarter turns; 0 does nothing. One turn maps (x, z) to (−z, x):

- every cell list and the handle offset are turned;
- each check type's edges and masks are rotated left by `8n` bits (byte `i` moves to side `i + n`);
- each model: rotation `+n` (unflipped) or `−n` (flipped), mod 4; its position turned;
- the tile map entries: rotation `+n` or `−n` likewise, the flip kept;
- the constraint and tile grids are turned (`0x00609040`);
- the bounds: for a half turn (bit 1 of `n`), min and max swap and all four are negated; then for an odd turn (bit 0),
  `minZ := minX`, `minX := −maxZ`, `maxZ := maxX` and **`maxX := −(old minX)`**. The correct value would be
  `−(old minZ)`, so after an odd turn `maxX` is wrong whenever `minX ≠ minZ` *[quirk]* (`0x006097A3`–`0x006097B9`). It
  only affects `IsWithinBounds`;
- `OneWayDir := (d + 2n) & 7`.

### 10.5 Matching an intersection rule

**IsWithinBounds(x, z, width, height)** (`0x00608CD0`): `x + minX ≥ 0`, `z + minZ ≥ 0`, `x + maxX < width` and
`z + maxZ < height` (the last two compared unsigned).

**Test(x, z, tool)** (`0x00609A00`) returns 0 (match), 1 (no match) or 2 (hard failure). `ow` := the one-way pattern
for `OneWayDir`, table `0x00AA752C` = `0x22, 0xAA, 0x88, 0x99, 0x11, 0x55, 0x44, 0x66` (index 0–7).

For each tagged cell `(rx, rz, tag)` with its check type `ct`:

1. `ct` optional: next cell.
2. `info` := the tool's cell info at `(x + rx, z + rz)`. None: return **2**.
3. If `ct` occupies:
   1. The cell is an **intersection** when its occupant has all of the flags `0x100000` (network occupant slot 8) or
      `info.+0x52` is set. If so: without `ReplacementIntersection` (`+0x48 = 0`), return **1**. If the flag came from
      the occupant, its anchor (network occupant slot 71, `+0x11C`, two out-values) must equal
      `(x + +0x40, z + +0x44)`, else return **1**.
   2. If `info` is fixed (`+0x53`) and `+0x52` is not set: return **1**.
4. `acc := 0`. For each network check `k` (type `t ≥ 0`): if the cell's flags lack `t`'s bit, return **1**;
   `acc |= bit`; if `(mask_k & (info.edges[t] ^ edges_k)) ≠ 0`, return **2**.
5. If not multi-ok and the cell's flags carry a type bit outside `acc` (`flags & ~acc & 0x1FFF ≠ 0`): return **1**.
6. If `acc` has the OneWayRoad bit and `ct` does **not** occupy: if `((info.+0x50 ^ ow) & D(e)) ≠ 0`, where `e` is the
   one-way check's edges, return **2**.

Then, if the rule has no new cells (`+`), return **0**. Otherwise, for each new cell `(x + rx, z + rz)`: if the
terrain says it holds water (terrain slot 34), return **1**; if the occupant manager has a network occupant there
(slot 32 with the network-type filter `0x1F4F`), return **1**. Finally build the demolition region of the new cells
(`ComputeDemolitionRegionForPlacementCells`, `0x0061F7C0`, with the tool's occupant filter `+0x364`): it fails when a
lot on those cells has a zone type from 10 to 15 (the zone-type mask `0xFC00`, zones.md) or the demolition utility
(query slot 6, flags `0x2001`) refuses; a failure returns **1**, success **0**.

**DoAutoComplete(x, z, tool)** (`0x006097E0`, returns 1). It adds the **extra cells** that complete the
intersection's arms (the tool's list `+0xF0`, network-geometry.md NG3). For each tagged cell (optional ones included)
and each network check `k` with type `t ≥ 0`:

1. `info` := the cell info at `(x + rx, z + rz)`; `mismatch := info` absent, or `(mask_k & (info.edges[t] ^
   edges_k)) ≠ 0`.
2. For `t` = OneWayRoad: `cow := info ? info.+0x50 : 0`, `d := D(edges_k)`, `owbad := (d & (cow ^ ow)) ≠ 0`. If the
   tag does not occupy, `G` := `cow` with each two-bit pair set to `11` when it is non-zero; if
   `(d & G & (cow ^ ow)) ≠ 0`, skip this check (an existing one-way arm points the other way).
3. If `info` exists and neither `mismatch` nor `owbad`: skip. Otherwise add an extra cell (`0x00629170`) at
   `(x + rx, z + rz)` with edges `edges_k`, type `t`, one-way bits `ow & d` and mask `d` (both 0 for other types).

**GetPrimaryType()** (`0x006099A0`): the first type of the tagged cell at (0, 0), else 2 (Highway).

### 10.6 IdentifyHighwayIntersections(v, type) (`0x0063B340`, pop 8)

Called by `PlaceNetwork` after the edge major types (network-geometry.md §8.2). It finds the intersections that a drag
across existing networks forms automatically:

1. Clear the tool's intersection list `+0xCC`. `inRun := false`.
2. For each step `s` < `v`, for each cell `(x, z)` of the step whose cell info has an existing occupant:
   1. Call the occupant's slot 38 (`GetEdgeMaskForType(type)`); the result is not used.
   2. For each rule of the global map, in its hash order (buckets `0x00B452B4`, ids by identity modulo the bucket
      count): the rule matches when `AutoPlace` (`+0xE9`), `+0xE4` meets the cell's flags, `IsWithinBounds(x, z,
      city width, city height)` and `Test(x, z, tool) = 0`.
   3. On a match: the step matched. If not `inRun`: start a run with this rule's id, `(x, z)` and `s`;
      `MarkIntersection(run, true)` (`0x0063B130`) sets `+0x52` on the cell infos of the rule's occupied cells. Later
      matches during a run are ignored.
3. After a step without a match, if `inRun`: **record** the run and append `(x, z, type)` to the tool's
   potential-intersection list `+0x08`; `inRun := false`.
4. After the last step, if `inRun`: record the run and append `(x, z, 4)` to `+0x08`. The 4 is a constant, not the
   type *[quirk]*.
5. Return `v`, on every path. So the caller's failure code `0xF0000005` is never set (network-geometry.md quirk 7).

A **record** (`+0xCC`, 40 bytes): `{rule id, x, z, start step s, s + StepOffsets.a, s + StepOffsets.b, an empty
vector}` *[inferred: the vector]*. `MarkHighwayIntersectionPieces` (`0x0063B7F0`) and
`RemoveHighwayIntersectionPiecesFromSetOfCellsToBeSolved` (`0x0063B1A0`) use the records whose start step is below
`+0x78`; `InsertHighwayIntersectionPieces` (`0x0062C4B0`) places their models at commit (network-tools.md §6.1).

### 10.7 Placing one intersection

**InsertIsolatedHighwayIntersection(x, z, id, real)** (`0x0062CD50`, the intersection tool) places rule `id` at
`(x, z)`:

1. Reset the run state; `+0x50 := real`; `type := GetPrimaryType()`; record the intersection in `+0xCC`; one step of
   one cell `(x, z)`; the dragged and valid counts `+0x74 = +0x78 = 1`.
2. The cells: the rule's new cells plus the extra cells (`DoAutoComplete`), without connection nodes.
3. The demolition region of those cells (`+0x210`, `+0x220`) must succeed, else return 0.
4. Slot 68 (reserved tiles, `(&v, type, 1)`): if it cuts `v` below `+0x78`, fail with `0xF0000002`.
5. Cell infos (slot 71), `DoAdditionalDraws`; `Test` must return 0, else `0xF0000007`.
6. `MarkHighwayIntersectionPieces(true)`; `BuildSetOfCellsToBeSolved`; `SolveNetwork(type, false)` must return a
   positive count, else `0xF0000007`.
7. The smoothing levels 0–2 (slot 56); a failure at level 3 gives `0xF0000006` or `0xF0000008` (network-geometry.md
   §9.4).
8. If `real`: slots 70, 60, 57 and the position-changed marks (network-tools.md §6.1).
9. Slot 55 (`InsertOccupantsForSolvedCells`), `InsertHighwayIntersectionPieces`, `MarkOccupantsUsable`; return 1.

**QueryUserForPotentialIntersections** (`0x0062D450`). For each `(x, z, ·)` entry of the tool's `+0x08` list (only x
and z are read): get the highway tool (`GetNetworkTool(2, true)`); take the first rule, in map order, that has a
`ConvertQueryID` and whose `Test(x, z)` is 0; do a dry `InsertIsolatedHighwayIntersection(x, z, id, false)`; if it
succeeds and the city's funds (city slot 52, budget slot 6) cover its cost, show the confirmation text (string table
group `0x2A592FD1`, instance `ConvertQueryID`; the fallback text is `## Intersection query string missing ##`). On
"yes": place it for real, spend the cost (budget slot 8) and post message `0xEA5EC59F`.

### 10.8 Ordering sections (the intersection tool's cycling)

An `Ordering…` section (`ReadIntersectionOrderingEntries`, `0x006416B0`) reads one of two keys, each a comma list of
hex rule ids:

- `RotationRing`: the listed rules form a **circular list** for the rotate command: each rule's `+0xEC` (previous)
  and `+0xF0` (next) are linked in list order and the last closes on the first;
- `AddTypes` (read only when there is no `RotationRing`): links the listed rules' rings into a second circular list
  through `+0xF4` and `+0xF8`, for cycling between intersection types *[inferred]*.

Unknown ids are skipped. These links only drive the tool's user interface; the solver never reads them.

---

## 11. From a solved cell to an occupant

A solved record `{piece id, orientation, key}` becomes an occupant at commit (`InsertOccupantsForSolvedCells`, slot
55, `0x006258C0`; network-tools.md §6.2). The record's piece id becomes the occupant's **piece id** (network occupant
slot 23) and its orientation byte the occupant's rotation and flip (read back through slot 26 by §9) *[inferred: the
slot 26 meaning]*.

- An **ordinary** network occupant (`CreateNetworkOccupant`, slot 54, `0x00624210`, with no class id) is a textured
  cell: its piece id names the cell's texture. The multi-type solutions file calls the field `textureId`. The texture
  lookup belongs to the network rendering, outside this spec (§14).
- A **prebuilt-model** occupant (class `0x49C1A034`) takes its model from an exemplar: `GetOccupantExemplar(piece,
  group)` (`0x006244B0`) fetches the resource with key `{type 0x6534284A, group, instance = piece id}` through the
  resource manager (slot 3, interface `0xA52160F5`), and `SetExemplarForOccupant` (`0x00624860`) adds the model when
  it is found. `group` defaults to the tool's type record `+0x0C` (§2.2); `InsertHighwayIntersectionPiece`
  (`0x006276D0`) passes `0xA8434037` (the Highway group) for the intersection models of §10.

So a rule file names pieces by id only; what the id selects depends on the occupant class and, for prebuilt models,
on the group *[compat]*.
