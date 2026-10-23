# Coverage radius effect helper

## Status and scope

**Partial, instruction-backed specification, Windows Deluxe 1.1.641; 2026-10-23.**
This describes the selection, persistent occupant entries and effect-call ordering of
`cSC4CoverageRadiusEffectHelper`. It does not describe simulator coverage grids or
the implementation of an effect resource. Requirements marked *[compat]* apply
to the supported normal-return domain below. Storage choices marked *[choice]*
may differ in a portable implementation.

### Outline

- [Ownership and evidence](#ownership-and-evidence)
- [State and primitive boundaries](#state-and-primitive-boundaries)
- [Capacity and transfer order](#capacity-and-transfer-order)
- [Category lookup](#category-lookup)
- [Setting categories](#setting-categories)
- [Refreshing coverage](#refreshing-coverage)
- [Updating one occupant](#updating-one-occupant)
- [Reset and teardown evidence](#reset-and-teardown-evidence)
- [Proposed literal traces](#proposed-literal-traces)
- [Evidence ledger](#evidence-ledger)
- [Limits](#limits)

## Ownership and evidence

The canonical Windows export and its byte-bearing instruction queries establish
the following attribution. Decompiled text was navigation only: its apparent
two-argument update signature and several stack-derived values are wrong.
The working PE SHA-256 and `ghidra/export/windows/export-summary.txt` agree:
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
PE machine is `014C`, image base `00400000`, fixed-file version 1.1.641.0.
The five named entry instruction prefixes also match bytes read from that PE.
This is source identity evidence, not execution evidence.

| Entry | Finding |
|---|---|
| `007D1AB0`, 133 bytes | Named category lookup; independent leaf selection body. |
| `007D1D10`, 149 bytes | Named mask setter; store, stop traversal, then clear. |
| `007D1E40`, 664 bytes | Named per-occupant update; three stack inputs, returns with 12-byte argument cleanup. |
| `007D20E0`, 1532 bytes | Named whole-helper refresh; four service passes and stale-effect retirement. |
| `007D26E0`, 5 bytes | Unconditional jump to `007D20E0`; no second algorithm. |
| `007D1B40`, 97 bytes | Shared container clear/release body, also called by CitySituationManager-side sites `0052735D`, `00527886`. Not exclusively helper-owned. |
| `007D1BB0`, 138 bytes; `007D1C40`, 168 bytes | Unnamed insert and lookup/default-insert container operations reached by update. Their helper call-site consequences are covered; exclusive class ownership is unproved. |

The planning inventory “8 entries / 2886 bytes” is not a completion denominator.
The four substantial named bodies total 2478 bytes; the jump is an alias.
The unnamed constructor `007D26F0` is positively bound by the enclosing
constructor call `007F0EB4–007F0EC0`, rather than by adjacency. Stop/clear
`007D1DB0` and clear/free `007D1CF0` retain their bounded lifecycle roles below.
Existing PlaceLot/Query consumer behavior in
[city-tools-input.md](city-tools-input.md) remains consumer evidence, not proof
of these helper bodies. Direct export callers connect setter/refresh to those
consumers; update's four direct callers are the refresh passes.

## State and primitive boundaries

*[compat]* The helper stores a raw 32-bit selected mask and a map keyed by the
raw 32-bit occupant identity. Each entry contains an effect reference (possibly
null), a binary32 creation-time scalar and a one-byte visited marker. Identity
equality is exact; distinct occupants with identical properties remain distinct.
There is no category field in the entry key and no owned occupant-reference
acquisition in these map operations.

For describing native traversal only: nodes have next/key/effect/scalar/visited
at offsets 0/4/8/12/16; the helper has bucket begin/end at 4/8, count at 16 and
mask at 20. Lookup uses unsigned occupant identity modulo bucket count, then
the next chain. Traversal follows the current chain then increasing bucket
indices. *[choice]* A portable representation need not use these offsets, but
must preserve externally observable order when supplied a concrete traversal
fixture. Shared growth/rehash has the bounded transfer contract below;
do not promise sorted occupant order or insertion order across growth.

The following are primitive host boundaries, not permission to delegate the
helper's category decisions, passes or reference chronology:

| Symbolic primitive | Native receiver / operation | Raw result consumed here |
|---|---|---|
| Occupant property source | occupant slot `+18` | pointer used in two property reads |
| P membership | service at `00B43D98`, slot `+58(occupant)` | low-byte nonzero |
| F membership | service at `00B43D78`, slot `+1C(occupant)` | low-byte nonzero |
| Property read | `005FD480(source, property ID, output)` | low-byte status; output ignored by category lookup |
| Position object | service at `00B43D08`, slot `+38(occupant)` | pointer, null distinguished |
| Position read | position object slot `+3C(output triple)` | three raw 32-bit components; return ignored |
| Create named effect | service at `00B43D1C`, slot `+1C(name, output reference)` | low-byte status, output reference |
| Effect operations | effect slots `+20(payload)`, `+0C(0)`, `+14()`, `+10(0)` | only `+14` low-byte status is tested |
| Reference events | object slots `+04()`, `+08()` | calls observed; numeric return values ignored |

`+04` and `+08` are called retain/release below as event names. No numeric
reference-count result or destruction-at-zero policy is required by this spec.
Resource lookup, effect playback, coverage scalar calculation, position geometry,
and allocator internals remain foreign. Shared container storage/transfer
operations below are intrinsic primitives used by this helper, not additional
helper-owned algorithms or a host-completed map/effect outcome.

## Capacity and transfer order

*[compat]* The insertion wrapper passes the current 32-bit count plus one
to `004F0940` **before** looking up the destination bucket, allocating the
20-byte node, retaining its copied nonnull effect, linking it or incrementing
count (`007D1BB6–007D1C2D`). The ordinary nonoverflow domain is required.
The missing-key/default-insert call path is the one already described above;
existing-key reuse does not newly request growth. This section closes the
helper's postgrowth order obligation; it does not research a generic container
framework or change the existing effect algorithms.

Let n=(bucket-end minus bucket-begin)/4 in the native valid pointer domain,
and r be the unsigned request. `004F0940–004F095B` returns without allocation,
transfer or field writes if r<=n. Otherwise `0079FA90–0079FACE` performs an
unsigned lower-bound search of the 28 32-bit values at `00A829B0–00A82A1F`.
Select the first value >=r; if the result pointer is the table end, use
`FFFFFFFB` (`004F0984–004F099A`). This is a literal ladder, not a computed
next-prime algorithm or an inferred load factor:

53, 97, 193, 389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317,
196613, 393241, 786433, 1572869, 3145739, 6291469, 12582917, 25165843,
50331653, 100663319, 201326611, 402653189, 805306457, 1610612741,
3221225473, 4294967291.

If the selected capacity m<=n, return unchanged even though r>n. Otherwise:

1. Build a temporary bucket vector, initially three null pointers. Request
   4*m bytes from the raw allocation primitive; record its returned begin,
   temporary end=begin and allocation-end=begin+4*m; fill m heads with null
   in increasing address order, then set temporary end=allocation-end
   (`0051C950–0051C9AA`). No helper map fields are changed before that fill.
2. Visit old buckets from index 0 through n-1. Repeatedly take the current
   old head X, calculate unsigned32(X.key) modulo m using a zero-extended
   32-bit numerator and 32-bit unsigned division, replace the old head with
   X.next, replace X.next with the current new head, and replace the new head
   with X (`004F09B9–004F09FA`). Continue until this old bucket is empty,
   then advance the old bucket index. No node/payload allocation, copy,
   retain, release, effect call, visited reset or count change occurs here.
   Null-effect nodes transfer exactly like other nodes.
3. Store helper bucket begin, end, allocation-end in that order. Only then
   free the nonnull old bucket allocation (`004F09FC–004F0A1B`). The old
   heads have already been emptied; this does not clear or free their nodes.
   The helper count and selected mask retain their values throughout growth.

Within a new bucket, the final chain is the **reverse of the sequence of
nodes encountered for that destination**. Later old buckets therefore precede
earlier old buckets when they collide in the new table. Native traversal
remains new-bucket index ascending, then current next-chain order. Stop/clear
and refresh retirement observe this order; transfer itself makes no effect
calls. A subsequent insertion prepends its new node in the new table and only
then increments count. Copied nonnull effect retain occurs after storing that
effect in the new unlinked node, before storing scalar/visited, linking and
incrementing count (`007D1C02–007D1C2D`). Existing nodes' scalar bits, visited
bytes and effect references survive growth unchanged.

These claims independently match the previously published same-body
[selection transfer contract](water-ripples.md#23-drawing-update-0x007514f0).
That contract's historical parked round is superseded by its final report;
no new generic-owner coverage or renewed foreign-body scope is claimed here.
The exact helper call and node layout above supply applicability. The
constructor's request 100 selects 193 by this same table/search
(`00829BC0–00829BFC`). For its initially null vector, reserve requests 772
bytes, stores begin=P, end=P, allocation-end=P+772, then null fill advances
end to P+772 (`0045D56F–0045D59F`, `004FBA47–004FBB1A`). Count is zeroed
after setup; default mask is then zeroed by the helper constructor.

No checked allocation-failure result, false return, rollback or null-allocation
recovery exists in these normal storage paths. A null allocation for positive
m reaches unchecked writes. Exceptions, nonreturn, huge-size arithmetic,
bad pointers and callback mutation remain outside this specification. The
selector's fallback value alone does not admit an overflowing allocation.

## Category lookup

*[compat]* `007D1AB0–007D1B32` performs all four tests, in order, without looking
at the helper's selected mask or persistent map:

1. Obtain the occupant's property source (slot `+18`).
2. Set bit 0 if P membership returns a nonzero low byte.
3. Set bit 1 if F membership returns a nonzero low byte.
4. Set bit 2 if reading property `091B42A2` returns a nonzero low byte.
5. Set bit 3 if reading property `691B42B3` returns a nonzero low byte.

The result is the sum of the four independent bits, in the range 0–15; the
instructions produce the zero-extended result in EAX (`007D1B2E`). Property
values themselves do not gate the bits: successful reads returning zero still
set their respective bits. A service return of `0x100` has a zero low byte and
does not set its bit. No raw occupant type test, value threshold, short circuit,
effect operation or whole-view refresh occurs here. Service pointers and occupant
are not null-checked in this body. This is not “all selected categories match.”

## Setting categories

*[compat]* Every call to `007D1D10` stores the full supplied 32-bit mask **before
any effect callback** (`007D1D22`). No old/new equality comparison occurs.
Traverse every entry in native map order. For a nonnull effect, query slot
`+14`; only if its low byte is nonzero, call slot `+10(0)`. Entries are still
present during this stop traversal. Then call the shared clear primitive
`007D1B40` (`007D1D98`): in bucket/chain order, release each nonnull effect,
free each node, null the bucket, finally set map count to zero. Clear retains
the bucket array and does not clear the newly stored mask.

The stop pass finishes for **all** entries before the clear/release pass begins.
Calling with the same mask, zero, or high bits still follows this sequence.
An empty map still gets the mask store and clear call. This setter does not
call refresh, create effects or acquire occupants. It supplies no semantic
boolean return (`RET 4`, `007D1DA2`).

## Refreshing coverage

*[compat]* `007D20E0` first traverses all entries and clears their visited bytes
(`007D2101`). It does not clear their references or creation scalars. It then
evaluates the following passes in order; each pass tests its mask bit and loads
its service at that time. Missing service skips just that pass. High mask bits
do not enable any additional passes.

| Bit | Service | Collection primitive | Per-item scalar primitive | Initialized input | Resource name |
|---|---|---|---|---|---|
| 1 | P, `00B43D98` | `+54(list output)`; test full EAX | `+50(occupant, input/output A, output B, scalar output)` | A=`0x3F000000` (0.5) | `PlopMode_Police_Existing` |
| 2 | F, `00B43D78` | `+58(list output)`; test full EAX | `+54(occupant, input/output A, output B, scalar output)` | A=`0x3F000000` (0.5) | `PlopMode_Fire_Existing` |
| 4 | H/E, `00B43DA4` | `+C4(list output, 0)`; ignore status | `+B4(occupant, input/output A, output B, scalar output)` | A=`0x3F333333` (binary32 near 0.7) | `PlopMode_Health_Existing` |
| 8 | H/E, `00B43DA4` | `+BC(list output, 0)`; ignore status | same `+B4` | same `0x3F333333` | `PlopMode_Education_Existing` (`00ABB460`) |

The table's scalar is the **last** output argument, not initialized A. On a
zero low-byte per-item status, override scalar with binary32 32 (`0x42000000`);
otherwise consume the written scalar. Outputs A and B are not used afterward.
Call the helper's one-occupant update with occupant, scalar and pass resource;
then release that listed occupant, regardless of whether update found a position
or created anything. Continue in list next order. There is no category lookup
or additional filter per listed occupant. P/F skip processing when collection's
full EAX is zero even if the list is nonempty; H/E process the returned list
regardless of status. All passes clean up their temporary lists afterward.

Each enabled, available pass builds its own empty sentinel list using the cached
allocator `00B207E4` (lazy framework lookup IDs `3AE4BEA3`/`3AE4BEAB`), then a
12-byte sentinel allocation. It visits first-next through the sentinel, calls
list cleanup `00413E60`, and frees the sentinel via allocator slot `+10`.
The collection boundary must specify the list and occupant references it returns;
it cannot hide the helper's loop or admission decisions. Allocation failure is
not handled by the helper's ordinary path.

Finally traverse the persistent map in bucket/chain order. For every unvisited
entry, if effect is nonnull query `+14` and conditionally call `+10(0)` as in the
setter. Reload the effect; if nonnull, **store null before releasing it**
(`007D2684–007D2690`). Retain the node, occupant key and creation scalar. This
is effect retirement, **not removal of the map entry**. Visited entries receive
no final effect operation. A zero/high-only mask retires all live entries but
does not empty the map. Repeated refresh of such a mask has no further effect
calls once references are null.
Refresh supplies no semantic return/status value (`RET`, `007D26DB`).

Primary pass citations: `007D2145–007D227A`, `007D227D–007D23B9`,
`007D23BC–007D24F4`, `007D24F7–007D262F`; retirement `007D2632–007D26DB`.

## Updating one occupant

*[compat]* Inputs are occupant identity, binary32 scalar and resource-name
pointer. No selected-mask test occurs here. These three 32-bit stack arguments
are proved by all four refresh call sites and `RET 0C` at `007D20D5`.
Update supplies no semantic return/status value; incidental EAX contents must
not be interpreted as a creation or update result.

First call the position-object primitive. If null, return without map lookup,
visited marking, effect creation or release. Otherwise retain that object,
initialize the position triple from `00B4E2D4`, and ask it to fill the triple;
ignore the position-read return. Only then look up the raw occupant key.

### Existing entry

Compare the input scalar with the **stored creation scalar**, using x87
unordered comparison. Equal ordered values skip the effect update. Unequal
or unordered values proceed only if the stored effect is nonnull. In that case
send a transform payload to effect slot `+20`: identity 3-by-3 basis, copied
position triple, doubled scalar, first control byte 2; second control byte 1
for an ordered doubled scalar equal to 1, otherwise 3. Mark visited **after**
the effect call, then release the position object. If effect is null, merely
mark visited and release position. No creation, start, resource-name change,
effect retain/release or stored-scalar write occurs on this existing-key path.
An existing entry whose effect was retired therefore does not recreate an
effect on a later refresh. Citations: `007D1E8F–007D1F8E`.

### Missing entry

Initialize an identity transform with zero translation and an initial scalar
of binary32 1. Clear its first two control bytes. Request the
named effect, passing an output-reference address. On zero low-byte create
status, release the position object and return: do not insert, call the effect,
or release a possibly written output reference. This is an ignored-output
failure path, not rollback or guaranteed cleanup of a nonnull failed output.

On nonzero low-byte status the output is dereferenced without a null check.
Fill position and doubled scalar. The first control byte is incremented twice
from zero to 2. The second becomes 1 for an ordered doubled scalar equal to 1,
otherwise 3. The initialization instructions execute with the output argument
already pushed; accounting for that four-byte stack shift establishes that
these are the same initially cleared control bytes.

Call effect `+20(payload)` then `+0C(0)`, ignoring both results. Look up/default
insert the occupant entry via `007D1C40`, set visited=1 and creation scalar to
the original input scalar, then assign the created reference. Reference
assignment (`005447B0`) does nothing for identical references; otherwise retain
new nonnull reference, store it, then release old nonnull reference. Finally
release the create output reference, then release the position object. Entry
state is written **after** update/start callbacks, but visited/scalar are written
**before** the reference assignment's retain callback. Citations:
`007D1F91–007D20D5`, particularly `007D2015`, `007D2089–007D20CB`.

### Numeric qualification

Doubling loads binary32 scalar, adds it to itself in x87 and stores a binary32
payload scalar; the comparison with binary32 1 (`00A81228`) uses the still-live
x87 value, not a reload of the rounded stored scalar (`007D1EF4–007D1F1E`,
`007D202C–007D2051`). Do not replace this by an arbitrary binary64 pipeline or
assume process floating-point precision/rounding controls. Ordered equality
suppresses reuse updates; unordered comparison takes the update path and the
non-unit scale flag path. Signaling-NaN exceptions and unmasked floating-point
exceptions are outside the supported normal-return domain.

The observable payload fields are a control byte 2, a scale-control byte 1 or
3, nine binary32 basis components `[1,0,0; 0,1,0; 0,0,1]`, three position
components copied without helper arithmetic, and the doubled scalar. The
control bytes precede the basis by four bytes; the intervening two padding
bytes are not initialized here and are not specified as zero. This contract
does not infer a radius disc, terrain projection, axis convention, effect class
ID or screen geometry from the resource names. Those belong to the receiving
effect/position primitives. The default triple source `00B4E2D4` has initial PE
contents zero; a runtime constant-value guarantee is not established.

## Reset and teardown evidence

### Construction and receiver identity

*[compat]* Let B be the complete view receiver. Its constructor establishes
the window subobject at B+12 (`007F0A72–007F0A8E`) and installs `00ABCBC0`
at that subobject (`007F0AB1`). That table's
slots +10/+14 target Init/Shutdown. Consequently their receiver I=B+12,
and I+`3D8` = B+`3E4` is the same helper H passed by the complete view
constructor and teardown. These are distinct enclosing identities, not two
helper allocations. Init independently forms I-12 at `007E9915`.

The constructor calls `007D26F0` with H at `007F0EB4–007F0EC0`.
The complete 33-byte helper body stores zero to H+4, H+8, H+12, H+16
in that order, then calls `00829BC0` with request 100, then stores zero to
H+20 (selected mask), and returns H (`007D26F6–007D2710`). H+4/+8/+12
are bucket begin/end/allocation-end; H+16 is entry count. H+0 is not written
by this helper body. No per-entry visited/scalar/effect storage exists yet.
The mask zero store is **after** container setup and its allocation primitives;
it must not be moved before them in an observation of construction prefixes.
No helper effect callback, retain, release or registration call occurs in this
body. Container setup selects a capacity and initializes null bucket heads,
then zeroes count (`00829BC0–00829C29`); allocation internals remain foreign.
Construction does not call refresh or the category setter. This identifies
embedded helper construction, without asserting a standalone factory/class ID.

Independent **PROPOSED UNEXECUTED** lifecycle literals (additional to the
unchanged 30 cases below):

| Case | Fixture | Exact expected observations |
|---|---|---|
| L0 | Complete receiver B=`10000000`; helper bytes initially `A5` | I=`1000000C`, H=`100003E4` both from I+`3D8` and B+`3E4`; constructor returns H. |
| L1 | H initially filled `A5`; successful ordinary container setup | Stores H+4=0,+8=0,+12=0,+16=0; setup request=100; only after setup H+20=0. At setup entry mask remains `A5A5A5A5`; H+0 remains that value throughout this body. Empty count=0 and all allocated heads=null on return; no effect events. |
| L2 | Constructed empty helper; manually selected mask=`8000000F`; stop/clear | No effect events; count=0, mask=`8000000F`, bucket allocation retained. Separate clear/free fixture frees nonnull bucket storage after clear, with no mask store and no bucket-pointer nulling. Neither fixture is a second constructor. |
| L3 | Constructor allocation returns P=`20000000` normally | Request=772 bytes; before allocation H+4/+8/+12/+16=0, mask still initial bytes. Reserve stores begin=`20000000`, end=`20000000`, allocation-end=`20000304`; fill writes 193 null heads, end becomes `20000304`; count=0; only afterwards mask=0. H+0 unchanged. No effect/reference calls. |

The alias call at `007E9CCB` passes the view-interface receiver plus `3D8` to
refresh. The shutdown call at `007F14AE` passes receiver plus `3D8` to unnamed
`007D1DB0`. This positively associates that stop/clear body with the helper
subobject. *[compat]* `007D1DB0–007D1E35` performs the setter's complete stop
traversal, then shared clear, **without a mask store**. Thus an explicit call
to this body clears entries but preserves categories; an empty map has no
effect callbacks. It is not justified to invent an independently registered
reset interface or add this call to every refresh.

View teardown site `007F11BB–007F11C1` passes its receiver plus `3E4` to unnamed
`007D1CF0`. That body calls shared clear and then frees the nonnull bucket
array (`007D1CF0–007D1D09`), with no stop traversal or mask reset. These are
positive embedded-helper destruction observations: constructor, Init,
Shutdown and teardown refer to the same H as proved above. The deleting view
destructor calls this complete-view teardown without changing its receiver
(`007F3E30–007F3E4D`), and conditionally frees the complete view afterwards.
No separately named helper destructor, GZCOM factory or standalone allocation
is implied. This clear/free body does not invoke stop/clear: effect references
are released directly by shared clear. Whether another enclosing stage already
stopped effects, and any wider notification/registration lifecycle, remain
unproved; no extra helper reset/notification call is invented.
Clear/free does not null the freed bucket-array pointer; reusing such an object
as a live helper is not within the supported domain.

Shared clear retains the allocated bucket range, zeroes each head after
releasing/freeing its chain, and zeroes count after all buckets. Stop/clear
therefore leaves a live empty helper with the previous capacity and selected
mask. Clear/free first performs that same release/free/count sequence, then
frees its bucket allocation without nulling begin/end/allocation-end. The
selected mask and H+0 are not reset by either body. Setter ordering remains
different: it stores its new mask before the first effect callback. Destruction
does not add an active-test or stop call inside shared clear.

## Proposed literal traces

**PROPOSED — UNEXECUTED.** These are independent expectations derived from the
cited instructions and public binary32 arithmetic before any portable output
exists. They are neither native goldens nor shadow/game results. Each fixture
supplies raw primitive results independently; the host must not return a
precomputed category mask, completed coverage pass or effect-update outcome.
Use ordinary nonthrowing/nonmutating callbacks, valid readable inputs, a valid
nonzero bucket count for lookups and full output writes on successful reads.
`retain`, `release`, `update`, `start`, `active?` and `stop(0)` mean the slots
specified above; none implies a numeric return convention.

### Category and mask cases

Each lookup calls property-source, P, F, property `091B42A2`, property
`691B42B3`, in that order; all four tests run even when the first succeeds.
The property output values below are deliberately independent of status.

| Case | Raw P/F/property1/property2 statuses | Property output values | Expected EAX |
|---|---|---|---|
| C0 | `0,0,0,0` | arbitrary failed outputs | `0` |
| C1 | `1,0,0,0` | arbitrary failed outputs | `1` |
| C2 | `0,0x80,0,0` | arbitrary failed outputs | `2` |
| C3 | `0,0,1,0` | `0`, arbitrary | `4` |
| C4 | `0,0,0,0x80` | arbitrary, `0` | `8` |
| C5 | `0x100,1,0x100,1` | nonzero, `0` | `10` |
| C6 | `1,0x80,1,1` | `0,0` | `15` |

These cases are unchanged if the helper mask is zero or `0x80000000`, or if
the queried occupant already has a live entry: category lookup never updates
the whole view. C5 distinguishes low-byte tests from full-register truth and
from tests of property output values.

| Case | Starting state / action | Expected state and ordered events |
|---|---|---|
| S0 | Empty map; set `0x80000008` twice | Both calls store that exact mask and call clear; no effect calls and no refresh. |
| S1 | Fixed map traversal A then B; A effect E1 active, B effect E2 inactive; old mask=3, set 3 | Store 3; E1 `active?`, E1 `stop(0)`, E2 `active?`; then E1 release/free A, E2 release/free B; map count=0. No equality shortcut. |
| S2 | Same as S1, set 0 then set 0 again | First call has the same stop/release order, new mask=0; second has no effect calls. Stop callbacks can observe mask=0 while entries still exist. |
| S3 | Fixed live A; invoke stop/clear `007D1DB0`, mask=8 | A `active?`, conditional `stop(0)`, release, free; entries empty, mask remains 8. Empty repeat has no effect callbacks. |

### One-occupant cases

Use occupant A=`0x1000`, a position object Q, and Q output `(1,-2,3)` unless
specified otherwise. All successful creates return distinct nonnull effect E.
Only a creation writes the original scalar to the entry. These traces exercise
the helper directly and do not assume any selected category.

| Case | Input / precondition | Expected state and ordered events |
|---|---|---|
| U0 | No entry; position lookup returns null | Only position lookup; no create, insert, visited write or release. |
| U1 | No entry; scalar=0.5; create status low byte=1, output E | Q retain/read; create requested name; E update with position `(1,-2,3)`, identity basis, scalar=1, control bytes `(2,1)`; E start(0); insert A; visited=1 and stored scalar=0.5; E retain/store; E release; Q release. |
| U2 | U1 entry; scalar=0.5; Q output changed to `(9,8,7)`; different name | Q retain/read/release; visited=1. No E update/start; old effect/name and stored scalar persist. Position changes alone do not trigger update. |
| U3 | U1 entry; scalar=2; repeat twice | Each call Q retain/read; E update with doubled scalar=4 and controls `(2,3)`; visited=1; Q release. Stored scalar stays 0.5, so both calls update. No effect retain/release or start. |
| U4 | No entry; create status low byte=0, output nonnull E | Q retain/read, create, Q release; no insert or E operation, including no E release. Same flow for status `0x100`; failed output is not adopted. |
| U5 | Existing A, effect=null, stored scalar=0.5; scalar=2 | Q retain/read; visited=1; Q release; no create or effect update. |
| U6 | Existing A with E, stored scalar=+0; input=-0 | Q retain/read; no effect update; visited=1; Q release. Ordered zeros compare equal. |
| U7 | Existing A with E, stored scalar=0.5; input is quiet NaN, normal floating-point return assumed | Q retain/read; E update, controls `(2,3)`; visited=1; Q release. No NaN payload-bit promise; stored scalar stays 0.5. |
| U8 | Missing A; scalar bits `0x3F000001`; successful create | Same events as U1, but doubled scalar bits `0x3F800001`, controls `(2,3)`, stored scalar bits `0x3F000001`. This exact finite doubling needs no rounding-mode assumption. |
| U9 | Existing A with E, stored scalar=0.5; input=-2 | Update scalar=-4, controls `(2,3)`, stored scalar unchanged. No owned negative-scalar rejection/clamp. |

Nonzero create status with null output is **not a supported normal-flow trace**:
`007D2086` dereferences that pointer. Do not invent a null-success return or
cleanup rule. A callback returning a failure value from E update/start does not
change U1's following start/insert/retain/release sequence: those results are
ignored. During update/start the new entry is absent; during the reference
assignment retain callback its visited/scalar fields have already been stored,
while assignment has not yet stored the new effect (`005447C7–005447CC`).

### Refresh, duplicates and retirement

All cases specify collection next-order explicitly. Scalar success outputs are
raw independent binary32 values, not derived from the lookup category bits.

| Case | Fixture | Expected calls/state |
|---|---|---|
| R0 | Mask=`0x80000000`; entries A/E1 and B/E2, fixed traversal A then B; E1 active, E2 inactive | Clear both visited bytes; no collection; E1 active?/stop(0), null A reference, E1 release; E2 active?, null B reference, E2 release. Both nodes and scalars persist. Repeat: markers cleared again, no effect calls. |
| R1 | Mask=1; P list `[A,B]`; scalar primitive A returns false, B true with scalar=0.5; neither entry exists; all positions/creates valid | A gets scalar=32, resource Police, payload scalar=64/controls `(2,3)`; release listed A after its update chronology. Then B gets 0.5, payload scalar=1/controls `(2,1)`; release listed B. Two distinct map entries/effects, both visited. Cleanup list after both. No occupant category lookup. |
| R2 | Mask=3; P list `[A,A]` with scalar outputs 0.5 then 2; F list `[A]` with scalar=4 | Create Police E once for first A, storing 0.5; update same E with 4 then 8 on the next two occurrences. Start once, no Fire creation or resource rename. Release listed A three times, once per occurrence. No selection of min/max/last stored scalar: stored scalar stays 0.5. |
| R3 | Mask=1; P collection full EAX=0 but writes list `[A]`; live old A/E | No per-item scalar/update or explicit occupant-release loop; list cleanup still called. A unvisited, so retire E. Any additional cleanup's element-reference behavior remains foreign. |
| R4 | Mask=4; H collection status=0 but writes `[A]`; scalar status=false | Process A anyway, scalar=32, resource Health, then release listed A. Distinguishes ignored H/E collection status from tested P/F status. |
| R5 | Mask=8; only education service available; list `[B]`, scalar success=1.5 | Only Education collection, initialized A bits `0x3F333333`; use resource Education, scalar payload=3. Health is not collected merely because it shares this service. |
| R6 | Mask=1; old A/E; list `[A]`, but position lookup null | Update returns before marking; explicit listed A release still follows. Final sweep retires E and retains the map key. Next successful refresh of A marks the null-effect entry, without recreation. A subsequent setter clears that key, permitting later creation. |
| R7 | Mask=`0x8000000F`; available services, all lists empty | Collections in P, F, H, E order with separate cleanup per pass; retire old entries in map order. High bit adds no pass. With a missing P service, skip only P and retain F/H/E order. |
| R8 | Teardown primitive `007D1CF0` with fixed live A/E, then separate empty fixture | Live fixture: E release, free A, clear count, free nonnull bucket array; **no active? or stop** in this body. Empty fixture: no effect calls, still free a nonnull bucket array. This does not claim the enclosing full destruction sequence omits stopping. |

### Additional growth literals

All cases G0–G7 are **PROPOSED UNEXECUTED**, instruction/public-integer
expectations, not native runs or implementation outputs. A supplied map state
is an analytical fixture, not a claim that construction produces 0, 1 or 2
buckets. Keys below are raw pointer identities; no occupant is dereferenced
by transfer. Allocations return valid distinct storage; scalar bits and visited
bytes are arbitrary fixed values unless specified. Null effects are valid nodes,
not absent keys. Count includes them.

| Case | Supplied state / operation | Exact expected sequence/result |
|---|---|---|
| G0 | Zero bucket pointers, count=0; insert key 53 with null effect | Growth request=1 selects 53; temporary allocation=212 bytes, 53 null heads, then helper begin/end/allocation-end stores; no old bucket free. Count still 0. Node allocation=20 bytes; prepend key53 in bucket0, then count=1. Mask unchanged; no effect/reference calls. No modulo-zero lookup occurs before this insertion wrapper's growth call. |
| G1 | n=1, count=0; insert key53/null, then key106/null | First request1<=1: no bucket allocation; bucket0=[53], count1. Second request2>1 selects53, allocates212 bytes, transfers53 to new bucket0, commits range, frees old bucket allocation; node106 prepends: bucket0=[106,53], count2. Two 20-byte node allocations total; no node freed/released during growth. |
| G2 | n=2, count=1, bucket0=[106], other head=null; insert212/null, then318/null | First request2<=2: no growth, bucket0=[212,106], count2. Second request3 selects53; old head212 transfers first, then106, giving new bucket0=[106,212]. Insert318 gives [318,106,212], count3. All keys have remainder0 for both 2 and53. No effect calls. This differs from retaining the old chain or reversing the final new chain again. |
| G3 | n=193, count192; absent-key insertion, then another absent-key insertion | Requests193 then194: first has no bucket allocation, count becomes193; second selects389 and requests1556 bytes, transfers all193 old nodes with count still193, commits range/frees old bucket allocation, then allocates20-byte node and increments to194 only after head store. New threshold is389: request389 does not grow; request390 selects769. No load-factor or per-bucket-length trigger. |
| G4 | n=193, count193; old bucket0=[A,N], bucket3=[B,F3], bucket6=[C,F6]; Fj for every j=1..189 in its old bucket; A=75077, N=150154, B=389, C=778, Fj=j. A/B/C have EA/EB/EC; N and all Fj effects=null. Insert D=1167/null. | Old bucket0 encounters A,N, new chain [N,A]; old bucket3 contributes B, new chain [B,N,A]; old bucket6 contributes C, new chain [C,B,N,A]. Fj transfer to new bucket j. After insert, new bucket0=[D,C,B,N,A]; other nonempty heads are F1..F189 at indices1..189. Count194. Transfer has no retain/release/effect calls and preserves all scalar/visited bits. A following stop traversal with EA/EB/EC active calls EC active?/stop, EB active?/stop, EA active?/stop, skipping D,N,Fj. Shared clear then releases EC,EB,EA in that order while also freeing every null-effect node; count0, capacity389 retained. |
| G5 | G4 snapshot after growth/insertion, before its stop/clear; A scalar bits=`3F000000`, N bits=`40000000`, both visited=1 | Same scalar bits/visited bytes survive transfer. Null-effect N remains a found key, contributes to count and chain, and a subsequent normal update of N marks it without create/recreation. A's later scalar=2 compares with creation scalar0.5 and updates with doubled4, leaving creation scalar0.5. Rehash adds no effect ownership event. |
| G6 | Direct shared growth with r=194,n=389; separate n=0,r=0 | Both return before selector/allocation/transfer/stores, with counts/masks/heads unchanged. The zero-bucket fixture is not then used for lookup. Ladder lower-bound request100 is193, not101; request194 is389, not197. |
| G7 | n=193,count193; allocation1556 returns P=`20001000`; insertion operand D carries nonnull ED, scalar0.5, visited1 | During bucket allocation, helper still exposes old range/count193. Temporary heads zero before any old head removal. After transfer store begin=`20001000`, end=`20001614`, allocation-end=`20001614`, then free old bucket allocation. Node allocation20 follows; store key/effectED, ED retain while node is unlinked and count193, store scalar/visited, store next=current destination head, store bucket head=new node, then count194. No release of ED or old node effects is part of this wrapper. Raw allocator numeric status/exception policy is not invented. |

G4 discriminates against stable transfer, sorting by input/key/node address,
reverse old-bucket traversal, discarding null-effect nodes and clearing the map
on growth. Its 193 old nodes are four special identities plus exactly 189 Fj;
75077=193*389, 150154=2*193*389, and 389/778/1167 have old remainders
3/6/9 but new remainder0. Thus later old buckets precede earlier buckets in
the new chain without relying on a host-provided completed map order.

## Evidence ledger

| Status | Bounded claim / primary basis |
|---|---|
| Supported | Four independent category tests and low-byte statuses, `007D1AB0–007D1B32`; zero-extended category result, `007D1B2E`. |
| Supported | Full mask stored before stop callbacks; same-mask clear; `007D1D10–007D1DA4`, shared clear `007D1B40–007D1BA0`. |
| Supported | Four ordered refresh passes, list-status differences, default failure scalar and per-occurrence release; `007D2145–007D262F`. |
| Supported | Visited reset and stale-reference retirement without node erasure; `007D2101`, `007D2632–007D26DB`. |
| Supported | Pointer-key lookup, no scalar replacement on reuse, no null-effect recreation, effect chronology; `007D1E40–007D20D7`, assignment `005447B0–005447DE`. |
| Supported | Local bucket/chain traversal and insert-at-head consequences; `004B0390–004B03DD`, `007D1BB0–007D1CE7`; exclusive class ownership not claimed. |
| Supported | Alias `007D26E0–007D26E4`; extra caller-bound stop/clear and clear/free bodies as stated in Reset and teardown evidence. |
| Supported | Shared intrinsic growth, actual capacity ladder and unsigned lower-bound, ascending old-bucket/head-to-new-head transfer, null-node retention and field/free timing; `004F0940–004F0A26`, `0079FA90–0079FACE`, `0051C950–0051C9AA`; no exclusive helper-owner credit. |
| Supported | Embedded construction/default mask and range/count setup; receiver identity; reset-versus-destruction retention; `007D26F0–007D2710`, `00829BC0–00829C29`, constructor site `007F0EB4–007F0EC0`, window table `00ABCBC0`, reset/free bodies as above. |
| Partial | Wider enclosing registration/notification and whether another enclosing stage stops effects before destruction; exceptional allocation/overflow domain. |
| Foreign | Single service membership, property conversion/read, scalar computation, collection contents/order/reference policy, position query, allocator, list cleanup and effect primitive internals. |
| Unknown / unsupported | Exceptional or invalid-input paths, callbacks mutating/reentering helper, incomplete output writes, uninitialized padding, ambient floating-point controls and overflow domains; see Limits and questions. |

## Limits

The supported domain uses valid services where not checked, valid occupants,
proper map/list invariants, successful allocations, fully supplied successful
primitive outputs, synchronous ordinary returns, and no helper mutation/reentry
from callbacks. The native body does not validate these preconditions. A future
bounded portable contract must explicitly reject or otherwise designate cases
outside them, rather than inventing atomic rollback or “safe” native behavior.
Failure statuses covered above are supported with their actual mutation prefixes.
Failures of update/start/stop/reference calls are ignored as numeric outputs;
exceptions and missing returns are not converted into status failures.

There is no proven helper guard against null required services, unreadable
occupants, zero bucket divisors, bad list nodes, uninitialized successful scalar
outputs, allocation failure or unchecked arithmetic overflow. No exception,
thread-safety, reentry, retention-across-mutating-callbacks or all-input numeric
equivalence contract is established. x87 precision/rounding controls, infinities,
overflow/underflow, signaling NaNs and foreign simulator/transcendental numeric
domains need separately bounded admission before portable equivalence claims.
Quiet-NaN branch expectations are conditional on ordinary floating-point return;
they do not establish exact NaN arithmetic bytes.

Open questions are recorded in
[questions/coverage-radius-effect-helper.md](questions/coverage-radius-effect-helper.md).
This publication grants no complete eight-entry/class/family, native execution,
shadow, default enablement, or simulator coverage credit. It reopens none of the
closed particle, manager/emitter, Rigid, Query, PlaceLot, View3D or geometry
algorithm histories; caller slices establish only receivers and inputs.
