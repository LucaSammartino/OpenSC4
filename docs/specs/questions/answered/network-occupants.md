# network-occupants: answered questions (archive)
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
