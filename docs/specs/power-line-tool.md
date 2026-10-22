# Power-line tool: owner contract

Windows GOG 1.1.641, image base `0x00400000`. This supplements network-tools §9 and
network-geometry §11. Evidence addresses refer to that executable. Prose, tables and vectors are the public
contract; implementers must not consult the original program or its decompilation.

## 1. State and lifecycle

| State | Meaning |
|---|---|
| `+0x50` byte | real placement when nonzero; draft when zero |
| `+0x54`, `+0x60` | step ranges (inclusive indices into cells), cell pairs |
| `+0x70`, `+0x74`, `+0x78` | dragged, previously placed, valid step counts |
| `+0x118..+0x11C` | ordered input anchor cells; these are not the generated pole records |
| `+0x368` byte | power-line initialisation latch |
| `+0x36C..+0x370`, capacity `+0x374` | generated 48-byte pole records |
| `+0x378..+0x37C`, capacity `+0x380` | vectors of 16-byte wire-cell records |
| `+0x384`, `+0x388` | counted wire cells and counted poles for cost |
| `+0x38C` | draft newly created poles |
| `+0x390` | draft reused poles |
| `+0x394` | draft connection edits to undo |
| `+0x398..+0x39C`, capacity `+0x3A0` | groups of pole-record indices to consider for adjacent joins |

Construction (`0x006503C0`) constructs the base network tool with type **5**, installs the power-line
interfaces (primary vtable `0x00AA9F30`, secondary `0x00AA9F20`), clears the latch, vectors, counts,
and creates three empty list sentinels. Construction clears both cost parameters; it does **not** initialise
the spacing and clearance parameters until Init.

| Parameter | Type | Fresh-object default after Init | Property |
|---|---|---:|---|
| `+0x3A4` | uint32 | 20 | `0x098B25C8` |
| `+0x3A8` | float32 | 7.0 | `0x098B27D0` |
| `+0x3AC` | uint32 | 0 | `0x4A5F5953` |
| `+0x3B0` | float32 | 0.0 | `0x0A5F596B` |

Init, slot 3 (`0x00650630`), always returns AL=true. If already initialised it does nothing. Otherwise:

1. Once per process, set global latch `0x00B467A8` **before** StaticInit and ReadPowerPoleInfos. A missing
   resource does not cause a retry on the next object.
2. Set the object latch before calling base Init; the base return is not used to fail this Init.
3. Reset spacing to 20 and clearance to 7.0, then request exemplar TGI
   `6534284A/E7E2C2DB/C911E35B`, interface `0xA52160F5`. If obtained, read the four properties in table order
   and release the exemplar. A missing property leaves its destination unchanged. Cost fields are not reset
   on re-Init; their fresh-object zeros come from construction.
4. Clear pole and wire vectors. This is a structural clear, not the reference-release pass below.

The integer/float overloads and literal defaults are verified at `0x006506AA–0x00650704`.

Shutdown, slot 4 (`0x0064F0D0`), returns true immediately if the latch is clear. Otherwise clear the latch
**before** tail-calling base Shutdown, returning its result. Shutdown itself does not execute the preview undo.

ReleasePoleInfos (`0x006505D0`) visits every pole record in order, releases and nulls the two saved endpoint
references at record `+0x24`, `+0x28`, then structurally clears the records and their nested links. It does not
release a pole object at record `+0x10`; PlacePoles consumes that reference itself.

Slot 51 (`0x00650560`) is the deleting destructor wrapper: destruct, then free the object if argument bit 0
is set. The destructor (`0x00650580`) destroys the adjacency groups, connection-undo list, reused-pole list,
new-pole list, wire vectors, pole vectors, then base tool, in that order. It is not a substitute for slot 16's
world-side preview undo. Evidence: `0x00650580–0x006505CC`.

## 2. Placement wrapper and highlight

Slot 53 (`0x00652980`) resets wire/pole counts, releases old pole records, clears wire vectors and sets
valid := dragged. At two or more steps it runs DeterminePolePositions followed by PlacePoles(true).
An empty generated-pole vector forces valid := 0. The regional gate requires neighbour mode, valid at least
dragged, **at least two input anchors**, and failure=0. It calls Attempt in both draft and real modes;
PlaceNeighborConnections runs only if Attempt succeeds **and** placement is real (`0x00652A34–0x00652A46`).
Every draft path then invokes slot 82(0); real paths do not. Finally release pole infos and return true.

Slot 82, SetTerrainHilight (`0x0064F2B0`), is only a selection refresh:

- Walk steps from the supplied start index up to valid, and every inclusive cell range in those steps.
  A cell matching any generated pole record's x,z is skipped; every other cell gets an untextured selection
  with colour `0x800000FF` and the remaining three arguments zero.
- Walk all invalid steps from valid to dragged, irrespective of the supplied start index. Every cell gets
  colour `0x80FF0000`, including cells that match a pole record.
- The skip is a coordinate comparison, not a terrain/building/reuse check. There is no pole creation,
  resource lookup, cost change or valid-count mutation here. Evidence: `0x0064F2B0–0x0064F3E5`.

## 3. Failure text

GetFailureReasonString, slot 52 (`0x0064F570`), takes `(reason, outputString)` (two dwords, pop 8).
For reason != 0 it delegates to the base implementation. For reason=0 it examines only the **last input
anchor**. If there is an anchor and zone-manager slot 12 `(x,z)` returns a signed positive value, load
string group `0x2A592FD1`, instance `0x8AADD2B5` into the supplied output, final load argument 0.
Otherwise leave the output unchanged; it is not cleared and no base fallback occurs.
Evidence: `0x0064F581–0x0064F5CB`.

## 4. Discriminating vectors

These are host-observation vectors, not claimed in-game measurements.

| Vector | Inputs | Required result/order |
|---|---|---|
| PL-L1 | fresh object, exemplar unavailable; Init twice | type 5; defaults 20,7,0,0; resource attempt/base Init only on first Init; both return true |
| PL-L2 | successful Init; change cost fields; Shutdown; Init with missing properties | Shutdown clears latch before base; Init resets only spacing/clearance; costs retain prior values |
| PL-L3 | first resource lookup fails, second object initialises | process resource enumeration is not retried |
| PL-N1 | regional gate holds, Attempt=false, real=true | Attempt once; no PlaceNeighbour; no slot 82 |
| PL-N2 | regional gate holds, Attempt=true, real=false | Attempt once; no PlaceNeighbour; slot 82(0), release infos |
| PL-N3 | regional gate holds, Attempt=true, real=true | Attempt, PlaceNeighbour, release infos, in that order |
| PL-H1 | start=1, valid=2, dragged=3; a pole cell in each step | no selection in valid step 0; skip pole in valid step 1; invalid step 2 is entirely red |
| PL-F1 | reason=0, no anchors, output prefilled | output unchanged; no zone query or base fallback |
| PL-F2 | reason=0, two anchors; zone(last)=0 then 1 | last anchor only; unchanged for 0, special resource string for 1 |
| PL-F3 | reason != 0, zoned last anchor | base reason text; no special last-anchor query |

## 5. Evidence and remaining owner audit

The local attribution algorithm (`tools/coverage.py`, read-only Attribution.run) reports **46 entries,
16,156 function bytes** for this owner. The brief's 49-function count is not reproduced; byte count matches.
This is not a completed-owner claim. Geometry, connection mutation, zone update and supporting resource
functions are being reconciled against that inventory before the final report.

REA batch queries for `0x0064F3F0`, `0x0064F230`, `0x00650760`, `0x0064FBF0` returned
`target_unavailable` (no app open). The permitted fallback is the local export and existing class decompile.
