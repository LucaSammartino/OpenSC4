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
  (`0x00542EE0`): an untextured blended state (source alpha, one minus source alpha, alpha test "greater" than 0),
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
  6. When the key's byte 1 is `0xFF`, a bar 6 m across, perpendicular to the first segment, marks the start; when
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

## 2. The train-junction view (`SC4TrainJunctionView`) *[compat]*

- Made by the U-Drive-It train control (`cSC4ViewInputControlPlayerDrive::Init`, `0x00565A10`). `Init`
  (`0x005633C0`): the draw state, a table of the unit directions between the four edge midpoints of a 16 m cell
  (west (0, 8), north (8, 0), east (16, 8), south (8, 16)), six arrow pictures (FSH group `0x1ABE787D`, instances
  `0x6BE09921`–`0x6BE09926`: three turns, each plain and selected) and two marker models (type
  `0x5AD0E817`, group `0xBADB57F1`, instances `0x8BB70000` and `0x8BB80000`).
- `Update(context)` (`0x005657A0`): over the visible cell rectangle, for each network occupant and each of its four
  entry edges, the network manager (slot 41) gives the routes leaving that edge and the one chosen; a junction with
  routes is drawn (`DrawTrainJunction`, `0x00564BE0`).
- **Drawing a junction:** its transparency is 0.8 at the junction the player's train is at or heads for, else 0.5.
  Each route but the chosen one gets its plain picture (`GetPathBitmap`, `0x00564A60`): with d the unit direction
  of the route's last segment (taken over its last three points, or the whole route when that is shorter than
  0.25 m) and e the entry direction, c = d.x e.z − d.z e.x chooses the picture pair `0x6BE09923`/`24` below −0.25,
  `0x6BE09925`/`26` up to 0.25 (no turn) and `0x6BE09921`/`22` above (plain, selected) *[inferred: the two turn
  directions]*. The picture is a quad at the entry edge, 12 m into the cell (`GetArrowBitmapQuad`, `0x005637E0`), at the
  ground height (3 m higher on elevated rail, track type 7), each one a little above the last (+0.01 per quad). The
  chosen route is drawn last in its selected picture, then the marker model is placed there 0.2 m up (3 m more on
  elevated rail) and drawn.

## 3. Users

The automata manager (cheat) and the U-Drive-It player train control.

## 4. Quirks

1. Only the first 25 points of a path are drawn by the path-info view (§1).
2. Junction arrows are stacked by small depth steps rather than sorted (§2).

## 5. Open questions

See [questions/network-overlay-views.md](questions/network-overlay-views.md) NOV1: which cheat turns the path-info
view on, and what the path key's bytes and type nibble denote in traffic-network.md's terms.

## 6. Conformance tests

1. **Colour:** a path of type nibble 4 is drawn yellow with alpha 200.
2. **Limit:** a path of 40 points draws a strip of 25 points.
3. **Arrow choice:** entry direction e = (1, 0) and a route ending along d = (0, −1) give c = 1: the pair
   `0x6BE09921`/`0x6BE09922`; a route ending along e gives c = 0: the pair `0x6BE09925`/`0x6BE09926`.
4. **Transparency:** a junction away from the player's train draws at 0.5.
