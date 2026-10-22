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
- **Height** (`AdjustHeightToTerrain`, `0x005D5A90`): along the footprint's border vertices, the largest drop from
  the top to the ground, or to the water surface where that is higher; when that needs more whole 16 m steps n than
  the box has *[inferred: rounded up]*, the base is lowered to top − 16 n and the box rebuilt (or rebuilt anyway
  when asked). The base never moves up here.
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

`DoZoomChange` stores the zoom byte before loading (`0x005D5EA8`); a failed load therefore does not
cause retries on every subsequent draw at the same ordinary zoom. Returning to that zoom after another
zoom retries its now-stored fallback key. Top drawing has its own binding/first-texture checks and is
reachable even if the side is absent or its first texture is zero (`0x005D6084–0x005D60F1`).

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
the flat draw submits these four records directly as primitive type 1, format 10 (`0x005E0522`).

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
colour offset +3 is the record's alpha byte. Output channels 0,1,2 occupy colour bytes 0,2,1 respectively
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
