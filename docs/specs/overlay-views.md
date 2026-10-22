# Spec: overlay views (`cSC4PathView`, `SC4MarkedCellView`, `cSTEVertBufExtn`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [render3d.md](render3d.md) (§11 view objects: these views are registered there and drawn in
their layer), [draw-context.md](draw-context.md), [render-properties.md](render-properties.md),
[terrain-view.md](terrain-view.md) (§4, which uses the vertex-buffer extension), [view-utilities.md](view-utilities.md)
(frustum tests).

## 0. Summary

Two small view objects draw marks over the city: the **path view** draws routes as ribbons with arrowheads, and
the **marked-cell view** shades a set of cells, following the terrain, the water or the lots on them. The
**vertex-buffer extension** is the terrain view's optional path for submitting terrain rows through a driver
vertex buffer.

## 1. The path view (`cSC4PathView`) *[compat]*

- **State:** a list of paths ordered by priority, a thickness t (`SetThickness`, `0x008B6DE0`), the current path
  texture (`SetPathTexture`, `0x007DCE20`, through the texture binding factory), and two packed render states made
  by `Init` (`0x007DCD90`): plain and textured (one texture), both colour write with no depth write and no depth
  test, culling, blended (source alpha, one minus source alpha) unless boolean 33 `kPreferOpaque` is on.
- `NewPath(id, priority, flags)` (`0x007DDC50`): a new path, inserted before the first path of higher priority
  (equal priorities keep their order), with the current texture. Flag bit 0 draws an arrowhead at the start, bit 1
  at the end.
- `AddPathVertex(p)` (`0x007DDB30`): appends p to the newest path, unless it lies within √(10⁻⁵) m of the previous
  vertex in x and z.
- `ClearPaths` (`0x007DDBE0`) removes every path.
- `Update(context)` (`0x007DD9B0`, the view-object draw): with any path, depth offset 15, the identity model
  transform, then each path in order (`DrawPath`), then the depth offset is restored.

### 1.1 Drawing a path (`DrawPath`, `0x007DD410`)

A path of n ≥ 2 vertices becomes one triangle strip of 2n vertices (vertex format 10: position, colour, uv):

- **Offsets:** at the two ends, the left and right points are the vertex ± t times the unit normal of the end
  segment in x–z (`0x007DCE60`); at a joint, the offset follows the bisector of the two segments' normals
  (`0x007DCF20`), falling back to the next segment's normal when the bisector is shorter than 0.0001 (a path that
  doubles back).
- **Texture coordinates:** u is 0 on one side and 1 on the other; v is the distance along the path so far times
  r ÷ (2 t), where r is the path's repeat factor.
- **Arrowheads** (`DrawArrowhead`, `0x007DD0D0`): with the start or end flag, a single triangle (format 10) is drawn
  at that end along the end segment, sized from t, and the ribbon's end point moves back to the arrow's base; no
  arrow is drawn when the end segment is shorter than t.
- A path with a texture draws with the textured state, that texture and repeat wrapping; the plain state is
  restored after.

## 2. The marked-cell view (`SC4MarkedCellView`) *[compat]*

- **State:** the marked cells (x, z), a colour for all of them or one per cell, the cell size (w, h) and its
  inverse (`SetCellSize`, `0x007DC1D0`), and the switches `SetShouldIgnoreDepthBuffer` (`0x007DC1A0`) and
  `SetShouldUseWaterAltitude` (`0x007DC1B0`). `SetCellColor(r, g, b, a)` (`0x007DCB30`) packs the colour into bytes.
- `Init` (`0x007DC200`): the packed state, colour write without depth write, no texture, culling, blending source
  alpha / one minus source alpha, the depth test less-or-equal unless the depth buffer is ignored; and an occupant
  filter for lots.
- `Update(context)` (`0x007DC9F0`): with any cell, depth offset 4 and the identity model transform; the cells are
  drawn in their colours (`DrawCells`); then, with the outline colour set and `kPreferOpaque` off, they are drawn
  again with the depth test "greater" and additive blending (one, one) in that colour, so the parts hidden behind
  buildings show through brighter; the depth offset is restored.

### 2.1 Drawing cells (`DrawCells`, `0x007DC3A0`)

Cells outside the terrain are skipped. Each cell becomes a quad of two triangles in vertex format 1 (position,
colour), batched up to 1,024 vertices per indexed draw.

- **16 m cells** (w = h = 16): the corners are the terrain vertices of the cell, at the terrain altitude (or the
  water altitude where higher, with the water switch); the diagonal follows the terrain's split of that cell. A cell
  is skipped unless its first or third corner is in the view frustum. With lots considered, a cell under a lot that
  the filter accepts takes the lot's altitude at each corner, 15.5 m higher for lots of certain types (flags
  `0x304`), and is skipped for a lot with flag `0x100000` that does not show marks.
- **Other sizes:** the corners are the cell's grid corners times the cell size, clamped to the terrain, at the
  altitude of the terrain vertex nearest to each corner; the diagonal is always the same.

## 3. The terrain vertex-buffer extension (`cSTEVertBufExtn`) *[compat]*

A module-level helper of the terrain view (terrain-view.md §4):

- `GetVertexBufferExtension(driver)` (`0x0075CD40`): only with boolean 25 `kUseVertexBuffers`. The driver's
  interface `0x09CD86F9` makes a vertex buffer of format 11; when the made format is 11 and it holds at least two
  terrain rows (W + 1 vertices each, W the terrain's cell width), the index list is sized for as many rows as fit
  (at most the terrain's height) × W × 6, and the extension is used; otherwise it is released.
- `DrawPrimsIndexedUsingVertBuf(row, column, n)` (`0x0075CB50`): when the next two rows of n + 1 vertices do not
  fit, the buffer is flushed and locked again (`CreateBuffer` `0x0075C9F0`); the two rows' terrain vertices (32 bytes
  each) are copied in (`0x0075CA30`) and the n cells' six indices each are appended, rebased to the copied rows.
- `FlushBuffer` (`0x0075CAD0`): unlocks the buffer and draws its indices as triangles (the count rounded down to a
  multiple of 3). `SetIndexBuffer` (`0x0075CAB0`) sets the terrain's index source; `ReleaseVertexBufferExtension`
  (`0x0075CA90`) releases the buffer.

### 3.1 Exact batching and failure contract (OVV7)

This is a process-wide helper, not one independent buffer per view. Let W and H be terrain cell dimensions,
C the driver's vertex capacity, S = W + 1, V the used vertex count, and I the used index count.
`GetVertexBufferExtension` accepts C ≥ 2S and pre-sizes its **16-bit** index vector to
`min(floor(C/S), H) × W × 6`: there is **no subtraction of one vertex row**. This is storage sizing,
not a promise that that many independent cell rows fit. Format 11 is requested and checked. Acquisition
does not lock. Rejected format/capacity releases the queried interface; query failure returns null.
Evidence: `0x0075CD7F–0x0075CE53`, especially unsigned division at `0x0075CE12`.

`CreateBuffer` calls interface slot 6 with `(handle, C)` and keeps its returned pointer. There is no
separate lock-mode argument at this call site. Success sets V = I = 0 and returns true; null returns
false without resetting those counts (`0x0075C9F0–0x0075CA25`). The terrain callers explicitly call this
before submission (e.g. `0x007521CB`, `0x0075B822`); submission is not a lazy initial lock.

The submission arguments are `(row, firstColumn, endColumn)` with an **exclusive** end; n is their
difference. If V + 2(n+1) > C (equality fits), flush, then call slot 6 again. If the resulting/current
pointer is null, return without copying or drawing. Otherwise copy n+1 vertices from row and row+1,
each 32 bytes, into two consecutive compact rows. There is no row cache or overlap reuse. n = 0 still
copies two vertices and appends no indices. Negative ranges and requests larger than C are not
validated; there is no repeated capacity test after relocking (`0x0075CB78–0x0075CC0F`).

`SetIndexBuffer(pointer, byteFlag)` borrows the source; it does not AddRef or copy it. Source starts at
`pointer + 12 × firstColumn` bytes, plus `12 × W × row` when byteFlag is nonzero. Each of the 6n source
entries is an unsigned 16-bit value q **relative to a pair of full terrain rows**, not an absolute
terrain vertex number. With A and B the compact starts of the copied first and second rows, append
q + A − firstColumn when q ≤ W, otherwise q + B − S − firstColumn, storing the result modulo 65536.
No check tests whether q actually refers to one of the copied columns; hidden/degenerate indices
outside that range are neither repaired nor culled. Index-vector shortage grows it to I+6n+16 entries;
it does **not** flush (`0x0075CC14–0x0075CCD1`, word loads/stores `0x0075CCAA/0x0075CCC1`).

Flush with a null lock pointer does nothing. Otherwise call slot 8 `(handle)` to unlock, clear the
pointer, and, if signed I > 0, call slot 10 `(handle, 0, 3 × trunc(I/3), indices)`. Ignore both driver
return values, then set V = I = 0. Flush never relocks. Release calls slot 2 and clears only the
interface pointer: it does not unlock, flush pending indices, destroy the index vector, or clear the
stored lock pointer. Thus release while locked/pending is not an implicit successful submission;
subsequent helper calls require a valid reacquisition/caller sequence. No allocation-failure recovery
is present in the vector-growth path. Evidence: `0x0075CAD0–0x0075CB40`, `0x0075CA90`.

Independent discriminators derived from these instructions (no portable implementation consulted):

| Input/sequence | Native consequence |
|---|---|
| W=4, H=8, C=10 | Accept; initial index-vector length 48, not 24. No lock yet. |
| Lock; row=2, columns [1,3), source q=1,2,6,2,7,6 | Copies six vertices; compact indices 0,1,3,1,4,3. Row selection changes only source address when byteFlag is set. |
| Same range, source q=0 | Rebased first index is 65535; no validity repair. |
| V=4, C=10, n=2 | Equality fits: no flush. V becomes 10. |
| Empty range [2,2), lock valid | V increases by 2, I unchanged; later flush unlocks even if I=0. |
| Flush; small submission without CreateBuffer | Null lock pointer: no new copy/draw. |
| Grow index vector with room for vertices | Grow storage; no driver unlock/draw. |

## 4. Users

Tools and views create these view objects and register them with the renderer (render3d.md §11): routes and
dispatch paths use the path view, zoning, lot and query previews use the marked-cell view; the terrain view owns the
vertex-buffer extension.

## 5. Quirks

1. A path vertex closer than about 3 mm to the previous one is dropped (§1).
2. Marked cells under certain lots float 15.5 m above the lot (§2.1).
3. Hidden parts of marked cells are drawn a second time, additively (§2).

## 6. Open questions

See [questions/overlay-views.md](questions/overlay-views.md) OVV1: which lot types the flags `0x304` and
`0x100000` mean in §2.1, and where a path's repeat factor r comes from.

## 7. Conformance tests

1. **Order:** paths with priorities 2, 1, 2 created in that order draw as 1, 2 (first), 2 (second).
2. **Strip:** a straight path of 3 vertices draws one strip of 6 vertices, 2t wide.
3. **Duplicate:** a vertex 1 mm from the previous one is not added.
4. **Cells:** 100 cells of 16 m draw in one batch; 300 cells draw in two (1,200 vertices).
5. **Vertex buffers:** with `kUseVertexBuffers` off the extension is never made.
