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
- [Limits](#limits)

## Ownership and evidence

The canonical Windows export and its byte-bearing instruction queries establish
the following attribution. Decompiled text was navigation only: its apparent
two-argument update signature and several stack-derived values are wrong.

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
| 8 | H/E, `00B43DA4` | `+BC(list output, 0)`; ignore status | same `+B4` | same `0x3F333333` | Resource literal at `00ABB460` (spelling pending verification). |

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

Initialize an identity transform with zero translation/scalar fields and an
additional trailing binary32 1. Clear its first two control bytes. Request the
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

## Limits

This initial publication still requires literal traces, source identity checks,
and a final evidence/unknown ledger. It grants no complete class/family, native
execution, shadow, default enablement, or simulator coverage credit.
