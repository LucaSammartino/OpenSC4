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

## 6. City messages and city-owned close order

The city creates this class/IID into its +0x1E0 field (`cSC4City` creation body, call
identified by `CA14CA03`/`CA14C9FC`). The getter `0x00459710` names that service.
`cSC4City::Shutdown` calls this field's virtual Shutdown at `0x0045B2F2`, then Release
at `0x0045B2FD`, then nulls the field. The occupant-manager field is +0x12C, independently
identified by getter `0x00423710`; its Shutdown occurs later at `0x0045B833` and its
Release at `0x0045B869`. Thus the controller manager's explicit initialized cleanup
precedes the occupant manager's Shutdown. It does not depend on RemoveOccupant messages
from that later Shutdown. Other earlier city-shutdown callbacks are foreign operations;
this ordering alone does not say no occupant could have been removed earlier.

The message-target body `0x00519360` accepts one message pointer and returns true.
Its receiver is primary +8. `PreCityShutdown` (`0x26D31EC2`) has no matching branch:
it falls through the comparisons at `0x005193A9–0x005193B9` to the true-return path.
It neither drains the maps nor releases the clock there. The manager does not subscribe
to that ID in Init. This does not establish every other service's response to the message.

`PostCityInit` (`0x26D31EC1`) directly invokes `0x00518660`. That helper does nothing
unless loaded byte primary +0x11 is nonzero. For each nonnull controller in the +0x58
map, it asks controller virtual +0x1C for the template identity and script-system
virtual +0x44 to resolve it. On a nonzero reply it assigns that reply through controller
virtual +0x3C, calls controller Init (+0x0C), then grid AddController. No extra manager
AddRef occurs in that successful branch. On a zero reply it unlinks/frees that list
node **before** controller Shutdown and Release. It does not remove the occupant entry
or release its key at this point. The helper does not reset the loaded byte, so repeated
PostCityInit is not established as a one-time operation (`0x00518660–0x005187E4`).

`RemoveOccupant` (`0x99EF1143`) obtains the occupant through message virtual +0x28.
If QI for `0xA821EF94` succeeds, it marks byte node +0x28 zero in **every** queue-map
entry and Releases the QI result. It then asks occupant virtual +0x18 for its property
interface and tests virtual +0x0C with `0x029244DB`. A true reply skips controller
removal; a false reply calls primary slot 14 `RemoveControllers(occupant)`
(`0x0051953A–0x005195C6`). The QI and property calls' internal effects are foreign.

`RemoveControllers(occupant)` (`0x0051A520`, one input, boolean result) forwards to
`0x0051A350` with group=null and that occupant. If an occupant entry is found, every
controller in it is selected. In list order it constructs a message `0xEA298D50` with
the controller in the pointer field, submits it via message-server virtual +0x0C and
Releases the message; calls grid removal; then controller Shutdown and Release;
then unlinks/frees that controller-list node. The current node remains linked during
the notification and controller callbacks. Message allocation failure has no safe
fallback specified by this contract. If the list is empty afterward, it finds the
occupant's queue entry: Releases every queued automaton, frees queue-list nodes, resets
sentinel/count, Releases the queue occupant key and erases that queue entry. It then
erases the controller-map entry and Releases its occupant key. Returns true iff at
least one controller was selected, even though an already-empty found entry can still
be erased (`0x0051A350–0x0051A513`, queue drain `0x00517010`).

The overload at `0x0051A530` is a separate name-filtered removal path. Its list node
is unlinked **before** notification/Shutdown; it must not be substituted for the
group-null path's callback order. Its other filtering semantics are outside this
supplement. Neither close-time map drain emits the per-controller notification used
by ordinary removal.

## 7. Queue access and normal mutation

`GetControllerQInfo(occupant, create)` (`0x0051B9D0`, direct helper, two inputs)
returns the existing entry's embedded queue-info address, or null if absent and create
is false. When absent and create is true, it inserts a new entry, directly AddRefs the
occupant if nonnull, zeros the entry's count and queue-info byte +0x20, and calls
`InitQueuePoint(occupant)` on the queue info before returning its address. The returned
pointer is borrowed manager storage, not a reference-counted interface; no queue-info
AddRef is performed. Queue-point calculation is a foreign primitive here. Calls from
controller behavior/state methods exist at `0x0051BAF9`, `0x0051BB8A`, `0x0051BCBD`,
and attractor state entry at `0x0051C012`; their internal decisions are not imported.

Queue info is entry +8. `AddToQueue(automaton)` (`0x005184E0`) and
`RemoveFromQueue(automaton)` (`0x005172D0`) return no specified value; both take one
nonnull automaton identity. The manager exposes these state mutations through its
borrowed queue storage, not as primary vtable slots.

| Operation/input state | Mutation and direct calls, in order |
|---|---|
| Add; identity already in list | No mutation, AddRef or predecessor call |
| Add; empty list | Append identity, AddRef it, then call its virtual +0xC4 with null predecessor |
| Add; nonempty list | Remember old tail, append identity, AddRef it, then call its virtual +0xC4 with old tail |
| Remove; identity absent | No mutation or Release |
| Remove; identity last | Unlink/free its node, then Release identity |
| Remove; identity has successor and was first | Call successor virtual +0xC4(null), then unlink/free removed node, Release removed identity |
| Remove; identity has successor and was not first | Call successor virtual +0xC4(previous identity), then unlink/free removed node, Release removed identity |

These two helpers do not update the separate queue count field. Close-time draining
does not call virtual +0xC4 to repair predecessor links. They preserve list traversal
order for surviving entries. Callbacks must return normally and not mutate the same
list; Add's predecessor callback sees the appended list, Remove's sees the old list.
The foreign predecessor operation may have additional effects; only its invocation
and argument are specified here.

## 8. Clock and tick admission boundary

`GetControllerTime()` (primary slot 24, `0x0051A310`) returns a borrowed address at
primary +0x34, with no AddRef. `DoAnimationTick` (`0x00518B80`, two raw message values,
no specified return) reads the cached clock through virtual +0x34 and +0x28 before
checking its controller-update gate. It updates current time and computes a delta;
full floating-point arithmetic remains with the established time contract in
automata.md, not newly specified here. The controller-update gate is exactly:
primary +0x55 nonzero, and either city service +0xAC exists and its virtual +0x18
returns true, or city service +0xB0 exists and its virtual +0x18 returns true. Calls
short circuit in that order (`0x00518C15–0x00518C68`). Regardless of gate result,
current time is copied into previous-time storage +0x3C at the end
(`0x00518DA8–0x00518DB3`). A live cached clock and normal-return callbacks are required;
direct ticking after completed Shutdown is not a supported input state here.

When admitted it first calls UpdateControllers. That helper visits the controller
map and, for each controller whose virtual +0x30(1) returns true, calls grid movement
then controller virtual +0x6C (`0x00518950`, 161 bytes). Tick then walks the map again.
Per controller: if virtual +0x94 returns true, call virtual +0x98 with the second raw
message value, first raw message value, and computed delta pointer. If virtual +0x8C
and then +0x90 both return true, remove that node, notify `EA298D50`, remove from grid,
then Shutdown and Release the controller. Otherwise retain it. This path does not
erase now-empty occupant entries (`0x00518CA0–0x00518DA1`). The tick message branch
(`0x005194F4–0x00519509`) supplies message virtual +0x38 then +0x4C as the two raw inputs.
Controller body semantics and notification-delivery reentrancy remain foreign.

## 9. Literal reference/event traces

All traces are **PROPOSED / UNEXECUTED**; input counts describe chosen raw graphs,
not historical saves. Only manager's direct reference operations are counted. Foreign
Shutdown, notification, grid, service and predecessor replies are assumed to return
normally without modifying the stated maps/lists. Any foreign ref effects must be
added separately; a count reaching zero is an intended final Release, not permission
to invoke further methods on a destroyed object.

1. **Loaded manager, one generator G and one attractor A.** Input: initialized=1,
   loaded=1, controller map one occupant O with list [G,A], each controller refcount=1;
   O refcount=2 (one external + one controller-map key); queue map empty; clock K and
   script S each count=2 (one external + one manager). Output sequence: subscription
   removals; Release(S):2→1; Release(K):2→1; empty queue drain; grid storage clear;
   Shutdown(G), Release(G):1→0; Shutdown(A), Release(A):1→0; free list nodes;
   Release(O):2→1; map clear; initialized=0. No serializer-table reference is present
   in the input and none is needed for these direct operations.
2. **Multiple ownership sites.** Input: controller C occurs once in each of two
   controller-map lists, refcount=3 (two sites + external). Output at close: two
   Shutdown(C)/Release(C) pairs in supplied map traversal order, counts 3→2→1.
   This is a conditional multiplicity trace, not a claim ordinary creation admits
   duplicate controller registrations. Separately O used as key in both maps receives
   two Releases: queue key first, controller key later. Each map key is a distinct site.
3. **Queue plus controllers.** Input: O keys both maps; queue [X,Y], each automaton
   count=2 (queue + external); controller list [G], G count=2; O count=3 (two keys +
   external). Close produces Release(O):3→2, Release(X):2→1, Release(Y):2→1, grid
   storage clear, Shutdown(G), Release(G):2→1, Release(O):2→1. No automaton Shutdown
   or predecessor callback is issued by this drain.
4. **Remove before close.** Same input as trace 3, but a non-skipped RemoveOccupant
   branch arrives first and the QI fails (no temporary QI reference). Group-null removal
   notifies G, removes it from grid, Shutdown(G), Release(G):2→1; then removes its node;
   Release(X):2→1, Release(Y):2→1; Release(O) for queue:3→2; erase queue entry;
   erase controller entry; Release(O):2→1. Later manager Shutdown has empty maps and
   does no second controller/automaton/key Release. Without that notification, trace 3
   applies at manager close, before occupant-manager Shutdown. If the property test
   skips removal, the maps also survive until close.
5. **Empty and repeated close.** Input initialized=1, both maps empty, null cached
   services, no animation/debug/message service available: no virtual object releases,
   empty storage cleanup, initialized→0. Second Shutdown: no cleanup. Final Release
   with manager count=1 calls virtual Shutdown (guard skips), storage destructor/free,
   returns 0. No call after that final Release is a valid repeat operation.
6. **Normal queue mutation.** Input queue empty, X and Y each external count=1.
   Add X: [X], count(X)=2, predecessor(X,null). Add Y: [X,Y], count(Y)=2,
   predecessor(Y,X). Add X again: unchanged. Remove X: predecessor(Y,null) before
   unlink, then [Y], count(X)=1. Remove absent X: unchanged. Manager close later
   Releases Y once: count(Y)=1, without predecessor repair.

## 10. Publication boundary

This supplement establishes a bounded direct owner contract; it is not a complete
substitute-manager specification. AT21 remains open for the residual foreign effects
identified in the question record. Constructor defaults, complete public overload
signatures/filtering/creation admission, remaining message branches, numeric time
arithmetic, callback mutation and failure rollback are not closed by this publication.
Successful creation's owner-side tail is directly visible: it inserts the factory's
controller reply into the occupant list; a newly inserted nonnull occupant key is
AddRef'd; it attaches occupant or supplied position, assigns template, calls controller
Init, then grid AddController, and returns the same controller pointer
(`0x0051AF6E–0x0051AF8E`, `0x0051B043–0x0051B094`, `0x0051B108–0x0051B14D`).
This tail has no additional manager AddRef of that controller. It does not establish
the factory's internal construction or reference contract, or admit arbitrary repeated
registrations. Earlier creation-admission tests remain unclosed here.

The existing serialized record in automata.md §3 is prior input, not new coverage here.
In a loaded city whose serializer object table has gone away, the manager's direct
controller-list Releases still follow §5; no object-table reference is required by
the loop. Whether each direct Release is the last reference depends on the input graph
and the called controller's own contract.

## 11. Individually measured body/status ledger

Sizes are exported instruction-body bytes, not address-span subtraction (some bodies
contain alignment holes). Each entry below is one canonical body; shared helpers and
interface adjustment thunks are not counted again as manager bodies. This is an exact
ledger of the **39 candidates inventoried here**, totaling **10,884 bytes**: 36 named
candidates / 10,274 bytes, plus the three independently traced manager destructor/map
cleanup bodies / 610 bytes. It is not proof of the provisional 58-body / 13,229-byte
attribution, nor a complete native-family census.

Status **P** means the direct bounded behavior is published above; **B** means a named
boundary/branch is published but the remainder is unclosed; **U** means measured address
and size only, no new behavior credit. Names on U rows remain export attribution rather
than independent proof of owner/signature. Read/Write are established prior record
inputs and carry no new record-byte credit.

| Entry | Bytes | Role / status |
|---|---:|---|
| 00514CA0 | 53 | AddControllerFromTemplateName — U |
| 00514CE0 | 48 | GetController overload — U |
| 00514D10 | 115 | QueryInterface — U |
| 005163D0 | 56 | AddControllersFromOccupantGroup candidate — U |
| 00516410 | 275 | GetController overload — U |
| 00516530 | 490 | Composite attraction effect — U |
| 00516720 | 223 | Composite commotion — U |
| 00516800 | 71 | ReleaseControllerList — U |
| 00517060 | 233 | AddControllers — U |
| 00517150 | 54 | AddControllersFromOccupantGroup overload — U |
| 00517190 | 54 | AddControllersFromOccupantGroup overload — U |
| 005173A0 | 128 | GetController overload — U |
| 005174D0 | 799 | Write — U / prior record |
| 00517880 | 317 | RemoveController overload — U |
| 005179C0 | 201 | Relevant controllers — U |
| 00518660 | 381 | Loaded PostCityInit — P (§6) |
| 005187F0 | 204 | Debug command message — U |
| 005188C0 | 103 | MoveControllers — U |
| 00518930 | 31 | MoveController — U |
| 00518950 | 161 | UpdateControllers — P (§8, foreign grid primitive) |
| 00518B80 | 573 | Animation tick — B (§8, numeric time unclosed) |
| 00519360 | 1340 | Message target — B (§6/§8 branches only) |
| 0051A280 | 125 | Manager storage destructor — P (§5) |
| 0051A300 | 6 | GetGZCLSID — U |
| 0051A310 | 4 | Borrowed controller time — P (§8) |
| 0051A350 | 452 | Group removal — B (§6 group=null path only) |
| 0051A520 | 15 | RemoveControllers forwarding — P (§6) |
| 0051A530 | 441 | Name removal — B (§6 call-order distinction only) |
| 0051A6F0 | 217 | Queue-map teardown — P (§4) |
| 0051AB00 | 49 | Group removal overload — U |
| 0051AB40 | 49 | Group removal overload — U |
| 0051AC10 | 268 | Grid/controller-map teardown — P (§5) |
| 0051AD20 | 1176 | Template creation — B (§10 successful owner tail only) |
| 0051B1C0 | 44 | Primary Release — P (§5) |
| 0051B3E0 | 311 | Shutdown — P (§3) |
| 0051B520 | 1197 | Read — U / prior record |
| 0051B9D0 | 215 | Borrowed queue access — P (§7, queue-point primitive opaque) |
| 0051C140 | 397 | Init — P (§3) |
| 00695330 | 8 | Folded AddRef candidate — U; shared code, not independently credited |

Supporting bodies, excluded from the owner total: queue AddToQueue `005184E0` / 102
bytes, RemoveFromQueue `005172D0` / 103 bytes and drain `00517010` / 76 bytes are
published manager-storage operations (§4/§6/§7), not three extra service methods.
The generic map clear `006F2970` / 168 bytes and list-vector destruction `006F1EA0` /
78 bytes were instruction-checked for storage-only behavior at these call sites;
range helper `00518580` / 61 bytes receives an empty range here. No foreign controller,
automaton, grid-geometric, allocator or message implementation bytes are credited.
