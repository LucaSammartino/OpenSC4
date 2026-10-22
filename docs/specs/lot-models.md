# Spec: lot render models (`cSC4FoundationModel`, `cSC4LotRetainingWallModel`, `cSC4CellTextureModel`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [building-development.md](building-development.md) (the foundation and retaining-wall occupants,
their types and properties; DG35), [terrain-view.md](terrain-view.md) (§4.4 `DrawTexturedTerrainQuad`, which draws
sloped cell textures), [draw-context.md](draw-context.md), [lighting.md](lighting.md) (the light colours),
[view-utilities.md](view-utilities.md) (§4.2 visible edges), [water-ripples.md](water-ripples.md) (§2.2, whose
textured-cell modes the cell texture repeats).

## 0. Summary

Three small model instances draw the parts of a lot that are not building models: the **foundation** box under a
building on a slope, the **retaining walls** along a lot's edges where the terrain falls away or rises, and the
**cell textures**, one textured square per lot cell. Each keeps its own vertices, rebuilds them when the zoom or
rotation changes, takes its texture per zoom with a fallback key, and draws with a fixed opaque state.

## 1. Common behaviour *[compat]*

- All three derive from `cS3DModelInstanceBasic` (as a virtual base; pool-allocated, freed to the pool by the
  deleting destructors) and answer one IID each besides the basic ones: `0xA9E2831D` (cell texture, `0x005DEB00`),
  `0xA9E2831E` (retaining wall, `0x005E1200`).
- **Sort value** (`GetSortValue`): the current texture's ID OR a class constant, so models sharing a texture draw
  together; without a texture a fixed value. Foundation: `| 0x2AB6`, else `0xE9862AB6` (`0x005D5570`); wall:
  `| 0x2A86`, else `0xC9862A86` (`0x005E14F0`).
- **Zoom changes** load textures through the binding factory. A zero configured foundation/wall key disables
  that binding. A failed lookup of a nonzero key replaces that zoom's stored key with the fallback and retries
  once, including when that key already equals the fallback (§§2.1, 3.1).
- The draw state, made once from the 3D view manager's draw context, is colour and depth write, depth test
  less-or-equal, one texture, no alpha test, no blending, no culling.

## 2. Foundations (`cSC4FoundationModel`) *[compat]*

- Constructor `0x005D5D70`; destructor body `0x005D5300`.
- **Shape** (`SetupVertices(geometry, colours, lighting)`, `0x005D55C0`): a box over the occupant's footprint from
  the base height to the top height: four side faces of four vertices and a top face. The sides' texture
  coordinates count one repeat per 16 m; the top's divide the footprint by the top texture's scale (`+0x58`). With
  `colours`, each side takes the lighting manager's light for its outward normal (`GetVectorForEdge`, lighting
  slot 20), the top the light for (0, 1, 0).
- **Height** (`AdjustHeightToTerrain`, `0x005D5A90`): use the maximum border drop to the higher of ground/water.
  Compare truncated `(depth+16)/16` counts for required and existing depth, as specified in §2.2. This differs
  from mathematical ceiling at exact multiples of 16. Rebuilding changes geometry without relighting.
- **Textures** (`0x005D5C70`): a side texture and a top texture per zoom; failed nonzero side keys fall back to
  `0x69F20000 + zoom`, failed nonzero top keys to `0x0912220F + zoom`. Zero keys are disabled.
- **Draw** (`0x005D5FC0`): not when highlighted. A rotation change re-takes the two visible edges (view-utilities
  §4.2) and recolours; a zoom change reloads the textures. With a side texture, the two visible sides are drawn as
  four-vertex primitives of type 6 (quads, graphics-formats.md §3.7); then the top with the top texture.

### 2.1 Binding failure and independent top draw (LM5)

`SetupTextures(previousZoom, newZoom)` does nothing when the two full arguments compare equal.
Otherwise it releases and clears the old side binding, then attempts the nonzero side key. A null result
stores `0x69F20000 + newZoom` in that key slot and performs exactly one further lookup. It then releases
and clears the old top binding and applies the same rule with `0x0912220F + newZoom`. No third lookup
or restoration of the old binding occurs. Keys are not changed when initially zero. These are ID-only
factory calls, not TGI construction in this owner (`0x005D5CAB–0x005D5D39`).

The ID-only factory entry is `0x008094C0`: it supplies type `0x7AB50E44` and group `0x1ABE787D`.
Sort IDs are the first underlying texture object's first 32-bit word, not the configured resource instance:
foundation prefers the side binding, then the top binding, then `0xE9862AB6`, and ORs the result with
`0x2AB6` (`0x005D5570`). Walls similarly OR their first texture word with `0x2A86`, falling back to
`0xC9862A86` (`0x005E14F0`); cells use that texture word directly, or their configured key if unbound.

`DoZoomChange` stores the zoom byte before loading (`0x005D5EA8`); a failed load therefore does not
cause retries on every subsequent draw at the same ordinary zoom. Returning to that zoom after another
zoom retries its now-stored fallback key. Top drawing has its own binding/first-texture checks and is
reachable even if the side is absent or its first texture is zero (`0x005D6084–0x005D60F1`).

### 2.2 Foundation sampling and step arithmetic (LM3)

Bounds store `(x0,base,z0,x1,top,z1)` as binary32. Convert each horizontal bound to a lattice index by
storing `coordinate × 0.0625 + 12582912` as binary32, reinterpreting that word as an integer and
subtracting `0x4B400000`. This is the native nearby-integer bit technique under the active floating-point
rounding mode, not truncation or floor (`0x005D5A98–0x005D5B1E`). Enumerate x from its lower index
through its upper index inclusive, with z increasing in the inner loop. Only points on the rectangle's
border are sampled. Corners are visited once. A degenerate rectangle samples every enumerated point;
reversed index bounds sample none. Terrain index = x + z × (terrain-width + 1). Read ground first
(terrain slot 12), water second (slot 31), store each result as binary32, and select their maximum.
Required drop starts at zero; replace it only when `top − selectedHeight` is larger, storing each
replacement as binary32 (`0x005D5B42–0x005D5C10`).

Let trunc mean the native `_ftol2` conversion toward zero, with its signed 32-bit low result used here.
Existing count C = trunc(((top−base)+16)×0.0625); required count N = trunc((maximumDrop+16)×0.0625).
These count expressions stay in x87 registers until conversion; they do not have an intermediate
binary32 store after adding 16. If N>C, store binary32 `top − signed32(N << 4)` as the new base.
Otherwise leave base alone. Return true and rebuild geometry if N>C or force is true; else return
false. The rebuild calls `SetupVertices(true,false,null)`, so neither a changed base nor a forced
unchanged base relights (`0x005D5B12–0x005D5B33`, `0x005D5C10–0x005D5C66`).

The constructor initializes an empty bounds sentinel `(1,1,1,0,0,0)`, not a usable footprint or a
separate step counter (`0x005D5D70`). Thus its algebraic existing count is zero; placement must supply
the intended bounds before relying on meaningful terrain adjustment. Examples requiring a starting
base state are given in §8; top/ground/water alone do not determine whether an already-deep box moves.

### 2.3 Foundation buffer and UV order (LM3)

Four side quads, numbered 0–3, each contain four consecutive 24-byte position/BGRA/UV records; a
separate four-record top quad follows the top binding. Define W = binary32(x1−x0), D = binary32(z1−z0),
H = binary32(top−base), X = x0+W, Z = z0+D and B = top−H. Native reconstruction uses these additions
and subtraction, rather than copying all upper bounds. Each listed triple is `(x,y,z)`.

| Face | Vertex 0 | Vertex 1 | Vertex 2 | Vertex 3 |
|---|---|---|---|---|
| 0 | X,top,z0 | X,B,z0 | x0,B,z0 | x0,top,z0 |
| 1 | X,top,Z | X,B,Z | X,B,z0 | X,top,z0 |
| 2 | x0,top,Z | x0,B,Z | X,B,Z | X,top,Z |
| 3 | x0,top,z0 | x0,B,z0 | x0,B,Z | x0,top,Z |
| top | x0,top,z0 | x0,top,Z | X,top,Z | X,top,z0 |

Every side uses UVs `(0,0),(0,H/16),(W/16,H/16),(W/16,0)`. In particular, **sides 1 and 3 also
use W/16**, even for a nonsquare footprint. Top UVs are `(0,0),(0,V),(U,V),(U,0)`, where the owner
computes binary32 reciprocal R = 1/topScale, then U = (W×0.0625)×R×16 and
V = (D×0.0625)×R×16, stored as binary32. Scale is metres per repeat and defaults to 16. There is
no zero-scale guard: with masked x87 exceptions a zero scale can produce infinity or NaN UVs;
the owner neither substitutes 16 nor rejects the update (`0x005D55D5–0x005D56B5`).

Geometry and lighting are separate flags. Side colour is constant over each face, from lighting slot 20
with `GetVectorForEdge(face)`; top colour uses normal (0,1,0). Colours are BGRA bytes with alpha 255.
Both side and top use four-vertex primitive type 6, format 10. Buffer assignments and top lighting:
`0x005D56B9–0x005D5A4E`; this contract supplies native record order, not generic box triangulation.

## 3. Retaining walls (`cSC4LotRetainingWallModel`) *[compat]*

- Constructor `0x005E17C0`; destructor body `0x005E1A60`. `Init(box, side, lot)` (`0x005E2780`): the wall's cell
  range along the side (0–3) of the lot's box (`SetupVerticesFromBBoxAndWallEdge`, `0x005E1520`), then its vertices.
- **Shape** (`SetupVertices`, `0x005E1BC0`): the lot's flat altitude (the lot at the wall's inner cell, lot slot 16)
  against the terrain altitude at each vertex along the edge (terrain slot 14). For each 16 m segment the wall
  fills the gap between the lot level and the terrain, above or below, as triangles (position, colour, texture
  coordinates) set 0.001–0.002 m out from the edge, with the top and bottom carried 0.2 m past the lot level and the
  terrain to close the seams, and v = height × the wall's texture scale; segments where the terrain meets the lot
  level get nothing.
- **Textures** (`SetupTextures`, `0x005E1620`; `DoZoomChange`, `0x005E1710`): the zoom's key from FSH group
  `0x891B0E1A`; a missing key falls back to `0x695C00C6 + zoom` — the five shipped wall textures `0x695C00C6`–
  `0x695C00CA` of the Terrain Properties exemplar's `0x295961F2` (terrain-view.md §1).
- **Draw** (`0x005E27C0`): not when highlighted. A zoom change reloads the texture; a rotation change recolours.
  With the facing test on (`+0x6A`), a wall is drawn only when its side is the rotation or the rotation + 3 (mod 4),
  the sides facing the camera *[inferred]*; then one triangle list of its vertices.
- `0x005E1240` tells whether the lot on the cell across the wall's edge is flat (lot flag bit 0) *[inferred: to skip
  walls between flat lots]*.

### 3.1 Binding failures (LM5)

`SetupTextures(previousZoom, newZoom)` compares the full arguments first. On a change it releases and
clears its old binding. A zero key makes no resource request. A nonzero key requests FSH type
`0x7AB50E44`, group `0x891B0E1A`, instance equal to the stored per-zoom key. If the result is null,
it overwrites that key with `0x695C00C6 + newZoom` and makes one more request, even if identical to
the first. A second null result stays null (`0x005E1620–0x005E16CD`). Draw skips a null binding
(`0x005E2851–0x005E2856`); it does not construct geometry through an external owner to recover it.

## 4. Cell textures (`cSC4CellTextureModel`) *[compat]*

- Constructor `0x005DF880`; destructor body `0x005DEAB0`. A record (`0x005DF9A0`): the texture key, the cell
  (x, z), the orientation (0–3), a mirror flag, a colour (RGB), an alpha and a mode.
- **Texture** (`0x005DF020`): FSH type `0x7AB50E44`, group `0x0986135E`, instance key + zoom, else the key; the
  texture's own alpha flag is kept.
- **Corners** (`0x005DEEA0`): the four texture corners are rotated by the orientation and mirrored in u by the flag.
- **Place** (`0x005DF9E0`): the cell is 16 m square at (16 x, 16 z). When the lot on that cell is flat (lot manager
  slot 6, the lot's flag bit 0), the four corners take the lot's altitude and the lighting manager's colour for each
  corner (slot 19, normal (0, 1, 0)); otherwise the terrain's corner altitudes (terrain slot 17) and no colours. The
  bounds are the square inset by 0.1 m and the height range widened by 2 m.
- **Init** (`0x005DF100`): three draw states: opaque; alpha (alpha test "greater" at 0, blend source alpha / one
  minus source alpha); tinted (blending, no depth write, depth less); and a combiner record.
- **Draw** (`0x005E0200`): not when highlighted. A zoom change reloads the texture; the mode is **tinted** when the
  record's mode is set, else **alpha** when the texture has alpha or the alpha is not 255, else **opaque**; alpha and
  tinted cells are skipped while the draw context's `+0x04` is 0 *[inferred: drawn in the translucent pass]*. Wrap
  clamp. Tinted: linear filtering, the environment colour := the record's RGB × the global light ÷ 255 and
  alpha ÷ 255, the combiner with the record's mode, where mode 3 with `kNoAddSigned` becomes mode 1 at half the
  alpha. On a flat lot the model draws its own four-vertex strip; on a slope it asks the terrain view to draw the
  cell (`DrawTexturedTerrainQuad`, terrain-view.md §4.4) with its corners and alpha, forced when the draw context's
  `+0x20` is 8.

### 4.1 Corner, placement and update contract (LM6)

The stored cell x and z are **unsigned bytes**, not signed world coordinates (`0x005DFA0C`,
`0x005DFA10`). Let X = 16x and Z = 16z. Vertices are 24-byte position/colour/UV records in this order;
the flat draw submits these four records directly as primitive type 1, format 10 (`0x005E0475–0x005E0482`).

| Vertex | World x,z | Terrain altitude output slot | UV orientation 0 | UV orientation 1 | UV orientation 2 | UV orientation 3 |
|---|---|---|---|---|---|---|
| 0 | X,Z | 0 | 0,0 | 0,1 | 1,1 | 1,0 |
| 1 | X+16,Z | 1 | 1,0 | 0,0 | 0,1 | 1,1 |
| 2 | X,Z+16 | 2 | 0,1 | 1,1 | 1,0 | 0,0 |
| 3 | X+16,Z+16 | 3 | 1,1 | 1,0 | 0,0 | 0,1 |

This table, rather than a screen-space clockwise convention, defines orientation 1. Mirroring applies
**after** the selected orientation: replace every u with 1−u; leave v unchanged. The four resulting UV
pairs are also copied to the terrain-draw corner array. For orientation bytes greater than 3 the switch
writes no new UVs, but the optional mirror and corner-array copy still execute; repeated mirrored calls
can therefore toggle retained u values (`0x005DEEA0–0x005DF004`, jump table `0x005DF008`).

On a geometry update, flat placement requires a lot at x,z with flag bit 0 and the global bypass query
returning false. All y values then use the lot altitude. Otherwise terrain slot 17 fills four altitudes
which are copied in the slot order above (`0x005DFA18–0x005DFAB1`). Positions have no altitude offset.
Bounds are x,z inset by the binary32 constant `0x3DCCCCCD` (approximately 0.1 m). Their y range is
`[altitude−2, altitude+2]` for a flat cell and `[minimumCorner−2, maximumCorner+2]` otherwise. A final
ordered maximum≤minimum check repairs maximum to minimum+0.1 (`0x005DFB3B–0x005DFC92`).

A colour update runs only for flat placement, with normal (0,1,0), at each of the four stored positions,
through lighting slot 19. Each output integer's low byte is stored in the vertex colour; the byte at
colour offset +3 is the record's alpha byte. Output arguments 0,1,2 occupy colour bytes 2,1,0 respectively
(`0x005DFCAF–0x005DFD29`); there is no owner-side saturation. Geometry and colour updates are independent
flags. Geometry-only calls do not relight, and nonflat colour calls do nothing. Both services have
optional explicit arguments, with global terrain/lighting services used when null.

The first zoom change (cached zoom 255) requests geometry **and** colours; later zoom changes request
colours only. Every such change stores the low zoom byte and reloads the texture. A draw-time rotation
change requests colours only and caches the low rotation byte (`0x005DFD40–0x005DFD7A`,
`0x005E0230–0x005E0253`). `SetTexture` copies the entire 16-byte record, loads the texture and updates
UVs; it does not rebuild positions, bounds or colours (`0x005DF9A0–0x005DF9D4`).

### 4.2 Cell resource failures (LM5)

For nonzero key K, try `(0x7AB50E44, 0x0986135E, K+zoom)`, then the same type/group with instance K
if the first result is null. Addition is a 32-bit wraparound addition. The second request is made even
when zoom is zero and both instances are equal. The stored key is not rewritten. K=0 makes no lookup
and returns success with no binding. Two failed requests return false. On replacement the old binding
is released; if the returned pointer equals the old pointer, the extra acquired reference is released
instead. A null replacement clears the old binding and its alpha flag. A successful binding obtains
the alpha flag from the factory's metadata. The loader never retains an old successful binding to mask
a failed replacement (`0x005DF020–0x005DF0FB`). Draw with no binding returns 1 before state setup;
failed loads are cached by the zoom byte just like successful loads.

## 5. Users

The building developer creates the foundation and wall occupants (building-development.md) and the lot developer
the cell textures; the model maker inserts their models; the 3D renderer draws them in the static pass.

## 6. Quirks

1. Foundation bases move in whole 16 m steps (§2).
2. Walls overshoot the lot level and the terrain by 0.2 m (§3).
3. A cell texture on a slope is drawn by the terrain view, not by itself (§4).

## 7. Open questions

See [questions/lot-models.md](questions/lot-models.md) LM1: what the cell texture record's mode values mean
beyond 3 and who sets the colour.

## 8. Conformance tests

1. **Foundation steps:** top 40 m, lowest border ground 21 m: base at 40 − 32 = 8 m (two steps).
2. **Fallback:** a foundation at zoom 3 with a nonzero side key whose lookup fails retries `0x69F20003`;
   a zero side key makes no request.
3. **Wall texture:** a wall at zoom 4 with a failed nonzero key retries `0x695C00CA`; a zero key disables it.
4. **Facing:** at rotation 1 a wall on side 1 or side 0 is drawn, one on side 2 is not.
5. **Cell mode:** a record with alpha 128 and no mode draws with the alpha state.
6. **Orientation:** orientation 1 rotates the corners one step; the mirror flag swaps u for 1 − u.

The following are independent arithmetic/buffer vectors derived from the cited native operations;
they are expected results, not observations from a portable implementation or runtime verification.

| Vector | Inputs | Expected result |
|---|---|---|
| F-step-water | top 40, base 41, all border ground 21, water 25 | C=0, N=1; base 24; geometry rebuild, no lighting |
| F-step-existing | top 40, base 40, ground 21, water 25 | C=1, N=1; base stays 40; false unless forced |
| F-equality | top 40, base 40, ground 24, water below 24 | C=1, N=2; base 8, rather than 24 |
| F-fraction | top 40, base 24.5, ground 24.25, water lower | C=1, N=1; fractional base 24.5 retained |
| F-force | previous vector, force true | true, geometry rebuilt, same base and colours |
| F-UV-width | footprint 32×16, depth 16, scale 16 | every side ends at u=2, v=1; top U=2,V=1 |
| C-corners | cell (2,3), orientation 1, mirror true | positions (32,48),(48,48),(32,64),(48,64); UVs (1,1),(1,0),(0,1),(0,0) |
| C-bounds | same cell, nonflat altitude outputs 10,14,8,20 | x bounds approximately 32.1..47.9; z 48.1..63.9; y 6..22 |
| F-zero-key | zoom 3, side key 0, successful top | no side requests; top quad still eligible |
| W-repeat-failure | zoom 4, key already 0x695C00CA, both requests fail | two identical requests, stored key unchanged, binding null |
| C-wrap-key | key 0xFFFFFFFE, zoom 3; first request fails | requests instances 1 then 0xFFFFFFFE; key retained |
