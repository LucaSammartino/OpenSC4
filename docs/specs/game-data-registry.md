# Game-data registry

First owner contract, Windows GOG 1.1.641.0, 2026-10-23. This is a bounded static
contract, not whole-owner completion. Addresses refer to image base `0x00400000`.
The export summary, `re/analyze.json` subject and the working executable agree on
SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
The executable version resource says `1.1.641.0`, original filename `SimCity 4.exe`.
Instructions queried through `tools/re_query.py` are primary evidence; local
pseudocode is supporting evidence only. Host admission is recorded separately;
no live REA target readiness or native execution is asserted.

## 1. Boundary and interface

The registry owns opaque entry records, two key indexes, child lists and notification
subscriptions. A node contains an external reference-counted object; a leaf contains
a callback and a raw value/key. The registry forwards primitive exchange/call requests
to those callbacks. It does not implement budget values, variant-to-Lua conversion,
Lua tables, or command execution. Those belong to publishers and consumers
([budget §4.4](budget.md), [Lua host §9](lua-host.md)).

The 29 compiled slots are at `0x00A8CBF8`. The typed SDK header is useful for names
and callback types but its declaration order is **not** the compiled overload order.
Foundations §17.7 gives the compiled order of the overloaded groups; the bodies
below confirm it. `GetNode` returns the stored object pointer, despite the SDK
declaring `cEntry*`. Neither it nor `GetValue` returns a newly retained reference.

| Slot | Native body | Contract/name |
|---|---|---|
| 0 | `00473A40` | QueryInterface |
| 1 / 2 | `00800C30` / `00800C40` | shared adjusting AddRef / Release |
| 3 | `00474C50` | AddChildEntry(raw key, name, call callback, parent, flags, context) |
| 4 | `00474B30` | AddChildEntry(raw key, name, exchange callback, parent, flags, context) |
| 5 | `00474A50` | AddChildEntry(object key, name, parent, flags, context) |
| 6 / 7 | `00474DA0` / `00474D70` | RemoveChildEntry(raw key / object key) |
| 8 / 9 | `0080E040` / `00474270` | GetEntryID(raw key / object key) |
| 10 / 11 | `00473AA0` / `00473AC0` | GetNode / GetValue |
| 12 / 13 / 14 | `00473F00` / `00473AE0` / `00473B00` | IsValidEntry / IsNodeEntry / GetEntryName |
| 15 / 16 | `00473CE0` / `00473B20` | DoValueExchange / DoValueCall |
| 17 / 18 / 19 | `00473B70` / `00473B90` / `00473B50` | SetEntryContext / GetEntryContext / CheckEntryFlags |
| 20 / 21 | `00473BB0` / `004ED900` | GetParentNodeEntry / GetRootNodeEntry |
| 22 / 23 | `004750F0` / `00473E60` | VisitBreadthFirst / VisitImmediateChildren |
| 24 / 25 | `00475030` / `00474F70` | AddNotificationTarget(message target / typed target) |
| 26 / 27 | `00474020` / `00473FB0` | RemoveNotificationTarget(message target / typed target) |
| 28 | `00475630` | DebugDumpRecursive; formatting remains partial |

These are 32-bit x86 interfaces: keys, handles, pointers, contexts, flags and callback
arguments occupy 4-byte words; Boolean results are tested in AL. A false AL does not
promise EAX is all zero. There are no HRESULT error channels in these methods.
An entry ID is its allocated record address, not a sequence number, name or hash.
No public path-string lookup occurs in these 29 slots. Dotted Lua access is a
consumer operation. No case folding, separator parsing or name uniqueness rule is
established for an additional private path facility.

## 2. Service identity and lifetime

Construction (`004748F0`) constructs the system-service face at object +4 with
service ID `0xEBE0E860` and signed priority -1000000 (`0xFFF0BDC0`); installs the registry face at
+0 and another framework face at +8. The app directly calls this constructor
at `004507AE` during AddSystemServices. This is evidence of app construction,
not proof of a separately callable registry factory. The SDK also names
`0xEBE0E860` as CLSID; factory registration is not proved by that label. The base
constructor `0091DE0F` stores the first word as service ID and the second as
both service and tick priority, and initializes the running byte to false.

QI (`00473A40`) first zeroes the required output pointer, recognizes registry IID
`0xE898ED03`, writes the +0 face and AddRefs it, returning true. Other requests
delegate to the system-service QI on +4. The +4 and +8 QI thunks (`00474670`,
`00474680`) adjust back to the registry face. Shared AddRef/Release thunks adjust
+4 and then delegate to the system-service implementation, whose refcount face
is another +4. The shared bodies are not draw-system-owned behavior merely because
the export names them after a draw-system method. Exact base-class deletion and
unsupported-IID policy follow the base bodies: QI recognizes system-service IID
`0x287FB697` at +4, then IID 1 at +8 (`0091DD67`, `009457C6`), retaining the
returned face; unsupported IDs return false with the wrapper’s output still null.
AddRef increments the 32-bit counter at registry +0x0C and returns it (`00968B60`).
Release decrements/returns a count greater than 1; at count 0 or 1 it invokes
the +8 face’s deleting slot with argument 1 and returns zero (`0090D997`). The
+8 deleting thunk `00474690` adjusts back to the allocation.

The constructor creates two empty ordered key indexes (+0x24 node, +0x30 leaf)
and two empty target lists (+0x3C typed, +0x40 message). It allocates an 8-byte
unknown object as the root key, constructs a 0x30-byte node named `__root__`
with null parent and flags zero, stores it at +0x20 and indexes it by that key
(`00474992`–`00474A1F`). The unknown constructor starts its 32-bit reference
count at zero (`0090DA1E`); the node constructor’s AddRef makes the root key
count 1. There is no extra constructor-local retained reference to balance.
Root destruction releases that one reference. The registry’s own base counter
also starts at zero through this unknown constructor.

The framework-init hook `00473A80` (service-face slot 8) registers its service face with framework
slot +0x24 and returns true. Shutdown `004746A0` (service-face slot 9) receives the +4 face: first
framework slot +0x28, then broadcast `(type=1, entry=null)`, clear both key
indexes, recursively destroy all root children, clear both subscription lists,
and return true. It preserves the root record and its name/object/child-list
container; the emptied node index does **not** reinsert the root key. Thus
GetRootNodeEntry still returns the root, while IsValidEntry(root) becomes false
after shutdown. No restart guarantee follows.

Destruction (`00474580`) destroys the root tree, clears/releases subscriptions,
destroys indexes and delegates base destruction. The deleting wrapper `00474A30`
also frees the service allocation when its flags low bit is set. Destruction
does not itself call the shutdown broadcaster. Entry destruction `00474350`
releases node object keys, recursively destroys children in list order and frees
their records/list cells, then destroys the name storage. Leaves do not retain
or release raw key, callback or context pointers.

## 3. Entry identity, creation and access

An entry has a one-byte kind at +0x2C: 1 for node, 0 for callback leaf. Common
fields are name at +8, parent at +0x20, mutable context at +0x24 and 32-bit flags
at +0x28. A node has its child-list container at +0 and retained object key at
+4. A leaf has its callback at +0 and raw value/key at +4. Constructors
`004743E0`, `00474460`, `004744E0` establish these fields; neither leaf kind
has separate runtime tagging beyond flags bit 0.

### 3.1 Creation and duplicate keys

- **Node, slot 5:** resolve the object key through slot 9 first. If found,
  return the existing entry, without changing its name, parent, flags or context
  and without notification. Otherwise allocate/construct a node, retaining the
  supplied object pointer once; store the caller context; insert into the node
  index; append to the supplied parent child list; synchronously broadcast type 3;
  return the entry (`00474A50`–`00474B23`). Unlike the leaf overloads, this body
  does not call IsNodeEntry on the parent before dereferencing its child list.
- **Exchange leaf, slot 4:** an already indexed raw key returns null. Otherwise
  IsNodeEntry(parent) must be true; failure returns null. Construct a leaf with
  flags bit 0 cleared, set context, index it, append to parent, notify type 3 and
  return the entry (`00474B30`–`00474C4F`).
- **Call leaf, slot 3:** same rules, but forces flags bit 0 on
  (`00474C50`–`00474D6E`). Both constructors also enforce their flag bit.
- **Null leaf key:** allowed. Before indexing, replace key with the allocated
  entry address and store that address as the leaf value (`00474BE1`–`00474BED`,
  mirrored in the call overload). Therefore GetEntryID(null raw key) stays null;
  GetEntryID(the returned entry address as raw key) finds the leaf.

Names are copied into entry string storage; indexing is by pointer-sized key,
not name. Two distinct keys with the same name are allowed by the creation
bodies; no name comparison occurs. Node and leaf key spaces are separate, so
the same literal 32-bit key can identify one of each. Parents are not checked
for membership in this registry: leaf creation checks only non-null/kind==1.
Foreign, stale and fabricated pointers are not safe inputs. Null name behavior
and allocation failure are not defined as ordinary recoverable failures here:
the creation bodies can dereference a null allocation after their construction
branches. Do not promise an atomic null-return allocation-failure path.

### 3.2 Lookup and access channels

Raw-key lookup is an exact index lookup, returning a borrowed entry or null
(`0080E040`). Object-key lookup first tries the exact pointer; on a miss and
non-null key, it QIs the key for IID 1 (canonical unknown), searches that pointer,
and releases the QI result before returning (`00474270`). Creation stores the
original supplied key, not the canonicalized one: the fallback only finds a
canonical pointer that was itself indexed. A failed QI returns null, with any
non-null QI output released. Missing keys do not insert entries.

| Operation | Null entry | Non-null entry |
|---|---|---|
| GetNode | null | +4 if kind==1, otherwise null; no AddRef |
| GetValue | null | +4 if kind!=1, otherwise null |
| IsNodeEntry | false | kind==1 |
| GetEntryName | null | borrowed address of embedded string at +8 |
| SetEntryContext | false | store full 32-bit context; true |
| GetEntryContext | null | stored context |
| CheckEntryFlags(mask) | false | true exactly when `(flags & mask)==mask` |
| GetParentNodeEntry | null | stored parent; root parent null |
| GetRootNodeEntry | n/a | service +0x20, with no reference increment |

CheckEntryFlags(0) is true for every non-null record, including nodes. These
accessors do not call IsValidEntry. IsValidEntry instead scans the **values** of
both key indexes for pointer equality (`00473F00`), so it need not dereference
the candidate pointer. Removed handles fail membership; they are not safe for
the other accessors, nor guaranteed never to alias a later allocation.

### 3.3 Primitive callback dispatch

DoValueCall(non-null entry, input, output) calls the function stored at +0 as
`(stored +4 value, input, output, registry)` and returns the callback Boolean.
Null entry or null callback returns false. It does not check kind, membership,
flags bit 0 or context (`00473B20`). The first callback argument is the raw key,
**not** GetEntryContext; context remains separately available through the registry.

DoValueExchange similarly calls `(stored +4 value, variant, set, registry)`.
If `set` is true and flags bit 1 (`0x2`) is set, it returns false without calling
the callback. Reads are allowed through that flag. Null entry or null callback
returns false (`00473CE0`); the null-callback branch constructs/destroys a local
variant but does not overwrite the supplied variant. Neither routine validates
the callback signature against the chosen dispatch method. Publisher callbacks
own numeric conversion, value storage, output parameter contents and failure
side effects. These forwarding routines do not broadcast value-change events.

## 4. Child order, traversal and removal

Successful creation appends a list cell immediately before the parent sentinel.
VisitImmediateChildren visits first-to-last creation order, excluding the supplied
parent. Null, leaf, absent child container or empty list is a successful empty
visit. A callback `(child, context)` returning false stops the visit and returns
false; otherwise true. The next list cell is fetched before the callback
(`00473E60`). This is not a snapshot or an arbitrary-reentry safety guarantee.

VisitBreadthFirst (slot 22, `004750F0`) includes the supplied entry, then queues
children first-to-last, with FIFO processing. Null start returns true without
callbacks; a leaf start invokes the callback once. False callback stops and
returns false before enqueuing that entry’s children. Queue allocations contain
raw entry pointers, with no entry retention. Mutating the traversed tree can
invalidate queued pointers.

Registration uses a different internal traversal (`00474DE0`): a LIFO vector.
It visits the start first, then appends children first-to-last and pops the last
one, so siblings are visited in **reverse insertion order**, depth first. False
visitor stops with false; null start succeeds. Registration wrappers ignore this
traversal result. Their adapters always return true regardless of target callback
return values (`00473BD0`, `00473BF0`).

Removal overloads resolve their key, return false on a miss, then invoke
`00474780`. Removing the root returns false. For any other resolved entry:

1. Collect subtree entries in depth-first **postorder**, following each node’s
   children first-to-last (`00473DF0` plus collector `00474310`).
2. For each collected entry: synchronously broadcast type 2, then erase its key
   from the node or leaf index. The notified entry is still indexed during its
   own callback; previously notified descendants have already been unindexed.
3. Find the original subtree entry in its parent’s list, recursively destroy it
   and all descendants, then unlink/free that parent list cell. Return true.

Thus entry records remain allocated throughout the notification/index-erasure
pass, but membership changes between notifications. Destruction releases node
keys in parent-before-child order, distinct from removal notification order.
Callbacks can execute arbitrary foreign code; the algorithm neither snapshots
targets nor locks tree mutation. Reentrant removal of the same subtree or a
queued sibling has no demonstrated safe outcome. There is no proven deferred
mutation protocol or rollback on callback failure.

## 5. Notification targets and replay

Typed and message targets are held in separate append-order circular lists.
Registration searches for the exact supplied interface pointer, temporarily
AddRef/Release around the search; an existing pointer returns false without
another list reference or replay. A new cell owns one net reference. After
linking it, registration synchronously replays the whole root tree **only to the
new target**, using the LIFO traversal in §4, including the root. It returns true
after replay. The target is already subscribed during replay, so reentrant
creation can cause ordinary broadcasts as well as replay visits. No exactly-once
or snapshot rule is proved under mutation. Null targets are not rejected before
insertion, and replay dereferences the target; null registration is not a safe
successful no-op (`00474F70`, `00475030`).

Typed replay calls target slot +0x0C with `(registry, 3, entry)`. Message replay
constructs a standard message, sets type `0x2BDFE414`, puts registry through
message slot +0x30, event code 3 through +0x40, and entry through +0x54; it
directly invokes target slot +0x0C then releases the message. No message-server
posting or delayed delivery occurs (`00473BF0`). Exact message accessor names
are delegated to the standard-message interface.

Unregistration searches by exact pointer, returns false if absent; otherwise
unlinks the first matching cell, releases its stored reference, frees the cell
and returns true (`00473FB0`, `00474020`). It sends no shutdown/removal replay.

**Ordinary broadcast quirk:** `00473D40` walks typed targets in registration
order and directly calls `(registry, type, entry)`, fetching next cell before
each call and ignoring callback return. In normal nonmutating execution its
typed iterator equals the typed sentinel when this loop finishes. Instructions
`00473D6B`–`00473D72` then compare those same pointers and branch to return on
equality, skipping the entire message-target broadcast loop. This also skips
message targets when the typed list is empty. Therefore message targets receive
registration replay, but no ordinary add/remove/shutdown events on this normal
path. Do not infer delivery from the presence of the unreachable message-building
loop. No claim is made for callbacks that corrupt or invalidate iteration state.

Lua-host Q7 is resolved for nonmutating registration: existing entries are replayed
synchronously, including root, in the reverse-sibling depth-first order above.
The Lua consumer’s table handling and return behavior remain in its own spec.

## 6. Literal conformance vectors

These are independent expected native outcomes, not results of running an
implementation. Assume successful allocations, valid pointers and callbacks
that do not mutate the registry unless stated.

| Vector | Setup/action | Expected |
|---|---|---|
| G1 identity | node key `0x11110000` → N; leaf same raw key → L | N != L; object lookup returns N, raw lookup returns L |
| G2 duplicates | add node key again with new name/context; add leaf key again | node returns N unchanged, no event; leaf returns null, no event |
| G3 anonymous leaf | add leaf with raw key 0 → L | stored value L; raw lookup 0 is null; raw lookup L returns L |
| G4 ordering | root children A then B; A children X then Y | immediate root: A,B; breadth first root: root,A,B,X,Y; replay: root,B,A,Y,X |
| G5 cancellation | G4 BFS callback returns false at A | visits root,A; false; neither B nor X nor Y visited |
| G6 flags | exchange flags `0xFFFFFFFF`; call flags 0 | stored exchange flags `0xFFFFFFFE`; call flags 1; exchange set rejected by bit 1; exchange read may call callback |
| G7 contexts | leaf raw key `0x22220000`, context `0x33330000` | callback first argument `0x22220000`; GetEntryContext gives `0x33330000`; setting it to 0 succeeds |
| G8 missing/null | missing raw/object key; null entry accessors | lookups null; GetName/GetNode/GetValue/parent/context null; SetContext/CheckFlags/IsNode false; empty visits true |
| G9 removal | remove A from G4 | notifications X,Y,A; during X callback X,Y,A indexed; during Y callback X absent, Y,A indexed; during A callback X,Y absent, A indexed; afterwards all absent, root child list B |
| G10 target refs | new typed target T, duplicate T, remove T, remove T | net list ownership +1, +0, -1, +0; results true,false,true,false; duplicate/remove send no replay |
| G11 delivery | message M subscribed, then typed T; create leaf Z | each target gets its own registration replay; creation delivers `(registry,3,Z)` to T; no ordinary event to M |
| G12 shutdown | populated registry, call shutdown | typed event `(registry,1,null)` before index clearing; both indexes empty; root survives; child keys released and subscriptions released; root lookup by key now misses |
| G13 node references | object K starts with count 4; create node N; duplicate K; remove K | counts 5,5,4; duplicate performs no new node ownership; removed node handle fails membership |
| G14 service views | allocation R, QI for registry/system-service/unknown IIDs | returned faces R, R+4, R+8; each increments shared counter at R+0x0C; unsupported IID false and output null |

No vector licenses unsafe pointers, allocation failures, reentrant deletion, or
name-based paths; those need separate evidence.

## 7. Body audit and remaining work

The public “48 functions / 5807 bytes” attribution is an inventory claim, not a
completion claim. The per-body reconciliation and exact status are maintained
below; shared library helpers, thunks and diagnostic visitors must not acquire
new tree semantics just from proximity or a recovered name.

The contiguous export interval `00473A40` through `00475630` contains 51 bodies,
6794 bytes. Removing the separately attributed nested-entry constructor
`004744E0` (159), shared list clear `00473EC0` (60), and dump visitor `00475330`
(768) reproduces **exactly 48 bodies / 5807 bytes**. This reconciles the public
total without treating all 48 as distinct owner algorithms.

S = supported bounded behavior in the cited section; P = partial body or
delegated helper contract; F = generic/foreign primitive, not an additional
registry algorithm. None promises arbitrary reentry or invalid-pointer safety.
Bytes are export body sizes, excluding gaps.

| Body | Bytes | Status | Contract |
|---|---:|---|---|
| `00473A40` | 54 | S | QI, §2 |
| `00473A80` | 27 | S | framework registration hook, §2 |
| `00473AA0` | 25 | S | GetNode, §3.2 |
| `00473AC0` | 25 | S | GetValue, §3.2 |
| `00473AE0` | 27 | S | IsNodeEntry, §3.2 |
| `00473B00` | 19 | S | GetEntryName, §3.2 |
| `00473B20` | 42 | S | call forwarding, §3.3 |
| `00473B50` | 32 | S | full-width flag mask, §3.2 |
| `00473B70` | 25 | S | context write, §3.2 |
| `00473B90` | 19 | S | context read, §3.2 |
| `00473BB0` | 19 | S | parent read, §3.2 |
| `00473BD0` | 27 | S | typed replay adapter, §5 |
| `00473BF0` | 108 | P | message replay; allocation failure unsafe, accessor types delegated |
| `00473C60` | 51 | F | recursive node-index RB-tree cell free; does not destroy entries |
| `00473CA0` | 51 | F | corresponding leaf-index cell free |
| `00473CE0` | 82 | S | exchange forwarding, §3.3 |
| `00473D40` | 165 | S | typed broadcast and skipped message branch, §5 |
| `00473DF0` | 108 | S | removal postorder traversal, §4 |
| `00473E60` | 88 | S | immediate visit, §4 |
| `00473F00` | 175 | S | membership scan, §3.2 |
| `00473FB0` | 106 | S | typed unsubscribe, §5 |
| `00474020` | 106 | S | message unsubscribe, §5 |
| `00474090` | 189 | F | node-index range erasure; generic RB-tree rebalance delegated |
| `00474150` | 83 | F | unsigned 32-bit key equal-range bounds used by removal |
| `004741B0` | 189 | F | corresponding leaf-index range erasure |
| `00474270` | 152 | S | object lookup/canonical QI fallback, §3.2 |
| `00474310` | 54 | F | raw-pointer vector append adapter used for removal |
| `00474350` | 143 | S | entry destruction, §2 |
| `004743E0` | 114 | S | exchange leaf constructor, §3 |
| `00474460` | 114 | S | call leaf constructor, §3 |
| `00474580` | 230 | P | registry destructor; base teardown delegated |
| `00474670` | 8 | S | +4 QI adjustment, §2 |
| `00474680` | 8 | S | +8 QI adjustment, §2 |
| `00474690` | 8 | S | +8 deleting adjustment, §2 |
| `004746A0` | 213 | S | shutdown hook, §2 |
| `00474780` | 356 | S | subtree removal, §4 |
| `004748F0` | 316 | P | service/root construction; allocator/partial-failure behavior unresolved |
| `00474A30` | 30 | S | deleting wrapper, §2 |
| `00474A50` | 212 | S | node creation, §3.1 |
| `00474B30` | 288 | S | exchange creation, §3.1 |
| `00474C50` | 287 | S | call creation, §3.1 |
| `00474D70` | 37 | S | object-key removal wrapper, §4 |
| `00474DA0` | 58 | S | raw-key removal wrapper, §4 |
| `00474DE0` | 385 | S | LIFO replay traversal, §4 |
| `00474F70` | 183 | S | typed registration/replay, §5 |
| `00475030` | 183 | P | message registration; standard-message accessor types delegated |
| `004750F0` | 555 | S | FIFO breadth-first visit, §4 |
| `00475630` | 31 | P | null no-op; otherwise LIFO visit with dump visitor; output unknown |

Additional bodies outside the 48: `004744E0` supports node construction (§3);
`00473EC0` is generic target-list clear (releases non-null targets, frees cells,
resets sentinel); `00475330` diagnostic formatting remains unknown. Shared slot
bodies `00800C30`, `00800C40`, `0080E040`, `004ED900` are specified above but
not newly attributed owner functions. The export labels “GetOrdinanceByID” and
“GetTractCountX” at the last two do not change their compiled registry role.

Precise unknowns are in [questions/game-data-registry.md](questions/game-data-registry.md).
No runtime tests, engine source, Lua mirror outputs or Rust collection behavior
were used as an oracle.
