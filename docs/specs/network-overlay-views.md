# Spec: network overlay views (`SC4PathInfoView`, `SC4TrainJunctionView`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [render3d.md](render3d.md) (§11 view objects), [traffic-network.md](traffic-network.md) (§7.3
the path info, network occupant slot 49 `GetPathInfo`), [automata.md](automata.md) (§9, the cheat handler and
U-Drive-It), [draw-context.md](draw-context.md), [overlay-views.md](overlay-views.md).

## 0. Summary

Two view objects draw over the network. The **path-info view**, a debugging aid that the automata manager's cheat
handler turns on, draws every network path of the visible cells as coloured lines with arrowheads. The
**train-junction view**, made by the U-Drive-It train control, shows at each rail junction ahead the routes the
player can choose, as arrow pictures, with the chosen one highlighted.

## 1. The path-info view (`SC4PathInfoView`) *[compat]*

- Made and initialised by the automata manager's cheat handler (`0x0050DD30`, automata.md §9). `Init`
  (`0x00542EE0`): untextured source-alpha blending; alpha test disabled (function/reference fields greater/0),
  without depth write; with the depth test less-or-equal unless the view was told to ignore depth.
- `Update(context)` (`0x00542FC0`), each draw:
  1. A process-wide pulse byte becomes 0 if its old value is 255, otherwise increases by 5 modulo 256.
  2. Over the network manager's visible cell rectangle, each network occupant on each cell gives its path info
     (occupant slot 49), and each of its paths is drawn when the view's path-type filter (`+0x18`, −1 = all) is −1
     or equals the path key's bits 16–23.
  3. A path whose key has any of bits 28–31 set is drawn as a red cross, spanning 4 m on each axis, at its first point.
  4. Otherwise its points (at most 25) form one line strip, each point 0.5 m above the path's height (or the
     ground under it when the path has no heights), in a colour (alpha 200) chosen by the key's bits 24–27: 1 light
     green, 2 blue, 4 yellow, 6 and 7 magenta, others light red; a path with flag bit 1 instead pulses from red to
     white.
  5. An arrowhead marks the full path's end, using a backward chord of at least 1 mm in 3D (§1.1).
  6. When the key's byte 1 is `0xFF`, a bar across the first segment marks the start; only otherwise, when
     its byte 0 is `0xFF`, the same marks the end.

### 1.1 Native geometry and colour corrections (NOV2)

The following rules supersede the earlier approximate colour/arrow descriptions. Native Update is
`0x00542FC0–0x00543B21`; the strip cap is at `0x005431A0`, full-path endpoint loads at
`0x00543231–0x00543273`, and squared-distance arithmetic includes **all three axes** at
`0x00543277–0x00543295`. Positions are passed directly in the path/occupant's world coordinate space;
there is no cell-origin addition. SetModelTransform uses the identity transform.

Vertices are format 1: three binary32 coordinates followed by four bytes **B,G,R,A** (16-byte stride).
The strip uses primitive 5, format 1. Its RGBA values are:

| Type nibble | RGBA bytes |
|---|---|
| 1 | (128,255,128,200) |
| 2 | (0,0,255,200) |
| 4 | (255,255,0,200) |
| 6 or 7 | (255,0,255,200) |
| Other | (255,128,128,200) |
| Path record flag bit 1, any type | (255,pulse,pulse,200) |

These are literal byte stores, not a floating colour conversion (`0x00543110–0x005431BC`). The pulse
advances even if no network manager exists and is shared across instances. Its update rule includes
255 for one draw, then resets to 0 on the next; the initialized-start value remains to be established
from static initialization, rather than inferred from the constructor.

For the strip only, take the first min(N,25) points. When path-info says it has heights, use stored y;
otherwise ask the occupant for altitude separately at each x,z. Add 0.5 m in both cases. The arrow
instead reads the **full vector's last point E** and initially its penultimate point Q. While |Q−E|₃D
< binary32(0.001), step Q backward without changing E. If a chord of at least that length exists,
normalize Q−E in 3D, rotate its x,z components by +/−binary32(π/4), multiply by 3, and emit the
three-point line strip `wing+, E, wing−`. All three y values are stored E.y+0.5, even when strip
altitudes came from the occupant. Consequently a sloping chord makes the projected wings shorter
than 3 m; a vertical chord makes both wings coincide with E in x,z. Equality at 1 mm is eligible.
The RGB bytes are copied into all three arrow vertices, but alpha is **not rewritten**: it retains
the corresponding strip scratch vertex's alpha. With fewer than three strip vertices the third
arrow alpha has no initialized-value guarantee. Evidence: `0x0054331E–0x005435FF`.

Endpoint bars also use the full vector and **immediate** first/last segments, normalized in 3D,
without a short-segment search or zero-length guard. Offset in x,z is `3 × (dz,−dx)/length3D` on
either side of that endpoint, and y is the endpoint's **stored** y, without +0.5 or terrain lookup.
RGB is red; alpha retains scratch contents. Byte 1 = FF draws the start bar; only **else**, byte 0
= FF draws the end bar: both FF produces just the start bar (`0x00543600–0x005438A8`).

The red cross is two separate two-point line strips, first x from P.x−2 to P.x+2, then z from
P.z−2 to P.z+2. RGBA is (255,0,0,255), not alpha 200. Each endpoint separately uses stored P.y
or an occupant altitude query at that endpoint, then +0.5 (`0x005438AD–0x00543A63`).

Traversal is x outer, z inner over inclusive visible limits, then linked occupant order, then the
path hash table's bucket order and linked chains. It is not sorted by key or by path type. An absent
manager or empty visible rectangle returns success; a filtered-out path emits nothing. Regular paths
with zero/one points reach unconditional endpoint loads before the later bar N≥2 test: they have no
defined safe empty-path behavior. Repeated endpoint pairs can cause division by zero in bars; there
is no explicit finite-input validation. Do not replace these boundaries with invented native rejects.

Independent discriminators: a 40-point path draws a 25-point main strip but locates its arrow at
point 40; E=(3,4,0), Q=(0,0,0) gives x,z wing offsets (−9√2/10,±9√2/10), at y=4.5;
a no-heights path can have its strip at occupant altitude+0.5 while its arrow remains at stored
E.y+0.5 and its bar at stored E.y. A key with both low bytes FF draws only the start bar. A cross
at (10,7,20), with heights, draws (8,7.5,20)→(12,7.5,20), then
(10,7.5,18)→(10,7.5,22), opaque red. Default/setter/state and degenerate-input residuals are
tracked in NOV2 until the remaining lifecycle evidence is reconciled.

### 1.2 State/default checklist (NOV2, partial)

Constructor `0x00542F50` sets initialized=false, ignoreDepth=false, packed state value/mask=0,
filter u32=0xFFFFFFFF. It does not reset the pulse. Init is idempotent and returns true, setting its
flag even without a render service. With that service its packed-state argument tuple (draw-context
§5.1 order) is **(1,0,!ignoreDepth,0,1,0,0,ignoreDepth?−1:3,4,5,4,0,1,−1)**
(`0x00542EF9–0x00542F41`). Alpha test and culling are off; depth write is off; source-alpha blend
is on. Changing a field after Init does not itself repack. Update ignores DrawPrims results and
acquires no resources. Destructor thunk 0x00542F90 forwards to deleting destructor 0x00542FA0:
shared cleanup 0x0046B4C0, then free only if deleting-flags bit 0 is set. Overlay-views §3.4 now
establishes the shared refcount transition and this cleanup dispatch. Concrete public filter/depth
setters and pulse static-initializer write remain residuals.
Primary interfaces and coverage limits are in overlay-views §3.2.

## 2. The train-junction view (`SC4TrainJunctionView`) *[compat]*

- Made by the U-Drive-It train control (`cSC4ViewInputControlPlayerDrive::Init`, `0x00565A10`). `Init`
  (`0x005633C0`): the draw state, a table of the unit directions between the four edge midpoints of a 16 m cell
  (west (0, 8), north (8, 0), east (16, 8), south (8, 16)), six arrow pictures (FSH group `0x1ABE787D`, instances
  `0x6BE09921`–`0x6BE09926`: three turns, each plain and selected) and two marker models (type
  `0x5AD0E817`, group `0xBADB57F1`, instances `0x8BB70000` and `0x8BB80000`).
- `Update(context)` (`0x005657A0`): over the visible cell rectangle, for each network occupant and each of its four
  entry edges, the network manager (slot 41) gives the routes leaving that edge and the one chosen; a junction with
  routes is drawn (`DrawTrainJunction`, `0x00564BE0`).
- **Drawing a junction:** selected-route/marker transparency is 0.8 at either stored train cell, else 0.5;
  unselected-route transparency is respectively 0.7 or 0.4 (§2.1).
  Each route but the chosen one gets its plain picture (`GetPathBitmap`, `0x00564A60`): with d the unit direction
  of the route's last segment (taken over its last three points, or the whole route when that is shorter than
  0.25 m) and e the entry direction, c = d.x e.z − d.z e.x chooses the picture pair `0x6BE09923`/`24` below −0.25,
  `0x6BE09925`/`26` up to 0.25 (no turn) and `0x6BE09921`/`22` above (plain, selected) *[inferred: the two turn
  directions]*. The picture is a quad at the entry edge, 12 m into the cell (`GetArrowBitmapQuad`, `0x005637E0`), at the
  ground height (3 m higher on elevated rail, track type 7), each one a little above the last (+0.01 per quad). The
  chosen route is drawn last in its selected picture, then the marker model is placed there 0.2 m up (3 m more on
  elevated rail) and drawn.

### 2.1 Exact junction draw contract and remaining lifecycle boundary (NOV3)

Constructor `0x005649D0` sets initialized=false, packed state zero, both stored cell pairs
(+0x1C/+0x20 and +0x24/+0x28) to (−1,−1), and both models null. It does not initialize the
stacking offset or the six texture-binding slots. Init is guarded by initialized, sets the flag
before resource loading, and returns true without aggregating resource failures. It recomputes the
process-wide 4×4 direction table: D(i,j)=normalize(edge[j]−edge[i]), with zero on its diagonal.
All y components are zero. The native packed-state argument tuple (draw-context §5.1 order) is
**(1,1,1,1,1,1,1,3,4,5,4,0,1,−1)** (`0x005633E3–0x00563402`): notably depth write **on**,
depth test ≤, alpha test greater than zero, one texture, source-alpha blending, culling on.

The six binding keys have type **0x7AB50E44**, group 0x1ABE787D, instances 0x6BE09921–26.
The two animated models retain the keys in §2. Model assignment retains a changed successful result
before releasing the old model; temporary model and service references are released at the end of
Init. A missing render service skips packing but still sets initialized and attempts resources. A
missing resource service is not guarded before its calls; no graceful false-return contract is proved.
Failed/null individual route/path/bitmap results are skipped in the drawing loops; null models skip
the marker. Init does not certify that a failing provider initializes every output slot.

Update first obtains the network manager: absent means true with no draw. Otherwise set the
stacking offset to **binary32 0.1 once per Update**, not once per junction. Iterate x outer, z inner,
inclusive visible limits, linked occupants, then entry bytes 0,1,2,3. Network-manager slot 41 gives
route count, key vector and chosen index. Only nonzero route counts call DrawTrainJunction. The
route vector is temporary and freed after the occupant. There is no frustum test or geometric cache
in this owner (`0x005657A0`).

DrawTrainJunction obtains world-space origin O from path-info slot 22 for the **first route**;
all route quads in this junction share O. There is no `(16×cellX,16×cellZ)` translation. For each
route key its entry is byte 1 and exit byte 0, selecting e=D(entry,exit). GetPathBitmap uses the
full path vector: Q=point[max(0,N−3)], E=last; if the **horizontal** chord length |E−Q| < .25,
replace Q by the first point. Normalize the resulting horizontal chord d and compare
`d.x×e.z−d.z×e.x`: below −.25 uses instances 23/24, [−.25,+.25] uses 25/26, above +.25 uses
21/22 (plain/selected). These are literal thresholds; no directional turn names are asserted.
Zero/empty paths are not protected by a public reject branch: N=0 reads before the vector and a
zero full chord divides by zero. There is no native finite-input validation (`0x00564A60`).

GetArrowBitmapQuad (`0x005637E0–0x00563968`) uses e, with perpendicular n=(−e.z,0,e.x).
Query terrain height at **O+12e**, take max(result,O.y), add current stacking offset, then add
3 if key bits 24–27 equal 7. Use this same height h on all four corners:

| Vertex order | x,z position | u,v |
|---|---|---|
| 0 | O+6n | 0,0 |
| 1 | O+6n+12e | 0,1 |
| 2 | O−6n+12e | 1,1 |
| 3 | O−6n | 1,0 |

This is a 12m-wide, 12m-long quad, format **2** (xyz,uv, 20 bytes), primitive **6**, count 4,
in that winding. The height query does not use the quad center, four independent corner heights, or
the route endpoint. Degenerate e=0 produces a collapsed quad; no substitute direction is chosen.

The draw resets transparency, sets identity model and its packed state, then draws unchosen routes
in vector index order, skipping the chosen **low-16-bit** index. Plain transparency is binary32 .7
if either stored cell pair matches, else .4. Then set transparency to .8 or .5 respectively and draw
the chosen index last. This latter indexing has no bounds check, even for a directly supplied empty
vector. Every successful bitmap quad uses the current offset, then increments it by binary32 .01;
null path/bitmap skips do not increment. Offset carries across junctions within the frame.
Evidence: `0x00564C19–0x00564C75`, `0x00564D20–0x00564E78`; constants are immediate .7/.8/.4/.5
bit patterns and .01 at `0x00A823A4`.

Marker uses model 0x8BB80000 **only** when cell matches pair +0x24/+0x28; otherwise 0x8BB70000,
including a match only to pair +0x1C/+0x20. It is temporarily retained during draw. Translation is
the shared origin, with y=O.y+currentOffset+binary32(.2), plus 3 for selected key type 7; unlike the
quad, it does not max with terrain height. Orientation is derived from e of the **first route**,
not necessarily the chosen route (`0x00564CA0–0x00564CD7`, `0x00564E78–0x00565085`). The exact
rotation-helper handedness/remaining model state and the public setters which populate the two
cell pairs remain unproved; this section does not certify those methods or train-current/target names.

Shutdown `0x00563730` is guarded by initialized; clear flag, release/null binding slots in order
+0x34,+0x38,+0x2C,+0x30,+0x3C,+0x40, then release/null both model references. It does not flush
draws or clear train cells. Repeated shutdown is a no-op. Overlay-views §3.4 establishes shared
refcount/deletion dispatch, including the deleting body's model cleanup **without** a Shutdown call.
Whether all callers ensure Shutdown before deletion remains unproved.

Independent discriminators: for O=(0,7,8), entry west/exit east, terrain at (12,8)=5,
offset=.1 and type not 7, quad xyz are (0,7.1,14),(12,7.1,14),(12,7.1,2),(0,7.1,2).
With no matched train cell, one plain route then selected route draw at .4 then .5 transparency,
at offsets .1 then binary32(.1+.01); the next junction starts at the offset left by those draws.
Matching only +0x1C/+0x20 gives .7/.8 but still model 0x8BB70000. A route whose last three points
have a vertical-only chord falls back to the horizontal whole-route chord, irrespective of its 3D length.

## 3. Users

The automata manager (cheat) and the U-Drive-It player train control.

## 4. Quirks

1. Only the main strip is capped to 25; arrows/bars use the full path (§1.1).
2. Junction arrows are stacked by small depth steps rather than sorted (§2).

## 5. Open questions

See [questions/network-overlay-views.md](questions/network-overlay-views.md): NOV1 retains the cheat
and path-key meanings. NOV2 retains pulse static initialization and public filter/depth setters;
NOV3 retains marker rotation/model-state details, cell setters/current-target identities and caller
Shutdown discipline. The geometry/state corrections in §§1.1–1.2 and 2.1 and shared lifecycle in
overlay-views §3.4 answer portions of these questions; NOV2 and NOV3 remain open.

## 6. Conformance tests

1. **Colour:** a path of type nibble 4 is drawn yellow with alpha 200.
2. **Limit:** a path of 40 points draws a strip of 25 points.
3. **Arrow choice:** entry direction e = (1, 0) and a route ending along d = (0, −1) give c = 1: the pair
   `0x6BE09921`/`0x6BE09922`; a route ending along e gives c = 0: the pair `0x6BE09925`/`0x6BE09926`.
4. **Transparency:** away from both stored cells, plain routes draw at .4, selected/marker at .5.
