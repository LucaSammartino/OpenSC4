# Game of Life simulator

Status: first dedicated native-owner publication, Windows GOG **1.1.641.0**, source-only RE.
**[compat]** marks instruction-backed requirements on the stated domain; **[choice]** marks a portable
design freedom. All examples are **PROPOSED / UNEXECUTED**. No game, native execution, implementation
output, or engine tests were used. This is a bounded contract, not whole-owner or byte coverage credit.

## 0. Evidence and outline

Sections: §1 body attribution and limits; §2 storage and lifetime; §3 initialization/reset/randomization;
§4 mutation/readout; §5 generation and callbacks; §6 literal discriminators; §7 unresolved boundaries.

The independently measured executable is 7,524,352 bytes, file version 1.1.641.0, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`. The canonical Windows export
records that identical hash, program `SimCity 4.exe`, base `00400000`. Function metadata, callers,
data references, and byte-bearing instruction pages from `re_query.py` are the primary evidence.
Local decompilation was navigation only; its allocation/destruction type names are unreliable here.
Known live REA `No app is open` failure remains unresolved; no live setup or retry was attempted.

The public inventory's approximately 3 KB is a planning estimate. Earlier
[AnimationTickManager](animation-tick-manager.md) work expressly excluded this owner. Its TK questions,
scheduler, renderer, region, QuadGrid, and generic RNG/allocator algorithms remain separate.

## 1. Body attribution and supported scope

The simulator is embedded at **parent +0x40** in `cSC4ViewInputControlGameOfLife`. No simulator vtable,
QueryInterface, reference count, independently allocated simulator object, or standalone constructor
was established. Parent methods inline several simulator operations. Descriptive names below for
unnamed entries are behavioral labels, not new exported symbols or invented interface slots.

| Address / exported body bytes | Attribution | Published scope |
|---|---|---|
| `004BBE90` / 94 | named simulator RandomizeCells | §3 loop, byte stores, raw RNG calls; RNG internals foreign |
| `004BBF00` / 75 | unnamed, receiver proven by both parent mouse callers | §4 direct cell mutation and AL result |
| `004BBF50` / 572 | unnamed, simulator receiver proven by tick wrapper | §5 generation, including age/source mutation and callbacks |
| `004BC230` / 191 | named simulator Init | §3 destructive reallocation/defaults/seed selection; allocation failure partial |
| `004BC310` / 160 | foreign parent constructor, simulator initialization inlined at `004BC346–004BC35E` | §2 zero initial fields only; remaining parent construction excluded |
| `004BC190` / 158 | foreign parent Shutdown, inline simulator cleanup `004BC1DA–004BC1F8` | §2 callback nulling and buffer release order; parent/service lifetime partial |
| `004BC440` / 85 | unnamed parent destruction helper | §2 buffer cleanup `004BC46A–004BC486`; vector/string/base destruction foreign |
| `004BC7D0` / 147 | parent OnKeyDown | §3 inline two-buffer clear; calls RandomizeCells for R; unrelated keys excluded |
| `004BC2F0` / 25 | parent adjusted-interface tick wrapper | §5 one clear-view call then one generation; always AL=1 on normal return |
| `004BBE50` / 23 | parent live-cell callback adapter | §5 argument forwarding to marked-cell view; drawing excluded |
| `004BC670` / 338 | parent Init | embedded receiver, dimensions from external provider, callback installation; other initialization foreign |
| `004BC870` / 138; `004BC900` / 117 | parent mouse-down/move | §4 mode selection/direct mutation call sites; coordinate picking and capture foreign |
| `004BC4D0` / 405 | parent RefreshMarkedCellView | §4 current-buffer nonzero read only; vector growth/drawing not specified |

Four distinct out-of-line simulator bodies above total **932 exported body bytes**. Ten distinct
boundary/parent bodies total **1,596 bytes**; these are inspected contexts, not additional simulator
coverage. Exported RandomizeCells ranges are `004BBE90–004BBEB7` and `004BBEC0–004BBEF5`;
the other three simulator bodies have contiguous ranges ending `004BBF4A`, `004BC18B`, `004BC2EE`.
Padding gaps are not body bytes. No folded/shared simulator entry has been established. Shared parent
QI/AddRef/Release and inherited vtable entries are excluded rather than credited to this owner.
This inventory does not prove absence of further inlined, folded, or as-yet unattributed operations.

## 2. Storage, construction and lifetime

**[compat]** Offsets are relative to the embedded simulator; cells are unsigned bytes, one per position.
There is no bit packing or row padding: offset **y × width + x**. Width/height and index arithmetic
are 32-bit. Ordinary rules below assume distinct valid buffers covering the full mathematical product,
positive dimensions, representable allocation/index arithmetic, and no destructive/reentrant callbacks.

| Offset | Observed field |
|---|---|
| +0 | byte suppressing generation when nonzero |
| +4 | 32-bit direct-edit mode: 0 adds, 1 erases, others leave cells alone |
| +8, +12 | width and height (unsigned loop comparisons) |
| +16 | current/published buffer pointer, called **C** below |
| +20 | alternate buffer pointer, called **P** below |
| +24, +28 | live-result and dead-result callback pointers |
| +32 | callback context pointer |

Construction in the parent zeros the suppress byte and all listed 32-bit fields. Padding bytes between
the suppress byte and mode are not specified. There is no constructor seed pattern, generation counter,
timer field, or simulator-owned COM reference demonstrated here (`004BC346–004BC35E`).

**[compat]** Parent Shutdown, when its parent initialized flag is nonzero, zeros live callback, dead
callback and context, then frees C, sets C null, frees P, sets P null, before subsequent parent/service
cleanup (`004BC1DA–004BC20A`). Width, height, mode and suppress byte are not reset by that sequence.
If the parent initialized flag is zero it skips this cleanup. The parent destruction helper frees C and
P in the same order and nulls them, without first nulling callback fields (`004BC46A–004BC486`).
There is no simulator callback invocation, AddRef or Release in these buffer-cleanup sequences.
Null-free behavior and repeated cleanup depend on the foreign free primitive, not a proven owner guarantee.

## 3. Initialization, clearing and randomization

### 3.1 Destructive initialization

**[compat]** Init is a thiscall with three stack arguments: width word, height word, and a flag tested
as its low byte; normal return discards 12 stack bytes. It has no established success return channel.
Instructions `004BC24C–004BC274` distinguish the first two argument positions after stack adjustment.

Order (`004BC233–004BC29E`): free old C and null C; free old P and null P; assign dimensions, replacing
**each zero independently with 16**; allocate width×height bytes into P first, then allocate the same
size into C. The multiply supplies its low 32 bits to allocation. There is no owner maximum-size,
negative-input rejection, overflow check, rollback or early allocation-null failure result.

Nonzero seed flag calls RandomizeCells. Zero flag clears P then C, skipping each null pointer separately
(`004BC2AD–004BC2E8`). Each clear writes the low-32-bit product's full byte span, in increasing address
order (four-byte zero groups, then remainder). Dimensions, suppress byte, edit mode and callbacks otherwise
retain their prior values. Calling Init again always frees/replaces storage, even for identical dimensions.
Zero dimensions request defaults; they do not construct an empty normal grid. Large/high-bit dimensions
and allocation failures are partial, not a supported safe-false contract.

### 3.2 Clear without allocation

**[compat]** Parent key values `0x43`, `0x2E`, `0x08` select the inline clear path (`004BC7D4–004BC7E4`,
`004BC817–004BC852`): clear P then C, independently skipping null pointers, with the same product-sized
zero fill. Then call the foreign marked-cell-view ClearCells and normally return AL=1. It does not swap
buffers, change dimensions/mode/callbacks, or invoke per-cell callbacks. Repeated clear stays empty.
No standalone simulator ClearCells body is asserted.

### 3.3 Randomize both buffers

**[compat]** Every RandomizeCells invocation constructs a fresh local foreign RNG with 32-bit seed
`0xFFFFFFFF` (`004BBE97–004BBE9D`). The raw dependencies are **construct RNG(seed32)** and
**draw uint32 with bound32=2** (`00913A5C`, `009136B1`), not a host-supplied whole grid. Their seed
source, sequence and distribution internals remain foreign. For each y ascending, then x ascending,
perform exactly one draw, keep its low byte, write P[y×width+x] first, then C at that offset
(`004BBEC0–004BBEDC`). No null-buffer test or callback occurs. Row/column bounds are unsigned.
Parent R (`0x52`) calls this operation then RefreshMarkedCellView (`004BC7E6–004BC7F5`).

With the ordinary bound-2 primitive contract yielding 0 or 1, this overwrites all ages in both buffers
with 0/1. Repeated calls construct a new RNG; identical outputs are not guaranteed by the owner.
Even a zero-height internal state constructs the RNG before skipping draws; such a state exists before
Init, but is not a zero-dimension Init result. Foreign RNG failure and failed allocations stay partial.

## 4. Direct mutation and readout

**[compat]** The unnamed method at `004BBF00` takes x32,y32, discards eight argument bytes, and returns
a Boolean in **AL**. Full EAX is not its Boolean contract. It computes offset y×width+x without bounds
checks or wrapping. Valid coordinates and valid buffers are preconditions; the edge wrapping in §5
does not apply to edits.

| Mode | Writes and result, in order |
|---|---|
| 0 | Write C=1 unconditionally. If P was zero, write P=2 and return true; otherwise retain P and return false. |
| 1 | Write C=0 unconditionally. If P was nonzero, write P=0 and return true; otherwise retain zero and return false. |
| other 32-bit value | No cell writes; return false. |

Thus false does not mean no mutation: mode 0 may reset an aged C to 1 while P remains nonzero, and mode
1 may erase a nonzero C even when P was already zero (`004BBF1A–004BBF48`). There are no callbacks,
allocation, buffer swap, generation or counter changes. Repeated add preserves nonzero P but reassigns
C=1; repeated erase leaves both zero.

Parent mouse-down reads C's selected byte, sets mode to its nonzero test (0 or 1), then calls this method
on parent+0x40 (`004BC8C8–004BC8E3`). Mouse move reuses that mode and suppresses repeated coordinates
at the parent boundary (`004BC94A–004BC95F`). Picking, coordinate conversion, bounds validation, capture
and rendering are not supplied by this simulator contract. Parent RefreshMarkedCellView saves C and
scans its nonzero bytes (`004BC4D6`, `004BC540–004BC547`); it is not a read of P or a Boolean-only
storage format. No standalone public GetCell method is established.

## 5. Generation, edge policy and publication

### 5.1 Entry, visit order and neighbor reads

**[compat]** The unnamed generation body takes no stack arguments and has no established result value.
If suppress byte +0 is nonzero it returns before swapping or visiting cells (`004BBF56–004BBF59`).
Otherwise it **swaps C and P before the first visit** (`004BBF5F–004BBF6D`). After the swap, P is the
source (the previously published C) and C the destination. Height is checked only after the swap.
Visit y=0 upward, and for each row x=0 upward, using unsigned bounds (`004BC160–004BC17E`).
Current C therefore exposes a partially written destination during callbacks, not an atomic final
publication. Dimensions are reread by loops; arbitrary reentry/mutation of dimensions is not supported.

Every neighbor contributes **1 if its source byte is nonzero, otherwise 0**, regardless of age. All
eight directional samples are counted, including repeated samples of the same physical cell on tiny
grids. No set deduplication or center exclusion after coordinate wrapping is performed.

For interior cells, sample order is SW, SE, NW, NE, W, E, N, S (`004BBFD0–004BC024`). Here increasing
y is S. For x=0, y=0, x=width−1 or y=height−1, use wrapped coordinates and sample in order
S, N, SE, NE, SW, NW, E, W (`004BC02B–004BC102`). Each negative-direction coordinate is
`(coordinate + dimension − 1) mod dimension`; each positive direction is
`(coordinate + dimension + 1) mod dimension`, using 32-bit sums and unsigned division. On the ordinary
non-overflowing domain this is a torus. High-dimension overflow is not promoted to a normal geometry rule.

### 5.2 Transition and source-buffer side effects

Let n be the eight-sample count and s the source byte for this cell **at its visit**. The following
table states the writes, including writes to P; these side effects distinguish the native contract
from a pure Boolean next-generation calculation (`004BC104–004BC14A`).

| Condition | Destination C | Source P | Callback selection |
|---|---|---|---|
| n=0,1,4,5,6,7,8 | 0 | retain s | dead-result |
| n=3 and s=0 | 1 | retain 0 | live-result |
| n=2 and s=0 | 0 | retain 0 | dead-result |
| n=2 or 3 and s=191 (`0xBF`) | 191 | retain 191 | live-result |
| n=2 or 3 and any other nonzero s | (s+1) modulo 256 | same incremented byte | live-result |

The final row selects the live callback even if an externally introduced s=255 increments to zero.
Ordinary seeding, clear, edits and updates keep reachable cell values in **0…191**. In that domain,
source writes preserve occupancy, births never alter source occupancy, and deaths leave source occupancy
intact, so occupancy matches a synchronous B3/S23 update. Full byte state is not two immutable generations:
P holds incremented ages at surviving cells and retained old bytes at deaths. Arbitrary injected bytes
above 191 have the literal byte behavior in the table; especially 255 can alter later neighbor counts.
No ordinary owner operation shown here generates those bytes when the RNG returns 0/1.

### 5.3 Callbacks and the parent tick boundary

**[compat]** After each cell's stores, load the selected callback pointer. If nonnull, invoke it with
three 32-bit stack arguments **(x, y, context)**, caller cleaning 12 bytes; ignore its result
(`004BC14D–004BC15D`). Callback fields/context are read per visit. Callbacks represent resulting live/dead
status, not only changes: an unchanged dead cell still selects dead-result, and age 191 still selects
live-result. The owner takes no reference on callback/context and has no in-progress guard set by this body.
No timer read, RNG draw, allocation or generation counter update occurs in this generation body.

Parent Init sets live callback to `004BBE50`, dead callback null and context to the parent
(`004BC78D–004BC79B`). That adapter forwards x,y to a marked-cell-view operation at `007DCD30`, using
parent+0x64 as receiver (`004BBE50–004BBE66`); drawing and callback internals are foreign.
The adjusted tick receiver is parent+0x28: wrapper `004BC2F0` first calls marked-cell-view ClearCells,
then passes receiver+0x18 (parent+0x40) to generation, then returns AL=1. Its one stack argument is not
used for generation timing. Thus even a suppressed generation still follows the wrapper's view-clear
call. Scheduling cadence belongs to the separate manager, not this owner.

## 6. Independent literal vectors and effect traces

All vectors are **PROPOSED / UNEXECUTED**, derived from the instructions above before any implementation
output. Rows below list decimal byte values from y=0 onward; x increases left to right. Unless noted,
buffers are valid/disjoint, suppress=0, callbacks absent, and ordinary arithmetic applies. Explicit raw
RNG outputs are primitive inputs, not claims of a reachable native seed sequence. Direct generation
states are conditional stage vectors; §6.3 supplies an owner-operation construction of its old grid.

### 6.1 Allocation, seeding and repeated initialization

- **V1 — defaults/order:** Init(0,1,0) frees C then P, allocates P(16 bytes) then C(16 bytes), clears P
  then C. Result dimensions 16×1, both buffers sixteen zeros. Init(1,0,0) independently yields 1×16;
  Init(0,0,0) yields 16×16. No empty grid is implied.
- **V2 — random rectangular grid:** Init(3,2,1), supplied draws `1,0,1,0,0,1`, produces C=P rows
  `1 0 1 / 0 0 1`. Effect order: frees; P allocation(6); C allocation(6); RNG construct(FFFFFFFF);
  at coordinates `(0,0),(1,0),(2,0),(0,1),(1,1),(2,1)`, draw(bound=2), write P byte, write C byte.
  Re-randomize with draws `0,1,0,1,1,0`: both become `0 1 0 / 1 1 0`, regardless of prior ages.
- **V3 — reinit destroys edits/ages:** after any advance and direct edit, Init(3,2,0) frees the currently
  published C first (which may be a different allocation after a swap), then P; new P/C are six zeros.
  Existing mode and callback pointers retain their values; no per-cell callbacks occur.
- **V4 — partial allocation trace, not safe success:** supply P allocation=null, C allocation=valid,
  zero seed flag. Owner skips the P clear and zeros C; no explicit failure Boolean is produced.
  This does not authorize generation or randomization on the resulting null P. A nonzero seed flag
  instead reaches unchecked stores, with no safe failure output established.

### 6.2 Edits distinguish current and alternate state

- **V5:** valid selected bytes C=9,P=0, mode=0: writes C=1,P=2, AL=true. Repeat without changing mode:
  C=1,P=2, AL=false. With C=9,P=7, mode=0: C becomes 1, P stays 7, AL=false.
- **V6:** C=9,P=0, mode=1: C becomes 0, P remains 0, AL=false. C=0,P=7, mode=1: both zero,
  AL=true. Mode=2 with C=9,P=7: unchanged, AL=false. No callbacks for any of these edits.

### 6.3 Full-grid swap, edges, synchronous occupancy and two advances

**V7:** a 4×4 grid has C rows `1 1 1 0 / 0 0 0 0 / 0 0 0 0 / 0 0 0 0`; P may initially contain
any valid bytes. This asymmetric edge placement is derived here, not an imported textbook golden.
It can be constructed through Init(4,4,0), mode=0, edits (0,0),(1,0),(2,0). Those edits leave P's
top row `2 2 2 0`, demonstrating that initial P need not equal C.

First generation neighbor counts (source C at entry):

| y | n(x=0…3) | Published C after first generation |
|---|---|---|
| 0 | 1 2 1 2 | 0 2 0 0 |
| 1 | 2 3 2 2 | 0 1 0 0 |
| 2 | 0 0 0 0 | 0 0 0 0 |
| 3 | 2 3 2 2 | 0 1 0 0 |

If original allocation C=A and P=B, the first result is **C=B**, P=A; A's top row is `1 2 1 0`,
other rows zero (births are absent from A). The first visited cell dies, but its source byte remains
1, so subsequent counts still see it. A naive single-buffer, row-major occupancy overwrite would
erase the first three cells before reaching the births and yield an empty grid. A zero-outside edge
policy would miss the birth at (1,3). This vector distinguishes both errors.

Second generation publishes **C=A**: the wrapped vertical triple at y=3,0,1 returns a horizontal
triple across row 0. The exact result is
`1 3 1 0 / 0 0 0 0 / 0 0 0 0 / 0 0 0 0`.
The alternate B retains `0 3 0 0 / 0 1 0 0 / 0 0 0 0 / 0 1 0 0` after source aging.
The literal result above, rather than a Boolean grid alone, checks age and allocation publication.

### 6.4 Tiny wrapped domains and local transition boundaries

- **V8 — 1×1:** C=[1] samples itself eight times, n=8, publishes [0], source retains [1]. Next
  generation has n=0 and publishes [0]; both allocations now zero. C=[0] stays zero. Init(1,1,0)
  and a mode-0 edit construct the nonzero starting state normally.
- **V9 — 2×1 rectangle:** source C=`1 0`: at x=0, N/S are the same live cell twice, E/W and four
  diagonals are the zero cell, n=2, so output 2. At x=1, E/W and four diagonals sample x=0 six
  times, n=6, so output 0. Published C=`2 0`, P=`2 0`. Distinct-neighbor deduplication would differ.
- **V10 — transition stage inputs:** for n=2 or 3, source bytes `1,190,191,192,254,255` yield destination
  bytes `2,191,191,193,255,0`, respectively, and equal source writes except the retained 191.
  All select live callback, including 255→0. For s=0: n=2 gives 0/dead; n=3 gives 1/live with source
  still zero. For s=7 and n=`0,1,4,8`, destination=0/dead and source retains 7. These are conditional
  per-visit inputs, not proof that the high-byte states are produced by ordinary owner calls.
- **V11 — corner sample trace:** in V7 at (0,0), wrapped S,N,SE,NE,SW,NW,E,W visit
  `(0,1),(0,3),(1,1),(1,3),(3,1),(3,3),(1,0),(3,0)` with occupancy `0,0,0,0,0,0,1,0` (n=1).
  Interior (1,1) visits SW,SE,NW,NE,W,E,N,S = `(0,2),(2,2),(0,0),(2,0),(0,1),(2,1),(1,0),(1,2)`
  with occupancy `0,0,1,1,0,0,1,0` (n=3).

### 6.5 Callback, suppression and clear traces

- **V12:** V7 with nonnull live L, dead D, context K produces exactly 16 calls, after each destination
  store, at `(0,0)…(3,0),(0,1)…(3,1),(0,2)…(3,2),(0,3)…(3,3)`. Selections by row are
  `D L D D / D L D D / D D D D / D L D D`; each call receives `(x,y,K)`. All-zero 2×1 produces
  D(0,0,K), D(1,0,K), despite no occupancy changes. Null selected callback means no invocation.
- **V13:** suppress=1 with C=A,P=B: direct generation leaves pointers and bytes unchanged and calls
  nothing. The parent tick wrapper still clears marked-cell view first and returns AL=1. With suppress=0,
  it clears view then runs exactly one generation; no elapsed-time argument selects multiple generations.
- **V14:** after V7's first advance, mode-0 edit (3,2) sets current B's byte to 1 and alternate A's zero
  byte to 2. Parent clear-key path then clears A followed by B, keeps C=B/P=A pointer roles, and clears
  the marked-cell view once. A second clear repeats zero writes with the same roles and no cell callbacks.
- **V15 — cleanup stage trace:** with valid C=A/P=B and callbacks L,D,K, admitted parent Shutdown nulls
  L,D,K, frees A, nulls C, frees B, nulls P. Destruction helper afterward passes null twice to the foreign
  free primitive. This records call order, not an assumption that the primitive tolerates null or that
  every parent lifetime admits repeated Shutdown.

## 7. Limits and open questions

Supported normal behavior is the storage/update/edit contract above under valid arithmetic, buffers,
primitive returns and non-destructive callbacks. Foreign allocator and RNG internals, view/drawing,
parent COM/base lifetime, service scheduling, coordinate picking and renderer/region logic are excluded.
Normal-operation dimensions 1 and rectangular grids are supported by the actual loops; zero means 16
through Init. No maximum-bound normal domain or negative/high-bit safety guarantee has been proved.
No automatic whole-owner, native/default, coverage, ABI-table or live-query promotion follows.

[GL1–GL3](questions/game-of-life.md) delimit exceptional allocation/RNG inputs, reentry, and residual
attribution/interface reachability. **[choice]** An implementation may use different internal storage
if it preserves the supported byte state, alternate-buffer effects, publication and callback ordering.
It must not silently replace that observable state with a Boolean-only synchronous grid.
