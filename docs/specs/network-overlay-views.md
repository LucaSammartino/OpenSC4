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
255 for one draw, then resets to 0 on the next. The initial byte at **0x00B228C8 is zero**:
it lies in the loader-zero-filled part of the executable's `.data` section (VA 0x00B07000,
raw size 0x1A000, virtual size 0xB40D8; this byte is at section offset 0x1B8C8).
The complete exported instruction scan has four direct accesses: the compare/reset/increment at
0x00542FC6/FD7/FE0 and the colour load at 0x00543114; no separate scalar static initializer writes it.
Thus in a fresh process Update number n uses **5n modulo 260**, for n=1,2,… (values 5,…,255,0,…).
This counts Updates across all instances, including Updates that find no network manager, rather
than frames or elapsed time. Construction, Init and Shutdown do not restart the sequence.

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
(10,7.5,18)→(10,7.5,22), opaque red. Degenerate-input boundaries are stated above; they are
not successful-input guarantees for invalid vectors or uninitialized scratch alpha.

### 1.2 State, setters and lifetime (NOV2)

Constructor `0x00542F50` sets initialized=false, ignoreDepth=false, packed state value/mask=0,
filter u32=0xFFFFFFFF. It does not reset the pulse. Init is idempotent and returns true, setting its
flag even without a render service. With that service its packed-state argument tuple (draw-context
§5.1 order) is **(1,0,!ignoreDepth,0,1,0,0,ignoreDepth?−1:3,4,5,4,0,1,−1)**
(`0x00542EF9–0x00542F41`). Alpha test and culling are off; depth write is off; source-alpha blend
is on. Changing a field after Init does not itself repack. Update ignores DrawPrims results and
acquires no resources. Destructor thunk 0x00542F90 forwards to deleting destructor 0x00542FA0:
shared cleanup 0x0046B4C0, then free only if deleting-flags bit 0 is set. Overlay-views §3.4 now
establishes the shared refcount transition and this cleanup dispatch.

The concrete depth setter is the **same folded body 0x007DC1A0** named for SC4MarkedCellView:
one boolean byte argument, callee removes 4 stack bytes, store at primary-view +0x0D, no return-value
contract and no repacking. The automata caller demonstrably invokes it on a path-info view
**after Init** (0x0050DDA3–B0). Consequently that caller changes the stored byte to true while
retaining the depth-tested packed state made by Init. Repeating Init while initialized is true
does not change that state. To repack through the native sequence, Shutdown then Init.

The filter is a **32-bit value at +0x18**, compared in full against the zero-extended key byte
(bits 16–23); only 0xFFFFFFFF bypasses the comparison. There is no separate out-of-line filter
setter or virtual setter slot in the recovered interface. The actual caller stores the 32-bit
result of decimal string conversion directly at +0x18 (0x0050DDF7–0x0050DE03), or writes
0xFFFFFFFF when no filter argument is supplied (0x0050DE19–22). It neither truncates the value to
a byte nor invalidates any cache. These stores publish the effective inline mutation contract,
without inventing a native method address or recoverable C++ method name.

Shutdown uses the shared **0x0053C940** body: clear initialized at +0x0C if set; leave filter,
ignoreDepth, packed state and pulse alone. The actual removal caller unregisters the view through
manager slot 33, calls this body, calls primary Release slot 2, then nulls its global pointer
(0x0050DE47–6C). Allocation is 0x1C bytes; construction starts refcount zero and the creator
acquires a reference before Init/registration (0x0050DD78–0x0050DDC7). A null allocation is not
handled as a successful empty view: the subsequent AddRef dereferences it. Bounds and transforms
have no public slots; Update obtains visible bounds from the network service and sets identity
model transform. Primary/secondary interface slots and shared lifetime are in overlay-views §§3.2–3.4.

**Worked state case (UNEXECUTED):** on a fresh process, construct A and Init with the render
service present. Its packed state uses depth test ≤. Invoke the folded depth setter(true), then
Init again: the byte is true but the packed state still uses ≤. Shutdown/Init now packs ignored
depth. Set filter=256: no byte-valued path type matches; set filter=0xFFFFFFFF: all match. Let A
Update once with no network manager: pulse becomes 5 without drawing. Construct/Init B and draw
a flagged three-point path in B's first Update: its strip RGBA is **(255,10,10,200)**, BGRA bytes
**0A 0A FF C8**. Destroying A does not reset that shared pulse. This case distinguishes a late
setter repack, byte-truncated filter, per-instance pulse and draw-only pulse advancement.

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

### 2.1 Exact junction draw contract (NOV3)

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
vector. Every route with a nonnull path and bitmap uses the current offset, then increments it by
binary32 .01 before submission, irrespective of the draw result; null path/bitmap skips do not
increment. Offset carries across junctions within the frame. Obtaining the first-route origin is
earlier and has no null-path guard (0x00564C75–94); this skip rule does not make a null first route safe.
Evidence: `0x00564C19–0x00564C75`, `0x00564D20–0x00564E78`; constants are immediate .7/.8/.4/.5
bit patterns and .01 at `0x00A823A4`.

Marker uses model 0x8BB80000 **only** when cell matches pair +0x24/+0x28; otherwise 0x8BB70000,
including a match only to pair +0x1C/+0x20. It is temporarily retained during draw. Translation is
the shared origin, with y=O.y+currentOffset+binary32(.2), plus 3 for selected key type 7; unlike the
quad, it does not max with terrain height. Orientation is derived from e of the **first route**,
not necessarily the chosen route (`0x00564CA0–0x00564CD7`, `0x00564E78–0x00565085`). Exact
orientation, state submission and the identities of the two cells are specified in §2.2.

Shutdown `0x00563730` is guarded by initialized; clear flag, release/null binding slots in order
+0x34,+0x38,+0x2C,+0x30,+0x3C,+0x40, then release/null both model references. It does not flush
draws or clear train cells. Repeated shutdown is a no-op. Overlay-views §3.4 establishes shared
refcount/deletion dispatch, including the deleting body's model cleanup **without** a Shutdown call.
The sole exported direct Shutdown caller is the player-drive control: unregister through manager
slot 33, call Shutdown, clear its stored pointer, then primary Release slot 2
(0x00563B8A–0x00563BC0). Thus the observed owning caller does follow Shutdown discipline;
Release itself still does not substitute for Shutdown. The creator constructs 0x4C bytes, retains
a changed nonnull view before releasing the previous view, calls Init and registers it through
manager slot 32 (0x00565D2E–0x00565D98). A failed view allocation is not a successful no-view
initialization path: the following Init requires a nonnull object.

Independent discriminators: for O=(0,7,8), entry west/exit east, terrain at (12,8)=5,
offset=.1 and type not 7, quad xyz are (0,7.1,14),(12,7.1,14),(12,7.1,2),(0,7.1,2).
With no matched train cell, one plain route then selected route draw at .4 then .5 transparency,
at offsets .1 then binary32(.1+.01); the next junction starts at the offset left by those draws.
Matching only +0x1C/+0x20 gives .7/.8 but still model 0x8BB70000. A route whose last three points
have a vertical-only chord falls back to the horizontal whole-route chord, irrespective of its 3D length.

### 2.2 Cell mutations and marker basis/state (NOV3)

The two cell pairs are **interaction highlights**, not a train's current and target coordinates.
They are signed 32-bit x,z with (−1,−1) meaning unset. The recovered setters are inline stores,
not additional callable or virtual methods:

| Primary-view fields | Producer and mutation rule | Drawing meaning |
|---|---|---|
| +0x1C,+0x20 | Player-drive OnMouseMove 0x005631C0: pick the world point under the mouse through view slot 36, convert its x,z through city slot 95, store the resulting cell only when changed; a failed pick stores (−1,−1). Stores: 0x0056321F–54. | Hover cell: .7 plain/.8 chosen transparency, ordinary marker. |
| +0x24,+0x28 | OnMouseDownL 0x005655D0: when network picking succeeds and service slot 42 accepts that occupant, obtain its cell through occupant slot 59 and store only when changed (0x0056568E–B5). OnMouseUpL 0x00564300 clears both to −1 (0x00564303–1F). | Pressed/accepted junction: .7/.8 and marker 0x8BB80000. Takes precedence when both pairs match. |

No world-to-cell conversion occurs in DrawTrainJunction. It compares its integer cell arguments
with these pairs. These stores do not retain an occupant, acquire a model, repack draw state or
rebuild routes. There is no recovered out-of-line cell setter, public bounds/transform setter or
getter in either interface table; Update requests the current visible rectangle from network
slot 17. The inline operation's source-level C++ name/signature is not recoverable from these stores.

The rotation helper **0x0080B830** receives the first route's e directly and writes nine binary32
values (0x00564F04–0x00564FAB). Let h=√(e.x²+e.z²). For h>0 its three columns are
**X=(e.z/h,0,−e.x/h)**, **Y=(−e.x·e.y/h,h,−e.z·e.y/h)**, **Z=e**, with intermediate float stores
before the cross-product that forms Y (0x0080B830–0x0080B8E4). For this owner's horizontal unit
directions, Y=(0,1,0): positive local z follows e, positive local x follows (e.z,0,−e.x).
For e=(1,0,0), this is a quarter-turn with local x toward negative world z, not positive world z.
The helper uses the constants float 0 at 0x00A81054 and float 1 at 0x00B0EC64. When h=0
(also the unordered comparison branch), it writes X=(0,0,1), Y=(1,0,0), Z=e
(0x0080B8E5–0x0080B906); a diagonal direction-table entry therefore gives a singular basis,
not identity and not a substitute heading. No NaN repair contract follows.

Marker state is a temporary **0x38-byte** record passed to the model's adjusted secondary
interface slot 10 (+0x28). These are the bytes/fields this caller initializes, relative to that record:

| Offset | Value |
|---|---|
| +0,+1 | Byte values 2 and 5; bytes +2,+3 are padding not explicitly written. |
| +4…+0x24 | Nine binary32 matrix values, grouped by consecutive rows: (X.x,Y.x,Z.x), (X.y,Y.y,Z.y), (X.z,Y.z,Z.z). |
| +0x28,+0x2C,+0x30 | Translation x,y,z from §2.1, using the offset left after the chosen quad, even if that quad was skipped. |
| +0x34 | Binary32 1 (unit scale). |

Submission order is temporary model AddRef through adjusted slot 1; secondary slot 10 with that
state; primary slot 18 (+0x48) with integer 1 and binary32 chosen transparency (.8 or .5);
secondary slot 12 (+0x30) with (context,0); secondary Release slot 2
(0x00564E90–0x00565082). Results are ignored; no reset of the shared draw-context transparency
or restoration of the model's previous state occurs here. Model animation/material behavior beyond
these submissions belongs to the model service, not an additional train-view state field.

**Worked interaction/marker case (UNEXECUTED):** start with both pairs unset, cell C=(3,4),
first-route e=(1,0,0), origin (10,7,20), and one successful selected non-elevated quad. Its offset
is .1, leaving binary32(.1+.01) for the marker; marker y is approximately **7.31**, independent
of a higher terrain query used for the quad. The row-grouped rotation is
**(0,0,1), (0,1,0), (−1,0,0)**. Before interaction use .5 and model 0x8BB70000. Move the mouse
over C: only +0x1C/+0x20 change, transparency becomes .8, model remains 0x8BB70000. Accept a
left press on C: +0x24/+0x28 become C and model changes to 0x8BB80000. Mouse-up resets the
pressed pair only; a continuing hover still gives .8 and model 0x8BB70000. No train motion is
needed to produce these changes. Shutdown/release should release each of the six bindings once
and leave neither model retained; Release without Shutdown does not release those six bindings.

## 3. Users

The automata manager (cheat) and the U-Drive-It player train control.

## 4. Quirks

1. Only the main strip is capped to 25; arrows/bars use the full path (§1.1).
2. Junction arrows are stacked by small depth steps rather than sorted (§2).

## 5. Open questions

See [questions/network-overlay-views.md](questions/network-overlay-views.md): NOV1 retains the cheat
and path-key meanings. NOV2 is answered by §§1.1–1.2, including loader-zero pulse and folded/inline setters;
NOV3 is answered by §§2.1–2.2: hover/pressed cells, exact marker basis/state and observed owning
caller Shutdown discipline. Overlay-views §3.4 supplies shared lifetime. All worked cases remain
static expectations, not runtime verification.

## 6. Conformance expectations (UNEXECUTED)

1. **Colour:** a path of type nibble 4 is drawn yellow with alpha 200.
2. **Limit:** a path of 40 points draws a strip of 25 points.
3. **Arrow choice:** entry direction e = (1, 0) and a route ending along d = (0, −1) give c = 1: the pair
   `0x6BE09921`/`0x6BE09922`; a route ending along e gives c = 0: the pair `0x6BE09925`/`0x6BE09926`.
4. **Transparency:** away from both stored cells, plain routes draw at .4, selected/marker at .5.

### 6.1 Distinguishing literal vectors (static; UNEXECUTED)

These are independently derived primary-source expectations from §§1.1–1.2 and 2.1, not executed
results. Coordinates are (x,y,z); decimal heights are geometric approximations to native float stores.
Resource/path providers are assumed successful unless a row says otherwise. NO-L1 supplies its own
pulse precondition; §1.2 also supplies a fresh-process pulse case. Marker rotation and cell identities
are qualified in §2.1.

| ID / input | Expected observation | Distinguishes |
|---|---|---|
| NO-L1: type nibble 1, ordinary path with at least three points | Every strip vertex BGRA bytes 80 FF 80 C8. Type nibble 3 instead gives 80 80 FF C8. With flag bit1 and pulse pre-update=250, the next Update uses RGBA (255,255,255,200); following Update uses (255,0,0,200). | Approximate green/red, channel swap, clamping the pulse at 255 or resetting it per instance. 0x00542FC6–FE7, 0x00543110–BC. The supplied pulse is a precondition, not a certified initial value. |
| NO-L2: three-point path (0,0,0),(0,0,0),(3,4,0), no heights; occupant altitude constantly 20 | Strip y=20.5. Arrow center (3,4.5,0), wing x≈1.727208, z≈±1.272792, y=4.5; alpha 200 for all three. | x-z normalization (which gives projected wing offsets ≈2.12132), occupant-height arrow, duplicate removal. 0x00543231–0x005435FF. |
| NO-L3: key low word=0xFFFF; full vector (0,0,0),(3,4,0),(6,8,0) | Only start bar: (0,0,−1.8)→(0,0,1.8), at stored y=0, red RGB. No end bar. | Both-bar emission, +.5 on bars, horizontal-only length (which gives half-width3). 0x00543600–0x005438A8. |
| NO-L4: 40 points Pi=(i,0,0), i=0..39, with heights | Main strip ends at x=24 and has 25 vertices; arrow center is (39,.5,0), wings x≈36.87868,z≈±2.12132. | Arrow derived from truncated strip. 0x005431A0, 0x00543231 onward. |
| NO-L5: cross at (10,7,20), heights present | Two opaque-red line strips, (8,7.5,20)→(12,7.5,20), then (10,7.5,18)→(10,7.5,22). | ±4 half-width, alpha200, swapped axes/order. 0x005438AD–0x00543A63. |
| NO-L6: e=(1,0,0); route (0,0,0),(0,0,10),(10,0,−10),(10,0,0) | Last-three-point chord starts at index1, d=(1/√2,0,−1/√2), classifier c=+1/√2: bitmap pair21/22. | Last segment gives pair23/24; full-route chord gives25/26. 0x00564A60–0x00564BDF. |
| NO-L7: e=(1,0,0); route (0,0,0),(10,0,10),(10,100,10),(10,200,10) | Last-three horizontal chord is zero, so use full-route chord: c=−1/√2, pair23/24. | A 3D-length threshold would retain the vertical chord and misclassify it. 0x00564ADF–0x00564B25. |
| NO-L8: O=(10,7,20), e=(1,0,0), offset=.1, terrain query at (22,20) returns9, key type7 | Quad order (10,12.1,26),(22,12.1,26),(22,12.1,14),(10,12.1,14); UV=(0,0),(0,1),(1,1),(1,0); primitive6/format2/count4. | Cell-origin translation, center/corner sampling, wrong width/winding, failure to add3. 0x005637E0–0x00563968. |
| NO-L9: one Update, junction J1 routes indices0,1,2 chosen2; index1 path is null; first route/origin and remaining bindings valid; J2 has one valid selected route | Draw J1 index0 plain at offset .1, then index2 selected at float(.1f+.01f); J2 selected at float(float(.1f+.01f)+.01f). Null index1 consumes no increment. Away from both stored pairs, transparencies .4,.5,.5. With J1 matching only pair +0x1C/+0x20, its values become .7,.8 but marker still uses model0x8BB70000. | Per-junction offset reset, increment on skip, selected-first order, matching-cell model conflation. 0x00564C19–0x00564E78, Update0x005657A0. |

The marker's translation/model-selection portions can be checked separately from the basis/state
case in §2.2. Successful bitmap lookup consumes an offset increment even when the eventual
draw-context submission fails; no driver-result rollback is established.
