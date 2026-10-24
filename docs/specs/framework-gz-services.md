# Framework services (GZ): message server 2, message queues

Behaviour spec for the GZ framework's second-generation messaging service: the message server
(`cGZMessageServer2`), its FIFO message queue (`cGZMessageQueue2`) and the named-queue registry
(`cGZMessageQueueManager2`). Written from the exe (SimCity 4 Deluxe 1.1.641) by lane re-gzservices4,
2026-10-21. Evidence: Ghidra export instructions (`tools/re_query.py asm`) and the vtables read from
`bin/SimCity4.exe`; the REA session was unavailable this round (no binary open), so no REA pseudocode
was used. Addresses are VAs in that exe.

Outline: §1 owners and identity. §2 message queue. §3 queue manager (including §3.1 observation
limits and §3.2 optional-service acquisition, GZ11/GZ12). §4 message server 2
(§4.1 layout, §4.2 notification registry, §4.3 send, §4.4 post and tick, §4.5 directed (general)
messages, §4.6 message creation, §4.7 service lifecycle). §5 worked example. §6 open questions.

## 1. Owners and identity

| Owner class | Bytes (coverage owners) | Functions | Identity |
|---|---|---|---|
| `cGZMessageServer2` | 2,780 | 38 | service id `0x04FA845B`, interface `cIGZMessageServer2` iid `0x652294C7`, tick priority 5,000,000 |
| `cGZMessageQueue2` | 1,941 | 30 | queue object, 0x1C bytes, vtable `0x00AD1AD0`, 25 slots |
| `cGZMessageQueueManager2` | 951 | 17 | service id `0x64FA8434`, iid `0x652294BC`, tick priority 5,000,000 |
| `cGZMessageServer` | 5,796 | 81 | service id `0x64063387`, interface `cIGZMessageServer` iid `0xC1085722` (§4A) |

Owner bytes are from `tools/coverage.py --owners` (subsystem "framework services (GZ)") at main
`ab23a7fa`. The messages themselves are `cRZMessage2Standard` (RZ side, not in this spec); a message
exposes, in its own vtable, `GetType` at +0x10 and an equality test against another message at
+0x18. A message target exposes `DoMessage(message)` at +0x0C.

Reference counting: server 2's queue slots, registry entries and pending entries hold counted
references (AddRef on store, Release on removal). Every "out" message pointer handed back to a
caller is AddRef'd for that caller. Server 1's directed records and pending removals are raw
pointers; its pending adds have the special transfer rules in §4A.

## 2. Message queue (`cGZMessageQueue2`)

State: a doubly linked list of message references (front = oldest for `PushBack`), a critical
section, a thread signal, and an own reference count starting at 0 (constructor `0x0092DD7E`). Every
operation except AddRef/Release runs under the critical section.

**Reference count.** AddRef (`0x0093EC9F`) increments and returns the count. Release (`0x0092CFAA`):
if the count is above 1, decrement and return it; otherwise (count 0 or 1) destroy the queue through
its deleting destructor (slot 23) and return 0.

**Vtable slots (behaviour by slot).** The docs/abi names for slots 5-10 do not match the behaviour;
the table gives the behaviour (proposed names in `build-runs/abi-rows-re-gzservices4.tsv`).

| Slot | Address | Behaviour |
|---|---|---|
| 3 | `0x0092DDD2` | PushBack(msg): append; if the queue was empty before the append, set the signal. Returns nothing. |
| 4 | `0x0092DE2C` | PushFront(msg): prepend; same signal rule. |
| 5 | `0x0092D78B` | Remove first by type: scan front to back, remove the first message whose `GetType` equals the argument; true if one was removed. |
| 6 | `0x0092D66F` | Remove first equal message: scan front to back with the message equality test; remove the first match; bool. |
| 7 | `0x0092D7E6` | Remove last by type: scan back to front, remove the first match met (the newest); bool. |
| 8 | `0x0092D6CC` | Remove last equal message: back to front, equality test; bool. |
| 9 | `0x0092D842` | Remove all by type: remove every match; returns the number removed. |
| 10 | `0x0092D72A` | Remove all equal messages: returns the number removed. |
| 11 | `0x0092DB79` | RemoveAll: empty the queue; returns how many were in it. |
| 12 / 13 | `0x0092D313` / `0x0092D34B` | Type of the front / back message, 0 when empty. |
| 14 | `0x0092D384` | True if any message has the given type. |
| 15 / 16 | `0x0092D3CE` / `0x0092D414` | Peek front / back: write the message (AddRef'd) to the out pointer, or null; true if not empty. The message stays queued. |
| 17 | `0x0092D45B` | Peek anywhere (msg, out): first message equal to the argument, front to back; AddRef'd copy or null; bool. |
| 18 / 19 | `0x0092D8A1` / `0x0092D8F0` | Get front / back: as peek, then remove it from the queue (the caller now owns one reference). |
| 20 / 21 | `0x0092CFC1` / `0x0092CFF7` | Blocking get front / back: call slot 18 / 19; while it fails, wait on the signal with timeout 1,000,000 and retry. Always returns true. |
| 22 | `0x0092F821` | Message count. |
| 23 | `0x0092E27C` | Destructor (deleting when bit 0 of the flag is set): releases every queued message. |
| 24 | `0x0092D940` | Get anywhere (msg, out): as slot 17, then remove the found message. |

**Equality receiver (GZ7).** All five equality operations invoke the queued message's slot 6
(vtable +0x18), passing the search argument: `queued.equals(argument)`. Only the low byte of the
return value is tested. Equality need not be symmetric. Native call sites: `0x0092D698`,
`0x0092D6F6`, `0x0092D756`, `0x0092D482`, `0x0092D969` (slots 6/8/10/17/24 respectively).

**Blocking signal primitive (GZ2).** The embedded signal is constructed with a 32-bit mode of 0
(`0x0092DD9D–0x0092DDA2`). Its vtable `0x00AD1F48` has Signal at slot 1 (`0x00932DA7`),
Pulse at slot 2 (`0x00932D95`), Wait at slot 3 (`0x00932DE0`), and handle-valid test at slot 4
(`0x00932E3A`). Constructor `0x00932E65` allocates 0x24 bytes of platform state and calls
CreateEventA with no name, initially nonsignalled, manual-reset iff mode is nonzero.

Wait takes an **unsigned 32-bit microsecond timeout**: `0xFFFFFFFF` is passed through as infinite;
every other value is divided by 1,000 with unsigned division, discarding the remainder
(`0x00932DF6–0x00932E11`). Thus 999 means a zero-millisecond poll, 1,000 means 1 ms, and the
queue's 1,000,000 means **1 second**. Wait holds the signal platform critical section across
WaitForSingleObject; it returns dword 0 for WAIT_FAILED, 2 for WAIT_TIMEOUT (0x102), otherwise 1.
It does not explicitly call ResetEvent. The queue ignores this return and retries its get until
successful (`0x0092CFC1`, `0x0092CFF7`); it has no timeout/failure return to its caller.

Signal's mode-0 branch calls **PulseEvent**, returning normalized Win32 success. Its nonzero-mode
branch calls SetEvent but returns 0 even on success (`0x00932DA7–0x00932DCD`). Slot 2 always pulses.
The queue calls Signal only on an empty-to-nonempty transition. Consequently a pulse without an
eligible waiter leaves no persistent notification: a producer can insert between a failed get
and the wait, and the consumer then finds that message only after its wait times out. Do not
model this as a permanently set event. The unused reset body `0x00932DCE` calls ResetEvent; it
is not one of these five vtable slots.

The signal is an embedded, uncounted resource. Queue destruction closes its event and destroys
its platform critical section/string and frees platform state before destroying the queue lock
and releasing queued messages (`0x0092DDB0`, `0x00932EC8`, `0x00932EB4`). There is no waiter join,
cancellation wake, or deferred close. A live blocking call therefore requires the queue to remain
alive. Allocation failure is not recovered (constructor dereferences null platform state);
CreateEventA failure leaves a null handle, detectable by slot 4, but the queue never tests it.

## 3. Queue manager (`cGZMessageQueueManager2`)

A system service (constructor `0x0092E714`) holding a sorted map from a 32-bit queue id to a queue
reference, guarded by a critical section. Vtable `0x00AD1B80`:

| Slot | Address | Behaviour |
|---|---|---|
| 0 | `0x0092D033` | QueryInterface: iid `0x652294BC` returns this (AddRef'd); others go to the system-service base. |
| 3 | `0x0092DBB1` | GetQueue(id, out): AddRef'd queue or null; true if found. |
| 4 | `0x0092E37A` | AddQueue(id, queue): insert when the id is absent (the map takes a reference), true; false when the id exists (nothing changes). |
| 5 | `0x0092E2E3` | CreateQueue(id): when absent, allocate a new empty queue (§2) and insert it, true; false when present. |
| 6 | `0x0092DED7` | RemoveQueue(queue): linear scan for the entry holding that queue pointer, erase it, true; false if none. |
| 7 | `0x0092DE86` | RemoveQueue(id): erase the entry, true; false if absent. |
| 8 | `0x0092E3FC` | RemoveAllQueues: clear the map; always true. |
| 9 | `0x009DB663` | GetQueueCount: return the unsigned 32-bit number of registered IDs, without acquiring the manager lock. |

Erasing releases the map's reference; a queue nobody else holds is destroyed then.

### 3.1 Public observation limits (GZ11)

There is **no complete read-only enumeration or snapshot interface on these owners**. The
manager exposes lookup by supplied ID and a count, not iteration over IDs. Its last slot,
slot 9 at vtable `0x00AD1BA4`, reads the map's stored count at manager +0x30, with no lock
(`0x009DB663`–`0x009DB666`). This is the number of **ID entries**, not distinct queue objects
or messages: two IDs may hold the same queue pointer. Construction clears that count
(`0x0092E73D`, `0x00892027`); insertion increments it (`0x0093804E`), erasure decrements it
(`0x00968DBD`), and RemoveAll clears it (`0x0092DC7B`). No extra slot supplies the missing
keys. The inherited system-service and reference-count faces expose identity, lifetime,
running state and lifecycle callbacks, not queue enumeration (tables `0x00AD1B4C` and
`0x00AD1B34`). Thus the answer is a limitation of the native interface, not an unknown
enumerator signature to guess. A complete live shadow cannot be seeded through these
interfaces alone; no complete public mutation-observer hook is supplied either.

| State to observe | Available public observation | Limit |
|---|---|---|
| Manager registry | GetQueue(id, out), GetQueueCount | Lookup needs a known ID. Count supplies no IDs or identities. |
| A held queue | Count (slot 22), front/back type, front/back peek, type/equality searches (§2) | No index/iterator or nondestructive traversal of arbitrary contents. Equality search needs a supplied message and returns only its first match. Taking and reinserting messages changes the queue and its references/signals. |
| Server-2 ordinary posted messages | Posted-message count (slot 10) | Counts the embedded ordinary post queue only; supplies neither message identities/order nor the separate directed deque. |
| Server-2 directed posts and pending registration changes | No enumeration | Post/cancel/send/tick and add/remove notification are actions, not observation APIs. |
| Server-2 notification registry | No enumeration | Registering a wildcard target changes registrations; receiving future deliveries does not reconstruct existing registrations, pending records or all prior mutations. |

Evidence for the boundary is the constructor-installed manager table `0x00AD1B80` (slots
0–9), queue table `0x00AD1AD0` (slots 0–24), and server-2 table `0x00AD1BF4` (slots 0–13).
The server's posted-count body locks its post region and calls only the embedded queue's
count (`0x0092D0FC`–`0x0092D10D`); its registry and directed deque are separate (§4.1).
The server constructs that queue directly (`0x0092EB70`–`0x0092EB73`): it is not evidence
of a queue-manager service or a manager registration for the embedded queue.

**Synchronization and lifetime.** GetQueue locks one lookup and AddRefs a found queue before
unlocking; a miss writes null and returns false (`0x0092DBC6`–`0x0092DBFD`). Release each
successful result through that returned queue interface after use. A held queue can outlive
its removal from the registry; the hold does not preserve its ID binding or freeze its contents.
Peeks likewise return counted messages. These are read-only with respect to the requested
container operation, not reference-neutral: AddRef/Release and message type/equality methods
can execute foreign code. There is no public cross-object snapshot lock, mutation sequence
number, or atomic count-plus-lookups operation. In particular, the unlocked manager count
does not make a series of locked lookups one consistent snapshot. The private layout in §4.1
does not authorize a live memory walk: it provides no public lock/lifetime protocol for one.

**Diagnostic scope.** A finite ID probe establishes only the answers to those lookups at
those call times. IDs 0–255, `0x64FA8434`, `0x652294BC` and `0xFFFFFFFF` are not an enumeration
contract or reserved queue-ID namespace. Equal numeric pointers on later samples do not prove
absence of intervening mutations or address reuse. A mismatch against an immutable baseline
must remain a mismatch; resynchronizing would hide it. A host may arrange a quiescent fixture
with known IDs for bounded conformance tests (§5.3), but neither that fixture nor an unchanged
monthly probe establishes whole-owner native verification. Full-state shadow needs a separately
established complete capture/replay facility; this answer does not supply one.

### 3.2 Optional service and existing-manager acquisition (GZ12)

The requested service/IID pair **is correct**: `0x64FA8434` / `0x652294BC`. The constructor
sets that service ID (`0x0092E720`), and manager QI accepts that IID, returns its primary
interface and AddRefs it (`0x0092D033`–`0x0092D04A`). However, the shipped application startup
does **not install this manager as a system service**. It registers a COM **class factory**
for class ID `0x452294DF` (`0x0092D02D`–`0x0092D032`, director registration
`0x004601F6`–`0x00460203`), which makes the class constructible, not an existing service.
Application AddSystemServices (`0x00450350`; [application §4.2](application.md#42-system-services-addsystemservices-0x00450350))
installs the generation-1 and generation-2 message servers, not this manager. There is no
guaranteed city-load or monthly phase at which an existing manager appears. The failed
lookup at load and all 72 monthly samples in `gzmsg-shadow8-r1-2032` is therefore consistent
with a genuinely absent optional service; it is not evidence for another manager ID or IID.

Read-only acquisition means querying the framework for that exact pair **if another owner
has already registered a manager**. GetSystemService searches the existing service registry
and, only on a hit, forwards the requested IID and output to that service's QI
(`0x0087A0C4`–`0x0087A0ED`). It does not instantiate classes, search inside message servers,
or wait for service creation. On a missing service it returns false and **leaves the output
unchanged** (`0x0087A0D5`–`0x0087A0EF`); initialize the output to null before probing and
use it only on success. A successful manager lookup gives the caller one owning reference;
balance it with manager-interface Release (slot 2, `0x00930577`, forwarding through
`0x0091DD89` to `0x0090D997`). Keep that hold across any queue lookups; acquiring the manager
does not hold all its queues or prevent registry mutations. Availability depends on actual
registration, not the service's running flag. A diagnostic host should finish its reads and
release its holds before framework teardown; a counted hold is not a guarantee that shutdown
leaves the service operational.

The factory `0x0092EA44` allocates a **new**, initially empty 0x38-byte manager and returns its
system-service face at object +4; it neither retrieves an existing instance nor registers one
(`0x0092EA44`–`0x0092EA64`, constructor `0x0092E714`). Substituting class ID `0x452294DF`
for the service ID, calling that factory, or inserting its result cannot repair an observation
of the original registry. The export's only direct manager-constructor caller is that factory,
and its factory pointer is registered by the director above; this is static provenance, not a
claim that optional plugins or arbitrary indirect clients can never instantiate/register it.
No unsupported EnumSystemServices callback, object-layout cast or fallback service is needed
for this acquisition contract. When no manager is registered, report **unavailable**, retain
the default-off diagnostic and its mismatch result, and give no queue-manager verification
credit. An absent service is not an acquired manager with an empty registry.

## 4. Message server 2 (`cGZMessageServer2`)

### 4.1 Layout and construction

Constructor `0x0092EB23`: interface vtable `0x00AD1BF4` at +0, system-service base at +4 (vtable
`0x00AD1BC0`, service id `0x04FA845B`, priority 5,000,000), a reference-count interface at +8
(vtable `0x00AD1BA8`). Then three critical sections: registry lock (+0x20), post-queue lock (+0x28),
directed-message lock (+0x30); an embedded `cGZMessageQueue2` (+0x38, the post queue); a 64-bit
"messages taken" counter (+0x58) = 0; the drain-all flag (+0x60) = **true**; registry depth (+0x64)
= 0; a pending-registration list (+0x68); the registry hash map type -> list of targets (+0x6C);
the directed-message countdown (+0x80) = 0; the directed-message deque (+0x84).

Interface `cIGZMessageServer2` (vtable `0x00AD1BF4`):

| Slot | Address | Behaviour (section) |
|---|---|---|
| 0 | `0x0092D091` | QueryInterface: iid `0x652294C7` returns this (AddRef'd), else system-service QI |
| 1 / 2 | `0x0093056F` / `0x00930577` | AddRef / Release of the system-service base |
| 3 | `0x0092EA65` | MessageSend(msg) (§4.3) |
| 4 | `0x0092D0BB` | MessagePost(msg, toFront) (§4.4) |
| 5 | `0x0092E795` | AddNotification(target, type) (§4.2) |
| 6 | `0x0092E560` | RemoveNotification(target, type) (§4.2) |
| 7 | `0x0092EA00` | Post directed message (msg, target) (§4.5) |
| 8 | `0x0092E642` | Cancel directed messages for target (§4.5) |
| 9 | `0x0092E040` | OnTick (§4.4); also reached from the service tick slot 10 via thunk `0x00473000` |
| 10 | `0x0092D0F4` | Posted-message count (§4.4) |
| 11 | `0x0092D11F` | Set drain-all flag (§4.4) |
| 12 | `0x0092D0B3` | Shared system-service reference count, unsigned dword at server +0xC (GZ1) |
| 13 | `0x0092D1FE` | CreateMessage (§4.6) |

**Counter forwarding (GZ1).** Slot 12 adds 8 to the server pointer and tail-calls that
interface's slot 5; `0x009D7E63` reads its +4, hence server +0xC. The export's name
`cRZString::ToChar` on that shared four-byte body is not evidence of string semantics.
The system-service constructor at server +4 constructs cRZUnknown at server +8 and initializes
this count to 0 (`0x0091DE0F`, `0x0090DA1E`). The service ID is instead at server +0x10;
priority fields are +0x14/+0x18, and the running byte is +0x1C.
Interface AddRef/Release (`0x0093056F`/`0x00930577`) first adjust +4 to the service base;
base wrappers (`0x009297A0`/`0x0091DD89`) adjust another +4 to the unknown interface.
Its AddRef increments the same dword (`0x00968B60`). The +8 vtable's AddRef thunk
`0x00473020` first adjusts -8 before using this chain. RemoveRef at +8-interface slot 4
(`0x0090D981`) only decrements a positive unsigned count and returns it, without destruction.
This is one shared count, not the posted-message count or 64-bit messages-taken counter.

### 4.2 Notification registry

A target registers for one message type; type **0 is the wildcard** (receives every sent message,
§4.3). Per type the targets are kept in registration order without duplicates.

- **AddNotification(target, type)** under the registry lock. If the registry depth is non-zero, the
  request is appended to the pending list as (add, target, type) and the call returns true. Otherwise
  depth becomes 1; the type's list is created if missing; if the target is already in it nothing
  changes, else it is appended. Then depth is decremented and the pending list is flushed (below).
  Always returns true.
- **RemoveNotification(target, type)**: same deferral rule (pending entry marked remove). Otherwise
  it removes the target from the type's list (if present); when the list becomes empty the list is
  freed and the type removed from the map. Flush, return true (also when nothing was registered).
- **Flush** (`0x0092DF35`): under the registry lock, guarded by a process-wide re-entry flag
  (byte `0x00B629F2`): if not already flushing and depth is 0, take pending entries front first and
  replay each through AddNotification / RemoveNotification, popping it, until the list is empty.
- **Apply pending** (`0x0092E8B5`), run at the start of every MessageSend: walks the pending list in
  order; an add appends the target if absent (creating the type's list); a remove deletes every
  occurrence of the target from the type's list and frees an emptied list; then clears the list.

**Re-entry and removal timing (GZ3, GZ6b).** Depth is a 32-bit dword, tested against zero,
incremented/decremented with dword instructions. Removal unlinks the registry node **before**
calling the target's Release, and frees the node afterwards. That Release runs with depth still
1 and the registry lock held (`0x0092E603`, erase `0x007E9330`, then depth decrement
`0x0092E627`). A final Release/destructor can therefore call back into AddNotification or
RemoveNotification and create pending work. Appending a registry entry also calls target
AddRef while depth is 1 (`0x0092E895`, helper `0x005447B0`). MessageSend calls the message's
GetType at `0x0092EA94` and snapshot target AddRef at `0x0092EAD7` while depth is positive.
The premise that no foreign code runs in these windows is false. The pending path is a live
interface contract, even without a measured in-game occurrence of a particular re-entrant
target. Ordinary DoMessage callbacks run after depth decrement/unlock; those calls alone do
not create pending work. Cross-thread callers serialize on the registry critical section.

Pending records carry a byte add/remove discriminator, a target interface pointer and a dword
type. Both add and remove keep a counted target reference in server 2. Construction uses two
temporary counted wrappers before copying the record into the list; temporary releases occur
before unlocking (`0x0092E57D–0x0092E5BF`, `0x0092E7B4–0x0092E7F6`). These are observable
foreign calls, not a single atomic reference transfer.

Flush replays the **still-linked** front record through virtual slot 5 or 6, then unlinks it and
releases its pending hold (`0x0092DF65–0x0092DF85`, `0x0092D9AF`, `0x009491FF`). Replay's
registry AddRef/Release occurs before the pending reference is dropped. Its process-wide byte
guard suppresses recursively entered flushes, including flushes on another server-2 instance;
it does not suppress registry changes themselves. The guard is cleared before the outer unlock.
Apply-pending instead walks records directly and clears the list after the walk, releasing all
pending holds then (`0x0092E9F6`, `0x0092DA64`). It runs **before** MessageSend increments depth;
there is no depth/flush-guard gate on that call (`0x0092EA87–0x0092EA8F`). Thus a nested send can
apply pending work before the enclosing send finishes. General re-entry that invalidates a live
native list iterator has no promised safe outcome; the controlled callbacks in §5.1 avoid that.

### 4.3 MessageSend (synchronous)

Under the registry lock: apply pending registrations, increment the depth, read the message type T,
copy the target list of T (in registration order) into a local array, then, when T is not 0, append
a copy of the wildcard (type 0) list. Decrement the depth and release the lock. Then call
`DoMessage(msg)` on each copied target in array order; return values are ignored. A target that
registers or unregisters during delivery does not change the current delivery. A target registered
for both T and 0 receives the message twice. Returns true.

**Snapshot holds (GZ6a).** Each copied target is AddRef'd while depth is positive and the
registry lock is held, including a separate hold for its wildcard occurrence. The vector first
appends a null reference, then assigns the target (`0x0092EAC2–0x0092EAD7`, assignment helper
`0x005447B0`). All snapshot holds survive **all** DoMessage calls. After the last callback,
the vector destructor releases its entries in forward order (`0x0092EB12–0x0092EB15`,
`0x006C6E90`). It does not release each target immediately after that target's delivery.
MessageSend itself borrows the message; it does not AddRef it.

### 4.4 MessagePost, the tick and the counters

- **MessagePost(msg, toFront)** under the post-queue lock: toFront false appends to the post queue,
  true prepends. Always returns true. Posting never delivers immediately.
- **Posted-message count**: the post queue's count under the post-queue lock.
- **Set drain-all flag(bool)**: stores the byte; constructor default true.
- **OnTick** (`0x0092E040`, argument unused): under the post-queue lock compute
  `limit = taken + queue count` (64-bit). Then repeat: under the lock take the front message; if none,
  stop; otherwise increment `taken`; outside the lock, MessageSend it and release it; then continue if
  the drain-all flag is set, else continue only while `taken < limit`. So with drain-all (the default)
  a tick empties the queue including messages posted during the tick; without it a tick delivers only
  the messages queued when it started. Finally, if the directed-message deque is not empty, run the
  directed delivery (§4.5). Returns true.

The service is ticked by the framework (Init `0x0092D05B` calls the framework's AddToTick, vtable
+0x24 = `0x00879BB9`; Shutdown `0x0092D076` calls +0x28, RemoveFromTick). Priority 5,000,000 puts it
ahead of the simulator (-2,000,000), see docs/specs/flammability.md §SimCycleComplete.

### 4.5 Directed messages

- **Post directed (msg, target)** (slot 7, argument order message then target): under the directed
  lock append an 8-byte (target, msg) counted pair. The temporary first AddRefs target, then message;
  deque copy construction repeats target then message AddRef; temporary destruction releases message
  then target, before unlock. Net hold: one of each (`0x0092EA00`, `0x0092D1C5`, `0x0092D293`,
  `0x004E4BB0`, `0x00473180`). Slot 7 is void, with no success result.
- **Cancel (target)** (slot 8, void): scan by raw interface-pointer identity under the directed
  lock. On a match, advance the scan iterator before erasing the old position, ignoring erase's
  returned iterator (`0x0092E685–0x0092E69D`). Consequently the intended remove-all operation has
  an iterator defect; see below. It does not change the directed countdown or cancel a record
  already copied out for delivery. Erasure releases references while the directed lock is held.
- **Delivery** (`0x0092DFA3`, end of OnTick): under the directed lock, if the countdown is <= 0 it is
  set to the deque size (nothing to do when 0). While the countdown is > 0: decrement it; if the deque
  is not empty, pop the front entry, release the lock, call `target->DoMessage(msg)`, retake the lock.
  Finally decrement the countdown once more and release the lock. Entries added during delivery wait
  for the next tick **when the original entries consume the remaining quota**; cancellation can
  make room for a newly posted entry in the same pass. A nested delivery shares the countdown.

**Delivery holds (GZ6c).** There is one initially null local pair for the **entire** delivery pass,
not one temporary per callback. Before each pop, assign front target into local target, then front
message into local message (`0x0092DFF8`, helper `0x004731D0`). For each changed nonnull pointer,
assignment AddRefs the new object, stores it, then Releases the previous local object; equal
pointers cause no reference calls. Pop destroys the deque pair, releasing message then target
under the lock (`0x004732E0`, boundary path `0x00473290`). Both local references survive DoMessage.
After its callback the local pair persists until assignment of the next front or final cleanup.
Final cleanup is **after unlocking**, message Release then target Release (`0x0092E026–0x0092E036`,
`0x00473180`). Thus the previous target may be released during the next iteration before the
previous message; simply releasing message then target immediately after every callback differs.

The countdown is **signed 32-bit**, using signed positive tests (`0x0092DFC5`, `0x0092E024`),
not an unsigned size. Normal nonempty completion leaves -1; an empty direct invocation also
decrements 0 to -1. Nested completion may leave smaller negative values. No floating arithmetic
is involved. Callback bools are ignored; directed delivery does not consult the registry or wildcard.

**Cancellation iterator defect.** Erase computes the index and compares it with floor(size/2).
In the first half it shifts the prefix and pops front; otherwise it shifts the suffix left and
pops back (`0x0092E17F–0x0092E1FA`). The already-advanced iterator remains usable on the prefix
path, but on the suffix path it can skip the element shifted into the erased position. Example
within a single block: targets [B,C,A,A], cancel A: index 2 takes the suffix path, leaves [B,C,A],
and the scan equals the new end, so the final A survives. Cancelling the last entry instead can
leave the scan beyond the new end; the native equality-only end test provides no safe bounded
result. Do not invent a deterministic delivery/crash for that invalid-iterator case. The defect
also exists in server 1 (`0x0092FFAA`, `0x0092F023`, erase `0x0092FD24`). Server-2 suffix shifts
assign pairs in forward order, target then message (`0x0092DD3F`, `0x0092DB38`, `0x0092D62A`,
`0x004731D0`), so erasure can call AddRef/Release on surviving entries as well as the removed one.

**Native uses (GZ4).** `cSC4WinIntroVideoScreen::DoUserStop` obtains server 1 through RZ::MS
and posts a stack record of type `0xEAAD620A` to its own unadjusted target pointer
(`0x0079D423–0x0079D452`). GZPaint does likewise for `0xEAAC1A96` and `0xCAAC6A52`
(`0x0079D55E–0x0079D581`, `0x0079D5B7–0x0079D5DA`). This is concrete shipped UI use of
slot 7, with FIFO deferred delivery observable by that screen.
Server-2 call site `0x007724A3` acquires the exact service/IID `0x04FA845B`/`0x652294C7`,
posts type `0x0A344006` to its supplied target, then releases the message (`0x00772440`).
The exported unnamed body `0x009851F9` similarly obtains RZ::MS2 and posts type `0x6AE76406`
to its +0x24 target when nonnull (`0x00985289`), otherwise uses ordinary MessagePost.
No class name is inferred for those two unnamed bodies. The acquisition-seeded native caller
search proves positive uses, not an exhaustive list of all indirect calls; no concrete slot-8
game caller was established in this bounded trace. This does not weaken the native cancellation
body's ordering/lifetime contract or imply that it is unused.

### 4.6 CreateMessage (slot 13, `0x0092D1FE`)

`CreateMessage(clsid, iid, out)`: only class id `0x46DF45E7` is known; it allocates a 0x2C-byte
`cRZMessage2Standard`, asks it for `iid` into `out` and returns that answer (the temporary reference
is released, so on success the caller holds the only reference). Any other class id returns false
and leaves `out` untouched.

### 4.7 Destruction

Destructor `0x0092E42D` (deleting wrapper `0x0092E779`): under the registry lock sets depth 0 and
frees every type list with its references, then the pending list, deque, queue and locks.

## 4A. Message server 1 (`cGZMessageServer`, 5,796 bytes)

The first-generation server has the same structure and algorithms as §4 with these exact
differences. Constructor `0x0093035D`: system-service base at +0 (service id `0x64063387`, priority
5,000,000, vtables `0x00AD1D20` / `0x00AD1D08`), interface `cIGZMessageServer` at **+0x1C** (vtable
`0x00AD1CD8`, iid `0xC1085722`, QI `0x0092ED0D`); locks +0x20 / +0x28 / +0x30, an embedded
first-generation queue `cGZMessageQueue` at +0x38, counter +0x58, registry depth **+0x60**, drain-all
flag **+0x64** (default true), pending list +0x68, registry map +0x6C, directed countdown +0x80,
directed deque +0x84. Interface slots (addresses take the +0x1C interface pointer):

| Slot | Address | Behaviour |
|---|---|---|
| 3 | `0x009302AC` | MessageSend(msg) |
| 4 | `0x0092ED37` | MessagePost(msg, toFront), as §4.4 |
| 5 / 6 | `0x00930030` / `0x0092FC2E` | AddNotification / RemoveNotification(target, type), as §4.2 (flush `0x0092F88A`, apply pending `0x00930121`) |
| 7 / 8 | `0x00930273` / `0x0092FF6F` | Post directed (msg, target) / cancel for target, as §4.5 (delivery `0x0092F90C`) |
| 9 | `0x00472FC0` -> `0x0092F9AC` | OnTick, as §4.4 |
| 10 | `0x0092ED70` | Posted-message count |
| 11 | `0x0092ED9B` | Set drain-all flag |

Differences that matter:
- **Pending ownership differs.** A pending add copies raw (flag,target,type) then AddRefs target
  once (`0x00930051–0x00930071`); a pending remove copies the raw pointer without AddRef
  (`0x0092FC4B–0x0092FC62`). Flush uses a separate global byte guard `0x00B629F3`. Replaying an
  add calls AddNotification then Releases its pending-add hold before popping the record;
  replaying a remove has no pending hold to release (`0x0092F8C2–0x0092F8ED`). Direct apply at
  MessageSend start appends the raw target for an absent registration, transferring the pending-add
  hold; it does not create server 2's counted wrapper. A pending add already present in the list
  is skipped, and bulk clearing does not release those raw records, so this path leaks that extra
  hold (`0x00930121`, add/duplicate branches through `0x009301EB`, clear `0x00930269`).
  Normal removal unlinks matching raw entries, then Releases the target at depth 1, before
  decrementing depth and flushing (`0x0092FCAD–0x0092FCE0`). Re-entrant callbacks must preserve
  the lifetime of raw pending-removal targets themselves.
- **Directed entries have no reference holds (GZ5).** Each entry is exactly 20 bytes: raw target
  interface pointer followed by a copied 16-byte message record. Append constructs and copies
  those values without target AddRef (`0x00930273`, `0x009DB1A1`, `0x0092EE7C`, `0x0097BF85`).
  Delivery copies all five dwords to its local record (`0x00473110`), removes the deque front
  without releasing anything (`0x00473240`), then calls the raw target with a pointer to the
  local message (`0x0092F97E–0x0092F987`). Unlike server 2, neither cancellation nor delivery
  owns a target/message reference; the caller must keep the target and pointed-to payloads alive.
- **Messages are values.** A first-generation message is a 16-byte record whose first dword is the
  type; the queue stores copies, OnTick takes the front record into a zeroed local copy and sends a
  pointer to it, and nothing is reference counted for the message.
- **Send has no wildcard and ignores type 0.** MessageSend reads the type from the record; type 0
  delivers to nobody; otherwise only the targets registered for exactly that type receive it, in
  registration order. There is no second pass over type 0 listeners.
- **Fixed snapshot size.** The target snapshot is a stack array of 0x608 bytes (386 slots, one used
  as the null terminator), filled without a bound check: the original supports at most 385 targets
  per type; behaviour beyond that is undefined (stack overflow), so an implementation may treat 385
  as the limit. Delivery stops at the first null slot; each target is AddRef'd in the snapshot and
  released right after its `DoMessage` call (+0x0C).

## 5. Worked example

Targets A (type 5) and W (type 0) registered, drain-all true. Post m1 (type 5), then m2 (type 7,
toFront true). Queue = [m2, m1], count 2. Tick: limit = 0 + 2. Take m2: send -> W. Take m1: send ->
A then W. With drain-all false and W posting m3 while handling m2, m3 would wait: `taken` reaches 2
= limit after m1. With drain-all true m3 is delivered in the same tick.

### 5.1 Independent contract vectors (GZ1–GZ7)

These are instruction-derived expectations for an implementer's reference-logging targets/messages;
they are not results from the partial core or an in-game run. Use separate live external holds
unless a vector explicitly exercises final Release. A callback's target pointer is the exact
registered/posted interface pointer; DoMessage is thiscall at vtable +0x0C, AddRef/Release at
+0x04/+0x08. Server 1 methods take its +0x1C interface pointer, server 2 methods its +0 pointer.

| Vector | Setup / action | Required observation |
|---|---|---|
| GZ-V01 shared count | Construct server 2, AddRef through message interface, then AddRef through +8 interface; read slot 12 after each. RemoveRef through +8 slot 4. | Counts 0,1,2,1; no dependence on queue size or service ID. RemoveRef at 0 remains 0 without deleting. The count is an unsigned dword; increment wraps modulo 2^32. |
| GZ-V02 wait units/results | Wait on a valid nonsignalled signal with 999, 1,000, 1,000,000 and 0xFFFFFFFF. | OS timeout arguments 0,1,1000,0xFFFFFFFF milliseconds respectively. Timeout maps to dword 2; forced invalid handle maps WAIT_FAILED to 0. No implicit explicit-reset call. |
| GZ-V03 lost pulse | Queue get fails empty; producer appends before consumer enters Wait; no eligible waiter at the pulse. | No persistent wake token. Consumer's one-second wait can time out, then its get obtains the already queued message. A failed Wait is also followed by another get; an invalid handle plus an empty queue can spin indefinitely. |
| GZ-V04 asymmetric equality | Queue distinct q1,q2,q3. Instrument q1.equals(a)=false, q2.equals(a)=true, q3.equals(a)=true; make a.equals(any)=false. Run each equality method on a fresh queue. | Slot 6 removes q2; slot 8 removes q3; slot 10 removes q2 and q3, returns 2. Slots 17/24 yield q2 with one caller hold; 17 retains it in queue, 24 removes the queue hold. The receiver a is never queried. A return dword 0x100 has false low byte. |
| GZ-V05 counted snapshot | Register A,B for T and A for wildcard 0. Send T; A's first DoMessage removes B's T registration. | Calls A,B,A, despite unregistering B. B remains alive through its callback. All three retained snapshot holds persist until all deliveries complete, then Release in A,B,A order. This differs from both borrowed targets and per-callback cleanup. |
| GZ-V06 registry destructor re-entry | A has one remaining registry hold for T. Remove A; its final Release makes one AddNotification(B,T) call. B's AddRef has no re-entry. | A's node is already unlinked but depth is 1 during Release. B is queued as pending, not inserted immediately. After outer depth drops, flush adds B before dropping the pending hold. At return B has one registry hold and pending is empty. |
| GZ-V07 GetType re-entry | Server 2 has A for T. Message.GetType makes one AddNotification(B,T), then returns T. | GetType sees positive depth, so B enters pending. Current snapshot is A only. Next Send first applies pending, and its snapshot is A,B; pending's reference is released after its registry reference is acquired. Keep this callback one-shot. |
| GZ-V08 generation-1 value/raw lifetime | Post a directed 16-byte record to live target A on server 1; alter every original record dword afterwards, then deliver. | A receives the original four dwords through a local record pointer. There is no target AddRef/Release at append, copy-out, pop, callback completion or cancellation; no message reference calls at all. Keep A and any pointers encoded in its record alive externally. |
| GZ-V09 generation-2 delivery holds | Begin delivery with distinct pairs (A,m1),(B,m2), no re-entry. Log only the delivery interval. | First acquire local A,m1, then pop releases queue m1,A, then DoMessage(A,m1). Next local assignment AddRefs B and Releases A, then AddRefs m2 and Releases m1; pop releases queue m2,B, then DoMessage(B,m2). Final unlock precedes local Release(m2), Release(B). |
| GZ-V10 cancellation prefix | Either generation has targets [A,A,B,C,D] in one deque block; cancel A, then deliver. | Both front A entries are removed by prefix erasures; remaining delivery order is B,C,D. Server 2 drops removed holds under the directed lock; server 1 makes no reference calls. |
| GZ-V11 cancellation suffix defect | Either generation has targets [B,C,A,A] in one block; cancel A, then deliver. | Erasing index 2 uses suffix shift; the pre-advanced scan equals the new end. Queue remains [B,C,A] and the last A is delivered. Server 2 has net one remaining A hold. This is a defined concrete witness against blanket remove-all. |
| GZ-V12 quota after cancel/post | Start [A,B,C,D,E]. During A's callback cancel B (now in prefix of four), then append F. No nested tick. | Original quota is 5. Delivery is A,C,D,E,F in this pass. Cancellation does not reduce countdown, so F can run during the current tick. |
| GZ-V13 nested quota | Start [A,B,C]. A's callback enters OnTick once without posting or cancelling. | Outer pass decrements 3 to 2 before A. Nested pass consumes B,C using remaining 2, exits at -1; outer exits at -2. Each invocation has its own local pair, sharing only the countdown. |
| GZ-V14 raw pending add transfer | Server 1: during A's one-shot AddRef callback in AddNotification(A,U), call AddNotification(B,T) twice, with T distinct from U. In case 1 return and allow normal flush; in case 2 perform a nested Send of a value message T before returning. Keep A/B externally alive; B's reference callbacks do not re-enter. | Case 1 leaves one registry hold on B and releases both pending-add holds. Case 2 directly applies pending before incrementing send depth: the first pending hold becomes the registry hold, the duplicate add is skipped and its extra hold is leaked on raw-record bulk clear. Snapshot holds are temporary. Pending removals have no protective reference. |

For invalid-iterator cancellation (for example, matching the last/sole element), native scanning
can pass the new end. There is deliberately no expected crash, return time or subsequent delivery
vector: native instructions establish the unsafe traversal, not a deterministic result after it.
For ordinary posting and failed allocation there is likewise no invented recoverable failure:
the examined message/record paths do not return a failure bool or establish exception recovery.

### 5.2 Missing-contract method/body inventory and scope

This inventory identifies the bodies needed to complete the seven **specification** gaps; it
does not claim any implementation method is missing or present. Names attached to folded/shared
bodies in the export can be misleading; entry addresses and call adjustments control this spec.

| Gap | Entry methods / required helper bodies | New contract |
|---|---|---|
| GZ1 | server2 slot 12 `0x0092D0B3`; +8 getter `0x009D7E63`, RemoveRef `0x0090D981`; service ctor `0x0091DE0F`, unknown ctor `0x0090DA1E`; `0x0093056F/77`, `0x009297A0`, `0x0091DD89`, `0x00968B60` | Shared unsigned dword at +0xC; complete +4,+4 and -8 forwarding (§4.1). |
| GZ2 | queue blocking gets `0x0092CFC1/0x0092CFF7`, ctor/dtor `0x0092DD7E/0x0092DDB0`; foreign signal ctor `0x00932E65`, Signal/Pulse/Wait `0x00932DA7/0x00932D95/0x00932DE0`, valid/reset `0x00932E3A/0x00932DCE`, cleanup `0x00932EC8/0x00932EB4` | Microsecond unsigned conversion, pulsed queue notification, status mapping and handle lifetime (§2). Native PE table `0x00AD1F48` and Win32 import pointers corroborate the virtual dispatch. |
| GZ3/GZ6b | server2 add/remove `0x0092E795/0x0092E560`, flush/apply `0x0092DF35/0x0092E8B5`; unlink/release `0x007E9330`, pending pop/erase/clear `0x0092DC24/0x0092D9AF/0x0092DA64`, pending target cleanup `0x009491FF`; server1 counterparts `0x00930030/0x0092FC2E/0x0092F88A/0x00930121`, raw clear `0x009A841F` | Callback windows, pending vs immediate order, per-generation ownership and duplicate-add leak (§4.2/§4A). |
| GZ6a | send `0x0092EA65`, vector append `0x00442E40`, assignment `0x005447B0`, vector destructor `0x006C6E90` | Retained per-occurrence targets; all callbacks before forward cleanup (§4.3). |
| GZ4/GZ5/GZ6c | server2 post/cancel/deliver `0x0092EA00/0x0092E642/0x0092DFA3`, pair ctor/copy `0x0092D1C5/0x0092D293`, assignment/destructor `0x004731D0/0x00473180`, pop `0x004732E0/0x00473290`, erase `0x0092E162`; server1 `0x00930273/0x0092FF6F/0x0092F90C`, raw ctor/copy `0x009DB1A1/0x0092EE7C/0x0097BF85/0x00473110`, raw pop/erase `0x00473240/0x0092FD24` | Pair vs raw 20-byte record; persistent delivery local, quota/cancel defects and positive native callers (§4.5/§4A). |
| GZ7 | equality methods `0x0092D66F/0x0092D6CC/0x0092D72A/0x0092D45B/0x0092D940` | Queued receiver, argument direction and low-byte truth (§2). |

Only the foreign ThreadSignal and SystemService/unknown forwarding needed by these questions
are added here. No File/INI/Directory/CmdLine/SysServPtr owner completion is claimed; the native
acquisition helpers are caller provenance only. No aggregate GZ/RZ owner or byte coverage increase
follows from this contract pass. GZ1–GZ7 are resolved in their named sections and archived.

### 5.3 Observation and acquisition vectors (GZ11/GZ12)

These are independent, instruction-derived expectations, not new game measurements. Registry
fixtures are host-controlled and quiescent; do not create or insert native services in a live
city to run them. Keep explicit external holds on fixture queues and messages. The negative
observability cases state what a diagnostic may conclude, not a new native snapshot API.

| Vector | Setup / action | Required observation |
|---|---|---|
| GZ-V15 optional service absent | Framework with no service `0x64FA8434`; initialize output to null, then GetSystemService with IID `0x652294BC`. Repeat separately with a nonnull sentinel output. | False in both calls. Output remains null/sentinel respectively; no factory invocation, manager or queue creation. Diagnostic records unavailable, not exact-empty. |
| GZ-V16 existing service | Host registers a fixture manager under its own service ID, then looks up `0x64FA8434` / `0x652294BC`. | True, primary manager interface returned, one AddRef for the caller. Balance with slot-2 Release. Looking up class ID `0x452294DF` as a service ID fails when no service has that ID. |
| GZ-V17 ID cardinality | Empty manager; AddQueue(7,Q), AddQueue(8,Q), AddQueue(7,R), RemoveQueue(7), RemoveAllQueues; read slot 9 after each action without concurrent writers. | Counts 0 initially, then 1,2,2,1,0. Duplicate ID fails; aliasing Q under two IDs counts twice. Count reads acquire no queue references and make no manager lock call. |
| GZ-V18 hidden key | Register Q only at `0x00010000`. Probe 0–255, `0x64FA8434`, `0x652294BC`, `0xFFFFFFFF`; separately read manager count. | Every probe misses and writes null; count is 1. The finite probe is not an empty-registry observation. Slot 9 supplies no missing key. |
| GZ-V19 held lookup survives removal | Register Q at 7; GetQueue(7,out); remove 7 while caller retains out; GetQueue(7,other). | First lookup succeeds and AddRefs Q; second misses and writes null. The first returned queue remains alive through the caller hold, but ID 7 is absent and count is 0. Release the held result after use. |
| GZ-V20 equal bounded samples | Between two quiescent sample times, replace hidden ID `0x10000` with `0x10001`, keeping all probed IDs unchanged and the same total count. | Bounded membership/identity answers and count match, although the registry changed. No complete-state equality or absence-of-mutation conclusion is allowed. |
| GZ-V21 ordinary count boundary | Fresh server 2: add a notification; post one directed pair; then post one ordinary message. Observe slot 10 after each action, without Send or Tick. | Counts 0 initially, then 0,0,1. Neither a notification nor a directed pair appears in ordinary posted count. Nothing enumerates the registered target or directed pair. |
| GZ-V22 endpoints are not contents | Distinct queues contain [A,B,C] and [A,D,C], with four distinct messages of the same type. Read queue count and front/back peeks only. | Both counts are 3 and both endpoint identities/types match. These observations cannot distinguish the middle message. Peeks AddRef their returned endpoints; release those results. Do not drain/refill either queue to claim a read-only snapshot. |

GZ-V15/V16 follow framework lookup `0x0087A0AC` and manager QI `0x0092D033`.
GZ-V17–V20 follow manager slots 3–9, including the unlocked count getter, and
GZ-V21/V22 follow server count `0x0092D0F4` and the queue interfaces in §2. They do not
resolve the separate cleanup, forwarding or complete-body-inventory questions GZ8–GZ10.

## 6. Open questions

GZ1–GZ7 and GZ11/GZ12 are answered; GZ8–GZ10 remain open. GZ11's answer is an explicit
native observation limit and GZ12's is optional-service absence, not new shadow verification.
Native indirect-caller enumeration and invalid-iterator
behaviour are bounded as stated in §4.5; they are not claims of exhaustive runtime coverage.

See docs/specs/questions/framework-gz-services.md.
