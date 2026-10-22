# Spec: animated sprite model instances (`cS3DModelInstanceAnimatedSprite`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [draw-context.md](draw-context.md) (every draw call, the packed state),
[view-utilities.md](view-utilities.md) (§7 picking, the metres per pixel), [camera.md](camera.md) (the camera's local
transform and view volume), [model-maker.md](model-maker.md) (which creates model instances),
[graphics-formats.md](graphics-formats.md).

## 0. Summary

An animated sprite is a model instance drawn as one camera-facing quad. Its picture comes from a sprite
animation: for each zoom, a sheet of frames in a texture, with one row of frames per view direction. Each draw
picks the row from the direction the camera looks at the instance and the frame from the animation clock, and
sizes the quad from the frame's pixels and the view's metres per pixel, so the sprite keeps its pixel size.

## 1. The object *[compat]*

- Built by `0x007FDCB0` (the constructor entry; its body `0x007FDCBA`), from a pool, with the virtual base
  `cS3DModelInstanceBasic` (owner, custom data, bounds, transform; its slots are the `cS3DModelInstanceBasic::*`
  names). The `Pick` and `Draw` thunks at `+4` (`0x007FE230`, `0x007FEE90`) are what the scene calls.
- Its draw state (made by the constructor): colour and depth write, depth test less-or-equal, alpha test "greater"
  at 0.5, blending source alpha / one minus source alpha, one texture, mode 1.
- Destructor body `0x007FEEF0`: releases the per-zoom resources.

## 2. Creation and loading *[compat]*

- `GetAnimatedModelFromResourceKey(key)` (`0x007FEDE0`) by the key's type: `0x29A5D1EC` →
  `GetMeshAnimFromResourceKey` (`0x007FED50`), which **builds one of these sprite instances** and has it load the
  key (slot 22, so type `0x29A5D1EC` is the sprite animation), returning it or null; `0x5AD0E817` →
  `GetSpriteAnimFromResourceKey` (`0x007FD7D0`), which fetches the resource through the resource manager. Other
  types give false. The two Mac-matched names look swapped against what the functions build *[inferred]*.
- Loading (`0x007FD120`): from the model's per-zoom table, for each zoom the sprite-animation record of type
  `0x69B6F01C` is fetched and set up (`0x007FC320`): its frame count, its number of view directions N and the angle
  step 2π ÷ N, and the frame layout of §4.1; a missing zoom ends the loading.

## 3. The interface (vtable `0x00ABD220`) *[compat]*

| Slot | VA | Call | Behaviour |
|---|---|---|---|
| 1 | `0x007FC480` | `SetPosition(p)` | the translation; the transform changes |
| 2 | `0x007FE250` | `LookAt(from, up)` | §3.1 |
| 3 | `0x007FC4B0` | `SetOrientation(yaw, pitch, roll)` | §3.1 |
| 4–7 | `0x007FCD30`, `0x007FCC90`, `0x007FCE20`, `0x007FCDC0` | point and vector transforms | by the instance's transform and its inverse |
| 8 | `0x007FB180` | `SetView(zoom, rotation)` | two bytes; the frame is due again when they change |
| 9 | `0x007FCEA0` | `SetAnimation(index)` | picks a sequence of the animation table (its first frame and length) |
| 10 | `0x007FCF00` | `SetFrame(n)` | n modulo the sequence's length |
| 11, 12 | `0x007FB1C0`, `0x007FB1D0` | `GetFrame`, `GetFrameCount` | |
| 13 | `0x007FCF60` | `AdvanceFrame()` | §4 |
| 14 | `0x007FB310` | `SetRate(r)` | |
| 15 | `0x007FCFB0` | `Tick(elapsed)` | §4 |
| 16 | `0x007FB330` | `SetPlayMode(m)` | §4 |
| 18 | `0x007FB340` | `SetFade(on, alpha)` | flag 2 and the alpha, when the alpha is within range |
| 19 | `0x007FB3B0` | `SetHighlight(index)` | flag 1 and the highlight index; 0 clears |
| 20 | `0x007FB410` | `SetSelfLit(on)` | flag 4 |
| 21 | `0x007FD120` | `LoadFromResource(resource)` | §2 |
| 22 | `0x007FD0C0` | `Load(key)` | the resource of type `0x29A5D1EC` for the key, then slot 21 |

Slot 0 (`0x007FAF40`) and slot 17 (always 0) are shared bodies.

### 3.1 Placement

- The instance keeps a transform (rotation, translation, a change counter); `0x007FE0F0` and `0x007FE190` copy it
  out and in. `SetOrientation(yaw, pitch, roll)` (`0x007FC4B0`) builds the rotation from the angles that are not
  −100 (−100 means "unset"); `0x007FE250` turns the instance to look from its position along a direction (an up
  vector given); `0x007FCC90` transforms a point by it.

## 4. Animation and frames *[compat]*

- The clock (`Tick`, slot 15): a rate whose bits are `0xC2C80000` (−100) calls slot 14 with −200.
  The native sprite's `SetRate` simply stores that float and returns 2: **there is no resource-rate lookup**.
  A rate of 0 does nothing. Otherwise position := position +
  elapsed × rate × direction. When |position| reaches 1: below 2, one frame is advanced (slot 13) and the direction
  subtracted; at 2 or more, frames are advanced until |position| < 1 when `+0xAA` is set *[inferred: frame
  skipping]*, else one frame is advanced and the position set to exactly 1 *[quirk]*. It returns 2 when frames
  advanced.
- Advancing (`AdvanceFrame`, slot 13): with an animation table, the table's sequence steps (`0x007FAE90`); without
  one (`0x007FB1E0`), with more than one frame, the next frame; at the end, by the play mode (`+0xA8`): 3
  loops to frame 0, 2 reverses (ping-pong, direction −1), anything else stops (direction 0, position 0). Each change
  marks the frame dirty (`+0x9C`).

### 4.2 Exact stepping and returns

`SetRate` preserves the supplied 32 bits, with return 2 (`0x007FB310–007FB321`). `SetPlayMode` stores only
the low byte, without touching direction, clock or the separate sequence controller; its return register is
not a defined status (`0x007FB330–007FB33C`). Frame and owner direction are unsigned and signed bytes
respectively. `AdvanceFrame` always returns 2, including no change and stopping (`0x007FCF60–007FCFA6`).
Without a sequence controller, counts at most one leave everything unchanged. For larger counts:

| Boundary/state | Mode 3 | Mode 2 | Any other mode |
|---|---|---|---|
| Forward at last frame | frame 0 | previous frame, direction −1 | retain frame, direction 0, clock 0 |
| Reverse at frame 0 | last frame | frame 1, direction +1 | retain frame, direction 0, clock 0 |
| Direction 0 | retain frame | retain frame | retain frame |

All rows for counts greater than one mark the frame dirty, including a stopped direction. Interior steps move
one frame according to the sign of direction. Evidence: `0x007FB1E0–007FB306`.

`Tick` evaluates elapsed × stored rate × signed direction + stored clock with x87 arithmetic, storing the
clock as binary32. The first threshold tests use the still-held x87 result, rather than reloading that store.
After every advance it reads the **new** signed direction and subtracts it from the current stored clock.
It returns 0 for zero rate or magnitude below 1; it returns 2 after an advance, even if no frame changed.
For magnitude at least 2, a zero skip byte advances once and overwrites the clock with +1. A nonzero skip byte
repeats advance then subtraction until magnitude is below 1. There is no progress/iteration guard: direction 0
with a large clock can loop forever, as can a negative rate that drives the clock opposite the subtraction.
Single-frame playback is not specially guarded by Tick; it terminates only when those subtractions converge.
Evidence: `0x007FCFB3–007FD0B7`; threshold data `0x00A81228=1`, `0x00A8825C=2`.

Independent examples, with no sequence controller: four frames, frame 3, direction +1, mode 2, clock 0,
rate 1, elapsed 1.25 yields frame 2, direction −1, clock **2.25**, return 2. Four frames, frame 0,
direction −1, mode 3, clock 0, rate 1, elapsed 1.25 yields frame 3, clock −0.25, return 2.
One frame, direction +1, skip enabled, rate 1, elapsed 2.5 keeps frame 0 and leaves clock 0.5, return 2.

### 4.3 Sequence-controller boundary contracts

The optional owner pointer `+0x34` is a mutable controller, not a decoded sprite frame record. Its fields are:
u16 selected sequence at +0, u16 sequence count at +2, u16 selected length at +4, pointer to u16 starts at +8,
u16 absolute current frame at +12, mode byte at +14 and signed direction byte at +15. Starts include a terminal
entry, since selecting i calculates the low 16 bits of starts[i+1] − starts[i].

`SetAnimation(i)` returns 0 if absent or signed i is at least the count. It does not reject negative i before
indexing: such calls are outside a safe decoded-record contract. An accepted call changes selection/length
but **does not reset** controller current frame, direction or clock. It copies the low byte of current frame
to the owner and dirties only if the full u16 differs from the old owner byte; return 2 (`0x007FCEA0`).

`SetFrame(n)` without a controller stores low byte n (no modulo), dirtying if the full argument differs from
the current byte. With a controller it uses signed remainder by selected length **only when n > length**;
equality is not wrapped. It adds that value to starts[selected] with u16 wrapping, stores controller current,
then compares/copies to the owner byte; return 2 (`0x007FCF00–007FCF53`). A zero length and positive n faults
at division; negative n is not normalized. Example: starts [10,13], selected 0, length 3: n=3 gives absolute
frame 13, n=4 gives 11, n=−1 gives 9.

Controller advance uses its own mode/direction, independently of the owner's. Length at most one returns true
unchanged; otherwise it steps inside [start,start+length), loops/reverses/stops using the same boundary modes
as the table above. Stopping only clears controller direction, not owner clock/direction. `AdvanceFrame`
copies its u16 current frame to the owner as above. Evidence: `0x007FAE90–007FAF34`, `0x007FCF60`.

### 4.4 Fade, highlight and self-light contracts

These setters preserve unrelated bits of the flags byte and do not dirty the frame cache.

`SetFade(false, alpha)` always clears bit 2 and sets stored alpha to 1. With on=true: exactly −100 leaves
both fields untouched; exactly −200 clears bit 2 and restores alpha 1; otherwise alpha at most the binary32
constant `0x3F7D70A4` (approximately 0.99) enables bit 2 and stores the supplied bits. Larger alpha disables
and restores 1. There is **no lower bound**, and the native unordered comparison also accepts NaN. Return is
always 2 (`0x007FB340–007FB3A8`; constants `0x00A98F64`, `0x00ABD0DC`, `0x00ABD0E0`). Thus alpha 1 disables,
alpha −1 enables, and disabling with alpha −100 still disables. This supersedes a guessed [0,1] interval.

`SetHighlight(index)` compares the full integer with the old unsigned index byte and compares (index!=0)
with bit 1; if both agree it returns 0. Otherwise it sets/clears bit 1 from the full integer's nonzeroness,
stores its low byte and returns 2 (`0x007FB3B0–007FB401`). Example: 256 enables highlight but stores index 0;
repeating 256 still returns 2. `SetSelfLit(on)` compares the input byte against bit 4 normalized to 0/1,
returns 0 when equal, otherwise toggles bit 4 according to nonzeroness and returns 2 (`0x007FB410–007FB44B`).

### 4.1 Frame lookup (`GetTextureResourceData` `0x007FD2F0`, `GetTextureData` `0x007FD3D0`)

For the zoom's record: the **direction** d = round(N × ((yaw + π) − base) ÷ 2π) mod N, where yaw is the angle at which
the camera sees the instance (negative pitches are wrapped by 2π first); the frame is then the record's entry for
(d, frame mod frames). The entry gives the texture page and the frame's place in it: frames are laid out left to
right and then downwards in a square page of side 2^k pixels, starting a new page when one is full; the frame's
texture coordinates follow from its pixel position and size, with the sign bits of two entry bytes flipping u or v.
A per-zoom scale other than 1 (by more than 0.0001) scales the frame's size and offset.

## 5. Drawing (`Draw`, `0x007FEA90`; `UpdateView`, `0x007FE400`) *[compat]*

Nothing is drawn when highlighted, or at zoom 0 or 1. Otherwise:

1. When the zoom or rotation differs from the cached one, or the frame is dirty: the view angles (yaw from the
   instance's forward vector, pitch by arcsine) choose the frame (§4.1); after a view change `UpdateView` recomputes
   the quad: the frame's width and height in pixels times the metres per pixel (the view utilities' `+0x150`, or from
   the camera's view volume when there are none), laid along the camera's right and up axes (camera.md §1.3) from
   the frame's offset, and the instance's box is widened to contain the four corners.
2. The corners are moved to the instance's position (when it moved).
3. Model transform; the shade from the lighting manager (`SetModelShade`), or self-lit with flag 4; the draw state;
   with flag 2 (fading) the instance's alpha as transparency and alpha test "greater" at 0; the frame's texture; one
   triangle fan of four vertices (`DrawPrims(2, 10, 4, …)`); shade and transparency are restored.

## 6. Picking (`Pick`, `0x007FD710`)

The quad's two triangles are tested with `SC4ViewUtilities::TrianglePick` (view-utilities.md §7); true on the first
hit.

## 7. Users

The model maker creates sprite instances for occupants whose models are sprite animations; the scene draws and
picks them through the two thunks; animation ticks drive the clock.

## 8. Quirks

1. Sprites vanish at the two farthest zooms (§5).
2. A fading sprite passes the alpha test at any non-zero alpha (§5).
3. The resource-key functions' names do not match what they build (§2).
4. Without frame skipping a long tick advances one frame and leaves the clock at 1, so the next tick advances
   again (§4).

## 9. Open questions

See [questions/animated-sprite.md](questions/animated-sprite.md) AS1–AS2: the sprite-animation record's layout
(type `0x69B6F01C`) and the bytes of a frame entry; which occupants use sprite models.

## 10. Conformance tests

1. **Zoom:** at zoom 1 the instance draws nothing; at zoom 2 it draws one quad.
2. **Direction:** with N = 8 and base 0, a yaw of 0 gives direction 4 (round(8 × π ÷ 2π)).
3. **Loop:** play mode 3 goes from the last frame to frame 0; mode 2 steps back to the previous frame.
4. **Clock:** rate 10, direction 1, elapsed 0.25: with `+0xAA` set, two frames and 0.5 kept; without it, one
   frame and the position 1.
5. **Size:** a 32-pixel frame at 0.5 m per pixel gives a 16 m quad.
