# Automata controller manager: owner lifecycle supplement

## 1. Scope and evidence

**[compat, bounded native contract]** This supplements [automata.md §3](automata.md#3-component-41-the-controller-manager-0xca14c9fc--0xca14ca03-compat)
for `cSC4AutomataControllerManager`, IID `0xCA14C9FC`, class `0xCA14CA03`, city slot 47.
It specifies manager decisions and direct calls to foreign interfaces. A controller's
own Shutdown, automaton destructor, grid geometry and serializer object-table lifetime
are separate contracts. In particular, a direct Release count below is not a prediction
of the complete transitive reference graph or a native ref-warning verdict.

Source: Windows GOG 1.1.641.0 executable SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The export summary identifies the same digest, image base `0x00400000`, 35,637 functions,
2,193,251 instructions. Export queries preceded local pseudocode navigation; instruction
citations establish the virtual slots, pointer offsets, branch guards and call order.
No original execution is claimed. Examples are **PROPOSED / UNEXECUTED**.

All offsets in this supplement are from the primary manager interface, unless explicitly
called a node offset. The message-target interface is primary +8, serialization +12.
This matters: the Read pseudocode's field names are relative to the latter interface.

## 2. Root state and reference sites

| Site | Manager-owned state | Direct teardown rule |
|---|---|---|
| +0x10 | initialized byte | gates Shutdown; cleared after all cleanup |
| +0x14 | manager reference count | Release decrements if above 1; otherwise invokes virtual Shutdown, destroys storage and frees manager |
| +0x18 | cached 24-hour clock interface | one Release when nonnull, then null, during initialized Shutdown |
| +0x1C..+0x28 | grid's vector/list storage | grid list storage drained before controller-map callbacks; no controller Release at this call site |
| +0x44 | cached script-system interface | one Release when nonnull, then null, before clock Release |
| +0x58 | occupant-to-controller-list map | per list occurrence: controller Shutdown then Release; later occupant-key Release |
| +0x6C | occupant-to-automaton-queue map | per entry: nonnull occupant-key Release, then one automaton Release per list occurrence |

The controller-map entry contains an occupant pointer at node +4 and a list sentinel at
node +8. The queue map has the same prefix and a count at node +12. These are independent
reference sites: equality of pointer values does not coalesce list occurrences or the
two maps' occupant-key Releases. A null controller-map key is allowed by the cleanup
guard; controller and automaton list values are dereferenced without a null check.
Acquisition paths and normal mutation admission are delimited separately below; this
table does not infer AddRef from a saved reference token.

Evidence: `0x0051A6F0–0x0051A7C8`, `0x0051AC10–0x0051AD1B`,
`0x0051B4DA–0x0051B50F`, `0x0051B1C0–0x0051B1EB`.

## 3. Init and initialized Shutdown

Primary slot 3 `Init()` returns boolean true, including when already initialized.
When +0x10 is zero it first initializes the embedded grid. If the clock is null it obtains
the city's +0x158 service and directly AddRefs it; if the script pointer is null it obtains
the city's +0xC0 service and AddRefs it. The returned services are assumed nonnull at
these call sites. It registers primary +8 for the ordered message IDs below when the
global message server exists, then requests animation-tick registration with argument
`0xC0` when that service exists. Optional debug-command setup follows; finally +0x10=1.
Foreign return values do not gate that final state (`0x0051C140`, 397 bytes).

Message registration/removal order:
`99EF1142`, `99EF1143`, `26D31EC1`, `89D3E9F3`, `44778230`, `AA1D07F3`,
`A6B79602`, `A6B79612`, `AA375FE8`, `CAD61E00`, `EAD61E16`, `EBFF18A3`.
These are hexadecimal IDs. There is no `26D31EC2` PreCityShutdown subscription here.

Primary slot 4 `Shutdown()` returns boolean true (AL=1; upper return bits are not a
specified integer status). When +0x10 is zero it performs none of the following work.
When initialized, its exact manager-side sequence is:

1. If the debug service exists, remove command `0xABA2B5BB`, then remove primary +4 as
   its target with argument zero. This is not guarded by whether Init's optional debug
   setup actually registered it.
2. If the animation service exists, remove primary +8 with argument `0xC0`.
3. If the global message server exists, unregister primary +8 for the twelve IDs above,
   in the same order. These calls are to the services present at teardown, not stored
   server references. Subscription internals' own AddRef/Release effects are foreign.
4. Release script pointer if nonnull, null it; Release clock pointer if nonnull, null it.
5. Drain the queue map as §4 describes.
6. Drain grid storage and controller map as §5 describes.
7. Clear initialized byte. No loaded/new-city branch occurs in this body.

Evidence: `0x0051B3E0–0x0051B516`. A second completed Shutdown is a no-op apart from
returning true. The guard is cleared **after** callbacks; recursive Shutdown during
the first call is not admitted by the completed-teardown idempotence rule.

## 4. Queue-map teardown

For each entry in native hash traversal order: Release the occupant key if nonnull;
then walk its automaton list from first node to sentinel and Release each value once.
There is no manager-side automaton Shutdown call. Only after all those Releases does
the manager free the list nodes, restore the empty sentinel links and set the entry
count to zero. Finally it clears all hash entries. It does not deduplicate identical
automaton identities. Hash traversal follows each chain then subsequent buckets;
there is no promise of pointer-sorted or insertion order across entries.

At each Release callback the current entry/list has not yet been detached. There is
no clear-before-callback guarantee. This rule covers callbacks that return normally
without mutating these containers and objects that remain valid for the described
calls. Callback mutation, exceptions and resurrection are not given fallback rules.
Evidence: `0x0051A720–0x0051A776`, `0x0051A780–0x0051A7BD`.

## 5. Controller-map teardown and final manager Release

Before entering the controller map, the manager removes all embedded grid list
storage and sets the vector end to its begin. At that site the range supplied to
`0x00518580` is empty; list-storage destruction `0x006F1EA0` frees nodes/sentinels,
with no virtual controller calls. This is not evidence about grid geometry or a
controller's separate grid membership callback (`0x0051AC16–0x0051AC44`).

For each controller-map entry in hash traversal order, the manager walks the list
from first to sentinel. Each occurrence receives virtual slot 4 Shutdown immediately
followed by slot 2 Release. All list nodes still exist during this first walk. A second
walk frees them; sentinel links are then reset. The nonnull occupant key receives
Release **after** all that entry's controller calls and node removals. The whole map
is cleared after traversal. There is no identity deduplication and no manager-side
controller Init or AddRef in this teardown (`0x0051AC68–0x0051AD10`). The same normal,
nonmutating callback domain as §4 applies. Controller Shutdown may have additional
foreign effects; these do not replace the manager's immediately following Release.

Primary slot 2 `Release()` returns the decremented count for count >1. For count <=1
it calls primary virtual slot 4 **before** the destructor `0x0051A280`, then frees the
manager and returns zero. The destructor itself resets vtables and destroys map/grid
storage. The shared map-storage helper `0x006F2970` frees nodes and list sentinels;
it makes no virtual Release calls. Thus normal final Release runs initialized cleanup
before storage destruction; it is incorrect to describe the held controllers as being
released solely by the destructor. Final Release following completed Shutdown invokes
Shutdown again, whose initialized guard skips the release work.

## 6. Publication boundary

This first supplement establishes the direct initialized close sequence. AT21 remains
open pending the city-message/occupant-removal ordering and acquisition/mutation ledger.
The existing serialized record in automata.md §3 is prior input, not new coverage here.
In a loaded city whose serializer object table has gone away, the manager's direct
controller-list Releases still follow §5; no object-table reference is required by
the loop. Whether each direct Release is the last reference depends on the input graph
and the called controller's own contract.
