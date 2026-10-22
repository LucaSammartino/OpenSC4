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

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- **NO18 - pipe/subway `Read` after a failed underground record:** §3.3 says pipe `Read` "reads the same and then
  sets the byte `+0x260 = 1`"; when the inner underground `Read` returns 0 (wrong version, failed base), does the
  pipe still read its u32 `+0x270` and set `+0x260`, or return at once? The core returns at once. [ambiguous]

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- **NO19 - the v < 7 cell rounding:** §2.3's read tail computes `floor(((v0.x + v2.x) · 0.5 + dx) · 0.0625)` "in
  x87 then rounded through f32". Is the sum `v0.x + v2.x` itself rounded to f32 before `· 0.5 + dx`, or only the
  final product? The core keeps every step in f64 and rounds the product to f32 before the floor. [ambiguous]

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- **NO20 - `IsImaginaryConnection` with dir > 3:** the offset table `0x00B0CFE8` has four (dx, dz) pairs; what does
  a direction above 3 read (the bytes after the table, or a guard)? The core returns false. [missing]

<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NO24 - §5.2, `ComputePolePosition` "guarded against `-FLT_MAX`":** what happens when the terrain answers
  `-FLT_MAX` (`0x00AA9F0C`): is y left at `-FLT_MAX`, replaced by a value (0? the previous y?), or does the call fail
  and the pole is skipped? The core returns no position. [ambiguous]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO27 - §10.3 Bind constants and format:** what numeric colour is stored at `0x00B0E37C`, what numeric
  format does `PackStandardVertexFormat(2)` return, and what is the processor's destination format?
  The core requires these three constants explicitly; there is no assumed default. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO28 - §10.3 processor LightModel:** the owner dispatch, flip-adjusted lighting transform and
  ReleaseModel order are published, but the processor's normal generation and colour-lighting arithmetic
  are not. What are those contracts? The core records an explicit pending LightingRequest and releases
  its processor source references, without accepting an original owner output or inventing colours. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO29 - §10.3 Clip mode 0:** the six-plane clipping path is expressly unspecified. What is its exact
  edge walk/interpolation/output order? The core returns UnspecifiedClipMode for a non-null box in mode 0;
  the network caller always uses the fully implemented mode 1. Null lo still returns immediately. [missing]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO30 - §10.2 clipped path with option `0x6A845768`:** does the true option skip Bind and FlipAndLight
  after Clip, or does the clipped row override the options-skip row for all three calls? The core treats
  the explicit parenthesis as an exception for Clip alone: Clip runs, Bind/FlipAndLight skip. [ambiguous]

<!-- from: core-networkmodel-contractfix4 (2026-10-22) -->
- **NO31 - §10 instance constructor/destructor inventory:** beyond the published object size, base-class
  identity, processor member, resource reference and prebuilt cache lifetimes, what are the initial state
  and teardown side effects of instance ctor `0x00614A30` and destructor `0x00614830`? The pure core models
  resource/mesh/processor reference lifetime with RAII; it does not claim unreported native body effects. [missing]

  NO29 implementation refinement: the published mode-0 x/z bounds early-out is implemented too. Only a
  mode-0 call that actually needs six-plane cutting returns `UnspecifiedClipMode`; null lo or contained
  x/z bounds return unchanged. No six-plane output is fabricated.
