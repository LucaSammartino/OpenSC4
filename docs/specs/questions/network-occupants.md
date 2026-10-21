# network-occupants: open questions
<!-- Questions about [network-occupants.md](../network-occupants.md). Append new entries at the END; when one is answered, move it to answered/network-occupants.md. Rules: README.md. -->

## From the network occupants spec (re-netocc lane, 2026-10-20)

Spec: [network-occupants.md](../network-occupants.md) §§1–10. RE-side follow-ups; nothing here blocks an
implementer of the class tables and records.

<!-- from: the network occupants spec (2026-10-20) -->
- **NO1 - network flag 0x40 vs the intersection bit:** the meaning of network flag `0x40` tested by
  `IsIntersection` (primary slot 55, `0x0061BD80`) versus the `0x100000` intersection bit used elsewhere
  (network-manager.md §5). [inferred: `0x40` is a styling/presentation bit; the slot checks it before the
  highway-intersection piece-id lookup at `0x00B4528C`]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO2 - the power pole's delegate interface:** the identity of the delegate interface `0x26A68448` answered by
  the power pole via `+0xBC` (jump-table dispatch; likely a UI/drag helper). Probe: dump the `[+0xBC]` vtables on
  a live pole. [unknown]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO3 - the underground occupant's type:** does `cSC4UndergroundOccupant` ever carry a distinct occupant type?
  Its base ctor leaves `0x74758925`; the demolition dispatcher never names an underground type. [inferred:
  underground pieces are dispatched as `0xC772BF98` through their `cISC4NetworkOccupant` interface]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO4 - the prebuilt record's first five fields:** the semantics of the prebuilt `Write` fields
  `+0x170`/`+0x174`/`+0x178`/`+0x17C`/`+0x180` (network-occupants.md §2.8): model refs? shear inputs?
  [not specified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO5 - the 0x4835C6A7-AC property family:** SDK names of the property-ID family `0x4835C6A7` (edge types),
  `0x4835C6A8` (passage ceiling), `0x4835C6AA` (one-way), `0x4835C6AB` (anchor), `0x4835C6AC` (retrofit) stored
  in the occupant's own property holder. [unknown]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO6 - the pipe's +0x268 field:** the meaning of pipe `Read`'s extra u32 → `+0x268` and `+0x258 = 1`
  (network-occupants.md §3.3): funding state? watered state? [unverified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO7 - GetSortValue's stored field:** which field holds the sort value of an underground occupant when flag
  `0x6000000` is set (§3.2); the negative displacement (`−0x178`/`−0x274`) in the decomp is ambiguous.
  [not specified]

<!-- from: the network occupants spec (2026-10-20) -->
- **NO8 - the connection registration strings:** the strings `0x00B0B6D0`/`0x00B0C388` passed with the class IDs
  to helper `0x00654710` by the pipe/subway-connection `Init` (§3.6): GZCOM class names? [unknown]
<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO9 - IsManagedGlobally for the underground family:** §2.7 gives slot 5 for the plain class (false) and the
  prebuilt/bridge/tunnel primaries (true) but not for `cSC4UndergroundOccupant`/pipe/subway (`0x00AAAD40`,
  `0x00AA97B8`, `0x00AAA3C8`). The core answers false (inherited plain body). Which body do those vtables hold?
  (Numbered NO9 per QUESTIONS.md; §12 also uses NO9/NO10 for two settled notes.) [not specified]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO10 - the pylon's QueryInterface IIDs:** §1.2's pylon row answers only `0xC9B6D7D6` (and `0x452294AA`), while
  the prose says every class accepts `0xA9764C8C`, and other rows also accept the own class ID. Does the pylon QI
  answer `0xA9764C8C` and/or `0x49B6D69B`? The core follows the row (neither). [ambiguous]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO11 - the tunnel constructor's flag write:** §2.8 says `+0x13C &= ~0x400 | 0x10C00000`, "clearing the prebuilt
  bit", but the prebuilt bit is `0x400000`. Is the cleared mask `0x400` or `0x400000`, and is the expression
  `(f & ~mask) | 0x10C00000`? The core applies the literal `0x400` (a tunnel keeps `0x400000`). [ambiguous]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO12 - the path-connection table shape and the wildcard count:** §2.6 addresses `[+0x144 + exit*4 + type]` and
  `[+0x144 + type]` (a 4 × 8 byte region that also contains the saved `+0x15C` u32), traffic-network.md §7.2 says
  `(pathType, entry, exit)` with a bit per path type. Which index is entry/exit, how many bytes per row, and with
  `0xFF` wildcards what is counted (matching entry/exit cells, or path-type bits too)? The core keeps
  `[entry][exit]` path-type bit bytes and counts (type, entry, exit) triples. [ambiguous]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO13 - the power line's field offsets and record:** §5.1's layout puts the cell at `+0x40/+0x44` and three
  poles at `+0x48/+0x4C/+0x50`; its slot table reads `GetCell` from `+0x44/+0x48` and `GetConnectedPoles` from
  `+0x4C/+0x50` plus byte `+0x54`; §5.2 writes u32 `+0x30…+0x40` then references from `+0x4C`, `+0x44`, `+0x48`.
  Which live field is each record word/reference, is the cell z saved, does `Read` check the version (only "v2"
  is stated), and does `SetPosition`'s float→int helper truncate or round? The core keeps the record as a literal
  field list and truncates. [ambiguous]

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- **NO14 - conformance test 6's slot:** §13 test 6 says "`SetRotation(2)` then slot 24 returns 2", but §2.4 maps
  slot 24 to `GetFlip` (slots 26/24/25 = orientation/flip/rotation). Is the test's slot 25 (`GetRotation`)? The
  core tests `GetRotation`. [ambiguous]
- **NO15 - the `S3DTransform` stream forms:** §2.3 field 3 writes `operator<<` (`0x005DA8C0`); lot-records.md §2.1
  gives its form (u8 mask; `& 4` 9 floats, `& 1` 3 floats, `& 2` 1 float, in that order). Which mask does the
  writer emit for a given live transform (the `+0x01` flags byte of the 0x38-byte object, a computed mask, or
  always 7), and what does the reader leave in a part the mask omits? And what is the byte layout of
  `S3DTransformLegacyLoad` (`0x005DA930`, records below v5; effects.md §… calls it "56 bytes")? The core keeps
  the mask as data, leaves omitted parts at identity, and fails v4 records. [missing]
- **NO16 - the §2.3 record's field shapes and v5-v7 conversions:** (a) field 4 says 4 x {5 u32, 4 u8} from
  `+0x94` but notes the loop covers only `+0x94...+0xE8`, then field 5 writes `+0xF4/+0xFD/+0xF8/+0xFC`: is the
  quad loop 3 or 4 iterations, and which quad dword is the fifth u32? (b) field 8's type column gives
  u32 + 3 f32 + 3 f32 + u32 + u32 + 4 u8 (40 bytes), its notes name u32 + 3 floats + u32 + u32 + 4 bytes
  (28 bytes), and the stride is 0x4C: which is the triangle entry? (c) field 9 names three u32 pairs at
  `+0x11C...+0x130` but §2.2 puts the box at `+0x120...+0x134`. (d) the rule of
  `ConvertV5FlagsToCurrentFlags` (`0x006191A0`, v < 6). (e) which "two floats it just read" the v >= 7 tail
  passes to slot 0x120, and what v < 7 with `+0x15C == 0` does. The core keeps the record in stream order
  (4 quads, type-column triangles), masks without converting, and reports the tail decision only. [ambiguous]
- **NO17 - the underground record and heights:** (a) §3.4 says the underground record is "the base network
  record (§2.3) with version 3": does the underground `Write` emit its own u16 3 and then the v8 base record,
  or the base record with version 3? (b) its offsets (u8 `+0x48`, u8 `+0x49`, u32 `+0x58`, ..., u32 `+0x260`)
  clash with §2.2/§3.1 (`+0x48` is the u32 lifecycle word, `+0x68` the model instance): which object are they
  relative to? (c) the byte layout of the wall-quad helper `0x0065B070` (six/seven calls in §3.4/§3.6).
  (d) `SetTerrainHeights(const float[4])` writes `+0x248...+0x258` (five floats) while `GetTerrainHeights` reads
  `+0x24C...+0x258` and `GetPosition` takes the minimum of `+0x248...+0x254`: which four floats are the heights?
  And pipe/subway `Read` set "`+0x258 = 1`" (a float height slot) where §3.4 sets bytes `+0x259`/`+0x25A`.
  (e) `IsImaginaryConnection` with no neighbour found. (f) `GetPosition` sums five points' x/z without dividing:
  confirm (not an average). The core implements the stated parts and leaves the record and
  `SetTerrainHeights` unimplemented. [ambiguous]
