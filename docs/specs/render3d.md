# `cSC43DRender`: owner behaviour beyond the draw pass (behaviour spec)

Scope: the exact owner `cSC43DRender` (130 owner functions, `0x007C1890`–`0x007CB530` plus folded bodies
`0x004E13E0`, `0x004E82A0`, `0x004F7BA0`, `0x007D30D0`, `0x007D32D0`, `0x007FF0B0`). Not `cSC4View3DWin`
(view3d-window.md), not the terrain view, span kernels or the DirectX 7 driver (renderer-driver.md).

This spec **extends** view3d.md and does not repeat it. Already specified there and reused by reference:
the field layout (view3d.md §3.1), the 76-slot `cISC43DRender` table (§3.2), the pass brackets (§3.3),
the sub-objects and `DoMessage` (§3.4), and `Draw` with its static/dynamic blocks (§4).

**Completeness: partial.** §1–§6, §9 (lifecycle) and §10 (screen shake) are proof-backed (addresses cited,
numeric paths and constants read from instructions and the binary's data).
§7 lists every owner method this spec and view3d.md do **not** yet specify. No whole-owner credit is claimed.

Conventions: offsets are from the renderer base (`this`, the `cISC43DRender` pointer) unless marked
"sub-object relative". "Model" is the object passed to slots 27–31; its interface is named only by slot
offsets here (`model+0x14` = get grid element, `+0x18` = set grid element, `+0x20` = get world bounding box,
`+0x34` = object-flag byte, `+0x3C` = property block), because no name table covers it.

---

## 1. Identity, references and lifetime *[compat]*

### 1.1 `QueryInterface` (`0x007C1890`, slot 0)

Returns in **`AL`** (`mov al, 1` / `xor al, al`, `ret 8`), not EAX.

| IID | Result pointer |
|---|---|
| `1` (GZIID_cIGZUnknown), `0x681BD529`, `0xE9C622D8` | `this` |
| `0x452294AA` (cIGZMessageTarget2) | `this + 4` |
| `0xE4FDA3D4` (cIGZSerializable) | `this + 8` |
| anything else | false, out pointer **not written** (`0x007C18CC`) |

Every success path calls own `AddRef` (slot 1) before returning true. `0xE9C622D8` is also the class's
CLSID (`re/class_id_table.tsv`); `0x681BD529` is *[inferred]* the `cISC43DRender` IID (no name table row).

### 1.2 `AddRef` / `Release`

- `AddRef` (`0x004F7BA0`, folded): `++*(this+0x10)`, returns the new count in EAX.
- `Release` (`0x007C3060`): if the count is **greater than 1**, decrement and return the new count. Otherwise
  (count 1 **or 0**) it does **not** decrement: it calls own slot 4 `Shutdown`, then the deleting destructor
  (slot 75, flag 1), and returns 0. A replacement must run `Shutdown` before destruction on the last release.
- The deleting destructor `0x007CB090` runs the body `0x007CA020` and frees through `GlobalHeapFree`
  (`0x005E5620`) only when bit 0 of its argument is set.

### 1.3 Construction defaults (`0x007C9B10`)

The constructor allocates two owned helpers: an `SC4ViewUtilities` (0x470 bytes) at `+0x114` and an
`SC4DrawContext` (0x200 bytes) at `+0x11C` (either may be null on allocation failure; nothing checks).
Reference count `+0x10` starts at **0**. Non-zero defaults a replacement must reproduce:

| Field | Default | Meaning (where known) |
|---|---|---|
| `+0x4D` | 1 | background picture enabled (§3.4) |
| `+0x60` | 1 | byte returned by `GetBackgroundPicture`; when set, `Draw` skips `ClearViewport` (view3d.md §4.1 step 12) |
| `+0x63`, `+0x64` | 1, 1 | *[unknown]* |
| `+0x90`, `+0x94` | −1, −1 | camera-derived values (§3.1) |
| `+0xE8` | 1 | **lighting enabled** (§3.3) |
| `+0xE9` | 0 | **view fog enabled** (§3.3) |
| `+0xEC`…`+0xFC` | 1.0f ×5 | fog colour/params passed to `TurnOnFog` |
| `+0x100`, `+0x104` | 10000.0f, 15000.0f | fog start / end distance passed to `TurnOnFog` (view3d.md §4.1 step 14) |
| `+0x10C`, `+0x110` | 4096.0f, 4096.0f | *[unknown]* |
| `+0x1A4` | 0x5000 | *[unknown]* |
| `+0x1B8` | 1.0f | clear depth pushed to driver slot 13 |
| `+0x1E4`, `+0x1F4` | 0x7FFFFFFF | empty-rect sentinels of the dirty-rect accumulator |
| `+0x1FC` | 1 | *[unknown]* flag |
| `+0x210` | 0x40 | *[unknown]* capacity |
| `+0x230` | 10000 | *[unknown]* |
| `+0x264` | 1 | map scale factor (§3.1) |

Four empty list heads (`+0x188`…`+0x194`, node size 0x10), the dirty-rect list head `+0x1D4` (node 0x24) and a
list head `+0x234` (node 0x48) are created empty. Everything else is zero.

### 1.4 `GetGZCLSID` (`0x007C9F90`, serialisable slot 5)

Returns the constant `0xE9C622D8`.

---

## 2. Serialised state (`Write` `0x007C30F0`, `Read` `0x007C3180`) *[compat]*

Both run on the serialisable sub-object: `ECX = this + 8` (slots 3/4 of `0x00ABAB38` point straight at the
bodies, while slots 0–2 are adjustor thunks). The instructions address `[ECX+0x60]` etc.; **renderer-base
offsets are therefore the instruction offset + 8**. Ghidra's struct view of these two bodies is shifted by 8.

Record layout, in order:

| # | Stream call | Renderer field | Contents |
|---|---|---|---|
| 1 | slot 7 `PutUint16` / `GetUint16` | — | version, **2** |
| 2 | slot 8 (`+0x20`) | `+0x68` | viewport width |
| 3 | slot 8 | `+0x6C` | viewport height |
| 4–7 | slot 8 ×4 | `+0x70`, `+0x74`, `+0x78`, `+0x7C` | the viewport rect (left, top, right, bottom) |
| 8 | `SC4WriteGZSerializable` / `SC4ReadGZSerializable(stream, 0xC9C628EC, &field, 0)` | `+0x8C` | the camera control (`cSC4CameraControl`) |
| 9 | slot 9 `PutUint32` / `GetUint32` | `+0x90` | camera-derived value (§3.1) |
| 10 | slot 9 | `+0x94` | camera view state (§3.1) |
| 11 | slot 8 | `+0xDC` | current camera mode (§3.2) |

Both return `!stream->GetError()` (slot 18) in `AL`. `Read` returns false **without reading further** if the
version is not exactly 2 (`0x007C3194`). The camera pointer `+0x88` is not saved.

---

## 3. Camera, viewport and state accessors *[compat]*

### 3.1 Camera, camera control, viewport

| Slot | Method | Behaviour |
|---|---|---|
| 5 | `SetMapScaleFactor(int)` `0x007C9FA0` | stores the argument at `+0x264`; nothing else (used by `CreateQuadGrid`: grid size = factor × 8192.0, cell = factor × 32.0, `0x007C2390`) |
| 8 | `GetViewUtilities` | returns `+0x114` |
| 9 | `GetLightingManager` | returns `+0x118` |
| 11 | `GetViewportSize(int*, int*)` | writes `+0x68`, `+0x6C` |
| 12 | `SetViewportSize(w, h)` | writes `w` to `+0x68` **and** `+0x78`, `h` to `+0x6C` **and** `+0x7C` (the rect's right/bottom); no redraw |
| 13 | `SetSubpixelImageBias(x, y)` | stores `+0x80`, `+0x84` (added to the shake offsets in `Draw`) |
| 14 | `GetCamera` | returns `+0x88`, no AddRef |
| 15 | `SetCamera(cam)` `0x007C1980` | if different: AddRef new, Release old, store; if non-null: driver-service (`0x00B4E1EC`) slot `+0x100` into a local, `SetDepthSize(65536.0f)` (`0x47800000`) on the camera, and `+0x90 = cam[+0x17C] − 1` |
| 16 | `GetCameraControl` | returns `+0x8C`, no AddRef |
| 17 | `SetCameraControl(cc)` `0x007C1A00` | if different: AddRef new, Release old, store; if non-null `SC4ViewUtilities::UpdateCameraPosition(+0x114, cc)` |

Zoom and rotation forward to the camera control at `+0x8C` without a null check:

| Slot | Method | Forwarding |
|---|---|---|
| 35 | `SetZoom(int32)` | converts the int to **float** (`fild`/`fstp`) and calls `cSC4CameraControl::SetZoom` `0x007CDC50` |
| 36/37 | `ZoomIn`/`ZoomOut` | tail-jump `0x007CE080` / `0x007CE0E0` |
| 38 | `SetRotation(int32)` | tail-jump `0x007CD540` with the **int unchanged** (no conversion) |
| 39/40 | `RotateLeft`/`RotateRight` | tail-jump `0x007CD6A0` / `0x007CD6C0` |
| 41 | `SetZoomAndRotation(a, b)` | tail-jump `0x007CDE60`, arguments unchanged |
| 45 | `GetShakeOffsets(float*, float*)` | int `+0x258` and `+0x25C` converted to float |

### 3.2 Camera mode `+0xDC` and the render-system borrow

`+0xDC` caches which camera the draw context holds: 0 none, **1 = 2D**, **2 = 3D**, **3 = UI**. Each setter is a
no-op when the mode already matches:

- `Set3DCameraView` (slot 24): `SC4DrawContext::SetCamera(+0x11C, +0x88)`, mode 2, `+0x94 = cS3DCamera::GetViewState`.
- `Set2DCameraView` (slot 25): `SC4DrawContext::SetCamera(+0x11C, +0xD8)`, mode 1.
- `SetUICameraView` (slot 26): driver-service slots `+0xA0(1)`, `+0xA4(this+0x98)`, `+0xA0(0)`, `+0xA8()`, then
  `SetModelViewTransformChanged`, mode 3.

`BorrowRenderSystem` (slot 58) sets byte `+0x4C = 1` and returns 1. `ReturnRenderSystem` (slot 59, `0x007C2CE0`)
clears `+0x4C`, calls camera-control `0x007CCF80` with 5, **resets the mode to 0 and re-applies the previous mode**
(1 → slot 25, 2 → slot 24, 3 → slot 26, other → none), then `SetDefaultRenderStateUnilaterally`, and returns
`AL = 1`. So a borrower may freely change the draw context; returning restores camera and render state.

### 3.3 Fog and lighting toggles

| Slot | Method | Behaviour |
|---|---|---|
| 69 | `GetViewFogEnabled` | `AL = +0xE9` |
| 70 | `SetViewFogEnabled(b)` | if changed: store, **then** `ForceFullRedraw`; returns `AL = 1` always |
| 71 | `GetLightingEnabled` | `AL = +0xE8` |
| 72 | `SetLightingEnabled(b)` | if changed: `ForceFullRedraw` **before** storing; returns `AL = 1` always |

### 3.4 Background, transparency, backing-store switch

| Slot | Method | Behaviour |
|---|---|---|
| 46 | `SetViewportClearColor(float[4])` | copies four dwords to `+0x1A8`…`+0x1B4` and clears `+0x1A0`, so the next `Draw` re-pushes viewport colour (view3d.md §4.1 step 11) |
| 51 | `EnableBackgroundPicture(b)` | byte `+0x4D` |
| 52 | `GetBackgroundPicture(bool* opt)` | if the pointer is non-null writes byte `+0x60` to it; returns `+0x50` |
| 55 | `EnableTransparentBackground(b)` | byte `+0x62` |
| 56 | `GetBackingStoreEnabled` | `AL` = **process-global** byte `0x00B0DC19` (not per instance) |
| 57 | `SetBackingStoreEnabled(b)` | if the global backing-store object `0x00B4E1E4` is null the value is forced to false; stores to `0x00B0DC19` if different. Returns nothing defined (AL holds the stored value) |

---

## 4. Full redraw (`ForceFullRedraw` slot 20 `0x007C7DD0`, `DoFullViewUpdate` `0x007C7310`) *[compat]*

Both write a reason code to `*(+0x11C) + 0x20` — `ForceFullRedraw` writes **10**, `DoFullViewUpdate(n)` writes `n`
(the model queue passes 4) — and then, **only if `+0x109` is clear**: set `+0x109`, clear `+0x65` (the saved
backing store is stale), `SC4ViewUtilities::BuildDrawFrustumDataFromViewportRect(+0x114, &+0x70, &+0x78)`, clear
bytes `+0x160` and `+0x174`, and `ClearDirtyRectangleQueue` (`0x007C6D80`). `Draw` clears `+0x109` (view3d.md §4.1
step 17), so repeated requests between two draws collapse to one. While `+0x109` is set, `AddDirtyRectangle` is
ignored (§6).

`UpdateViewportRect(rect)` (`0x007C94C0`) does nothing unless `left < right` and `top < bottom` (signed); otherwise it
rebuilds the frustum from the rect, clears `+0x160`/`+0x174`, `SetClipRect`, `ClearViewport(0)` (slot 47),
`Set3DCameraView`, and runs the static triple if the static gate byte is set (view3d.md §4.2).

---

## 5. The model change queue *[compat]*

Static models live in an `SC4QuadGrid` at `+0x120`, each wrapped in an `SC43DGridElement` that the model holds
(`model+0x18` set, `+0x14` get). Changes to models drawn from the backing store are not applied at once: they are
queued in the vector `+0x1C4`/`+0x1C8`/`+0x1CC` of **0x30-byte** items (model, kind byte at `+4`, bounding box)
built by `0x007C2E50` and applied by `ProcessModelChangeQueue` (`0x007C9600`) at the next `Draw` (view3d.md §4.1
step 6) or earlier as below.

Grid-element flag word at element `+6`:

| Bit | Meaning |
|---|---|
| `0x0004` | added to the quad grid |
| `0x0008` | **in the backing store** (static image) |
| `0x0010` | has a queued change (cleared when processed) |
| `0x0020`/`0x0040`/`0x0100`/`0x0200`/`0x0400`…`0x2000` | object-class bits copied from `model+0x34` / `model+0x3C` by `UpdateGridElementObjectFlags` (`0x007C2B40`) |
| `0x0080` | set by `ChangeModel(…, 1)` from property byte 1 |

`ModelShouldGoIntoBackingStore` (`0x007C5540`): false if `0x100`; true only if `0x20` clear and `0x40` set and one
of: `0x400` with draw-context `+0x14` < props `+0x1AC`; `0x1800` with `+0x14` < props `+0x184`; `0x2000` with
`+0x14` < props `+0x1D4` (props = `*(*(+0x48)+0x18)`, signed compares).

Queue item kinds and their processing:

| Kind | Queued by | Processing (`0x007C9600`) |
|---|---|---|
| 0 | `InsertModel` when the new element should go into the backing store | set `0x08`; dirty-rect the model's box |
| 1 | `RemoveModel` of a backing-store element | clear `0x08`; dirty-rect the old box; then call slot 29 `RemoveModel` again with the item's model slot nulled during the call |
| 2 | `MoveModel` of a backing-store element | `MoveGridElement`; if it succeeded: when old and new boxes are separated (any axis differs by ≥ 0.0001 the "outside" way) two dirty rects, else one for their union |
| 3, 4 | `ChangeModel` with type 2 / 3 | dirty-rect the box (mode 0 for kind 3, 3 for kind 4) |
| 5 | `ChangeModel` with type 5, or any other type on a backing-store element | re-derive flags; move the element into/out of the backing store per `ModelShouldGoIntoBackingStore`, dirty-rect on change |

Dirty rects in processing are only added when `+0x109` is clear **and** the global backing-store byte
`0x00B0DC19` is set. Byte `+0x1D0` is a re-entrancy guard (set for the walk, cleared after). Every queued item holds
a model reference (AddRef at queue time, Release after processing); afterwards processed items are erased.

Entry points:

- **`IsModelInserted(model)`** (slot 27): false if the model has no grid element. If the element has a queued change
  (`0x10`): false when the queue holds a **remove** (kind 1) for this model; and if the queue holds **more than 8**
  items, it is processed now. Otherwise true.
- **`InsertModel(model)`** (slot 28): false for null. If the model already has an element: without a queued change
  (`0x10`) nothing is inserted (the model is released and the call returns, `0x007CA3B4`); with one, if the model is
  not in the queue return false, else process the queue and continue. Builds a new element, sets
  its rect (`WorldToContainerRect`), resets flags, `UpdateGridElementObjectFlags`, `SC4QuadGrid::AddElement(…, 1)`. On
  failure clears the model's element. On success AddRefs the model, sets `0x04`, and if it should go into the backing
  store queues kind 0 and sets `0x10`.
- **`RemoveModel(model)`** (slot 29): if the element is not in the backing store: process the queue first when it has a
  queued change, remove from the grid (Release the model if removed), clear the model's element. If it is: process
  the queue first when queued, then queue kind 1 and return true.
- **`MoveModel(model)`** (slot 30): no element → call slot 28 `InsertModel`. Not in backing store → `MoveGridElement`
  now. In backing store → queue kind 2. Returns true.
- **`ChangeModel(model, type)`** (slot 31): type 4 → **slot 30 `MoveModel`**. No element → false. Type 1 → set or clear
  element bit `0x80` from property byte 1 (setting also calls `SetQuadrantFlagsForElement`), true. Other types queue an
  item only if the element is in the backing store or type is 5: jump table `0x007C94B0` maps type 2→kind 3, 3→kind 4,
  5→kind 5, anything else→kind 5. Returns true.

**Overflow:** after each push, if the queue length is **exactly 200**, `DoFullViewUpdate(4)` (§4). It is an
equality test, so pushes 201+ do not re-trigger.

---

## 6. Dirty rectangles: entry points *[compat]*

- `AddDirtyRectangle(mode, int rect[4], model, extra)` (`0x007C9540`): ignored while `+0x109` (full redraw pending)
  is set; copies the rect, and if `0x007C2C70` accepts it (*[inferred]* clip to viewport / non-empty) forwards to
  `AddDirtyRectangleInternal` (`0x007C8F30`).
- `AddDirtyRectangle(mode, box_min, box_max, model, extra)` (`0x007C95B0`): projects the 3D box with
  `SC4ViewUtilities::ProjectBoundingBoxToViewportRectangle` and calls the first overload.

### 6.1 The queue *[compat]*

`+0x1D4` heads a circular doubly linked list of **0x24-byte** nodes: next `+0`, prev `+4`, a copy of renderer
`+0x15C` at `+8`, the **mode** at `+0xC`, the screen rect (left, top, right, bottom, ints) at `+0x10`…`+0x1C`, and a
model reference at `+0x20`. `+0x1D8` counts nodes; `+0x1E0` accumulates their pixel area against the limit `+0x1E4`
(default `0x7FFFFFFF`).

`AddDirtyRectangleInternal(mode, rect, model, delay)` (`0x007C8F30`), common path:

1. A non-zero mode drops the model (only mode 0 keeps one).
2. Area = (bottom − top) × (right − left), int.
3. If the queue is **empty**, the deadline base `+0x1F8 = draw counter (+0x158) − delay`. (`Draw` processes the queue
   once the counter leaves `[+0x1F8, +0x1F8 + props(+0x2EC)]`, view3d.md §4.1 step 16.)
4. Append: `+0x1E0 += area`; if that is **≥ `+0x1E4`**, call `DoFullViewUpdate(4)` (§4) instead of queuing.
   Otherwise AddRef the kept model and append a node at the tail; `+0x1D8 += 1`.

When the process-global byte `0x00B0DC18` is set and the queue is not empty, a **merge** step runs first
(`0x007C8F9A`): `+0x1F8 −= delay`, every node's rect is unioned with the new one (`0x005DC980`) and the node with the
best area saving is chosen; if the node count reaches the global `0x00B0DC14` or the best saving reaches the global
float `0x00B0DC10`, the chosen node becomes the union with mode **6** (its model released), and if it grew while
more than one node exists it is unlinked and re-added through this function as mode 6. The exact scoring is §7
remaining work (R3D-3).

### 6.2 Processing, clearing, cleanup *[compat]*

`ProcessDirtyRectangleQueue` (`0x007CAE00`) sets the guard `+0x1DC`, then walks the nodes in insertion order:

| Node mode | Action |
|---|---|
| 0 with a model | if the rect passes `0x007C2C70`: `DoSingleModelStaticDrawAddition(model, rect)` |
| 0 without a model | nothing |
| non-zero | only if the rect overlaps `[0, +0x68) × [0, +0x6C)` (right > 0, left < width, bottom > 0, top < height): clamp to the viewport and `DoPartialViewUpdate(rect, mode == 1 ? 8 : 7)` |

Each node's model is released; the list is emptied, the vector `+0x1E8`/`+0x1EC` cleared, `+0x1D8 = +0x1E0 = 0`.
Then, **even when the queue was empty**, if `+0x65` (backing store saved) is clear:
`+0x65 = SaveBackingStore(0, 0, +0x68, +0x6C)`. Finally `+0x1DC = 0`.

`ClearDirtyRectangleQueue` (`0x007C6D80`): if non-empty, release every node model, empty the list, clear the vector,
`+0x1D8 = +0x1E0 = 0`. No drawing, no backing-store save.

`DoPartialViewUpdate(rect, reason)` (`0x007CA9C0`): only if the global backing-store byte `0x00B0DC19` is set and the
rect is non-empty (signed `left < right`, `top < bottom`): draw-context `+0x20 = reason`, `UpdateViewportRect(rect)`
(§4), driver-service slot `+0x12C` (300), `UnsetClipRect` on the 3D camera `+0x88` and the 2D camera `+0xD8`, camera
mode `+0xDC = 0`, and if `+0x65` is set `+0x65 = SaveBackingStore(left, top, width, height)` of that rect; then
draw-context `+0x20 = 0`.

`DoViewCleanup` (`0x007CAFB0`, called from `Draw` when `+0x1FD` is set): needs `+0x1FD` and `+0x1FC`. Uses the timer
at `+0x218` (constructed with unit code 4) and threshold `+0x230` (default 10000). Timer not running → start it.
Running and elapsed **>** threshold → refresh one band: rect = (`+0x200`, `+0x204`, min(`+0x200 + +0x210`,
`+0x208`), `+0x20C`) with reason 9, then `+0x200` = the band's right edge; when it reaches `+0x208`, clear `+0x1FD`
and reset the timer. With the default band width `+0x210 = 0x40` the view is refreshed in 64-pixel columns.

### 6.3 Screen shakes: the list and `StopScreenShake` *[compat]*

Active shakes are nodes (0x48 bytes) of the circular list headed at `+0x234`; elapsed time comes from the timer
at `+0x240` (constructed with unit code 5), read as a **float**. The node layout, both `StartScreenShake`
overloads and the per-frame `SetShakeOffsets` are §10. Fields used here: `+0x08` start time, `+0x1C` duration,
`+0x20` fade-out length, owned float arrays `+0x24`/`+0x30`, and the **handle** at `+0x44`.

- `StopScreenShake(handle)` (`0x007C8630`, slot 44): finds the first node whose `+0x44` equals the handle by an
  **integer** compare (`0x007C8645`); none → no effect. If the node's `+0x20` equals the float constant at `0x00A81054`
  (0.0) it is unlinked and freed at once (with its two arrays). Otherwise it is kept and its **start time** is pulled
  back: `+0x08 = min(+0x08, now + (+0x20 − +0x1C))` (float, `0x007C86AD`–`0x007C86C1`, stored only when strictly
  smaller). The shake's age `now − start` then jumps to `duration − fade` (unless it is already older), so it plays
  the last `fade` time units of its curves instead of stopping dead.

---

## 7. Remaining owner methods (not specified here or in view3d.md)

Picking: `Pick` ×2 (`0x007C2220`, `0x007C6830`), `PickTerrain` ×2 (`0x007C35C0`, `0x007C5000`), `PickTerrainAlways` ×2
(`0x007C50F0`, `0x007C7E20`), `IsModelVisible` ×2, `FindViewObject`, `AddViewObject`, `RemoveViewObject`,
`InsertIntoViewObjectList`, `FindInViewObjectList`. Lifecycle (`Init`, `Shutdown`, `ReadProperties` are §9): `SetGraphicSystem`, `SetGDriver`, `StaticSetGDriver`, `StaticShutdown` (outline:
releases the globals `0x00B4E1EC`, `0x00B4E1F0`, `0x00B4E1F4` once, destroying the backing store first),
destructor body `0x007CA020`. (Shake is complete: §6.3 and §10.)
Backing store: `CreateBackingStore`, `DestroyBackingStore`, `SaveBackingStore`, `RestoreBackingStore` ×2,
`ReevaluateBackingStoreObjectFlags`, `DoSingleModelStaticDrawAddition`. Dirty rects: the merge scoring inside
`AddDirtyRectangleInternal` (§6.1), the rect filter `0x007C2C70`, the writer of `+0x1FD`. View updates:
`DoTranslatedViewUpdate`, `DoViewZoomChange3`, `DoViewRotationChange2`, `AttemptTranslatedViewUpdate`,
`UpdateVisibleTerrainMetrics`. Geometry: `WorldToContainerRect`, `Get{View,Draw}FrustumContainerRect`,
`GetVisibleContainerRect`, `GetViewportRayContainerRect`, `GetModelContainerRect`, `RebuildQuadGrid`,
`SetClipRect`, `UnsetClipRect`. Drawing helpers: `ClearViewport` (2433 bytes), `ClearViewportWithSolidColor`,
`DrawViewportRectangle`, `DrawTexturedViewportRectangle`, `SetBackgroundPicture` ×2, `TakeSnapshot` ×2,
`GetStatsString1/2`, `DrawStaticView`/`DrawDynamicView` bodies beyond view3d.md §4.2, `TurnOnLighting`, `TurnOnFog`.

---

## 8. Conformance cases

1. `QueryInterface(0x452294AA)` yields `this+4`; `(0xE4FDA3D4)` yields `this+8`; `(0x12345678)` returns `AL=0` and leaves
   the out pointer unchanged. Each success raises the count by 1.
2. New object: count 0; `AddRef` → 1; `Release` → `Shutdown` + destroy, returns 0.
3. `SetViewportSize(800, 600)` then `GetViewportSize` → 800, 600; the rect right/bottom are 800/600 and its left/top
   unchanged.
4. `SetZoom(3)` hands the camera control `3.0f`; `SetRotation(3)` hands it the int 3.
5. `SetLightingEnabled(1)` on a new object → no redraw (default already 1), returns 1. `SetViewFogEnabled(1)` → redraw.
6. `SetBackingStoreEnabled(1)` with no backing-store object → `GetBackingStoreEnabled` stays 0.
7. Two `ForceFullRedraw` calls without a `Draw` between: the frustum is rebuilt and the dirty queue cleared once; the
   reason code is 10 after both.
8. `ChangeModel(m, 4)` behaves exactly as `MoveModel(m)`; `ChangeModel(m, 2)` on an element not in the backing store
   queues nothing and returns true.
9. `Write` then `Read` round-trips version 2, viewport size and rect, camera control, `+0x90`, `+0x94` and the camera
   mode; a stream with version 3 makes `Read` return false.
10. Dirty rects with merging off (`0x00B0DC18` = 0): two `AddDirtyRectangle` calls give two nodes processed in order;
    a node of mode 1 at (−10, −10, 50, 50) on an 800×600 viewport is redrawn as (0, 0, 50, 50) with reason 8; a node at
    (900, 0, 950, 10) is skipped. `ProcessDirtyRectangleQueue` on an empty queue still saves the backing store when
    `+0x65` is clear.
11. Areas whose running total reaches `+0x1E4` turn the add into `DoFullViewUpdate(4)`; after that, adds are ignored
    until the next `Draw` clears `+0x109`.
12. With 199 queued items the next backing-store insert forces `DoFullViewUpdate(4)`; the 201st does not.

Questions: docs/specs/questions/render3d.md.

---

## 9. Lifecycle: `Init`, `ReadProperties`, `Shutdown` *[compat]*

All three return through AL. `Init` (slot 3) and `Shutdown` (slot 4) are guarded by the **initialised byte
`+0x0C`** (0 after construction): `Init` on an initialised object and `Shutdown` on an uninitialised one do
nothing and return 1 (`0x007C641B` → `0x007C6822`; `0x007C707C` → `0x007C7303`). Both always return 1.

### 9.1 `Init` (`0x007C6410`)

In this order:

1. **Once per process** (process byte `0x00B4E1E0`, shared with `0x007C3BB0`): set it and register
   `StaticShutdown` (`0x007C3090`, data 0) with the application (`0x00B43C94`, slot 69 `+0x114`), the
   application-shutdown cleanup hook (building-development.md, DG7).
2. `+0x120 = CreateQuadGrid()` (`0x007C2390`, sized from the map scale factor `+0x264`, §3.1 slot 5).
3. Eight **per-rotation container margins**: each is `float(int +0x264) × constant`, computed as an int-to-float
   load times a **float** constant, stored as float (`0x007C644D`–`0x007C64DD`):

   | field | constant (address) | field | constant (address) |
   |---|---|---|---|
   | `+0x268` | 3000.0 (`0x00A9DA68`) | `+0x278` | 4500.0 (`0x00ABAB18`) |
   | `+0x26C` | 1400.0 (`0x00ABAB14`) | `+0x27C` | 6400.0 (`0x00ABAB10`) |
   | `+0x270` | 5200.0 (`0x00ABAB0C`) | `+0x280` | 7200.0 (`0x00ABAB08`) |
   | `+0x274` | 6800.0 (`0x00ABAB04`) | `+0x284` | 5300.0 (`0x00ABAB00`) |

   They are two 4-entry float arrays (`+0x268[i]`, `+0x278[i]`) indexed by the view-state index at
   `SC4ViewUtilities+0x10` (*[inferred: the rotation 0–3]*); the frustum container-rect helpers
   (`0x007C1E10`/`0x007C1EF0` and the routine before them, §7 geometry) add them to projected X and Y.
4. **Preferences object** `+0x48` := global `0x00B43CC4` with AddRef-new / Release-old, only if different.
5. When the backing-store global `0x00B4E1F0` exists: byte `+0x63` := its slot 9 (`+0x24`) result; byte `+0x64` :=
   1 only when that result is non-zero **and** byte `[[0x00B43CC4]+0x0C]+0x3DC` is 0, else 0.
   (`+0x63`/`+0x64` keep their constructor values when the global is null.)
6. When message server 2 (`0x00B43CCC`) exists: register the message-target face (`this+4`) through its slot 5
   (`+0x14`) for these 11 message types, in this order: `0xA6B79601`, `0xA6B79602`, `0xA6B79612`, `0x870BC918`,
   `0xA6B79621`, `0x26D31EC1`, `0xC9F775BB`, `0xC53D10AA`, `0x4A9F2FFE`, `0x8ACAA95F`, `0x6ACB92A9`
   (`0x007C654B`–`0x007C65D8`). `DoMessage` (view3d.md §3.4) handles them.
7. When `+0x118` is null: allocate 0x1D8 bytes, construct a `cSC4LightingManager` (`0x007DA0A0`), store it at
   `+0x118` (null on allocation failure, then the next line faults) and AddRef it.
8. **2D camera**: allocate 0x1A8 bytes, construct a camera (`0x007FF930`), store at `+0xD8` **without releasing a
   previous value** (none exists: `Shutdown` clears it), AddRef. Then on it: a no-op stub (`0x00648F00` only returns
   1), `SetViewport(float(+0x68), float(+0x6C))`, `SetOrtho(0, float(+0x68), float(+0x6C), 0, 0, 32767.0f)` (the last
   argument is the bit pattern `0x46FFFE00`, `0x007C6664`), then `0x007FF190`.
9. **Projection copy**: the 4×4 float matrix returned by `cS3DCamera::ProjectionMatrix` (`0x007FFEF0`) of the 2D
   camera is copied **transposed** into `+0x98`..`+0xD4`: destination row *i* = source column *i*
   (`0x007C66A2`–`0x007C66C5`).
10. View utilities `+0x114`: `+0x04 = this`, `+0x20 = +0x24 = 4096.0f` (`0x45800000`); then
    `UpdateCameraPosition(+0x114, +0x8C)` and `BuildDrawFrustumDataFromViewportRect(+0x114, &+0x70, &+0x78)`.
11. Clear bytes `+0x160` and `+0x174` (the same two flags `DoFullViewUpdate` clears).
12. Draw context `+0x11C` fields: `+0x04 = 0`, `+0x24 = +0x114`, `+0x28 = +0x14`, `+0x2C = +0x18`,
    `+0x30 = global 0x00B4E1EC`, `+0x34 = this`, `+0x38 = +0x48`, `+0x40 = +0x8C`, `+0x44 = +0x118`, `+0x48 = 0`,
    `+0x4C = 0`, `+0x08 = +0x68`, `+0x0C = +0x6C` (ints), `+0x10 = float(view +0x0C)`, `+0x14 = view +0x0C` (int),
    `+0x18 = view +0x10`; then `SC4DrawContext::Init` (`0x007D6320`).
13. `ReadProperties()` (§9.2), then the lighting manager's slot 3 (`+0x0C`).
14. `+0x1E4 = 0x7FFFFFFF` (dirty-area limit, again), `+0x1F4 = 40000` (`0x9C40`), `+0x0C = 1`.

### 9.2 `ReadProperties` (`0x007C5F40`)

1. Ask the resource manager (`0x00B43CA8`, slot 3 `+0x0C`) for the exemplar with key **type `0x6534284A`, group
   `0x690F693F`, instance `0x4A639EF2`** through interface `0xA52160F5` (out pointer pre-set to null, two trailing
   zero arguments). Failure (AL = 0) → skip to step 4.
2. Take its property holder: the resource's slot 7 (`+0x1C`).
3. For *k* = 0..10, property ID `0xEA639FB1 + k`: read it with `GetS3DPropertyValue(holder, id, 4, buf)`
   (`0x007C5E20`). That helper succeeds only when the property exists **and has exactly 4 values**; it then copies the
   four 32-bit values into `buf`. On success, `SC4DrawContext` routine `0x007D4F80(k, buf)` copies the four values
   into entry *k* of the 16-byte array pointed to by draw context `+0x54`. Missing or wrongly sized properties leave
   entry *k* untouched. (`buf` is pre-filled once from the zero-initialised global `0x00B4E1F8`; it is never
   written on failure and always fully written on success, so the pre-fill is unobservable.)
4. Release the resource (slot 2) when the out pointer is non-null.

The 11 entries are four floats each *[inferred: colours/lighting tuples; their consumers are the draw context,
outside this owner]*.

### 9.3 `Shutdown` (`0x007C7070`)

In this order (field clears are exact; "release" = slot 2 `+0x08`):

1. `+0x44 := 0` (cleared, **not** released).
2. Drain the model change queue (`0x007C6E30`), unless its guard byte `+0x1D0` is set: set the guard; for each
   0x30-byte item from `+0x1C4` to `+0x1C8` with a model: if the model's grid element (model slot 5 `+0x14`) is
   non-null, clear bits `0x18` of the element's byte `+6` and call own slot 29 `RemoveModel(element)` with the item's
   model slot nulled during the call (§5); then release the model. Erase all items (`+0x1C8 = +0x1C4`), clear the
   guard.
3. When the quad grid `+0x120` exists: collect all its elements into a temporary array (count from grid `+0`);
   for each element release the object at element `+0x18` (if any) and null it; `RemoveAllElements`; free the array.
4. Clear the four list heads `+0x188`, `+0x18C`, `+0x190`, `+0x194` (`0x007C3B60`: release every node's object at
   node `+0x08`, free the nodes, relink the head to itself).
5. Draw context `0x007D2720`: if its byte `+0` is set, clear it and the four bytes `+0x38`..`+0x3B`.
6. When message server 2 exists: unregister `this+4` through slot 6 (`+0x18`) for **12** types, in this order:
   `0x6ACB92A9`, `0x8ACAA95F`, `0xA6B79601`, `0xA6B79602`, `0xA6B79612`, `0x870BC918`, `0xA6B79621`, `0x26D31EC1`,
   `0xC9F775BB`, `0xC53D10AA`, **`0x4348B111`**, `0x4A9F2FFE` (`0x007C7152`–`0x007C71EC`). `0x4348B111` is not
   registered by `Init`; a replacement unregisters it too (harmless when absent).
7. Lighting manager `+0x118`: slot 4 (`+0x10`), release, `:= 0`.
8. Release and null, each only when non-null, in this order: background picture `+0x50`; texture `+0x54`
   (`nsTexUtil::ReleaseTexture(0x00B4E1EC, +0x54)` only when the global `0x00B4E1EC` is non-null, but `+0x54` is
   cleared either way); 3D camera `+0x88`; camera control `+0x8C`; `+0x14`; `+0x18`; `+0x198`; `+0x19C`.
9. 2D camera `+0xD8`: the no-op stub `0x00648F00`, release, `:= 0`.
10. Preferences `+0x48`: `:= 0` **then** release the old value.
11. Quad grid `+0x120`: destroy (`0x005F3EB0`) and free when non-null; `+0x120 := 0` always.
12. `+0x0C := 0`.

Not touched by `Shutdown`: the dirty-rectangle list, the screen-shake list `+0x234`, the view utilities `+0x114`
and draw context `+0x11C` objects themselves (the destructor `0x007CA020` owns those, §7). A second `Init` after
`Shutdown` therefore rebuilds everything above while shakes and dirty rectangles survive.

Cases (lifecycle):

- L1. `Init` twice: the second call returns 1 and changes nothing (no second grid, camera or registration).
- L2. `Shutdown` on a never-initialised object returns 1 and releases nothing.
- L3. Map scale factor 2 (slot 5 before `Init`): `+0x268` = 6000.0, `+0x26C` = 2800.0, `+0x284` = 10600.0.
- L4. Exemplar property `0xEA639FB3` with 4 values (1, 2, 3, 4) → draw-context entry 2 = (1, 2, 3, 4); the same
  property with 3 values → entry 2 unchanged; `0xEA639FBC` (k = 11) is never read.
- L5. `Init` with message server present then `Shutdown`: 11 registrations, 12 unregistrations, the extra one
  `0x4348B111`.
- L6. After `Init`, `+0x98` holds the transpose of the 2D camera's ortho projection; `+0x1F4` = 40000.

---

## 10. Screen shake: start, per-frame offsets *[compat]*

### 10.1 Node and parameter block

`StartScreenShake` (slot 42, `0x007CACD0`) takes a 0x28-byte **parameter block** `P` and an optional pointer to a
2-float **position** `pos` (world X, Z *[inferred: compared with the camera centre below]*). The node (0x48 bytes:
`+0` next, `+4` prev, then the payload copied by `0x007C63A0`) holds:

| node | from | meaning |
|---|---|---|
| `+0x08` | timer `+0x240` elapsed (float) at start | start time |
| `+0x0C` | 0 | phase accumulator (float) |
| `+0x10` | `pos != null` (byte) | positional flag |
| `+0x14`, `+0x18` | `pos[0]`, `pos[1]` (only when `pos` non-null) | position |
| `+0x1C` | `P+0x00` | duration (float) |
| `+0x20` | `P+0x04` | fade-out length (float), used only by `StopScreenShake` (§6.3) |
| `+0x24` (begin/end/cap) | float array copied from `P+0x08` | amplitude curve |
| `+0x30` (begin/end/cap) | float array copied from `P+0x14` | rate curve |
| `+0x3C` | `P+0x20` | X/Y aspect `k` (float) |
| `+0x40` | `P+0x24` | pattern (int): 0 random, 1 sine, other none |
| `+0x44` | counter `+0x238`, then `+0x238 += 1` | handle, also the return value (EAX) |

Order (`0x007CACD9`–`0x007CADE9`): if the list is empty, restart the `+0x240` timer and set float `+0x260 = 0`; the
new node is **inserted at the front** (after the head, `0x007CAD45`–`0x007CAD55`); fill it as above. The handle
counter starts at the constructor's value and is never reset.

The **overload** (slot 43, `0x007C6BC0`) takes two 32-bit arguments `(a, d)`, both floats by use, and calls slot
42 with `pos = null` and the block: duration = `d`; amplitude curve = `[a, 0.0]`; rate curve = `[0.0, 60.0, 0.0]`
(`0x42700000`, `0x007C6CC8`); `k = 1.0`; pattern 0. It returns slot 42's handle. **`P+0x04` (fade-out length) is never
written** — it is whatever the stack held (see question R3D-4); a replacement stores 0.0, so `StopScreenShake`
removes such a shake at once.

### 10.2 The two pattern tables (built once)

On the first `SetShakeOffsets` call (process tables at `0x00B4E224` and `0x00B4E230`, built when the first is
empty):

- **Random table** (64 pairs): seed the process random generator `0x00B4E240` with **88** (`0x58`); then 64 times
  draw *x* then *y*, each `RandomDoubleRangeUniform(−10.0, 10.0)` (double arguments `0x00ABAB30`, `0x00AA2028`)
  narrowed to float; the pair is (*x*, *y*).
- **Sine table** (64 pairs): entry *i* = (0.0, float(sin(*i* × 0.09817477f) × 10.0f)) for *i* = 0..63; the
  multiplier `0x00ABAB28` is π/32 as a float, `FSIN` and the multiply by 10.0f run in x87 extended precision and
  only the product is rounded to float (`0x007C87C6`–`0x007C87E8`).

### 10.3 `SetShakeOffsets` (`0x007C86D0`), called from `Draw`

`Draw` calls it only when byte `[[+0x48]+0x0C]+0x2DC` is set; otherwise it zeroes `+0x258`/`+0x25C` itself
(`0x007CB889`–`0x007CB8A7`, view3d.md §4.1 step 21). Then:

1. `+0x258 = +0x25C = 0`. Empty list → return.
2. Frame constants: camera centre `cx = view+0x3DC`, `cz = view+0x3E4` (view = `+0x114`); radius
   `R = cameraControl(+0x8C)+0x134 × 640.0f`; `inv = 1 / (R × R)` (extended, stored float); `steps = 4 − drawContext+0x14`
   (the zoom, §9.1 step 12); `now` = timer `+0x240` elapsed; `dt = now − +0x260`; `+0x260 = now`;
   `sx = sy =` the zero-initialised global `0x00B4E1F8`.
3. For each node, front to back:
   - `t = now − start`. If `t > duration` (strict): unlink and free the node; next.
   - `w = 1.0`; if positional: `w = 1 − ((px − cx)² + (pz − cz)²) × inv`; if `w < 0.0` the node contributes nothing
     this frame (it stays in the list, its phase does not advance).
   - `u = t / duration` (float). `A = sample(amplitude curve, u)`, `F = sample(rate curve, u)`, where
     `sample(c, u)` with *n* entries is: *n* = 1 → `c[0]`; else `x = (n−1)·u`, `i = floor(x)`, `r = x − i`;
     `r ≤ 0` → `c[i]`; else `c[i] + (c[i+1] − c[i])·r`.
   - `ax = A·w / k`, `ay = A·w·k`.
   - `phase(+0x0C) += F × min(t, dt)` (stored float; `t` is used when `t < dt`, i.e. on the shake's first frame).
   - `f = frac(phase / 64)` (64 = the random table's size, used for both patterns).
   - Pattern 0 or 1: look up the table with `x = 63·f`, `i = floor(x)`, `r = x − i`: `r ≤ 0` → `T[i]`; else
     `X = r·T[i+1].x + (1−r)·T[i].x`, `Y = (1−r)·T[i].y + r·T[i+1].y` (every product rounded to float except `r·T[i+1].x`, which stays
     extended);
     `sx += X·ax`, `sy += Y·ay`. Other pattern values add nothing.
4. `s = 0.6f^steps` by repeated multiplication in extended precision (`steps ≤ 0` → 1.0; `0x00A9809C`).
5. `+0x258 = round(sx·s)`, `+0x25C = round(sy·s)`, both by the float trick "add 12582912.0f, store as float, take
   the integer bits − `0x4B400000`" (`0x007C8EEC`–`0x007C8F20`): **round half to even**, valid for |v| < 2²². For X the
   product stays extended before the add; for Y it is first stored as float.

`floor` above is the same float trick on `x − 0.5` with a correction for exact integers; it equals the
mathematical floor for the ranges that occur (0 ≤ x < 2²²). A curve with **no** entries makes *n*−1 wrap to
2³²−1 *[pathological, not reachable from slot 43]*.

`GetShakeOffsets(float* x, float* y)` (slot 45, `0x007C2330`) returns `float(+0x258)`, `float(+0x25C)`.

Cases (shake):

- S1. `StartScreenShake` overload twice on an empty list → handles *h*, *h*+1; the list order is newest first.
- S2. Overload shake `(a = 5, d = 2)`: at `t = 1` the amplitude is 2.5 and the rate 60; at `t = 2` the node still
  exists (amplitude 0); at the first frame with `t > 2` it is removed.
- S3. Positional shake at distance ≥ R from the camera centre: `w ≤ 0` (exactly R → `w` = 0, still sampled, adds 0);
  beyond R it neither contributes nor advances its phase.
- S4. Zoom 3 (`steps` = 1): offsets are scaled by 0.6; zoom 4 or 5: unscaled; zoom 1: 0.216.
- S5. Sum 2.5 → offset 2; 3.5 → 4; −2.5 → −2 (half to even).
- S6. Preference byte `+0x2DC` clear: `Draw` writes zero offsets and never advances phases or removes nodes.
