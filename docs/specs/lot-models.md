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
rotation changes, takes its texture per zoom with a fallback key, and selects its documented draw state.

## 1. Common behaviour *[compat]*

- All three derive from `cS3DModelInstanceBasic` (as a virtual base; pool-allocated, freed to the pool by the
  deleting destructors). Cell and wall add IIDs `0xA9E2831D` (`0x005DEB00`) and `0xA9E2831E`
  (`0x005E1200`). Foundation delegates to the basic QueryInterface without adding an IID (`0x005D5410`).
- **Sort value** (`GetSortValue`): the current texture's ID OR a class constant, so models sharing a texture draw
  together; without a texture a fixed value. Foundation: `| 0x2AB6`, else `0xE9862AB6` (`0x005D5570`); wall:
  `| 0x2A86`, else `0xC9862A86` (`0x005E14F0`).
- **Zoom changes** load textures through the binding factory. A zero configured foundation/wall key disables
  that binding. A failed lookup of a nonzero key replaces that zoom's stored key with the fallback and retries
  once, including when that key already equals the fallback (§§2.1, 3.1).
- The draw state, made once from the 3D view manager's draw context, is colour and depth write, depth test
  less-or-equal, one texture, no alpha test, no blending, no culling.

### 1.1 Proven basic ownership and lifetime (LM2, partial)

The basic constructor starts the reference count at **zero**. It is an unsigned 16-bit field, not a
32-bit atomic counter. AddRef increments modulo 65536 and returns the zero-extended word. Release
decrements and returns the count only when it was greater than one; counts zero and one both invoke
deleting destruction and return zero (`0x00801900–0x00801932`, constructor `0x00801CF0`).

The basic QueryInterface recognizes IID 1 and `0x26A68449`; success returns the adjusted basic
interface and calls its virtual AddRef. Failure returns false without clearing the output pointer
(`0x00801940`). Cell/wall recognize their extra IID, return their owner interface and AddRef it;
otherwise they delegate to the basic operation. Foundation's `0x005D5410` is an adjustment/forwarder,
not a fourth IID implementation.

The owner pointer is initially null. SetOwner on a different pointer releases the old owner **before**
AddRef of a nonnull new owner; equal pointers cause neither call. It then stores the new pointer.
GetOwner returns the stored pointer without acquiring a reference. Custom data starts at zero and is
an uninterpreted stored word; it has no lifetime calls. The basic constructor also sets properties
to 2, two flag bytes to zero, and scale to binary32 1 (`0x00801850–0x008018AA`, `0x00801CF0`).

Foundation destruction releases/clears side binding, then top binding, then invokes the basic
destructor. Cell destruction releases/clears its binding, then basic destruction. Wall destruction
releases/clears its binding, frees its vertex allocation if nonnull, then invokes basic destruction.
The basic destructor finally releases/clears the owner pointer (`0x005D5300`, `0x005DEAB0`,
`0x005E1A60`, `0x00801810`). Resource binding references and the model's owner reference are separate.

### 1.2 Native entry inventory and limit of this audit (LM2, partial)

These are the **51 explicitly named owner entries** returned by export searches, including adjusting
thunks and deleting destructors. They are not 51 distinct behavior bodies or a reconciliation of the
brief's 82-function total. Addresses below are hexadecimal native entry points.

| Owner | Behavior bodies | Adapters / deleting entry |
|---|---|---|
| Foundation (13 entries) | destructor 005D5300; world bounds 005D5530; sort 005D5570; vertices 005D55C0; terrain adjustment 005D5A90; textures 005D5C70; constructor 005D5D70; zoom 005D5E90; draw 005D5FC0 | bounds 005D5D50; sort 005D5D60; deleting destructor 005D5F60; draw 005D6120 |
| Cell (17 entries) | destructor 005DEAB0; QI 005DEB00; world bounds 005DEE30; sort 005DEE70; corners 005DEEA0; texture 005DF020; Init 005DF100; constructor 005DF880; SetTexture 005DF9A0; vertices 005DF9E0; zoom 005DFD40; draw 005E0200 | QI 005DEC00; bounds 005DF1C0; sort 005DF1D0; deleting destructor 005E01A0; draw 005E0540 |
| Wall (21 entries) | QI 005E1200; neighbour flat 005E1240; bounds comparison 005E1400; world bounds 005E14C0; sort 005E14F0; box-to-edge 005E1520; textures 005E1620; zoom 005E1710; constructor 005E17C0; destructor 005E1A60; vertices 005E1BC0; Init 005E2780; draw 005E27C0 | QI 005E1940; AddRef 005E1950; Release 005E1960; bounds 005E1A40; sort 005E1A50; deleting destructor 005E1AD0; draw 005E28B0 |

The foundation model-instance table at `0x00AA2808` and wall table at `0x00AA2EA8` each expose 19
slots: QI, AddRef, Release, GetOwner, SetOwner, GetCustomData, SetCustomData, GetBounds,
GetWorldBounds, GetTransform, SetTransform, slot 11, Draw, GetProperties, SetProperties,
GetDrawAttributes, SetDrawAttributes, GetSortValue, GetResModel. Foundation basic adapters are
`0x005D5420–0x005D54F0`; wall equivalents span `0x005E1950–0x005E1A30`. These adapters are not
additional independent owner algorithms. Slot 11 forwards through `0x005E19E0` to `0x005DEB40`.
Its behavior, the full cell inherited table, complete constructor fields, allocation failure and all
remaining unnamed entries still require reconciliation; LM2 remains open. Do not turn this bounded
inventory into an 82-body completion claim.

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
- **Shape** (`SetupVertices`, `0x005E1BC0`): compare the lot altitude with consecutive terrain samples.
  Only segments with at least one terrain endpoint strictly above the lot altitude are eligible.
  The branch-specific triangles and reported bounds are specified in §3.2; the 0.001/0.002 constants
  change bounds, not vertex positions. A generic wall quad does not reproduce these branches.
- **Textures** (`SetupTextures`, `0x005E1620`; `DoZoomChange`, `0x005E1710`): the zoom's key from FSH group
  `0x891B0E1A`; a missing key falls back to `0x695C00C6 + zoom` — the five shipped wall textures `0x695C00C6`–
  `0x695C00CA` of the Terrain Properties exemplar's `0x295961F2` (terrain-view.md §1).
- **Draw** (`0x005E27C0`): not when highlighted. A zoom change reloads the texture; a rotation change recolours.
  Drawing requires `+0x6A` nonzero **and** side equal to rotation modulo 4 or rotation+3 modulo 4.
  A zero flag skips the draw; it does not disable the facing restriction. Submit one triangle list.
- `0x005E1240` tests the across-edge lot's flag bit 0. A true result suppresses construction of that
  segment (`0x005E1D8F–0x005E1D96`), rather than merely altering the draw state.

### 3.1 Binding failures (LM5)

`SetupTextures(previousZoom, newZoom)` compares the full arguments first. On a change it releases and
clears its old binding. A zero key makes no resource request. A nonzero key requests FSH type
`0x7AB50E44`, group `0x891B0E1A`, instance equal to the stored per-zoom key. If the result is null,
it overwrites that key with `0x695C00C6 + newZoom` and makes one more request, even if identical to
the first. A second null result stays null (`0x005E1620–0x005E16CD`). Draw skips a null binding
(`0x005E2851–0x005E2856`); it does not construct geometry through an external owner to recover it.

### 3.2 Finite wall range and ordered construction (LM4)

Evidence identity for this audit: the original Windows image SHA-256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`, identical to
`ghidra/export/windows/export-summary.txt` (image base `00400000`, 2,193,251 instructions).
The following rules were checked against its instruction bytes and constant data, not a portable mesh.
They cover valid sides 0–3, finite ordered samples, successful allocation and available services. Arithmetic
must remain finite; signed cell shifts must not overflow, each 16 m traversal step must remain representable,
and the emitting-segment count must stay below 256. Invalid/unordered/overflow cases remain LM8.

**Arithmetic boundary.** Constants e, d, D, r and b are binary32 words `3E4CCCCD`, `3A83126F`,
`3B03126F`, `3D800000`, `4B400000` (approximately 0.2, 0.001, 0.002, exactly 1/16 and
12582912). The range bias c is `3F666666` (approximately 0.9). Native arithmetic uses the externally
supplied x87 precision/rounding control; this owner does not set it. Evaluate each multiply/add/subtract
in its written order at that precision; each position, UV, sampled altitude and local T is stored to
binary32. Do not silently use floor or reassociate. The literal ledgers in §8.1 assume round-to-nearest,
ties-to-even and x87 precision 53 or 64 bits; they do not establish the process's control word.

For box `(xmin,ymin,zmin,xmax,ymax,zmax)`, define lower(q) by multiplying q by r, adding b,
storing binary32, interpreting the stored word as an integer and subtracting `4B400000` modulo 2^32.
Define upper(q) by multiplying by r, subtracting c, adding b, storing binary32, then subtracting
`4B400000` and adding 1 as integer operations. The helper evaluates zmax, xmax, zmin, xmin in that
order (`005E1527–005E159A`). Under the ledger controls, for nonnegative q no greater than 2^24 these
are nearby-integer(q/16) and nearby-integer(q/16−c)+1; neither is generally floor/ceil.

| Side | Start cell (a,b) | End cell (exclusive along the varying axis) | Inner lot cell | Across-edge cell for segment (i,j) | Traversal |
|---|---|---|---|---|---|
| 0 | lower(xmin),lower(zmin) | upper(xmax),b | a,b | i,j−1 | increasing x |
| 1 | upper(xmax),lower(zmin) | a,upper(zmax) | a−1,b | i,j | increasing z |
| 2 | lower(xmin),upper(zmax) | upper(xmax),b | a,b−1 | i,j | increasing x |
| 3 | lower(xmin),lower(zmin) | a,upper(zmax) | a,b | i−1,j | increasing z |

These are the actual side switch and lot-query offsets (`005E159F–005E1611`, `005E1BF0–005E1C2E`).
A missing inner lot resets the emitting count to zero but leaves geometry/bounds untouched and goes
to the optional recolour phase. Otherwise obtain L from lot slot 16, store binary32, then sample terrain
slot 14 at (a,b), even if the range is empty. Initial world x,z are signed `(cell << 4)` converted to
binary32, not coordinates displaced off the edge (`005E1C45–005E1C82`).

At each increasing segment sample the next terrain endpoint first, storing H1; H0 is the preceding
sample (including skipped segments). Stop when the varying cell index reaches the exclusive end.
The test order is H0>L, then if false H1>L. If both fail, advance without a neighbour query.
Only an eligible segment calls the across-edge flat test. That test asks terrain slots 7 then 8 for the
cell extents, adjusts the cell as in the table, checks signed bounds, queries lot slot 6 with final
argument 0, and returns flag bit 0 from lot slot 12. Missing/out-of-range/nonflat means false; true
skips construction. There is no neighbour-altitude comparison (`005E1240–005E12B3`,
`005E1D20–005E1D96`). Sampling and traversal still advance on either skip.

For an eligible, non-suppressed segment, store T=L−e. Let A(y) be the current endpoint position with
height y and B(y) the next endpoint, exactly 16 m farther along the varying axis. Append records in
the following order; each consecutive triple is one triangle, with no index buffer or vertex sharing.

| Endpoint tests (after eligibility) | Ordered records | Triangles | Bounds y update from initial [L,L] |
|---|---|---|---|
| H0>T and H1>T | A(T), B(T), A(H0+e); A(H0+e), B(T), B(H1+e) | 2; diagonal A(H0+e) to B(T) | maximum with raw H0, then raw H1; minimum unchanged |
| H0>T and H1≤T | A(T), B(H1−e), A(H0+e) | 1 | maximum with raw H0; minimum with raw H1 |
| H0≤T and H1>T | A(H0−e), B(T), B(H1+e) | 1 | minimum with raw H0; maximum with raw H1 |
| H0≤T and H1≤T | none | 0 | unchanged (unreachable after finite eligibility) |

Evidence: tests `005E1D9C–005E1DDB`; six-record path `005E1E05–005E2030`; descending
path `005E2105–005E2207`; ascending path `005E22FA–005E2410`. Equality with **L** is not the
split test: H0=L,H1>L still uses six records. Equality with **T** selects the ≤ branch. A segment
entirely at or below L emits nothing, even if one endpoint is above T. Crossing is not interpolated.
All three emitted patterns have geometric cross-product normal +z on sides 0/2 and −x on sides 1/3
when the positive-height spans survive rounding. Ordering does not reverse on opposite sides.
The lighting normal is separately side-dependent below. There is no area/duplicate-vertex rejection:
rounded zero spans, or T=H+e after rounding, can produce degenerate triangles without reducing count.

Each record is 24 bytes: x,y,z; four colour bytes; u,v. For sides 0/2, u=stored x×scale; for 1/3,
u=stored z×scale. In every case v=stored y×scale, stored binary32 after multiplication.
The origin is world coordinate zero, not the box or segment origin; there is no fractional wrap or
per-segment reset (`005E1E2B–005E1E59` and corresponding stores on the other two paths).

**Reported bounds are not the seam-vertex extrema.** Initialize varying-axis minimum/maximum to
largest finite binary32 / its negative, perpendicular minimum to stored start coordinate+d and
maximum to start coordinate+D, and y to [L,L] (`005E1CA4–005E1D08`). Each emitting branch updates
the varying minimum with the current coordinate and maximum with current+16; skipped segments update
neither. Y updates use the raw samples in the table, without ±e (`005E2033–005E20D2`,
`005E220A–005E22A9`, `005E2413–005E2477`). Thus the box can exclude seam vertices.
After at least one emitting segment, add d to the varying minimum and subtract d from its maximum;
leave the perpendicular offsets and y unchanged. Compare previous bounds to this candidate, component
order xmax,ymax,zmax,xmin,ymin,zmin; return-change is true if any absolute difference exceeds binary32
`38D1B717` (approximately 0.0001), then replace the bounds regardless (`005E1400–005E148C`,
`005E24B5–005E25B5`). Output geometry-present is true. The local bounds become [0,0,0] to
stored(max−min); world bounds retain the candidate. Subtractions use the stored
candidate endpoints in native order.

If no segment emits, including an empty or reversed range with an inner lot, resample terrain at
(a,b) and replace reported world/local bounds with `(16a,Hstart,16b)` to that point+(1,1,1), output
geometry-present false. The return-change flag stays false; the candidate comparison is not called
(`005E25BD–005E2642`). This does not apply to the earlier missing-inner-lot exit.

The byte count at +66 counts emitting **segments**, not triangles. The builder only grows the stored
record sequence when too short; it does not shorten it after a smaller rebuild. Consequently the table
defines the newly written prefix. On a fresh empty buffer it defines the entire sequence; on reuse an
old tail survives. Draw submits the full stored record count, not three/six times the segment byte
(`005E1DD2–005E1E00`, `005E1B30–005E1BBF`, `005E287A–005E289E`). No-emission does not clear it.

If recolouring is requested and the stored sequence is nonempty, visit **all stored records**, including
any retained tail, in record order. Request lighting slot 19 at each position with the negative of the
edge vector: sides 0..3 use (0,0,+1), (−1,0,0), (0,0,−1), (+1,0,0). Output integer arguments 0,1,2
contribute their low bytes to record offsets 14,13,12; offset 15 is always 255. There is no clamp.
Geometry-only calls leave existing colour bytes; grown records start zeroed. Recolour-only calls leave
positions/UV/bounds alone. These are explicit terrain/lot/lighting inputs, not a host-provided final mesh
(`007F6D10–007F6D5D` with table `007F6D60`, `005E2646–005E275C`).

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
  record's mode is not 255 (including explicit 0), else **alpha** when the texture has alpha or the alpha is not 255, else **opaque**; alpha and
  tinted cells are skipped while the draw context's `+0x04` is 0 *[inferred: drawn in the translucent pass]*. Wrap
  clamp. Tinted: linear filtering, the environment colour := the record's RGB × global-light component 0 × binary32(1/255) and
  alpha ÷ 255, the combiner with the record's mode, where mode 3 with `kNoAddSigned` becomes mode 1 at half the
  alpha. On a flat lot the model draws its own four-vertex strip; on a slope it asks the terrain view to draw the
  cell (`DrawTexturedTerrainQuad`, terrain-view.md §4.4) with its corners and alpha, forced when the draw context's
  `+0x20` is 8.

### 4.1 Corner, placement and update contract (LM6)

The stored cell x and z are **unsigned bytes**, not signed world coordinates (`0x005DFA0C`,
`0x005DFA10`). Let X = 16x and Z = 16z. Vertices are 24-byte position/colour/UV records in this order;
the flat draw submits these four records directly as primitive type 1, format 10 (`0x005E0424–0x005E0433`).

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

### 4.3 Owned cell draw state and ordered calls (LM7)

This is the owner-selected request and its typed driver boundary, not a claim that every device applies
every requested combine operation. The initialized-state domain requires `Init` to run with the render
service present; otherwise it returns before packing the three states (`005DF100–005DF110`). Context
zoom/rotation comparisons use full integers against unsigned cached bytes; normal vectors use zoom 0–4,
rotation 0–3. Do not extend the finite wall domain to other numeric or service-failure cases.

**Control and return sequence** (`005E0200–005E0495`):

1. Highlighted draw returns 1 immediately. No reload, recolour, state setup or geometry call.
2. If zoom differs, perform §4.1's zoom update and texture load; cache the low zoom byte before
   the load, even on failure (`005DFD40–005DFD7A`). If rotation differs, request colours only,
   then cache its low byte. These updates precede both binding and pass checks.
3. Null binding returns 1. Otherwise classify: record mode≠255 → tinted; mode=255 and
   (binding alpha flag nonzero or record alpha<255) → alpha; otherwise opaque.
4. Alpha/tinted with context +04=0 returns 2. Opaque proceeds regardless. Context +20=8 is
   not inspected yet and cannot bypass return 2 (`005E0273–005E02DC`).
5. Call context `SetModelTransform`, `ResetShade`, then `SetTexture(selected,0)`. Select binding
   element at unsigned cached zoom if less than its element count, otherwise element 0.
   A valid binding with no elements is outside this contract; there is no owner check for that case.
6. Opaque/alpha: apply the corresponding packed state below, then request wrap (2,2,0).
   Tinted: apply tinted state, request wrap (2,2,0), request filters (1,1,0), ask context +44's
   slot 10 for three global-light floats, calculate the colour and combiner below, then call
   `SetTexColor`, `SetTexCombiner(record,0)` in that order.
7. Flat placement: `DrawPrims(1,10,4,storedRecords)`; Draw itself does not rewrite vertex alpha.
   The flat recolour routine writes the original record alpha to all four records (`005DFCBD`,
   `005DFD29`), including when mode 3 falls back. Nonflat: terrain-view slot 15 gets context,
   unsigned cell x,z, stored corner array, original alpha×k, and boolean(context +20=8).
   Return 1 on either path. The owner does not propagate driver/delegated return values.

The three initialized packed states consist of two 32-bit words, **value, mask**, in increasing address
order. These are runtime results of `PackRenderState`, not the executable's pre-init data bytes.
The table follows the actual argument order verified at `005DF113–005DF186` and bit packing
`007D2B50–007D2D15`; compare/blend meanings use [graphics-formats.md §3.8](graphics-formats.md#38-mats-compat)
and the public D3D7 mappings already recorded in [renderer-driver.md §5.1](renderer-driver.md#51-comparison-blend-stencil-and-shading-enumerations).

| State/address | Value | Mask | Explicit fields |
|---|---|---|---|
| opaque / 00B23090 | 020001A7 | 1E0003FF | colour/depth writes, depth test, stage0 texture on; alpha test/blend/cull/stage1 texture off; depth func 3; stage0 env 1 |
| alpha / 00B23098 | 020A91BF | 1FFFFFFF | colour/depth writes, depth/alpha tests, blend, stage0 texture on; cull/stage1 texture off; depth func 3; blend 4,5; alpha func 4, ref 0; stage0 env 1 |
| tinted / 00B230A0 | 000A80B5 | 101FE3FF | colour write, depth test, blend, stage0 texture on; depth write/alpha test/cull/stage1 texture off; depth func 1; blend 4,5 |

Fields absent from the mask are retained: opaque retains blend factors, alpha func/reference and stage1
env; alpha retains stage1 env; tinted retains alpha func/reference and both env fields. Thus tinted is
not a complete replacement word. Bits: colour/depth writes 0/1; depth/alpha tests 2/3; blending 4;
stage0 texture 5; cull 6; depth func 7–9; alpha func 10–12; source/destination blend 13–16/17–20;
alpha reference 21–24; stage0 env 25–27; stage1 texture 28; stage1 env 29–31.
Depth 3 is less-or-equal, 1 less; alpha 4 greater; blend 4,5 is source-alpha/one-minus-source-alpha.

**Context effects and driver-call order.** For these packed states, take changed bits under the mask
against the current context word. With any change, store `(old & ~mask) | value` and select driver
stage 0 (slot 51), then issue only changed groups in this exact order: colour mask (15), depth mask
(17), depth capability 1 (enable/disable 44/45), alpha capability 0, blend capability 4, texture
capability 5, cull capability 3, depth func (16), alpha func/reference (22), blend factors (21), stage0
env (28); finally stage1 enable/env if changed and permitted, selecting 1 then restoring 0.
For all three owned tuples stage1 texture is explicitly off; its env is retained. Alpha/blend force
switches can replace an enabled driver capability with disabled while the requested bits remain cached;
stage1 calls require `kUseSecondStage`. Even if **no packed bit changes**, request wrap (3,3,0),
filter (1,7,0), clear transform stage1 then stage0, in that order (`007D4530–007D47D7`).
Then the owner's step 6 overrides the wrap to (2,2,0), and tinted overrides filters to (1,1,0).
Wrap/filter setters suppress unchanged requests and apply the render-property overrides in
[draw-context.md §§4,7](draw-context.md#7-texture-stages-compat); this is a request sequence, not an
unconditional list of device calls. SetTexture reaches driver slot 58 for stage 0 even if equal;
ResetShade only reaches slot 32 if its override flag was set. Model transform behaviour remains the
existing context contract, not another lot-owned transform algorithm.

**Tint arithmetic and combiner.** Let k be binary32 `3B808081`, not an exact division by 255.
Store all three returned light floats, but use only component 0, G, for **every** RGB channel.
Unsigned R,Grecord,B are converted to x87, each multiplied by G, then k, then stored binary32.
Alpha is converted unsigned and multiplied by k, then stored binary32. No saturation or integer
colour packing occurs in this owner (`005E0389–005E03E2`). If record mode=3 and the byte at
`context[+38][+0C]+3BC` is nonzero, set effective mode=1 and shift the unsigned alpha right once
**before** conversion; otherwise keep both. No other mode is changed. Vertex/delegated alpha remains
the original byte, irrespective of this environment-alpha change.

After completed Init the shared 12-byte combiner is
`00 00 02 21 01 00 03 01 00 00 00 00`. Init writes bytes 0–6 (`005DF18B–005DF1B5`);
byte 7=01 and bytes 8–11=00 come from image data `00B0A87F–00B0A883`. Each tinted draw writes
only byte 0 to effective mode. `SetTexColor` reaches driver `TexEnv(0,1,colour)` slot 27;
`SetTexCombiner` reaches `SetCombiner(record,0)` slot 62 unconditionally, then sets context stage0
env bits to 4 and copies all 12 bytes into its stage record (`007D3030–007D30C3`).

The small driver decoder proves this owned tuple directly (`00884A70–00884B91`): it flushes queued
primitives, sets the selected stage's env=4 and dirty flags, and, unless byte0=255, decodes RGB op from
byte0, alpha op from byte4; RGB sources from the low nibbles of bytes1,2,3; alpha sources from 5,6,7.
Their high nibbles select operand tables (2,3,4,5) and (4,5,4,5), respectively. Bytes8,9 are scales.
Bytes10,11 are context-copy padding, not additional operations. For this owner the decoded tuple is:

| Fields | Exact values | Typed meaning at the already documented driver boundary |
|---|---|---|
| RGB op | effective mode 0,1,2,3 | select arg1, modulate, add, add-signed, respectively (scale 0) |
| alpha op | 1 | modulate (scale 0) |
| RGB source triple | 0,2,1 | texture, texture factor, current |
| RGB operand triple | 2,2,4 | first two colour; third alpha replicate |
| alpha source triple | 0,3,1 | texture, diffuse, current |
| alpha operand triple | 4,4,4 | alpha, without complement |
| RGB/alpha scale | 0,0 | base operation |

Semantic names above follow the existing public driver contract
[renderer-driver.md §10.3](renderer-driver.md#103-applying-a-stage-at-draw-time-0x00884320-0x00884500),
not a guessed mapping from the record's mode labels. In particular mode0 selects texture colour despite
entering the tinted state/setup path. Driver applied-state caching and device capability handling belong
to that foreign boundary; these owner traces do not establish final pixels or a full renderer contract.
Other record mode bytes remain raw operations; their intended meaning/upstream author remains LM1.

## 5. Users

The building developer creates the foundation and wall occupants (building-development.md) and the lot developer
the cell textures; the model maker inserts their models; the 3D renderer draws them in the static pass.

## 6. Quirks

1. Foundation bases move in whole 16 m steps (§2).
2. Walls overshoot the lot level and the terrain by 0.2 m (§3).
3. A cell texture on a slope is drawn by the terrain view, not by itself (§4).

## 7. Open questions

See [questions/lot-models.md](questions/lot-models.md): LM1 (upstream authors and other mode bytes),
LM2 (remaining owner/inherited inventory and lifecycle), LM8 (exceptional/invalid numeric domains).

## 8. Conformance tests

1. **Foundation steps:** top 40 m, lowest border ground 21 m: base at 40 − 32 = 8 m (two steps).
2. **Fallback:** a foundation at zoom 3 with a nonzero side key whose lookup fails retries `0x69F20003`;
   a zero side key makes no request.
3. **Wall texture:** a wall at zoom 4 with a failed nonzero key retries `0x695C00CA`; a zero key disables it.
4. **Facing:** at rotation 1 a wall on side 1 or side 0 is drawn, one on side 2 is not.
5. **Cell mode:** a record with alpha 128 and mode 255 draws with the alpha state; explicit mode 0 selects tinted.
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

### 8.1 Independent finite wall ledgers (proposed, UNEXECUTED natively)

These literal inputs/expected records are analytic consequences of the primary instructions in §3.2.
No engine output, native execution, runtime geometry comparison or default-path promotion is claimed.
Exact rational arithmetic with binary32 storage was used to check the hexadecimal literals; it does not
provide a replacement wall builder. PC=53 or 64, RC=nearest-even; all input terrain/lot heights below
are raw service results stored binary32, and raw +80 texture scale is 1/16. Use a fresh empty buffer,
geometry+colour enabled, successful allocation, valid terrain extents and a nonflat/missing across lot
unless the row says otherwise. Previous bounds for changed-return examples are six zero floats.

The common box has x=32..48, z=80..96, and lot L=10 at inner cell (2,5). The following table is a
literal position/UV dictionary: A(y), B(y) mean the positions shown, **not** a host-supplied final mesh.
All four sides must be crossed with every height row below. This gives ordered records on each side
without depending on a screen-space side convention.

| Side | Terrain request sequence | A(y) | B(y) | u(A),u(B) | Across query | Lighting normal |
|---|---|---|---|---|---|---|
| 0 | (2,5), then (3,5) | (32,y,80) | (48,y,80) | 2,3 | (2,4) | (0,0,1) |
| 1 | (3,5), then (3,6) | (48,y,80) | (48,y,96) | 5,6 | (3,5) | (−1,0,0) |
| 2 | (2,6), then (3,6) | (32,y,96) | (48,y,96) | 2,3 | (2,6) | (0,0,−1) |
| 3 | (2,5), then (2,6) | (32,y,80) | (32,y,96) | 5,6 | (1,5) | (1,0,0) |

Let t=`411CCCCD`, p=`41433333`, q=`41633333`, m=`40F9999A`, n=`41233333`,
w=`4119999A` denote stored y values 9.8,12.2,14.2,7.8,10.2,9.6 approximately, respectively.
The corresponding **exact v words** at scale 1/16 are `3F1CCCCD`, `3F433333`, `3F633333`,
`3EF9999A`, `3F233333`, `3F19999A`. For any row A/B determines x,z,u by the preceding table;
y determines v by this dictionary. Repeated labels are repeated records. Semicolons delimit triangles.

| Raw H0,H1 | Eligibility/split | Ordered complete fresh-buffer records | Triangle count | Reported y bounds |
|---|---|---|---|---|
| 12,14 | both above L | A(t),B(t),A(p); A(p),B(t),B(q) | 2 | 10..14 |
| 10,12 | first equal L | A(t),B(t),A(n); A(n),B(t),B(p) | 2 | 10..12 |
| 12,10 | second equal L | A(t),B(t),A(p); A(p),B(t),B(n) | 2 | 10..12 |
| 8,12 | ascending through L and T | A(m),B(t),B(p) | 1 | 8..12 |
| 12,8 | descending through L and T | A(t),B(m),A(p) | 1 | 8..12 |
| t,12 | first equal T | A(w),B(t),B(p) | 1 | t..12 |
| 12,t | second equal T | A(t),B(w),A(p) | 1 | t..12 |
| 8,10 | below/equal L | none, no neighbour query | 0 | fallback 8..9 |
| 10,8 | equal/below L | none, no neighbour query | 0 | fallback 10..11 |
| 10,10 | both equal L | none, no neighbour query | 0 | fallback 10..11 |
| 8,9 | both below L | none, no neighbour query | 0 | fallback 8..9 |

For example side1, H0=8,H1=12 gives exactly `(48,m,80;5,3EF9999A)`,
`(48,t,96;6,3F1CCCCD)`, `(48,p,96;6,3F433333)` in x,y,z;u,v notation.
Side2, H0=12,H1=8 gives `(32,t,96;2,3F1CCCCD)`, `(48,m,96;3,3EF9999A)`,
`(32,p,96;2,3F433333)`. Side0/3 use the same explicitly tabulated A/B expansion; their ordering
does not reverse to match their opposite side's lighting normal.

For every emitting row, reported horizontal bounds in binary32 words are:

| Side | xmin,xmax | zmin,zmax |
|---|---|---|
| 0 | 42000106,423FFEFA | 42A00083,42A00106 |
| 1 | 42400106,4240020C | 42A00083,42BFFF7D |
| 2 | 42000106,423FFEFA | 42C00083,42C00106 |
| 3 | 42000106,4200020C | 42A00083,42BFFF7D |

Use raw y bounds from the height table, not m/p/t extrema. Output geometry-present=true and
changed-return=true for these initial bounds. For a nonemitting row the builder resamples the first
listed terrain cell: prescribe that result equal H0. Bounds then run from the listed A(H0) to
A(H0)+(1,1,1), geometry-present=false, changed-return=false, with no lighting requests on a fresh buffer.

Raw lighting inputs are intentionally asymmetric. For side s, give each distinct position in its ordered
records a first-encounter ordinal j=0,1,2,… and prescribe output integers
`(257+16s+3j, 514+16s+3j, 771+16s+3j)`. Repeated identical positions return the same tuple.
Expected colour bytes in memory are `(3+16s+3j, 2+16s+3j, 1+16s+3j, 255)`.
Thus side0's six-record above/above row has colours `03 02 01 FF`, `06 05 04 FF`,
`09 08 07 FF`, `09 08 07 FF`, `06 05 04 FF`, `0C 0B 0A FF`, in order; sides1/2/3 add
0x10/0x20/0x30 to each RGB byte. The low-byte rule deliberately distinguishes raw service outputs from
already packed colours and checks that duplicates are lit as separate requests.

Additional literal control cases:

| Case | Inputs | Expected ordered effects |
|---|---|---|
| W-empty | box x=32..32,z=80..96, side0; L=10,Hstart=12 | start=end x cell2; request terrain (2,5), then fallback resample (2,5); no neighbour query/records; bounds (32,12,80)..(33,13,81), output false, return false |
| W-narrow | x=32..34, same other inputs | same empty range/effects as W-empty despite positive box width |
| W-tie | lower x input 8, then 24 | lower cell 0, then 2 (nearest-even); floor would give 0,1 |
| W-flat-neighbour | side0 H0=12,H1=14; across (2,4) exists with flag bit0 set | sample both endpoints, extent queries then across lot; no append; fallback resample at (2,5), reported unit box, no fresh-buffer lighting |
| W-reuse | first side0 above/above row, then rebuild same range H0=12,H1=8 | prefix becomes A(t),B(m),A(p); records3..5 remain A(p),B(t),B(q); stored length six; recolour visits all six, segment byte is 1 |

### 8.2 Independent cell draw traces (proposed, UNEXECUTED natively)

These are owner request traces, not final-state/pixel observations. Unless overridden: not highlighted,
matching zoom/rotation caches, valid nonempty binding, alpha flag false, flat placement, pass +04=1,
context +20=0, RGB=(64,128,192), alpha=255, global-light raw output=(1,2,3), completed Init,
no force properties. Let P mean `SetModelTransform`, `ResetShade`, `SetTexture(element0,0)` in order.
Let S0/S1/S2 be the opaque/alpha/tinted packed applications from §4.3 including their unconditional
wrap/filter/transform-reset requests. Let W request wrap(2,2,0), F filters(1,1,0), L request light slot10,
C set environment colour, K set combiner stage0, and D the flat `(1,10,4)` draw. These abbreviations
expand to the exact calls and bits in §4.3, including conditional driver-call suppression.

| Inputs/counterexample | Exact owner trace/result |
|---|---|
| mode255, alpha255, no binding alpha | P,S0,W,D; return1; packed 020001A7/1E0003FF; no L,C,K |
| mode255, alpha128 | P,S1,W,D; return1; packed 020A91BF/1FFFFFFF; no L,C,K |
| mode0, alpha255 | P,S2,W,F,L,C,K,D; return1; K bytes `00 00 02 21 01 00 03 01 00 00 00 00`; explicit zero does not choose opaque |
| mode1, alpha255 | same tinted trace; K byte0=01, other bytes unchanged |
| mode2, alpha255 | same tinted trace; K byte0=02, other bytes unchanged |
| mode3, capability byte0 | same tinted trace; K byte0=03; environment alpha word 3F800000 |
| mode3, capability byte1 | same tinted trace; K byte0=01, alpha255 shifted to127; environment alpha word 3EFEFF00, not the word for 0.5; flat vertex alpha remains FF after recolour |
| mode0, pass0, context +20=8 | return2 before P,S2,W,F,L,C,K,D; force cannot bypass pass filtering |
| mode255, alpha255, pass0 | P,S0,W,D; return1; opaque ignores pass0 |
| mode255, binding alpha flag1, pass0 | return2; metadata flag changes classification even with alpha255 |
| mode3 fallback, nonflat, context +20=8 | P,S2,W,F,L,C,K; terrain call original alpha word3F800000 and force=true; return1 |
| highlighted, changed zoom | return1 without even zoom update or resource lookup |
| cached zoom0→1, key100, both loads fail, pass0 | recolour-only update, cache1, requests101 then100, release old binding/clear alpha flag; rotation update if needed; null-binding return1 before pass filtering |
| repeat preceding failed draw at zoom1 | no reload or resource requests; null-binding return1 |
| old cached zoom255→1, binding load succeeds, mode0/pass0 | geometry+colour update, cache1, load; rotation recolour if needed; then return2; no render-state setup |

For all tinted rows above, C's RGB words are **3E808081,3F008081,3F40C0C2**, despite returned light
components1,2 being 2,3. Normal alpha255 produces 3F800000. This distinguishes G0-for-all from
component-wise lighting and multiplication by stored k from a mathematical division by 255.
The initialized alpha op stays 01 when byte0 varies through 00,01,02,03,01.

For a literal driver sequence, start the context word at `020A91BF` with no overrides and apply S2:
new word `020A90B5`, changed bits `0000010A`; select stage0, depth-mask(false),
disable alpha capability0, depth-func(1), then the unconditional wrap/filter/clear requests.
The alpha-function bits remain 4 and reference0 despite alpha testing being off; blend(4,5) is retained.
After W,F,C,K the stage0 env field becomes4, so the requested context word is `080A90B5`.
This is a trace of supplied literals at the driver boundary, not a native execution or host-computed oracle.
