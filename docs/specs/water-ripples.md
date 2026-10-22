# Spec: shore ripples and terrain selection highlights (`cSTEWaterRipple`, `cSTESelectedTerrainArea`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [terrain.md](terrain.md) (§15, the terrain view's sub-objects: the selection at `+0x6C`, the ripples
at `+0x70`; the vertex flags), [texture-bindings.md](texture-bindings.md) (the ripple texture),
[render-properties.md](render-properties.md) (`kRenderWaterEffects`, `kPreferOpaque`, `kNoAddSigned`),
[INVENTORY-M6.md](INVENTORY-M6.md) (R5).

## 0. Summary

Both objects belong to the terrain's 3D view. The **ripples** are animated foam strips laid along every shore: the
shores are traced from the terrain's vertex flags, smoothed with a cubic B-spline and turned into triangle strips
from just inland to a wobbling distance out over the water, textured with an animated FSH whose frame advances at
random intervals. The **selection** highlights terrain vertices for tools: plain coloured cells and textured cells,
drawn over the terrain with a depth offset.

## 1. Ripples (`cSTEWaterRipple`) *[compat]*

### 1.1 Tuning (`ReadTuningParameters`, `0x00760060`)

Read once (the first `Init`) from the exemplar `0x6534284A / 0x88CD66E9 / 2` ("Ripple Properties", `SimCity_1.dat`).
"Per zoom" values are 5 entries, zoom 0–4.

| Property | Shipped | Use |
|---|---|---|
| `0x491C9C00` | true | ripples on |
| `0x491C9C01` | 50 | minimum shore length in points (at least 2); also the terrain-edit count that triggers a rebuild (§1.5) |
| `0x491C9C02` | 16 ×5 | per zoom: spacing between ripple points (its square is compared); the spline step is it ÷ 64 |
| `0x491C9C03` | 0.3 | the ripple height above the water level |
| `0x491C9C04` | 0.15, 0.3, 0.3, 0.6, 0.75 | per zoom: texture advance per segment |
| `0x491C9C05` | 45, 40, 35, 30, 26 | per zoom: the outer distance (m) |
| `0x491C9C06` | 1 ×5 | per zoom: the inner distance (m, inland) |
| `0x491C9C07` | 175000 | frame period; frames last a random 0.7–1.3 of it (122500–227500) *[inferred: µs]* |
| `0x491C9C08`, `0x491C9C09` | 100, 150, 200, 255, 255 | per zoom: the inner and outer alpha |
| `0x491C9C0A` | 0.2 ×5 | per zoom: the wobble as a fraction of the outer distance |
| `0x491C9C0B` | 60 ×5 | per zoom: the largest turn between segments (degrees) |
| `0x491C9C0C` | 12 | terrain-edit area that starts a check (§1.5) |
| `0x491C9C0D` | 6 | the number of point pairs of the fade at each end |

### 1.2 Shores (`DetectShores`, `0x00761B70`)

Every terrain vertex with flag `0x10` set and flag `0x01` clear starts a shore trace (terrain helpers `0x0074B570`);
a shore with more points than `0x491C9C01` is kept (`0x00761A00`). The terrain's edit counter (terrain slot 54) is
remembered.

### 1.3 Strips (`CreateRipples` `0x00761CF0`, `CreateRipplesForShore` `0x00761140`)

For each shore, the parameter walks the shore's points in steps of `0x491C9C02` ÷ 64 (zoom-dependent). At each step
(`ComputeSplinePoints`, `0x0075FC40`) the uniform cubic B-spline of four consecutive shore points gives a point P
and a unit tangent; with N the tangent turned a quarter, the **outer** vertex is P + (D + w sin φ) N and the
**inner** vertex P − d N, with D = `0x491C9C05`, w = D × `0x491C9C0A`, d = `0x491C9C06` for the zoom, and φ
growing by 0.1 per step. N = (tangent z, −tangent x) *[inferred: it points over the water for the traced shore
direction]*.

A strip (`StartNewRipple`, `0x00760EE0`) is an ordered vertex stream, with separate outer and inner acceptance.
The following is the WR5 native contract, including asymmetric partial-pair behavior.

**Parameter and endpoint domain.** For n stored shore points, start span index i = 0 and binary32 local parameter
t = 0; sample while i < n − 3, using points i through i + 3. There is no endpoint padding, duplication or wrapping
in this owner, even for a closed trace. After every candidate, including a rejected/splitting candidate, add the
binary32 zoom step spacing/64 to t. If t ≥ 1, remove its integral floor and advance i by that amount. The native
floor conversion uses the binary32 `(t − 0.5) + 12582912` bit-conversion with exact-integer correction
(`0x007614A8–0x00761529`), rather than a cast truncating the whole shore parameter on every step.

**Spline and phase.** This is the four-point uniform cubic B-spline, with weights
`(1−3t+3t²−t³, 4−6t²+3t³, 1+3t+3t²−3t³, t³)/6`. Differentiate those weights and normalize the resulting
2D derivative; there is no zero-length fallback. The constants 1/6 and 1/2, intermediates spilled to binary32,
and x87 evaluation order belong to `0x0075FC40–0x0075FE65`; an algebraically rearranged polynomial need not be
bit-identical. Phase is a binary32 owner field, reset to zero once per `CreateRipples`, before its shore loop,
not per shore or strip (`0x00761E1A`, `0x00761E48`). Each spline evaluation uses sin of the old phase, then
adds binary32 `0.1` (`0x3DCCCCCD` at `0x00A8C950`; `0x0075FDF8–0x0075FE1C`). Rejected samples consume phase too.

**Vertices.** The first vertex is outer, the second inner. Outer position is `(P.x + rN.x, waterY, P.z + rN.z)`,
where r = D + (D × wobble)sin(phase); inner is `(P.x − dN.x, waterY + height, P.z − dN.z)`.
Thus the configured height raises only the inner side (`0x007603F3–0x00760420`, `0x0075FE22–0x0075FE5C`).
Both have white RGB and their zoom alpha. Texture coordinates are `(u,1)` outer, `(u,0)` inner. Starting a strip
zeros both u values; its first pair uses u = 0. Each subsequently completed pair uses the current u and then
advances both u fields by the zoom texture advance. In particular, the second complete pair also uses u = 0
(`0x0076126F–0x00761282`, `0x0076144C–0x00761481`).

**Candidate processing, in order** (`0x007611A3–0x007614A8`):

1. Test inner position, then outer position, against the city inset. For each coordinate the accepted interval is
   `[binary32(0.2), binary32(cityExtent − 0.2))`: equality at the lower edge is accepted, equality at the upper
   edge rejected. An outside candidate calls StartNewRipple immediately, arms a new start, and is consumed.
2. If a new start is armed, call StartNewRipple, reset u, append the current outer then inner, save both positions,
   set the completed-followup counter to zero, and arm the outer-side test. Consume this candidate.
3. If the outer-side test is armed, compare squared outer displacement from the last accepted outer with squared
   spacing. Strictly less consumes the candidate. Otherwise normalize that displacement. From the second followup
   onward, compare acos of its dot with the previous accepted outer direction to the zoom angle in radians;
   equality passes. A failed turn arms a new start and consumes the candidate, without calling StartNewRipple yet.
   Passing appends the outer vertex immediately, saves its position/direction and disarms the outer test.
4. Test inner displacement and turn by the corresponding inner history and same comparisons. A spacing rejection
   leaves the accepted outer pending: the next sample bypasses the outer test and tries only its inner vertex.
   It can therefore complete a pair whose two sides came from different spline parameters. An inner turn failure
   arms a new start and consumes the candidate; it does not undo the already appended outer vertex.
5. An accepted inner appends the vertex, increments the followup counter, advances both u fields, and rearms the
   outer test. Only then test the counter: values through 1000 continue; value 1001 arms a new start. Including
   the initial pair, the maximum before this split is **1002 pairs**, not 1000 (`0x0076145A–0x00761488`).

Neither turn nor size splitting retries the candidate. Saved direction vectors initially contain zero; the counter
suppresses turn comparisons for the first followup. StartNewRipple reuses/clears a current strip with fewer than
8 vertices, otherwise appends a new strip; after traversal the final strip is removed if it has fewer than 8
vertices. It does not close a strip back onto its first pair (`0x00760F54–0x00760F87`, `0x0076152F–0x00761569`).

**Display preparation and alpha.** Each retained strip gets its random frame/period and clock zero (§1.5), then
loses one trailing vertex if its count is odd. Let V be that even count and R = min(floor(V/4), configured fade
pair count). For each side with full alpha A, the first ramp value is low-byte(trunc(A × binary32(0.1))) and
the increment is max(1, low-byte(trunc(A × binary32(0.9) / configured fade pair count))). The denominator is the
configured count, **not R**. For R pairs from each end, write that start value first, then add the byte increment
with byte wrap, then clamp the resulting byte down to A if greater. The tail ramp is visited backwards with
inner written before outer; middle pairs retain A. There is no zero-alpha endpoint or normalization to reach A
within a shortened ramp (`0x00761D36–0x00761E09`, `0x0075FF27–0x00760055`; 0.9 is `0x3F666666` at
`0x00B0D7D8`). Nonpositive/invalid tuning is not sanitized by this preparation.

The texture is the FSH (`0x7AB50E44`, `0x891B0E1A`, `0xE9A44A88`) bound through the texture binding factory (slot
6); without it no strip is kept.

### 1.4 Drawing (`Update`, `0x00761FF0`; `DrawRipple`, `0x0075FE70`)

The first `Update` detects the shores and makes the strips. Strips are drawn only when the draw context's render
properties have `kRenderWaterEffects` (boolean 3) set, with an alpha-blended render state; a zoom change
rebuilds the strips for the new zoom. Each shore and each strip is tested against the view frustum; a strip is drawn
as one triangle strip with its current frame's texture.

### 1.5 Updates (`DoMessage`, `0x00761F80`)

- `0x86AD10EE` (animation tick with the elapsed time; `DoAnimationTick` `0x00761E70`): each strip's clock advances;
  past its period the frame advances (wrapping at the texture's frame count) and a new random period is drawn. When
  no rebuild is pending and the terrain's edit counter has grown by more than 50² / 10 = 250, a rebuild is armed; an
  armed rebuild happens after 2,000,000 time units *[inferred: 2 s]*.
- `0x278E6003` terrain changed (`DoOnTerrainEdit` `0x00760E10`): the changed area is added up; past
  `0x491C9C0C` (12) the sum restarts and the rectangle is checked: strips it touches are removed and a rebuild is
  armed; otherwise the rectangle is merged into a dirty rectangle and a rebuild is armed once that covers more than 400
  cells.
- `0x89171496`: rebuild at once.

## 2. Selection highlights (`cSTESelectedTerrainArea`) *[compat]*

### 2.1 State

Two tables keyed by vertex index (z × (cells x + 1) + x): **plain** cells (a 32-bit colour) and **textured** cells (a
texture key, two bytes, a byte, an RGB colour, an alpha, a mode). Putting a cell in one table removes it from the
other. `Init` (`0x00750AA0`) takes the terrain, its cell counts and the texture binding factory, and builds the render
states (§2.3).

### 2.2 Marking (`MarkSelected`, five forms)

The last argument of each form clears both tables first (the rectangle form then also rebuilds the render states if
`kPreferOpaque` changed).

| VA | Arguments | Effect |
|---|---|---|
| `0x007516D0` | (cell list, colour, clear) | the cells become plain cells of that colour |
| `0x00751780` | (rectangle, colour, clear) | every vertex of the inclusive rectangle becomes a plain cell of that colour |
| `0x00751910` | (cell, colour, clear) | one plain cell |
| `0x007519A0` | (cell, key, b1, b2, palette index, b3, clear) | a textured cell coloured from the palette `0x00AB4238` (0 red, 1 green, 2 blue, 3 (0.4, 0.3, 0), 4 black), alpha 1, mode 2 |
| `0x00751A70` | (cell, key, b1, b2, packed colour `0xRRGGBBxx`, mode, clear) | a textured cell of that colour; mode 3 with `kNoAddSigned` becomes mode 1 with alpha 0.5 |

The palette index is not range-checked: indices above 4 read unrelated constants *[quirk]*. `ClearCurrentSelections`
(`0x00750CA0`) empties both tables.

### 2.3 Drawing (`Update`, `0x007514F0`)

When either table has cells: depth offset 4, the plain cells (`DrawUntexturedSelections` `0x00750FC0`), then the
textured ones (`DrawTexturedSelections` `0x00750CF0`), then the previous depth offset. The four render states
(`InitRenderState` `0x00750600`) are opaque or blended variants chosen by `kPreferOpaque` (boolean 33), rebuilt when
that property changes.

## 3. Users

The terrain's 3D view creates both (terrain.md §15); the tools mark selections (zone, lot and terrain tools); the
animation tick drives the ripples.

## 4. Quirks

1. The palette index of §2.2 is not checked.
2. `0x491C9C01` serves both as the minimum shore length and as the base of the edit threshold (§1.5).
3. A failed texture leaves the shores with no strips until the next rebuild (§1.3).

## 5. Open questions

See [questions/water-ripples.md](questions/water-ripples.md) WR1–WR3: the time unit of the tick and the periods, the
terrain flag meanings `0x10` / `0x01` (shore and water *[inferred]*), and the bytes b1–b3 of a textured selection.

## 6. Conformance tests

1. **Spline:** with points (0, 0), (16, 0), (32, 0), (48, 0) and t = 0, P = (16, 0) and the tangent is +x.
2. **Strip width:** at zoom 4, φ = 0: the outer vertex is 26 m out, the inner 1 m in.
3. **Spacing:** two candidate outer points 10 m apart at zoom 0 (spacing 16): the second is not kept.
4. **Turn:** a 70° turn between segments ends the strip at zoom 2.
5. **Short shore:** a shore of 40 points gets no ripples.
6. **Frames:** a strip whose period is 150000 advances one frame after ticks totalling 150000.
7. **Selection:** marking a cell plain after textured leaves it only in the plain table.
8. **Palette:** index 3 gives (0.4, 0.3, 0) with alpha 1 and mode 2.
9. **WR5 partial pair:** with a strip already started, outer spacing passing and inner spacing failing, retain
   the outer. A later inner success appends only the later inner, even if that later outer would pass spacing.
10. **WR5 UV/size:** an initial pair and first followup both use u = 0; the next pair uses one texture advance.
    A 1001st followup is appended before splitting, giving 1002 pairs including the initial pair.
11. **WR5 fade:** A = 100, configured fade = 6 and V = 8 give R = 2, start = 10, increment = 14 (binary32 0.9
    is below 0.9). Pair alpha is `[10,24,24,10]`, not `[0,100,100,0]` or a fade divided by R.
