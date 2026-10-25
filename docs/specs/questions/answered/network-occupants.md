# network-occupants: answered questions (archive)

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- ~~**NO33 - sections 4.1-4.3 power-pole owner algorithms:** please publish constructor defaults and
  the numeric connection point tables `0x00B0C0D8`/`0x00B0C198`, their subtype extraction and unlisted
  subtype behavior; connection lookup/copy ownership, RemoveConnection, BreakAllConnectionsAtDestination,
  refresh, SetDistressed, unnamed flag storage, slots 16/17 outputs, ChangeZoomLevel, SetPosition,
  SetModelPosition, LoadModel key/failure semantics, bounds/transform/sort, highlight and wire Pick,
  Draw/DrawPowerlines, floor/wall generation, and shutdown/resource ownership. The spec gives member
  names/addresses but no algorithms. The core implements published state, two table selections and Init's
  known state changes; no owner-result callbacks or invented geometry are supplied. [missing; sections
  4.1-4.3 remain open, including native pool allocation required by section 13 test 4]~~
  **Progress 2026-10-23 (re-netocc-answers5):** constructor defaults, exact table bits/retained-pointer
  behavior and unrotated floor/wall generation now appear in §§4.1,4.6,4.7.1 and tests 43–44. Harvested
  §§4.2–4.7 also supply connection removal, distress, model/position and drawing summaries. Complete
  connection/cell-walk, model/resource, transform/bounds/Pick/Draw algorithms still require verification;
  the diagonal floor's numeric operation order remains NO43. NO33 stays open for full owner closure.
  **Progress 2026-10-23 (re-netocc-answers5d):** §4.7.2/tests 54–55 settle the occupant bounding
  rectangle, sampled-list eligibility, unsigned extrema comparisons, signed output conversion and
  wrapping maximum-cell increment. Remaining model/Pick/wire/Draw contracts still prevent closure.
  **ANSWERED 2026-10-25 (re-netocc-no33):** DIFFERS — network-occupants.md §§4.1–4.3/4.6–4.7
  and tests 62–73 complete the native 0x124 pool, output/transform, connection lifetime/cell walk,
  model failure/zoom, Pick/Draw and wire contracts. Enumeration returns the pole's own counted model
  interface even without a delegate; LoadModel always returns false and preserves an old delegate on
  acquisition failure; Pick is one model-and-walls box; wet refresh appends coarse curves. Saved +110
  remains independent opaque storage (NO42), with no invented meaning. Numeric operation order and
  constants were checked against instruction bytes; see the cited addresses in those sections.

<!-- from: re-powerpoles (2026-10-21) -->
- ~~**NO43 - §4.2, §4.6, §4.7:** the pole's floor and walls (`CreateFloor` `0x0064BFD0`, `AddWallQuad` `0x0064C2C0`,
  `CreateWalls` `0x0064C470`), `Draw` and `DrawPowerlines` (`0x0064C800`, `0x0064B750`), `GetBoundingBox`
  (`0x00649A00`), `Pick` (`0x0064A870`) and the cell walk under a wire (`0x0064A4D0`) in detail. [summary]~~
  **ANSWERED 2026-10-25 (re-netocc-no33):** DIFFERS — network-occupants.md §§4.6.1/4.7.1–4.7.5
  and tests 44,54–55,65–66,68–70 give the complete geometry/drawing rules. The diagonal angle is the
  double 0.78539001941680908203125, with f32 c/s bits `3F350554`/`3F350493`; transformed x alone is
  rounded before translation (`0x0064C217` versus z's final store `0x0064C248`). Test 66's translated
  vertex z=`B4400000` rejects rounding z early. Cell walking truncates, retains duplicates and does
  not force the endpoint; occupant bounds, Pick and Draw use the distinct inputs specified there.

<!-- from: glue-netocc-shadow5 (2026-10-23) -->
- ~~**NO45 - §2.3 byte-exact record-shadow comparison versus required Read fixups:**
  Native Write saves flags unchanged whereas Read clears `0xE000`, forces triangle
  alpha to `0xFF`, and can change transform mask 7 to 5 at scale 1.0. Which
  comparison boundary applies, including inherited underground records? The test
  `native_read_fixups_prevent_an_unconditional_byte_round_trip` demonstrates the
  flags counterexample. [implementation held pending contract]~~
  **ANSWERED 2026-10-23 (coordinator's harness decision):** normalize native Write
  using exactly the documented Read fixups: clear flags `0xE000`, force triangle
  alpha `0xFF`, change transform mask 7 to 5 when scale is 1.0 (omit scale bytes).
  This is the native save/reload reference. Count changed fields separately as
  `fixups=N` on every line; every other byte must match. Live Read is unchanged.
<!-- from: re-ugbase (2026-10-20) -->
- **NO41 - §3.1, §3.6:** what the saved underground `+0x268` (returned by slot 96, set by nothing in these
  classes) and the connection's `+0x68`, `+0x170`, `+0x178` and `+0x240` hold. [as read]
  **ANSWERED 2026-10-23 (re-netocc-answers5d):** DIFFERS — §3.6.3/test 51: base +268 is an opaque
  saved model-key word; Init's descriptor store is actually primary+26C. Connection words are active
  wall count, upper anchor elevation, signed shade index, and creation class ID respectively.

<!-- from: core-netocc-poles5 (2026-10-23) -->
- ~~**NO44 - §6.1 constructor enum defaults:** what initial 32-bit values does the pylon constructor assign to UI state +6C, dispatch state +70 and dispatch type +74? The published constructor defaults cover dirty, references, transform, call, blink, drag and the v5 tail but do not specify these three values. [unpublished; core construction requires the three values as explicit inputs rather than assuming zero]~~
  **ANSWERED 2026-10-23 (re-netocc-poles5):** DIFFERS — §6.1 and §13 test 45: UI=1,
  dispatch state=0; dispatch type is not initialized and retains allocation bytes. No third numeric
  default exists. Verified both most-derived and basic-model constructor instruction stores.

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- ~~**NO35 - section 4.4 power-pole record and live layout:** please specify the wire-point list count
  encoding, the three `0x006574A0` struct stream bodies, exact four vertex offsets/order and live-to-record
  mapping. Section 4.1 places model IDs at C0/C4 while section 4.4 writes B8/BC/C0, and GetCell reads
  C8/CC although layout says C8 is bbox dirty and CC/D0 are cells. Position and vector offsets likewise
  overlap the record's unrelated words. Which is authoritative? What is released at B0 after Read,
  given B0 is also the connection vector end? Which reference uses which of the two class-ID IIDs,
  and what happens on failed reads? No pole record codec is fabricated. [missing; section 4.4,
  section 11 quirk 11 and section 13 test 16 remain open]~~

  NO35 refinement: section 3.4 does define helper `0x006574A0` as a u32 count and
  24-byte quad vertices, so that part can reuse the existing wall codec. The
  wire-point nesting/counts and inconsistent live/record offsets still need clarification.
  **ANSWERED 2026-10-23 (re-netocc-answers5):** DIFFERS — §§4.1/4.4, quirk 11 and test 16:
  record offsets are serializable-view relative (+C to get primary); cleanup releases live +BC delegate.
  Destination pole IID is 0x09C05C6A; counted linked list uses line IID 0xC9C05C5D. Both geometry
  collections contain counted inner xyz lists. Four xyz triples go through the selected shared +E4
  pointer. Three wall lists are count + (xyz, uv, RGBA) per vertex. Reached-body failure has no rollback.

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- ~~**NO37 - sections 6.1-6.3 pylon offsets, Read and placement:** the record calls +5C and +70
  transforms but also names their interior +60/+64/+68 as separate bytes and +74 as a call number.
  How do these map to live state? Does v3 use legacy decoding for both transforms? Which IIDs are
  requested for the two refs, what are absent-v5 tail defaults, and when is the old +6C source released
  on Read failure? Shutdown says release +78 while the source is +6C: same resource or distinct?
  What are the exact state bits and manager/renderer calls for PlaceInOccupantManager and PlaceInRenderer,
  and what is the IsInitialized branch/return for repeat Init? Current literal implementation keeps
  serialized state separate, uses legacy for both transforms, preserves absent tail state, reads generic
  serializable refs, releases old source immediately before the replacement read, skips repeated Init,
  and removes an inserted occupant through city-handle/manager primitives before the three unsubscriptions.
  No unspecified renderer calls/flag masks are invented. [ambiguous; sections 6.1-6.3, conformance 19/20
  have executable partial coverage but no whole-owner closure]~~
  **ANSWERED 2026-10-23 (re-netocc-answers5):** DIFFERS — §§6.1–6.3, tests 19/20: serializable-view
  offsets require +C; first field is bool, only one transform exists (live +7C); call is u16 +14C;
  the alleged int64 is GZStr +1AC. Source IID 0x26BB767E, owner IID 1; v3/v4 preserve absent tail.
  Full placement transition/result table and repeat-Init true/no-op are specified; renderer insertion is +70.

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- ~~**NO34 - sections 4.2-4.3 and section 13 test 17 demolition cost:** how do the control exemplar
  properties (in particular float `0xE9A18D3F` at `0x00B46754`) update the signed int64 demolition
  statics at `0x00B46770/74`? The published init table does not write those statics. The core can
  return supplied low/high words verbatim, but does not invent a conversion or scaling formula.
  [missing; exemplar-to-quote half of conformance test 17 remains open]~~
  **ANSWERED 2026-10-23 (re-netocc-answers5):** DIFFERS — §4.3 and test 17: one-time StaticInit reads
  signed-int64 property `0x0A2ECED8` from the control exemplar directly into the zero-initialized quote.
  `0xE9A18D3F` is independent; there is no scaling formula. Missing/failed lookup preserves the quote.
<!-- Archive for [network-occupants.md](../../network-occupants.md). Lanes do not read this file. Open questions are in ../network-occupants.md. -->

<!-- from: re-netocc-contracts4 (2026-10-21) -->
- **NO15 - the `S3DTransform` stream forms:** §2.3 field 3 writes `operator<<` (`0x005DA8C0`); lot-records.md §2.1
  gives its form (u8 mask; `& 4` 9 floats, `& 1` 3 floats, `& 2` 1 float, in that order). Which mask does the
  writer emit for a given live transform (the `+0x01` flags byte of the 0x38-byte object, a computed mask, or
  always 7), and what does the reader leave in a part the mask omits? And what is the byte layout of
  `S3DTransformLegacyLoad` (`0x005DA930`, records below v5; effects.md's component placement row calls it "56 bytes")? The core keeps
  the mask as data, leaves omitted parts at identity, and fails v4 records. [missing]
  **ANSWERED 2026-10-21 (re-netocc-contracts4):** DIFFERS — network-occupants.md §2.3 field 3 and "Transform read rules": the writer emits the live mask byte (+0x65 of the object, +1 of the transform) unchanged; the reader resets to identity only when the live mask is non-zero, ORs each present bit, clears bit 2 when the scale read is exactly 1.0; omitted parts keep their prior (identity after reset) values. LegacyLoad (v4) is 56 bytes: 9 f32 rotation, 3 f32 translation, 1 f32 scale, u32 whose low byte replaces the counter; mask bits are ORed, never reset.
- **NO16 - the §2.3 record's field shapes and v5-v7 conversions:** (a) field 4 says 4 x {5 u32, 4 u8} from
  `+0x94` but notes the loop covers only `+0x94...+0xE8`, then field 5 writes `+0xF4/+0xFD/+0xF8/+0xFC`: is the
  quad loop 3 or 4 iterations, and which quad dword is the fifth u32? (b) field 8's type column gives
  u32 + 3 f32 + 3 f32 + u32 + u32 + 4 u8 (40 bytes), its notes name u32 + 3 floats + u32 + u32 + 4 bytes
  (28 bytes), and the stride is 0x4C: which is the triangle entry? (c) field 9 names three u32 pairs at
  `+0x11C...+0x130` but §2.2 puts the box at `+0x120...+0x134`. (d) the rule of
  `ConvertV5FlagsToCurrentFlags` (`0x006191A0`, v < 6). (e) which "two floats it just read" the v >= 7 tail
  passes to slot 0x120, and what v < 7 with `+0x15C == 0` does. The core keeps the record in stream order
  (4 quads, type-column triangles), masks without converting, and reports the tail decision only. [ambiguous]
  **ANSWERED 2026-10-21 (re-netocc-contracts4):** DIFFERS — §2.2/§2.3: Write/Read run with this = object+8, so record offsets are object-8. (a) the loop is 4 iterations over ONE quad of 4 vertices {f32 x,y,z,u,v; bytes R,G,B,A}; field 5 is four separate scalars u32 +0xFC, u8 +0x105, u32 +0x100, u8 +0x104. (b) triangle = u32 + 3 such vertices = 76 stream bytes (0x4C in memory). (c) box = 6 f32, min/max interleaved per axis, object +0x124..+0x138. (d) bit map table in §2.3. (e) v<7 with +0x164 != 0 reads two extra f32 offsets, derives a cell, sets property 0x4835C6AB = (z<<16)+x via slot 72; every v<8 with +0x164 != 0 adds property 0x4835C6AC; the ref and int64 are read in all versions.
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
  **ANSWERED 2026-10-21 (re-netocc-contracts4):** DIFFERS — §3.1/§3.2/§3.4: (a) own u16 3 then the full v8 base record (own u16 8), then a 13-field tail; (b) offsets are object-8 (code this = object+8); (c) wall list = u32 count + count x 24-byte vertex {5 f32, R,G,B,A}; (d) the heights are the four f32 +0x24C..+0x258 (Set writes exactly these; GetPosition uses this = object+4); pipe/subway set the byte object +0x260, underground Read +0x261/+0x262; (e) no neighbour -> false, and a piece without flag bit 31 -> false; (f) x/z are the 4-vertex average (x 0.25), not a sum of five.
<!-- from: core-networkmodel4 (2026-10-21) -->
- **NO21 - §10 `Deform` details:** (a) is the corner bend bilinear over the four corners, or a two-triangle split (which diagonal)? (b) is the deformed y the flat model's y plus the interpolated height, or replaced by it, and does each call start from the flat model? (c) the model's local x/z frame: does corner (x, z) sit at local (0, 0) with a 16 m extent, or is the model centred (−8…8)? (d) are vertices outside the cell clamped or extrapolated? [bilinear; flat y + height; recomputed from the flat model; origin (0, 0), extent 16; extrapolated]
  **ANSWERED 2026-10-22 (re-networkmodel-answers4):** DIFFERS — §10.3: (a) two triangles, not bilinear: diagonal v0-v2 (x′=z′) with network flag 0x1000000 clear, v1-v3 (x′=−z′) when set; plane P1 for s≥0, P0 otherwise (planes from RecomputeShearTransform 0x0061D230); (b) y += a·x′+b·z′+c on a fresh private copy of the flat model (c relative to the piece position y); (c) centred frame, corners at ±8 (extent 16), after rotation table 0x00614CE4, flip x′=−x′ and origin offset; (d) extrapolated, no clamp.
<!-- from: core-networkmodel4 (2026-10-21) -->
- **NO22 - §10 `FlipAndLight`, `Clip`, `Prelight` bodies:** what does the flip (bit 7 of `+0x104`) do to the model (mirror axis, winding, UVs), what inputs and outputs do the lighting, the underground/clipping and the pre-light passes have, and in what order does the renderer call the four overrides? [core hands the three passes to a renderer provider with the flip bit; no order imposed]
  **ANSWERED 2026-10-22 (re-networkmodel-answers4):** DIFFERS — §10/§10.2/§10.3: the four are non-virtual members called only from the prebuilt GetModelInstances 0x0061E8A0; "Prelight" 0x00614350 is Bind (processor SetModel: private vertex copy); order Bind→Deform→FlipAndLight (surface) or Clip→Bind→FlipAndLight (flag 0x40000); flip mirrors by negating transform row 0 and FlipAndLight only reverses winding (indices/ranges), then LightModel(*0x00B43DDC, flip) and ReleaseModel. Clip kept side/edge interpolation still open (NO25).
<!-- from: core-networkmodel4 (2026-10-21) -->
- **NO23 - §10 model-instance lifetime:** (a) when is the `cSC4NetworkOccupantModelInstance` created (ctor `0x00614A30` called from `Init`, the prebuilt `SetModel`, or the first draw) and released (`Shutdown`)? (b) is it the `+0x38`/`+0x68` `cS3DModelInstanceBasic` sub-object of §2.1/§2.2 or a separate object? (c) what does `GetModelInstances` (slot 33) return for a piece with no model (null, or an instance over the §2.2 quads)? [created from the exemplar model when there is one, owned by the piece; null with no model, which renders the §2.2 fallback]
  **ANSWERED 2026-10-22 (re-networkmodel-answers4):** DIFFERS — §10.1: created fresh on every prebuilt GetModelInstances call (ctor callers 0x0061E9B7/0x0061F06C), separate 0xE4 heap object, cached in AutoRefs +0x1C4 (main) / +0x1C8 (second model +0x19C); plain GetModelInstances 0x0061AF90 never returns null — it always emits the piece's embedded +0x38 cS3DModelInstanceBasic (the §2.2 quad path). Release in dtor/Shutdown still open (NO26).
<!-- from: re-networkmodel-answers4 (2026-10-22) -->
- **NO26 - §10.1 cache release:** do the prebuilt destructor and `Shutdown` release the +0x1C4/+0x1C8 instance caches, and is +0x1C0 reset? [not proved]
  **ANSWERED 2026-10-22 (re-networkmodel-answers4):** DIFFERS — §10.1 item 3: Shutdown 0x0061D0D0, gated by +4 view slot 5, releases and clears +0x1C4 then +0x1C8, then base Shutdown; +0x1C0 not reset; destructor not examined.
<!-- from: re-networkmodel-answers4 (2026-10-22) -->
- **NO25 - §10.3 `Clip` 0x00618B00 kept side and cut:** with lo = (−FLT_MAX, h, −FLT_MAX), hi = +FLT_MAX, mode 1, is the geometry above h kept (inside the box) or below; how are cut-edge vertices interpolated (position, uv, colour) per format 2/0x80004001/10; exact early-out test on the bounds? [not proved]
  **ANSWERED 2026-10-22 (re-networkmodel-answers4):** DIFFERS — §10.3 Clip rules 1-9: kept when lo.y ≤ y ≤ hi.y (outcode 0x00614100, y bits 0x30), so the call site keeps geometry at/above h and removes below; mode≠0 cuts iff min.y<lo.y or max.y>hi.y (unordered = cut, 0x00618B65-0x00618BC0), mode 0 tests x/z only, null lo returns; cut vertex lerped from the OUTSIDE end, t=(plane−out)/(in−out) stored f32, a′=(in−out)·t+out for x,y,z,u,v (0x00614490/0x00614170; 24-byte 0x006145D0/0x00614200); format 10 colour bytes trunc via _ftol2 (0x0061424F-0x006142A9); formats 2/0x80004001 → 20-byte pool 0x00B45240, 10 → 24-byte pool 0x00B45244, others → empty mesh; polygon emitted as fan, cut vertices never shared (0x006175C0); bounds clamped to the box, not recomputed (0x00618FD2-0x00619024). Vector D: outside-anchored gives 1.0 (0x3F800000), inside weighted form 0x3F7FFFFF.
<!-- from: core-powerline4 (engine lane, 2026-10-21) -->
- **NO24 - §5.2, `ComputePolePosition` "guarded against `-FLT_MAX`":** what happens when the terrain answers
  `-FLT_MAX` (`0x00AA9F0C`): is y left at `-FLT_MAX`, replaced by a value (0? the previous y?), or does the call fail
  and the pole is skipped? The core returns no position. [ambiguous]
  **ANSWERED 2026-10-22 (re-powerline-completion4):** DIFFERS — network-occupants §5.2/power-line-tool §4.2:
  start height at -FLT_MAX, replace only by strictly greater ordered corners, add 0.5, store float32.
  All-sentinel corners return y=-FLT_MAX; no failure/skipped pole. NaN corners are ignored. Evidence
  `0x0064F00B`, `0x0064F061–0x0064F0BB`; `0x00AA9F0C=0xFF7FFFFF`.

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO1 - network flag 0x40 vs the intersection bit:** the meaning of network flag `0x40` tested by
  `IsIntersection` (primary slot 55, `0x0061BD80`) versus the `0x100000` intersection bit used elsewhere
  (network-manager.md §5). [inferred: `0x40` is a styling/presentation bit; the slot checks it before the
  highway-intersection piece-id lookup at `0x00B4528C`]~~
  **ANSWERED 2026-10-21 (re-ugbase):** DIFFERS — `0x40` is `1 << 6`, the avenue network type bit (`IsOfType(6)`,
  §1.4), not a styling bit: `IsIntersection` (slot 55) is true for an avenue piece whose piece id is in the 13-entry
  set `StaticInit` fills from `0x00AA7D40` (network-occupants.md §2.4, §2.5).

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO3 - the underground occupant's type:** does `cSC4UndergroundOccupant` ever carry a distinct occupant type?
  Its base ctor leaves `0x74758925`; the demolition dispatcher never names an underground type. [inferred:
  underground pieces are dispatched as `0xC772BF98` through their `cISC4NetworkOccupant` interface]~~
  **ANSWERED 2026-10-21 (re-ugbase):** DIFFERS — the underground constructor `0x0065C330` runs the network constructor
  first (`0x0065C354`), which writes `0xC772BF98` at `+0x28`; nothing later changes it (§3.2; quirk 1).

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO5 - the 0x4835C6A7-AC property family:** SDK names of the property-ID family `0x4835C6A7` (edge types),
  `0x4835C6A8` (passage ceiling), `0x4835C6AA` (one-way), `0x4835C6AB` (anchor), `0x4835C6AC` (retrofit) stored
  in the occupant's own property holder. [unknown]~~
  **ANSWERED 2026-10-21 (re-ugbase):** the SDK names the slots that read them: `0x4835C6A7`
  Get/SetConnectionNodeEdgeTypes (13, 14), `0x4835C6A8` Get/SetPassageCeiling (67, 68), `0x4835C6A9`
  Get/SetConnectionPathIDOverride (15, 16), `0x4835C6AA` Get/SetOneWayDirections (69, 70), `0x4835C6AB`
  Get/SetComplexPieceOrigin (71, 72), `0x4835C6AC` CheckForRetrofit / SetRetrofitFlag (6, 7) (§2.2 "Properties").

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO7 - GetSortValue's stored field:** which field holds the sort value of an underground occupant when flag
  `0x6000000` is set (§3.2); the negative displacement (`−0x178`/`−0x274`) in the decomp is ambiguous.
  [not specified]~~
  **ANSWERED 2026-10-21 (re-ugbase):** the piece id `+0xFC`: `GetSortValue` runs on the drawable at the virtual base
  `+0x274`, so `[this − 0x178]` is object `+0xFC`; without flags `0x06000000` it answers `0x4A10AFC2` (§3.1).

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO8 - the connection registration strings:** the strings `0x00B0B6D0`/`0x00B0C388` passed with the class IDs
  to helper `0x00654710` by the pipe/subway-connection `Init` (§3.6): GZCOM class names? [unknown]~~
  **ANSWERED 2026-10-21 (re-ugbase):** they are not strings but the classes' static model descriptors (`0x00B0B6D0`
  begins with the IDs `0x095A8D70`, `0xE95A8D8D`, …, pointers and counts), the second argument of the connection's
  `Init(classID, descriptor)`, main slot 34, which begins at `0x00654710` (§3.6).

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- ~~**NO9 - IsManagedGlobally for the underground family:** §2.7 gives slot 5 for the plain class (false) and the
  prebuilt/bridge/tunnel primaries (true) but not for `cSC4UndergroundOccupant`/pipe/subway (`0x00AAAD40`,
  `0x00AA97B8`, `0x00AAA3C8`). The core answers false (inherited plain body). Which body do those vtables hold?
  (Numbered NO9 per QUESTIONS.md; §12 also uses NO9/NO10 for two settled notes.) [not specified]~~
  **ANSWERED 2026-10-21 (re-ugbase):** MATCHES — slot 5 of `0x00AAAD40`, `0x00AA97B8` and `0x00AAA3C8` is the shared
  `0x00648EF0`, false (§2.7).

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- ~~**NO12 - the path-connection table shape and the wildcard count:** §2.6 addresses `[+0x144 + exit*4 + type]` and
  `[+0x144 + type]` (a 4 × 8 byte region that also contains the saved `+0x15C` u32), traffic-network.md §7.2 says
  `(pathType, entry, exit)` with a bit per path type. Which index is entry/exit, how many bytes per row, and with
  `0xFF` wildcards what is counted (matching entry/exit cells, or path-type bits too)? The core keeps
  `[entry][exit]` path-type bit bytes and counts (type, entry, exit) triples. [ambiguous]~~
  **ANSWERED 2026-10-21 (re-ugbase):** DIFFERS — 8 rows of 4 bytes: byte `+0x144 + type·4 + entry` holds a bit per
  exit side (`AddPathConnection` `0x00619790` ORs `1 << exit` there and into row 0, byte `+0x144 + entry`). With
  `0xFF` wildcards `PathConnectionExists` counts set exit bits: both wild over the row's 4 bytes, `entry` wild the
  bytes with bit `exit`, `exit` wild the bits of byte `entry` (§2.6).

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- ~~**NO14 - conformance test 6's slot:** §13 test 6 says "`SetRotation(2)` then slot 24 returns 2", but §2.4 maps
  slot 24 to `GetFlip` (slots 26/24/25 = orientation/flip/rotation). Is the test's slot 25 (`GetRotation`)? The
  core tests `GetRotation`. [ambiguous]~~
  **ANSWERED 2026-10-21 (re-ugbase):** slot 24 is `GetRotation` (SDK order; `+0x104 & 0x7F`), 25 `GetFlip`, 26
  `GetRotationAndFlip`; slot 27 is `SetVariation`, and the rotation is set by slot 30 or 74. Test 6 now uses
  `SetRotationAndFlip` (slot 74) and `GetRotation` (slot 24) (§2.4, §13).

<!-- from: core-netocc-records4 (engine lane, 2026-10-21) -->
- ~~**NO20 - `IsImaginaryConnection` with dir > 3:** the offset table `0x00B0CFE8` has four (dx, dz) pairs; what does
  a direction above 3 read (the bytes after the table, or a guard)? The core returns false. [missing]~~
  **ANSWERED 2026-10-21 (re-ugbase):** DIFFERS — there is no guard: the side indexes the table directly, so a side
  above 3 reads the bytes after it (quirk 19). The core's false is a choice for an input the game never passes (§3.2).

<!-- from: the network occupants spec (2026-10-20) -->
- ~~**NO2 - the power pole's delegate interface:** the identity of the delegate interface `0x26A68448` answered by
  the power pole via `+0xBC` (jump-table dispatch; likely a UI/drag helper). Probe: dump the `[+0xBC]` vtables on
  a live pole. [unknown]~~
  **ANSWERED 2026-10-21 (re-powerpoles):** `+0xBC` is the pole's model instance and `0x26A68448` is a model instance's
  mesh-container interface (model-instances.md): `QueryInterface` forwards that IID to the instance's own
  `QueryInterface` and fails when the pole has no instance (network-occupants.md §4.1).

<!-- from: core-netocc4 (engine lane, 2026-10-21) -->
- ~~**NO13 - the power line's field offsets and record:** §5.1's layout puts the cell at `+0x40/+0x44` and three
  poles at `+0x48/+0x4C/+0x50`; its slot table reads `GetCell` from `+0x44/+0x48` and `GetConnectedPoles` from
  `+0x4C/+0x50` plus byte `+0x54`; §5.2 writes u32 `+0x30…+0x40` then references from `+0x4C`, `+0x44`, `+0x48`.
  Which live field is each record word/reference, is the cell z saved, does `Read` check the version (only "v2"
  is stated), and does `SetPosition`'s float→int helper truncate or round? The core keeps the record as a literal
  field list and truncates. [ambiguous]~~
  **ANSWERED 2026-10-21 (re-powerpoles):** with object offsets: position `+0x38` (3 × f32), cell `+0x44`/`+0x48`, the
  two poles `+0x4C`/`+0x50`, the bool `+0x54`, preview `+0x55` (the earlier table was 4 low). The record
  (`0x0064A310`) is u16 2, the occupant record, 3 × f32, 2 × s32, a u8 and two refs (network-occupants.md §5.1, §5.2).
  **ANSWERED 2026-10-25 (re-netocc-no33):** MATCHES — the corrected live layout above is confirmed;
  §5.1/test 74 now specify Shutdown (`0x00649270–0x00649289`): occupant this=object+4, virtual main
  slot 4 on object with `(null,null,false)`, then base `0x005E8980` with the original occupant pointer.
  Each held pole is released, including two refs to the same pole; position/cell/preview survive.
  Base Shutdown clears object+32 bit 0 and posts `0x99EF1141`/0 only if initialized, and always
  returns true. Equal-pointer SetConnectedPoles still AddRefs before releasing each old field.

<!-- from: core-netocc-close5 round 2 (2026-10-23) -->
- ~~**NO36 - sections 6.1-6.2 pylon owner algorithms:** please publish constructor defaults beyond the
  named matrices; SetUIState, SetBitmapInfo, dispatch type/state, SetSourceOccupant and SetPylonOwner
  side effects; bounds/transform/position/pick; DoAnimationTick, SetVisibility, DoMessage, Draw and sort;
  BuildMySimTexture, RebuildCurrentTexture, BuildCallNumberTexture and RebuildModel algorithms. Also
  supply both PackRenderState descriptions and the renderer slot-0x28 result interpretation. The spec
  names these members without behavior. The core implements only explicit lifecycle subscriptions,
  call/drag storage and the record, with no host callbacks standing in for these owner methods.
  [missing; sections 6.1-6.2 remain open]~~
  **Progress 2026-10-23 (re-netocc-answers5):** §6.2.1 and tests 40–42 now specify changed/equal enum
  setters, unconditional bitmap dirtying, source/owner ref ordering, the shared u32 blink timer,
  position/reinsertion and visibility. Existing §6.2/test 20 already supplies both packing tuples and
  the discarded renderer +28 result. Texture/model composition, Draw/Pick, transform/bounds/sort and
  message decisions remain open; these partial rules do not close NO36.
  **Progress 2026-10-23 (re-netocc-poles5):** §6.2.2/tests 46–48 now settle cached local/world
  bounds, transform counter/mask copies, texture-ID sorting, pick eligibility/height/strict distance,
  ordinary texture resource tables and stale-ID behavior on replacement failure. Full model construction,
  MySim/label composition, Draw and message owner algorithms remain open. NO44 is answered separately.
  **Progress 2026-10-23 (re-netocc-answers5d):** §6.2.3/tests 52–53 settle Draw's pass-specific dirty
  consumption, main/label submissions, dispatch-state label gate and all message decisions. Model and
  MySim/number-label composition remain open; no full owner closure is claimed.
  **ANSWERED 2026-10-24 (re-netocc-answers5d):** DIFFERS — §§6.1–6.2.4/tests 56–59 complete
  the owner contract with MySim copying, number-label rasterization, billboard geometry, transform
  rounding and two-endpoint bounds. +150 is the call-number texture; MySim uploads to main +148.
  Rebuild clears dirty even after a resource/placement failure; texture failure is not a geometry gate.
