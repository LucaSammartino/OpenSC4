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

Read on the first eligible process-wide `Init`, and again after the process reset callback (§§2.4,2.6), from
exemplar `0x6534284A / 0x88CD66E9 / 2` ("Ripple Properties", `SimCity_1.dat`). Shipped "per zoom" values
have 5 entries, zoom 0–4; the native reader does not validate that count. "Shipped" is not an absent-property default.

| Property | Shipped | Use |
|---|---|---|
| `0x491C9C00` | true | ripples on |
| `0x491C9C01` | 50 | minimum shore length in points (at least 2); also the terrain-edit count that triggers a rebuild (§1.5) |
| `0x491C9C02` | 16 ×5 | per zoom: spacing between ripple points (its square is compared); the spline step is it ÷ 64 |
| `0x491C9C03` | 0.3 | the ripple height above the water level |
| `0x491C9C04` | 0.15, 0.3, 0.3, 0.6, 0.75 | per zoom: texture advance per segment |
| `0x491C9C05` | 45, 40, 35, 30, 26 | per zoom: the outer distance (m) |
| `0x491C9C06` | 1 ×5 | per zoom: the inner distance (m, inland) |
| `0x491C9C07` | 175000 | base period in microseconds; exact native sampled range 122499..227498 (§1.5) |
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
16 × (index / stride))`, **signed 32-bit** division/remainder then conversion (`IDIV` at `0x00761ADC`);
valid terrain indices are nonnegative. No simplification, endpoint extension or
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

The texture is the FSH (`0x7AB50E44`, `0x891B0E1A`, `0xE9A44A88`) requested through texture binding factory slot
6. The old statement that failure safely leaves no strips is withdrawn: exact acquisition-failure control flow
is not a safe skip. StartNewRipple zeroes the cached binding and caller's current-strip pointer on false
acquisition (`0x00760F43–0x00760F4D`), releases its temporary factory reference, then follows the null-strip
path to append a new empty strip (`0x00760F58–0x00760FA2` onward). It does not return at acquisition failure.
Subsequent requests retry because the cache remains null. If no request succeeds, display preparation reads
frame count at cached-binding+4 without a null guard (`0x0075FEE8–0x0075FEF8`); this path has no defined
successful geometry result. If an earlier current strip existed, nulling the caller pointer does not remove
that strip. An unavailable factory also has no graceful branch: slot 6 is called through null. Successful
acquisition is cached until Shutdown, with no extra AddRef of the returned binding.

### 1.4 Drawing (`Update`, `0x00761FF0`; `DrawRipple`, `0x0075FE70`)

The first `Update` detects the shores and makes the strips. Strips are drawn only when the draw context's render
properties have `kRenderWaterEffects` (boolean 3) set, with an alpha-blended render state; a zoom change
rebuilds the strips for the new zoom. Each shore and each strip is tested against the view frustum; a strip is drawn
as one triangle strip with its current frame's texture.

**Proved WR7 draw preparation.** After creating all strips, `0x00761010` calls `0x007609B0` for each strip.
Partition its vertex stream into consecutive groups of at most **20 vertices**, without overlap. Each group
gets a cell rectangle `(floor(minX/16), floor(minZ/16), ceil(maxX/16), ceil(maxZ/16))`; the strip cell box is
the union of those rectangles. Native floor/ceil use the binary32 magic-constant conversion with exact-integer
correction (§1.5), not truncation (`0x00760C70–0x00760D62`; scale `0x3D800000` at `0x00B0D7CC`).
Thus a segment between the last vertex of one group and the first vertex of the next has no separate contact
rectangle. Shore cell bounds union all its strip bounds. Empty shores use all-INT_MAX cell bounds.

Strip and shore render boxes multiply those integer x/z bounds by binary32 16 and store binary32 coordinates;
both minY and maxY equal current terrain waterY. They do **not** enclose the raised inner-side height.
Instruction stores `0x00760DBC–0x00760DFB` and `0x007610E4–0x00761122` disambiguate misleading integer casts
in decompilation. Update frustum-tests a nonempty shore box first; DrawRipple tests its strip box and requires
a nonnull binding. A passing strip binds frame[frameIndex], texture unit 0, then submits primitive token 7,
vertex-format token 10, the whole vertex count and vertex stream (`0x0075FE70–0x0075FEDE`).

The ripple packed-state arguments, in native PackRenderState parameter order, are
`(1,0,1,0,1,1,1,3,4,5,FFFFFFFF,BF800000,1,FFFFFFFF)`; the last two hexadecimal words here and in §2.3 are
raw 32-bit arguments (`BF800000` is binary32 −1). State is packed lazily once process-wide. Update sets this
state and the default model transform; it does not restore the prior state/texture afterwards (`0x00762027–0x0076207A`).

### 1.5 Updates (`DoMessage`, `0x00761F80`)

**Process RNG initialization (WR12).** The shared object is constructed during executable static initialization,
not by a ripple constructor, Init, tuning reset or city load. Initializer `0x00A751D0` (table entry
`0x00B07680`) obtains the current `cRZTime` 32-bit value T and supplies `T XOR 0x8C09C57A` to the
random constructor `0x00913A5C`. T is the host's current date/time conversion, not simulation time or an
animation timer: default time construction uses host localtime and the date/hour/minute/second constructor
(`0x00470B50–0x00470BA7`, `0x00470010`). This does not promise identical startup seeds across runs.
Seed `0x00913667` copies an ordinary seed to both current-state and remembered-seed words; the constructor
also zeroes its third word. There is one sentinel: if the supplied seed is `0xFFFFFFFF` (T = `0x73F63A85`),
the effective seed is host time XOR `(clock OR low32(clock << 16))`, sampled by that helper instead.
Zero is an ordinary seed. No ripple lifecycle or tuning reset reseeds this object. A deterministic fixture
must supply T (and sentinel host samples if applicable), or capture the effective primitive state once;
it must not substitute a constant seed or create a generator per owner.

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

**Time units (WR1).** The elapsed word in animation message `0x86AD10EE` is in microseconds. The manager
constructs its elapsed timer with unit 3 (`0x00448E63–0x00448E95`; [timers.md](timers.md) §1.2), reads it
at `0x0044884E` and publishes the bounded integer delta at `0x00448934–0x00448987`. Its separate floating
message value is scaled by 0.001; the ripple uses the integer slot 14, not that float
(`0x00761FC4–0x00761FCF`). Thus 175000 is 0.175 seconds and the strict rebuild threshold is 2 seconds.
Direct tick callers must use these same units; the owner itself adds the raw word without conversion.

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
**strictly > 2000000**, not equality. Delayed rebuild **clears pending first**, clears all shores/strips,
redetects, then recreates; CreateRipples consequently does not redetect a second time. Pending is already
false during both detection and recreation (`0x00761E9F`, calls at `0x00761EA3/0x00761EAA/0x00761EB1`).
The rebuild happens before frame animation on that tick (`0x00761E87–0x00761EE9`). This is the WR14 rule;
the immediate message below deliberately differs. The rebuild clock retains its accumulated value until
another arming event resets it; clearing pending is not a clock reset.

Message `0x89171496` clears the shore collection, redetects and recreates immediately (`0x00761FC1–0x00761FE8`).
It does not itself clear a pending rebuild; CreateRipples sees that pending flag, clears/redetects again and
clears it. A zoom mismatch in Update calls CreateRipples: normally it reuses cached shores and rebuilds only
strips; if pending, CreateRipples first clears/redetects shores. Zoom comparison/recreation occurs only inside
the nonempty-shore and water-effects-enabled draw branch (`0x00761CF7–0x00761D31`, `0x00762004–0x00762094`).

## 2. Selection highlights (`cSTESelectedTerrainArea`) *[compat]*

### 2.1 State

Two tables keyed by vertex index (z × (cells x + 1) + x): **plain** cells (a 32-bit palette index) and **textured** cells (a
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

**Proved WR7 selection geometry/state subset.** Both draw routines execute **one pass**, with plain state A
and textured state C below. The B/D states and 0.67 multipliers occur in the bodies, but their second-pass
branches are unreachable: loop bounds are literal 1 (`0x007514D2–0x007514DA`, `0x00750F94–0x00750FA0`).
The four parameter sets are still initialized; list their arguments in native PackRenderState order:

| State | kPreferOpaque false | kPreferOpaque true |
|---|---|---|
| A, plain `00B4BDA0` | `(1,1,1,0,1,0,0,3,4,5,FFFFFFFF,BF800000,FFFFFFFF,FFFFFFFF)` | `(1,1,1,0,0,0,0,3,FFFFFFFF,FFFFFFFF,FFFFFFFF,BF800000,FFFFFFFF,FFFFFFFF)` |
| B, plain `00B4BDA8` | A with argument 8 = 4 | A with argument 8 = 4 |
| C, textured `00B4BDB0` | A with argument 6 = 1 and argument 13 = 1 | A with argument 6 = 1 and argument 13 = 1 |
| D, textured `00B4BDB8` | C with argument 8 = 4 | C with argument 8 = 4 |

Arguments are numbered from 1, excluding the output-state pointer (`0x00750600–0x00750795`). Shared combiner
bytes 0..7 after initialization are `(2,0,2,32,!preferOpaque, preferOpaque?3:0,3,1)`; opaque initialization does
not rewrite byte 6. Only clear-marking or Init refreshes these shared states; Update does not.

For key k and stride s, x = binary32(signed32(k % s)) × 16, z = binary32(signed32(k / s)) × 16. Corner height
is selected from the two live terrain render arrays at globals `0x00B4C758`/`0x00B4C75C`, each with 32-byte
records and binary32 height at +4. Let A be the first array's height and B the second's: choose B only when
A is ordered strictly less than B; otherwise copy A's original 32 bits. Thus a NaN in A propagates, a NaN
only in B is ignored, and equal signed zeros preserve A's sign. This is not an operand-independent maximum.
The x87 status test uses C0/C2 parity (`0x0075086A–0x0075088D`, `0x007509AA–0x007509C3`), repeated at
the other corners. Plain helper `0x00750820` supplies corners in order NW, NE, SE, SW; textured
helper `0x00750960` supplies NW, SW, SE, NE. Corner indices are k, k+1, k+s+1, k+s, reordered accordingly.
No normal is supplied in either vertex stream. Plain format token 1 is position plus packed colour; textured
token 2 is position plus two texture coordinates (`0x0075096E–0x00750A9E`, `0x00751417–0x00751450`).

UV tables at `0x00AB42A4` and `0x00AB4294` are respectively `(0,0,1,1)` and `(0,1,1,0)`. b1 is the first
table index, then the three succeeding indices wrap with &3. The first lookup does **not** mask b1, so b1 > 3
reads unrelated data. Nonzero b2 mirrors: even b1 swaps u of vertices 0/3 and 1/2; odd b1 swaps their v instead
(`0x007504F0–0x007505BA`). b3 sets transparency to binary32(b3 × binary32(1/255)). For textured drawing,
bind unit 0, wrap `(2,2)`, filtering `(1,1)`; if draw-context byte +0x78 is nonzero, set RGB/alpha texture colour
on change and send the low byte of mode as combiner byte 0 on mode change (`0x00750DBA–0x00750EBB`).

Each selection submits 4 vertices, primitive token 0 and six unsigned16 indices. Terrain slot 70 selects the
diagonal from (x,z). False gives `(0,3,2,0,2,1)`, true `(0,3,1,1,3,2)`; there are separate identical tables
for plain/textured at `0x00AB42B4–0x00AB42E3`. No per-tile frustum test occurs in these bodies. Traversal is
native hash-table bucket order ascending, linked-node order inside each bucket, **not sorted key order**.

**Submission history.** The constructor's requested 100 buckets becomes **193**, the first prime not less
than 100 (`0x00829BC0`, lower-bound helper `0x0079FA90`, prime data `0x00A829B0–0x00A82A1F`). Hashing
is unsigned32(key) modulo bucket count. An existing-key update preserves position and does not resize;
an absent-key insertion first requests capacity for size+1, then prepends a node (plain 12 bytes, textured
44 bytes) to its bucket (`0x007515E0–0x00751640`, `0x00750C30`, `0x00750BC0`). When size+1 exceeds
bucket count, choose the first prime not less than size+1; if none, use 4294967291. Rehash visits old buckets
ascending and repeatedly removes each old head, prepending it to its new bucket. Consequently collisions
can reverse at growth, and nodes from a later old bucket precede nodes from an earlier bucket when they
collide in the new table (`0x004F0940–0x004F0A26`). No node-address comparison participates. Clear frees
all nodes and zeroes count and heads, but **retains bucket count** (`0x008EBA60–0x008EBAB3`), so clearing
a previously grown selection is not equivalent to a fresh constructor for later submission order.
The capacity ladder is 53,97,193,389,769,1543,3079,6151,12289,24593,49157,98317,196613,393241,
786433,1572869,3145739,6291469,12582917,25165843,50331653,100663319,201326611,402653189,
805306457,1610612741,3221225473,4294967291. There is no checked allocation-failure return in these insertions.

The textured binding cache lasts one draw call: initial key `(0,0,0)`, binding null; on key change release the
old binding and request the new key from factory slot 6. False acquisition skips this tile. Repeated adjacent
equal keys reuse the cached binding, or keep skipping after failure without retry. A first all-zero key does
not request a binding. Release the last binding at the end, then reset transparency. Plain drawing sets white
self-lit shade, transparency 1 and default model transform; resets shade/transparency at the end. Update saves
the incoming depth offset at context +0xBC, sets 4, draws plain then textured, and restores exactly that saved
offset. Other render state is not saved/restored (`0x00750CF0–0x00750FB8`, `0x007514F0–0x0075153B`).

Plain marking's 32-bit value is used as an **unchecked palette index**, not a packed colour: RGB palette is
red/green/blue/yellow/black; alpha table is `(0.5,0.5,0.5,0.5,1)` at `0x00AB42E4`. RGB values are converted
to packed BGRA bytes using q = low32(bits(binary32(value + 49152)) − `0x47400000`), byte = low8(q − (q >> 8));
alpha uses the same conversion. This gives 255 for 1 and 128 for 0.5. There is no clamp or palette bounds check
(`0x0075110E–0x00751413`). First plain draw copies RGB triples into the shared palette once, guarded by bit 0
at `0x00B4BDFC` (`0x00750FC0–0x00751084`). Entries 0..3 copy the executable's red/green/blue/yellow
constants at `0x00B0D6B4–0x00B0D6E3`; entry 4 copies `0x00B4BD94–0x00B4BD9F`, process-zero-filled
storage (PE section virtual tail). Each of those three source addresses has only its copy load in the exported
instruction stream and its address is not otherwise present as an immediate in the executable. Their initial
triple is therefore (0,0,0); no owner path changes it. This palette survives instance Shutdown/destruction.

### 2.4 Lifecycle and tuning ownership (WR8)

Both constructors initialize their inherited reference count to **0** through `0x0090DA1E`, not 1. Selection
constructor initializes two empty hash tables with requested bucket count 100, terrain/factory pointers null;
it does not initialize cell counts or the scratch vertex buffers (`0x00751BC0`). Ripple constructor initializes
initialized=false, first-Update=false, null terrain/texture, empty shores, zoom 0, pending=false, accumulated
area/counter snapshot zero and the empty dirty rectangle (§1.5); scratch vertices and pending clock are not
generally zeroed (`0x00761790`).

Selection Init(terrain*) returns true on its normal path, AddRefs terrain, gets cell dimensions and stride,
acquires factory service `(0x1AC0E11A,0xFAC0E219)`, retains a reference and releases the temporary service
reference. There is no already-initialized guard or release of the old pointers on repeated Init. Null terrain
or unavailable factory is not a graceful false-return path: subsequent calls dereference them. Shutdown returns
true, releases/non-null-clears terrain then factory; repeated Shutdown is harmless for these pointers and does
not clear selections (`0x00750AA0–0x00750B5B`, `0x007505C0–0x007505F8`). The deleting destructor frees both
tables and invokes its base destructor; its direct body does not call Shutdown (`0x00751C40`).

Ripple Init() rejects an already-initialized instance with false. Otherwise the process-wide tuning-read flag
is set before reading tuning and a reset callback registered. The object marks itself initialized, obtains city
extents and retained terrain, dimensions/stride, water height and current zoom, initializes white RGB/UV-side
constants, then subscribes to animation ticks (mask `0xC0`) and the terrain edit/immediate rebuild messages;
returns true on the normal path. Missing optional subscription services are skipped, but missing required city/
terrain services have no checked false path. The texture is acquired lazily in StartNewRipple, not Init.
Shutdown, guarded by initialized, clears that flag, unsubscribes messages/tick, releases/nulls terrain and binding,
and returns true. It does not clear shores, reset first-Update, zoom or dirty rectangle. Re-Init after Shutdown
therefore is not construction of fresh state (`0x00760330–0x00760477`, `0x0075FBB0–0x0075FC31`).

**Reference and deletion paths.** Object base+4 is the reference interface; base+8 is its unsigned32 count.
AddRef increments with wrap and returns the new count (`0x00968B60`). RemoveRef decrements only if nonzero
and never deletes (`0x0090D981`). Release decrements and returns when count >1; at count 0 or 1 it invokes
the deleting destructor with flag 1 and returns 0 (`0x0090D997`). Refcount is not atomic. Destructors free
container storage, invoke `0x0046B4C0` / base `0x0090D990` (which only installs its base vtable), and free
the object only when flag bit 0 is set. Ripple destruction visits nested shore storage (`0x007619D0` →
`0x00761760`); selection destroys textured then plain tables (`0x00751C40`). Neither path calls Shutdown,
so destruction alone does not release held terrain/binding/factory or cancel ripple subscriptions.

QueryInterface(1,out) returns object+4 and AddRefs, even when called through the primary interface. Unsupported
IDs return false **without zeroing out** (`0x009457C6`). Ripple additionally accepts `0x452294AA`, returns
object+12 (message interface) and AddRefs (`0x0075FAE0`). Selection has no corresponding override.
Null out pointers are unchecked on success. The nominal null-this branch in ripple QueryInterface still
dereferences this to AddRef; it is not a valid null-object operation.

**Shared tuning and Cleanup.** The Init test/set is process-wide, not per object and not conditioned on a
successful resource lookup. It sets `0x00B4D524` and registers callback `0x0075FB20` before reading tuning
(`0x0076033E–0x00760368`). That callback returns true and only clears the flag if set; it does not free arrays,
restore executable scalar values, reset period bounds or touch any object. There is no Cleanup slot in either
owner's vtables below; do not invent instance cleanup behavior from the reset callback's role. After callback,
the next not-already-initialized ripple Init rereads tuning. Shutdown/destruction do not trigger that callback.

Before any read, executable scalar storage at `0x00B0D7B0–0x00B0D7C7` contains enabled=true (byte),
minimum=10, height=2, period=1000000, edit area=850 and fade count=5. These are storage initializers, distinct
from the exemplar values in §1.1. The float/byte vectors and derived step/period arrays are initially empty/
zero-filled. Failed exemplar acquisition returns from ReadTuningParameters without changes or resource Release;
Init still continues and the shared flag remains set (`0x00760099–0x007600A1`, `0x00760368`). Thus missing
tuning does not install usable five-element vectors and does not produce a false Init result by itself.

Property lookup failure leaves its destination unchanged; the owner ignores every getter result. Scalar types
are bool for suffix 00, unsigned32 for 01/07/0C/0D, float32 for 03. Their helpers call variant output getters
at byte offsets 0x28/0x64/0x94 (`0x005FD390`, `0x005FD480`, `0x005FD4F0`). The native variant getters
directly copy matching types (tags 1/6/10); otherwise they call ConvertTypes into a temporary target variant,
write destination only on conversion success, destroy the temporary, and return success/failure. They do not
silently write zero on conversion failure (`0x009174F6`, `0x009176C8–0x00917720`, `0x00917852–0x009178B1`).
The caller-specific numeric conversion contract is in §2.6; the shared conversion rules remain in
[data-formats §5.1](data-formats.md#51-converting-getters), not a separate ripple conversion algorithm.

Float arrays (02/04/05/06/0A/0B) and byte arrays (08/09) use different copy helpers `0x005FFCC0` and
`0x005FFB40`. On property presence they take the variant's raw array pointer and element count (slots
0x1BC or 0x17C, and 0x10), resize an **owned global vector** to that count, copy exactly count floats or
bytes, and compact excess capacity. No array type or length check occurs: the native variant pointer getters
both load +4, the count getter loads +0x10 (`0x0060CE80`, `0x0040CE60`, variant vtable `0x00ACECD0`).
Malformed types therefore do not get a safe typed conversion or a five-element fallback. There is no release
of borrowed property/variant pointers; successful exemplar acquisition is released after all copies at
`0x00760320–0x0076032B`. Owner destruction does not free these global vectors.

After reading minimum, clamp it to 2 by an **unsigned** comparison, so `0xFFFFFFFF` remains unchanged
(`0x007600CC–0x007600E3`). After the spacing getter, access exactly elements 0..4 regardless of result or
length: save each original value/64 to the step array and square it in place. Turn-limit elements 0..4 are
multiplied in place by the binary32 degree factor at `0x00AA8C60`: bits **`0x3C8EFA35`**, exactly
`0.01745329238474369049072265625` (`0x007602AB` onward, WR11). Each product is stored back to binary32;
do not use a double-precision π/180 expression as an intermediate substitute. The other arrays
retain their supplied count. A missing/short spacing or turn vector can cause invalid reads/writes; no false
return, padding, or rollback exists. If a reset/reload omits an array that was already loaded, old transformed
values are transformed again. Earlier field changes survive any later failure.

### 2.5 Per-owner native API checklist

Addresses here are **actual dispatch targets**, not the unrelated folded names in the export. Signatures
describe the observed call boundary; they are not new C ABI exports. Both primary tables have five slots:

| Slot | Call shape / behavior | Selection table `00AB42F8` | Ripple table `00AB4624` |
|---:|---|---|---|
| 0 | bool QueryInterface(uint32 id, void** out) | `005BCB40` → `009457C6` | `0075FAE0` |
| 1 | uint32 AddRef() | `005BE3E0` → `00968B60` | same |
| 2 | uint32 Release() | `005BCB30` → `0090D997` | same |
| 3 | Update(draw-context pointer), result unused | `007514F0`, §2.3 | `00761FF0`, §1.4 |
| 4 | bool Pick(four 32-bit arguments) | `00735290`, false, no output writes | same |

Each reference-interface table (`00AB430C` selection, `00AB460C` ripple) has six slots: QueryInterface,
AddRef, Release, deleting destructor(unsigned flags), RemoveRef(), GetRefCount(). Query/Add/Release adjust
back to the primary object then use the targets above; GetRefCount simply returns count (`0x009D7E63`,
whose export name is misleading). Deleting-destructor thunks subtract 4 and reach `0x00751C40`/`0x007619D0`.
Ripple's message table at `0x00AB45FC` has four slots: adjusted Query/Add/Release (subtract 12) and
DoMessage(message pointer) at `0x00761F80` (§1.5). No Init, Shutdown or Cleanup slot occurs in these tables.

| Owner | Nonvirtual callable contracts / direct entry points | Proof obligation |
|---|---|---|
| Selection | constructor `00751BC0`; bool Init(terrain*) `00750AA0`; bool Shutdown() `007505C0`; ClearCurrentSelections() `00750CA0` | §2.4; repeated Init leaks prior retained pointers, repeated Shutdown leaves selections intact |
| Selection | five MarkSelected argument forms `007516D0/00751780/00751910/007519A0/00751A70` | exact input widths/order in §2.2; helper insertions are not additional virtual API |
| Selection | draw preparation/UV helpers `007504F0/00750820/00750960`; DrawTextured/Untextured `00750CF0/00750FC0`; InitRenderState `00750600` | §2.3; allocation/copy/destruction helpers have separate storage roles |
| Ripple | constructor `00761790`; bool Init() `00760330`; bool Shutdown() `0075FBB0`; process reset callback `0075FB20`; ReadTuningParameters() `00760060` | §2.4; failed/partial Init is not rolled back |
| Ripple | DetectShores `00761B70`, AddShore `00761A00`, CreateRipples `00761CF0`, shore traversal `00761140`, spline `0075FC40`, StartNewRipple `00760EE0`, preparation `0075FEE0` | §§1.2–1.3; resource failure contract above |
| Ripple | DrawRipple `0075FE70`, grouped bounds `007609B0/00761010`, contact `0075FB40/007607F0`, edit `00760E10`, tick `00761E70`, remove-last `007606B0` | §§1.4–1.5; message dispatch table above |

WR8 remains open for remaining literal signatures/owner attribution beyond the bounded storage contracts in
§2.7, Pick's semantic argument types, and reconciliation of the brief's 65 functions/15308 bytes with the
69-body/15276-byte address-window inventory.
The vtable checklist, direct lifecycle/resource
paths and array copy/failure rules above are proved independently of those residuals; no whole-owner credit.

### 2.6 Tuning caller conversion and retained-state ledger (WR8)

This is the normal-return contract of `00760060–0076032E`, not a promise that malformed exemplars return
normally. In order, the reader requests suffixes 00, 01, 02, 03, 04, 05, 06, 07, 08, 09, 0A, 0B, 0C, 0D
of `0x491C9C00`. All helper bool results are discarded. The following table binds the actual calls to the
shared variant contract; an absent property returns false without writing the named destination.

| Suffix | Helper; variant slot in bytes | Destination and immediate use |
|---|---|---|
| 00 | `005FD390`; `+28` → `009174F6` | one-byte enabled at `00B0D7B0` |
| 01 | `005FD480`; `+64` → `009176C8` | u32 minimum at `00B0D7B4`; unsigned clamp to at least 2 even if absent |
| 02 | `005FFCC0`; raw `+1BC`, count `+10` | owned float vector `00B4D548`; always transform indices 0..4 to step and squared spacing |
| 03 | `005FD4F0`; `+94` → `00917852` | binary32 height at `00B0D7B8` |
| 04, 05, 06 | same raw float-vector helper | `00B4D554`, `00B4D560`, `00B4D56C`; copied without a five-element clamp |
| 07 | same u32 helper | period `00B0D7BC`; recompute the bounds in §1.5 even if absent |
| 08, 09 | `005FFB40`; raw `+17C`, count `+10` | owned byte vectors `00B4D578`, `00B4D584` |
| 0A, 0B | same raw float-vector helper | `00B4D590`, `00B4D59C`; 0B always multiplies indices 0..4 by the stored degree factor |
| 0C, 0D | same u32 helper | edit area `00B0D7C0`, fade count `00B0D7C4`; no clamp here |

**Scalar match and conversion gates.** The property helper obtains the borrowed property through holder
slot `+24`, then its variant through property slot `+14`. The native variant vtable `00ACECD0` binds the
three output getters above. They compare the stored 16-bit tag with Bool=1, Uint32=6 or Float32=10.
A matching tag takes the direct output path and returns true. A different tag constructs a zero-valued
**temporary target** of exactly that type (`00915E8E`, `00915D6D`, `00915E0B`), calls
`00917350(source, target)`, and copies target to the global only when AL is nonzero. The target variant
is mutated in place by conversion; the borrowed source variant is not converted in place by this getter.
Temporary destruction happens on both return paths. Conversion false leaves the old global value, after
which the owner still performs its clamp/derived-array work. There is no caller-level type rejection,
default installation, abort-on-false or rollback (`009174F6–0091754C`, `009176C8–0091771E`,
`00917852–009178AF`).

For **scalar numeric tags 1..14 (Bool through RZChar)**, this is positively the shared converter of
data-formats §5.1: its source and target range gates are `0091735A–00917389`; bool and u32 targets take
`00915AA5` at `009173B0`. The helper widens the source to 64 integer bits (signed/unsigned according to
its tag, signed Char/RZChar, unsigned UnicodeChar). Float32 and Float64 loads are respectively 32 and
64 bits at `00915B13/00915B1D`, then `_ftol2` truncates finite representable values toward zero.
Bool tests the OR of **both** 32-bit halves (`009173D0–009173DF`); u32 stores only the low half
(`009173EA–009173F0`). Neither is a floating-point nonzero test or saturating unsigned conversion.
Float32 targets instead take the shared to-double path and store a binary32 result
(`00917434–0091744B`). No process-wide floating precision/rounding setting is established by this reader;
the inherited mode remains a parameter wherever the shared operation depends on it.

Array/string tags, Empty and objects do **not** pass that numeric source-range gate. Their scalar requests
enter the foreign round-trip path at `0091745F`; apply an already specified data-formats §5.1 case only
with its original source type and assumptions. A raw pointer from the vector helper is not evidence that
the scalar numeric converter accepts that type. This spec adds no text/parser/object/exception matrix.

**Raw arrays are a separate operation.** On presence, the vector helper fetches pointer then count,
resizes to count, copies count × 4 bytes for float vectors or count bytes for byte vectors, and compacts
excess capacity on ordinary allocation success (`005FFCC0–005FFD74`, `005FFB40–005FFBFA`). There is no
ConvertTypes call, element-type gate, count-equals-five check or float interpretation during the copy.
For a correctly typed nonempty float array of six elements, all six are copied, and only the first five
spacing/turn elements are transformed. An absent property preserves the old vector; a present zero-count
property resizes it to empty instead. Neither makes spacing/turn's subsequent five unchecked accesses safe.
A wrong-width array can over-read backing storage. Only valid backing and valid subsequent indices are
within this normal-domain contract; no safe zero/default result is inferred for the other inputs.

**Independent caller-state ledger (proposed, unexecuted).** In each numeric row the other properties,
especially spacing and turn arrays, supply valid backing. Inputs are literal typed variants, not text.

| Input / event | Conversion and caller state after normal return |
|---|---|
| 00 Float32 0.75, old enabled=true | truncation gives int64 0, so enabled=false; a direct float nonzero test would differ |
| 01 Float32 −1.75 | int64 −1 → low u32 `FFFFFFFF`; unsigned clamp keeps `FFFFFFFF` |
| 01 Sint32 −2 | sign extension → low u32 `FFFFFFFE`; clamp keeps it |
| 00 Uint64 `00000001_00000000` | both-half bool test gives true despite zero low half |
| 01 same Uint64 | low half 0, then clamp → 2 |
| 01 Uint64 `00000001_00000007` | low half 7, clamp keeps 7 |
| 01 absent, old minimum=1 | helper leaves 1; caller clamp changes it to 2 |
| 01 absent, old minimum=50 | helper leaves 50; clamp keeps 50 |
| 02 Float32 array `[2,3,4,5,6,99]` | owned count 6; step `[1/32,3/64,1/16,5/64,3/32]`, squared storage `[4,9,16,25,36,99]`; source array unchanged |
| Existing spacing `[4,9,16,25,36,99]`, 02 absent on reread | count still 6; step `[1/16,9/64,1/4,25/64,9/16]`, squared storage `[16,81,256,625,1296,99]` |

The read flag is a **process** flag (`00B4D524`), set before reading/registering the reset callback
(`0076033E–00760368`). A reset clears only that flag (§2.4), retaining scalar globals, array contents,
counts and derived values. Already-initialized Init still returns false before checking that flag.
For a stopped instance, Shutdown then Init with flag still set skips rereading and preserves the tuning.
After reset, the next eligible Init rereads: a missing scalar retains its previous value, while absent 02/0B
cause another transform of retained arrays. A missing whole exemplar returns before **any** clamp or
posttransform, unlike a present exemplar with missing properties. A successful present exemplar releases
its acquired reference only after the last property call. Earlier mutations are not transactional.

### 2.7 Owned geometry storage operations (WR8)

These are owner-instantiated storage operations, not additional public owner methods. The contracts below
require valid container invariants, non-overflowing sizes, valid iterator ranges and ordinary allocation
success. Constructors require distinct uninitialized destination storage; assignment permits exact
self-assignment where explicitly stated, or distinct independently owned containers. Arbitrarily overlapping
backing stores, foreign allocator failures/exceptions and payload reentrancy are outside these contracts.
They imply neither rollback nor a universal coherent-Drop guarantee.

**Record layout and copy depth.** A strip payload occupies `0x4C` bytes. Its first `0x0C` is a vector
descriptor (begin/end/capacity-end) for `0x18`-byte vertex records; `+0C..+14` are three copied words,
`+18..+27` a copied four-word record; `+28..+33` is a second owned vector descriptor, for `0x10`-byte
contact rectangles; `+34..+4B` are six more copied words. A strip list node is `0x54` bytes:
next/previous links at 0/4, payload at +8. These widths follow the allocation sizes, increments and literal
loads/stores, not the folded template names (`007605B0–007606A2`, `00760754–0076077F`).

A shore record is `0x38` bytes: an owned vector of `0x08`-byte control points at +0, a strip-list sentinel
pointer at +0C, then a copied four-word record at +10 and six copied words at +20..+37.
Shore copy construction allocates a separate control-point buffer and a separate `0x54`-byte sentinel,
then deep-copies source strips in forward list order before copying the tail words
(`00761590–00761624`, point-vector constructor `007CED60–007CEDD5`). Strip copy construction separately
allocates/copies its vertex and contact-rectangle buffers (`00A6CB50–00A6CBBC`, `004B00B0–004B010B`).
The copied records are literal two-, four- or six-word values, not borrowed geometry-buffer pointers.
These paths perform no payload AddRef/Release. They do not copy or retain the owner's terrain/texture
binding; those references follow §2.4 instead.

**Literal operation channels.** “Receiver” means ECX; listed stack arguments are in increasing stack
order after the return address. Names here describe the evidenced payload/iterator role, not an inferred
SDK signature. Ignored allocator/tag slots are opaque words, not callable recipes for invalid inputs.

| Body | Inputs actually used; result | Normal-domain effects |
|---|---|---|
| `00760480` | receiver vertex-vector descriptor; stack source descriptor; EAX=receiver | Exact descriptor self-assignment is a no-op. Otherwise copy all source `0x18`-byte records. If source count exceeds destination capacity, allocate exactly count records, copy before freeing old buffer, then replace begin/capacity/end. Otherwise reuse buffer: overwrite existing prefix, copy-construct any growing tail, set end to new count, retain capacity. Shrinking does not free the buffer. |
| `007605B0` | receiver uninitialized strip; stack source strip; EAX=receiver | Deep-construct vertices first, copy +0C..+27, deep-construct rectangles at +28, copy +34..+4B. |
| `00760630` | receiver live strip; stack source strip; EAX=receiver | Assign vertex vector first, copy +0C..+27, assign rectangle vector at +28, copy +34..+4B. Exact same-strip assignment is safe through the nested descriptor self-checks; distinct overlapping buffer ownership is unsupported. |
| `007606B0` | receiver strip-list header (pointer to sentinel), no stack input used; no specified result | Remove sentinel.previous, unlink it, free rectangle buffer, vertex buffer, then node. Requires a nonempty list; there is no empty-list guard in this helper. Despite its exported RemoveLastRipple name, receiver is shore+0C, not the ripple owner. |
| `007606F0` | receiver strip-list header; no specified result | Visit nodes forward, remembering next before freeing rectangle buffer, vertex buffer and node; finally set sentinel.next and sentinel.previous back to sentinel. Retain sentinel. Empty list is supported. |
| `00760740` | stack insertion-position node, source first node, source end node, ignored word; no specified result | Walk source `[first,end)` forward; allocate/copy one `0x54`-byte node at a time and link each immediately before the fixed insertion position. Source order is preserved; prefix already inserted is not undone by any local rollback path. Range must remain traversable to end; do not generalize this to self-insertion. |
| `00760790` | stack output iterator-word address, first node, end node; EAX=output address | Unlink/free `[first,end)` forward in rectangle/vertex/node order. Write end node to the output word, including for an empty range. Receiver is unused. Retain sentinel. |
| `00761590` | receiver uninitialized shore; stack source shore; EAX=receiver | Deep-copy points, construct new empty sentinel, invoke `00760740` before that sentinel for every source strip, then copy +10..+37. |
| `00761630` | stack first shore, end shore, live destination shore; EAX=advanced destination | Assign `(end−first)/0x38` records forward: points via `00611B00`, list via `00760930`, then +10..+37 word copies. Empty range returns the unchanged destination. Observed callers pass two additional unused opaque words. This is a range-assignment primitive, not vector replacement/allocation. |
| `007616E0` | stack first shore, end shore, uninitialized destination shore, ignored word; EAX=advanced destination | Deep-copy-construct each shore via `00761590`, advance source and destination by `0x38`. Empty range returns unchanged destination. Null destination is not supported backing for a nonempty range. |
| `00761710` | stack first shore, end shore, ignored word; no specified result | Destroy shores forward: clear strip list, free its sentinel, free control-point buffer. Does not deallocate the outer shore array or zero destroyed descriptors. Empty range is supported. |
| `00761760` | receiver shore-vector descriptor; no specified result | Destroy `[begin,end)` through `00761710`, then free outer begin if nonnull. Does not reset descriptor fields, so repeated destruction of the same live descriptor is unsupported. |

Vertex copying is six literal 32-bit words per element, via `0082C140` for construction and `006240D0`
for assignment. Rectangle copying is four literal words via `00637E20`/`007257C0`. The latter folded
helper names containing “clear” do **not** prove clear semantics. Point assignment's `00611B00`
self-check, capacity branch and `007CEC30`/`00463DF0`/`0063BFF0` callees similarly copy two-word records.
Rectangle vector assignment `00727D70` has the same self/capacity/reuse shape as the vertex vector, with
`0x10`-byte elements. These specific primitive uses establish copy depth; they are not a specification of
all generic vector/allocator behavior.

**List assignment and outer storage.** `00760930–007609AE` returns its receiver list descriptor.
Exact descriptor self-assignment returns immediately. Otherwise, while both lists have nodes, it assigns
their payloads through `00760630` in forward order. If source ends first, it erases the remaining destination
suffix through `00760790`. If destination ends first, it copies the source suffix before destination sentinel
through `00760740`. Existing common-prefix nodes and sentinel survive; nested buffers may change according
to their own capacities. Shore range assignment invokes this operation at shore+0C.

The actual callers of `00761630` in shore-vector clear `00761870` and rebuild `00761CF0` pass **end,end,begin**:
the assignment range is empty. They then destroy `[begin,oldEnd)` and set end=begin, retaining outer capacity.
The nonempty assignment branch is specified from its own instructions, not falsely reported as executed
by those clear callers. Outer growth `007618B0–007619C3` allocates a new block, deep-copies prefix and
inserted records, and (unless its final byte flag is set) deep-copies the suffix; only then destroys old
shores and frees old outer storage before installing the new descriptor. This is a copy, not transfer of
the old nested buffers. Its capacity arithmetic and exceptional paths are not a complete generic insertion
contract here.

**Destruction and reference order.** For a shore with strips A then B, ordinary destruction frees
A.rectangles, A.vertices, A.node, then B.rectangles, B.vertices, B.node, then the shore sentinel, then
shore control points. The next shore follows; the outer array is freed last (`00761710`, `00761760`).
No texture/terrain Release appears in these storage paths. Ripple Shutdown instead Releases terrain then
binding (§2.4); direct deleting destruction visits nested geometry and does not call Shutdown. The last-strip
caller checks list nonempty and last vertex count <8 before calling `007606B0`
(`0076152F–00761563`); that caller guard does not make empty pop safe in the primitive.

**Selection storage boundary.** `00750BC0` takes a table receiver and a stack source record, calls the same
`004F0940` growth helper proved in §2.3, hashes the source key with unsigned remainder by bucket count,
allocates `0x2C` bytes and prepends it. EAX points at node+4 (the copied record), and table+10 increments.
`00750B60–00750BB2` copies the `0x28`-byte record's words at +00,+04,+08,+0C,+14,+18,+1C,+20,+24 and
bytes +10,+11,+12; byte +13 is padding and is not copied. There is no reference acquisition in this copy.
Plain insertion `00750C30` uses the same growth/remainder/prepend sequence, a `0x0C`-byte node and two
copied payload words; EAX=node+4. Neither insertion primitive itself searches for duplicates: owner marking
performs its lookup/update decision before insertion. The folded find_or_insert name is insufficient evidence
for another duplicate gate.

Both selection tables are destroyed through `0058F3A0`: clear via `008EBA60` visits buckets in ascending
order, frees each forward-chain node, nulls the bucket, then zeros count; the wrapper frees bucket storage.
There is no per-record virtual Release. Deleting selection destruction visits table+ A0, then table+8C,
then base destruction and optional object free (`00751C40–00751C75`); Shutdown's terrain/factory Release
sequence is separate. This says nothing about reentrant foreign payloads or allocator errors.

**Independent storage ledgers (proposed, unexecuted).** Copy a shore with two control points and two
strips, each with nonempty vertex and rectangle vectors: destination point buffer, sentinel, both nodes
and all four nested buffers are distinct allocations with equal record bits and the same A,B order.
Changing source point/vertex/rectangle bits afterward leaves the copied geometry unchanged. Destroying
source leaves destination geometry alive and produces no geometry-triggered texture Release.
For list assignment destination `[X,Y,Z]` from `[A,B]`, the X/Y nodes survive with A/B payloads; Z's
rectangle buffer, vertex buffer and node are freed in that order, and destination sentinel survives.
For destination `[X]` from `[A,B,C]`, X survives with A, then new B and C nodes are appended in order.
For exact list/vertex-descriptor self-assignment, pointer identities/counts/capacities remain unchanged.
For vertex source count 2 and destination size 3/capacity 4, reuse storage, copy two records and set end
to begin+48 while retaining capacity 4; source count 5 instead allocates 120 bytes before freeing old storage.

**Evidence ledger, not owner coverage.** S means a fully supported body/domain, P a bounded path contract
with explicit residuals, F a foreign dependency beyond this owner's contract. Every owner-instantiated body
below remains **P**, even where all its ordinary control-flow branches were inspected: allocation failure,
invalid alias/range and exceptional behavior have not been promoted to a total contract. Ranges are inclusive;
sizes are export body bytes, not attribution totals.

| Body range | Bytes | Status / published operation |
|---|---:|---|
| `00760060–0076032E` | 719 | P: tuning normal caller CFG, numeric/common converter binding and raw arrays (§2.6); foreign conversions and unsafe backing excluded |
| `00760480–007605A5` | 294 | P: vertex assignment, all three ordinary size/capacity cases |
| `007605B0–0076062A` | 123 | P: strip copy construction |
| `00760630–007606A2` | 115 | P: strip assignment |
| `007606B0–007606EA` | 59 | P: nonempty pop |
| `007606F0–0076073F` | 80 | P: list clear |
| `00760740–00760789` | 74 | P: list range copy insertion; **not list destruction** |
| `00760790–007607E6` | 87 | P: list range erase; **not shore destruction** |
| `00760930–007609AE` | 127 | P: list assignment |
| `00761590–00761624` | 149 | P: shore copy construction |
| `00761630–007616DC` | 173 | P: shore range assignment; **not vector replacement** |
| `007616E0–0076170E` | 47 | P: shore range construction |
| `00761710–00761752` | 67 | P: shore range destruction |
| `00761760–00761789` | 42 | P: shore-vector destruction |
| `00761870–007618A3` | 52 | P: shore-vector clear, retained capacity |
| `007618B0–007619C3` | 276 | P: normal nested-copy/release order; full insertion contract residual |
| `00750B60–00750BB2` | 83 | P: textured record copy, padding excluded |
| `00750BC0–00750C2E` | 111 | P: textured node insertion |
| `00750C30–00750C95` | 102 | P: plain node insertion |

`0090CF54/0090CF63` forward to shared allocation/free infrastructure despite their folded constructor/string
names. Their foreign error/exception policies are F here; they are not ripple or selection owner methods.
The shared conversion, allocation and container primitives retain their own ownership attribution. The
brief's 65/15308 versus the address-window inventory's 69/15276 remains **unreconciled**: these positive
body contracts do not manufacture owner membership by subtraction or promote the historical A/R counts.

## 3. Users

The terrain's 3D view creates both (terrain.md §15); the tools mark selections (zone, lot and terrain tools); the
animation tick drives the ripples.

## 4. Quirks

1. The palette index of §2.2 is not checked.
2. `0x491C9C01` serves both as the minimum shore length and as the base of the edit threshold (§1.5).
3. Texture acquisition failure follows the WR8 control-flow proof in §§1.3,2.4; no safe empty-strip behavior is promised.

## 5. Open questions

See [questions/water-ripples.md](questions/water-ripples.md). WR4–WR7 and WR9–WR10 have native contracts here;
WR7 was answered in §2.3. WR8 has bounded lifecycle, caller and storage contracts in §§2.4–2.7 and retains
the explicit owner-attribution/signature residuals. WR1–WR3 remain separate semantic questions; in particular,
bit 1 is a temporary trace visit mask here, not evidence that it means water.

## 6. Conformance tests

All entries are proposed **UNEXECUTED** vectors, not native/game outputs or implementation validation.
Round 3 adds caller/storage ledgers in §§2.6–2.7 and entries 22–24 below. Entry 25 records a previously
pending rehash discriminator; it is not new native execution or additional credit for the older WR7 vectors.

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
16. **WR7 operand identity:** A=`0x7FC01234`, B=5 copies `0x7FC01234`; A=5, B=that NaN copies 5.
    A=-0, B=+0 copies `0x80000000` for either vertex stream. These distinguish operand-independent maxima.
17. **WR7 history:** in a fresh table insert keys 0,193,386: submission order is 386,193,0. Updating 193
    preserves that order. Insert all keys 0..192, then key 193: growth to 389 gives ascending key order
    0..193. Clear that grown table, insert 0 then 193: order is 0,193; the same pair in a fresh table draws
    193,0. This distinguishes retained capacity from rebuilding a table on clear.
18. **WR7 palette:** the first plain draw with index 4 submits packed opaque black `0xFF000000`, including
    when no prior draw initialized any of the other palette entries.
19. **WR8 references:** constructor count 0 → RemoveRef returns 0 and keeps storage → AddRef returns 1;
    Release at either count 0 or 1 deletes, but does not Shutdown. Unsupported QueryInterface preserves a
    prefilled output pointer. ID 1 yields object+4; ripple message ID yields object+12, each with one AddRef.
20. **WR8 reset/reload:** load spacing `[16]*5`, then reset callback and reread an exemplar missing property
    02 but otherwise valid: old squared spacing 256 becomes step 4 and squared spacing 65536. A freshly
    copied spacing 16 would instead give step 0.25 and squared spacing 256. Callback itself changes neither.
21. **WR8 failure:** a first failed ripple texture request nulls current-strip but appends a new strip; if no
    later request succeeds, preparation reaches null-binding+4. A first missing spacing property similarly
    reaches empty-vector element 0. Neither yields a safe empty successful result or transactional rollback.
22. **WR8 scalar caller ledger:** use the literal typed rows in §2.6, including Float32 0.75 → enabled=false,
    Float32 −1.75 → minimum `FFFFFFFF`, and Uint64 `00000001_00000000` → bool true but minimum 2.
    Missing minimum preserves the old value before the unsigned clamp; this differs from a conversion to zero.
23. **WR8 copy/absence/reset ledger:** copy spacing `[2,3,4,5,6,99]`, retaining count 6 and untransformed
    sixth element; reset then reload with 02 absent gives the second-transform row in §2.6. In contrast,
    missing whole exemplar does no transform; no reset before re-Init skips reading. Present count zero or
    insufficient/wrong backing is an unsupported posttransform domain, not a safe fallback expected result.
24. **WR8 nested ownership ledger:** use §2.7's independent two-strip deep-copy/free order and list grow/shrink
    examples, plus vertex size/capacity examples. Verify source/destination allocation independence and no
    geometry-path AddRef/Release, with Shutdown's terrain/binding reference order observed separately.
25. **WR7 pending rehash discriminator (retained lead):** insert 0,75077,1..191, then 192 into a fresh
    selection table. Before the last insertion there are 193 records and bucket 0 is `[75077,0]`.
    The last insertion requests 194, selecting 389 buckets; because `75077=193×389`, prepend rehash
    yields bucket 0 `[0,75077]`, whereas stable rehash would leave `[75077,0]`. The plain/textured insertion
    callers both invoke the proved helper `004F0940` (§§2.3,2.7); no prime-ladder/hash inference is transferred
    to another owner (including TickTK4) without its own helper/caller evidence.
26. **WR1 units:** inject integer elapsed 175000 microseconds, clock 0 and period 175000: advance once and
    zero the clock. Injecting 175 (milliseconds without conversion) must not advance. This tests owner input
    units, not host timer precision or scheduler cadence.
27. **WR11 factor:** turn factor bits `3C8EFA35`; transforming binary32 60 degrees once gives binary32
    `3F860A92`. Reset/reload with that array absent transforms the retained value again. An omitted whole
    exemplar does neither transform (§2.6).
28. **WR12 startup/shared stream:** supply host time word `8C09C57A`: initial effective/remembered seed are
    both zero, third word zero. First nonzero-bound raw draw returns 0 and leaves state `00003039`.
    Creating a second owner, tuning reset, Shutdown and re-Init do not restart that stream. For time word
    `73F63A85`, supply fallback time/clock samples and require their specified XOR/OR seed instead of
    `FFFFFFFF`. These are injected-source expectations, not native-run captures.
29. **WR14 delayed/immediate distinction:** with first-Update true, pending true and rebuild clock 2000000,
    elapsed 0 does not detect; elapsed 1 clears pending before the detection callback, detects once, recreates
    and animates new strips with elapsed 1. Clock remains 2000001. An immediate message with pending true
    detects twice; zoom-driven pending recreation detects once (§1.5).
