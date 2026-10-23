# GZ 2D graphics: contexts, targets, graphics service and Windows canvas

Status: **behaviour spec with explicit gaps**, Windows SimCity 4 Deluxe 1.1.641.
This is clean-room input under [CONTRIBUTING.md](../../CONTRIBUTING.md).
*[compat]* denotes required observed behaviour; *[choice]* denotes an implementation
choice where the original has no safe contract. Addresses identify evidence, not
implementation dependencies. A symbol name shared with an unrelated class in the
export can be a folded function; it does not identify the owner of that slot.

## Outline

1. Scope, ownership and conventions
2. `cGZDrawContext`: interface and state
3. Context transforms, drawing and target changes
4. Render-to engines and dispatch
5. `cGZGraphicSystem`
6. `cGZAlphaBlt`
7. `cGZCanvasW32`
8. Conformance vectors
9. Evidence limits and open questions

## 1. Scope, ownership and conventions

The GZ layer supplies a widget-facing 2D drawing context, a target-specific engine,
a graphics/driver service, alpha-blit operations and the host canvas. The context
retains the drawing target and engine independently. The engine consumes absolute
canvas coordinates and owns target-specific drawing resources. Drawing is
synchronous at this boundary, except for explicitly queued host messages and
driver work completed by a flush.

This context is distinct from the game's 3D helper:
[draw-context.md](draw-context.md) specifies `SC4DrawContext` and packed 3D render
state; [view3d.md](view3d.md) explains the view/driver boundary. The font catalogue,
glyph cache, metrics and text drawing are in [drawing-fonts.md](drawing-fonts.md).
Do not substitute either context's slots or state layout for the other's.

The software span kernels, their colour types, source sampling, blending, spans
and destination records are normative in these related specs:

- [span-render-8.md](span-render-8.md), particularly section 10, specifies the
  buffer engine's lifetime and its caller relationship to the context.
- [span-render-1555.md](span-render-1555.md),
  [span-render-565.md](span-render-565.md),
  [span-render-4444.md](span-render-4444.md) and
  [span-render-8888.md](span-render-8888.md) specify the other destination kernels.

*[compat]* Rectangles used for intersection have exclusive right/bottom edges.
Two rectangles touching only on an edge have empty intersection. Empty
intersection is represented by `(0,0,0,0)`, rather than an inverted rectangle
(`0x0079DD60`). Coordinates supplied to `Draw` are interleaved 32-bit float
`(x,y)` pairs; rectangle primitives use two such pairs. Integer convenience
methods convert their arguments before drawing. Unless otherwise stated, slot
numbers below are zero-based and byte displacement is four times the slot.

The Windows object's field offsets are interoperability facts. An implementation
which does not expose the native object layout may represent the same logical
state differently. Do not treat uninitialized native storage as a specified
default.

## 2. `cGZDrawContext`: interface and state

### 2.1 Method inventory

The primary table is `0x00AC16C8` (48 entries). This is the widget-facing
interface, IID `0x2AE63219`, also returned for unknown IID `1` at the same pointer.
Successful `QueryInterface` adds one reference. Unsupported IIDs return false;
the method does **not** clear the caller's output (`0x0082BE90`).

| Slot(s) | Method(s), in slot order | Evidence |
|---|---|---|
| 0–2 | QueryInterface, AddRef, Release | `0x0082BE90`, `0x0082BE80`, `0x0082CF10` |
| 3–6 | PushTransform, ClipReset, Clip, ClipAbsolute | `0x0082CFE0`, `0x0082BEC0`, `0x0082C6D0`, `0x0082C190` |
| 7–9 | Offset, OffsetAbsolute, PopTransform | `0x0082BEE0`, `0x0082BF00`, `0x0082C780` |
| 10–11 | PushState, PopState | `0x0082CFF0`, `0x0082C7D0` |
| 12–15 | GetCanvasArea, GetClipRect, GetOrigin, GetTransformStackDepth | `0x0082BF20`–`0x0082C1F0` |
| 16–18 | IsTargetAttached, SetTarget, GetEngine | `0x0082C210`, `0x0082C830`, `0x0060FB80` |
| 19–21 | ResetToDefaultState, SetColor(index, colour), SetColor(colour) | `0x0082BF80`, `0x0082C220`, `0x0082BFC0` |
| 22–25 | SetAlphaBlend, SetBltModulate, SetAntialiasing, SetLineThickness | `0x0082C270`–`0x0082C2D0` |
| 26–27 | ClearGradient, SetGradientTransform | `0x0082C350`, `0x0082C360` |
| 28–31 | ClearBlt, SetBlt, ClearBltTiling, SetBltTiling | `0x0082C370`–`0x0082C480` |
| 32–35 | Clear, Point, Line, Fill | `0x0082BFE0`, `0x0082C020`, `0x0082C060`, `0x0082C0C0` |
| 36–37 | two convenience primitives sharing the exported Ellipse-labelled entry | `0x0082C100`; signatures/geometry unresolved, GZG-1 |
| 38–40 | Blt, BltTiled, Flush | `0x0082C530`, `0x0082C620`, `0x0082C520` |
| 41–43 | IsSuspended, Suspend, Resume | `0x0082BFD0`, `0x0082C4E0`, `0x0082C500` |
| 44–47 | Draw, DrawNoTransform, Lock, Unlock | `0x0082EA60`, `0x0082FC30`, `0x0082C6B0`, `0x0082C6C0` |

`GetEngine` exposes the engine pointer. The folded getter is a field read, not a
new owning reference. The caller must keep the context/target alive.
`IsTargetAttached` tests the target pointer, not initialization success of the
engine. `Release` destroys at the final reference; destruction releases both
engine and target and frees the context's temporary vectors (`0x0082CA40`).

### 2.2 Logical state and defaults

| Field(s) | State | Construction/default contract |
|---|---|---|
| `+0x04`–`+0x10` | four caller colours | not initialized by the constructor examined |
| `+0x14`–`+0x20` | target-format conversions of those colours | not initialized by that constructor |
| `+0x24`, `+0x28` | retained target, retained engine | null |
| `+0x2C`, `+0x30` | float origin x/y | established as zero on attaching a non-null target |
| `+0x34`–`+0x40` | current integer clip | established from target on attachment |
| `+0x44`, `+0x45` | antialias and blit modulation bytes | zero |
| `+0x48` | alpha-blend mode (integer) | zero |
| `+0x4C`, `+0x50` | float line thickness, integer raster thickness | `1.0`, `1` |
| `+0x54`–`+0x60` | canvas area supplied by engine | established during attachment |
| `+0x64`–`+0x70` | source-supported destination bounds | established by SetBlt |
| `+0x74`, `+0x75` | active blit, tiling | cleared on non-null attachment |
| `+0x78` | nesting count of suspension | zero |
| `+0x7C` onward | transform stack, state stack, temporary geometry | empty vectors |

Evidence: constructor `0x0082C9A0`, attachment `0x0082C830`.
*[compat]* `ResetToDefaultState` is deliberately narrower than construction: it
sets alpha blend, modulation and antialiasing to zero, line thickness to `1.0`,
and clears blit, tiling and gradient. It does not reset the origin, current clip,
colour array, target, transform stack or state stack (`0x0082BF80`).

### 2.3 Attribute changes and engine notifications

`SetColor(colour)` changes index zero. The indexed operation stores the caller's
packed colour, obtains its target-native RGB representation through target slot
30 (`+0x78`), then notifies an existing engine with all four caller colours via
engine slot 7 `(first=0,count=4,colours)`. Alpha is preserved in the caller-colour
array. No bounds check of the colour index is present; valid callers use 0–3.
This operation needs an attached target even when no engine notification is
needed (`0x0082C220`).

Alpha mode, modulation and antialias setters suppress engine notifications when
the stored value already equals the supplied value. Modulation and antialias are
stored as bytes; alpha mode is stored as an integer. Their engine slots are 8,
9 and 10 respectively. These three setters tolerate an absent engine.

For finite ordinary inputs, line thickness is clamped at zero; the raster
thickness is the integer conversion of `(clamped thickness + 0.5)`, with a zero
result replaced by one. The original conversion is `_ftol2` under the caller's
floating-point environment. Stored thickness remains the float, not that integer.
Changed input triggers engine slot 11 if an engine exists. Negative inputs can
therefore trigger a notification even if clamping leaves the old value unchanged.
Instruction evidence: float loads/comparisons at `0x0082C2D1`–`0x0082C30B`,
addition and conversion `0x0082C30F`–`0x0082C31A`, minimum-one rule
`0x0082C31F`–`0x0082C326`. Non-finite values are outside the settled contract.

ClearGradient forwards `(null,0,0)` to engine slot 12; SetGradientTransform forwards
its arguments to the same slot. These calls require an engine. The exact
fixed-point gradient conversion is deferred in GZG-1; it must not be replaced
with the 3D packed-state gradient rules.

## 3. Context transforms, drawing and target changes

### 3.1 Translation and clipping

`Offset` adds an integer pair to the float origin; `OffsetAbsolute` replaces the
origin. Neither alters already stored clip coordinates. `Clip(rect)` truncates
each float origin component using `_ftol2`, translates the supplied integer
rectangle by that pair, and intersects it with the current clip. `ClipAbsolute`
intersects directly without translation; it does not replace the clip.
The engine receives the resulting clip through slot 6 (`+0x18`).
Instruction evidence for origin conversion: `0x0082C6E4`–`0x0082C718`;
absolute intersection and notification: `0x0082C190`–`0x0082C1AA`.

When a blit is active and tiling is disabled, relative Clip and SetBlt also
restrict the engine's effective clip to the intersection with the blit's
supported bounds. The saved context clip remains the separately maintained
clip. `ClipAbsolute` directly notifies its intersection and has no corresponding
blit-bound adjustment. `ClipReset` copies the canvas area into the context clip
**without notifying the engine** (`0x0082BEC0`). A subsequent drawing operation
must not be assumed to repair that omission.

`PushTransform` saves exactly the two origin floats and four clip integers
(24 bytes). `PopTransform` restores that record and notifies the engine of its
restored clip. An empty pop has no effect. Stack depth is the number of saved
records. `PushState` is a separate 16-byte record of the mode bytes, alpha mode,
float line thickness and cached raster thickness; PopState restores antialias,
modulation, alpha and thickness through the setters. It does not restore colour,
gradient, blit, tiling, target or translation. An empty state pop has no effect
(`0x0082C780`, `0x0082C7D0`, `0x0082CFE0`, `0x0082CFF0`).

### 3.2 Geometry and dispatch

`Draw(kind,points,count)` adds the current origin to each float coordinate pair
in temporary storage, leaving the caller's points intact, then invokes
DrawNoTransform. DrawNoTransform does not add origin. Its kinds are:

| Kind | Geometry family |
|---|---|
| 0 | points |
| 1 | independent lines |
| 2 | connected line geometry; detailed endpoint coverage deferred in GZG-1 |
| 3 | rectangles, two points per rectangle |
| 4 | outline convenience geometry; detailed subdivision deferred in GZG-1 |
| 5 | remaining outline/polygon geometry; detailed closure deferred in GZG-1 |
| 6 | filled triangles, three vertices per triangle |
| 7 | filled quads, four vertices per quad |
| 8 | one filled polygon |

Unknown kinds do no drawing in the generic dispatcher (`0x0082FC30`). Filled
primitives first offer the target engine an opportunity to handle them. Engine
result zero means handled; nonzero requests generic fallback. Values 1 and 2
are distinct native results and must be preserved at the engine boundary;
their detailed differences in generic tessellation are GZG-1. The generic
rasterizer emits clipped spans to engine slot 22; span contents and blitter
selection are covered by the linked span specs. Direct filled-primitive
entry points are `0x0082EA20`, `0x0082FB40`, `0x0082FB90`.

Point adds `(0.5,0.5)` to its integer location and submits kind 0 with one point.
Line adds that same half-pixel shift to **both** integer endpoints and submits
kind 1 with two points. Fill submits the rectangle's integer edges converted
to floats, with no half-pixel shift, as kind 3/count 2. These three operations
use Draw and therefore receive the origin. Clear submits the **current clip**
as kind 3/count 2 through DrawNoTransform; Clear is a fill with current drawing
attributes, not an unconditional zeroing of all target bytes. Evidence:
`0x0082BFE0`–`0x0082C0F9`; point's integer loads, float addition and float stores
are verified at `0x0082C023`–`0x0082C046`.

### 3.3 Blit binding and suspension

SetBlt supplies a source plus placement and x/y scaling to engine slot 13. The
placement is translated by the current float origin. The engine returns success
and the bounds supported by that source; success becomes the active-blit byte.
Without tiling, the engine clip is restricted to those bounds. With tiling, the
ordinary context clip is used. ClearBlt sends a null source with zero placement
and unit scales, clears the active byte and restores the engine's context clip.
SetBltTiling sets the tiling byte and calls engine slot 15 with the supplied
tiling parameter. ClearBltTiling clears the byte and calls slot 14; if a blit is
still active its bounded effective clip is re-established. Set/ClearBlt do not
save a restorable source in PushState (`0x0082C370`–`0x0082C4DD`). Detailed
source rectangles and Blt/BltTiled convenience argument semantics are GZG-1.

Suspend increments the nesting count on **every** call, then invokes engine
Flush (slot 16) and Suspend (slot 17) on every call. Resume does nothing at zero;
otherwise it decrements the count and invokes engine Resume (slot 18) only when
the new count reaches zero. IsSuspended tests count greater than zero.
Evidence: `0x0082C4E3`–`0x0082C4F8`, `0x0082C500`–`0x0082C512`.
Flush, Lock and Unlock are engine forwards, to slots 16, 19 and 20 respectively.
Suspension is not a blanket guard on every context drawing entry point.

### 3.4 Attachment, reference ownership and failures

*[compat]* Setting the **same target pointer** is a no-op. Changing it shuts down
and releases the old engine before replacing the retained target (new target
AddRef before old target Release). Setting null detaches and does not clear all
the old drawing coordinates. A non-null attachment establishes origin zero and
clip from the target, asks the draw-engine service (class `0xEB2EA3CB`, IID
`0x4B2EA3B3`) for an engine, and initializes that engine for the target.

The engine then receives clip, all four colours, alpha mode, modulation,
antialias and thickness. Source and gradient are cleared. Engine GetCanvasArea
fills the saved canvas area. Active blit, tiling and suspension count are cleared.
The stacks are not emptied. This means a later PopTransform can restore a clip
saved against a different target (`0x0082C830`–`0x0082C99A`).

The native attachment routine assumes the service and engine exist and continues
to use the engine without a recoverable failure result. IsTargetAttached only
reports a retained target. *[choice]* A standalone core may report failure from
its own fallible attachment API and leave a usable detached context; that is not
an observed guarantee of the GZ method. Before attachment, calls such as
SetColor, clipping, gradient and drawing may dereference missing interfaces.
Do not interpret these as successful no-ops. The export also labels an unrelated
routine at `0x009C40F4` SetTarget; the contract above is for primary slot 17 at
`0x0082C830`.

## 4. Render-to engines and dispatch

### 4.1 Common method inventory

Buffer engine table `0x00AC17D0` and SimGL table `0x00ADB500` have the following
common first 23 slots. These engine slots are **not** the context's slots.

| Slot | Contract | Buffer entry | SimGL entry |
|---|---|---|---|
| 0 | QueryInterface | `0x0082FD60` | `0x0082FD60` |
| 1 | AddRef | `0x005F47F0` | `0x005F47F0` |
| 2 | Release | `0x008306C0` | `0x00A64110` |
| 3 | Init(target) | `0x00830710` | `0x00A643C0` |
| 4 | Shutdown | `0x0082FF20` | `0x00A64130` |
| 5 | GetCanvasArea(out rectangle) | `0x00A64430` | `0x00A64430` |
| 6 | SetClipRect | `0x0082FD90` | `0x00A64440` |
| 7 | SetColor(first,count,array) | `0x0082FF70` | `0x00A64140` |
| 8 | SetAlphaBlend | `0x0082FEA0` | `0x00A64250` |
| 9 | SetBltModulate | `0x0082FEB0` | `0x00A64270` |
| 10 | SetAntialiasing | `0x0082FEC0` | `0x00A64280` |
| 11 | SetLineThickness | `0x00648F10` | `0x00648F10` |
| 12 | SetGradientTransform | `0x0082FDB0` | `0x00A641B0` |
| 13 | SetBlt(source,placement,scales,out bounds) | `0x0082FFD0` | `0x00A64490` |
| 14 | ClearBltTiling | `0x0082FE60` | `0x00A64170` |
| 15 | SetBltTiling | `0x0082FE70` | `0x00A64180` |
| 16 | Flush | `0x00407990` | `0x00407990` |
| 17 | Suspend | `0x008302A0` | `0x00992150` |
| 18 | Resume | `0x008302C0` | `0x009921D0` |
| 19 | Lock / expose target record | `0x0082FED0` | `0x0062C980` |
| 20 | Unlock | `0x00407990` | `0x00407990` |
| 21 | Draw(kind,points,count), result code | `0x00831000` | `0x009922D0` |
| 22 | DrawPreclippedSpanList | `0x008302E0` | `0x006F7DC0` |

The shared QueryInterface identity and hardware-only extension slots need further
audit (GZG-2). Do not infer an IID from the table address. The hardware owner is
a common base; SimGL inherits its initialization, state and reference handling,
with its own device hooks. These owners are not message-server subscribers at
the drawing boundary; lifecycle is driven through the slots above.

### 4.2 Buffer engine

Construction (`0x00830690`) clears source, gradient flag, destination-lock flag
and span-renderer pointer. Init first shuts down the previous binding; queries
target IID `0x20732180` (`cIGZBuffer`); locks it with `0x8040`; selects a
destination-format span renderer; then caches pixel pointer, stride, size,
colour type and pixel width. Query failure returns false. Lock failure releases
and clears the queried buffer and returns false.

| Destination colour type | Span renderer |
|---|---|
| 2 | 8-bit |
| 4, 6 | 1555 |
| 5 | 565 |
| 7 | 4444 |
| 8, 9 | 8888 |
| other, including 1 and 3 | rejected |

The selection, cached record and shutdown details are specified in the linked
span specs (particularly 565 and 8). *[compat, quirk]* Rejection **after a
successful lock** releases the buffer without a matching unlock. Allocation
failure for the four-byte span-renderer object stores a null pointer but Init
still returns success. Later span drawing assumes that pointer is non-null.
Normal explicit Shutdown destroys the span renderer, unlocks with `0x8040` if
marked locked, then releases the buffer. Final Release is not a substitute for
that explicit Shutdown; it can release without unlocking. These are failure
observations, not safe behavior required of a standalone allocator.

Buffer Lock copies six words to the caller: pixels, byte stride, width, height,
colour type, bytes per pixel, and returns true (`0x0082FED0`). This does not
acquire another destination lock. Engine alpha mode is reduced to `mode > 0`;
modulation and antialiasing retain their supplied byte values. Tiling clear
disables a flag; tiling set enables it and copies a four-word record.

When antialiasing is zero, Draw returns **2**, irrespective of primitive kind.
With antialiasing enabled, kinds 6 and 7 handle complete groups of three and four
vertices respectively; incomplete trailing vertices are ignored. Kind 8 handles
one polygon. These handled cases return **0**. All other kinds return **1**.
The antialiased path writes pixels directly through the buffer and bypasses the
span renderer (`0x00831000`, `0x008308D0`); see span-render-8 section 10.5 for its
relationship to normal span drawing. This document does not assert exact edge
coverage for the antialiased path (GZG-2).

### 4.3 Hardware base and SimGL

Hardware Init queries the target as `cIGZBuffer`, caches its width/height, and
calls the hardware initialization hook (slot 24 / `+0x60`). Failed query or failed
hook releases and clears the target and returns false. Success marks render
state dirty and returns true (`0x00A643C0`). Colour changes copy the caller's
packed words. Alpha mode records both `mode > 0` and `mode > 1`, then marks state
dirty. Modulation stores a byte and marks state dirty; antialias stores its byte.
Tiling maintains an enable byte plus four words (`0x00A64140`–`0x00A64280`).

The common hardware Draw handles kinds **3** and **6** through rectangle and
triangle submission. Before either it applies pending state through a device
hook and updates a dirty coordinate transform. Other kinds return **1** for
fallback (`0x00A666C0`). The rectangle and triangle helpers split geometry against
clip and source/tiling bounds; their exact subdivision, gradient rounding and
device vertex values remain GZG-2. No software span blend formula is implied for
device rendering.

SimGL construction (`0x009920E0`) builds that hardware base, clears three device
resource pointers, retains the draw-engine service when available, initializes
its half-pixel offset to `0.5`, and sets the observed control bytes `+0xD5=0`,
`+0xD6=1`, `+0xD7=0`. Its factory probes target IID `0xE3094DC8`; unsupported
targets return false (`0x00993270`). Factory construction is `0x00991E50`.
The drawing context's engine-selection service, rather than callers testing the
framebuffer depth, chooses this implementation.

SimGL Draw has a specialized route only for kind 3, with `+0xD8` enabled and a
non-null source. In that route it treats the vertex count as a count of point
pairs; no complete rectangle returns zero. Entry tests and pair count are
verified in bytes at `0x009922DC`–`0x00992317`. Other inputs reach the later
fallback path, whose exact delegation and specialized texture subdivision remain
GZG-2. Its Draw and HWSetImage pseudocode are absent because export decompilation
failed; no claim of exact device rendering is made here.

## 5. `cGZGraphicSystem`

### 5.1 Identities, inventories and state boundary

Construction is `0x0098FDC0`, service class `0xC416025C`, priority 1,500,000.
QueryInterface (`0x0098DEF0`) exposes IID `0x0073283C` at object `+0x1C`,
IID `0xC47B747C` at `+0x20`, IID `0x03FA40BF` at `+0x24`, and IID `0xE98B2F57`
at `+0x28`; other requests delegate to the system-service base. Each successful
query adds a reference. **Offsets inside a method invoked through an adjusted
interface are relative to that interface pointer**, not automatically to the
complete service. This matters especially for folded getters and the driver
interface's state fields.

Driver/control interface table `0x00ADAFF0`:

| Slot(s) | Operations in order |
|---|---|
| 0–2 | QI, AddRef, Release thunks |
| 3–6 | GetGDriver; driver-count getter; GetGDriverByIndex; GetGDriverByID |
| 7–9 | SetGDriver; SetDefaultDriverClassID; SetDriverSettingsIniFilePath |
| 10–11 | IsScreenSettingAvailable; DowngradeToAvailableScreenSetting |
| 12–16 | SetUIRenderingMode; RenderBufferToScreen; DisableScreenBuffer; GetScreenPixelFormat; LazyFlush |
| 17–20 | folded setter at `0x00648F10` (semantics unresolved); SetForceNoTranslucency; IsTranslucencyEnabled; folded float getter at `0x007624E0` (meaning unresolved) |
| 21–23 | EnableCursorEmulation; EnablePageFlipping; IsPageFlippingEnabled |

Entries for slots 5–16 are respectively `0x0098F0A0`, `0x0098FCB0`,
`0x0098E070`, `0x00990010`, `0x0098AD70`, `0x0098E1F0`, `0x0098E290`,
`0x0098EA30`, `0x0098F110`, `0x0098ACE0`, `0x0098ACF0`, `0x0098AD10`.
Slots 18–19 and 21–23 are `0x0098E020`, `0x0098AD20`, `0x0098E040`,
`0x0098AD30`, `0x00741740`.

The graphics interface table `0x00ADB050` has 65 entries. The following inventory
preserves unnamed operations without assigning speculative public names:

| Slots | Entry facts / identified contract |
|---|---|
| 0–2 | QI/AddRef/Release thunks |
| 3–4 | buffer factory helpers `0x0098E190`, `0x0098E140` |
| 5–9 | `0x007D6680`, `0x00989EB0`, `0x0098E980`, `0x0098E9A0`, `0x0098ED70`; slots 7–9 expose screen-buffer-related interfaces/format |
| 10 | pure virtual in base |
| 11–15 | `0x0098E1D0`, `0x00989EA0`, `0x00648EF0`, `0x007624D0`, `0x0098E120` |
| 16–20 | `0x0098AC70`, `0x0098E5C0`, pure virtual, `0x0098DFA0`, `0x0098DFB0` |
| 21–25 | pure virtual, `0x0098ACC0`, `0x0098ACD0`, pure virtual, `0x00407990` |
| 26–31 | `0x0098E850`, `0x0098E870`, `0x0098E880`, `0x0098E910`, `0x0098DFC0`, `0x0098DFE0` |
| 32–36 | `0x0098AD60`, pure virtual, `0x0098E9E0`, `0x0098E9B0`, `0x0098EA00` |
| 37–42 | `0x0098A040`, `0x00989EC0`, `0x0098A3A0`, `0x0098A1C0`, `0x0098A6D0`, `0x0098A5B0` |
| 43–49 | `0x0098AA60`, `0x0098A870`, `0x0098AB70` twice, `0x0098ABA0`, `0x0098AC30`, `0x0098ACA0` |
| 50–57 | `0x0098EF50`, `0x0098E840` twice, `0x00989E30`, `0x00989E50`, `0x00989E60`, `0x00989E70`, `0x00989E80` |
| 58–64 | `0x004E13E0` three times, `0x00648F10` three times, `0x00407990` |

The buffer/drawing/IO helper signatures and semantics not identified above are
open in GZG-3; this inventory alone is not an implementation contract for those
operations. The concrete W32 subclass overrides base pure slots; it must not be
implemented by calling a base pure stub. Service table `0x00ADB17C` has Init at
slot 8 (`0x00990880`) and Shutdown at slot 9 (`0x0098FCE0`), with ordinary
system-service identity, running state and lifecycle operations around them.

Complete-object construction starts uninitialized, with no framework, driver
collection entries, canvas, screen buffer or engine cache. The constructor sets
`+0x34=1`, bytes `+0x40=0`, `+0x41=1`, `+0x42=0`, the no-translucency flag
`+0xEC=0`, and bytes `+0xFC=1`, `+0xFD=1`; their meanings must be interpreted
through the appropriate adjusted interface, not guessed from these raw defaults.

### 5.2 Startup, shutdown and message dispatch

Init is idempotent once initialized. With no framework and no framework available
globally, it returns false. It loads/configures drivers and screen resources,
creates the SimGL buffer cache, registers itself with the framework, and
subscribes to message `0x8ACAA95F` through the message-server-2 service. It also
gets canvas service class `0xA3460DEB`, IID `0x66B217A1`; a missing canvas makes
initialization unsuccessful. Exact driver preference/INI resolution and rollback
after each intermediate failure remain GZG-3 (`0x00990880`, LoadAllDrivers
`0x009904D0`, CallbackDriverLoad `0x00990210`).

Shutdown is idempotent and returns true. When initialized it clears that state,
releases the retained canvas, unsubscribes `0x8ACAA95F`, removes its canvas
message handler for `0x1A10630A`, shuts down/releases the buffer cache, frees all
drivers and removes its framework registrations (`0x0098FCE0`). Objects retained
by callers still require normal reference management; Shutdown is not deletion.

Message-server-2 target table `0x00ADAFA0` has QI/AddRef/Release at slots 0–2 and
DoMessage at slot 3 (`0x0098ED10`). Message `0x8ACAA95F` obtains class
`0xEB2EA3CB`, IID `0x4B2EA3B3`, and invokes its slot 9 (`+0x24`) when present.
Other message types are ignored; the handler returns true in either case.
The callback at `0x0098EC80` interprets canvas-event IID `0xDA1062FA`: event
subtypes 0, 1 and 7 invoke the registered graphics listener's slots `+0x108`,
`+0x10C` and `+0x110`. Unsupported subtypes cause no such call. Obtained message
interfaces are released. Exact Windows procedure translation is GZG-4.
OnTick (`0x0098DF90`) delegates to a graphics-service helper; no independent
simulation timing contract is established by that method.

### 5.3 Driver selection and screen contracts

SetGDriver with the current pointer does nothing. Changing it releases the old
driver, clears the related driver flags, retains the new driver and requests
mode enumeration/setup when its enumerated mode count is zero. It also notifies
the screen-related object when one is available (`0x0098E070`). Driver getters
must not be presumed to transfer ownership; callers needing ownership query or
retain the returned interface.

Screen setting is `(width,height,bits)`. Availability requires an enumerated
mode matching those three values and the service's current mode discriminator.
*[quirk]* A nonzero driver ID is looked up, but the enumerated mode calls still
use the active driver in the routine inspected (`0x0098E1F0`). A missing selected
driver returns false; exact handling of an inactive/missing active driver is
not promised safe.

Downgrade tries a reduced candidate **before** checking availability. Depth above
16 becomes 16 first. Otherwise it chooses the next size from this descending
pixel-area ladder, using strict greater-than comparisons against each size's
area: 2048×1536, 1600×1200, 1280×1024, 1024×768, 800×600, 640×480,
320×240. A candidate is the largest ladder size with area strictly less than
the input's area; if none exists it returns false. It tests availability and
continues reducing until a match or exhaustion. It mutates the caller's setting
on unsuccessful attempts too (`0x0098E290`). The area is a native integer
product; overflow cases are outside the settled contract.

SetForceNoTranslucency stores the supplied byte and the accompanying word.
IsTranslucencyEnabled returns true exactly when that byte is zero; it does not
interrogate device capability (`0x0098E020`–`0x0098E036`,
`0x0098AD20`–`0x0098AD2D`). LazyFlush sets a pending byte rather than performing
an immediate flush. DisableScreenBuffer disables its flag. Exact presentation
semantics, copy regions, page flipping and hardware-versus-software fallback in
RenderBufferToScreen remain GZG-3. SetUIRenderingMode configures driver render
state for 2D and a projection cached on its first setup; exact matrix, state
ordering and viewport-change implications are also GZG-3. Do not reuse the 3D
helper's default-state method as a substitute.

## 6. `cGZAlphaBlt`

### 6.1 Identity, defaults and complete operation inventory

This is a system-service wrapper around the standalone alpha/blit operations,
not a per-target drawing context. Construction allocates 32 bytes, initializes
service class `0x44136FDC` with priority `0xFFF0BDC0`, and installs its interface
at complete object `+0x1C` (`0x009895A0`). QI for IID `0xE3260321` returns that
interface with an added reference; other requests delegate to the service base
(`0x00985D00`). No independent source, destination, clipping or blend state is
stored by these forwarding methods. There is no owner-specific message dispatch
in the examined interface.

Alpha interface table `0x00ADA6F0`:

| Slot(s) | Operation(s) in order |
|---|---|
| 0–2 | QI, AddRef, Release thunks |
| 3–7 | GenericBufferClip; InitAlphaSystem; GetAlphaLevelCount; AlphaSurfaceFromFile; MaskSurfaceFromFile |
| 8–9 | AlphaBlt; AlphaBltNoClip |
| 10–11 | AlphaBltConstAlpha; AlphaBltConstAlphaNoClip |
| 12–13 | AlphaBltConstSource; AlphaBltConstSourceNoClip |
| 14–15 | ShadeBltConstAlpha; ShadeBltConstAlphaNoClip |
| 16–17 | ShadowBlitConstAlpha; ShadowBlitConstAlphaNoClip |
| 18–19 | BrightBlitConstAlpha; BrightBlitConstAlphaNoClip |
| 20–21 | ShadeBlt; ShadeBltNoClip |
| 22–23 | ShadeBltStretch; ShadeBltStretchNoClip |
| 24–25 | ShadeBltWithAdditionalAlpha; ShadeBltWithAdditionalAlphaNoClip |
| 26–27 | ShadeBltStretchWithAdditionalAlpha; ShadeBltStretchWithAdditionalAlphaNoClip |
| 28–29 | AlphaBltSelfAlpha; AlphaBltSelfAlphaNoClip |
| 30–31 | AlphaBltHalfAlphaVariableSource1ConstSource2; corresponding NoClip |

Wrapper entry ranges: `0x00988BE0`–`0x00988EA0` and
`0x009895F0`–`0x00989800`. Slot 14 starts at `0x00989680`; a decompiler label
at `0x0098968A` is inside that wrapper, not its callable entry.

*[compat]* GetAlphaLevelCount returns **32** (`0x00988C00`). InitAlphaSystem
passes its arguments to the shared initialization helper. Every drawing method
forwards to its corresponding standalone operation without applying a context
origin, selecting a new target engine, or saving/restoring context attributes.
The clipped and NoClip variants are distinct entry points; a caller choosing
NoClip must already satisfy that operation's bounds requirements. ConstAlpha,
ConstSource, additional-alpha and self-alpha inputs must not be collapsed into
one interpretation merely because their names resemble span-renderer modes.

### 6.2 Surface IO and failure boundary

AlphaSurfaceFromFile and MaskSurfaceFromFile are legacy file loaders returning
a buffer pointer, with null on file-open failure. They create class
`0xC470D325` as IID `0x20732180`. The loaders expect a legacy indexed BMP-like
layout: seek to byte 14, read `0x428` bytes of DIB header plus palette, obtain
width/height there, and read padded rows. They construct colour type 2 output,
reverse bottom-up source rows when copying into the locked output, and unlock
after a successful destination lock (`0x009888E0`, `0x00988A80`). They do not use
the image decoder selected by the draw context or any 3D texture loader.

These routines are not evidence for acceptance of arbitrary BMP variants,
negative-height images or truncated files. Exact alpha quantization, mask byte
interpretation, argument widths and allocation/short-read failure behaviour
are GZG-5. Implementers must not infer them from the unreliable pointer/local
types in exported pseudocode. Likewise the underlying pixel formulas and
GenericBufferClip mutation contract remain GZG-5. Until settled, the 32-level
count is a service fact, not proof of a particular divisor or endpoint rounding
rule for every alpha method.

## 7. `cGZCanvasW32`

### 7.1 Identity and method inventory

The canvas is the Windows host/event adapter. Primary IID `0x66B217A1` returns
the complete pointer, IID `0xE46397DB` returns `+4`, IID `0x3AC7A210` returns
`+8`; remaining requests delegate to its embedded system-service base.
Success adds a reference (`0x0098B7A0`). Its service subobject is at `+0x14`,
class `0xA3460DEB`, priority `0x1312CF`. The primary table `0x00ADAD88` ends
after slot 30; the following bytes are a string, not additional methods.

| Slot(s) | Contract / entry facts |
|---|---|
| 0–2 | QI, AddRef, Release (`0x0098B7A0`, `0x0098B820`, `0x0098B830`) |
| 3 | store canvas type and request initial `(0,0,width,height)` bounds, `0x0098BCC0` |
| 4 | canvas realization/configuration, `0x0098DD70` (GZG-4) |
| 5 | folded getter `0x00680920` (GZG-4) |
| 6 | Shutdown, `0x0098DDB0` |
| 7–8 | listener attach/remove helpers, `0x0098DC40`, `0x0098DCB0` (GZG-4) |
| 9–10 | set four bounds; copy saved bounds, `0x0098BD80`, `0x0098CAB0` |
| 11–12 | show and focus; show, `0x0098BA30`, `0x0098BA50` |
| 13–14 | set title; forward to slot 15, `0x0098B9B0`, `0x0098CAE0` |
| 15 | host event pump, `0x0098C690` (GZG-4) |
| 16–17 | folded entries `0x008265C0`, `0x0068D1B0` (GZG-4) |
| 18 | host-window procedure bridge, `0x0098CDB0` (GZG-4) |
| 19 | post a packaged host message, `0x0098BA60` |
| 20 | folded entry `0x00578900` (GZG-4) |
| 21–22 | host-window helpers `0x0098B850`, `0x0098B8E0` (GZG-4) |
| 23–24 | width, height getters `0x0098CC30`, `0x0098CC40` |
| 25–28 | folded entries `0x0068D1C0`, `0x0081F910`, `0x006AAEB0`, `0x0040D390` (GZG-4) |
| 29–30 | input-channel enabled query; nested enable/disable, `0x0098BAC0`, `0x0098BDC0` |

The embedded service table `0x00ADAD08` has Init at slot 8 (`0x0098BAF0`)
and Shutdown at slot 9 (adjusting thunk `0x0098DEE0`). The ordinary service and
primary canvas pointers are not interchangeable. Other subinterfaces supply
message/listener plumbing; their exact thunk mapping is GZG-4.

### 7.2 Defaults and lifecycle

Construction (`0x0098C7B0`) starts with no host handle, no active initialization,
empty stored title, zero saved bounds, wheel-message ID `0xFFFFFFFF`, canvas type
`0x1B`, and two zero input-disable counters. It constructs reusable canvas,
mouse and key message objects and acquires message-server services.
This does not itself create a usable drawing target.

Service Init obtains the user's locale ANSI code page; zero falls back to
`0x10`. It acquires the graphics service and message-server-2, initializes from
the graphics service's screen setting through the primary canvas interface,
and emits canvas event IID `0xDA1062FA`, subtype **0**, on successful initialization.
Missing graphics/message-server-2 or failed canvas initialization returns false
(`0x0098BAF0`). The code-page field is relative to the service pointer in this
routine; do not combine its exported field offsets with complete-object offsets.

Windows realization (`0x0098D9C0`) coordinates the graphics driver window/mode,
and registers `MSWHEEL_ROLLMSG`. Registration failure stores `0xFFFFFFFF`.
The windowed/driver-owned paths emit canvas-event subtypes **8** and **9**;
the exact conditions and HWND ownership are GZG-4. These native host resources
are *[compat-limit]*: other operating systems may use their own window system
while preserving the canvas/listener contract.

Shutdown acts only when initialized, emits subtype **1**, invokes the canvas's
window cleanup, releases retained services and reusable messages, restores both
disabled input channels by balancing their counters, and clears initialization.
It returns true, including when already shut down (`0x0098DDB0`).

### 7.3 Bounds, title, messages and input

Set bounds saves `(left,top,right,bottom)` and, when a host handle exists, calls
the host positioning operation with width `right-left`, height `bottom-top`
and flags `0x14`. Get bounds copies the saved values rather than querying the
OS. Width/height getters return those differences; verified integer loads and
subtraction at `0x0098CC30`–`0x0098CC46`.

Set title frees the old saved title. A nonempty input is copied into a new
zero-terminated buffer and applied to the host window if present. An empty
input clears stored title but does **not** call the host title setter, so a
previously displayed title may remain (`0x0098B9B0`). Show-and-focus invokes
ShowWindow with command 1 then SetFocus; the other show method invokes only
ShowWindow with that same command (`0x0098BA30`, `0x0098BA50`).

Packaged message posting accepts a non-null package only if its type getter
returns `0x8006` and its element count is greater than two. It posts the first
three supplied words as Windows message ID, wParam and lParam to the saved
handle and returns whether PostMessage succeeds. Unsupported package shape
returns false (`0x0098BA60`). It does not synchronously invoke the window
procedure.

Channel query accepts selectors **1** and **2**, returning whether the respective
disable count is zero. Other selectors return false. Disable with a bitmask
increments each selected channel's count. Enable decrements a positive count
but never underflows it. Channel 2 additionally notifies message-server-2 on
transitions into and out of its disabled state; nested calls do not repeat that
notification. A combined mask affects both channels independently
(`0x0098BAC0`, `0x0098BDC0`).

DoMessage (`0x0098BD00`) returns true for all input messages. For type
`0xFA0C6390`, it queries IID `0x1A0C615F`; subtype **13** updates two stored
coordinate words from that message. Other types, failed QI and other subtypes
do not update those words. The acquired interface is released. Exact payload
coordinate names and translation into mouse/key/canvas events are GZG-4.

## 8. Conformance vectors

These are declarative vectors for future core tests; no engine files are added
by this RE lane. A recording target/engine/service can check call ordering and
reference events without rendering a game frame. Expected results below are
specified behaviour, **not measured game-run results**. The linked span specs
supply pixel-result vectors for their kernels. Set up an attached context unless
a vector explicitly says otherwise.

| ID | Setup and action | Expected result |
|---|---|---|
| GZG-V01 | Context QI for unknown IID with output sentinel | false; sentinel unchanged |
| GZG-V02 | Intersect `(0,0,10,10)` with `(10,2,20,8)` | `(0,0,0,0)`, empty |
| GZG-V03 | Attach target with area `(0,0,100,80)` | origin zero; clip copied; engine initialized/notified; no active source/tiling/suspension |
| GZG-V04 | Reattach identical target pointer | no shutdown, AddRef, Release or Init |
| GZG-V05 | PushTransform; Offset `(5,7)`; Clip; PopTransform twice | first pop restores saved origin/clip and notifies once; second pop has no effect |
| GZG-V06 | Clip `(2,3,9,10)` at origin `(5,7)`, initial clip `(0,0,20,20)` | context clip `(7,10,14,17)` |
| GZG-V07 | Same input via ClipAbsolute | clip `(2,3,9,10)` |
| GZG-V08 | ClipReset after smaller clip | context clip becomes canvas area; no engine SetClip notification |
| GZG-V09 | PushState; change colour, origin, alpha, AA, thickness; PopState | alpha/AA/thickness restored; changed colour and origin retained |
| GZG-V10 | ResetToDefaultState after setting origin and colour | origin and colour retained; alpha/modulation/AA zero; thickness one; blit/tiling/gradient cleared |
| GZG-V11 | SetAA(1) twice with recording engine | one slot-10 notification |
| GZG-V12 | SetLineThickness(0), then SetLineThickness(1.6), default FP environment | float/raster pairs `(0,1)`, `(1.6,2)` |
| GZG-V13 | Point(2,3), origin `(10,20)` | DrawNoTransform receives kind 0/count 1, `(12.5,23.5)` |
| GZG-V14 | Fill `(2,3,6,8)`, origin `(10,20)` | kind 3/count 2, `(12,23),(16,28)` |
| GZG-V15 | Clear at nonzero origin with clip `(1,2,7,9)` | kind 3 uses `(1,2),(7,9)`, without origin translation |
| GZG-V16 | Suspend twice, Resume three times | two Flush/Suspend pairs; one engine Resume on second Resume; count ends zero |
| GZG-V17 | SetBlt success with bounded source, enable tiling, clear tiling | engine clip restricted, then ordinary clip, then restricted again |
| GZG-V18 | Buffer Init target fails buffer QI | false, no destination lock |
| GZG-V19 | Buffer Init target QI succeeds, Lock fails | false; queried buffer released and cleared |
| GZG-V20 | Buffer Init with type 5; explicit Shutdown | 565 selected; destination Lock/Unlock both use `0x8040`; retained target released |
| GZG-V21 | Successful lock, unsupported type 3 | false; queried buffer released; native trace contains no unlock |
| GZG-V22 | Span-renderer allocation fails after supported-format lock | native Init returns true, null renderer; do not execute a native draw with this fixture |
| GZG-V23 | Buffer Draw kind 6/count 7, AA zero | result 2, no AA polygon calls |
| GZG-V24 | Same kind/count with AA enabled | two 3-vertex AA polygons, trailing vertex ignored, result 0 |
| GZG-V25 | Buffer Draw kind 3, AA enabled | result 1, generic fallback requested |
| GZG-V26 | Hardware Init QI succeeds but hardware hook fails | false; target released and cleared |
| GZG-V27 | Hardware alpha mode 0,1,2 | state predicates `(false,false)`, `(true,false)`, `(true,true)`; dirty each time |
| GZG-V28 | Graphics receives unrelated message then `0x8ACAA95F` | both return true; only latter calls draw-engine service slot 9 if available |
| GZG-V29 | SetForceNoTranslucency(1,word), query; then set byte 0 | false then true, independent of driver capabilities |
| GZG-V30 | Downgrade `(1024,768,32)` with matching 16-bit active-driver mode | success and `(1024,768,16)` |
| GZG-V31 | Downgrade `(1024,768,16)` with 800×600 available | success and `(800,600,16)`; original mode is not tested first |
| GZG-V32 | Downgrade `(320,240,16)` | false; no smaller ladder entry |
| GZG-V33 | Alpha service GetAlphaLevelCount | 32 |
| GZG-V34 | Alpha service drawing method through recording delegate | matching standalone operation, unchanged arguments, no context-state calls |
| GZG-V35 | Alpha/mask surface filename cannot be opened | null result |
| GZG-V36 | Canvas bounds `(10,20,110,70)` | saved bounds identical; width 100, height 50 |
| GZG-V37 | Nonempty title then empty title, live HWND | only nonempty title sent to OS; saved title cleared afterward |
| GZG-V38 | Disable channel 2 twice, enable three times | counts 1,2,1,0,0; transition notifications only at first disable and second enable |
| GZG-V39 | Query canvas input selector 3 | false, not the conjunction of selectors 1 and 2 |
| GZG-V40 | Packaged message wrong type or count 2 | false, no PostMessage |
| GZG-V41 | Packaged type `0x8006`, count 3; host PostMessage succeeds | first three words posted unchanged; true |
| GZG-V42 | Unrecognized canvas DoMessage | true; no stored-coordinate mutation |

## 9. Evidence limits and open questions

The export was queried before reading matching decompilation. Native table
entries were checked against the executable's read-only table bytes where the
export did not expose an owner-named table. Numeric rules explicitly described
above use instruction evidence where decompiler types/calls are misleading;
in particular several `_ftol2` conversions are incorrectly labelled as Lua calls
in the pseudocode. No REA session was used: this lane was instructed that none
had a binary open.

This spec settles the interface-level context, engine selection/initialization
boundary, buffer dispatch and host/service rules listed above. It does **not**
claim exact generic geometry coverage, SimGL vertex emission, all graphics
service IO helpers, Windows event translation or standalone alpha pixel kernels.
Those are explicit follow-up questions, not free design choices. They are filed
as GZG-1 through GZG-5 in
[questions/gz-graphics.md](questions/gz-graphics.md). An engine lane can implement
the settled contracts and linked software kernels while leaving those unresolved
operations outside any replacement/compatibility claim.
