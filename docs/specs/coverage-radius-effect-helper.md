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
Nearby stop/clear `007D1DB0` and clear/free `007D1CF0` are not named lifecycle
methods on this evidence. No constructor, destructor or registration claim is
made from adjacency. Existing PlaceLot/Query consumer behavior in
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
fixture. Growth/rehash remains a foreign container primitive; do not promise
sorted occupant order or insertion order across growth.

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
allocator and container growth algorithms remain foreign.

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

Primary pass citations: `007D2145–007D227A`, `007D227D–007D23B9`,
`007D23BC–007D24F4`, `007D24F7–007D262F`; retirement `007D2632–007D26DB`.

## Updating one occupant

*[compat]* Inputs are occupant identity, binary32 scalar and resource-name
pointer. No selected-mask test occurs here. These three 32-bit stack arguments
are proved by all four refresh call sites and `RET 0C` at `007D20D5`.

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
positive call-site and cleanup observations, not a complete helper destructor
identity/registration contract: the two view receiver conventions differ by
12 bytes (the Init caller explicitly forms a receiver-minus-12 pointer at
`007E9915`). Constructor/default mask, registration, callable lifecycle names,
and whether another enclosing stage already stopped effects remain unproved.
No GZCOM factory, standalone allocation or default-enable policy is implied.

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
| Partial | Container growth/rehash is delegated by `007D1BBC` to shared `004F0940`; exact traversal after growth is not closed here. |
| Partial | Constructor/default category and whole lifecycle/registration provenance; bounded call-site evidence only. |
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
