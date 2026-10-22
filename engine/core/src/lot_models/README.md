<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Lot model public-contract checklist

Source: `docs/specs/lot-models.md` §§1–4, 8; edge normals/visible edges from
`docs/specs/view-utilities.md` §4.2. No ABI, maps, native analysis or owner
output callbacks were used. The task's 3 owners / 82 functions / 11439 bytes
are not an inventory in this public spec: it names only the entries below.
There is no claim of complete-owner/native coverage (LM2).

| Owner | Published entries/decisions implemented | Missing native contracts |
|---|---|---|
| `cSC4FoundationModel` | owned portable construction/drop; semantic box/face spans and per-normal colours (`SetupVertices`); water-aware border maximum, upward-rounded 16 m extension and forced rebuild (`AdjustHeightToTerrain`); two per-zoom keys and fallbacks; sort; highlighted/rotation/zoom gates and ordered visible-side/top draw plan | constructor/destructor defaults, inherited base and pool/refcount/IID inventory (LM2); native vertex/UV ordering, top texture-scale failure semantics, exact numeric evaluation (LM3); factory failure and draw failure gates (LM5) |
| `cSC4LotRetainingWallModel` | owned portable construction/drop; explicit edge samples, seam-expanded per-segment bands (`Init`/`SetupVertices`); keys/fallback (`SetupTextures`/`DoZoomChange`); sort; highlight/facing/rotation/zoom draw decisions; across-edge flat-bit query | box-to-cell range, inner/across-cell mapping, offset selection, crossing/degenerate triangles, vertex colour formula/ordering (LM4); lifecycle/base (LM2); failed loads (LM5) |
| `cSC4CellTextureModel` | record; per-zoom FSH lookup and alpha metadata; orientation/mirroring with explicit input corner cycle; flat/terrain placement and bounds; three draw states; mode/pass/filter/environment/combiner decisions; strip versus terrain request and force flag | native record/default/lifetime/base (LM2); exact corner/strip cycle, lighting packing, combiner construction, nonfinite/overflow arithmetic (LM6–LM8); LM1 modes and flat-path bypass |

Portable geometry describes labelled faces/bands and preserves known dimensions;
it does not pick a native winding or triangle diagonal. `WallShape::triangles`
returns an explicit missing-contract error, never a fabricated mesh. Cell corner
layout is explicit caller-supplied coordinate data, not a call to an original
model's corner routine. Providers only load owned texture leases or sample light.
Native wall recolouring remains an explicit pending flag rather than a callback
to the wall owner's original `SetupVertices`/Draw.

All inputs to construction are explicit: no native defaults are assumed. Rust
ownership models lifetime, with immutable getters and exclusive mutators; native
pool allocation, QueryInterface and AddRef/Release are unpublished contracts.
The API rejects out-of-domain inputs instead of asserting native error behavior.
Floating geometry uses the existing `Vec3 = [f32; 3]`; LM3/LM8 track exact native
precision. No fallback white/black lighting or missing-texture success is invented.
