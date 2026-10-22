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

When enabled, DetectShores resets the accumulated edit area, snapshots the unsigned 32-bit terrain edit counter
(slot 54), and refreshes terrain flag `0x10` through `0x007428E0` (the existing terrain shoreline kernel).
It scans **all vertices**, z ascending outside x ascending, using stride cellsX + 1. A vertex with `0x10` set
and `0x01` clear starts a trace. The length comparison is unsigned and strict: retain only length > minimum,
not ≥ minimum (`0x00761B86–0x00761C9E`). Rejected short traces still consume visit flags during this scan.

The ordered trace is defined by `0x0074B570`, forward walk `0x0074AD20`, and backward walk `0x0074AB60`.
Its direction table is executable data at `0x00AB3B80` (x) and `0x00AB3BA4` (z):

| Direction | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| dx | −1 | −1 | 0 | 1 | 1 | 1 | 0 | −1 |
| dz | 0 | −1 | −1 | −1 | 0 | 1 | 1 | 1 |

Clear the temporary deque, append the seed index once, and OR visit mask 1 into its 16-bit flags. First walk
forward from the seed, appending each accepted index; then walk backward **from the original seed**, prepending
each accepted index. Both walks share the mutated flags. Each walk initially searches for an in-bounds neighbour
whose binary32 height is strictly below waterY: forward searches directions 0..7, backward 7..0. If none exists,
that walk ends immediately. This initial underwater direction is the excluded direction for the first scan.

Forward scans the seven other directions starting at excluded + 1 modulo 8, increasing modulo 8; backward starts
at excluded − 1 modulo 8 and decreases modulo 8. Out-of-bounds neighbours are skipped, using unsigned x/z bounds
checks (negative coordinates wrap and fail). At each in-bounds neighbour:

- If flag `0x10` is set and visit mask 1 is clear, OR the mask into its flags, append/prepend its index, make it
  the current vertex, and restart with excluded direction `(acceptedDirection − 4) & 7`.
- Otherwise, height strictly above waterY terminates this walk immediately. Equal-height vertices do not stop
  the directional scan. The shore/visit test precedes the height test, so an unvisited shore candidate can be
  accepted even when its height is above waterY.
- Exhausting the seven directions without acceptance ends the walk. Visited shore vertices are never appended
  again, including the seed: there is no explicit closed-loop endpoint duplication or closure flag.

This is an eight-neighbour directional trace with a height barrier, not enumeration of a connected component.
Initial direction and barriers use height, not a guessed water flag. Instruction proof: initial comparison
`0x0074AD7C–0x0074AD87`, shore/visit/barrier order `0x0074ADF3–0x0074AE1E`, reverse order
`0x0074AC34–0x0074AC51`, append/prepend and direction reset `0x0074AE35–0x0074AEF5`,
`0x0074AC6A–0x0074AD1C`. The visits are temporary: after **all seeds**, `0x00740980(1)` clears mask 1 from
every terrain vertex, rather than restoring a saved per-vertex flag word. Preexisting bit-1 values are therefore
also cleared. Other bits remain as produced by the flag refresh and tracing.

AddShore copies deque indices in their final order to binary32 control points `(16 × (index % stride),
16 × (index / stride))`, unsigned division/remainder then conversion; no simplification, endpoint extension or
height is stored (`0x00761A00–0x00761B6A`). DetectShores itself appends shores; callers clear the old shore
collection when a full redetection is intended (§1.5).

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

**Frame sampling (WR6).** Strip preparation samples a frame in `[0, frameCount)` first, then a period in `[L,H)`,
and initializes its unsigned 32-bit clock to zero (`0x0075FEF2–0x0075FF3B`). There is one shared native
cRZRandom object at `0x00B4D530`; calls consume it in shore order, strip list order. This is multiply-high range
scaling, not modulo, floating random interpolation, inclusive upper-bound sampling or rejection sampling:

- For nonzero bound B, obtain raw R and return floor(R × B / 2³²), using unsigned 32×32 multiplication. B = 0
  returns zero without consuming a random value (`0x009136B1–0x009136C9`).
- Signed range helper returns L + bounded(H − L), with 32-bit subtraction/addition (`0x009136CA–0x009136DE`).
- Raw step: form the unsigned 64-bit value Q = state × `0x41C64E6D` + `0x3039`; store low32(Q) as the next
  state; return low32(Q >> 16). The high multiplication bits participate in the returned value; this is not
  merely nextState >> 16 (`0x00913696–0x009136B0`).
- L and H come from unsigned base period P converted with x87 integer load plus 2³² correction when its signed
  interpretation is negative. Multiply by binary32 0.7 (`0x3F333333`, `0x00A98F80`) and binary32 1.3
  (`0x3FA66666`, `0x00AB45F4`), respectively. Each product remains extended precision for the native floor
  conversion: subtract binary32 0.5, add 12582912, spill binary32, subtract bit pattern `0x4B400000`; increment
  the resulting integer only if the original product equals that integer + 1. Within the intended conversion
  domain this is floor, including the representational error in the multipliers (`0x007601D3–0x0076026B`).
  For P = 175000 it gives **L = 122499, H = 227499**, so periods range 122499..227498. The earlier
  decimal-only 122500..227500 description is not the native arithmetic. No additional clamping is applied.

**Tick.** Before the first Update has built the collection, DoAnimationTick does nothing. Afterwards, scheduling
is processed first (§below), then each retained strip once in collection/list order. Add the raw elapsed value
to its clock modulo 2³². If the resulting unsigned clock is **≥ period**, increment frame once, sample one new
period, reset clock to zero (discard all overshoot), and reset frame to zero if it is ≥ frameCount. It never
loops to catch up. Equality advances; wrapping the clock can instead prevent advancement. Strips created by a
rebuild during this tick receive that same tick's full elapsed value (`0x00761E74–0x00761F5F`).

**Terrain edits and scheduling (WR10).** Message `0x278E6003` supplies packed unsigned 16-bit bounds: high/low
halves of message slot 9 are xmin/xmax; high/low halves of slot 14 are zmin/zmax. These are terrain cell-grid
coordinates for the cached contact rectangles, not metres. Area is signed low32 of `(xmax−xmin)(zmax−zmin)`,
without +1. If area ≤ 0, return. Otherwise add it modulo 2³² to an unsigned accumulated area. Values ≤ the
tuning threshold (12) return immediately; **no dirty-rectangle union occurs on this branch**. Only when the sum
is > threshold does it reset to zero and test/remove intersecting strips (`0x00760E21–0x00760E8A`).

Contact uses inclusive signed rectangle overlap: first shore cached cell bounds, then strip cached cell bounds,
then any cached subrectangle of that strip. Edge/corner contact passes. It is an AABB/subrectangle test, not
segment/polygon intersection. Remove **all** matching strips, freeing their vertices and subrectangle storage;
other strips remain. The cached-bound construction is a separate draw-preparation obligation (WR7).
Proof: `0x0075FB40–0x0075FBA9`, `0x007607F0–0x007608C7`.

If any strip was removed, arm the rebuild and reset its clock to zero **even when already armed**. Otherwise,
if already armed, return without union or clock reset. Otherwise union this edit rectangle into the dirty
rectangle using componentwise signed min/max; arm/reset only when its signed width×height is > 400.
The dirty rectangle starts `(INT_MAX,INT_MAX,INT_MIN,INT_MIN)` in the constructor. Neither DetectShores nor a
completed rebuild resets it; the accumulated area and edit-counter snapshot, by contrast, reset during enabled
DetectShores. A sequence of distant edits can therefore retain old dirty extents across rebuilds
(`0x00760E8C–0x00760EC4`, union `0x004665C0`, constructor `0x007617EA–0x00761806`).

When a tick starts unarmed, arm/reset if current unsigned edit counter is > low32(snapshot + floor(low32(minimum²)
/ 10)). This is comparison against a wrapped sum, **not a wrapped difference**. The arming tick contributes no
elapsed time to the rebuild clock. When a tick starts armed, add elapsed modulo 2³² and rebuild only at
**strictly > 2000000**, not equality. Delayed rebuild clears all shores/strips, redetects, then recreates; the
rebuild happens before frame animation on that tick (`0x00761E87–0x00761EE9`).

Message `0x89171496` clears the shore collection, redetects and recreates immediately (`0x00761FC1–0x00761FE8`).
It does not itself clear a pending rebuild; CreateRipples sees that pending flag, clears/redetects again and
clears it. A zoom mismatch in Update calls CreateRipples: normally it reuses cached shores and rebuilds only
strips; if pending, CreateRipples first clears/redetects shores. Zoom comparison/recreation occurs only inside
the nonempty-shore and water-effects-enabled draw branch (`0x00761CF7–0x00761D31`, `0x00762004–0x00762094`).

## 2. Selection highlights (`cSTESelectedTerrainArea`) *[compat]*

### 2.1 State

Two tables keyed by vertex index (z × (cells x + 1) + x): **plain** cells (a 32-bit colour) and **textured** cells (a
texture key, two bytes, a byte, an RGB colour, an alpha, a mode). Putting a cell in one table removes it from the
other. `Init` (`0x00750AA0`) takes the terrain, its cell counts and the texture binding factory, and builds the render
states (§2.3).

### 2.2 Marking (`MarkSelected`, five forms)

When the last argument is nonzero, **each of the five forms** clears both tables first and refreshes the shared
render states if `kPreferOpaque` changed. With clear false none refreshes that preference, including the rectangle
form (`0x007516D0`, `0x00751780`, `0x00751910`, `0x007519A0`, `0x00751A70`, first branches).

| VA | Arguments | Effect |
|---|---|---|
| `0x007516D0` | (cell list, colour, clear) | the cells become plain cells of that colour |
| `0x00751780` | (rectangle, colour, clear) | every vertex of the inclusive rectangle becomes a plain cell of that colour |
| `0x00751910` | (cell, colour, clear) | one plain cell |
| `0x007519A0` | (cell, key, b1, b2, palette index, b3, clear) | a textured cell coloured from the palette `0x00AB4238` (0 red, 1 green, 2 blue, 3 (0.4, 0.3, 0), 4 black), alpha 1, mode 2 |
| `0x00751A70` | (cell, key, b1, b2, packed colour `0xRRGGBBxx`, mode, clear) | a textured cell of that colour; mode 3 with `kNoAddSigned` becomes mode 1 with alpha 0.5 |

The palette index is not range-checked: indices above 4 read unrelated constants *[quirk]*. `ClearCurrentSelections`
(`0x00750CA0`) empties both tables.

**Data widths and unchecked inputs (WR9).** Coordinates and keys/colours/mode are 32-bit; b1–b3 are bytes;
RGB and alpha fields are binary32. Packed colour `0xRRGGBBxx` copies **xx as b3**. R/G/B are each multiplied
by binary32 `0x3B808081` (1/255 at `0x00A90BBC`) and stored as binary32. Alpha is exactly 1 in every case
except mode 3 with kNoAddSigned, which stores mode 1 and alpha 0.5. The low colour byte is not alpha
(`0x00751AD3–0x00751B91`). Palette marking copies only three RGB floats from 16-byte-stride entries, then
unconditionally stores alpha 1 and mode 2; it ignores the fourth palette float (`0x00751A17–0x00751A62`).

There is no terrain bounds validation or rectangle normalization. Cell key is low32(z × (cellsX + 1) + x);
out-of-range/negative coordinates can alias valid keys. An empty list still performs requested clear/preference
refresh, then succeeds. Rectangles iterate signed inclusive z outside signed inclusive x; reversed bounds insert
no entries (but still clear/refresh if requested). Valid portions of out-of-range rectangles are not selected
separately: all coordinates in the given loop are inserted. Marking returns boolean true; there is no atomic
invalid-input rejection (`0x007516D0–0x00751771`, `0x007517D0–0x0075190D`, `0x00751958–0x00751992`,
`0x00751653–0x007516A9`). Signed endpoint overflow is unsanitized; callers must avoid INT_MAX inclusive loops.

Palette indexing shifts the full index left four bits modulo 2³² and reads RGB at low32(`0x00AB4238 + offset`).
It neither clamps nor reports an error. Adjacent shipped-image RGB words are:

| Index | RGB binary32 words | Values |
|---|---|---|
| 5 | `3D800000 47400000 38D1B717` | 0.0625, 49152, approximately 0.0001 |
| 6 | `3F800000 3F800000 00000000` | 1, 1, 0 |
| 7 | `00000000 3F800000 3F800000` | 0, 1, 1 |

These are unrelated data, not extra palette entries. Arbitrary overflow is an unchecked native memory read;
addresses outside mapped memory can fault and are not a defined portable colour domain. The shift also makes
index `0x10000000` alias index 0 (`0x00751A17–0x00751A2E`; image data `0x00AB4288–0x00AB42B0`).

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

See [questions/water-ripples.md](questions/water-ripples.md). WR4–WR6 and WR9–WR10 have native contracts here;
WR7–WR8 retain explicit draw/lifecycle obligations. WR1–WR3 remain separate semantic questions; in particular,
bit 1 is a temporary trace visit mask here, not evidence that it means water.

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
12. **WR4 trace:** with only the seed flagged shore, the result is exactly one index, even if all eight
    neighbours are underwater; it is rejected for minimum ≥ 2. Visit mask 1 is cleared globally after detection.
    With all heights equal to waterY neither walk starts, regardless of neighbouring shore flags.
13. **WR6 overshoot:** clock 0, period 150000, frame 2 of 8, elapsed 900000 advances to frame 3, clock 0,
    and consumes one period draw. It does not advance six frames. P = 175000 samples `[122499,227499)`.
14. **WR9 alias:** stride 9, cells (9,0) and (0,1) share key 9. Packed `0x123456AB`, mode 2 stores
    b3 = 0xAB and alpha 1. A reversed rectangle with clear true empties old selections and succeeds.
15. **WR10 thresholds:** accumulated edit area 0 plus area 12 returns without union. A subsequent area 1
    resets the accumulated area and performs contact. Armed clock exactly 2000000 does not rebuild; one more
    unit does. Snapshot `0xFFFFFFF0` and minimum 50 give wrapped threshold 234: current counter 100 does not arm.
