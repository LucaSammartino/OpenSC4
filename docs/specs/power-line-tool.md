# Power-line tool: owner contract

Windows GOG 1.1.641, image base `0x00400000`. This supplements network-tools §9 and
network-geometry §11. Evidence addresses refer to that executable. Prose, tables and vectors are the public
contract; implementers must not consult the original program or its decompilation.

## 1. State and lifecycle

| State | Meaning |
|---|---|
| `+0x50` byte | real placement when nonzero; draft when zero |
| `+0x54`, `+0x60` | step ranges (inclusive indices into cells), cell pairs |
| `+0x70`, `+0x74`, `+0x78` | dragged, straight-prefix, valid step counts (network-tools §8.3.1.1, NT33) |
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

The following covers the ordinary drag, terrain test and interception gate. The break and saved-endpoint
rules below extend that path; NT18 still tracks any unproved surrounding construction, not permission
to substitute a simple fixed-spacing walk.

Reset wire count; release old pole infos; clear wire vectors. Traverse every inclusive cell range of every
valid step in order. Track the next input-anchor index, the last clear cell/step, the last emitted-pole
step, whether a span is active, and whether an anchor or break has requested a pole.

For each step, choose its **pivot** with `0x0064F230`: before the straight-prefix step count (when
that count is nonzero), use the step's first cell. Otherwise scan its cells in ascending index order
and take the first satisfying `abs(anchor.x-cell.x) == abs(anchor.z-cell.z)` for the next input anchor;
return -1 if none. This is a diagonal-distance equality, not a direction-change test.

The first cell of the step at the straight-prefix count that shares **either** x **or** z with the first input
anchor is a separate forced point when `0 < straightPrefix < valid`. Reaching it requests a pole once.
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

**Zoning and blocked cells:** Determine calls the classifier directly, without slot 81. Let C be the
number of poles emitted since the last span restart, L the remembered clear-cell index (initially -1,
reset to -1 on emission), A the span-active byte, and P the last emitted step (initially -1).

| Classifier result | Exact action |
|---|---|
| 1 (zone) | If C=1 and L=-1, set C=0 and remove the final pole record only if its link vector is empty; free its link storage if allocated. This branch does **not** release its saved endpoint references. Otherwise, if L>0 and A, emit a closure pole at L, link it to the previous pole, copy the accumulator to a new wire vector and clear the accumulator; reset descriptor pointers/state, C=0, L=-1, P=current step. In every zone case clear A and set the pending pole request. |
| 2 (building) | If neither (A and P>=0) nor forced, use the ordinary non-emitted-cell accumulation path, without assigning a failure. Otherwise assign failure `0xF0000012` and take the blocked termination below, even when unforced. |
| 3 (road) | Same initial guard. On passing it, assign failure `0xF0000013`; take blocked termination only when forced. An unforced road uses non-emitted-cell accumulation. |
| other nonzero result | Same initial guard; do not assign a failure. Only forced takes blocked termination; otherwise accumulate. |

Blocked termination: if C=1 and L=-1, set C=0 and, if the final record has no links, release each
nonnull saved endpoint, remove the record and free its link storage. Set L=-1. This branch does not
clear A or force a restart request. At traversal end, C=1 likewise removes a final linkless record,
with both saved-reference releases. Empty link vectors are tested by begin=end, irrespective of capacity.
The strict **L>0** zone-closure test excludes cell index 0. Neither discarded poles nor breaks recompute
the running wire count. Evidence: zone `0x006510C8–0x0065118E`, reset `0x006512A3–0x006512DB`,
blocked dispatch `0x006512E0–0x00651323`, release `0x00651323–0x0065139E`, end cleanup
`0x006514A4–0x00651505`. The zone path's missing saved-reference releases are an observable native
ownership asymmetry; do not infer those releases from the blocked/end paths.

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
in all hit cases, after the endpoint decision (`0x00650BCC–0x00650C35`). An accepted descriptor sets
the pending request even off-pivot: the non-pivot branch reaches the pending-request emission test
at `0x00650DFC–0x00650E08`. The pivot gate limits the terrain test, not pending-request emission.
At the end of the clear-cell path the pending byte is cleared whether or not a pole was emitted
(`0x00651073–0x00651075`).

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

1. Read the groups of **dragged-cell indices** at `+0x398..+0x39C` in order. For each group member, scan pole
   records from the start and collect the first record with that cell index (`record+0x0C`). In draft,
   also require its pole pointer to be null; real mode has no such filter. Missing members add nothing;
   duplicate members are not deduplicated. Consider every ordered pair in the resulting index vector.
   Manhattan cell distance exactly 1 adds a reciprocal link with wire-vector index -1. Coordinates are
   converted as unsigned 32-bit values to x87 before subtraction/absolute-value/sum, so this comparison
   does not wrap at 32 bits. This precedes occupant creation (`0x00651640–0x00651800`). The link helper
   appends both directed records on every call, without duplicate suppression; it ORs the same
   classifier-derived direction bit into both pole masks (`0x0064FB40–0x0064FBE4`). Ordered-pair
   traversal can therefore append repeated link records; the later occupant connection probe is separate.
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
   both receive null. The connection geometry builder `0x0064E3E0` is specified in §§5.3,5.5.
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

For each record, saved endpoints A/B and state bytes a/b are at +0x24/+0x28/+0x2C/+0x2D. Read A's
cell first, then B's (missing endpoint coordinates remain -1). If both exist, detach A→B, then B→A:
real mode calls RemoveConnection(other,0), draft calls `0x00649910(other,false)`. Detachment does not
depend on a/b. If A exists and a is nonzero, AddRef A and the record's new pole and append **(A,new)**
to the temporary repair vector. Do the analogous append for B/b afterwards. There is **no pair
deduplication**, no endpoint sorting and no suppression when the two pointers compare equal.

Draft with both saved endpoints transfers the descriptor, including state bytes, to the undo list
`+0x394`, with no extra AddRefs. Otherwise release each nonnull saved endpoint in A,B order. Null
both record pointers before processing generated links. Temporary repair-pair references are separate
from these descriptor references. Each queued pair is processed in append order after pole insertion;
in draft it invokes the connection builder in both directions, even when allowRepair=false. Only the
real recursive rebuild is gated by allowRepair. Evidence: `0x00651880–0x0065192E`,
`0x00651A08–0x00651A76`, `0x00651B4B–0x00651BD0`, `0x00651F6E–0x0065209D`.

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

### 5.2 Connection support contracts

`0x00649910(self, other, enable)` locates the **first** connection record whose endpoint pointer equals
other, scanning `self+0xAC..+0xB0` in 44-byte strides. It reads other's cell even if the connection vector
is empty. With no match it changes nothing. A false enable writes zero to that connection's byte +9.
A true enable rereads other's cell and writes 1 only if self's unsigned (z,x) is lexicographically less
than other's; equality and greater write zero. It never adds/removes a record, calls AddRef/Release,
or rebuilds geometry (`0x00649916–0x006499A2`).

`0x0064B6E0(destinationList, wireVector)` is a **copy of references**, not a transfer. Null or empty
wire vector does nothing. In 16-byte record order, read the interface at +8, AddRef it and append it
to the destination linked list using a 12-byte node. It performs no null-interface test, clears neither
source nor destination and does not deduplicate (`0x0064B6E0–0x0064B749`). The subsequent owner of
the destination list must release these references.

### 5.3 Connection builder: record, geometry dispatch and lifetime

`0x0064E3E0(self, other, optionalWireVector)` returns without changes for null other, an existing
connection with **both** cell coordinates equal to other's, or other at self's computed cell. It does
not use pointer equality to detect the existing connection. Self's cell here is truncation toward
zero of world x divided by the float at `0x00B46760`, and world z divided by `0x00B4675C`; other's
cell comes from its slot 8. The instruction trace, rather than the decompiler's register names,
establishes these inputs (`0x0064E3E9–0x0064E47F`).

The direction helper `0x0061F4C0(selfX,selfZ,otherX,otherZ)` returns 0 for equal z (including equal
cells), 2 for equal x and unequal z, otherwise 1 when the two unsigned coordinate comparisons have
the same result and 3 when they differ (`0x0061F4C0–0x0061F506`). This selects an attachment-bank
orientation; it is not an endpoint compass direction.

For a new connection allocate an empty circular list, retain other once, and initialise a 44-byte
record: endpoint pointer +0; list head +4; byte +8 zero; byte +9 equal to unsigned
`(selfZ,selfX) < (otherZ,otherX)`; other's cell at +12/+16; two empty geometry vectors at +20/+32.
Probe service `0x00B43CF4`, slot 34, at self's stored cell +0xC8/+0xCC; only if false probe other's
cell. The OR result controls the extra three-point geometry below. These probes also run for the
endpoint that will not build geometry (`0x0064E49D–0x0064E558`).

Only when byte +9 is true, obtain four world attachment points for each endpoint, using the **same**
orientation for both. Helper `0x00649BA0` selects one of four 48-byte banks at endpoint +0xE4 and
adds endpoint world position +0xA0/+0xA4/+0xA8 to each point, storing each sum as float32. For each
of the four corresponding pairs, in index order:

1. Call `0x0064A6C0(A,B,&controlA,&controlB,&N)`; there is no coincident-attachment guard here.
2. Allocate N float32 triplets. Copy A to the first triplet. Call `0x006493B0` with A, controlA,
   controlB, B, output beginning at triplet 1, and count N−1. Append this owned triplet buffer to
   the record's first geometry vector; each vector entry is 12 bytes (buffer/end/capacity).
3. If the service OR result is true, allocate three triplets and call the same sampler with the
   same four points and count 3, starting at triplet 0. Append this buffer to the second geometry
   vector. No separately copied A precedes these three samples.

These calls and counts are instruction-derived (`0x0064E55E–0x0064E7FE`); §5.5 specifies the numeric
curve contract. After all four pairs, copy optional wire references into the record list through §5.2, then
call self's occupant-view slot 15 with `(0x99EF1145,0)`. With byte +9 false, skip geometry, optional
wire copying and this call; still append the connection record and set self +0xD0 to 1
(`0x0064E804–0x0064E879`).

Appending deep-copies the record's list nodes and geometry storage (`0x0064CFA0`); the endpoint's
single retained reference is owned by the stored record, not released at this return. Temporary
geometry buffers, their vector storage, list nodes and sentinel are freed after the copy. The
temporary list is freed without releasing its copied interface payloads, transferring responsibility
for the AddRefs taken by §5.2 to the stored connection. The caller's optional vector is neither
cleared nor consumed (`0x0064E886–0x0064E91C`). The nested-copy helper's full allocation details
are explicit: `0x0064BC00` copies each 12-byte geometry-vector entry through `0x0050BCB0`, which
allocates exactly its used triplet count and copies all three dwords per triplet. `0x006F1570`
allocates fresh list nodes and copies interface pointers without AddRef. Thus stored geometry is
independent of the freed temporaries; payload-reference ownership is not multiplied by the copy.

### 5.4 Regional Attempt for network type 5

`0x00628BD0(mode,5)` examines the final two anchor cells (+0x118..+0x11C), copies them and invokes
tool slot 18 with their addresses, mode and type 5. Subsequent tests use the resulting copies. Equal
endpoints return false. This body assumes at least two anchors; the power-line wrapper's guard is
specified in §3. Let M and D be the type-5 rule entry's dwords +0x1C/+0x20, W/H the city dimensions,
and the last dragged step's inclusive cell interval be [first,last]. It does not inspect the tool's
real/preview byte (`0x00628BD8–0x00628C59`).

If straightSteps >= draggedSteps, direction d is signed penDirection/2. Otherwise require D=1;
when the final step has at most one cell, replace d by
`(endDirection−penDirection+penDirection/2) & 3`. The offset tables map d=0,1,2,3 to west,north,east,
south. Add that offset to the resulting final endpoint to obtain candidate (nx,nz). If both coordinates
are unsigned less than W/H, return false. For the straight case edgeWord is `2 << (8*d)`; for the
diagonal case it is `3 << (8*d)` when signed endDirection−2*d lies in [0,4], otherwise
`1 << (8*d)` (`0x00628C5F–0x00628DC9`). These formulas describe the stored word, not a compass mask.

When M>1, compare regional-service slot 12 results at the two final-step extreme cells projected onto
the candidate border. The straight case uses candidate z for odd d and candidate x for even d; the
diagonal case uses candidate z. Unequal results return false. Let low=edgeWord & 0x01010101;
low!=0 with mode>1 also returns false. Require a regional service and a nonzero regional-type table
entry. Original PE `0x00AA84C4+5*4` is **4**: type 5 therefore skips the special branch for regional
types 3,12,8 (`0x00628CC1–0x00628ECA`). No consent UI is called on this type-5 path.

Validate final-step cells in increasing index order, inclusive. For each available GetCellInfo result,
require its type flags at +8 to include 0x20 (the type-5 mask at `0x00AA835C+5*4`). If its aggregate
edge byte at +0x4C for direction d is nonzero, visit all 13 per-type edge bytes in type order. Every
nonzero byte must equal `(edgeWord >> (8*d)) & 255`. For a different type with greater priority than
type 5 (signed table comparison at `0x00AA83B0`), also require its bit in the type-5 rule entry's
+0xFC compatibility word. For each cell, ask regional-service slot 11 `(4,x,z)`, replacing the varying
border coordinate with the cell's coordinate: x at a north/south border, z at a west/east border.
A nonzero result rejects. Missing cell-info skips its local checks, not the regional probe
(`0x00628EC9–0x00629008`). No rejection writes a failure code or changes the pending vectors.

If low=0, append one 24-byte record per final-step cell to **both** vectors +0x100 and +0x10C, preserving
existing entries. Record fields are local cell key `(z<<16)+x`, edgeWord, external neighbor x,z,
regional type 4, and a byte at +20. That byte is false for the first record, true for every later
record on the type-5 path. The varying external coordinate follows the local cell. If low!=0,
require mode=1 and append one record to both vectors: key is the candidate clamped from negative
coordinates to 0 and from coordinates equal to W/H to W−1/H−1; retain the original external nx,nz
in the record, and byte +20 is false. Other out-of-range positive values are not generally clamped.
Both paths return true after appending (`0x00629008–0x0062916D`). Attempt itself creates no preview,
clears no vectors and neither requests consent nor places a regional connection. Its paired Place
operation remains a separate call in the power-line wrapper.

### 5.5 Connection curve numbers and evaluation

The control helper `0x0064A6C0` uses these parameters. The defaults and property IDs are verified at
`0x0064A18B–0x0064A24F`; each call of InitPowerLineControlVariables resets all defaults before
attempting resource `(0x6534284A,0xE7E2C2DB,0xC911E35B)`, interface `0xA52160F5`, then the six
property reads in table order. Values below are defaults, not claims about every loaded city resource.

| Storage | Meaning in this helper | Default | Property |
|---|---|---|---|
| 00B46754 | maximum control displacement from midpoint | 16.0 | E9A18D3F |
| 00B46750 | displacement percentage of span length | 10.0 | 49A1E8C4 |
| 00B4674C | maximum downward sag | 10.0 | 89A18FC4 |
| 00B46748 | sag percentage coefficient on squared length | float32 bits 3E19999A (about 0.15) | E9A1E8D4 |
| 00B46744 | distance per sample | 8.0 | 49A195C0 |
| 00B46740 | unsigned minimum sample count | 5 | 89A1E8FB |

For attachment points A,B let delta=B−A and L its length. The percentage multiplier is float32
bits `3C23D70A` at `0x00A823A4`; the half multiplier is exactly 0.5 at `0x00A84D2C`. Compute
g as the lesser of maximum displacement and float32(displacementPercentage × multiplier × L).
Compute s as the lesser of maximum sag and float32(sagPercentage × multiplier × L × L), using
left-to-right products. Let u be the normalised delta. Control points are A+u×(L/2−g) and
A+u×(L/2+g), with s subtracted from each resulting y. N is truncation toward zero of the greater
of float32(unsigned minimum count) and float32(L / distancePerSample). There is no additional lower clamp,
zero-length branch, or special treatment of negative/invalid resource values
(`0x0064A6C0–0x0064A86E`).

Numeric evaluation details: delta x,y initially remain in x87 precision; delta z is stored to float32
without popping and its square multiplies the retained value by that stored copy. Add y² then x²,
take sqrt, and store L to float32 while retaining the first root for reciprocal computation. Store
1/root to float32, then store all normalised components to float32 (z is reloaded from its stored
delta). Subsequent length products use stored L. Store control-point coordinates to float32 before
subtracting s from stored y and storing y again. Sag/displacement selection chooses the cap only
when cap is strictly less than the computed value; equality or unordered chooses the computed value.
For N, choose the minimum-count float only when the retained quotient is strictly less than it;
otherwise choose the quotient's float32 copy, including on unordered comparison. These
rounding points follow `0x0064A6DB–0x0064A719`, `0x0064A71D–0x0064A81C`, `0x0064A81F–0x0064A867`.

Sampler `0x006493B0(A,C1,C2,B,out,n)` writes n triplets at t=1/n,2/n,…, using a retained x87 step
and repeated addition, not recomputed integer division per sample. Each coordinate uses the cubic
weights w0=1−3t+3t²−t³, w1=3t−6t²+3t³, w2=3t²−3t³, w3=t³. Constants 1,3,6 are **float64** at
`0x00A80AB0`, `0x00A929F0`, `0x00AA9CA0`. It stores t², 3t, 3×stored(t²), and w1 to float64;
t³ uses the retained t². Evaluate w0 by successive subtraction/addition/subtraction, w1 as
(stored(3t)−6×stored(t²))+3t³, and w2 as stored(3×stored(t²))−3t³. Other intermediates remain x87.
Sum coordinate products in this order: x = A,C1,C2,B; y = B,C1,C2,A; z = C2,A,B,C1, then store
float32. The loop unrolls four samples with the same arithmetic, then handles the remainder. No
final overwrite forces the last output to equal B exactly (`0x006493B9–0x006497DB`).

Worked default example: A=(0,20,0), B=(32,20,0) gives N=5, g approximately 3.2 and s approximately
1.536. The main strand stores A followed by four samples (t=1/4,1/2,3/4,1); the optional strand stores
three samples (t≈1/3,2/3,1), without A. This example expresses approximate geometry; the rounding rules
above determine exact stored bits. World-to-cell divisors in §5.3 are supplied by terrain-service
slots 91/92 and stored to float32 by pole Init (`0x00649FA9–0x00649FC9`), not hardcoded here.

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
The entry, query, pole and cleanup contracts are in §6.4; wire handling is in §6.3. The unchecked
backward split boundary remains NT22. The existing decompile has invalid register attribution.

FindNewPolePosition (`0x006520C0`, pop 40) accepts an inclusive rectangle, start index, exclusive stop
index, signed index increment, a cell-vector reference, and two endpoint interfaces. Starting at the
given index, skip cells inside the rectangle or whose CellIsOKForPlacement result is nonzero; choose
the first cell outside the rectangle with result 0. If the exclusive stop is reached, return without
repair. For a chosen cell it rebuilds the input anchors and draws/determines/places a replacement span;
endpoint and release behavior is in §6.2. Bounds are unsigned inclusive comparisons
(`0x006520E3–0x00652120`); classifier is not called for cells inside the rectangle.

### 6.1 Classifier and occupant probes

CellIsOKForPlacement (`0x0064FD00`) returns an integer, not a boolean. Initialise result=0. Query the
zone service's slot 12 at (x,z). If nonzero, query it again at (x,z); if that second value is not 15,
query a **third time at (z,z)**. A third value other than 14 returns result=1, without querying
occupants. This literal third-call coordinate asymmetry is visible at `0x0064FD92–0x0064FD98`.
Do not fold these calls into one cached zone value.

On the remaining path, ask the tool's occupant manager slot 40 for the single-cell inclusive rectangle,
with flags 0 and mask -1, and walk the returned list in order. Class `0x278128A0` writes result=2.
Class `0xC772BF98` queries interface `0x4A10AFC2`: success releases that query reference and leaves
result unchanged; failure writes result=3. Other classes leave it unchanged. Release **every** returned
occupant reference as it is visited. Continue after every assignment; the last qualifying occupant wins,
so this is not building-before-road priority. Clear the query list's nodes. Only if result is still zero,
call service `0x00B43D08` slot 6 `(x,z,0)`; nonzero changes result to 2. Clear/free list storage and
return result. No failure-code field is written here (`0x0064FDAD–0x0064FEC8`).

The pole probe (`0x0064FFE0`, cdecl x,z,out) and wire probe (`0x0064FBF0`, thiscall x,z,out) both
query occupant manager `0x00B43D0C` slot 40 for that same single-cell rectangle, flags 0, mask -1.
They overwrite *out with null without releasing any incoming *out. Visit **all** returned occupants;
QI pole `0xA8C86142` or wire `0xC98F3B24`. The pole probe additionally obtains the occupant's world
position through occupant slot 8 and maps x,z through manager slot 19, requiring exact equality of both
cell coordinates to the requested x,z. A mismatching pole QI is released immediately. The wire probe
has no extra coordinate/endpoint/direction filter. For each accepted candidate release the previously
selected QI reference, store the new QI reference, and keep scanning. Release every query-list occupant
reference once, including accepted candidates. Finally destroy the list nodes and return AL=true iff
any candidate was accepted; output is the **last accepted** QI reference, owned by caller, or null.
Evidence: pole `0x0065007D–0x0065013A`, wire `0x0064FC8D–0x0064FCF8`.

`0x0064A2E0` allocates a raw pool block: pop the freelist head at `[0x00B0C25C]+8`, refilling through
`0x0090CF9A(0x00B0C258)` when empty; replace head with the returned block's first dword, increment the
pool's dword allocation count at +12, return the block. It does not initialise an occupant or AddRef
anything, and has no explicit null-recovery branch (`0x0064A2E0–0x0064A30A`).

### 6.2 FindNewPolePosition (`0x006520C0`, pop 40)

Ten stack arguments are consumed: rectangle minX,minZ,maxX,maxZ; start index, exclusive stop index,
signed index increment; pointer to a cell vector; an unused ninth dword; optional other-pole interface.
The ninth argument is not an endpoint selection fallback. The scan stops on equality with stop, not
on crossing it: callers must supply a reachable stop. Rectangle inclusion uses unsigned comparisons
on both axes, inclusive at both edges. Cells inside the rectangle are skipped without classification.
For an outside cell, classify it and stop at the first result 0; other results continue in the given
index direction. start=stop or exhaustion returns without creating anything (`0x006520C0–0x00652129`).

At a hit build a temporary pole record with that cell and an empty link vector. The direction mask is
zero when other is null; otherwise read other's cell and set the byte to 1 shifted by the low five bits
of `0x0061F4C0(foundX,foundZ,otherX,otherZ)`. CreatePowerPole creates/reuses it. Only newly created poles
are inserted through occupant-manager slot 20 `(AsOccupant,0)`. Release the returned pole reference
immediately after that insertion/reuse decision, before the remainder of this method.

If other is null, finish without replacing input anchors or calling Determine/PlacePoles. If nonnull,
reread other's cell, replace the input-anchor vector with [found cell, other cell], call draw slot 18
with those endpoints, snap=0 and current network type, set valid=dragged, DeterminePolePositions and
PlacePoles(true). This does not run the slot-53 regional gate or highlight path. Other is borrowed:
the function neither AddRefs nor Releases it. Free the temporary link-vector buffer before returning;
there is no success result or written output pole. Evidence: mask/create/release
`0x00652135–0x006521C2`, optional-other branch `0x006521C5–0x006521DB`, anchor replacement
`0x006521DE–0x00652298`, draw/solve `0x0065229D–0x006522E7`.

### 6.3 Zone-update wire selection and splitting (bounded)

Within UpdateOnZoneChange's wire branch, QI the queried occupant for `0xC98F3B24`. On success obtain
its endpoint references. Visit already retained wire interfaces in insertion order, obtaining their
endpoint references and releasing those temporary references after inspection. Reject the new wire
when **any endpoint pointer** equals either endpoint of a retained wire. This is endpoint overlap,
not equal wire identity or equality of an unordered endpoint pair. A null endpoint is not excluded
from the equality comparisons. Rejecting releases the new wire QI; accepting appends it to the retained
wire list without another AddRef. Release the new wire's temporary endpoint references in either case,
then release the query-list occupant (`0x006525BA–0x006526BA`).

In the later retained-wire walk, read endpoints A,B through wire slot 3. If either is null, skip repair.
Also skip if either equals a pole already retained in the separate pole list. Otherwise obtain the
connection cell vector through A's slot 16 `(B,&cells)`. Require more than one cell. Read the wire's
own cell through slot 5 and scan the connection vector from index 0 for the first equal x,z. With
no match, do not detach or relocate. At a match k, detach A→B then B→A with RemoveConnection(other,0).
Then invoke FindNewPolePosition twice with the same rectangle and cell vector:

| Order | Start | Exclusive stop | Increment | Ninth argument (unused by callee) | Optional other |
|---|---|---|---|---|---|
| first | k | cell count | +1 | A | B |
| second | k | cell count | -1 | B | A |

Both calls restart at k; the second does not start at k-1 and does not use -1 as its stop. The callee
has no lower-bound guard, so a caller assumption about finding a clear outside cell on the backward
walk remains an explicit NT22 boundary question. Free the connection-vector storage, release A then B,
then the retained wire QI. Evidence: guards `0x006526D0–0x00652758`, first-match search
`0x0065275E–0x00652795`, detachment/call arguments `0x0065279A–0x00652828`, cleanup
`0x00652835–0x0065286A`. The entry/query and pole-branch orchestration remain outside this bounded
contract. The source/reachability of `+0x398` adjacency groups has not been established by this trace.

### 6.4 Zone query, pole repair and cleanup

For enabled!=0, set real byte +0x50 to 1 and allocate separate empty pole and wire lists. Prepare a
third, allocator-backed occupant list, clear it, then call occupant-manager slot 40 with
`(list,&minimumCell,&maximumCell,0,-1)`. The two cell pairs preserve the supplied rectangle; there is
no expansion here (`0x00652309–0x006523EB`). Visit the query list in returned order.

Try pole QI `0xA8C86142` **first**. Only a failed pole QI permits the wire QI from §6.3. For a pole,
get the original queried occupant's position through its slot 8 and convert its x,z through manager
slot 19 to a cell. Accept this cell only within the supplied rectangle, with **signed inclusive**
comparisons on all four bounds. A rejected pole QI is released; the queried occupant is then released.
These signed comparisons differ from FindNewPolePosition's unsigned rectangle tests
(`0x00652410–0x00652497`, `0x006525B1`, `0x006526A0–0x006526BA`).

For an accepted pole, read connection count from pole slot 4, then visit connection indices from zero
upwards. Slot 5 copies that connection's cells into a temporary vector. Ignore vectors of size <=1.
If the first cell equals this pole's converted cell, call FindNewPolePosition with start=1, stop=count,
increment=+1. Otherwise use start=count−1, stop=0, increment=−1. Pass this pole as the unused ninth
argument and the result of pole slot 7(connectionIndex) as the optional tenth argument. Free each
temporary cell buffer after the call. Append the accepted pole QI to the pole list without another
AddRef, even with no connections, and release the queried occupant. This branch does not deduplicate
accepted poles (`0x0065249D–0x006525AC`).

Repair retained wires as in §6.3, releasing their endpoint references and each retained wire QI during
that walk. Then visit retained poles in insertion order: pole slot 6(true), manager slot 21
`(pole+8,true,false)`, and release the retained pole QI. Finally clear query-list **nodes**, free its
sentinel through its allocator, and free both retained-list nodes and sentinels. Query-list clearing
`0x00413E60` frees nodes without releasing interface payloads; their releases already happened in the
query walk. End cleanup does not restore real=false (`0x00652883–0x00652979`).

**Unchecked split boundary:** the wire branch supplies `(k,count,−1)` to its second relocation call,
not `(k,−1,−1)` or `(k,0,−1)`. Its preceding guards establish two endpoints, neither in the retained
pole list, more than one cell and a first match for the wire's cell. They do **not** test for a clear
outside-rectangle cell at or before k. FindNewPolePosition indexes before testing the next stop and
has no nonnegative-index check. Thus a safe backward exit is an additional caller/data invariant,
not proved by these guards (`0x0065270C–0x00652828`, `0x006520E1–0x00652120`). NT22 remains open for
the reachable connection-cell/rectangle invariant; no out-of-bounds result is asserted as an observed
game outcome.

### 6.5 Container copy, erase and destruction ownership

The following 13 attributed helpers total 1530 B. All allocate/free ordinary storage; none takes or
releases occupant-interface references. In particular, copying a pointer-bearing record is not a
new AddRef. Callers remain responsible for the explicit reference transfers/releases in §§4–6.
Vector triplets below are begin/end/capacity; lengths, not source spare capacities, determine new
copy allocations. Padding is not a semantic field.

| Entry / body bytes | Contract established by the native body |
|---|---|
| 0064F5D0 / 55 | Copy [first,last) 16-byte records forward to uninitialised output; three dwords and byte +12 per record. Return output end. No allocation or source mutation. |
| 0064F610 / 92 | Construct a vector copy of those 16-byte records: allocate exactly used count×16, invoke F5D0, publish end; empty source yields null triplet. Pointers within records are shallow copies. |
| 0064F670 / 148 | Copy [first,last) 12-byte vector triplets to uninitialised output; each receives its own exactly sized buffer of the 16-byte records. Return output-triplet end. |
| 0064F710 / 89 | Construct one 48-byte pole record: copy dwords +0..+16 and bytes +20/+21, copy-construct the 8-byte-element link vector at +24, and copy the three descriptor dwords at +36. No AddRefs on descriptor pointers. |
| 0064F770 / 260 | Assign the 16-byte-record vector, self-assignment no-op. Reallocate exactly source used size if destination capacity is too small; otherwise reuse capacity, overwrite existing elements and construct only the new tail. Shrinking changes end without releasing interface payloads. |
| 0064F920 / 113 | Copy-construct a range of 48-byte records using F710's field/link/descriptor rules; return output end. |
| 0064F9A0 / 111 | Construct count copies of the same 48-byte source record, independently copying each link buffer; return output end. |
| 0064FA60 / 135 | Assign a range of existing 48-byte records forward: overwrite scalar/descriptor fields, assign each 8-byte-element link vector (00611B00), return output end. No occupant release when overwriting a descriptor. |
| 0064FAF0 / 68 | Assign a range of 12-byte int-vector triplets forward through 0050DB90; each vector owns a separate int buffer, reusing capacity where sufficient. Return output end. |
| 00650250 / 84 | Erase a range of int-vector triplets: shift following triplets by deep assignment (FAF0), free the trailing inner buffers, reduce outer end. Preserve outer capacity; return erase position. |
| 006502B0 / 58 | Destroy a vector of 48-byte records: free every link buffer at record +24, then outer storage. No endpoint Release, no clearing of the now-dead triplet. |
| 006502F0 / 99 | Erase a range of 48-byte records: shift following records by FA60, free the trailing link buffers, reduce end and preserve capacity. Return erase position. |
| 00650480 / 218 | Allocate capacity n+max(n,count) records, copy prefix before position, construct count copies of value, and copy the old suffix only when the final byte argument is false. Destroy old storage through 502B0, publish the new triplet. No AddRef/Release accompanies relocation. |

The 8-byte link-vector constructor is `0x007CED60` (exactly used count×8 allocation and two dwords
per link); assignment `0x00611B00` grows/reuses storage. The int-vector assignment is
`0x0050DB90`. None turns these data elements into COM references. The caller's ReleasePoleInfos
and its saved-descriptor rules must be applied separately; freeing container storage alone does not
undo a retained endpoint. Native evidence is each row's entry through its measured body end,
especially `0x0064F734–0x0064F75F`, `0x0064FA88–0x0064FAD9`, `0x006502C0–0x006502E3`, and
`0x006504CE–0x00650550`.

### 6.6 Adjacency-group construction boundary (NT18)

A literal displacement search of the native export for +0x398/+0x39C within 0064xxxx–0065xxxx
finds only constructor zeroing (`0x00650458/5E`), destructor address-taking (`0x00650583`), and
PlacePoles reads/address-taking (`0x00651588/8E/98/D8`, `0x0065182B/31/37`). The final call is
the range-erase helper §6.5, not an append. No such access occurs in UpdateOnZoneChange. This proves
no direct producer in these traced bodies; it does not exclude an alias, inherited/external writer
or a different displacement from an adjusted base. Construction and runtime reachability of nonempty
+0x398 groups remain NT18. Do not attribute their construction to zone update.

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
| PL-B1 | zone, active span, C=2; L=0 versus L=1 | L=0 does not close; L=1 emits closure; both clear active and request restart |
| PL-B2 | building, active, P>=0, unforced; versus road with same state | building sets failure 0xF0000012 and terminates; road sets 0xF0000013 and accumulates |
| PL-B3 | zone after a solitary linkless pole, L=-1; versus blocked termination with same descriptor | both remove record; only blocked path releases saved endpoint refs |
| PL-EP1 | saved endpoints A,B both present; a=1,b=0 | detach both directions; queue (A,new) only; draft transfers descriptor to undo list, real releases A,B |
| PL-EP2 | saved A=B, a=b=1 | two repair entries, each with two AddRefs; no deduplication; two descriptor releases in real mode |
| PL-C1 | classifier zone calls return 2,2,14 respectively; x differs from z | calls (x,z),(x,z),(z,z); proceeds to occupant query, rather than returning zone |
| PL-C2 | no zone rejection; returned occupants building then road without QI, versus reversed list | result 3 versus 2; releases both occupants; skips fallback service in both cases |
| PL-C3 | two pole QIs pass, third QI maps to different cell | out is second QI; first and third QI released, all three occupant refs released |
| PL-C4 | two wire QIs in one cell with different spans | last QI wins without geometry filtering; first QI released |
| PL-S1 | enable connection with equal self/other cell, then enable with other's z one greater | byte +9 is 0 then 1; no connection mutation or refcount operation |
| PL-S2 | wire vector [W,W], destination initially nonempty | append W twice after existing entries; two AddRefs; source unchanged |
| PL-RL1 | outside clear candidate at start; ninth dword is a pole pointer; other=null | create/reuse and release found pole; no anchor replacement or solve; ninth dword ignored |
| PL-RL2 | first candidate exactly on rectangle maxX/maxZ, next outside and clear; other present | do not classify the boundary candidate; pick next; anchors found then other; release found pole before rereading other |
| PL-RL3 | start=stop, other present | no candidate read, no classification, no creation or geometry calls |
| PL-ZW1 | retained wire endpoints (A,B); next wire (B,C) | reject second QI despite distinct wire and endpoint pair; shared endpoint alone suffices |
| PL-ZW2 | retained wire has null endpoint; new wire also has null endpoint | literal pointer comparison rejects new wire; no null filtering in dedup gate |
| PL-ZW3 | eligible wire; connection cells include its cell first at k=2 and again at k=5; count=8 | detach once; relocation calls (start=2,stop=8,+1,other=B), then (2,8,-1,other=A) |
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
| PL-CN1 | connect null, duplicate cell, or same computed cell | no allocation, retention, record append or dirty flag change |
| PL-CN2 | self unsigned (z,x) greater than other; optional wire vector nonempty | record retained and dirty set; no geometry or optional-wire AddRefs |
| PL-CN3 | A=(0,20,0), B=(32,20,0), default curve properties | N=5; main strand includes A and four samples; extra strand has three samples without A |
| PL-CN4 | build lesser endpoint with two optional wire records | two AddRefs; caller vector unchanged; stored list survives temporary-node cleanup |
| PL-ZU1 | pole QI succeeds outside signed inclusive rectangle; wire QI would succeed | release pole QI and queried occupant; do not try wire QI |
| PL-ZU2 | accepted pole with zero connections | retain in pole list; later clear connections, manager remove, Release in that order |
| PL-ZU3 | split reaches k but no clear outside cell at or before k | local guards do not prove safety; unchecked backward scan remains NT22, no fabricated safe clamp |
| PL-RA1 | type 5; border validation rejects | false; no failure-code write, pending-vector append, consent dialog or preview |
| PL-RA2 | type 5, low=0, three final-step cells, successful probes | append three records to each pending vector; flag bytes false,true,true |
| PL-RA3 | type 5, low!=0, mode=2 | false; mode=1 can append one clamped-key record with external coordinates retained |
| PL-CO1 | copy a 48-byte record with two links and saved endpoint pointers | independent link allocation; same pointer values; zero AddRefs |
| PL-CO2 | destroy pole-record vector with saved endpoint pointers | free link buffers and outer storage; zero endpoint Releases |

## 9. Evidence and remaining owner audit

The local attribution algorithm (`tools/coverage.py`, read-only Attribution.run) reports **46 entries,
16,156 function bytes** for this owner. The brief's 49-function count is not reproduced; byte count matches.
This is not a completed-owner claim. The +0x398 group producer/reachability, unchecked backward split
invariant and the source of the 49-versus-46 count remain parked. All attributed entries are included; sizes are function
body bytes, not address-span lengths. No full-owner completion credit is claimed.

| Entries (address:bytes) | Contract status |
|---|---|
| `00626990:45`, `0064F0D0:25`, `006503C0:190`, `00650560:30`, `00650580:77`, `006505D0:89`, `00650630:289` | lifecycle §§1,7 |
| `0064F0F0:70` | existing cost, network-geometry §11 |
| `0064EFE0:232`, `0064F2B0:310`, `0064F3F0:381`, `0064F570:92` | position/highlight/undo/text §§2–5 |
| `0064F140:41`, `0064F880:153` | resource enumeration §7 |
| `0064F170:86`, `0064F1D0:95`, `0064F230:125`, `0064FB40:176`, `00650760:216` | record/link/pivot helpers; caller eligibility NT18 |
| `0064FED0:263`, `00650140:258` | reserved walk §6; existing create/reuse, network-occupants §4.5 |
| `0064FD00:459`, `0064FFE0:347`, `0064FBF0:267` | classifier and last-match probe contracts §6.1 |
| `00650840:3345`, `00651560:2884`, `00652980:235` | wrapper proved, Determine/PlacePoles partial, NT18 |
| `006520C0:552`, `006522F0:1658` | §§6.2–6.4; only backward split invariant remains NT22 |
| `00649910:149`, `0064A2E0:43`, `0064B6E0:106`, `0064E3E0:1338` | enable/reference-copy/raw allocator §§5.2,6.1; connection geometry/lifetime §§5.3,5.5 |
| `0064F5D0:55`, `0064F610:92`, `0064F670:148`, `0064F710:89`, `0064F770:260`, `0064F920:113`, `0064F9A0:111`, `0064FA60:135`, `0064FAF0:68`, `00650250:84`, `006502B0:58`, `006502F0:99`, `00650480:218` | 13 container helpers, 1530 bytes; §6.5; count-source reconciliation remains NT24 |

The descriptor predicate `0x0064F170` tests exact ordered endpoint-pointer and flag-byte equality against
each generated record, without normalising endpoint order. `0x0064F1D0` returns false for null; otherwise
obtains its cell and returns true when **either** x **or** z matches a generated pole record
(`0x0064F210–0x0064F218`). Do not replace this OR with same-cell AND. Interception eligibility remains NT18.

PlaceNeighborConnections (`0x00624610`) consumes its 24-byte records in vector order. For each, scan types
0..12; for a noncurrent type included in the tool mask with nonzero connection-kind mapping, call regional
manager slot 7 to remove that kind at record x,z. Then slot 6 creates record kind at x,z with endpoint flag.
Record x,z are at +8,+12, kind at +16, flag byte at +20. All removals precede creation per record.
The type-5 mapping and full Attempt derivation are in §5.4; wrapper pairing is specified in §2.

Implementation boundary: bounded connection geometry, type-5 regional Attempt, zone orchestration and
container ownership join the prior contracts. The whole owner is **PARKED**, incomplete, with NT18,
NT22 and NT24 residuals above. No whole-owner coverage, replacement or host-hook-removal credit.
This is the final third round; no fourth round is scheduled. No engine code was read or changed.

REA batch queries for `0x0064F3F0`, `0x0064F230`, `0x00650760`, `0x0064FBF0` returned
`target_unavailable` (no app open). The permitted fallback is the local export and existing class decompile.
