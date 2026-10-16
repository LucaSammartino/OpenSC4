| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-specs21 (RE spec: the 3D view and the drawing layer) | `re-specs21` / (RE lane) | **ready to merge, docs only** (new `docs/specs/view3d.md`, 14 sections, 18 quirks, 25 conformance tests; `docs/specs/terrain.md` gains §15.5 (the terrain 3D view) and `docs/specs/effects.md` §7 gains the exact call sites plus a DIFFERS on the return channel; no game runs). **Draw-pass entry point found and ordered:** `cSC43DRender::Draw` = `cISC43DRender` slot 21 (`+0x54`, `0x007CB530`), reached from `cSC4View3DWin::GZPaint` (`0x007F6200`) through the renderer at `view+0xF4`; order `DrawPreStaticView 0x007C3E90 -> DrawStaticView 0x007C7370 -> DrawPostStaticView 0x007C3ED0` then `DrawPreDynamicView 0x007C3E10 -> DrawDynamicView 0x007C7830 -> DrawPostDynamicView 0x007C3E50`, each triple gated on a different service byte (`+0x5C` static, `+0x3C` dynamic); two more sites run the same triples for partial updates (`UpdateViewportRect 0x007C94C0` static, `DoTranslatedViewUpdate 0x007CA4D0` dynamic). **Span block answered:** `0x008310B0`-`0x00864B58` is NOT the 3D path - it is `nGZDrawSoftRender`'s five `cSpanRender*` 2D blitters under `cGZDrawContextRenderToBuffer` (ctor `0x00830690`, span renderer at `+0xB0`, 840 dispatch slots); the 3D path is `SC4DrawContext` (`DrawPrims 0x007D2990`, 58 call sites, all occupant/model `Draw`); `tools/coverage-ranges.tsv` is correct as it stands. **Corrects:** `0x00B43CF0` is `cISTESurfaceWater` (city slot 83), not the terrain - `0x00B43CF4`/`0x00B43CF8` are `GetTerrain` (city slot 84), measured in the `SL::` cache setter `0x00601C20`; `cISC43DRender` slot 21 returns `al` (1/0) not `eax-or-void`; `cSC4EffectsManager::vftable_1` slot 3 `Update` returns `al = 1` not `eax-or-void`, `this = manager+4`, call sites `0x00593985`/`0x0059395B`/`0x0059397D`. **ABI rows left behind:** `build-runs/pending-abi/re-specs21-rows.tsv`, 104 rows (76 `cISC43DRender` 0-75 of `00abab60`, 27 `cISTETerrainView3D` of `00ab4410`/`00ab4480`, 1 `cISC4EffectsManagerDraw`); `docs/abi/interfaces.tsv` not touched and not regenerated. x87 confirmed still not in the exe (view3d.md §13) | Next: (1) apply the 104 pending ABI rows and check conformance slot counts for `cISC43DRender` (76) and `cISTETerrainView3D` (22 + 5); (2) settle V3D-1 (does the effects draw pass run inside `cSC43DRender`'s dynamic pass, and with which `SC4DrawContext`) with a breakpoint on `0x00593120` - that also closes ui-windows.md UIW-6; (3) V3D-2 `SC4DrawContext` slot 3 (the `DrawPre*`/`DrawPost*` bracket), V3D-3 what `0x00B43CC4` and its `+0x5C`/`+0x3C` bytes are, V3D-4 what `0x00B0DC19` is, V3D-5 the `SC4DrawContext` slot table (30 methods named, no slot numbers yet), V3D-6 the object at `view+0xE8`, V3D-7 `cSC43DRender::Write`/`Read` (`0x007C30F0`/`0x007C3180`), V3D-8 `cSTETerrainView3D`'s layout past `+0x28`; (4) ui-windows.md UIW-10 (MXF fonts, UI-script grammar) is still on the S4 critical path and untouched here |

## Next steps

1. **ABI.** `build-runs/pending-abi/re-specs21-rows.tsv` is in `docs/abi/interfaces.tsv`'s exact 12-column
   layout and was generated from `docs/abi/vtables.tsv` by
   `build-runs/re-specs21-scratch/genrows.py` (scratch, gitignored) so no row is hand-transcribed. Two rows
   carry measured corrections that the generator applied: `cISC43DRender` slot 21 `Draw` (`al`, was
   `eax-or-void`) and the effects draw interface slot 3 `Update` (`al`, was `eax-or-void`). Rows already in
   `interfaces.tsv` were **not** regenerated: `cISC4View3DWin` slot 4 `GetRenderer` (line 3102) and
   `cISTETerrain` slot 66 `GetView`. `cISC4EffectsManagerDraw` is a **new** interface name for
   `cSC4EffectsManager::vftable_1` (`00a9f250`), the draw interface whose sub-object sits at `manager + 0x04`;
   if the coordinator prefers a different interface name, only that column changes.
2. **The one thing that would most improve the spec** is V3D-1. Everything else in the draw chain is measured;
   the link from the renderer's dynamic pass to `cSC4EffectsManager::Update` is still *[inferred]*, and it is
   exactly ui-windows.md UIW-6. A single breakpoint on `0x00593120` during one harness frame settles it.
3. **`SC4DrawContext` is the largest unspecified surface left in the drawing layer** (V3D-5). Thirty of its
   methods are named in `re/names_windows.tsv` (`0x007D2750`-`0x007D5F10`) but none has a slot number, because
   the class has no row in `docs/abi/vtables.tsv`. Anyone picking this up should generate those rows the same
   way `cSC43DRender`'s were: read the vtable installs and the call sites.
4. **Do not treat the span block as unattributed any more.** `tools/coverage-ranges.tsv` already owns it and is
   right; view3d.md §7.2 restates the five ranges with their owners so the bytes have a prose home. The end
   address is `0x00864B60` (where `coverage-libs.tsv` puts the EA sound library), not `0x00864B58`.
