# The 3D view and the drawing layer (behaviour spec)

Status: M6 behaviour spec, written 2026-10-16 (re-specs21 lane). Every fact was checked on the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`); every address is a Windows address. Numeric details were
checked against the instruction bytes with `tools/x86dis.py` (Capstone linear sweep). Implementers work from this
spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags:

- *[compat]*: observable; a replacement must match it.
- *[choice]*: a FreeCity4 decision.
- *[inferred]*: consistent with all the evidence, but not proven.
- *[quirk]*: original behaviour that looks unintended but is reproduced.

Scope. The **drawing layer** of the city screen — everything between "the window is painted" and "pixels are in a
buffer":

- **`cSC4View3DWin`** — the city-screen window, and specifically its **paint path** (`GZPaint`). The window object
  itself, its 61-slot `cISC4View3DWin` table and its catalog menus are already specified in
  [ui-windows.md](ui-windows.md) §3–§4; this spec *extends* that layout with the draw-relevant fields and states the
  paint contract. It does **not** restate the slot table.
- **`cSC43DRender`** (`cISC43DRender`, vtable `0x00ABAB60`) — the 3D renderer: camera, view frustum, the static and
  dynamic view passes, the dirty-rectangle and model-change queues, the backing store, screen shake, picking.
- **`SC4DrawContext`** — the game's **3D** draw context (`SetVertexBuffer`, `DrawPrims`, `DrawPrimsIndexed`, depth
  and cull state). This is what the 3D view actually draws through.
- **`cSTETerrainView3D`** (class `0xC9B84E10`) — the terrain's 3D view: the cell-quad grid, terrain and water
  surfaces, side cliffs, cell selection and levelling. Reached through `cISTETerrain` slot 66 `GetView`.
- **`nGZDrawSoftRender::cSpanRender*`** and `cGZDrawContextRenderToBuffer` — the framework's **2D** software
  span blitter, which owns the ~203 KB of draw dispatch tables at `0x008310B0`–`0x00864B58`. §7 says exactly what
  they are and — importantly — what they are **not**.

What is not here:

- The window object model, `cIGZWin` slot semantics, dialogs and the catalog menus: [ui-windows.md](ui-windows.md).
- The visual-effect system and its draw pass: [effects.md](effects.md) §7, §11. §8 of *this* spec only says where
  that pass sits relative to the renderer.
- Terrain *simulation* (heights, brushes, water bodies, lava): [terrain.md](terrain.md). Only the terrain **view**
  is here.
- Model loading and the occupant→model pipeline (`cSC4ModelMaker`): INVENTORY-M6 §4.4; not re-specified.
- The `cSC4ViewInputControl*` tools (cursor feedback, drag rules, cost preview): INVENTORY-M6 §4.4; not re-specified.
- The MXF font format and the UI-script grammar (ui-windows.md UIW-10): still unspecified.

Related specs (cited, not repeated):

- [ui-windows.md](ui-windows.md) §3.2 (the `cSC4View3DWin` layout), §3.3 (the 61-slot table), §7 (the draw pass as
  seen from the window layer), §16 (the x87 precision trigger).
- [effects.md](effects.md) §7 (the finish test and the single-route chain), §11.1 (`cISC4VisualEffect`).
- [terrain.md](terrain.md) §15.1 (`cISTETerrain`), §13.2 (the class record's terrain-view reference).
- [save-records.md](save-records.md) §3.2: `0xC9B84E10` is the terrain's 3D view, record u16 1 + one byte.
- [graphics-formats.md](graphics-formats.md) §…: `Graphics Rules.sgr` and `cSC4View3DWin::SetRenderProperties`.
- [landfill.md](landfill.md) §3.2 and [city-situation.md](city-situation.md) §4.3: consumers of 3D-view slots through
  the global `0x00B43DC4`.
- [foundations.md](foundations.md) §18: x87 precision; §3.6 components.

---

## 0. Summary for the harness and for planning

1. **The draw pass entry point is `cSC43DRender::Draw`, `cISC43DRender` slot 21 (`+0x54`, `0x007CB530`).** It is
   reached from `cSC4View3DWin::GZPaint` (`0x007F6200`) through the renderer pointer at `view + 0xF4`. Nothing else
   in the image calls `DrawStaticView`/`DrawDynamicView`; they are reached only from inside `Draw` and from two
   partial-update paths (§4.2). **Corrects** the framing that `0x005922A0` might be a 3D-view slot — it is not
   (ui-windows.md §7 already said so; §8 here restates the correct chain).
2. **The ordering inside `Draw` is measured and is the contract:**
   `DrawPreStaticView → DrawStaticView → DrawPostStaticView` then
   `DrawPreDynamicView → DrawDynamicView → DrawPostDynamicView`, each triple gated on a different flag byte of the
   same service object (§4.3).
3. **The ~203 KB of dispatch tables at `0x008310B0`–`0x00864B58` are *not* the 3D view's draw path.** They are
   `nGZDrawSoftRender`'s five `cSpanRender*` software blitters under `cGZDrawContextRenderToBuffer`, a **2D**
   `cIGZDrawContext` implementation. The 3D view draws through `SC4DrawContext` (§7.1). The attribution in
   `tools/coverage-ranges.tsv` is **correct as it stands**; what was missing is the prose section, which §7 supplies.
4. **`cSTETerrain::GetView` (slot 66, `0x00741810`) returns `*(terrain + 0xE8) + 0x0C`** — measured, five
   instructions. `cSTETerrainView3D::Init` (`0x0075C420`) reads **two globals with no null test**:
   `0x00B43CD8` (the preferences object) and `0x00B43CF0`. §6.3 **corrects** what `0x00B43CF0` is.
5. **The x87 precision trigger is not in this executable.** It must be in `FreeCity4.dll` (§12).

---

## 1. Identity

| | `cSC4View3DWin` | `cSC43DRender` | `cSTETerrainView3D` | `SC4DrawContext` | `cGZDrawContextRenderToBuffer` |
|---|---|---|---|---|---|
| Role | city-screen window | 3D renderer | terrain's 3D view | **3D** draw context | **2D** software draw context |
| Class ID | `0x9A47B417` | — | `0xC9B84E10` | — | — |
| Primary vtable | `0x00ABCE40`, 61 slots | `0x00ABAB60`, **76 slots** (0–75) | `0x00AB4480`, 5 slots (0–4) | — | `0x00AC17D0`, 23 slots |
| Second vtable | — | `0x00ABAB38` (`cIGZSerializable`, 6 slots) | `0x00AB4410`, 22 slots (0–21) | — | — |
| Interface | `cISC4View3DWin` | `cISC43DRender` | `cISTETerrainView3D` *[inferred]* | — | `cIGZDrawContext` |
| Code region | `0x007E5630`–`0x007F64F0` | `0x007C1890`–`0x007CB0B0` | `0x00751ED0`–`0x0075C5xx` | `0x007D2750`–`0x007D5F10` | `0x0082FD60`–`0x00831000` |
| Global | `0x00B43DC4` (`SL::OnView3DCreate` `0x00602120`) | `view + 0xF4` | `terrain + 0xE8` | `renderer + 0x11C` | span renderer at `+0xB0` |
| Save record | `0xA9C8647C / 0xE9C8647E / 1` | via the View3D reference record | u16 1 + one byte (terrain.md §13.2) | — | — |

Evidence: `cSC43DRender::vftable` has 76 rows in `docs/abi/vtables.tsv` (slots 0–75, slot 75 the deleting destructor
`0x007CB090`). `cSTETerrainView3D` has 27 rows across its two vtables (`docs/abi/vtables.tsv`). `cGZDrawContextRenderToBuffer::vftable`
is 23 slots per `re/symbols/gzdraw-softrender.tsv:41`.

The **draw context split is the load-bearing fact of this spec**: the game has *two* `cIGZDrawContext`
implementations and they are not interchangeable.

| | `SC4DrawContext` | `cGZDrawContextRenderToBuffer` |
|---|---|---|
| Character | 3D, OpenGL-shaped | 2D span blits |
| Evidence | `SetVertexBuffer` `0x007D2970`, `SetIndexBuffer` `0x007D2980`, `DrawPrims` `0x007D2990`, `DrawPrimsIndexed` `0x007D29C0`/`0x007D29F0`, `EnableDepthTestFlag` `0x007D27B0`, `EnableDepthMaskFlag` `0x007D2800`, `EnableCullFaceFlag` `0x007D2850`, `SetDepthFunc` `0x007D28A0`, `SetBlendFunc` `0x007D28F0`, `SetCamera` `0x007D3D80`, `RenderModelInstance` `0x007D5710` | `SetClipRect` `0x0082FD90`, `SetBltTiling` `0x0082FE70`, `SetAlphaBlend` `0x0082FEA0`, `SetBltModulate` `0x0082FEB0`, `SetBltSource` `0x0082FFD0`, `DrawPreclippedSpanList` `0x008302E0`, `Draw` `0x00831000` |
| Who holds it | `cSC43DRender + 0x11C` | created per buffer, span renderer at `+0xB0` |
| Direct `DrawPrims` call sites | **58** | `Draw` `0x00831000` and `DrawPreclippedSpanList` `0x008302E0` have **0** direct call sites (both are vtable slots) |

`SC4DrawContext::DrawPrims` (`0x007D2990`) is called from 58 sites, every one of them an occupant or model
`Draw(SC4DrawContext*, bool)` — e.g. `0x0046E8CB`/`0x0046E8F6`/`0x0046E92E`/`0x0046E959` in
`cSC4ConstructionOccupant::Draw`, `0x005E0433` in `cSC4CellTextureModel::Draw`, `0x005CC615`–`0x005CC7C4` in the
prop-occupant draws. Measured with a full-image `E8 rel32` sweep.

---

## 2. The drawing stack

```
cGZWinGen::GZPaint (window slot 88)                     ui-windows.md §7
  └─ cSC4View3DWin::GZPaint           0x007F6200
       ├─ cSC4View3DWin::UpdateCostIndicator  0x007E8F30
       ├─ cSC4CameraControl::ScrollByViewportAngle  0x007CD330   (view+0xF0)
       ├─ view+0xE8 slot 9 → write SC4DrawContext+0x1C
       └─ view+0xF4 slot 21 = cSC43DRender::Draw   0x007CB530     <-- the draw pass
            ├─ AttemptTranslatedViewUpdate      0x007CABE0
            ├─ ProcessModelChangeQueue          0x007C9600
            ├─ SC4DrawContext::SetDefaultRenderStateUnilaterally 0x007D5230
            ├─ RestoreBackingStore              0x007C38D0   (backing-store path only)
            ├─ ClearViewport                    0x007C3F80   (slot 47, +0xBC)
            ├─ Set3DCameraView                  0x007C1A50   (slot 24, +0x60)
            ├─ DrawPreStaticView  0x007C3E90 ─┐
            ├─ DrawStaticView     0x007C7370  ├ gated on service[+0x5C]
            ├─ DrawPostStaticView 0x007C3ED0 ─┘
            ├─ DrawPreDynamicView  0x007C3E10 ─┐
            ├─ DrawDynamicView     0x007C7830  ├ gated on service[+0x3C]
            ├─ DrawPostDynamicView 0x007C3E50 ─┘
            └─ SetUICameraView     0x007C1AC0   (slot 26, +0x68)
                 └─ SC4DrawContext::DrawPrims / DrawPrimsIndexed / RenderModelInstance
                      └─ graphics-driver service (global 0x00B4E1EC)
```

The four `DrawPre*`/`DrawPost*` methods are all one or two instructions of bookkeeping and are listed in §3 for
completeness; the work is in `DrawStaticView` and `DrawDynamicView`.

**`cSC4View3DWin::GZPaint` is `cIGZWin` slot 88**, reached through the `cIGZWin` sub-object at `view + 0x0C`
(ui-windows.md §3.2). Its `this` is therefore the *sub-object*, and it recovers the game window with
`lea edi, [esi - 0xc]` at `0x007F62A1` before calling `cSC4View3DWin::UpdateCostIndicator` (`0x007E8F30`) at
`0x007F62AF`. That single instruction is a second, independent confirmation of the `+0x0C` layout.

---

## 3. `cSC43DRender`: identity, state and the full interface

### 3.1 Object layout *[compat]*

`GetDrawContext` (slot 10) is four instructions and is the anchor:

```
004e82a0  mov  eax, dword ptr [ecx + 0x11c]   ; return this + 0x11C field
004e82a6  ret
```

| Offset | What | Evidence |
|---|---|---|
| `+0x00` | `cISC43DRender` vptr → `0x00ABAB60` | `docs/abi/vtables.tsv` |
| `+0x48` | service object whose `+0x18` and `+0x0C` are read in `Draw` (`0x007CB7CB`, `0x007CB889`) | single-site |
| `+0x60` | byte: set → skip `ClearViewport` (`0x007CB676`) | `Draw` |
| `+0x61` | byte: screen-shake colour flag (`0x007CB70A`) | `Draw` |
| `+0x65` | byte: backing store already saved (`0x007CB5F0`, stored at `0x007CB799`) | `Draw` |
| `+0x68`/`+0x6C` | ints passed to `SaveBackingStore` (`0x007CB783`) | `Draw` |
| `+0x70`/`+0x78` | rects passed to `0x007F9F50` (`0x007CB60A`) | `Draw` |
| `+0x80`/`+0x84` | floats combined with the shake offsets at `+0x258`/`+0x25C` (`0x007CB582`) | `Draw` |
| `+0x8C` | object passed to `0x007CDA20` | `Draw` `0x007CB588` |
| `+0xE0`/`+0xE4` | pending translated-view update rect; both cleared after `AttemptTranslatedViewUpdate` | `0x007CB537`–`0x007CB562` |
| `+0xE8` | byte: fog flag → `TurnOnFog` | `0x007CB6D1` |
| `+0xE9` | byte: flag → `TurnOnFog`'s sibling `0x007D32D0` | `0x007CB6A7` |
| `+0xEC`, `+0x100`, `+0x104` | args to `0x007D32D0` | `0x007CB6BD` |
| `+0x108` | byte: **draw-in-progress re-entrancy guard** | `0x007CB56E`, `0x007CB591` |
| `+0x109` | byte: cleared at the start of the second half of `Draw` | `0x007CB80B` |
| `+0x114` | object passed to `0x007F9F50` and `SC4ComputeViewportZProjection` `0x007C2F80` | `0x007CB612`, `0x007C739E` |
| `+0x11C` | **the `SC4DrawContext*`** | `GetDrawContext` `0x004E82A0` |
| `+0x134`/`+0x138` | a vector begin/end pair copied on entry to each view pass | `0x007C73B5`, `0x007C7852` |
| `+0x140`/`+0x144` | second vector begin/end pair | `0x007C7880` |
| `+0x158` | **draw counter**, incremented once per `Draw` | `0x007CB5B6` |
| `+0x160`/`+0x174` | bytes cleared when the view rect is recomputed | `0x007CB61D` |
| `+0x1A0` | byte: "viewport + fog already pushed to the driver" | `0x007CB629` |
| `+0x1A8`…`+0x1B4` | four ints pushed to driver slot 12 (`+0x30`) | `0x007CB638` |
| `+0x1B8` | float pushed to driver slot 13 (`+0x34`) | `0x007CB65F` |
| `+0x1C4`/`+0x1C8` | dirty-rectangle queue begin/end | `0x007CB5BC` |
| `+0x1D4` | dirty-rectangle queue object (`cmp [eax],eax` empty test) | `0x007CB7C1` |
| `+0x1F8` | int added to a service value for the dirty-rect deadline | `0x007CB7D1` |
| `+0x1FD` | byte: enables `DoViewCleanup` | `0x007CB7A4` |
| `+0x258`/`+0x25C` | ints (shake offsets), converted to float and combined with `+0x80`/`+0x84` | `0x007CB582` |

Fields marked "single-site" are read once and are *[inferred]* in meaning only; the **offsets** are measured.

### 3.2 `cISC43DRender` slot table, vtable `0x00ABAB60` *[compat]*

Argument lists are `arg_dwords` (dwords the **callee** pops, exclusive of `this`); `ret` is the return channel:
`eax` = a value in EAX callers use, `eax-or-void` = EAX is not a usable status, `al` = a bool in AL only,
`x87` = `st(0)`, `via-call` = whatever a callee returned. `ret_confidence` is `low` where the extractor could not
tell, `high` where this spec read the body. Rows come from `docs/abi/vtables.tsv` (76 rows) and are reproduced here
because the implementer needs the drawing ones in one place.

| Slot | Offset | Target | Method | pops | args | ret |
|---|---|---|---|---|---|---|
| 0 | `+0x0` | `007c1890` | `QueryInterface(uint32, void**)` | 8 | 2 | `eax-or-void` |
| 1 | `+0x4` | `004f7ba0` | `AddRef()` | 0 | 0 | `eax-or-void` |
| 2 | `+0x8` | `007c3060` | `Release()` | 0 | 0 | `eax-or-void` |
| 3 | `+0xC` | `007c6410` | `Init()` | 0 | 0 | `eax-or-void` |
| 4 | `+0x10` | `007c7070` | `Shutdown()` | 0 | 0 | `eax-or-void` |
| 5 | `+0x14` | `007c9fa0` | `SetMapScaleFactor(...)` | 4 | 1 | `eax-or-void` |
| 6 | `+0x18` | `007c5d00` | `SetGraphicSystem(...)` | 4 | 1 | `eax-or-void` |
| 7 | `+0x1C` | `007c3be0` | `SetGDriver(...)` | 4 | 1 | `eax-or-void` |
| 8 | `+0x20` | `007c9fb0` | `GetViewUtilities()` | 0 | 0 | `eax` (borrowed, not a GZCOM object — landfill.md §4.7.5) |
| 9 | `+0x24` | `007c1910` | `GetLightingManager()` | 0 | 0 | `eax-or-void` |
| **10** | `+0x28` | `004e82a0` | **`GetDrawContext()`** | 0 | 0 | `eax` = `*(this + 0x11C)`, an `SC4DrawContext*` |
| 11 | `+0x2C` | `007c1920` | `GetViewportSize(...)` | 8 | 2 | `eax-or-void` |
| 12 | `+0x30` | `007c1940` | `SetViewportSize(...)` | 8 | 2 | `eax-or-void` |
| 13 | `+0x34` | `007c1960` | `SetSubpixelImageBias(...)` | 8 | 2 | `eax-or-void` |
| 14 | `+0x38` | `007c9fc0` | `GetCamera()` | 0 | 0 | `eax-or-void` |
| 15 | `+0x3C` | `007c1980` | `SetCamera(...)` | 4 | 1 | `eax-or-void` |
| 16 | `+0x40` | `007c9fd0` | `GetCameraControl()` | 0 | 0 | `eax-or-void` |
| 17 | `+0x44` | `007c1a00` | `SetCameraControl(...)` | 4 | 1 | `eax-or-void` |
| 18 | `+0x48` | `004e13e0` | `GetProjectionMatrixEntries(...)` | 0 | 0 | `eax-or-void` |
| 19 | `+0x4C` | `004e13e0` | `GetViewMatrixEntries(...)` (same folded body) | 0 | 0 | `eax-or-void` |
| 20 | `+0x50` | `007c7dd0` | `ForceFullRedraw()` | 0 | 0 | `edx:eax?` *[inferred]* tail to `0x007C6D80` |
| **21** | `+0x54` | `007cb530` | **`Draw()`** | 0 | 0 | `al` (see §4.1) |
| 22 | `+0x58` | `007c2570` | `TakeSnapshot(...)` | 8 | 2 | `eax-or-void` |
| 23 | `+0x5C` | `007c2530` | `TakeSnapshot(...)` (overload) | 8 | 2 | `eax-or-void` |
| 24 | `+0x60` | `007c1a50` | `Set3DCameraView()` | 0 | 0 | `eax-or-void` |
| 25 | `+0x64` | `007c1a90` | `Set2DCameraView()` | 0 | 0 | `eax-or-void` |
| 26 | `+0x68` | `007c1ac0` | `SetUICameraView()` | 0 | 0 | `unknown` |
| 27 | `+0x6C` | `007ca180` | `IsModelInserted(...)` | 4 | 1 | `eax-or-void` |
| 28 | `+0x70` | `007ca200` | `InsertModel(...)` | 4 | 1 | `eax-or-void` |
| 29 | `+0x74` | `007ca3d0` | `RemoveModel(...)` | 4 | 1 | `eax-or-void` |
| 30 | `+0x78` | `007c9220` | `MoveModel(...)` | 4 | 1 | `eax-or-void` |
| 31 | `+0x7C` | `007c9320` | `ChangeModel(...)` | 8 | 2 | `eax-or-void`, indirect jump `[ebp*4 + 0x7C94B0]` |
| 32 | `+0x80` | `007c5d90` | `AddViewObject(...)` | 12 | 3 | `eax-or-void` |
| 33 | `+0x84` | `007c5ac0` | `RemoveViewObject(...)` | 4 | 1 | `eax-or-void` |
| 34 | `+0x88` | `007c52f0` | `FindViewObject(...)` | 4 | 1 | `eax-or-void` |
| 35 | `+0x8C` | `007c22b0` | `SetZoom(int32)` | 4 | 1 | `eax-or-void` |
| 36 | `+0x90` | `007c22d0` | `ZoomIn()` | 0 | 0 | `eax-or-void` (tail `0x007CE080`) |
| 37 | `+0x94` | `007c22e0` | `ZoomOut()` | 0 | 0 | `eax-or-void` (tail `0x007CE0E0`) |
| 38 | `+0x98` | `007c22f0` | `SetRotation(int32)` | 4 | 1 | `eax-or-void` |
| 39 | `+0x9C` | `007c2300` | `RotateLeft()` | 0 | 0 | `eax-or-void` (tail `0x007CD6A0`) |
| 40 | `+0xA0` | `007c2310` | `RotateRight()` | 0 | 0 | `eax-or-void` (tail `0x007CD6C0`) |
| 41 | `+0xA4` | `007c2320` | `SetZoomAndRotation(int32, int32)` | 8 | 2 | `eax-or-void` (tail `0x007CDE60`) |
| 42 | `+0xA8` | `007cacd0` | `StartScreenShake(...)` | 8 | 2 | `eax-or-void` |
| 43 | `+0xAC` | `007c6bc0` | `StartScreenShake(...)` (overload) | 8 | 2 | `eax-or-void` |
| 44 | `+0xB0` | `007c8630` | `StopScreenShake(...)` | 4 | 1 | `eax-or-void` |
| 45 | `+0xB4` | `007c2330` | `GetShakeOffsets(...)` | 8 | 2 | `eax-or-void` |
| 46 | `+0xB8` | `007c2350` | `SetViewportClearColor(...)` | 4 | 1 | `eax-or-void` |
| **47** | `+0xBC` | `007c3f80` | **`ClearViewport(...)`** | 4 | 1 | `eax-or-void` |
| 48 | `+0xC0` | `007c3350` | `ClearViewportWithSolidColor(...)` | 12 | 3 | `via-call` |
| 49 | `+0xC4` | `007c3400` | `DrawViewportRectangle(...)` | 16 | 4 | `unknown` |
| 50 | `+0xC8` | `007c2400` | `DrawTexturedViewportRectangle(...)` | 20 | 5 | `unknown` |
| 51 | `+0xCC` | `007c25e0` | `EnableBackgroundPicture(...)` | 4 | 1 | `eax-or-void` |
| 52 | `+0xD0` | `007c25f0` | `GetBackgroundPicture(...)` | 4 | 1 | `eax-or-void` |
| 53 | `+0xD4` | `007c26d0` | `SetBackgroundPicture(...)` | 16 | 4 | `eax-or-void` |
| 54 | `+0xD8` | `007c2610` | `SetBackgroundPicture(...)` (overload) | 8 | 2 | `eax-or-void` |
| 55 | `+0xDC` | `007c2740` | `EnableTransparentBackground(...)` | 4 | 1 | `eax-or-void` |
| 56 | `+0xE0` | `007c2750` | `GetBackingStoreEnabled()` | 0 | 0 | `eax-or-void` |
| 57 | `+0xE4` | `007c2760` | `SetBackingStoreEnabled(...)` | 4 | 1 | `eax-or-void` |
| 58 | `+0xE8` | `007c2cd0` | `BorrowRenderSystem()` | 0 | 0 | `eax-or-void` |
| 59 | `+0xEC` | `007c2ce0` | `ReturnRenderSystem()` | 0 | 0 | `eax-or-void` |
| 60 | `+0xF0` | `007c5000` | `PickTerrain(...)` | 20 | 5 | `eax-or-void` |
| 61 | `+0xF4` | `007c35c0` | `PickTerrain(...)` (overload) | 20 | 5 | `eax-or-void` |
| 62 | `+0xF8` | `007c50f0` | `PickTerrainAlways(...)` | 16 | 4 | `eax-or-void` |
| 63 | `+0xFC` | `007c7e20` | `PickTerrainAlways(...)` (overload) | 16 | 4 | `eax-or-void` |
| 64 | `+0x100` | `007c6830` | `Pick(...)` | 24 | 6 | `eax-or-void` |
| 65 | `+0x104` | `007c2220` | `Pick(...)` (overload) | 16 | 4 | `eax-or-void` |
| 66 | `+0x108` | `005be2e0` | `[inferred]` `CreateMaterialVariant(...)` (folded with `cS3DMaterialManager`) | 12 | 3 | `eax-or-void` |
| 67 | `+0x10C` | `007c5260` | `IsModelVisible(...)` | 8 | 2 | `eax-or-void` |
| 68 | `+0x110` | `007c51d0` | `IsModelVisible(...)` (overload) | 8 | 2 | `eax-or-void` |
| 69 | `+0x114` | `007c2c00` | `GetViewFogEnabled()` | 0 | 0 | `eax-or-void` |
| 70 | `+0x118` | `007c2c10` | `SetViewFogEnabled(...)` | 4 | 1 | `eax-or-void` |
| 71 | `+0x11C` | `007c2c30` | `GetLightingEnabled()` | 0 | 0 | `eax-or-void` |
| 72 | `+0x120` | `007c2c40` | `SetLightingEnabled(...)` | 4 | 1 | `eax-or-void` |
| 73 | `+0x124` | `007c2d50` | `GetStatsString1(...)` | 4 | 1 | `via-call` |
| 74 | `+0x128` | `007c2db0` | `GetStatsString2(...)` | 4 | 1 | `via-call` |
| 75 | `+0x12C` | `007cb090` | `~cSC43DRender()` (deleting) | 4 | 1 | `eax-or-void` |

**Matches** `docs/abi/vtables.tsv` for all 76 rows. **Corrects:** slot 21's `ret` is recorded there as `eax-or-void`
/ `medium`; the body at `0x007CB530` writes `AL` on both exits (`xor al, al` at `0x007CB57B`, and the path through
`0x007CB568`), so the return channel is **`al`**, `ret_confidence` **high** — see §4.1.

The `cIGZSerializable` sub-object is at `+0x08`, vtable `cSC43DRender::vftable_2` (`0x00ABAB38`, **10** slots
0–9): slot 3 `Write` `0x007C30F0`, slot 4 `Read` `0x007C3180`, slot 5 `GetGZCLSID` `0x007C9F90`. Slots 0–2 are the
usual `QueryInterface`/`AddRef`/`Release` (tailing into the main ones). The renderer is therefore serialisable; it
is not clear from this spec alone which record carries it *[inferred]* that it is one of the View3D record's
references (save-records.md §2 / ui-windows.md §3.3a). See §3.4 for the third sub-object and `DoMessage`.

### 3.3 The four pass brackets

| Method | Address | Body |
|---|---|---|
| `DrawPreDynamicView` | `0x007C3E10` | `[this+0x08]` → `SC4DrawContext` slot 3 on `*(this+0x11C)` (`0x007C3E3E`) |
| `DrawPostDynamicView` | `0x007C3E50` | same shape (`0x007C3E7E`) |
| `DrawPreStaticView` | `0x007C3E90` | same shape (`0x007C3EBE`) |
| `DrawPostStaticView` | `0x007C3ED0` | same shape (`0x007C3EFE`) |

All four contain the identical 8-byte sequence
`8d 49 00 / 8b 4e 08 / 8b 87 1c 01 00 00 / 8b 11 / 8b 36 / 50 / ff 52 0c`, i.e. they pass
`(this+8, *(this+0x11C))` to `SC4DrawContext` slot 3. *[inferred]* that slot 3 is a begin/end bracket on the draw
context (it is not in the `SC4DrawContext` name table). What would settle it: `SC4DrawContext`'s vtable row for slot 3
in `docs/abi/vtables.tsv`, or the body at the target of `0x007D5F10`'s vtable.

### 3.4 Sub-objects and `DoMessage` *[compat]*

`cSC43DRender`'s constructor `0x007C9B10` installs three vtables (`0x007C9B24`–`0x007C9B31`), and the destructor
`0x007CA020` re-installs the same three (`0x007CA024`–`0x007CA031`):

| Offset | vtable | Slots | Interface |
|---|---|---|---|
| `+0x00` | `0x00ABAB60` | 0–75 | `cISC43DRender` (§3.2) |
| `+0x04` | `0x00ABAB50` | 0–3 | the **message target** — slot 3 is `DoMessage` |
| `+0x08` | `0x00ABAB38` | 0–9 | the **serialisable** sub-object |

This is the standard GZCOM shape (network-manager.md §1: message target at `+4`, serialisable at `+8`), and it is
confirmed by `DoMessage` itself, which recovers the renderer with `lea ecx, [edi - 4]` (`0x007CB226`) before
calling a renderer method.

**`cSC43DRender::DoMessage` is `0x007CB0B0`, message-target slot 3 (`+0x0C` of the `+0x04` sub-object).** It is
`__thiscall` with **one dword** (the message) and `ret 4`, and returns `al` — `1` when it handled the message, and
the unhandled path falls through to `0x007CB524`. It reads the type through `msg->slot4()` (`0x007CB0C0`,
`call [eax + 0x10]`) and binary-searches seven constants:

| Type | Name (`re/class_id_table.tsv`) | Handler | What it does |
|---|---|---|---|
| `0x4A9F2FFE` | `kSC4MessageCameraUpdated` | `0x007CB0F7` | **[gate]** only if `this + 0x198` is non-null: `cSC43DRender::UpdateVisibleTerrainMetrics` (`0x007C4910`) |
| `0x26D31EC1` | `kSC4MessagePostCityInit` | `0x007CB118` | the city-init path: city slots `+0x164`/`+0x168`, `0x004F20A0`, `+0xA0`, `+0x108`, then `UpdateVisibleTerrainMetrics` again (`0x007CB213`) |
| `0x4C8D1E8B` | *[unlisted]* | `0x007CB1AC` | walks an object's list (`test esi,esi / mov ebx,[esi] / lea ecx,[esp+0x2c]`); **[inferred]** a model/view-object notification. Not in `re/class_id_table.tsv` |
| `0x6ACB92A9` | *[unlisted]* | `0x007CB223` | `mov edx,[edi-4]; lea ecx,[edi-4]; call [edx+0x50]` = **renderer slot 20 `ForceFullRedraw`** (`+0x50`), then `al = 1`. Three instructions |
| `0x870BC918` | `kSC4MessageRedisplayTerrain` | `0x007CB237` | the terrain redisplay — terrain.md §8.1 says "only the renderer reads it" |
| `0x8ACAA95F` | `kSGLMessage_RestoreTextures` | `0x007CB23E` | texture restore |
| `0xA6B79601` | `kSC4MessageViewZRChange2` | `0x007CB3E3` | view Z/R change; INVENTORY-M6 §3.3 lists the family `0xA6B79600`–`0xA6B79622` |

Any other type falls to `0x007CB524` and is **not** handled. A replacement must return `al = 1` only for the seven
above and must not claim to handle the rest of the `0xA6B79600` family that `DoMessage` does not compare — those
go to other subscribers (the model maker, the terrain view, the lighting manager: INVENTORY-M6 §3.3).

`cSC4View3DWin::DoMessage` has two bodies in the name table (`0x007F3E50`, `0x007F53C0`) and is specified only as
far as other specs need it (modal-dialogs.md `0xAA5CF1E3`, automata.md `0x06CDB65A` `kSC4MessagePreViewFilterChange`
via `SetCurrentViewFilterInfo` `0x007E6435`). The drawing layer is not on that path.

The owner methods outside the draw pass (QI/refs, construction defaults, the `Write`/`Read` record, accessors,
camera mode, model change queue, dirty rectangles, shake stop) are specified in **render3d.md**.

**Save.** The renderer's `cIGZSerializable` sub-object at `+0x08` exposes `Write` `0x007C30F0` (slot 3),
`Read` `0x007C3180` (slot 4) and `GetGZCLSID` `0x007C9F90` (slot 5). Which save record carries them is V3D-7.


---

## 4. The draw pass

### 4.1 `cSC43DRender::Draw` (`0x007CB530`, slot 21) *[compat]*

The complete body, in order. Steps marked **[gate]** are conditional.

1. `0x007CB537`–`0x007CB568`: if `+0xE0` or `+0xE4` is non-zero, call `AttemptTranslatedViewUpdate` (`0x007CABE0`)
   with those two as arguments, then **clear both**. If it returns true in `AL`, jump to the exit at `0x007CB913`.
2. `0x007CB56E`: **[gate]** if `+0x108` (draw-in-progress) is set → `xor al, al`, return. **Re-entrancy guard.**
3. `0x007CB591`: set `+0x108 = 1`.
4. `0x007CB582`–`0x007CB5B1`: compute two floats from `+0x25C`/`+0x84` and `+0x258`/`+0x80` and call `0x007CDA20`
   on the object at `+0x8C` — the screen-shake offset application.
5. `0x007CB5B6`: `inc dword [+0x158]` — the **draw counter**. This is the value the dirty-rect deadline at
   `0x007CB7DD` compares against.
6. `0x007CB5BC`: **[gate]** if the model-change queue is non-empty (`+0x1C4 != +0x1C8`) → `ProcessModelChangeQueue`
   (`0x007C9600`).
7. `0x007CB5D1`: `SC4DrawContext::SetDefaultRenderStateUnilaterally` (`0x007D5230`) on `+0x11C`.
8. `0x007CB5E2`: `cSC43DRender::TurnOnLighting` (`0x007D30D0`) on `+0x11C` with argument 0.
9. `0x007CB5E8`–`0x007CB605`: **[gate]** if the global byte `0x00B0DC19` is set **and** `+0x65` is set →
   `RestoreBackingStore` (`0x007C38D0`). If it returns true, set a local flag and skip to step 14.
10. `0x007CB60A`–`0x007CB623`: otherwise compute rects (`+0x70`, `+0x78`) and call `0x007F9F50` on `+0x114`; then
    clear `+0x160` and `+0x174`.
11. `0x007CB629`–`0x007CB673`: **[gate]** if `+0x1A0` is clear → set it and push `+0x1A8`,`+0x1AC`,`+0x1B0`,`+0x1B4`
    to the graphics-driver service (`0x00B4E1EC`) slot 12 (`+0x30`), and `+0x1B8` to its slot 13 (`+0x34`).
12. `0x007CB676`–`0x007CB69A`: **[gate]** if `+0x60` is clear → call own **slot 47 `ClearViewport`** (`+0xBC`).
13. `0x007CB6A0`: call own **slot 24 `Set3DCameraView`** (`+0x60`).
14. `0x007CB6A7`–`0x007CB6E4`: **[gate]** if `+0xE9` → `cSC43DRender::TurnOnFog` sibling `0x007D32D0` on `+0x11C`
    with `1, +0xEC, +0x100, +0x104`; then `TurnOnLighting` (`0x007D30D0`) with `+0xE8` as the flag.
15. **Static pass** — see §4.2.
16. `0x007CB7C1`–`0x007CB7F4`: **[gate]** dirty-rect deadline: if the queue object at `+0x1D4` is non-empty and the
    draw counter `+0x158` is outside `[+0x1F8, +0x1F8 + service(+0x18)->+0x2EC]`, call
    `ProcessDirtyRectangleQueue` (`0x007CAE00`).
17. `0x007CB807`–`0x007CB811`: clear `+0x109`, call own **slot 24 `Set3DCameraView`** again.
18. **Static pass (second chance)** and **dynamic pass** — see §4.2.
19. `0x007CB85F`–`0x007CB87D`: `TurnOnLighting(0)` then `0x007D32D0` with `(0, 0, 0x7F7FFFFF, 0x7F7FFFFF)` on
    `+0x11C`.
20. `0x007CB882`: call own **slot 26 `SetUICameraView`** (`+0x68`).
21. `0x007CB889`–`0x007CB8A5`: **[gate]** if `service(+0x48)->+0x0C` byte at `+0x2DC` is clear → zero the shake
    offsets `+0x258` and `+0x25C`.
22. `0x007CB8F0`–`0x007CB91A`: re-apply the shake offsets through `0x007CDA20`, write `ebx` to
    `*(this + 0x11C) + 0x20`, **clear the re-entrancy guard** `+0x108`, then `pop esi; mov al, 1; pop ebx;
    add esp, 8; ret`.

Return channel: **`al`**. The early-out at `0x007CB57A` is `xor al, al; pop ebx; add esp, 8; ret`; the main exit at
`0x007CB913` is `mov al, 1; …; ret`. Both are plain `ret` (callee pops 0). A caller that treats the return as `eax`
reads garbage in the upper bytes; a declaration as `void` discards a real status. *[compat]*

### 4.2 The two static/dynamic blocks *[compat]*

`Draw` contains the static pass **twice**, at different points, and the dynamic pass once. Both static sites and the
dynamic site are identical five-instruction blocks:

```
007cb76e  mov  ecx, esi
007cb770  call DrawPreStaticView   0x007C3E90
007cb775  mov  ecx, esi
007cb777  call DrawStaticView      0x007C7370
007cb77c  mov  ecx, esi
007cb77e  call DrawPostStaticView  0x007C3ED0
```

| Site | Address | When |
|---|---|---|
| static #1 | `0x007CB76E` | global byte `0x00B0DC19` set, `+0x65` clear, and `*(byte*)(*(0x00B43CC4) + 0x0C) + 0x5C` non-zero. Followed immediately by `SaveBackingStore` (`0x007C2960`) with `0, 0, +0x68, +0x6C`, whose `AL` is stored into `+0x65` |
| static #2 | `0x007CB828` | the local "restored" flag is clear **and** the same `+0x5C` byte is non-zero |
| dynamic | `0x007CB84A` | `*(byte*)(*(0x00B43CC4) + 0x0C) + 0x3C` non-zero |

**The static gate and the dynamic gate are different bytes of the same object** — `+0x5C` for static, `+0x3C` for
dynamic, both reached as `*(byte*)(*(0x00B43CC4) + 0x0C) + N`. Measured at `0x007CB766`/`0x007CB769`,
`0x007CB820`/`0x007CB823` and `0x007CB842`/`0x007CB845`. *[inferred]* that `0x00B43CC4` is a service-manager pointer
and its `+0x0C` a render-properties object; what would settle it is the writer of `0x00B43CC4`.

A replacement that draws static and dynamic from one gate will differ: with `+0x5C` clear and `+0x3C` set the
original draws **only the dynamic pass**, and vice versa.

`DrawStaticView` (`0x007C7370`) and `DrawDynamicView` (`0x007C7830`) both begin by setting a mode word on the draw
context:

| Call | `*(this + 0x11C) + 0x50` |
|---|---|
| `DrawStaticView` `0x007C7392` | **1** |
| `DrawDynamicView` `0x007C7845` | **4** |

Both then copy the vector pair at `+0x134`/`+0x138` down onto itself (collapsing it) and use
`GetVisibleContainerRect` (`0x007C1FD0`) and `SC4ComputeViewportZProjection` (`0x007C2F80`). `DrawDynamicView` also
walks the view-object list through `0x007C61A0` and `SC4ViewUtilities::IsBoundingBoxInViewFrustum` (`0x007F6770`),
which is the same helper `cSC4EffectsManager::Update` uses (§8) — the two passes walk the same object list.

Two further call sites invoke the *same* five-instruction triple outside `Draw`:

| Site | Inside | Static / dynamic |
|---|---|---|
| `0x007C9522`/`0x007C9529`/`0x007C9530` | `cSC43DRender::UpdateViewportRect` (`0x007C94C0`) | static |
| `0x007CA964`/`0x007CA96B`/`0x007CA972` | `cSC43DRender::DoTranslatedViewUpdate` (`0x007CA4D0`) | dynamic |

So a **partial** view update (a scroll, a viewport-rect change) re-runs only the half of the pass it affects. That is
the mechanism behind the dirty-rectangle queue.

---

## 5. `cSC4View3DWin`: the paint path and the draw fields

The window's identity, its 61-slot `cISC4View3DWin` table and its catalog menus are [ui-windows.md](ui-windows.md)
§3–§4. What follows is the **draw** side.

### 5.1 The paint entry *[compat]*

`cSC4View3DWin::GZPaint` is `0x007F6200`, `cIGZWin` slot 88 (`+0x158` in the `cIGZWin` vtable). Its `this` is the
`cIGZWin` sub-object at `view + 0x0C`. It has **no direct callers** — it is dispatched by the window manager through
the `cIGZWin` vtable, exactly as ui-windows.md §7 says.

Measured sequence (all offsets below are **`cIGZWin`-relative**; add `0x0C` for the `cSC4View3DWin` offset):

| Step | Site | What |
|---|---|---|
| 1 | `0x007F6206` | **[gate]** `*(byte*)(this + 0x150)` must be set, else jump to `0x007F638C` (the post-draw tail) |
| 2 | `0x007F621E` | `0x00890198` on `this + 0xF4` — a tick counter of a timer object |
| 3 | `0x007F6223`–`0x007F625D` | compare `*(float*)(this + 0xF0)` against the tick; early-out to `0x007F638B` if not due |
| 4 | `0x007F625F` | `0x0089058F` on the same timer object |
| 5 | `0x007F6264`–`0x007F62AF` | convert the tick to a float, scale by `0x00A9F190`, clamp to `0x00A92D28`, then `lea edi, [this - 0xc]` and call **`cSC4View3DWin::UpdateCostIndicator`** (`0x007E8F30`) |
| 6 | `0x007F62B4` | **[gate]** `*(byte*)(this + 0x324)` — a "fade in progress" flag |
| 7 | `0x007F62BE`–`0x007F62DF` | `cSC4CameraControl::ScrollByViewportAngle` (`0x007CD330`) on `*(this + 0xE4)` with two floats derived from `*(this + 0x320)`, `*(this + 0x31C)` and `0x00ABCFB0`; `AL` is kept |
| 8 | `0x007F62E6` | **[gate]** `*(byte*)(this + 0x32D) == 1` → decrement `*(float*)(this + 0x320)` by `*(float*)(this + 0x330)`; when it goes below `0x00A81054` call `0x007F6160` and clear `+0x324` |
| 9 | `0x007F6326` | if step 7 returned true → `cSC4View3DWin::DoTemporarySimHiddenPause` (`0x007E5B40`) |
| 10 | `0x007F6331` | **[gate]** `*(byte*)(this + 0x16C)` and the timer's tick > `*(this + 0x170)` → `DoTemporarySimHiddenResume` (`0x007E5B70`) |
| 11 | `0x007F6355` | if `*(byte*)(this + 0xD9)` is clear, take `obj(this + 0xDC).slot9()` (`+0x24`); else use the float from step 5 |
| 12 | `0x007F6372` | `obj(this + 0xE8).slot10()` = **`cSC43DRender::GetDrawContext`** → write the step-11 value to **`SC4DrawContext + 0x1C`** |
| 13 | `0x007F6380` | `obj(this + 0xE8).slot21()` = **`cSC43DRender::Draw`** — **the draw pass** |
| 14 | `0x007F638C`–`0x007F64BB` | the tail: `SC4GetPopupModalDialogsSafe` (`0x005F9000`), a child walk with `AddRef`/`Release`, `0x005F98C0`, `0x007F5F00`, `0x007F6010` × 2, and four `+0x24` slot calls |

Step 13 is the **only** route from the window to the 3D draw pass.

### 5.2 Draw-relevant `cSC4View3DWin` fields *[compat]*

These extend ui-windows.md §3.2. `GZPaint`'s `this` is `view + 0x0C`, so `cIGZWin` offset `X` is `cSC4View3DWin`
offset `X + 0x0C`.

| `cSC4View3DWin` offset | What | Evidence |
|---|---|---|
| `+0xE5` | byte: select between the lighting-manager value and the computed float | `0x007F6355` |
| `+0xE8` | object whose slot 9 is read per paint | `0x007F635F` |
| **`+0xF0`** | **`cSC4CameraControl*`** — the save record's first reference | `0x007F62D9` calls `cSC4CameraControl::ScrollByViewportAngle` `0x007CD330` on it |
| **`+0xF4`** | **`cSC43DRender*`** — the save record's second reference | slots 10 and 21 match `cSC43DRender::vftable` exactly (`GetDrawContext`, `Draw`) |
| `+0xFC` | float: the paint period | `0x007F6229` |
| `+0x100` | timer object (`0x00890198` tick, `0x0089058F`) | `0x007F621E`, `0x007F625F` |
| `+0x14C` | object walked in the paint tail | `0x007F638C` |
| `+0x15C` | byte: **paint enabled** — the whole of step 2–13 is skipped when clear | `0x007F6206` |
| `+0x160` | timer object | `0x007F633B` |
| `+0x178` | byte: enables the auto-resume test | `0x007F6331` |
| `+0x17C` | int: the auto-resume threshold | `0x007F6346` |
| `+0x328` | int: camera-scroll argument | `0x007F62C9` |
| `+0x32C` | float: fade position | `0x007F62C3`, `0x007F62FB` |
| `+0x330` | byte: fade-in-progress flag | `0x007F62B4` |
| `+0x339` | byte: fade mode (1 = linear countdown) | `0x007F62E6` |
| `+0x33C` | float: fade step per paint | `0x007F62F5` |

**Corrects / extends** ui-windows.md §3.2: that table lists `+0xF0`/`+0xF4` only as "the camera control / renderer /
model maker / simulator references" via §3.3a's save discussion. They are the **camera control** and the **3D
renderer**, and they are *read every paint*. The view manager at `+0x300` is untouched by the paint path.

### 5.3 Save behaviour for the draw layer

None beyond ui-windows.md §3.3a. The View3D record's first two written values are `this + 0xF0` and `this + 0xF4`
written as references (ui-windows.md §3.3a), which this spec now identifies as the **camera control** and the
**3D renderer**. Nothing about the renderer's *state* (zoom, rotation, dirty rectangles, backing store, draw
counter) is saved by the window layer; `cSC43DRender::Write` (`0x007C30F0`) / `Read` (`0x007C3180`) exist on the
`cIGZSerializable` sub-object and are *[inferred]* to carry it.

---

## 6. `cSTETerrainView3D`: the terrain's 3D view

### 6.1 Identity

| Fact | Value | Evidence |
|---|---|---|
| Class name | `cSTETerrainView3D` | `re/names_windows.tsv` |
| Class ID | `0xC9B84E10` | save-records.md §3.2 |
| Primary vtable | `0x00AB4480`, 5 slots (0–4) | `docs/abi/vtables.tsv` |
| Second vtable | `0x00AB4410`, 22 slots (0–21) | `docs/abi/vtables.tsv` |
| Owner | `cSTETerrain`, field `+0xE8` | `cSTETerrain::GetView` `0x00741810` |
| Reached by | `cISTETerrain` **slot 66** (`+0x108`) | `docs/abi/interfaces.tsv` |
| Constructor | `0x007567C0` | `re/names_windows.tsv` |
| Init | `0x0075C420`, `ret 4` (1 dword) | body read |
| Save record | u16 `1` + one byte (0 or 1 in the corpus) | terrain.md §13.2, save-records.md §3.2 |

### 6.2 `GetView` (terrain slot 66) *[compat]*

```
00741810  mov  eax, dword ptr [ecx + 0xe8]
00741816  test eax, eax
00741818  je   0x74181e
0074181a  add  eax, 0xc
00741817  ret
0074181e  xor  eax, eax
00741820  ret
```

**`GetView()` returns `*(terrain + 0xE8) + 0x0C`, or null if the pointer is null.** Five instructions, read on
Windows. This is exactly the `AsIGZWin` pattern of `cSC4View3DWin` (ui-windows.md §3.2): the *object* has its primary
vptr at `+0` and the interface the caller wants at `+0x0C`. So:

- `terrain + 0xE8` holds the `cSTETerrainView3D*` (its `+0x00` vptr is `0x00AB4480`);
- the pointer handed to callers is the **sub-object at `+0x0C`**, whose vtable is `0x00AB4410` *[inferred]* — that
  is the 22-slot table whose slots 14/15 are `DrawTerrainQuad`/`DrawTexturedTerrainQuad`, i.e. the drawing
  interface. **Matches** the task's prior that `GetView` returns `view + 0x0C`.

The other sub-object is therefore at `+0x00`, and `0x00AB4480`'s slots are `QueryInterface` `0x00752730`,
`Update` `0x0075BFD0` (slot 3, pop 4) and `Pick` `0x00752700` (slot 4).

### 6.3 `cSTETerrainView3D::Init` (`0x0075C420`) and its unchecked globals *[compat]*

`Init` is `__thiscall` with **one dword argument** and `ret 4` (the terrain pointer — see step 4). It returns a bool
in `AL`: `0` if already initialised (`0x0075C43F` `xor al, al`).

| Step | Site | What |
|---|---|---|
| 1 | `0x0075C420` | if the global byte `0x00B4C740` is clear, call `0x0075C0C0` (a one-time static init) |
| 2 | `0x0075C437` | **[gate]** if `this + 0x18` is set (already initialised) → return `AL = 0` |
| 3 | `0x0075C448` | set `this + 0x18 = 1`. **[gate]** if `this + 0x19` is clear → read `*(byte*)(*(0x00B43CD8) + 1)` and store the 0/1 result in the global byte `0x00B4C741` |
| 4 | `0x0075C466` | if `this + 0x1C` is null, take the global `0x00B43CEC` (the **city**, `cISC4City`) and `AddRef` it — **this one is null-checked** |
| 5 | `0x0075C488` | if `this + 0x20` is null, store the **argument** (the terrain) and `AddRef` it |
| 6 | `0x0075C494` | if `this + 0x24` is null, take the global **`0x00B43CF0`** and `AddRef` it — **no null check** |
| 7 | `0x0075C4A8` | `obj(this+0x20).slot8()` and `obj(this+0x20).slot7()` → `0x007595A0` (allocates the cell-quad grid from the terrain's cell counts) |
| 8 | `0x0075C4C6` | `obj(this+0x20).slot42()` (`+0xA8`) → compare with the global `0x00B0D6FC`; on change call `0x00756CA0` then `0x007571D0` |
| 9 | `0x0075C4F7` | set globals `0x00B4C708 = 4`, `0x00B4C70C = 5`; if `this + 0x28` is null take `city.slot37()` (`+0x90`) and `AddRef` |

**The two unchecked globals.** Both are read and immediately dereferenced:

| Global | Site | Read | Null test? | Identity |
|---|---|---|---|---|
| `0x00B43CD8` | `0x0075C453`–`0x0075C458` | `*(byte*)(obj + 1)` | **none** | the **preferences** object (city-detail.md §2: detail byte at `+0xF16`) |
| `0x00B43CF0` | `0x0075C499`–`0x0075C4A5` | `obj->slot1()` (`AddRef`) | **none** | see the correction below |

By contrast `0x00B43CEC` **is** null-tested at `0x0075C473`, and so is the `this + 0x28` fetch.

**Corrects:** the brief describes `0x00B43CF0` as one of "its unchecked globals" alongside `0x00B43CD8`, which is
right about the *behaviour* but leaves the identity open. **`0x00B43CF0` is `cISTESurfaceWater`, not the terrain.**
The `SL::` global-cache setter at `0x00601C20` fills a run of globals from `cISC4City` slots:

| City slot | Offset | Method | Global |
|---|---|---|---|
| 31 | `+0x7C` | `GetDemolitionUtility` | `0x00B43D00` |
| 37 | `+0x94` | `GetHitTestResultData` (folded name) | `0x00B43D18` |
| 39 | `+0x9C` | `GetDispatchManager` | `0x00B43D04` |
| 83 | `+0x14C` | **`GetSurfaceWater`** | **`0x00B43CF0`** |
| 84 | `+0x150` | **`GetTerrain`** | **`0x00B43CF4` and `0x00B43CF8`** (both) |
| 85 | `+0x154` | `GetEffectsManager` | `0x00B43D1C` |

Measured at `0x00601C2E`–`0x00601C4C` (the terrain and surface-water stores) and `0x00601C53`–`0x00601CE1`; the
matching teardown at `0x00601FF0` clears `0x00B43CF0`, `0x00B43CF4` and `0x00B43CF8` together. So the terrain 3D
view holds **city, terrain and surface water** — which is what a view that draws `DrawWaterSurface` (`0x00752240`)
and `DrawWaterBox` (`0x007551C0`) needs. `0x00B43CF4` is `cISTETerrain`, as every other spec already says.

**Also corrects** any reading of "the terrain view" as reachable only through the terrain: it is reachable three
ways — `cISTETerrain` slot 66, the `terrain + 0xE8` field, and (for the city screen) through
`cISC4View3DWin`'s renderer.

### 6.4 `cSTETerrainView3D` interface tables

`0x00AB4410` (22 slots) — the drawing/selection interface returned by `GetView` *[inferred]*:

| Slot | Offset | Target | Method | pops | args | ret |
|---|---|---|---|---|---|---|
| 0 | `+0x0` | `00756910` | `QueryInterface(uint32, void**)` | 8 | 2 | `eax-or-void` |
| 1 | `+0x4` | `00761850` | `AddRef()` | 0 | 0 | `eax-or-void` |
| 2 | `+0x8` | `00761860` | `Release()` | 0 | 0 | `eax-or-void` |
| 3 | `+0xC` | `0075a230` | `Pick(...)` | 20 | 5 | `eax-or-void` |
| 4 | `+0x10` | `007526e0` | `MarkSelected(...)` | 28 | 7 | `eax-or-void` |
| 5 | `+0x14` | `007526c0` | `MarkSelected(...)` | 28 | 7 | `eax-or-void` |
| 6 | `+0x18` | `007526a0` | `MarkSelected(...)` | 12 | 3 | `eax-or-void` |
| 7 | `+0x1C` | `00752680` | `MarkSelected(...)` | 12 | 3 | `eax-or-void` |
| 8 | `+0x20` | `00752660` | `MarkSelected(...)` | 12 | 3 | `eax-or-void` |
| 9 | `+0x24` | `00751ed0` | `ClearCurrentSelections()` | 0 | 0 | `eax-or-void` |
| 10 | `+0x28` | `00752860` | `SetAutomaticCellCursorUpdateStatus(...)` | 4 | 1 | `eax-or-void` |
| 11 | `+0x2C` | `007568e0` | `IsWaterGlareEnabled()` | 0 | 0 | `eax-or-void` |
| 12 | `+0x30` | `00756080` | `SetWaterGlareStatus(...)` | 4 | 1 | `eax-or-void` |
| 13 | `+0x34` | `00755f30` | `SetLightingUpdateRect(...)` | 4 | 1 | `eax-or-void` |
| **14** | `+0x38` | `00752870` | **`DrawTerrainQuad(...)`** | 12 | 3 | `eax-or-void` |
| **15** | `+0x3C` | `00752940` | **`DrawTexturedTerrainQuad(...)`** | 24 | 6 | `eax-or-void` |
| 16 | `+0x40` | `00759750` | `LevelCell(...)` | 12 | 3 | `eax-or-void` |
| 17 | `+0x44` | `00758620` | `RemoveCellLevelling(...)` | 8 | 2 | `eax-or-void` |
| 18 | `+0x48` | `007558a0` | `GetOverlayManager(...)` | 4 | 1 | `eax-or-void` |
| 19 | `+0x4C` | `0075a770` | `HandleCheatCommand(...)` | 4 | 1 | `eax-or-void` |
| 20 | `+0x50` | `00752ae0` | `GetDisplayGridFlag()` | 0 | 0 | `eax-or-void` |
| 21 | `+0x54` | `00755e80` | `SetDisplayGridFlag(...)` | 4 | 1 | `eax-or-void` |

`0x00AB4480` (5 slots) — the object's own sub-object at `+0x00`:

| Slot | Offset | Target | Method | pops | args | ret |
|---|---|---|---|---|---|---|
| 0 | `+0x0` | `00752730` | `QueryInterface(uint32, void**)` | 8 | 2 | `eax-or-void` |
| 1 | `+0x4` | `005be3e0` | `AddRef()` (folded) | 0 | 0 | `eax-or-void` |
| 2 | `+0x8` | `005bcb30` | `Release()` (folded) | 0 | 0 | `eax-or-void` |
| 3 | `+0xC` | `0075bfd0` | `Update(...)` | 4 | 1 | `eax-or-void` |
| 4 | `+0x10` | `00752700` | `Pick(...)` | 16 | 4 | `via-call` |

The view's draw work is not in these slots: `DrawTextureLessTerrainSurface` (`0x00752100`),
`DrawTextureLessTerrainSurfaceWithVertBufExtension` (`0x007521C0`), `DrawWaterSurface` (`0x00752240`),
`DrawSideCliffs` (`0x00753E00`), `DrawWaterBox` (`0x007551C0`), `ComputeDrawRectsForDrawFrustum` (`0x00752E80`) and
`ComputeHeightBasedCullingInfoForDrawFrustum` (`0x007535A0`) are ordinary members, reached from inside
`cSC43DRender::DrawStaticView`/`DrawDynamicView`'s object walk *[inferred]*. Settle by a caller list of
`DrawWaterSurface`.

---

## 7. The software span renderers: what `0x008310B0`–`0x00864B58` actually is

The brief asks whether "the span/draw dispatch tables at `0x008310B0`–`0x00864B58` (~203 KB), the biggest single
unattributed block" are the 3D view layer's real draw path.

### 7.1 Verdict

**DIFFERS from that framing. They are not the 3D view's draw path.** They are the **2D software span blitter** of
`cGZDrawContextRenderToBuffer`, a separate `cIGZDrawContext` implementation from the framework. The 3D view's
geometry goes through `SC4DrawContext` (§1). What is true is that they are a real, correctly-attributed owner and
they sit on the *presentation* path, so they deserve a section — this one.

Evidence, all measured:

1. **Ownership.** `cGZDrawContextRenderToBuffer::Init` (`0x00830710`, `0x00AC17D0` slot 3) queries the buffer for
   IID `0x20732180` and, by its **colour type**, news one of five span-renderer classes into `+0xB0`:
   `2 → 0x00AC188C`, `4/6 → 0x00AC182C`, `5 → 0x00AC1844`, `7 → 0x00AC185C`, `8/9 → 0x00AC1874`
   (`re/symbols/gzdraw-softrender.tsv:18`).
2. **Each class is 6 slots** (a deleting destructor at slot 0 = `0x0082FF00`, then `RenderSolid`, `RenderGradient`,
   `RenderAlpha`, `RenderAlphaGradient`, `RenderBlt`). `RenderBlt` indexes a **14 × 12 dispatch table** of template
   blit / texel-conversion instantiations — 168 slots per class, 840 in all, at
   `0x00AC18A4`–`0x00AC1B40`, `0x00AC1B88`–`0x00AC1E24`, `0x00AC1E60`–`0x00AC20FC`, `0x00AC2138`–`0x00AC23D4`,
   `0x00AC2410`–`0x00AC26AC` (`tools/coverage-ranges.tsv:13-15`).
3. **The five link-order anchors** are `RenderSolid` of each class: `0x008310B0` (1555), `0x0083DE50` (565),
   `0x00847CE0` (4444), `0x00854E10` (8888), `0x0085E640` (8). Between them lie 793 unnamed functions — 768 of
   which take the range rule, the 25 vtable methods already carrying names (`tools/coverage-ranges.tsv:16-17`).
4. **No code outside the range calls into it.** `cGZDrawContextRenderToBuffer::Draw` (`0x00831000`, slot 21) and
   `DrawPreclippedSpanList` (`0x008302E0`, slot 22) have **zero direct call sites** in the whole image — they are
   dispatched as `cIGZDrawContext` vtable slots. Measured with a full-image `E8` sweep.
5. **The 3D path does not use it.** `SC4DrawContext::DrawPrims` (`0x007D2990`) has 58 call sites, all occupant/model
   `Draw` methods; `cSC43DRender` holds an `SC4DrawContext` at `+0x11C`, not a `cGZDrawContextRenderToBuffer`.
6. **The class's constructor is `0x00830690`** (`mov eax,ecx; xor ecx,ecx; mov [eax], 0x00AC17D0; …; mov [eax+0xB0], ecx; ret`),
   zeroing the refcount (`+0x04`, `+0x08`), the object at `+0x24`, the four flag bytes at `+0x41`, `+0x43`, `+0x74`,
   `+0x75`, and the **span-renderer pointer at `+0xB0`**. `Release` (`0x008306C0`) decrements `+0x4`, and on zero
   deletes the span renderer at `+0xB0` and releases `+0x24` and `+0x08`.

The Mac build carries 937 such `nGZDrawSoftRender::` template functions (`converttexel<RGBFormat…>`, 250 KB);
the Windows block is the same family, one instantiation set per colour format
(`tools/coverage-ranges.tsv:17-18`).

### 7.2 The five ranges, restated with their owners

`tools/coverage-ranges.tsv` already records these and is **correct**; reproduced here so the bytes have a prose
home and an implementer can see what they are replacing.

| Range | Owner | Colour types | Functions | Bytes |
|---|---|---|---|---|
| `0x008310B0`–`0x0083DE50` | `nGZDrawSoftRender::cSpanRender1555` | 4 `R5G5B5`, 6 `A1R5G5B5` | 172 | 50 817 |
| `0x0083DE50`–`0x00847CE0` | `nGZDrawSoftRender::cSpanRender565` | 5 `R5G6B5` | 145 | 39 051 |
| `0x00847CE0`–`0x00854E10` | `nGZDrawSoftRender::cSpanRender4444` | 7 `A4R4G4B4` | 172 | 51 809 |
| `0x00854E10`–`0x0085E640` | `nGZDrawSoftRender::cSpanRender8888` | 8 `R8G8B8`, 9 `A8R8G8B8` | 174 | 37 218 |
| `0x0085E640`–`0x00864B60` | `nGZDrawSoftRender::cSpanRender8` | 2 `RGB8` | 130 | 24 545 |

The last range ends where the EA sound library starts (`tools/coverage-libs.tsv`, `easnd` `0x00864B60`). Note the
brief's `0x00864B58` is two bytes short of that boundary: the *code* ends at `0x00864B5F` and the library marker is
`0x00864B60`. **PARTLY** — the range is right in substance, the end address in the brief is `0x00864B58` while
`tools/coverage-ranges.tsv` uses `0x00864B60`; use the latter.

### 7.3 What a replacement must provide

A replacement of the *drawing layer* has to supply **both** contexts:

- an `SC4DrawContext`-shaped 3D context with `SetVertexBuffer`, `SetIndexBuffer`, `DrawPrims`,
  `DrawPrimsIndexed`, the depth/cull/blend/alpha flags, `SetCamera`, `RenderModelInstance`, `DrawBoundingBox`,
  `SetDefaultRenderState(Unilaterally)`, `SetRenderState`, `SetRenderStateHighlight`, `SetShade`, `SetSelfLitShade`,
  `SetModelShade`, `ResetShade`, `SetTexColor`, `SetTexCombiner`, `SetTransparency`/`ResetTransparency`, and
  `EnableColorMaskFlag`/`EnableDepthTestFlag`/`EnableDepthMaskFlag`/`EnableCullFaceFlag`/`EnableAlphaTestFlag`/
  `EnableTextureStateFlag`/`SetModelViewTransformChanged` (all at `0x007D2750`–`0x007D5F10`);
- a 2D `cIGZDrawContext` that can blit spans into a locked buffer of any of the five colour types. FreeCity4 can
  implement this with any blitter it likes; only the **observable** result (the same pixels for the same span list)
  is *[compat]*. The 14 × 12 dispatch table is an implementation detail of the original's template instantiation
  and is **not** *[compat]*.

---

## 8. The effects draw pass and where it sits (answers ui-windows.md UIW-6)

effects.md §7 establishes that the finish test is reachable **only** through
`cSC4EffectsManager::Update(SC4DrawContext*)` (`0x00593120`) and that `Update` has no direct callers — it is
dispatched as a vtable slot. This spec measures the slot and the object shape:

| Fact | Value | Evidence |
|---|---|---|
| Draw interface vtable | `cSC4EffectsManager::vftable_1` `0x00A9F250`, 5 slots | `re/labels_windows.tsv:268`; slots read from the bytes: `0x58FE20, 0x58FD60, 0x58FD70, 0x593120, 0x735290` |
| Draw-interface sub-object offset | **`manager + 0x04`** | destructor `0x0058FA60`: `mov [esi], 0xA9F264` / `mov [esi+4], 0xA9F250` / `mov [esi+8], 0xA9F240` / `mov [ebx], 0xA9F210` with `ebx = esi+0x0C` |
| Slot 3 = `Update` | `0x00A9F25C` → `0x00593120` | bytes at `0x00A9F25C` |
| `Update`'s `this` | `manager + 4` | `0x00593958`/`0x0059397A`/`0x00593982` all do `lea ecx, [esi - 4]` before calling `DrawScreenFlashes` `0x0058E870`, `0x0058EC40` and `DeactivateUnusedVisualEffects` `0x005922A0` |
| `Update`'s argument | one dword: an `SC4DrawContext*` | prologue `0x00593124 mov ebx, [esp+0x5C]` = arg1 after `sub esp,0x54; push ebx`; it then reads `[ebx+0x38]` and `[[ebx+0x38]+0x0C]` |
| `Update`'s return | **`al` = 1 (true)** | `0x00593997 mov al, 1` before `pop ebx; add esp,0x54; ret 4` |
| `Update`'s callee_pop | **4**, 1 arg | `ret 4` at `0x0059399D` |

**Corrects:** `docs/abi/vtables.tsv` row 5004 records `cSC4EffectsManager::vftable_1` slot 3 as
`eax-or-void` / `medium`. The body sets `AL = 1` and returns, so the channel is **`al`**, confidence **high**.
ui-windows.md §7's table for the same slot says `eax-or-void`; that is likewise **DIFFERS**.

**Exact call sites inside `Update`.** effects.md says `DeactivateUnusedVisualEffects` is "called only from
`0x00593120`", meaning from within `Update`. The precise call site is **`0x00593985`**; `DrawScreenFlashes` is at
**`0x0059395B`** and `0x0058EC40` at `0x0059397D`, both also inside `Update`. **PARTLY** — the chain is right, the
addresses are the call sites rather than the function entry.

**What UIW-6 asked and what is settled.** UIW-6 asks whether the view's paint reaches `Update` directly or through
the 3D renderer's draw pass. Settled so far:

- `cSC4View3DWin::GZPaint` does **not** reach it directly. Its entire call list (§5.1) contains no effects-manager
  call; the only draw call is renderer slot 21 `Draw`.
- `Update` takes an `SC4DrawContext*`, and `cSC43DRender::GetDrawContext` returns exactly such an object
  (`renderer + 0x11C`).
- `Update` and `cSC43DRender::DrawDynamicView` share two view-object walk helpers. `0x007C61A0` is called from
  `Update` at `0x00593335` and `0x00593474`, and from `DrawDynamicView` at `0x007C7B2E`. `0x0051CA60` is called from
  `Update` at `0x005932B8`, `0x005933F7` and `0x00593539`, and from `DrawDynamicView` at `0x007C7B6F`.
  `SC4ViewUtilities::IsBoundingBoxInViewFrustum` (`0x007F6770`) is used by `Update` (`0x00593275`, `0x005933B4`,
  `0x005934F6`) but **not** by `DrawDynamicView` — measured over the whole `0x007C7830`–`0x007C7DD0` body.

**Still `[inferred]`:** that the effects draw pass is invoked from *inside* the renderer's dynamic pass, passing the
same `SC4DrawContext` at `renderer + 0x11C`. **What would settle it:** a breakpoint on `0x00593120` during one
harness frame, or the `lea ecx, [reg+4]` / `call [reg+0xC]` pair that carries it — a full-image scan for that
shape found only nine pairs, none of them in the renderer (`0x005BE86A`, `0x005BE885`, `0x00619056`, `0x008D2AD3`,
`0x0093C3EB`, `0x0093C439`, `0x0093C780`, `0x0093D8F6`, `0x0094B2B4`). Two of them (`0x005BE860`, `0x005BE880`)
are **`cISC4VisualEffect` slots 7 and 8** — their vtable addresses `0x00AA118C` / `0x00AA1190` are consecutive
slots of `cISC4VisualEffect` (`0x00AA1170`, effects.md §11.1), and they forward to slot 3 of a *different*
sub-object at `effect + 4`, passing `effect + 0x6C` / `effect + 0x34`. They are **not** the manager's draw dispatch
and must not be used to answer UIW-6.

The practical consequence for a headless harness is unchanged and is already stated as effects.md EF2: a session
that never draws the city never runs the finish test. §11 test 19 covers it.

---

## 9. Interface slot tables (ABI rows)

Rows this lane derived are appended to `build-runs/pending-abi/re-specs21-rows.tsv` in
`docs/abi/interfaces.tsv`'s exact 12-column layout. **Do not edit `interfaces.tsv` itself** and do not regenerate it
(`docs/abi/README.md:58-62`).

Rows included:

- `cISC43DRender` slots 0–75 (the complete `0x00ABAB60` table), with slot 21's `ret` corrected to `al`.
- `cIGZMessageTarget2` slot 3 `cSC43DRender::DoMessage` (`0x00ABAB50`, the `+0x04` sub-object).
- `cISTETerrainView3D` slots 0–21 (vtable `0x00AB4410`) and slots 0–4 (vtable `0x00AB4480`).
- `cISC4EffectsManagerDraw` slot 3 `Update` (`0x00A9F250`), with `ret` corrected to `al`.

That is **105 rows** in all. `cISC4View3DWin` slot 4 `GetRenderer` (line 3102) and `cISTETerrain` slot 66
`GetView` are **already** in `interfaces.tsv` and are **not** regenerated.

---

## 10. Quirks

1. **`cSC43DRender::Draw` returns a status in `AL`, not `EAX`.** *[quirk]* of the ABI tables: `docs/abi/vtables.tsv`
   records `eax-or-void`. A declaration as `void` loses the "did I actually draw" status; a declaration as `int`
   reads garbage in the upper 24 bits. See §4.1.
2. **`Draw` contains the static pass twice.** *[quirk]* A replacement that draws the static view once will differ on
   the backing-store path: with `0x00B0DC19` set and the backing store not yet saved, the original draws the full
   static view, **saves it as the backing store**, then later may re-draw static again (§4.2).
3. **The static and dynamic gates are two different bytes of the same object** (`+0x5C` and `+0x3C`). *[quirk]*
   Drawing both from one flag is wrong in both directions.
4. **`Draw` is re-entrant-safe by an explicit flag, not by a lock.** `+0x108` makes a nested `Draw` return `AL = 0`
   immediately (§4.1 step 2). A replacement that allows nesting will re-enter `ProcessModelChangeQueue` and
   `ProcessDirtyRectangleQueue`. *[compat]*
5. **`ClearViewport` is reached through the renderer's own slot 47, not as a direct call.** (`0x007CB69A`
   `call [eax+0xbc]`.) *[quirk]* A subclass that overrides slot 47 changes what `Draw` does; a replacement that
   calls its own clear directly loses that hook.
6. **`Set3DCameraView` (slot 24) is called twice per `Draw`** — at `0x007CB6A0` and again at `0x007CB811`, with the
   dynamic pass in between. *[quirk]* A single call at the top is not equivalent.
7. **The screen-shake offsets are zeroed only if a service byte is clear** (`0x007CB889`–`0x007CB8A5`), and only at
   the very end of `Draw`. A `Draw` that early-outs at step 2 leaves them untouched. *[quirk]*
8. **`cSC4View3DWin::GZPaint` early-outs on a byte at `view + 0x15C` before doing anything**, including before
   updating the cost indicator. Hiding the view is therefore not the same as "paint an empty view". *[compat]*
9. **`GZPaint` drives the sim pause.** Steps 7–10 call `DoTemporarySimHiddenPause` (`0x007E5B40`) and
   `DoTemporarySimHiddenResume` (`0x007E5B70`) from inside the paint path. A headless run that never paints never
   lifts that pause. *[compat]* — this is the same class of coupling as effects.md EF2 and is covered by test 20.
10. **`cSC4View3DWin::AsIGZWin` and `cSTETerrain::GetView` are the same trick** — `return obj + 0x0C`. *[quirk]* of
    the object model. Any code that assumes "the pointer I was given is the object" will be 12 bytes out.
11. **`cSTETerrainView3D::Init` dereferences `0x00B43CD8` and `0x00B43CF0` with no null test**, while it *does*
    null-test `0x00B43CEC` and its own `this + 0x28`. *[quirk]* With no preferences object or no surface water, the
    original crashes in `Init`. A replacement must not "helpfully" guard these if it wants to reproduce the crash;
    for a standalone game, guarding them is *[choice]* and should be logged.
12. **`cSTETerrainView3D::Init` returns `AL = 0` when it is already initialised**, and `AL` is uninitialised on the
    success path's fallthrough — the body sets `this+0x18 = 1` and continues, and the only explicit `xor al,al` is
    the early-out at `0x0075C43F`. *[quirk]* Treat the return as "did I do work", not "did it succeed"; see test 16.
13. **`cSTETerrainView3D::Init`'s `+0x19` flag caches the preference byte into a global (`0x00B4C741`) exactly
    once.** Changing the preference later does not update that global unless `+0x19` is cleared. *[quirk]*
14. **The terrain global is stored twice** — `0x00B43CF4` *and* `0x00B43CF8` get the same `GetTerrain` result
    (`0x00601C34`, `0x00601C39`), while `0x00B43CF0` gets `GetSurfaceWater`. *[quirk]* of the cache layout. A
    replacement that aliases them all to one pointer will be wrong for the terrain view.
15. **The 2D span block is 203 KB of template instantiations** and is entirely reproducible in a few hundred lines
    of real code. *[choice]* FreeCity4 may implement the 2D blitter however it likes; only the pixels are *[compat]*
    (§7.3).
16. **`cGZDrawContextRenderToBuffer::Draw` and `DrawPreclippedSpanList` have no direct callers at all** — they are
    reached only as `cIGZDrawContext` vtable slots 21 and 22. *[quirk]* A search for call sites finds nothing and
    the block looks dead; it is not.
17. **`cISC43DRender` slots 18 and 19 share one folded body** (`0x004E13E0`,
    `GetProjectionMatrixEntries`/`GetViewMatrixEntries`). *[quirk]* of identical-code folding. Writing two
    declarations with different signatures is fine; they must still be two slots.
18. **`cISC43DRender` slot 31 `ChangeModel` tail-jumps through a 5-entry table** at `0x007C94B0`. *[quirk]* The
    ABI row's `callee_pop` of 8 is the extractor's best reading; a replacement must match the *dispatch*, not a
    single body.

---

## 11. Conformance tests

An implementer should be able to make all of these pass without asking a question. "The renderer" is any object
answering `cISC43DRender`; where a test needs the original, it is marked.

1. **`GetDrawContext` is a field read.** `renderer.GetDrawContext()` returns the same pointer twice in a row and
   equals `*(renderer + 0x11C)` of the original. *(original)*
2. **`Draw` returns `AL`.** Call `Draw()` on a renderer whose `+0x108` is already set; the low byte of the return is
   0 and nothing is drawn. *(original)*
3. **Re-entrancy.** Inside a `Draw` callback, call `Draw()` again; the nested call returns 0 immediately and does
   not run `ProcessModelChangeQueue` or `ProcessDirtyRectangleQueue`. *(original)*
4. **Draw counter.** `+0x158` increases by exactly 1 per completed `Draw`, and by 0 for the early-out paths.
   *(original)*
5. **Static/dynamic ordering.** With both gates set, record the order of the six calls; it is
   `PreStatic, Static, PostStatic, PreDynamic, Dynamic, PostDynamic`. *(original)*
6. **Independent gates.** With `service+0x5C = 0` and `service+0x3C = 1`, only the dynamic triple runs. With
   `+0x5C = 1` and `+0x3C = 0`, only the static triple runs. *(original)*
7. **Pass mode word.** After `DrawStaticView`, `*(drawContext + 0x50) == 1`; after `DrawDynamicView`, `== 4`.
   *(original)*
8. **`ClearViewport` is hookable.** Override `cISC43DRender` slot 47; a `Draw` with `+0x60` clear calls it exactly
   once, and one with `+0x60` set never calls it. *(original)*
9. **`Set3DCameraView` is called twice** per full `Draw` — before and after the dynamic pass. *(original)*
10. **Backing store.** With `0x00B0DC19` set and `+0x65` clear, the first `Draw` runs the static pass and then
    `SaveBackingStore(0, 0, +0x68, +0x6C)` and sets `+0x65`; the second `Draw` takes the `RestoreBackingStore`
    path instead. *(original)*
11. **Partial updates.** `UpdateViewportRect` re-runs the **static** triple only; `DoTranslatedViewUpdate` re-runs
    the **dynamic** triple only. *(original)*
12. **Dirty-rect deadline.** With a non-empty dirty-rect queue and `+0x158` outside the window
    `[+0x1F8, +0x1F8 + service(+0x18)->+0x2EC]`, `Draw` calls `ProcessDirtyRectangleQueue` exactly once.
    *(original)*
13. **`GZPaint` gate.** With `view + 0x15C` clear, `GZPaint` does not call `UpdateCostIndicator`, does not touch the
    camera control and does not call `Draw`; it still runs the tail. *(original)*
14. **`GZPaint` period.** The paint only proceeds when the timer's tick has passed `view + 0xFC`. *(original)*
15. **`GZPaint` → `Draw`.** Exactly one call to `cISC43DRender` slot 21 per successful paint, and none on the
    early-out paths. *(original)*
16. **`GetView` shape.** `terrain.GetView() == *(terrain + 0xE8) + 0x0C`, and is null when `*(terrain + 0xE8)` is
    null. *(original)*
17. **`cSTETerrainView3D::Init` arity.** It is `__thiscall` with one dword (the terrain) and `ret 4`; calling it
    with no argument corrupts the caller's stack by 4 bytes. *(original)*
18. **`Init` idempotence.** A second `Init` returns `AL = 0` and does not re-read `0x00B43CD8` or `0x00B43CF0`.
    *(original)*
19. **Effects draw pass is paint-driven.** A session that never paints the city view never runs
    `cSC4EffectsManager::DeactivateUnusedVisualEffects` (`0x005922A0`) and never posts an effect end message
    (effects.md §7, EF2). *(original; a headless harness must synthesise the sweep)*
20. **`GZPaint` drives the sim pause.** After a paint that takes step 7's `ScrollByViewportAngle` branch,
    `DoTemporarySimHiddenPause` has been called; after the timer at `+0x160` exceeds `+0x17C`, `…Resume` is called.
    *(original)*
21. **Span-renderer selection.** For each of the five colour types, `cGZDrawContextRenderToBuffer::Init` creates the
    expected span renderer into `+0xB0`: type 2 → `0x00AC188C`, 4 and 6 → `0x00AC182C`, 5 → `0x00AC1844`,
    7 → `0x00AC185C`, 8 and 9 → `0x00AC1874`. *(original)*
22. **Span blit equivalence.** For a fixed span list and a fixed source, a replacement 2D blitter produces the same
    pixels as the original for all five colour types. *(original for the reference; the dispatch table itself is not
    tested)*
23. **`DrawPrims` reaches the 3D context.** An occupant's `Draw(SC4DrawContext*, bool)` that calls `DrawPrims`
    reaches the context returned by `GetDrawContext`, not any `cGZDrawContextRenderToBuffer`. *(original)*
24. **Terrain view holds three references.** After `Init`, `this+0x1C` is the city (`0x00B43CEC`), `this+0x20` is the
    terrain argument and `this+0x24` is `0x00B43CF0` (surface water); each has been `AddRef`ed exactly once per
    empty field. *(original)*
25. **View3D save references.** The View3D record's first two references are `view + 0xF0` (camera control) and
    `view + 0xF4` (3D renderer), and neither the renderer's draw counter nor its dirty-rect queue appears in the
    record. *(original; save-records.md §2)*

---

## 12. Open questions

1. **V3D-1 (settles ui-windows.md UIW-6).** Does the effects draw pass run from inside `cSC43DRender`'s dynamic
   pass, and with which `SC4DrawContext`? §8 measures the shape and the shared helpers but not the link. Settle by a
   breakpoint on `0x00593120` under one harness frame.
2. **V3D-2.** What `SC4DrawContext` slot 3 (`+0x0C`) does — the one every `DrawPre*`/`DrawPost*` bracket calls
   (`0x007C3E3E` and three siblings). Settle by reading `SC4DrawContext`'s vtable row 3 in `docs/abi/vtables.tsv`.
   **Answered 2026-10-21 (re-render3d):** no draw-context slot is called. Each bracket sets the draw context's pass
   word (`+0x50`: 0, 2, 3, 5 for `DrawPreStaticView`, `DrawPostStaticView`, `DrawPreDynamicView`,
   `DrawPostDynamicView`; the passes themselves are 1 and 4) and calls slot 3 (draw) of every view object registered
   in its layer (render3d.md §11.1), passing the draw context.
3. **V3D-3.** What `0x00B43CC4` is and what its `+0x0C` object's `+0x5C` and `+0x3C` bytes mean (the static and
   dynamic gates). Settle by the writer of `0x00B43CC4`.
   **Answered 2026-10-21 (re-render3d):** `0x00B43CC4` is the render properties (render-properties.md §1.1); its
   `+0x0C` is the boolean table, whose byte `+0x5C` is boolean 2 `kRenderStaticView` and `+0x3C` boolean 1
   `kRenderDynamicView`.
4. **V3D-4.** What `0x00B0DC19` is (the backing-store master switch) and who writes it.
   **Answered 2026-10-21 (re-render3d):** `0x00B0DC19` is the backing-store switch, 1 at start;
   `SetBackingStoreEnabled` (slot 57) writes it, forcing it off when no backing-store surface exists (render3d.md
   §3.4, §12).
5. **V3D-5.** The full `SC4DrawContext` slot table. This spec names 30 of its methods from `re/names_windows.tsv`
   but gives no slot numbers. Settle by generating its vtable rows (`cSC4DrawContext` is unnamed in
   `docs/abi/vtables.tsv` because it has no class vtable entry there).
6. **V3D-6.** What object is at `view + 0xE8` in `cSC4View3DWin` (its slot 9 is read every paint and written into
   `SC4DrawContext + 0x1C`). Presumably the lighting manager (`cISC43DRender` slot 9 `GetLightingManager`).
7. **V3D-7.** Whether `cSC43DRender::Write`/`Read` (`0x007C30F0`/`0x007C3180`) carry the renderer's state and in
   which save record.
8. **V3D-8.** `cSTETerrainView3D`'s object layout beyond `+0x18`/`+0x19`/`+0x1C`/`+0x20`/`+0x24`/`+0x28` (all
   measured in `Init`). The cell-quad vector, the water-colour rect and the selection state are not yet offset.
9. **V3D-9 (carried, ui-windows.md UIW-10).** The MXF font format and the UI-script grammar — still on the critical
   path for S4 and out of scope here.
10. **V3D-10.** The `cISC4View3DWin` message-filter sub-object at `+0x08` (ui-windows.md UIW-9) — not a draw-layer
    concern but adjacent.

---

## 13. The x87 precision trigger (answers the brief)

The brief asks this lane to note where the "what raises the main thread to 53-bit during app init" trigger lives.

**It is not in `SimCity4.exe`, so it must be in `FreeCity4.dll`.** ui-windows.md §16 already established this and
this lane **confirms** it without re-deriving the whole chain:

- The CRT raises the thread to 53-bit **before `WinMain`** (`__start` `0x009F15DC` → `__cinit` → `__fpmath` →
  `__setdefaultprecision` `0x009F6C30` → `__controlfp` → `__control87` `fldcw` at `0x00A02F93`).
- `cSC4App::PreFrameWorkInit`'s **first** call (`0x0045236C` → `0x005F85E0`) *lowers* it to 24-bit, and
  `0x005F85E0` has exactly one xref in the image.
- A function-granularity sweep finds 137 `fldcw`, 84 `fnstcw`, 2 `fnclex`, 1 `fldenv`, and **no** `fninit`/`finit`/
  `frstor`/`fxsave`. The only unguarded 53-bit raise is the additive text parser `0x0067BA40`, whose callers are all
  effect/particle/camera `*::Parse` and colour/vector parsers — text resource parsing, not app init.
- Therefore nothing in the shipped exe raises precision between `PreAppInit` and `PostAppInit`, and the writer is
  inside a DLL that `FreeCity4.dll`'s presence causes to load (ui-windows.md §16.2's minimal-DLL test is the
  instrument that would pinpoint it).

**One place it touches the drawing layer:** `__CIcos`/`__CIsin`/`__CItan` force 53-bit whenever the caller is not
already at 53-bit and then restore (ui-windows.md §16.3, **Corrects:** foundations.md §18.1). Their only callers in
the image are `cGZPieGraph::GetLineFromCenterOfDiskToEdge` (`0x009B2E3B`, `0x009B2E5B`, `0x009B2EA3`, `0x009B2EC3`)
and `FUN_009b27bd` (`0x009B2851`) — pie-graph drawing, a **UI widget**, not the 3D view. The 3D path uses `fsqrt` and
the precision-preserving `lib::__load_CW` family (`0x009FAB25`) and is unaffected. A replacement that draws pie
graphs while the thread is at 24-bit will see `sin`/`cos` computed at 53-bit and truncated back on restore: the
drawing is safe, an intermediate-value comparison across those calls is not.

---

## 14. Replaceability

- **The 3D renderer is replaceable as a unit** if it answers `cISC43DRender` (76 slots) and holds an
  `SC4DrawContext` at `+0x11C`. The simulation reaches it only through `cISC4View3DWin` slot 4 `GetRenderer` and
  through the draw pass (landfill.md §4.7.5, power.md §1003).
- **The 2D blitter is replaceable freely.** Only the pixels are *[compat]* (§7.3, test 22).
- **The terrain view is replaceable as a unit** if it answers `cISTETerrainView3D` (22 slots) and is reachable
  through `cISTETerrain` slot 66. It is rendering-only: terrain.md §13.2 saves a two-value record for it and nothing
  in the simulation reads its state.
- **`cSC4View3DWin` is not replaceable from this spec alone** — ui-windows.md §14 gives its 149-method surface and
  its catalog menus. What this spec adds is only the paint contract.
- **Nothing in the drawing layer is saved** except the two View3D references (`view + 0xF0`, `view + 0xF4`) and the
  terrain view's 3-byte record. A renderer with no serialisation support can still round-trip a city if it gives an
  exact opaque round trip for those records (save-records.md).
