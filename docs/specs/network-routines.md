# Network placement demolition region

## 1. Scope and evidence

This is the standalone contract of `cSC4NetworkRoutines::ComputeDemolitionRegionForPlacementCells`
(`0x0061F7C0`, 1,004 bytes) and its temporary `cSC4DemolishByNetworksOccupantFilter` predicate
(`0x0061F590`, 56 bytes). It replaces the abbreviated account in [network-occupants.md](network-occupants.md)
§9. The intersection caller is [network-rules.md](network-rules.md) §10.5; the foreign demolition operation is
[demolition.md](demolition.md) §4.4. This publishes no network model, underground-record, neighbour or evaluator
endpoint behavior.

Evidence is static instruction study of the Windows export, checked against the working original executable on
2026-10-23. Export identity: `program=SimCity 4.exe`, image base `0x00400000`, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`. The independently hashed
`bin/SimCity4.exe` matches exactly; its file version is **1.1.641.0**, original filename `SimCity 4.exe`.
The export assigns the helper to `cSC4NetworkRoutines` with medium-confidence name attribution; the instruction
behavior below does not depend on that name. The local decompilation was supporting evidence only. No original
execution was performed. The earlier REA query returned exactly `target_unavailable` / `No app is open`;
export identity does not establish or repair a live REA target.

All examples in §8 are **proposed conformance vectors derived from instructions**, not measured executions.
Addresses below identify evidence, not replacement implementation bindings.

## 2. Typed boundary and domain

The compiled entry is a **cdecl-style five-stack-argument function**, with no receiver consumed from ECX;
plain return, caller stack cleanup, boolean result in **AL**. The logical parameters, in stack order, are:

| Argument | Type and direction | Required behavior |
|---|---|---|
| 1, entry stack `+4` | borrowed reference to a contiguous sequence of cell pairs | Header `+0` is begin, `+4` is end; records are 8 bytes, x then z, each 32 bits. No separate count argument. |
| 2, `+8` | `SC4CellRegion<long>** outRegion` | Receives a newly allocated region when the construction path is taken. The helper does not destroy the previous value. |
| 3, `+12` | `int64* outCost` | Required writable storage on the final-call path; cleared there and passed to demolition. Untouched by earlier exits. |
| 4, `+16` | optional pointer to an occupant set, `demolished` | Forwarded unchanged to demolition argument 8; the helper itself does not initialize or clear the set. |
| 5, `+20` | optional borrowed `cISC4OccupantFilter* placementFilter` | Used only in the first-occupant lookups of §4, not as the final demolition filter. |

Entry loads and record stepping: `0x0061F7DA–0x0061F80D`, `0x0061F980–0x0061F98D`.
Output accesses and final forwarding: `0x0061F9F5–0x0061F9F9`, `0x0061FA5D–0x0061FA77`,
`0x0061FB25–0x0061FB87`. Returns: `0x0061F9A6–0x0061F9AF`, `0x0061FB1B–0x0061FB24`,
`0x0061FBA8–0x0061FBB1`. Three callers push five arguments (`0x00609DC3–0x00609DDE`,
`0x00629C3A–0x00629C53`, `0x0062D16B–0x0062D189`). Their larger subsequent stack adjustments also discard
previously outstanding stack operands; they do not imply a sixth helper argument.

The sequence's logical count is `(end − begin) / 8` for a well-formed contiguous range. The helper uses equality
with end and advances by 8; it does not read capacity or validate the header/range. Duplicate records are retained
in their original order. No sorting, unique operation or maximum-count check occurs.

The previous summary's “u32 pairs” describes storage only: comparisons for coordinates, bounds and lot walks
are **signed 32-bit**, not unsigned (`0x0061F819–0x0061F842`, `0x0061F87B–0x0061F8A7`,
`0x0061FA30–0x0061FA46`, `0x0061FAD3–0x0061FAE5`). An implementation can represent each record as two
32-bit words, interpreting their bit patterns as signed coordinates for these operations. The original source
typedef/template spelling is not established by these instructions.

The ordinary supported domain is valid in-city standard cells, valid lot rectangles, initialized services,
well-formed objects/ranges, successful required allocations and nonaliasing writable outputs. Standard cells are
16 m; ordinary city widths are 64, 128 or 256 cells ([occupant-manager.md](occupant-manager.md) §2.1).
The helper does **not** enforce this domain. It performs no city clipping, no range/pointer validation and no
overflow check. All additions, increments, subtractions and allocation-size shifts are 32-bit operations;
overflow wraps at the instruction level before subsequent signed comparisons. Do not translate malformed,
null, overflowing or out-of-city requests into a claimed native `false` result. See §7 and NRT1.

## 3. Bounds, lot records and zero bitmap

Read the sequence's begin/end and initialize bounds to `xmin = zmin = 0x7FFFFFFF`, `xhigh = zhigh = 0`.
Acquire `city.GetLotManager()` (city slot 33, `+0x84`) through global city `0x00B43CEC`.
The getter runs even for an empty sequence (`0x0061F803`), after those header reads and before testing
begin=end. For each input pair, in input order:

1. Lower xmin/zmin if x/z is smaller. If x is at least xhigh, replace xhigh by x+1; analogously for zhigh.
2. Ask `lotManager.GetLot(x, z, false)` (slot 6, `+0x18`, `0x0061F844–0x0061F852`).
3. For a non-null lot, ask `lot.GetBoundingRect(outRect)` (slot 14, `+0x38`, `0x0061F85D–0x0061F866`).
   Its returned rect has inclusive endpoints. Increment both high coordinates by one, producing a temporary
   **half-open** lot rect `[lx0,lx1) × [lz0,lz1)`. Merge its lows and incremented highs into the accumulated
   bounds, and append this rect to the temporary rect sequence.

Only non-null lots append rect records. An input without a lot still enlarges the accumulated bounds but does
not append a one-cell rect. Multiple inputs on one lot append the lot rect multiple times, including duplicates
of the same input. No lot identity set or transitive lot expansion exists. Cells reached through a lot extension
do not themselves cause further lot lookups (`0x0061F869–0x0061F97C`).

If begin equals end, return true without writing region, cost or set. After a nonempty input walk, the only
construction guard is signed `xhigh >= xmin`; failure frees the temporary rect sequence and returns true,
again without output writes. There is no corresponding z guard (`0x0061F993–0x0061F9AF`). For ordinary
nonnegative coordinates the construction guard passes.

Allocate **0x24 bytes** and store the accumulated four bounds in `outRegion`'s object, in the order
`xmin,zmin,xhigh,zhigh`. Its bitmap is at object `+0x10`, with dimensions
**width = xhigh − xmin + 1**, **height = zhigh − zmin + 1**, filled with false. Publish the object pointer
to `*outRegion` before scanning any lot cells (`0x0061F9B0–0x0061F9F9`).

The stored region rectangle is consumed **inclusively** by demolition. Consequently xhigh/zhigh are a clear
extra border one cell above the largest input/lot coordinates on an ordinary request. Do not silently tighten
the stored rectangle or remove this border. The half-open rects used for the zone scan exclude that border.
There is no bitmap union of input cells or lot cells at construction: **every bit starts clear**.

The bitmap has an array of width column pointers at region `+0x20`. Each column contains
`ceil(height/32)` 32-bit words; bit z is least-significant-bit first. Width/height are at region `+0x14/+0x18`;
words-per-column at `+0x1C`. Constructor `0x004422A0–0x00442311` allocates these arrays using unsigned
dimensions; fill `0x00441AA0–0x00441ADE` zeros every allocated word, including padding bits.

## 4. Zone scan, lookup polarity and failure mask

Scan the appended lot rects in **append order**, x increasing outermost, z increasing innermost. Each rect is
half-open. There is no deduplication of repeated rects or overlapping cells. For each visited cell:

1. Read `zoneManager.GetZoneType(x,z)` (global `0x00B43D14`, slot 12, `+0x30`). Zero means skip the lookup,
   bit write and type-mask write for this cell.
2. For nonzero zone, initialize a local occupant pointer to null and call
   `occupantManager.GetFirstOccupantByStandardCityCell(&occupant,x,z,placementFilter)`
   (global `0x00B43D0C`, slot 32, `+0x80`).
3. **False (no matching occupant)** sets the region bit for this cell, and ORs the type-mask word with
   `1 << (zone & 31)`. **True** sets neither. Existing bits are never cleared.
4. Release the local occupant if non-null, irrespective of the lookup's result. Continue the full rect walk.

Primary loop and writes: `0x0061FA30–0x0061FAFB`; lookup argument initialization/call `0x0061FA5D–0x0061FA77`;
AL test `0x0061FA7D`; bit ORs `0x0061FA81–0x0061FABB`; release `0x0061FABF–0x0061FAC9`.
For offsets `dx = x − xmin`, `dz = z − zmin`, the set operation addresses column `dx`, word `dz >> 5`
(logical unsigned shift), bit `dz & 31`. The zone shift also uses the x86 low-five-bit shift count.

Thus a set bit means **a zoned lot cell lacking an occupant accepted by placementFilter**, selected for the
demolition preview. It does not mean “protected/excluded cell,” “all placement cells,” or “all cells in the
bounding box.” Sparse gaps and unzoned lot cells stay clear. With stable services, the bitmap is the union of
those qualifying cells across the appended lot rects. Repeated visits still repeat the foreign queries and
releases; even if a later visit succeeds, an earlier set bit/type bit remains set.

After **all** rects, test `typeMask & 0xFC00`. Nonzero returns false, before clearing cost, constructing the
temporary filter or calling demolition (`0x0061FB05–0x0061FB24`). Ordinary zone types 10–15 contribute
these bits, but merely encountering such a zone is insufficient: its lookup must have failed. With nonstandard
zone values the instruction tests the corresponding bit modulo 32. The actual zone getter sign-extends its
stored byte (`0x004DC740–0x004DC75C`); ordinary types are 0–15 ([zones.md](zones.md) §2).

### 4.1 Typed-call reconciliation

`0x00B43D0C` is the **occupant manager**, not the demolition utility. `SL::OnCityInit`
(`0x00601C87–0x00601C95`) obtains it from city `+0x88`; that getter reads city `+0x12C`
(`0x00423710`). The original primary occupant-manager vtable `0x00A8F7F0`, slot 32 at `0x00A8F870`,
contains `0x0049E690`; adjacent slot 33 contains typed thunk `0x0049DCE0`. These pointer values were read
from the matching executable. The helper passes a filter pointer, not an occupant type or privilege.

The primitive contract is [occupant-manager.md](occupant-manager.md) §7: first accepted occupant with strict
x/z box overlap with the shrunken standard-cell box; caller filter applied before geometry; null filter allows
any occupant. Success writes an AddRef'd occupant and returns true; miss leaves the output alone. Instructions
`0x0049E76D–0x0049E77C` test the optional filter, `0x0049E78F–0x0049E7C9` test overlap,
`0x0049E7D3–0x0049E7E0` write/AddRef the hit. This is **not** a `CanDemolish` or a privilege test.
Keep its internals with the foreign owner; the network routine owns the loop, mask and decision above.

Likewise the lot query is lot-manager slot 6 (`0x0072B410`, three arguments, pop 12), not city slot 6.
It rejects out-of-grid x/z before the lookup and with `false` skips the containment recheck
(`0x0072B410–0x0072B46F`). Lot slot 14 `0x006BF320–0x006BF33F` copies four zero-extended byte bounds.
No guessed exported C++ method name is needed to select these bindings.

## 5. Owned temporary filter and composition

Only after passing the type-mask test, clear the two 32-bit halves of `*outCost`, allocate a **12-byte**
filter and initialize it. Its only state is primary vptr `0x00AA825C` at +0, serializable vptr
`0x00AA8244` at +4, and unsigned reference count at +8, initially 0, AddRef'd to 1
(`0x0061FB25–0x0061FB64`, constructor `0x005EA080–0x005EA09D`). There is **no stored placementFilter,
zone mask, bounds, type configuration or other predicate state**.

The matching executable's primary table contains QI `0x005E95D0`, AddRef `0x004BDF00`, Release
`0x007CB920`, then the following predicates; slot 6 is deleting destructor `0x006C4A00`:

| Slot | Method | Rule |
|---|---|---|
| 3, `+0x0C` | `IsOccupantIncluded(o)`, `0x0061F590` | Initialize QI output null; call `o.QueryInterface(0xA9764C8C,&out)`. Return true iff its **AL is zero**; release out when non-null before returning. |
| 4, `+0x10` | `IsOccupantTypeIncluded(type)`, `0x004FC900` | Always true; does not inspect type. |
| 5, `+0x14` | `IsPropertyHolderIncluded(holder)`, `0x004FC900` | Always true; does not inspect holder. |

Predicate proof: `0x0061F591–0x0061F5C5` supplies both QI arguments, converts the boolean to its complement,
and releases the output. It rejects by **QI result**, not pointer non-nullness or occupant type. It has no
null-occupant test. `0x004FC900–0x004FC904` returns true with pop 4 despite an unrelated folded export name.
The family QI marker `0xA9764C8C` is accepted by the network occupants listed in
[network-occupants.md](network-occupants.md) §2; use the literal QI rule rather than a type-list approximation.

The filter's QI accepts IID 1 and `0x46A80026` at +0, serializable IID `0xE4FDA3D4` at +4, AddRef'ing a
successful result (`0x005E95D0–0x005E9622`). AddRef increments +8; Release decrements when >1,
otherwise calls the deleting destructor with flag 1 and returns 0 (`0x004BDF00–0x004BDF07`,
`0x007CB920–0x007CB936`). The destructor runs the base destructor and frees from the global heap
(`0x006C4A00–0x006C4A1B`). No custom filter configuration occurs between construction and final call.

The two filters do **not** form a direct conjunction: placementFilter decides which zoned lot cells enter
the bitmap; the owned filter is passed as demolition's callerFilter and controls occupants considered there.
Demolition composes it with its own demolition filter Φ ([demolition.md](demolition.md) §4.2), including
realism, `CanRemove(2)` for privilege 1 and its permitted occupant types. Flag `0x2000` keeps the view filter
out of Φ and suppresses loose occupants on lot cells, but the view filter still gates empty-lot qualification.
The helper never AddRef's or Releases placementFilter; it borrows it for synchronous occupant queries.
Demolition temporarily holds a reference to the owned filter and clears/releases that reference at completion;
the helper then Releases its own reference (`0x0061FB8A–0x0061FB94`).

## 6. Exact final call and outputs

The call through demolition global `0x00B43D00`, slot 6 (`+0x18`, implementation `0x00468410`) has **eleven**
arguments, proven by the pushes at `0x0061FB70–0x0061FB87` and the typed interface in
[demolition.md](demolition.md) §8:

| Position | Parameter | Value |
|---|---|---|
| 1 | `bool doIt` | false |
| 2 | `const SC4CellRegion<long>& R` | `**outRegion`, passed as object pointer |
| 3 | privilege | 1 (player removal permission, `CanRemove(2)`) |
| 4 | `uint32 flags` | `0x2001` |
| 5 | `bool dezone` | false |
| 6 | `cISC4OccupantFilter* callerFilter` | the temporary filter from §5 (or null on its allocation-failure branch) |
| 7 | `int64* cost` | outCost |
| 8 | occupant-set pointer `demolished` | helper argument 4, unchanged; may be null |
| 9 | `cISC4Occupant* clicked` | null |
| 10 | signed 32-bit screen x | 0 |
| 11 | signed 32-bit screen y | 0 |

This is a **preview**: `doIt=false` suppresses actual removals, lot demolition/dezoning, network-removal
batching and effects. Bit 0 of flags does not override false doIt. No funds are charged. Nevertheless it is not
side-effect-free: it builds/uses the foreign owner's scratch collections and filter state, computes the cost,
and inserts successfully priced occupants into the optional output set even in preview. Set order, references,
lot qualification, cost formulas and the demolition result belong to demolition §4.4, not to this owner.

Demolition examines only **set** bitmap cells, in inclusive stored-rect x-major order; bounds/bitmap checks
`0x004685BB–0x0046861E` prove this polarity. The selected-cell lot query follows at `0x00468624–0x0046862E`.
Its result is not an “any occupant failed” aggregate: in particular a failed per-occupant price call does not
itself set the overall result false (demolition §4.4 step 5).

The helper saves demolition's AL across filter release/temporary-vector cleanup and returns that byte
unchanged (`0x0061FB8C`, `0x0061FBAB`). It does not recompute a bool from cost, mask emptiness or set size.
The region stays published even when demolition returns false. The helper initializes cost to 0 immediately
before filter construction; demolition writes its computed signed int64 total on completion. Cost is neither
added to the caller's old value nor cleared on the earlier `0xFC00` rejection.

## 7. Exit and lifetime table; limits

| Exit | Return AL | outRegion | outCost / demolished | Cleanup by this helper |
|---|---|---|---|---|
| empty input | 1 | unchanged | unchanged | no region/filter; lot-manager getter has already run |
| nonempty but signed xhigh < xmin | 1 | unchanged | unchanged | free temporary rect storage if present |
| `typeMask & 0xFC00` nonzero | 0 | newly published region retained | unchanged | release each lookup result as visited; free temporary rect storage |
| final demolition completes | demolition AL | newly published region retained | zero-then-demolition cost; optional set passed through | Release owned filter if non-null; free temporary rect storage |

The output region is caller-owned on both construction exits, including false. The helper does not free a
prior `*outRegion`; callers arrange an empty slot/free their previous region. It does not transfer a reference
to any queried occupant: the local reference is released immediately after each lookup. Rect storage is
temporary value data, freed at `0x0061F99E`, `0x0061FB13` or `0x0061FBA0` as applicable.

No normal exception/error cleanup is proved for invalid objects, arithmetic overflow or required allocation
failure. Region allocation has an explicit null branch that publishes null (`0x0061F9F3–0x0061F9F9`), but
subsequent bitmap/final demolition code can dereference it; it is **not** a clean false return. Failed temporary
filter allocation explicitly forwards null to demolition (`0x0061FBB2–0x0061FBB4`), without enforcing the
QI exclusion. Required bitmap/vector allocation failure is not a supported semantic alternative here.
Foreign services and relevant outputs are dereferenced without helper-level null checks.

NRT1 in [questions/network-routines.md](questions/network-routines.md) records the remaining domain/failure
guarantees not established here. An implementation should expose its own validation policy separately; it
must not present that policy as observed original behavior. No unresolved region, mask or filter algorithm is
delegated to a whole-helper host callback by this contract.

## 8. Proposed literal vectors (static deductions)

Use initialized services and successful allocations. Unless stated otherwise, all unmentioned zones are 0,
all lots have the stated inclusive rect, outputs start with sentinels, and the final demolition host records its
arguments and returns a specified bool. These test the helper independently of foreign cost formulas.

1. **Empty:** begin=end, outRegion=`old`, cost=91. Expect one GetLotManager, AL=1, region=`old`, cost=91,
   no lot/zone/occupant lookups and no demolition call.
2. **Sparse input without lots:** `[(2,3),(7,5)]`, both lot queries null. Stored rect `(2,3,8,6)`, bitmap
   width 7, height 4, every word 0. No zone lookups (no lot records). Final call occurs with cost initially 0
   and these bounds despite empty mask; if it returns false, helper AL=false and region remains available.
   Neither the two input bits nor gap cells are set automatically.
3. **Whole-lot extension:** input `[(5,6)]` finds lot rect `(4,5,6,7)`. Stored rect `(4,5,7,8)`, bitmap 4×4.
   Zone visits are `(4,5),(4,6),(4,7),(5,5),(5,6),(5,7),(6,5),(6,6),(6,7)`. If only `(4,5)` has zone 1
   with a missing accepted occupant and `(6,7)` has zone 2 with a found occupant, only column 0 word 0 = 1;
   typeMask=2. No queries at the clear border x=7 or z=8. The added cell `(4,5)` did not trigger a new lot lookup.
4. **Duplicates:** `[(5,6),(5,6)]` returns that same lot twice. Two lot/rect calls, then the nine-cell walk in
   vector 3 repeated twice. Stable answers produce the same bitmap, but repeat every zone/nonzero-zone lookup.
   If zone-1 lookup at `(4,5)` first misses then hits, its bit and typeMask bit 1 remain set. Any returned
   occupant is released on each visit that returns it.
5. **Blocked-zone mask and ordering:** inputs `[(1,1),(4,4)]`, one-cell lot rects at those coordinates.
   Zone 10 misses at `(1,1)`; zone 2 misses at `(4,4)`. Both lot queries precede both zone walks; the second
   walk still happens after the first high-zone failure. Stored rect `(1,1,5,5)`, bitmap 5×5: column 0 word 0
   =1, column 3 word 0 =8. typeMask=`0x404`; `&0xFC00=0x400`, so AL=0, region retained, old cost=91 and
   old output set unchanged; no temporary filter or demolition. If the zone-10 lookup instead hits, only
   typeMask=4 and the final preview is reached. “Any zone 10 lot fails” is therefore incorrect.
6. **Word boundary and gap:** `[(2,31),(2,33)]` each finds its own one-cell lot. Both zones are 1, both miss.
   Stored rect `(2,31,3,34)`, bitmap 2×4: column 0 word 0 =5 (relative z=0 and 2), column 1 word 0 =0.
   Absolute z does not select a word; relative z does. For a lot rect `(2,0,2,33)` and only z=31,32 missing,
   bitmap height=35, column 0 words are `0x80000000,0x00000001`; clear high border z=34 remains allocated.
7. **Filter composition:** one-cell lot at `(3,3)`, zone 1. Only occupant N overlaps and placementFilter
   accepts N: lookup succeeds, mask stays empty. If placementFilter rejects N, lookup misses and sets the bit.
   In that second case the final filter tests QI independently: N answering `0xA9764C8C` is rejected; another
   occupant P failing that QI is accepted by the temporary filter even if placementFilter would reject P.
   P must still pass demolition Φ and lot qualification. QI success with a null output still rejects; QI
   failure with a non-null output still accepts and Releases that output. No placementFilter AddRef/Release
   is introduced by the helper.
8. **Final failure order:** zone 1 misses, so no high-zone bit. Old cost=91 becomes 0 before filter allocation.
   Let demolition write cost=37, append P to the supplied set, return false. Expect AL=0, cost=37, appended P
   and region retained, filter released and temporary rects freed. This differs from vector 5's earlier false:
   that path never clears cost or calls demolition. A true result with cost=0 is also passed through as true.

## 9. Exact body and foreign-primitive inventory

| Body/operation | Evidence | Ownership in this contract |
|---|---|---|
| Five-argument helper, 1,004 bytes | `0x0061F7C0–0x0061FA28`, `0x0061FA30–0x0061FA3C`, `0x0061FA40–0x0061FBB5` | Owned bounds, rect accumulation, bitmap selection, ordering, type-mask test, outputs and final-call orchestration |
| Temporary filter predicate, 56 bytes | `0x0061F590–0x0061F5C7` | Owned literal QI exclusion; no configurable predicate state |
| Rect vector copy, 63 bytes | `0x00637E20–0x00637E5E` | Copies 16-byte rect values in order; support primitive, not another gameplay owner |
| Temporary rect allocation/free | `0x0090CF54`, `0x0090CF63` | Heap wrappers through `0x00B62698`; misleading export constructor/string-destructor names are not semantic calls |
| Region/filter global allocation | `0x005E55E0` | Foreign allocator; region and filter sizes specified above |
| Bitmap construction/fill | `0x004422A0`, `0x00441AA0` | Foreign storage primitives; zero fill, dimensions and layout verified for this helper |
| Base filter initialization/QI/refcount/destruction | `0x005EA080`, `0x005E95D0`, `0x004BDF00`, `0x007CB920`, `0x006C4A00` | Shared support methods; actual table pointers and instructions override misleading folded names |
| Always-true type/holder methods | `0x004FC900` | Shared support body, both filter slots |
| City GetLotManager | slot 33 `+0x84`, `0x00472E70` | Borrowed foreign service; no helper Release |
| LotManager GetLot | slot 6 `+0x18`, `0x0072B410` | Borrowed lot lookup, false containment flag |
| Lot GetBoundingRect | slot 14 `+0x38`, `0x006BF320` | Four inclusive endpoints; no lot AddRef/Release by helper |
| ZoneManager GetZoneType | slot 12 `+0x30`, `0x004DC740` | Foreign zone byte read, sign-extended |
| OccupantManager first filtered standard-cell query | slot 32 `+0x80`, `0x0049E690` | Foreign filtered geometric lookup; return and output reference consumed as §4 |
| Occupant QI / Release | slots 0 / 2 | Foreign polymorphic operations, used by the owned predicate and scan cleanup |
| DemolishRegion | slot 6 `+0x18`, `0x00468410` | Foreign eleven-argument preview, cost/set/results per demolition §4.4 |

Only three direct callers were exported: `cIntRule::Test` call `0x00609DDE`, tool
`UpdateOccupantHighlights` call `0x00629C53`, and `InsertIsolatedHighwayIntersection` call `0x0062D189`.
The latter two pass the tool's region slot `+0x210`, cost `+0x220` and placement filter `+0x364` (highlight
call setup `0x00629C03–0x00629C53`, isolated setup `0x0062D13E–0x0062D189`). Callers own disposal of
the published region and interpretation of the bool. No independent evaluator endpoint or additional full
network-owner coverage follows from this publication.
