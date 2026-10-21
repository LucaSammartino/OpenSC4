# `cSC43DRender`: owner behaviour beyond the draw pass (behaviour spec)

Scope: the exact owner `cSC43DRender` (130 owner functions, `0x007C1890`–`0x007CB530` plus folded bodies
`0x004E13E0`, `0x004E82A0`, `0x004F7BA0`, `0x007D30D0`, `0x007D32D0`, `0x007FF0B0`). Not `cSC4View3DWin`
(view3d-window.md), not the terrain view, span kernels or the DirectX 7 driver (renderer-driver.md).

This spec **extends** view3d.md and does not repeat it. Already specified there and reused by reference:
the field layout (view3d.md §3.1), the 76-slot `cISC43DRender` table (§3.2), the pass brackets (§3.3),
the sub-objects and `DoMessage` (§3.4), and `Draw` with its static/dynamic blocks (§4).

**Completeness: partial.** §1–§6 are proof-backed (addresses cited, numeric paths read from instructions).
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

The internal queue, its deadline and `ProcessDirtyRectangleQueue` are §7 remaining work.

---

## 7. Remaining owner methods (not specified here or in view3d.md)

Picking: `Pick` ×2 (`0x007C2220`, `0x007C6830`), `PickTerrain` ×2 (`0x007C35C0`, `0x007C5000`), `PickTerrainAlways` ×2
(`0x007C50F0`, `0x007C7E20`), `IsModelVisible` ×2, `FindViewObject`, `AddViewObject`, `RemoveViewObject`,
`InsertIntoViewObjectList`, `FindInViewObjectList`. Lifecycle: `Init` `0x007C6410`, `Shutdown` `0x007C7070`,
`ReadProperties` `0x007C5F40`, `SetGraphicSystem`, `SetGDriver`, `StaticSetGDriver`, `StaticShutdown` (outline:
releases the globals `0x00B4E1EC`, `0x00B4E1F0`, `0x00B4E1F4` once, destroying the backing store first),
destructor body `0x007CA020`. Shake: `StartScreenShake` ×2, `StopScreenShake`, `SetShakeOffsets` `0x007C86D0`.
Backing store: `CreateBackingStore`, `DestroyBackingStore`, `SaveBackingStore`, `RestoreBackingStore` ×2,
`ReevaluateBackingStoreObjectFlags`, `DoSingleModelStaticDrawAddition`. Dirty rects: `AddDirtyRectangleInternal`,
`ProcessDirtyRectangleQueue`, `ClearDirtyRectangleQueue`, `DoViewCleanup`. View updates: `DoTranslatedViewUpdate`,
`DoPartialViewUpdate`, `DoViewZoomChange3`, `DoViewRotationChange2`, `AttemptTranslatedViewUpdate`,
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
10. With 199 queued items the next backing-store insert forces `DoFullViewUpdate(4)`; the 201st does not.

Questions: docs/specs/questions/render3d.md.
