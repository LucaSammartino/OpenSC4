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

## 4. Pole records and geometry proved here

The record builder (`0x00650760`) appends a pole and, when its `linkPrevious` argument is true and the
new index is nonzero, adds a reciprocal link to the preceding pole using the supplied wire-vector index.
The record is not a persistent occupant record:

| Offset | Payload |
|---|---|
| `+0x00` | index in this pole vector |
| `+0x04`, `+0x08` | x,z cell coordinates |
| `+0x0C` | index into the dragged-cell array |
| `+0x10` | pole interface, initially null, consumed by PlacePoles |
| `+0x14` byte | created rather than reused |
| `+0x15` byte | direction-bit mask accumulated by links |
| `+0x18..+0x20` | vector of pairs (other pole index, wire-vector index) |
| `+0x24`, `+0x28` | saved old connection's endpoint references |
| `+0x2C`, `+0x2D` bytes | saved per-endpoint connection state |

A wire-cell record has x,z at +0,+4, a line interface at +8, and a pivot flag byte at +12; stride 16.
The reciprocal-link helper (`0x0064FB40`) appends one pair at each endpoint, then ORs **the same**
direction bit into both masks, using direction helper `0x0061F4C0`. It does not reverse the bit at the
second endpoint. Link index -1 denotes an adjacent join without a wire-cell vector.

### 4.1 DeterminePolePositions (`0x00650840`)

The following covers the ordinary drag and its terrain test. The old-wire interception branch and the
full adjacency-group construction remain NT18; callers must not substitute a simple fixed-spacing walk
for that branch.

Reset wire count; release old pole infos; clear wire vectors. Traverse every inclusive cell range of every
valid step in order. Track the next input-anchor index, the last clear cell/step, the last emitted-pole
step, whether a span is active, and whether an anchor or break has requested a pole.

For each step, choose its **pivot** with `0x0064F230`: before the previously placed-step count (when
that count is nonzero), use the step's first cell. Otherwise scan its cells in ascending index order
and take the first satisfying `abs(anchor.x-cell.x) == abs(anchor.z-cell.z)` for the next input anchor;
return -1 if none. This is a diagonal-distance equality, not a direction-change test.

The first cell of the previously placed step that shares **either** x **or** z with the first input
anchor is a separate forced point when `0 < previouslyPlaced < valid`. Reaching it requests a pole once.
Reaching the next anchor, or the final cell of the final valid step while an anchor remains, advances the
anchor index and requests a pole. At a pivot, an active span also requests a pole when the unsigned
step difference from the last emitted pole is **at least** `+0x3A4` (`0x00650A43–0x00650A62`). Thus
`+0x3A4` measures steps, not metres or Euclidean length.

Every cell is classified directly by CellIsOKForPlacement. The ordinary clear-cell branch:

1. Clear failure and remember this clear cell and step.
2. Probe for an existing pole. An existing pole at a pivot requests emission; that probe reference is
   released. Pole reuse is decided again by CreatePowerPole, not by transferring this probe reference.
3. At a pivot with no pending emission, with an earlier pole and a following dragged cell, test terrain
   clearance to that **following** cell. Let A be the last emitted pole position with `+0x3A8` added to y;
   compute B similarly at the following cell. Ray origin is A, direction is normalised B-A, two final
   terrain-intersection arguments zero. Emit at the **current** cell if the ray hits at distance <= |B-A|.
   A farther hit, or no hit, does not force a pole. This is why clearance is not a slope or pole-spacing limit.
4. When requested, emit a pole record, copy the current accumulated wire cells into a new wire vector,
   clear the accumulator, reset saved endpoints/state, set span active, remember this step, and update A.
5. If this cell was not emitted and it is before the final cell, append it to the wire accumulator and
   increment `+0x384`; the pivot flag records whether this was the chosen pivot.

Numeric order in the clearance test (`0x00650CE8–0x00650DF7`): use x87 arithmetic; store dy and dz to
float32, accumulate squared length as dz²+dy²+dx², store its float32 copy but retain the x87 sum for
the first sqrt; store reciprocal length to float32; store all three normalised components to float32.
The hit distance uses z²+y²+x² and is compared inclusively with sqrt of the stored squared length.
Constant `0x00B0C264` is float32 1.0. Do not replace the segment-bound check with an infinite-ray hit.

At completion valid becomes last-clear-step+1 (the last-clear-step local starts at 0), and pole count
becomes generated-record count. An empty result is subsequently forced to valid=0 by slot 53.
The wire counter is incremented during accumulation; it is not recomputed from the final vector sizes.

**Zoning and blocked cells:** a zone result breaks an active span and requests a restart; it can remove
an isolated generated pole or close the span at the last clear cell. Building/road handling depends on
span state and whether this is a forced point. These branches are not the slot-81 table: Determine
calls the classifier itself, and writes building/road failure codes internally. NT18 retains the exact
break/old-wire cases pending proof. There is no call of slot 81 in the direct Determine/PlacePoles chain.

#### Existing-wire interception: eligibility and reference transfer

On a clear cell with no existing pole, probe for a wire occupant (`0x00650ACE–0x00650AE6`). A hit
returns its two endpoint references through wire slot 3. Form the saved descriptor from those endpoints
in returned order with **both state bytes set to 1**. Read each endpoint's cell through pole slot 8.
The endpoint coordinates are signed integers for the following test; each nonzero coordinate difference
is reduced by signed division by its absolute value (`0x00650B53–0x00650BBB`).

Reject interception if the current step is at or beyond the previously placed-step count (`+0x74`),
or if **both** reduced coordinate differences are nonzero. Thus horizontal and vertical spans are
eligible; diagonal spans are rejected regardless of slope or length. Coincident endpoints pass this
part of the gate. Rejection releases both endpoint references, nulls both descriptor pointers and resets
both descriptor state bytes to 1. It does not itself clear an already pending pole request.

Otherwise set the pending pole request, then test, in short-circuit order:

1. An existing generated pole record has the exact same ordered endpoint pointers and both state bytes.
2. The first endpoint shares **either** x **or** z with any generated pole record.
3. The second endpoint shares **either** x **or** z with any generated pole record.

If any predicate succeeds and the current cell is **not forced**, clear the pending request and reject
the descriptor as above. A forced cell bypasses this duplicate rejection, but never bypasses the
previously-placed-step or diagonal gates. Here forced means the flag set for the special previous-run
point, next input anchor/final-cell anchor advance, or spacing trigger; it is separate from the pending
request retained across cells. If no duplicate predicate succeeds, or the cell is forced, retain the
two endpoint references in the descriptor for the emitted record. Release the probed wire reference
in all hit cases, after the endpoint decision (`0x00650BCC–0x00650C35`). The later pivot test still
controls whether this cell emits; accepting a descriptor is not itself an unconditional emission.

Evidence: branch predicates `0x00650BBF–0x00650BCA`, pending/forced handling
`0x00650BD3–0x00650C01`, release/reset `0x00650C06–0x00650C2E`; descriptor equality helper
`0x0064F170–0x0064F1C5`, coordinate-OR helper `0x0064F1D0–0x0064F22E`. This bounded contract
does not settle the later break/adjacency/repair branches still tracked by NT18.

### 4.2 ComputePolePosition, including NO24

Arguments are signed integer cell x,z and output position; no boolean result. Let s be terrain slot 6's
cell width. Store `x = r24((cellX+0.5)*s)`, `z = r24((cellZ+0.5)*s)`. If terrain slot 34 `(cellX,cellZ)`
is true, y is r24(terrain slot 30 at those stored x,z + 0.5). Otherwise request four corner floats from
terrain slot 17. Start the running height at **-FLT_MAX**; visit corners in returned order and replace
the height only on a strictly greater, ordered value. Store y = r24(height+0.5). NaN corners are ignored.
All corners equal to -FLT_MAX therefore return y=-FLT_MAX; no failure, no skipped pole, no prior y reuse.
Evidence: sentinel initialisation `0x0064F00B`, comparisons `0x0064F061–0x0064F0B1`, store `0x0064F0BB`;
sentinel bytes at `0x00AA9F0C` are `0xFF7FFFFF`.

## 5. PlacePoles and reversible draft state

PlacePoles (`0x00651560`) takes one boolean, **allow repair of displaced connections**. It is not the
real/draft selector; that selector is always the tool's `+0x50` byte. The real-only recursive repair
tests the argument at `0x00651F9D–0x00651FA6`; the recursive PlacePoles call passes false.

Proved order and mode differences:

1. Consider pairs of generated pole indices within each adjacency group. Manhattan cell distance exactly
   1 adds a reciprocal link with wire-vector index -1. This precedes occupant creation.
2. Clear those temporary adjacency groups; call CreatePowerPole for **all** pole records before processing
   the generated links. CreatePowerPole reuses the pole in that cell or creates and initialises one
   (network-occupants §4.5).
3. Process saved old endpoints before generated links. Saved endpoint state can queue a displaced pair
   for repair. Draft mode disables saved connections rather than removing them; real mode removes them.
   The draft undo list keeps the two endpoint references and saved flags until slot 16.
4. For each generated link, check the source pole's existing connection references and skip a destination
   already present. Draft mode does not construct permanent line occupants on these new links. Real mode
   creates missing line occupants for each wire cell: allocate 0x58, construct, AddRef, Init, set its mode,
   bind its endpoints and pivot flag, set its position from ComputePolePosition, and store the interface
   in that wire-cell record. Endpoint selection uses unsigned lexicographic **(z,x)** order.
5. Finish both endpoint connections in source/destination order. For a wire-bearing link the wire vector
   is supplied to the endpoint chosen by that ordering; the other endpoint receives null. For index -1
   both receive null. The connection geometry builder is `0x0064E3E0`; its full numeric body remains NT21.
6. After generated links, real mode releases and nulls stored wire interfaces through the pole-link walk
   (the native condition is wire-vector index **> 0**, so index 0 is skipped by this release pass).
7. Insert newly created poles through occupant-manager slot 20 `(AsOccupant,0)`. Draft poles receive
   occupant highlight `(3,0)`, are appended to the newly-created or reused list, and receive AddRef for
   that list. Real poles receive primary slot 9 (refresh connections). Release the per-record pole
   reference and null it in both modes.
8. Walk queued displaced pairs. Draft mode reconnects both directions with null wire vectors. Real mode
   with allowRepair=true releases the current pole infos, clears wire vectors, draws a line between the
   saved endpoints with snap=0 and current network type, sets valid=dragged, determines poles, and calls
   PlacePoles(false). Real mode with allowRepair=false skips that rebuilding. Release the queued pair refs.

The old-endpoint state flags and duplicate/repair queue details still require the NT18 proof. The above
order is not an authorisation to treat all wire crossings as new independent spans.

### 5.1 Slot 16: undo preview (`0x0064F3F0`)

Do these passes in order; empty each corresponding list after its pass:

1. Newly created draft poles: occupant SetHighlight(0,0). If occupant manager exists, remove occupant
   with `(AsOccupant,true,false)` and Release the pole. With no manager this Release is also skipped.
2. Reused poles: SetHighlight(0,0), primary slot 14(0), Release.
3. Saved connection edits: if endpoint A is nonnull, restore its connection-to-B state using
   `0x00649910(A,B,true)`, Release A and null it; then if B is nonnull restore its connection-to-A state,
   Release B and null it. Restoration selects the canonical direction using unsigned (z,x) ordering.
4. Call the base cleanup at `0x00625580`.

No rebuilding of pole positions or wire cells happens in this undo entry. The call order and the unusual
manager-null release branch are verified at `0x0064F407–0x0064F55C`.

## 6. Reserved-tile walk and zone update

Slot 81 (`0x0064FED0`) is specified by network-geometry §11. For NT20(b), its `last` starts at input
count-1, not -1. Clear cells set last=currentStep. Zones set failure `0xF0000011` and preserve last;
buildings set `0xF0000012`, set last=currentStep-1 and stop. Roads set `0xF0000013` and preserve last,
except a last-step road immediately after result 0 sets last=currentStep-1 and stops. Results above 3
preserve last and failure but become the next iteration's previous result. Finally count=last+1.
Thus a final zone after a clear step loses that final step; an all-zone walk can retain the original
count because last was initialised to count-1. The uninitialised first previous-result quirk remains.

UpdateOnZoneChange, slot 10 (`0x006522F0`), takes `(minX,minZ,maxX,maxZ,enabled)` (pop 20).
Enabled=0 is an immediate no-op; otherwise force real=true before querying occupants in the rectangle.
It distinguishes poles (QI `0xA8C86142`) from wire occupants (QI `0xC98F3B24`), repairs connections,
and manages temporary occupant/pole lists. Full ordering and boundary cases remain NT22; the existing
decompile has invalid stack/register attribution in this function, so that text alone is insufficient.

FindNewPolePosition (`0x006520C0`, pop 40) accepts an inclusive rectangle, start index, exclusive stop
index, signed index increment, a cell-vector reference, and two endpoint interfaces. Starting at the
given index, skip cells inside the rectangle or whose CellIsOKForPlacement result is nonzero; choose
the first cell outside the rectangle with result 0. If the exclusive stop is reached, return without
repair. For a chosen cell it rebuilds the input anchors and draws/determines/places a replacement span;
exact endpoint and release behavior remains NT22. Bounds are unsigned inclusive comparisons
(`0x006520E3–0x00652120`); classifier is not called for cells inside the rectangle.

## 7. Resources

The once-only StaticInit at `0x00626990` sets latch `0x00B452A8` before registering callback
`0x006232F0` with service slot 69, argument 0, then ReloadRules; returns true. It is separate from the
power-line Init latch and its resource-reading latch.

ReadPowerPoleInfos (`0x0064F880`) requests file TGI `00000000/8A5971C5/8A5993B9`, final argument 0.
When available, open as INI, enumerate section **PowerPoles**, then destroy the INI. Release the file
interface if nonnull, including on the failure path. The enumeration callback `0x0064F140` converts
the key and value to uint32 and stores value into `0x00B467B0[key]`; there is no bounds check in that
41-byte callback. It does not validate the exemplar or report an error to Init.

## 8. Discriminating vectors

These are host-observation vectors, not claimed in-game measurements.

| Vector | Inputs | Required result/order |
|---|---|---|
| PL-I1 | eligible prior-run step; horizontal old span; no generated record shares either endpoint coordinate; pivot; no prior request | retain both endpoints, request and emit pole; release wire probe only |
| PL-I2 | same as I1, but diagonal endpoints and a forced anchor | reject endpoints even though forced; release both endpoint refs, preserve pre-existing pending request |
| PL-I3 | eligible axis-aligned span; generated record shares endpoint x but has a different z; not forced | coordinate-OR predicate rejects; pending request cleared, both endpoint refs released |
| PL-I4 | same duplicate as I3, forced spacing point | accept descriptor and pending request; no endpoint release in the interception branch |
| PL-I5 | eligible axis-aligned span; descriptor matches only after endpoint reversal; no coordinate predicate matches | ordered descriptor comparison alone does not reject; no pointer-order normalisation |
| PL-I6 | current step equals previously placed count; axis-aligned old wire; forced | reject on equality boundary before duplicate probes; release both endpoints and wire |
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
| PL-G1 | clear, flat axis drag; anchors at steps 0 and 5; spacing=3; no existing poles/wires; no ray hit | poles at steps 0,3,5; cells at steps 1,2,4 count as wire cells |
| PL-G2 | same drag, spacing=20 | endpoint poles only; changing spacing must not change cell-width conversion |
| PL-G3 | no emission pending at pivot k; terrain probe from prior pole hits exactly at next-cell distance | force pole at k; a farther hit does not force it |
| PL-G4 | previous pole at (0,0), next test cell at (2,0), width=16, all ground=10, clearance=7 | test ray endpoints (8,17.5,8) and (40,17.5,8); pole base y remains 10.5 |
| PL-G5 | four returned corners all -FLT_MAX; output y prefilled 123 | output y becomes -FLT_MAX; return a position, do not skip creation |
| PL-G6 | corners NaN, 4, 3, 2; terrain special branch false | y=4.5; NaN does not poison later ordered comparisons |
| PL-P1 | draft, generated poles A,B, no pre-existing connection | create/reuse both before linking; no permanent wire-cell occupants; highlight/list/AddRef before final pole Release |
| PL-P2 | real, displaced pair, allowRepair=false then true | false skips redraw; true redraws, Determine, PlacePoles(false); recursive call cannot trigger another repair |
| PL-U1 | one new pole, one reused pole, one saved pair | clear/remove new; clear/reset/release reused; restore/release A then B; base cleanup last |
| PL-U2 | new draft pole, occupant manager null | clear highlight; native code skips both manager removal and pole Release |
| PL-R1 | count=3, classifier 0,0,1 | output count=2, failure=0xF0000011 |
| PL-R2 | count=3, classifier 1,1,1 | output count=3, same failure; distinguishes initial last=count-1 |
| PL-R3 | count=3, classifier 0,4,3 | result 4 leaves last=0 but becomes previous; final road does not stop via previous=0; count=1, failure=0xF0000013 |
| PL-Z1 | zone-update enabled=0 | no mode change, query or allocation |
| PL-Z2 | FindNewPolePosition candidates inside rectangle, outside blocked, outside clear | no classifier call on inside; reject blocked; first outside clear wins |

## 9. Evidence and remaining owner audit

The local attribution algorithm (`tools/coverage.py`, read-only Attribution.run) reports **46 entries,
16,156 function bytes** for this owner. The brief's 49-function count is not reproduced; byte count matches.
This is not a completed-owner claim. Geometry, connection mutation, zone update and supporting resource
functions remain partial as enumerated below. All attributed entries are included; sizes are function
body bytes, not address-span lengths. No full-owner completion credit is claimed.

| Entries (address:bytes) | Contract status |
|---|---|
| `00626990:45`, `0064F0D0:25`, `006503C0:190`, `00650560:30`, `00650580:77`, `006505D0:89`, `00650630:289` | lifecycle §§1,7 |
| `0064F0F0:70` | existing cost, network-geometry §11 |
| `0064EFE0:232`, `0064F2B0:310`, `0064F3F0:381`, `0064F570:92` | position/highlight/undo/text §§2–5 |
| `0064F140:41`, `0064F880:153` | resource enumeration §7 |
| `0064F170:86`, `0064F1D0:95`, `0064F230:125`, `0064FB40:176`, `00650760:216` | record/link/pivot helpers; caller eligibility NT18 |
| `0064FED0:263`, `00650140:258` | reserved walk §6; existing create/reuse, network-occupants §4.5 |
| `0064FD00:459`, `0064FFE0:347`, `0064FBF0:267` | classifier/pole search/wire probe partial, NT24 |
| `00650840:3345`, `00651560:2884`, `00652980:235` | wrapper proved, Determine/PlacePoles partial, NT18 |
| `006520C0:552`, `006522F0:1658` | zone repair partial, NT22 |
| `00649910:149`, `0064A2E0:43`, `0064B6E0:106`, `0064E3E0:1338` | connection helpers/allocator incomplete, NT21/NT24 |
| `0064F5D0:55`, `0064F610:92`, `0064F670:148`, `0064F710:89`, `0064F770:260`, `0064F920:113`, `0064F9A0:111`, `0064FA60:135`, `0064FAF0:68`, `00650250:84`, `006502B0:58`, `006502F0:99`, `00650480:218` | 13 container helpers, 1530 bytes; ownership audit NT24 |

The descriptor predicate `0x0064F170` tests exact ordered endpoint-pointer and flag-byte equality against
each generated record, without normalising endpoint order. `0x0064F1D0` returns false for null; otherwise
obtains its cell and returns true when **either** x **or** z matches a generated pole record
(`0x0064F210–0x0064F218`). Do not replace this OR with same-cell AND. Interception eligibility remains NT18.

PlaceNeighborConnections (`0x00624610`) consumes its 24-byte records in vector order. For each, scan types
0..12; for a noncurrent type included in the tool mask with nonzero connection-kind mapping, call regional
manager slot 7 to remove that kind at record x,z. Then slot 6 creates record kind at x,z with endpoint flag.
Record x,z are at +8,+12, kind at +16, flag byte at +20. All removals precede creation per record.
Type mappings and full Attempt derivation remain NT23; wrapper pairing is specified in §2.

Implementation boundary: defaults, wrapper pairing, position, highlight, text, resources/lifecycle and
undo are now public rules. Entire Determine/PlacePoles, regional Attempt and zone repair remain incomplete;
the whole host hooks cannot yet be removed. No engine code was read or changed.

REA batch queries for `0x0064F3F0`, `0x0064F230`, `0x00650760`, `0x0064FBF0` returned
`target_unavailable` (no app open). The permitted fallback is the local export and existing class decompile.
