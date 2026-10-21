# Spec: the hardware 3D driver (`nSGLDX7::cGDriver`, "Maxis 3D GDriver DirectX")

Status: **implementation spec (partial)**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program. Implementers must work from this spec, the public
LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled code
(see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tags: *[compat]* required for compatibility; *[choice]* our choice
where the original was arbitrary; *[inferred]* consistent with all evidence but not proven; *[quirk]* surprising
original behaviour kept on purpose. Addresses are SimCity 4 Deluxe 1.1.641 (GOG) virtual addresses, cited only so
the analysis team can re-check a claim.

Related specs: [view3d.md](view3d.md) (the renderer `cSC43DRender`, the main client of this driver, and its
handling of the driver's messages, §3.4), [foundations.md](foundations.md) (messages §5, x87 precision §18).

## 1. Scope and owners

The game draws through an abstract "graphics driver" interface with an OpenGL-like vocabulary (matrix modes,
`Enable`/`Disable` capabilities, texture names, `DrawArrays`/`DrawElements`). Three classes implement the same
83-slot interface table; this spec covers **only the DirectX 7 implementation**:

| Interface table | Implementation | Covered here |
|---|---|---|
| `0x00AC4660` | `nSGLDX7::cGDriver` (DirectX 7 hardware driver; slots named in `docs/abi/vtables.tsv`) | yes, §2-§8 |
| `0x00AC36D0` | an OpenGL driver (slot bodies at `0x0087C330`-`0x0087ED20` call the `gl*` imports) *[inferred]*; the export files some of its bodies (`GetBoolean 0x0087CEE0`, `GetFloat 0x0087CF20`, `QueryInterface 0x0087D670`, `VertexFormatElementOffset 0x0087C4C0`) under `nSGLDX7::cGDriver` | no |
| `0x00ACC4D8` | a third driver (bodies `0x008DA8C0`-`0x008E1140`, near `nRZSimGL`) *[inferred: software / null driver]* | no |

Owner patterns covered: `nSGLDX7::cGDriver::*` bodies in `0x00881DF0`-`0x00889260` (the device, window, state and
scene lifecycle), `nSGLDX7::TranslateColorToD3D` (`0x00881AB0`). Named but **not yet specified** (open, §9):
`nSGLDX7::cTextureManager` (`0x0088A380`-`0x0088B2C0`), `nSGLDX7::cPrimitiveManager` (`0x00889B60`-`0x0088A2B0`),
`cTextureStageData`, `cVertexBuffer`, `D3DLight`, `nRZSimGL` (vertex-format helpers and colour-key blitters),
`cGZBufferSimGLCache`, `cSC4GDriversCOMDirector`, and the texture / draw / blit / video-mode slots.

The driver object exposes four COM-style interface pointers at object offsets `+0x0`, `+0x4`, `+0x8` and `+0xC`
plus a reference-counted sub-object at `+0x10`; the 83-slot graphics interface is the one at `+0xC`. Slots 0-2
are the usual `QueryInterface`/`AddRef`/`Release`. Slot 78 `GetGZCLSID`, slot 77 `GetDriverInfo`.

## 2. Object state at construction (`0x00885F90`)

A new driver owns one texture manager (object of 0x348 bytes) and one primitive manager (0x40 bytes), created in
the constructor. Everything else starts empty: no window, no DirectDraw / Direct3D / device objects, no current
video mode, error code 0, "initialised" false, "device lost" false, "scene open" false. Four global per-stage
scale factors (`0x00B542F8`, `0x00B5430C`, `0x00B54320`, `0x00B54334`) are set to 1.0. *[inferred]* the 13-dword
current-mode record (§4) starts with mode index -1.

## 3. The error channel (`GetError`, slot 50)

The driver keeps one sticky error code. `GetError` returns it and resets it to 0 (`0x00882000`). Codes observed:

| Code | Set by |
|---|---|
| 0 | none / cleared by `GetError`, by `BeginScene` before it calls the device, by a successful `Disable` |
| 1 | a device call failed with any failure other than "surface lost" (`0x887601C2`). Every such site writes `code = (hr != 0x887601C2) ? 1 : 0`, so a lost surface **clears** the code *[quirk]* |
| 2 | `SetVideoMode` with a mode index ≥ the number of modes |
| 3 | `StencilFunc`, `StencilMask` or `StencilOp` when the current mode has no stencil buffer |
| 6 | window-class registration failed in `Init`, DirectDraw could not be created, or the window / 3D environment for `SetVideoMode` could not be built |

Codes are overwritten, not accumulated: the last failing call wins.

## 4. Lifecycle

### 4.1 `Init` (slot 79, `0x00881DF0`) and `Shutdown` (slot 80, `0x00881E40`)

`Init` first clears the error code. If the driver is already initialised it does nothing more. Otherwise it marks
it initialised, forgets any window handles, resets one word to 7 (object `+0x138`, purpose open, RD2) and sets
the current mode index to -1, then sets the mode count to 0 and:

1. registers the window class `"GDriverClass (DirectX)"` (redraw-on-resize style, arrow cursor, the driver's own
   window procedure) unless it already exists; on failure the error code becomes 6;
2. creates a DirectDraw object; on failure it shows the message box "Could not initialize Direct Draw." titled
   "Hardware Driver Error" and sets error 6;
3. builds the driver-information text: `"Maxis 3D GDriver DirectX 2.0"`, a newline, the adapter's description,
   a line `"%u.%u.%u.%u, GUID: ..."` with the driver version and device GUID, then
   `"Vendor: %04x, Device: %04x, Board: %04x, Chipset: %04x"`. If the reported driver version is 0.0.0.0 it
   is looked up another way first (`0x00888820`, unspecified). `GetDriverInfo` (slot 77) returns this text
   *[inferred]*;
4. releases the temporary DirectDraw object.

`Shutdown` does nothing unless initialised; otherwise it clears the flag and: switches to "no mode" (slot 66 with
-1, §4.2), updates the error code through a helper (`0x00886FB0`, open), sets the mode count to 0, unregisters the window class, and releases every held
DirectX object (the six interface slots at object `+0x218`..`+0x230`).

### 4.2 `SetVideoMode(mode, windowProc, show, flag)` (slot 66, `0x00888F50`)

1. If a scene is open, it is ended (§4.4 rule for errors) and the "scene open" flag cleared.
2. `mode == -1`: if a mode is current and a window exists, the 3D environment is destroyed (§4.5), the window is
   destroyed and the current mode becomes -1. Return.
3. Otherwise the fourth argument is stored (a per-mode flag, purpose open, RD1). If `mode` ≥ the mode count the error
   becomes 2 and the call returns without change.
4. If a mode is current, the driver first calls its own slot 66 with `(-1, 0, 0, 0)`.
5. The window procedure is remembered (`DefWindowProcA` when null). The driver fills its current-mode record from
   `GetVideoModeInfo(mode)` (slot 65). The record holds at least: mode index, width, height, a "full screen" flag
   and a "has stencil" flag.
6. Window: full-screen modes get a popup window of exactly width × height at (0,0); windowed modes get an
   overlapped, visible window whose outer size is computed so that the *client* area is width × height. Window
   class `"GDriverClass (DirectX)"`, title `"GDriverWindow (DirectX)"`. The window's user data points to the
   driver. The window is then positioned at (0,0) with that size.
7. The 3D environment is set up (§4.3). If that fails, it is destroyed again and the window too.
8. If no window remains: error 6, current mode index -1. Else, if `show` is non-zero, the window is shown and
   updated.

### 4.3 Setting up the 3D environment (`SetupViewport 0x00888E10`, `Initialize3DEnvironment 0x00887A10`)

1. Any open scene is ended.
2. The DirectX objects are created through a helper (`SetupD3DX 0x008876C0`). Failure: message box "Could not
   initialize graphics card." ("Hardware Driver Error"), error per §3, set-up fails.
3. If the device capabilities lack the hardware-transform-and-lighting-class bit (`0x80000` of the first device
   capability word) the set-up fails with the message box "Driver does not support DirectX. This may mean you have
   a Microsoft driver installed instead of the vendor driver..." *[compat: the check and the text]*.
4. If the helper produced no context object: message box "Internal Error.", set-up fails.
5. The device's viewport rectangle is read and kept as (x, y, x+width, y+height).
6. Initial transforms, all through the driver's own interface: model-view (matrix mode 0) := identity; projection
   (matrix mode 1) := the pixel-space orthographic matrix below; matrix mode back to 0; then `Clear(0x7000)`
   (colour, depth and stencil, §5.3).
7. Device properties are read; on failure set-up fails. The texture manager and the primitive manager are
   initialised against the new device, the default device state is applied (`SetDefaultD3DDeviceState
   0x008872A0`, contents open, RD2), the window's client rectangle is stored in screen coordinates, and the
   "environment ready" flag is set.

The initial projection (row-major, row-vector convention, viewport width `W` and height `H` converted from
unsigned integers to single floats):

| | col 0 | col 1 | col 2 | col 3 |
|---|---|---|---|---|
| row 0 | 2/W | 0 | 0 | 0 |
| row 1 | 0 | -2/H | 0 | 0 |
| row 2 | 0 | 0 | -1 | 0 |
| row 3 | -1 | +1 | 0 | 1 |

So pixel (x, y) maps to clip (2x/W - 1, 1 - 2y/H): (0,0) is the top-left corner. Worked example: W=800, H=600,
pixel (400, 150) → (0.0, 0.5). The quotients are single-precision (`2.0f / W`, constants `0x00A8825C` = 2.0,
`0x00ABD2D4` = -2.0).

### 4.4 Scenes and frames (`BeginScene 0x008871C0`, `Flush` slot 43 `0x00884BA0`)

There is no public "begin frame" slot: a scene is opened by the environment set-up path and by `Flush`.
`BeginScene` does nothing if a scene is already open or there is no device. Otherwise it clears the error code
and begins a device scene; the scene counts as open on success and on one specific failure code (`0x887602F8`),
otherwise not, and the error code follows §3. It then resets every texture-stage record to its default
(§5.4) and, if the "clear colour on begin" flag is set, calls `Clear(0x4000)`.

`Flush` is the frame boundary:

* **Device available, not lost**: the primitive manager's queued geometry is submitted (error per §3), the scene
  is ended and the frame presented (`0x00887260`, details open), the context's per-frame update is called, the
  vertex buffer is recreated if it was lost (`0x00889260`), and a new scene is begun.
* **Device just lost** (the cooperative-level test fails and the driver was not already lost): the driver marks
  itself lost, posts message `0xAACB92A3` (*[unlisted]*) through the message server, drops its texture surfaces
  (`0x0088A400`), stops the primitive manager, swaps in a placeholder device object and releases the real
  DirectX objects. `IsDeviceReady` (slot 81) now returns false.
* **Lost, cooperative level restored**: the driver asks its context to restore the surfaces; on failure it ends the
  frame and tries again next `Flush`. On success it clears "lost", re-acquires the device objects, re-applies the
  default device state, recreates the vertex buffer if needed, re-initialises the texture manager for the new
  device, then posts `0x8ACAA95F` (`kSGLMessage_RestoreTextures`) and `0x6ACB92A9` (the renderer's "force full
  redraw", view3d.md §3.4). Clients must re-upload texture contents on `kSGLMessage_RestoreTextures`.

### 4.5 Tearing down (`DestroyViewport 0x008881D0`, `Cleanup3DEnvironment 0x008870B0`)

Ends an open scene; unbinds the texture of every stage in use; empties the stage-record list; shuts down the texture
manager and the primitive manager; releases the context, the device, Direct3D and DirectDraw objects; clears
"environment ready".

## 5. Render-state slots

Every state slot below first **submits the primitive manager's queued geometry** (so state changes take effect
between draws in call order) and then sets one or more device render states. Enumerations are small integers
chosen by the interface; the tables map them.

### 5.1 Comparison, blend, stencil and shading enumerations

| Interface value | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| compare (table `0x00AC44EC`) | never | less | equal | less-equal | greater | not-equal | greater-equal | always | | | |
| blend factor (`0x00AC452C`) | zero | one | src colour | 1-src colour | src alpha | 1-src alpha | dst alpha | 1-dst alpha | dst colour | 1-dst colour | src alpha saturate |
| stencil op (`0x00AC450C`) | zero | keep | replace | increment-saturate | decrement-saturate | invert | | | | | |
| shade model (`0x00AC4524`) | flat | smooth (Gouraud) | | | | | | | | | |

(The tables hold the D3D7 enumeration values 1..8, 1..11, {2,1,3,4,5,6} and {1,2}; names are the public D3D7
meanings of those values.) Values outside a table read the following bytes *[quirk; callers must not pass them]*.

### 5.2 State slots

| Slot | Call | Effect |
|---|---|---|
| 16 | `DepthFunc(f)` | depth compare := compare[f] |
| 15 | `ColorMask(...)` | **only flushes; the mask is ignored** *[quirk]* |
| 17 | `DepthMask(b)` | depth writes on/off (the byte as given) |
| 18 | `StencilFunc(f, ref, mask)` | needs a stencil buffer (else error 3, nothing else): stencil read mask := mask, stencil compare := compare[f], stencil reference := ref |
| 19 | `StencilMask(m)` | needs stencil (else error 3): stencil **write** mask := m |
| 20 | `StencilOp(fail, zfail, pass)` | needs stencil (else error 3): the three stencil operations via the stencil-op table |
| 21 | `BlendFunc(src, dst)` | source and destination blend factors via the blend table |
| 22 | `AlphaFunc(f, ref)` | alpha compare := compare[f]; alpha reference := `q(ref)` (§5.5) |
| 23 | `ShadeModel(m)` | shade mode via the shade table |
| 67 | `PolygonOffset(v)` | the device depth-bias state := `v` unchanged (an integer bias) |
| 12 | `ClearColor(r, g, b, a)` | stores `TranslateColor(r,g,b,a)` (§5.5) as the clear colour |
| 13 | `ClearDepth(d)` | stores `d` (a double) rounded to single as the clear depth |
| 14 | `ClearStencil(s)` | stores `s` as the clear stencil value |

Failures of the stencil and polygon-offset calls set the error code per §3; the other slots ignore device failures.

### 5.3 `Clear(mask)` (slot 11, `0x00882C80`)

Flushes the queue, then clears the whole viewport. Mask bits: `0x4000` colour, `0x1000` depth, `0x2000` stencil
(stencil is silently dropped when the mode has no stencil buffer). Other bits are ignored. The values are the ones
stored by slots 12-14. Example: `Clear(0x7000)` on a mode without stencil clears colour and depth only.

### 5.4 Capabilities: `Enable` (44), `Disable` (45), `IsEnabled` (46)

| Cap | Meaning | `Enable` / `Disable` | `IsEnabled` |
|---|---|---|---|
| 0 | alpha test | flush; alpha-test state on / off | device state == on |
| 1 | depth test | flush; depth-buffer state on / off | device state == on |
| 2 | stencil test | flush; stencil state on / off | device state == on |
| 3 | face culling | flush; cull mode := clockwise / none | cull mode ≠ none |
| 4 | blending | flush; alpha-blend state on / off | device state == on |
| 5 | texturing on the **current texture stage** | no flush; sets the stage record's "enabled" byte to 1 / 0 and marks the record and the stage set dirty (applied at draw time) | the stage record's "enabled" byte |
| 6 | fog | only when the device reports the fog capability (bit 4 of a capability word): flush; fog on / off. Otherwise ignored | reads device state 0, *not* the fog state *[quirk: result meaningless]* |
| 7 | (accepted, ignored) | nothing | always true |

`IsEnabled` returns false for every cap while the device is lost. `Disable` of caps 0-4 sets the error code to 1
when the device call fails with anything but "surface lost", and to 0 otherwise; `Enable` never touches the
error code. Caps above 7 index past the table *[quirk; callers must not pass them]*.

### 5.5 Colour and alpha quantisation (`TranslateColorToD3D 0x00881AB0`)

A float channel `x` becomes a byte by `q(x)`: compute `s = x + 49152.0` in **single precision** (round to nearest
even), take `v = bits(s) - 0x47400000` (that is `round(x·256)`), then `q = (v - (v >> 8)) & 0xFF`. So 1.0 → 255,
0.5 → 128, 0.0 → 0, 0.998 → 255 (v = 255.488 rounded to 255; 255 - 0). Inputs outside [0, 1] wrap modulo 256.
`TranslateColor(r, g, b, a)` packs `q(a)<<24 | q(r)<<16 | q(g)<<8 | q(b)`.

## 6. Video-mode and viewport slots (outline)

Slots 63 `CountVideoModes`, 64/65 `GetVideoModeInfo`, 74/75 `SetViewport`, 76 `GetViewport` are named in the ABI
table; their contracts are open (RD3).

## 9. Open contracts

See [questions/renderer-driver.md](questions/renderer-driver.md).
