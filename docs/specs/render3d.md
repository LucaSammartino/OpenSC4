# `cSC43DRender`: owner behaviour beyond the draw pass (behaviour spec)

Scope: the exact owner `cSC43DRender` (153 functions as tools/coverage.py assigns them on `5c0ff782`, inventory in
§16: `0x007C1890`–`0x007CCF80` plus folded bodies such as `0x007D30D0`, `0x007D32D0`, `0x007D5BB0`, `0x007F6610`,
`0x007F8290`, `0x007FF0B0`). Not `cSC4View3DWin`
(view3d-window.md), not the terrain view, span kernels or the DirectX 7 driver (renderer-driver.md).

This spec **extends** view3d.md and does not repeat it. Already specified there and reused by reference:
the field layout (view3d.md §3.1), the 76-slot `cISC43DRender` table (§3.2), the pass brackets (§3.3),
the sub-objects and `DoMessage` (§3.4), and `Draw` with its static/dynamic blocks (§4).

**Completeness: whole owner** (since 2026-10-23, re-render3d7). The owner as tools/coverage.py assigns it on
`5c0ff782` has 153 functions (40,210 bytes); §16 places every one of them. §1–§6 and §9–§12 are proof-backed
(addresses cited, numeric paths and constants read from instructions and the binary's data); §13–§15 add the
bodies that §7 used to list as open, read the same way.

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
| `+0x60` | 1 | the picture is tiled on the ground plane (§13.3); written by both `SetBackgroundPicture` overloads, returned by `GetBackgroundPicture`; when set, `Draw` skips `ClearViewport` (view3d.md §4.1 step 12) |
| `+0x63`, `+0x64` | 1, 1 | the backing-store service supports partial copies; partial copies allowed (§12, §13.4) |
| `+0x90`, `+0x94` | −1, −1 | camera-derived values (§3.1) |
| `+0xE8` | 1 | **lighting enabled** (§3.3) |
| `+0xE9` | 0 | **view fog enabled** (§3.3) |
| `+0xEC`…`+0xFC` | 1.0f ×5 | fog colour/params passed to `TurnOnFog` |
| `+0x100`, `+0x104` | 10000.0f, 15000.0f | fog start / end distance passed to `TurnOnFog` (view3d.md §4.1 step 14) |
| `+0x10C`, `+0x110` | 4096.0f, 4096.0f | the city extent in metres (x, z) that `ClearViewport` and `UpdateVisibleTerrainMetrics` clip against (§13.3, §13.5); nothing in the owner writes them again, so it is always the large-city size *[quirk]* |
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
queued in the vector `+0x1C4`/`+0x1C8`/`+0x1CC` of **0x30-byte** items built by `0x007C2E50` and applied by `ProcessModelChangeQueue` (`0x007C9600`) at the next `Draw` (view3d.md §4.1
step 6) or earlier as below.

Grid-element flag word at element `+6`:

| Bit | Meaning |
|---|---|
| `0x0004` | added to the quad grid |
| `0x0008` | **in the backing store** (static image) |
| `0x0010` | has a queued change (cleared when processed) |
| `0x0001`, `0x0020`/`0x0040`/`0x0100`/`0x0200`/`0x0400`…`0x2000` | object-class bits from the model's properties and draw attributes, set by `UpdateGridElementObjectFlags` (`0x007C2B40`, below) |
| `0x0080` | set by `ChangeModel(…, 1)` from property byte 1 |
| `0x0002` | collected by a pick (§11.5) |
| `0x4000` | collected by `GetAllElements` (§5.1) |

`UpdateGridElementObjectFlags(instance, e)` (re-render3d, 2026-10-21) keeps bits 0–4 and 14–15 and sets the rest
from the instance's properties p (slot 13, `model+0x34`) and draw attributes a (slot 15, `model+0x3C`):

| Flag | Set when |
|---|---|
| `0x0020` | p bit 0 |
| `0x0101` | a bit 2 |
| `0x0100` | a bit 1, or p bit 2 (only when a bits 1 and 2 are clear) |
| `0x0040` | p bit 1 (only when a bits 1 and 2 are clear) |
| `0x0200` | p bit 3 |
| `0x0400`, `0x0800`, `0x1000`, `0x2000` | p bits 4, 5, 6, 7; `0x0800` also when p bits 4–7 are all clear |

`ModelShouldGoIntoBackingStore` (`0x007C5540`): false if `0x100`; true only if `0x20` clear and `0x40` set and one
of: `0x400` with draw-context `+0x14` < props `+0x1AC`; `0x1800` with `+0x14` < props `+0x184`; `0x2000` with
`+0x14` < props `+0x1D4` (props = `*(*(+0x48)+0x18)`, signed compares). The three limits are the integer render
properties 10 `kBackingStoreMaxZoomTerrain` (`+0x1AC`), 9 `kBackingStoreMaxZoomBuildingsProps` (`+0x184`) and 11
`kBackingStoreMaxZoomFlora` (`+0x1D4`) (render-properties.md §3), compared with the draw context's zoom.

Queue item layout (`0x007C2E50`, a plain field copy, `RET 0x14`):

| Offset | Size | Contents |
|---|---|---|
| `+0x00` | 4 | model (AddRef'd by the queuing method) |
| `+0x04` | 1 | kind (table below) |
| `+0x05` | 1 | byte argument, always 0 at the queuing sites seen (`MoveModel`, `InsertModel`, `RemoveModel`); never read |
| `+0x08` | 16 | copied from a caller stack buffer that is not initialised in `MoveModel`; never read by processing |
| `+0x18` | 12 | box minimum x, y, z (floats) |
| `+0x24` | 12 | box maximum x, y, z (floats) |

The box is the grid element's stored box (element `+0x1C`, 24 bytes) for kinds 1 and 2 (the **old** position), and
the empty box min (1, 1, 1) / max (0, 0, 0) for `ChangeModel` kinds (`0x007C9408`–`0x007C9430`). Kinds 0 and 3–5
take the model's current box (model slot `+0x20`) at processing time; kind 1 dirty-rects the stored old box
(mode 2); kind 2 compares the stored old box with the current one (mode = element flag bit `0x0200`, for both rects).
A replacement only needs model, kind and the 6-float box.

Dirty-rectangle **mode** values (the first argument of `AddDirtyRectangle`, §6): 0 = draw this model into the
backing store (`DoSingleModelStaticDrawAddition`, needs the model; flag bit `0x0200` clear), 1 = class bit `0x0200`
set (partial update reason 8), 2 = removal, 3 = kind-4 change, 6 = merged area (§6.1); every mode except 0 ends as
a `DoPartialViewUpdate` of the rect (reason 8 for mode 1, 7 for the others), §6.2.

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


### 5.1 The quad grid (`SC4QuadGrid`, `+0x120`) *[compat]*

*Added 2026-10-21 (re-render3d).* The renderer files every inserted model's grid element in a 256 × 256 grid over
screen-projected space, so a pass only visits the quadrants it can see.

- **Space.** Container coordinates are the fixed-view projection of world positions (view-utilities.md) plus a
  per-rotation offset, x and y: rotation 0 (3000, 4500), 1 (1400, 6400), 2 (5200, 7200), 3 (6800, 5300), each times
  the map scale s (`+0x264`, §3.1 slot 5). `CreateQuadGrid` (`0x007C2390`) makes a 0x20-byte grid (constructor
  `0x005F41F0`) of size 8192 s (`0x00ABA9E8`) with quadrants of 32 s (`0x00ABA9EC`): `SetSize(size, unused,
  quadrant)` (`0x005F40B0`) keeps n = trunc(size ÷ quadrant + 0.5) = **256** quadrants a side and the scale
  1 ÷ quadrant.
- **Cells of a rectangle** (`SetGridRectForElement`, `0x005F2A90`): each edge c ↦ round(c × scale − 0.5), rounded
  to nearest with ties to even (the `+ 1.5 × 2²³` float trick), so an exact multiple k of the quadrant size falls in
  quadrant k when k is even but k − 1 when k is odd *[quirk]*. Lookups clamp the cell range to [0, n − 1]
  (`0x005F29C0`).
- **Quadrants** hold a list of elements and a 16-bit flag word; bit 0 means "holds something the dynamic pass must
  visit".
- `AddElement(e, flags)` (`0x005F3270`): e joins every quadrant of its clamped range, gaining one reference per
  quadrant, and those quadrants OR in `flags`; false when the range is empty; the element count grows by one.
  `RemoveElement(e)` (`0x005F2B30`) takes it out of each quadrant (swapping in the last element) and drops the
  references. `MoveElement(e, rect, flags)` (`0x005F3390`): true at once when the cell range is unchanged; else
  remove, set the new range, add. `SetQuadrantFlagsForElement(e, flags)` (`0x005F2D40`) ORs `flags` into the
  quadrants of e's range. `GetAllElements(out, n)` (`0x005F34B0`): each element once (marked with bit `0x4000`
  while collecting), at most n. `RemoveAllElements` (`0x005F30A0`) releases everything.
- **A grid element** (pool `0x00B0DC34`) holds a reference count, the flag word (`+0x06`), its cell range
  (`+0x08`–`+0x14`), the model instance (`+0x18`) and the instance's last world box (`+0x1C`–`+0x30`). The instance
  keeps a pointer to its element (instance slots 5 and 6, model-instances.md).

### 5.2 The two passes (`DrawStaticView`, `DrawDynamicView`) *[compat]*

*Added 2026-10-21 (re-render3d); view3d.md §4.2 gives what calls them and the pass words.*

**Static** (`DrawStaticView`, `0x007C7370`; pass 1): over the quadrants of the draw-frustum rectangle, each static
element is taken once (in the first visible quadrant of its range). Its sort key is (the instance's sort value, slot
17, its two 16-bit halves added, in the high 16 bits) + the depth of its box along the view (`0x007C2EB0`: the box
centre and half-extents projected to viewport z, × 65535, rounded). The list is sorted **descending** by key.

**Dynamic** (`DrawDynamicView`, `0x007C7830`; pass 4): over the quadrants of the view-frustum rectangle that have
flag 1, each non-static element (or highlighted one) is taken once into one of three lists: elements with `0x0001`
(list C), with `0x0100` (list B), the others (list A, keyed like the static pass); highlighted elements also go to a
fourth list. A is sorted descending, B and C ascending, and they are drawn A, B, C.

**Rounds.** Each element is drawn with instance slot 12 `Draw(context, highlighted = false)`. A draw that returns 2
asks to be drawn again: after each round the draw context's round counter (`+0x04`, 0 at the start of the pass)
goes up, and the elements that asked are drawn again, until none asks or the counter reaches 12. Lot cell textures
use this to draw translucent cells in a later round (lot-models.md §4). In the dynamic pass the three lists
**share** the counter: once it reaches 12, the lists not yet drawn are skipped *[quirk]*. Then the highlighted
elements are drawn once each, with the counter at 0, depth offset 4 and `Draw(context, true)`.
---

## 6. Dirty rectangles: entry points *[compat]*

- `AddDirtyRectangle(mode, int rect[4], model, extra)` (`0x007C9540`): ignored while `+0x109` (full redraw pending)
  is set; copies the rect, and if the rect filter `0x007C2C70` (§6.1, end) accepts and clamps the copy forwards it to
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

**Merge step** (`0x007C8F9A`–`0x007C9172`). It runs only when the process-global byte `0x00B0DC18` is set (initial
value 1 in `.data`; no instruction in the binary writes it) and the queue is **not empty**:

1. `+0x1F8 −= delay` (the deadline base moves earlier by this request's delay).
2. Scan every node in list order. For each: U = union of the node rect and the new rect (`0x005DC980`: min left,
   min top, max right, max bottom); `nodeArea` = node's (bottom − top) × (right − left); `unionArea` likewise for U;
   `newArea` from step 2 of the common path. All 32-bit signed ints.
   - score = nodeArea + newArea − unionArea (the pixels saved by drawing U once instead of both).
   - If the incoming mode is 0, **or** the node's mode (`+0xC`) is 0, subtract also min(newArea, nodeArea)
     (strict `newArea < nodeArea` picks newArea, otherwise nodeArea). A pair with two non-zero modes keeps the full
     score.
   - The node is chosen when score **>** the best so far (strict, so the first of equal scores wins). The best
     starts at −`+0x1E4` (−INT_MAX by default) and the remembered growth at 0; the growth of the chosen node is
     unionArea − nodeArea.
3. Merge is taken if `+0x1D8` (node count) **≥** the global dword `0x00B0DC14` (value **16**, unsigned compare),
   **or** float(best) **≥** the global float `0x00B0DC10` (value **−2500.0**; x87 compare, so a best below −2500
   means "keep separate"). Otherwise fall to the plain append (step 4 of the common path).
4. Taken: `+0x1E0 += growth`; if that is **≥** `+0x1E4`, `DoFullViewUpdate(4)` and return. Otherwise U is
   recomputed for the chosen node, the node's model is released and cleared. Then:
   - growth **> 0** and node count **> 1**: unlink and free the chosen node, `+0x1D8 −= 1`, and call
     `AddDirtyRectangleInternal(6, U, no model, delay 0)` recursively (so U can merge again with another node; the
     recursion adds U's full area to `+0x1E0` again, nothing is subtracted, so `+0x1E0` over-counts and only
     resets when the queue is processed or cleared);
   - otherwise: the node keeps its place, its mode becomes **6** and its rect becomes U.
   The new rect itself is never appended in the taken case, and its model (mode 0) is neither AddRef'd nor stored.

Mode 6 is therefore "merged area": `ProcessDirtyRectangleQueue` treats it as any non-zero mode other than 1
(reason 7, §6.2).

**Rect filter `0x007C2C70`** (used by `AddDirtyRectangle` and by mode-0 nodes in `ProcessDirtyRectangleQueue`):
rejects (returns false, rect untouched) unless right > 0, left < viewport width `+0x68`, bottom > 0 and
top < viewport height `+0x6C` (signed). Otherwise clamps **in place** (left ≥ 0, right ≤ width, top ≥ 0,
bottom ≤ height) and returns true. It does not test left < right: (5, 0, 3, 10) passes.

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

## 7. Where the rest of the owner is specified

This section used to list the owner methods no spec covered. They are now in:
- §11.6: the ray `PickTerrainAlways` clip, with its point-insert helper `0x007C6EC0`;
- §13: the view updates (the scroll copy `DoTranslatedViewUpdate`, which is the writer of `+0x1FD`), clearing and
  backgrounds, the driver, snapshots, clipping, the statistics strings, the terrain metrics;
- §14: the container-rectangle geometry (`WorldToContainerRect`, the frustum and visible rectangles,
  `GetViewportRayContainerRect`, `GetModelContainerRect`) and the quad-grid rebuild;
- §15: `TurnOnLighting` and `TurnOnFog` (draw-context methods), `SetGraphicSystem`, `StaticShutdown`, the
  destructor, and the small helpers (sorts, vector code, thunks, the class factory).

The quad-grid cell query `0x005F2C70` that `Pick` uses belongs to `SC4QuadGrid` (§5.1), not to this owner.

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
13. Quad grid (§5.1): map scale 1 gives 256 × 256 quadrants of 32 units; an edge at exactly 64 (quadrant 2, even)
    falls in quadrant 2, one at exactly 96 (3, odd) in quadrant 2.
14. Rounds (§5.2): an instance whose `Draw` always returns 2 is drawn 12 times in the static pass.
15. Backing store (§5): a building (p bit 5) at zoom 4 with `kBackingStoreMaxZoomBuildingsProps` 5 is static; at
    zoom 5 it is dynamic.
16. Translated update (§13.1): a scroll of more than 14/16 of the width rebuilds the frustum and empties the dirty
    queue instead of shifting the copy.

Merge cases (§6.1), default globals (merge on, 16 nodes, −2500.0), limit `+0x1E4` = INT_MAX, all modes 1 unless said:

- D1. Queue [A (0,0,100,100)], `+0x1E0` = 10000. Add (50,0,150,100): union area 15000, score 5000 ≥ −2500, growth
  5000, one node → A becomes mode 6 (0,0,150,100); `+0x1E0` = 15000; no new node.
- D2. Queue [A (0,0,10,10)]. Add (500,500,510,510): score 200 − 260100 = −259900 < −2500 and 1 < 16 → appended;
  two nodes.
- D3. Queue [A mode 0 with model m, (0,0,100,100)]. Add mode 1 (0,0,100,100): score 10000 + 10000 − 10000 − 10000
  = 0 → merged, growth 0: A becomes mode 6, m released; at processing it is a reason-7 partial update.
- D4. Queue [A (0,0,50,100), B (100,0,200,100)], `+0x1E0` = 15000. Add (40,0,110,100), area 7000: A scores
  5000 + 7000 − 11000 = 1000 (growth 6000), B scores 10000 + 7000 − 16000 = 1000, tie → A (first). Growth > 0 and
  2 nodes → A unlinked, `+0x1E0` = 21000, recursive add of mode 6 (0,0,110,100): against B score 1000, growth 10000,
  one node → B becomes mode 6 (0,0,200,100), `+0x1E0` = 31000 (the true queued area is 20000).
- D5. 16 nodes queued, any score: the 17th add always merges into the best-scoring node.
- D6. Rect filter on 800×600: (−10, 590, 20, 700) → true, becomes (0, 590, 20, 600); (800, 0, 900, 10) → false.

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
written** — it is whatever the stack held (see question RND4); a replacement stores 0.0, so `StopScreenShake`
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

---

## 11. View objects, visibility and picking *[compat]*

Evidence: decomp/cSC43DRender.c cross-checked with re_query asm at the cited addresses. REA was unavailable this
round (no binary open), so nothing here comes from it.

### 11.1 View-object lists

Four circular doubly linked lists of **16-byte** nodes {next, prev, object, key}, sorted by key, plus one
special object pointer:

| Field | `AddViewObject` type | Notes |
|---|---|---|
| `+0x190` | 0 | adding or removing clears `+0x65` (backing store must be saved again) |
| `+0x194` | 2 | same, clears `+0x65` |
| `+0x188` | 3 | |
| `+0x18C` | 5 | |
| `+0x198` | none | terrain view object: set from the global `0x00B43CF4` (AddRef) by the PostCityInit message `0x26D31EC1` when still null |

- `AddViewObject(object, type, key)` (slot 32, `0x007C5D90`): types 3, 5, 0, 2 as in the table (tested in that
  order); any other type returns 0 and does nothing. Otherwise inserts and returns 1.
- `InsertIntoViewObjectList(list, object, key)` (`0x007C5C80`): AddRefs the object, walks from the head and inserts
  the new node **before the first node whose key is greater** (unsigned compare); none → append at the tail. Equal
  keys keep insertion order. Duplicates are not checked: the same object can be inserted twice.
- `FindInViewObjectList(list, object)` (`0x007C3B30`): true when any node holds the object.
- `FindViewObject(object)` (slot 34, `0x007C52F0`): true if object equals `+0x198` (so `FindViewObject(null)` is
  true while no terrain view is set); else searches `+0x18C`, `+0x188`, `+0x194`, `+0x190` in that order.
- `RemoveViewObject(object)` (slot 33, `0x007C5AC0`): if it equals `+0x198`, releases it (when non-null), sets
  `+0x198` = null, result 1. Then removes the **first** matching node from each of `+0x18C`, `+0x188`, `+0x194`,
  `+0x190` (one per list), releasing the object once per removed node; a removal from `+0x194` or `+0x190` clears
  `+0x65`. Returns 1 if anything matched, else 0.

### 11.2 `IsModelVisible`

Both overloads (slot 67 `0x007C5260`, slot 68 `0x007C51D0`) take (x, frustumKind). Slot 68 takes the model itself;
slot 67 takes an object whose `+0x18` holds the model (a grid element). The box starts as the empty box min
(1, 1, 1) / max (0, 0, 0) and is filled by model slot `+0x20` (world bounding box). frustumKind **2** →
`SC4ViewUtilities::IsBoundingBoxInViewFrustum` (`0x007F6770`), any other value →
`IsBoundingBoxInDrawFrustum` (`0x007F6910`), both on the view-utilities object `+0x114`; the result is returned in
AL (`0x007C52A7`).

### 11.3 Screen-point overloads

All three convert the screen point (x, y) to floats (`FILD`) and ask the 3D camera `+0x88` for the eye ray
(`cS3DCamera::GetEyeRay` `0x00800340`, origin and direction both pre-set to the zero vector `0x00B4E1F8`).

| Overload | Needs | Ray handling | Forwards to |
|---|---|---|---|
| `Pick(x, y, filter, out)` slot 65 `0x007C2220` | camera | GetEyeRay result **ignored** | slot 64 `Pick(x, y, origin, dir, filter, out)` |
| `PickTerrain(x, y, out, fallback, extra)` slot 60 `0x007C5000` | camera and `+0x198` | GetEyeRay false → 0 | slot 61 with dir normalised |
| `PickTerrainAlways(x, y, out, extra)` slot 62 `0x007C50F0` | camera and `+0x198` | GetEyeRay false → 0 | slot 63 with dir normalised |

Normalisation (`0x007C506C`–`0x007C50C7`): s = 1.0 (float `0x00B0DC20`) / sqrt(dy² + dx² + dz²), all in x87
extended precision (no intermediate float rounding); each component = component × s rounded to float on store. No
camera: all three return 0.

Cases:

- P1. `AddViewObject(o, 3, 5)`, `(p, 3, 2)`, `(q, 3, 5)` → list `+0x188` order p, o, q; each AddRef'd once.
- P2. `AddViewObject(o, 1, 0)` → returns 0, no AddRef. `AddViewObject(o, 0, 0)` → `+0x65` = 0.
- P3. o added as type 3 and type 0: `RemoveViewObject(o)` removes both nodes, releases twice, clears `+0x65`,
  returns 1; a second call returns 0.
- P4. `PickTerrain` ray: terrain misses at origin y = 100 and hits at y = 104 → result 1 from the second call;
  both miss with fallback 0 → 0 without the ground-plane test.
- P5. `Pick` ray with no candidates and a terrain hit → returns 0, `*out` null.
- P6. Screen `PickTerrain` with eye-ray direction (0, −3, 4) passes (0, −0.6, 0.8).

### 11.4 `PickTerrain(origin, dir, out, fallback, extra)` (slot 61, `0x007C35C0`)

Returns 0 at once when `+0x198` is null. Queries `+0x198` for IID `0x6771477D` (`cISTETerrainView3D2`).

1. Attempt with the ray as given: with the interface, its slot 3 (`+0xC`)(origin, dir, drawContext `+0x11C`, out,
   extra); without, `+0x198` slot 4 (`+0x10`)(origin, dir, drawContext, out) (extra dropped). Both get copies.
2. Miss → the same call again with the origin raised by **4.0** (float add of `0x00A8E17C` to y), direction
   unchanged.
3. Miss and the byte `fallback` is non-zero → `SC4ViewUtilities::RayIntersectsGroundPlane` (`0x007F6B50`) on `+0x114`
   with the **original** origin, dir and out.
4. Releases the interface if it was obtained; returns the hit flag in AL.

### 11.5 `Pick(x, y, origin, dir, filter, out)` (slot 64, `0x007C6830`)

1. `*out` = null; the candidate vector `+0x128..+0x12C` is emptied; maximum distance = FLT_MAX (3.4028235e38).
2. `GetViewportRayContainerRect(x, y)` gives a quad-grid rect; with a quad grid `+0x120`, its cell lists for that
   rect (`0x005F2C70`, at most 6) are walked and each element not yet marked (element byte `+6` bit `0x02`) is
   marked and appended to the candidate vector (each element once).
3. Terrain occluder, only with `+0x198`: `+0x198` slot 4 (origin, dir, drawContext, hit). Hit → maximum distance =
   |hit − origin| (x87 sqrt, stored float). Miss → retry with origin.y + 4.0; a hit there measures the distance from
   the **raised** origin; origin.y is then lowered by 4.0 again (`0x007C6ADA`, float subtract, so y returns to its
   value up to float rounding).
4. For each candidate element in vector order, model = element `+0x18`:
   - with a filter: model slot 3 (`+0xC`) must return an object that answers IID `0x26BB767E`, and the filter's
     slot 3 (`+0xC`) on that interface must return true (the interface is released after the call); otherwise
     skip;
   - model slot 11 (`+0x2C`) is called with the pick record {origin xyz, dir xyz, x, y, filter, max distance}
     (record at `ESP+0x64`). True → result = 1, the previous `*out` is released, `*out` = model, AddRef.
   - clears the element mark bit `0x02`.
   The model's hit test is expected to shrink the record's distance on a hit, so the last model that reports a hit
   is the closest one; the renderer itself does not compare distances.
5. Returns 1 if any model hit, else 0 (the terrain hit only limits the distance; it is not a result).

### 11.6 `PickTerrainAlways(origin, dir, out, extra)` (slot 63, `0x007C7E20`) *[compat]*

1. Returns 0 when `+0x19C` (the terrain) is null. A hit from slot 61 (origin, dir, out, fallback 0, extra) returns 1
   unchanged.
2. Otherwise, when dir.y > −1e-5 (the double `0x00ABAB20`; the ray does not point down) returns 0.
3. **The polyline.** A vector of 16-byte points (x, y, z, w), made with two entries (`0x007C5BE0`): p0 = (origin,
   1.0) and p1 = (origin − dir × (origin.y / dir.y), 1.0), the ray's y = 0 point. Each component is rounded to float
   when stored.
4. **The four planes** (a, b, c, d), with W, D from terrain slots 7 and 8 (`+0x1C`, `+0x20`) and the margin
   0.01 (`0x00ABAB1C`): (1, 0, 0, −0.01), (0, 0, 1, −0.01), (−1, 0, 0, 16·W − 0.01), (0, 0, −1, 16·D − 0.01). So
   x ≥ 0.01, z ≥ 0.01, x ≤ 16·W − 0.01, z ≤ 16·D − 0.01. `16·W − 0.01` is an int-to-float load minus the float
   margin, stored as float.
5. **Clipping**, for each plane in that order, over the pairs (p[k], p[k+1]), k = 0 … n − 2, with n re-read after
   every insert:
   1. d0 and d1 = `((w·d + z·c) + y·b) + x·a` for p[k] and p[k+1], each rounded to float.
   2. **d0 ≥ 0** (p[k] inside): when d1 < 0, insert q = p[k]·(1 − t) + p[k+1]·t, t = d0 / (d0 − d1), after p[k]
      (`0x007C6EC0`, a vector insert of one point), and skip over it.
   3. **d0 < 0** (p[k] outside): project p[k] onto the plane: s = `(((a·x + c·z) + b·y) + w·0) + d` (in x87
      precision, the w term is multiplied by 0), then x −= a·s, y −= b·s, z −= c·s, each rounded to float on store;
      w is unchanged. When d1 > 0 (strictly), insert the crossing point q, computed as above from the point's values
      **before** the projection, after it, and **stop this plane** (the remaining pairs are not visited) *[quirk]*.
   4. After the pairs: when the last point's d < 0, it is projected the same way.
6. **The segments.** For k = 0 … n − 3: slot 61 (p[k], p[k+1] − p[k], out, 0, extra), with all four components of
   the difference (w gives 0). A hit is accepted only when (out − p[k+1])·(p[k+1] − p[k]) ≤ 0 over x, y, z (not
   beyond the segment's end). Then the last segment p[n − 2] → p[n − 1] is tried with fallback 1, accepted as is.
7. **On a hit**, it is slid back onto the original ray at its own height: t = (out.y − origin.y) / dir.y,
   out.x = t·dir.x + origin.x, out.z = t·dir.z + origin.z. Returns 1, else 0. The point vector is freed.

Case: origin (10, 50, 10), dir (−1, −1, 0) (normalisation is the caller's), city 64 × 64 cells. p1 = (−40, 0, 10)
is outside plane 0: d0 = 9.99 ≥ 0, d1 = −40.01 < 0, t = 0.1998, so q = (0.01, 40.01, 10) is inserted after p0 and
skipped, which ends the pairs; the last point p1 is outside and is projected to (0.01, 0, 10). The other planes
leave the line. The segments are origin → q (fallback 0), then q → (0.01, 0, 10) (fallback 1).

---

## 12. Backing store *[compat]*

Process globals: the backing-store service `0x00B4E1F0` (set by `StaticSetGDriver`, released by `StaticShutdown`),
buffer handles `0x00B4E1E4` (kind 0) and `0x00B4E1E8` (kind 1), and the driver `0x00B4E1EC`. Service slots used:
4 (`+0x10`) create buffer(kind) → handle, 5 (`+0x14`) free(handle), 6 (`+0x18`) save and 7 (`+0x1C`) restore, both
(handle, srcX, srcY, w, h, dstX, dstY) → bool. All methods below are process-wide in effect (static buffers).

- `CreateBackingStore` (`0x007C27A0`): no service → false. Creates each missing buffer (kind 0, kind 1). If only
  one of the two exists it is freed and the call fails. With both: gets the viewport rect from the driver slot
  `+0x130` and **probes** the full size: save buffer 0, restore buffer 0, save buffer 1, restore buffer 1, each
  (0, 0, w, h, 0, 0); the first failure frees both (handles = 0). Returns handle 0 ≠ 0.
- `DestroyBackingStore` (`0x007C2910`): no service → 0. Frees and zeroes each existing handle; returns 1.
- `+0x64` (partial saves allowed) is set by `Init` when the driver supports partial copies and boolean 30
  `kNoPartialBackingStoreCopies` is off (re-render3d, 2026-10-21).
- `SaveBackingStore(x, y, w, h)` (`0x007C2960`), with the shake offsets sx = `+0x258`, sy = `+0x25C`
  (§10.3):
  1. When either offset is non-zero: srcX = x − sx; if srcX < 0 → dstX = x + sx, w −= sx, srcX = 0, and w ≤ 0
     returns **1**; else if srcX + w > `+0x68` → w = `+0x68` − srcX, w ≤ 0 returns 1. Same for y with sy and
     `+0x6C`. Without offsets src = dst = (x, y).
  2. Returns 0 unless w > 0, h > 0, the service and both handles exist.
  3. Returns 0 if byte `+0x64` (partial saves allowed) is clear and (w, h) is not exactly (`+0x68`, `+0x6C`).
  4. Slot 6 on buffer 0 then buffer 1 with (srcX, srcY, w, h, dstX, dstY); returns 1 (results ignored).
- `RestoreBackingStore(srcX, srcY, w, h, dstX, dstY)` (`0x007C2AC0`, pops 0x18): needs the service and both handles
  (else AL = 0); slot 7 on buffer 0 then buffer 1 with the same six values; returns 1.
- `RestoreBackingStore()` (`0x007C38D0`): no shake offsets → `Restore(0, 0, +0x68, +0x6C, 0, 0)`. Otherwise for
  sx ≥ 0: srcX = 0, dstX = sx; sx < 0: srcX = −sx, dstX = 0; the same for y; w = `+0x68` − |sx|,
  h = `+0x6C` − |sy|. Returns the inner result. The saved image is thus shifted by the shake offsets.
- `ReevaluateBackingStoreObjectFlags` (`0x007C55C0`): with a quad grid `+0x120`, takes all its elements
  (`SC4QuadGrid::GetAllElements` into a temporary array sized by the grid's element count) and sets element bit
  `0x08` = `ModelShouldGoIntoBackingStore(element)` (§5). Then sets bit 0 of the 16-bit word at `+0xC` of every
  16-byte cell record in every row vector of the grid (`+0x14`..`+0x18`, 12-byte rows) *[inferred: marks every cell
  changed]*. No dirty rects, no redraw.

Cases: B1. `SaveBackingStore(0, 0, 800, 600)` on 800×600, no shake → slot 6 twice with (0,0,800,600,0,0), returns 1.
B2. `+0x64` clear, `Save(10, 10, 50, 50)` → 0, nothing copied. B3. Shake (3, −2): `Restore()` →
`Restore(0, 2, 797, 598, 3, 0)`. B4. Shake sx = 5, `Save(2, 0, 10, 600)` with `+0x64` set: srcX = −3 <0 → dstX = 7,
w = 5, srcX = 0.

---

## 13. View updates, clearing, the driver and the rest *[compat]*

*Added 2026-10-21 (re-render3d) as summaries; the bodies were read on the instructions on 2026-10-23
(re-render3d7).* W and H are the viewport size `+0x68`, `+0x6C`; "the driver" is the global `0x00B4E1EC`
(renderer-driver.md; its slots 11 `Clear`, 12 `ClearColor`, 13 `ClearDepth`, 74/75 `SetViewport`); `dc` is the draw
context `+0x11C`. Render-property booleans are read as bytes of the properties object's table (render-properties.md
§2): boolean *i* is the byte at `+0x1C + 0x20·i`.

### 13.1 Scrolling, zooming and rotating

**`AttemptTranslatedViewUpdate(dx, dy)`** (`0x007CABE0`):
1. The scroll path needs all of: |dx| < (14·W) / 16 and |dy| < (14·H) / 16 (C integer division), partial copies
   allowed (`+0x64`), the backing store on (`0x00B0DC19`), a valid saved copy (`+0x65`) and buffer 0
   (`0x00B4E1E4`).
2. Then every queued dirty rectangle (list `+0x1D4`, rect at node `+0x10`…`+0x1C`) moves by (−dx, +dy), the view
   scrolls (`DoTranslatedViewUpdate`, below), and the result is 1.
3. Otherwise: dc `+0x20` := 3, `+0x109` := 1 (full update due), `+0x65` := 0, the draw frustum is rebuilt from the
   viewport rect (`+0x70`, `+0x78`), `+0x160`/`+0x174` cleared, the dirty queue emptied (`0x007C6D80`); result 0.

**`DoTranslatedViewUpdate(dx, dy)`** (`0x007CA4D0`). Nothing when the guard `+0x108` is set; it is set for the call
and cleared at the end.
1. dc `+0x20` := 3 (the update reason). The shake offsets `+0x258`, `+0x25C` := 0; the camera control gets
   `SetCameraPositionTweakAdd(+0x80, +0x84)` (`0x007CDA20`, the subpixel bias); `SetDefaultRenderStateUnilaterally`;
   slot 24 (`Set3DCameraView`).
2. With view fog on (`+0xE9`): `TurnOnFog(dc, 1, &+0xEC, +0x100, +0x104)`. Then `TurnOnLighting(dc, +0xE8 ≠ 0)`
   (§15.1).
3. **The copy.** Both backing buffers are restored (§12 service slot 7, `0x00B4E1E4` then `0x00B4E1E8`, results
   ignored) with
   - source (max(dx, 0), max(−dy, 0)),
   - size (W − |dx|, H − |dy|),
   - destination (max(−dx, 0), max(dy, 0)).

   So the picture moves by (−dx, +dy).
4. **The uncovered strips**, drawn by `UpdateViewportRect` (§4) in this order:
   1. dx < 0: the column (0, 0, −dx, H); dx > 0: (W − dx, 0, W, H); dx = 0: the row, (0, H + dy, W, H) for dy < 0
      or (0, 0, W, dy) for dy > 0;
   2. when both are non-zero, the row beside the column: dy < 0 → (max(−dx, 0), H + dy, W − max(dx, 0), H);
      dy > 0 → (max(−dx, 0), 0, W − max(dx, 0), dy); else nothing (an empty rect).

   (0, 0) copies nothing and passes two **uninitialised** rectangles *[quirk]*; the callers never pass it
   *[inferred]*.
5. Driver slot 75 (`SetViewport()`, the full viewport); `UnsetClipRect` on the 3D camera `+0x88` and the 2D camera
   `+0xD8` (§13.4).
6. **The stale area**: `+0x200`…`+0x20C` := the destination rectangle of the moved picture (left, top, right,
   bottom), `+0xDC` := 0 (camera mode), **`+0x1FD` := 1**, and the timer `+0x218` is reset. `DoViewCleanup` (§6.2)
   then redraws that rectangle band by band (this is the only writer of `+0x1FD` = 1; the constructor and
   `DoViewCleanup` clear it).
7. `+0x65` := `SaveBackingStore(0, 0, W, H)` (§12).
8. When boolean 16 `kShowUpdateRegions` is off: rebuild the draw frustum from the viewport rect and clear
   `+0x160`/`+0x174`.
9. Slot 24 again; when boolean 1 `kRenderDynamicView` is on, the dynamic triple (`DrawPreDynamicView`,
   `DrawDynamicView`, `DrawPostDynamicView`).
10. `TurnOnLighting(dc, 0)`, `TurnOnFog(dc, 0, null, FLT_MAX, FLT_MAX)`, slot 26 (`SetUICameraView`), dc `+0x20` := 0.

Case: W × H = 800 × 600, (dx, dy) = (10, −5): source (10, 5), size 790 × 595, destination (0, 0); strips
(790, 0, 800, 600) then (0, 595, 790, 600); `+0x200` = (0, 0, 790, 595).

**A zoom change** `DoViewZoomChange3(oldZoom, oldRot, newZoom, newRot)` (`0x007CAA60`, pops 16). Nothing when the
zoom is unchanged. Otherwise:
1. Process the change queue if it is not empty (§5).
2. Slot 57 `SetBackingStoreEnabled(newZoom < integer 8 kBackingStoreMaxZoomAll)` (props `+0x18` table, `+0x15C`).
3. `+0x109` := 1, `+0x65` := 0; dc `+0x10` := (float) newZoom, dc `+0x14` := newZoom, dc `+0x18` := newRot.
4. `RebuildQuadGrid(oldZoom, oldRot, newZoom, newRot)` (§14.3); rebuild the draw frustum;
   `ReevaluateBackingStoreObjectFlags` (§12); clear `+0x160`/`+0x174`; `+0x15C` += 1; empty the dirty queue.

**A rotation change** `DoViewRotationChange2(oldZoom, oldRot, newZoom, newRot)` (`0x007CAB30`) is the same, keyed on
the rotation, without step 2 and without the re-evaluation; dc `+0x10` and `+0x14` take newZoom, `+0x18` newRot.

### 13.2 Single-model additions

A mode-0 dirty rectangle with a model (§5) draws only that model into the static view.
`DoSingleModelStaticDrawAddition(model, rect)` (`0x007C32A0`) does nothing for an empty rect (left ≥ right or
top ≥ bottom). Otherwise:
1. dc `+0x20` := 8; slot 24; the round dc `+0x4` := 0.
2. Draw the model (model slot 12, `+0x30`, with (dc, 0)) and add 1 to the round, while the draw returns 2 and the
   round is at most 11 (at most 12 draws).
3. When `+0x65` (a valid copy) is set: `+0x65` := `SaveBackingStore(rect)` (§12).
4. dc `+0x20` := 0.

### 13.3 Clearing and backgrounds

**The picture fields:**
- `+0x4D`: enabled;
- `+0x50`: the image;
- `+0x54`: its texture;
- `+0x58`, `+0x5C`: its width and height;
- `+0x60`: tiled on the ground plane (else over the screen).

**`ClearViewport(full)`** (slot 47, `0x007C3F80`, pops 4). The **corner test** used twice below takes the x–z box of
the four draw ground points (view-utilities `+0x2E4`… via `0x007F8290`; box `0x005D6BA0`). It is true when `full` is
set, or when the box leaves [0, `+0x10C`] × [0, `+0x110`] (strict comparisons: min < 0 or max > 4096).

1. **No picture** (picture disabled, no texture, or the view utilities not valid, their byte `+0x00`):
   1. *need colour* := the corner test.
   2. When the draw frustum is narrowed (view utilities `+0x228`): `DrawViewportRectangle(draw rect, &+0x1A8, 1,
      need colour)` (slot 49) and return.
   3. Else, when *need colour* and the background is not transparent (`+0x62`): colour and depth masks on,
      `Clear(0x5000)` (colour and depth).
   4. Else: depth mask on, `Clear(0x1000)` (depth).
2. **With the picture**:
   1. Save dc's lighting state (`0x007D2A10`); under boolean 28 `kRenderSnapshot`, `TurnOnLighting(dc, 0)`.
   2. `SetDepthOffset(0)`.
3. **Screen tiling** (`+0x60` clear), only when the draw frustum is not narrowed: one quad over
   (0, 0) – (tx·pw, ty·ph), with tx = ⌈W / pw⌉, ty = ⌈H / ph⌉ (pw, ph = `+0x58`, `+0x5C`).
   - Depth −32500 (`0xC6FDE800`) and texture coordinates 0 … tx, 0 … ty.
   - State: colour, depth mask and depth test on, depth function 7, texturing on, blending and alpha test off.
   - The texture with filtering (1, 1) and wrap (3, 3).
   - Slot 25 (`Set2DCameraView`); driver vertex array (2, 20-byte stride); `DrawPrims(1, 0, 4)`.
4. **Ground tiling** (`+0x60` set):
   1. Alpha test off. Narrowed frustum: `DrawViewportRectangle(draw rect, &+0x1A8, 1, 0)` (a depth-only fill), then
      colour mask and depth test on, depth function 3. Else: depth mask on, `Clear(0x1000)`, colour mask on, depth
      test off.
   2. Slot 24 (`Set3DCameraView`).
   3. When the corner test holds:
      - State: depth mask off, texturing on, blending off; the texture as in step 3.
      - Tiles: the block size b is `0x00ABAAE4`[dc zoom `+0x14`] = 1024, 512, 256, 128, 64, 32 for zooms 0–5. The
        tiles cover the outer visible-area rectangle (view-utilities.md §6), its minimum rounded down and its
        maximum up to multiples of b.
      - Each tile (x … x + b, z … z + b) at y = 0, with texture coordinates 0 … 1, is drawn when `full`, or
        x < −4, or x ≥ 4096 − 4, or z < −4, or z ≥ 4096 − 4. So only the tiles outside the city are drawn.
      - Then **the two horizon fills**, each `DrawViewportRectangle(rect, &+0x1A8, 0, 1)`. The ray from frustum
        corner 3 along (corner 2 − corner 3) (view-utilities `+0xCC`, `+0xC0`) meets the ground (`0x007F6AB0`) at a
        screen point (`0x007F78E0`) with y: when y < H, fill (0, y, W, H). The ray from corner 7 (`+0xFC`) in the
        same direction gives y′: when y′ > 0, fill (0, 0, W, y′). Both are truncated to integers (`_ftol2`).

      When the corner test fails, neither the tiles nor the fills are drawn.
5. End (with the picture): under `kRenderSnapshot`, restore the saved lighting state.

**`ClearViewportWithSolidColor(colour[4], depth, unused)`** (slot 48, `0x007C3350`, pops 12):
1. `ClearColor(colour)`; colour mask on.
2. With `depth`: depth mask on, `ClearDepth(+0x1B8)`, `Clear(0x5000)`. Without: `Clear(0x4000)`.
3. `ClearColor(+0x1A8…+0x1B4)` (the stored clear colour back).

No camera is set.

**`DrawViewportRectangle(rect, rgba, depthMask, useColour)`** (slot 49, `0x007C3400`, pops 16). A quad at depth
−32500 over the integer rect (vertices top-left, bottom-left, top-right, bottom-right).
- Colour: with `useColour`, the four floats become the bytes of an ARGB dword. Each byte is
  `(v − (v >> 8)) & 0xFF`, v = `bits(c + 49152.0) − 0x47400000` (c × 256 rounded to nearest even), so 1.0 → 255 and
  0.5 → 128. Without it, `0xFF000000` (`0x00B0DC24`).
- State: texturing off, depth test on, depth function 7, depth mask := `depthMask`, colour mask := `useColour`.
- Slot 25 (`Set2DCameraView`); `DrawPrims(1, 1, 4)` (position and colour).

So `useColour` = 0 makes it a depth-only fill.

**`DrawTexturedViewportRectangle(rect, texture, z, depthMask, useTexture)`** (slot 50, `0x007C2400`, pops 20). The
same quad at depth −z.
- Texture coordinates (0, 0), (0, 1), (1, 0), (1, 1).
- State: texturing and colour mask := `useTexture`, depth test on, depth function 3, depth mask := `depthMask`, the
  texture.
- Slot 25; `DrawPrims(1, 2, 4)` (position and texture coordinates).

**Pictures:**
- `SetBackgroundPicture(image, groundTiled)` (slot 54, `0x007C2610`):
  1. `+0x60` := `groundTiled`. The same image: return 1.
  2. Release the old image (`+0x50` := 0); AddRef the new one; free the old texture
     (`nsTexUtil::ReleaseTexture(driver, +0x54)` when there is a driver; `+0x54` := 0).
  3. A null image returns 1.
  4. With a driver and `nsTexUtil::SetupTexture(image, driver, &+0x54, 0, 0, 0, 0)`: `+0x50` := image; `+0x58`,
     `+0x5C` := image slots 9 and 10 (width, height); return 1.
  5. Else return 0. The AddRef'd image is then neither kept nor released *[quirk: a leaked reference]*.
- `SetBackgroundPicture(texture, w, h, groundTiled)` (slot 53, `0x007C26D0`): `+0x60` := `groundTiled`. When the
  texture differs from `+0x54`: release the image, free the old texture, then `+0x54`, `+0x58`, `+0x5C` := texture,
  w, h. The same texture keeps the old size *[quirk]*. Returns 1.
- `GetBackgroundPicture(&groundTiled)` (slot 52) and `EnableBackgroundPicture` (slot 51) are in §3.4.

### 13.4 The driver, snapshots, clipping and statistics

**`StaticSetGDriver(driver)`** (`0x007C3810`). Nothing for the current driver. Otherwise:
1. With an old driver, `DestroyBackingStore` (§12). AddRef the new driver; release the old; store it.
2. Release and clear the backing-store service `0x00B4E1F0` and the snapshot service `0x00B4E1F4`.
3. With a driver: QI it for `0x669565FE` into `0x00B4E1F0` and `0xE69BFE2A` into `0x00B4E1F4`, then
   `CreateBackingStore` (§12).

**`SetGDriver(driver)`** (slot 7, `0x007C3BE0`):
1. `StaticSetGDriver`.
2. With a backing-store service: `+0x63` := its slot 9 (partial copies supported); `+0x64` := `+0x63` and not
   boolean 30 `kNoPartialBackingStoreCopies`.
3. With a 3D camera: driver slot 64 (`GetVideoModeInfo` into a 52-byte local, unused) and
   `SetDepthSize(65536.0)`.

**`TakeSnapshot(out, unused)`** (slot 23, `0x007C2530`): without the snapshot service, AL = 0. Else `*out` := service
slot 3 (0, 0, W, H, 0) (an image), AL = image ≠ 0.

**`TakeSnapshot(target, arg)`** (slot 22, `0x007C2570`):
1. Slot 23 (&image, arg). On success: a new `cGZGimexFactory` (0x38 bytes, `GlobalHeapAlloc`; a failed allocation is
   not tested *[quirk]*) is AddRef'd.
2. Its slot 8 (target, image, 100) writes the image (100: the quality argument *[inferred]*); its byte result is
   the return value.
3. Release the factory and the image.

**`SetClipRect(rect)`** (`0x007C3C60`): only under boolean 24 `kUseUpdateRectClipping`.
1. Driver slot 74 `SetViewport(left, H − bottom, right − left, bottom − top)` (a bottom-left origin).
2. `cS3DCamera::SetSubview` with the same four values (unsigned to float) on the 3D (`+0x88`) and 2D (`+0xD8`)
   cameras.
3. `+0xDC` := 0.

**`UnsetClipRect`** (`0x007FF0B0`) is a **camera** method (ECX = the camera):
- the subview offsets `+0x18`, `+0x1C` := 0 and the subview flag `+0x14` := 0;
- the size `+0x20`, `+0x24` := the full size `+0x08`, `+0x0C`;
- the camera's change counter `+0x17C` += 1.

**Statistics.**
- `GetStatsString1(str)` (slot 73) calls the string's slot 27 (a printf) with
  `"SPS:%.1f BST:%u BRT:%u VCT:%u PrVDT:%u TeDT:%u PoVDT:%u ToDT:%u"` and the float global `0x00B4E218`, then
  `+0x288`, `+0x28C`, `+0x290`, `+0x294`, `+0x2AC`, `+0x298`, `+0x2BC`.
- `GetStatsString2(str)` (slot 74) uses `"QGT:%u MST:%u MTC:%u, MC1:%u MDC:%u AIT:%u AMT:%u ART:%u TPT%u MPT:%u"`
  with `+0x2A4`, `+0x2A8`, `+0x2B0`, `+0x2B4`, `+0x2B8`, `+0x2C0`, `+0x2C4`, `+0x2C8`, `+0x2CC`, `+0x2D0`.

In the owner's code only the constructor writes these counters (to 0, all but `+0x2B0`), so the strings report
zeros *[inferred: no writer elsewhere was searched]*.

### 13.5 `UpdateVisibleTerrainMetrics()` (`0x007C4910`)

Called on message `0x4A9F2FFE` and after city init. It writes the view utilities' visible-terrain fields `+0x3DC`
(centre x, y, z), `+0x3E8` (half size) and `+0x3EC`, `+0x3F0`, `+0x3F4`, `+0x3F8` (min x, min z, max x, max z).
1. Terrain slot 21 (`MinHeight`) is called and its result dropped.
2. **Five terrain picks** (slot 60, screen point, fallback 1, extra 0) at (W/2, H/2), (W/2 + 100, H/2),
   (W/2, H/2 + 100), (W/2 − 100, H/2), (W/2, H/2 − 100) (W/2, H/2 are `int / 2`). Each output is first the zero
   vector `0x00B4E1F8`. A pick **counts** when its y > 1.0, so a miss, and a hit below 1 m, do not count.
3. **When any counts**: centre := their mean (sum × 1.0 / count). A sixth pick at (W, H) (unchecked) gives
   half size := max(|p.z − centre.z|, |p.x − centre.x|).
4. **When none counts**: walk the city's border every 4 cells (`BoundaryIterator` `0x0060CEE0` over
   (0, 0) – (cellsX / 4, cellsZ / 4), terrain slots 7 and 8, negative counts as 0).
   - Each point (64·i, altitude at vertex (4·i, 4·j) (terrain slot 14), 64·j) is projected to the viewport
     (`0x007F7850`).
   - Of those strictly inside (0, W) × (0, H), keep the one with the smallest squared distance to the screen centre
     (strict <).
   - centre := that point, else (0, 0, 0); half size := (16 << zoom) / 2 (view utilities `+0x0C`) when one was found,
     else 1.0.
5. **The rectangle**: min = centre − half, max = centre + half (x and z). A max that is not above its min becomes
   min + 1.
6. Then `ClipWorldPositionToCityPosition(centre)` (`0x007F7710`; after the rectangle, which does not move with it).
7. Clamp: a min < 0 → 0; a max ≥ 4096 (`+0x10C`, `+0x110`) → 4096.

---

## 14. Container rectangles and the quad-grid rebuild *[compat]*

The quad grid (§5.1) files elements by a 2D "container" rectangle in fixed view coordinates. `size` is the grid size
`+0x124`; `rot` is the view-utilities rotation `+0x10` (0–3); offX, offY are the per-rotation margins `+0x268[rot]`,
`+0x278[rot]` (§9.1 step 3).

### 14.1 `WorldToContainerRect(min, max, out, rot)` (`0x007C1B30`, pops 16)

1. Two points A and B from the box, padded along one axis by fractions of the extent e of the other one:
   a = 0.35355338 × e, b = 0.1464466 × e (`0x00ABAA40`, `0x00ABAA3C`).

   | rot | e | A | B |
   |---|---|---|---|
   | 1 | max.x − min.x | (max.x − b, max.y, min.z − a) | (min.x + b, min.y, max.z + a) |
   | 2 | max.z − min.z | (max.x + a, max.y, max.z − b) | (min.x − a, min.y, min.z + b) |
   | 3 | max.x − min.x | (min.x + b, max.y, max.z + a) | (max.x − b, min.y, min.z − a) |
   | other | max.z − min.z | (min.x − a, max.y, min.z + b) | (max.x + a, min.y, max.z − b) |

2. Each point goes to fixed view coordinates (`ProjectPointToFixedViewCoordinates` `0x007F6710`); then
   x += offX, y := −y + offY.
3. out = (A.x − 2, A.y − 2, B.x + 2, B.y + 2).
4. Clamp, in order:
   1. left ≤ 0.5 → 0.5, top ≤ 0.5 → 0.5;
   2. right ≥ size − 0.5 → size − 0.5, bottom likewise;
   3. right ≤ left → left := right − 0.1; bottom ≤ top → top := bottom − 0.1;
   4. right − left > size / 2 → right := left + 1; bottom − top > size / 2 → bottom := top + 1.

Step 4.4 makes a box wider than half the grid a 1-unit sliver at its left or top edge *[quirk]*.

### 14.2 The other rectangles

- `GetModelContainerRect(model, out, rot)` (`0x007C3F10`, pops 12): box := (min (1, 1, 1), max (0, 0, 0)), then
  model slot 8 (`+0x20`) fills it; `WorldToContainerRect(&min, &max, out, rot)`.
- `MoveGridElement(element, model)` (`0x007C3790`, pops 8):
  1. Model slot 8 writes the box into element `+0x1C` (min) / `+0x28` (max).
  2. `WorldToContainerRect` with the current rotation.
  3. `SC4QuadGrid::MoveElement(element, rect, flag)`, flag = 1 unless the element is in the backing store (`0x08`)
     without bit `0x80`.
  4. On failure the model's element is cleared (model slot 6 (0)). Returns the result.
- `GetViewFrustumContainerRect(out)` (`0x007C1E10`): the viewport corners (0, 0) and (view-utilities `+0x28`,
  `+0x2C`), through `ProjectPointFromViewportCoordinatesToFixedViewCoordinates` (`0x007F8390`); out =
  (p0.x + offX, −p0.y + offY, p1.x + offX, −p1.y + offY). No clamp.
- `GetDrawFrustumContainerRect(out)` (`0x007C1EF0`): the same with the draw rectangle's corners (view-utilities
  `+0x22C`, `+0x230`) and (`+0x234`, `+0x238`).
- `GetVisibleContainerRect(out, which)` (`0x007C1FD0`):
  - `which` 0 or 1: the draw rectangle, cached at `+0x178` (valid flag `+0x174`); any other value: the view
    rectangle, cached at `+0x164` (flag `+0x160`).
  - It is computed once per invalidation (§4 clears both flags). It is clamped when computed: a negative left or top
    → 0, a right or bottom above size − 1 → size − 1.
- `GetViewportRayContainerRect(x, y, out)` (`0x007C20E0`, pops 12):
  1. (x, y, 0) is unprojected to world coordinates (`0x007F79B0`), then projected to fixed view coordinates with the
     margins: (px, py).
  2. out = (px, py, px + 2, py + 2).
  3. px < 0 → left 0; left > size − 2 → size − 2; right < left + 1 → left + 1. The same for y.

### 14.3 `RebuildQuadGrid(oldZoom, oldRot, newZoom, newRot)` (`0x007C5370`, pops 16)

Nothing when the zoom and the rotation are both unchanged, or the grid holds no element. Otherwise:
1. Take all elements (`SC4QuadGrid::GetAllElements` into a temporary array) and make a new grid (`CreateQuadGrid`,
   §9.1).
2. For each element, in the array's order:
   1. Make a new `SC43DGridElement` from the element pool (`0x00B0DC34`, free list at `+8`, count `+0xC`), holding one
      reference. Copy the flags (`+6`), the model (`+0x18`) and the box (`+0x1C`–`+0x30`).
   2. Its rect := `GetModelContainerRect(model, newRot)`; the model's element := the new one (model slot 6).
   3. Add it to the new grid with the flag of §14.2. On failure, retry once with the rect (0, 0) – (1, 1)
      (`+8`…`+0x14`); a second failure clears the model's element.
   4. Drop the construction reference (a 16-bit count at `+4`; at 0 the element deletes itself).
3. Destroy the old grid (its destructor and `GlobalHeapFree`), keep the new one in `+0x120`, free the array.

---

## 15. Draw-context toggles, lifecycle remainder and helpers

### 15.1 `TurnOnLighting` and `TurnOnFog`

Both run with ECX = the draw context (dc). Its driver is dc `+0x30`.
- **`TurnOnLighting(on)`** (`0x007D30D0`): nothing when dc `+0x90` already equals `on`.
  - On: the lighting manager (dc `+0x44`), when present, fills dc `+0x94`…`+0x9C` (its slot 10), and those three
    floats are the colour.
  - Off: the colour is (1, 1, 1).
  - Then the driver's `ColorMultiplier(r, g, b)` (slot 32) and dc `+0x90` := `on`.
- **`TurnOnFog(on, colour, start, end)`** (`0x007D32D0`, pops 16):
  1. When dc `+0xA0` differs from `on`: on → driver `Enable(6)` and `Fog(0, 2)` (linear vertex fog); off →
     `Disable(6)`. Store `on`.
  2. A colour that differs from dc `+0xA4`…`+0xAC` (any NaN differs): `Fog(1, colour)`, then cache it.
  3. A start other than FLT_MAX (`0x00ABB4D0`, "unchanged") that differs from dc `+0xB4`: `Fog(3, &start)`, cache.
     The end likewise with dc `+0xB8` and `Fog(4, …)`.

### 15.2 Lifecycle remainder

- `SetGraphicSystem(gs)` (`0x007C5D00`, pops 4):
  1. AddRef `gs`; release `+0x14` and `+0x18`; `+0x18` := gs.
  2. QI `gs` for `0x0073283C` into `+0x14`.
  3. With `+0x14`: its slot 29 fills the two strings `+0x1C` and `+0x30`. Without it, both strings are emptied.

  The old `+0x14` is released but not cleared. A null `gs`, or a QI that leaves its output alone, therefore calls
  slot 29 on the released pointer *[quirk]*.
- `StaticShutdown()` (`0x007C3090`): once, while the byte `0x00B4E1E0` is set, clear it.
  1. With a driver: `DestroyBackingStore`, release the driver, clear `0x00B4E1EC`.
  2. Release and clear `0x00B4E1F0` and `0x00B4E1F4`.

  Returns 1.
- `HasBackingStore()` (`0x007C2790`): buffer 0 (`0x00B4E1E4`) ≠ 0.
- The destructor body (`0x007CA020`, under the deleting destructor `0x007CB090`):
  1. Restores the three vtables, then `Shutdown` (§9.3).
  2. Clears and frees the view utilities (`+0x114`). Frees the draw context's six arrays (`0x007D5BB0`: `+0x54`,
     `+0x60`, `+0x6C`, `+0xE0`, `+0x12C`, `+0x1EC`) and the draw context (`+0x11C`).
  3. Frees the member lists and vectors, including the dirty-rect list (`0x007C7020`: each node's arrays `+0x30`
     and `+0x24`, then the node).
  4. Releases the properties object `+0x48`; frees the strings `+0x30` and `+0x1C`.

  Nothing is observable beyond the releases.
- `GZCOM_CreatecSC43DRender` (`0x007CB070`): `GlobalHeapAlloc(0x2E8)` and the constructor (§1.3), or null.
- The thunks `0x007C9FE0` / `0x007C9FF0` adjust this by −4 / −8 into `QueryInterface` (`0x007C1890`);
  `0x007CA000` / `0x007CA010` do the same into `Release` (`0x007C3060`).

### 15.3 Helpers

- **The draw-list sorts** (§5.2). Entries are 8 bytes, sorted on the unsigned dword at `+4`.
  - `SortDrawListDescending` / `Ascending` (`0x007C56D0` / `0x007C5710`): while n − 1 > 0: p = partition(list, 0,
    n − 1); sort(list, p); list += p + 1 entries; n −= p + 1.
  - The partitions `0x007C3950` / `0x007C3A10`, (list, lo, hi):
    1. swap entry (lo + hi) / 2 into lo; pivot = its key; last = lo;
    2. for i = lo + 1 … hi: when key[i] > pivot (descending; < for ascending): last += 1 and swap(last, i);
    3. swap(lo, last); return last.

    Equal keys stay on the right of the pivot, and the order among them is not kept.
- **The model-change-queue vector** (0x30-byte items, §5):
  - `0x007C5750`: copy-construct a range;
  - `0x007C57E0`: fill n copies;
  - `0x007C5870`: assign a range (unrolled by 4), used by the flush and by the erase in processing;
  - `0x007C62B0`: insert n copies at a position, growing to size + max(size, n) when full.
- `AccumulateViewTranslation()` (`0x007C3230`):
  1. With the camera control (`+0x8C`): dx = cc `+0x98` − cc `+0xA8`, dz = cc `+0x9C` − cc `+0xAC`.
  2. `+0xE0` += round(dx), `+0xE4` += round(dz). The rounding is to nearest even, `bits(d + 12582912.0) −
     0x4B400000`.
  3. `+0x15C` += 1.
- **Points**:
  - `0x007C5BE0(n)`: a vector of n 16-byte points, each four copies of the zero global `0x00B4E1F8`;
  - `0x007C6EC0(pos, &p)`: insert one point before `pos`, growing through `0x004F5D50` when full; returns the new
    point's address.
- **View-utilities readers** called from here:
  - `0x007F6610`: the valid byte `+0x00`;
  - `0x007F8290(a, b, c, d)`: the four draw ground points as (x, z) pairs from `+0x2E4`, `+0x2F0`, `+0x2FC`,
    `+0x308` (x at the point, z at +8).

---

## 16. Function inventory (owner `cSC43DRender`, 153 functions on `5c0ff782`)

Specified in §1–§12 or view3d.md, at their addresses there (71):
`0x007C1890`, `0x007C1980`, `0x007C1A00`, `0x007C1A50`, `0x007C1AC0`, `0x007C2220`, `0x007C2330`, `0x007C2390`,
`0x007C27A0`, `0x007C2910`, `0x007C2960`, `0x007C2AC0`, `0x007C2B40`, `0x007C2C70`, `0x007C2CE0`, `0x007C2EB0`,
`0x007C2F80`, `0x007C3060`, `0x007C30F0`, `0x007C3180`, `0x007C35C0`, `0x007C38D0`, `0x007C3B30`, `0x007C3B60`,
`0x007C3E10`, `0x007C3E50`, `0x007C3E90`, `0x007C3ED0`, `0x007C5000`, `0x007C50F0`, `0x007C51D0`, `0x007C5260`,
`0x007C52F0`, `0x007C5540`, `0x007C55C0`, `0x007C5AC0`, `0x007C5C80`, `0x007C5D90`, `0x007C5E20`, `0x007C5F40`,
`0x007C61A0`, `0x007C63A0`, `0x007C6410`, `0x007C6830`, `0x007C6BC0`, `0x007C6D80`, `0x007C6E30`, `0x007C7070`,
`0x007C7310`, `0x007C7370`, `0x007C7830`, `0x007C7DD0`, `0x007C8630`, `0x007C86D0`, `0x007C8F30`, `0x007C94C0`,
`0x007C9540`, `0x007C95B0`, `0x007C9600`, `0x007C9B10`, `0x007C9F90`, `0x007C9FA0`, `0x007CA9C0`, `0x007CACD0`,
`0x007CAE00`, `0x007CAFB0`, `0x007CB090`, `0x007CB0B0`, `0x007CB530`, `0x007CCF80`, `0x007D2720`.

By name in §3 (the accessor tables) (22):
`0x007C1920`, `0x007C1940`, `0x007C1960`, `0x007C1A90`, `0x007C22B0`, `0x007C22D0`, `0x007C22E0`, `0x007C22F0`,
`0x007C2300`, `0x007C2310`, `0x007C2320`, `0x007C2350`, `0x007C25E0`, `0x007C25F0`, `0x007C2750`, `0x007C2760`,
`0x007C2C00`, `0x007C2C10`, `0x007C2C30`, `0x007C2C40`, `0x007C2CD0`, `0x007C9FC0`.

By name in §5 (the entry points) (5): `0x007CA180`, `0x007CA200`, `0x007CA3D0`, `0x007C9220`, `0x007C9320`.

In §11.6 (2): `0x007C7E20`, `0x007C6EC0`.

In §13 (20): `0x007CABE0`, `0x007CA4D0`, `0x007CAA60`, `0x007CAB30`, `0x007C32A0`, `0x007C3F80`, `0x007C3350`,
`0x007C3400`, `0x007C2400`, `0x007C2610`, `0x007C26D0`, `0x007C3810`, `0x007C3BE0`, `0x007C2530`, `0x007C2570`,
`0x007C3C60`, `0x007FF0B0`, `0x007C2D50`, `0x007C2DB0`, `0x007C4910`.

In §14 (8): `0x007C1B30`, `0x007C3F10`, `0x007C3790`, `0x007C1E10`, `0x007C1EF0`, `0x007C1FD0`, `0x007C20E0`,
`0x007C5370`.

In §15 (25): `0x007D30D0`, `0x007D32D0`, `0x007C5D00`, `0x007C3090`, `0x007C2790`, `0x007CA020`, `0x007D5BB0`,
`0x007C7020`, `0x007CB070`, `0x007C9FE0`, `0x007C9FF0`, `0x007CA000`, `0x007CA010`, `0x007C56D0`, `0x007C5710`,
`0x007C3950`, `0x007C3A10`, `0x007C5750`, `0x007C57E0`, `0x007C5870`, `0x007C62B0`, `0x007C3230`, `0x007C5BE0`,
`0x007F6610`, `0x007F8290`.

Not covered: the `SC4QuadGrid` and `SC43DGridElement` internals (their own owners, §5.1), and the readers of the
visible-terrain fields `+0x3DC`…`+0x3F8` (the terrain view, terrain-view.md).
