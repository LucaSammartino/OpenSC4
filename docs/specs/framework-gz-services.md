# Framework services (GZ): message server 2, message queues

Behaviour spec for the GZ framework's second-generation messaging service: the message server
(`cGZMessageServer2`), its FIFO message queue (`cGZMessageQueue2`) and the named-queue registry
(`cGZMessageQueueManager2`). Written from the exe (SimCity 4 Deluxe 1.1.641) by lane re-gzservices4,
2026-10-21. Evidence: Ghidra export instructions (`tools/re_query.py asm`) and the vtables read from
`bin/SimCity4.exe`; the REA session was unavailable this round (no binary open), so no REA pseudocode
was used. Addresses are VAs in that exe.

Outline: §1 owners and identity. §2 message queue. §3 queue manager. §4 message server 2
(§4.1 layout, §4.2 notification registry, §4.3 send, §4.4 post and tick, §4.5 directed (general)
messages, §4.6 message creation, §4.7 service lifecycle). §5 worked example. §6 open questions.

## 1. Owners and identity

| Owner class | Bytes (coverage owners) | Functions | Identity |
|---|---|---|---|
| `cGZMessageServer2` | 2,780 | 38 | service id `0x04FA845B`, interface `cIGZMessageServer2` iid `0x652294C7`, tick priority 5,000,000 |
| `cGZMessageQueue2` | 1,941 | 30 | queue object, 0x1C bytes, vtable `0x00AD1AD0`, 25 slots |
| `cGZMessageQueueManager2` | 951 | 17 | service id `0x64FA8434`, iid `0x652294BC`, tick priority 5,000,000 |

Owner bytes are from `tools/coverage.py --owners` (subsystem "framework services (GZ)") at main
`ab23a7fa`. The messages themselves are `cRZMessage2Standard` (RZ side, not in this spec); a message
exposes, in its own vtable, `GetType` at +0x10 and an equality test against another message at
+0x18. A message target exposes `DoMessage(message)` at +0x0C.

Reference counting: every queue slot, registry entry and pending entry holds one counted reference
(AddRef on store, Release on removal). Every "out" message pointer handed back to a caller is
AddRef'd for that caller.

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

The signal is set only on the empty to non-empty transition; it is never explicitly reset by the
queue (the signal object's own semantics decide whether a wait consumes it).

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

Erasing releases the map's reference; a queue nobody else holds is destroyed then.

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
| 12 | `0x0092D0B3` | Reference count of the +8 interface (returns its stored count) |
| 13 | `0x0092D1FE` | CreateMessage (§4.6) |

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

### 4.3 MessageSend (synchronous)

Under the registry lock: apply pending registrations, increment the depth, read the message type T,
copy the target list of T (in registration order) into a local array, then, when T is not 0, append
a copy of the wildcard (type 0) list. Decrement the depth and release the lock. Then call
`DoMessage(msg)` on each copied target in array order; return values are ignored. A target that
registers or unregisters during delivery does not change the current delivery. A target registered
for both T and 0 receives the message twice. Returns true.

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
  lock append (target, msg) (both referenced) to the deque.
- **Cancel (target)** (slot 8): under the directed lock erase every deque entry whose target is the
  argument.
- **Delivery** (`0x0092DFA3`, end of OnTick): under the directed lock, if the countdown is <= 0 it is
  set to the deque size (nothing to do when 0). While the countdown is > 0: decrement it; if the deque
  is not empty, pop the front entry, release the lock, call `target->DoMessage(msg)`, retake the lock.
  Finally decrement the countdown once more and release the lock. Entries added during delivery wait
  for the next tick; a nested delivery shares the countdown.

### 4.6 CreateMessage (slot 13, `0x0092D1FE`)

`CreateMessage(clsid, iid, out)`: only class id `0x46DF45E7` is known; it allocates a 0x2C-byte
`cRZMessage2Standard`, asks it for `iid` into `out` and returns that answer (the temporary reference
is released, so on success the caller holds the only reference). Any other class id returns false
and leaves `out` untouched.

### 4.7 Destruction

Destructor `0x0092E42D` (deleting wrapper `0x0092E779`): under the registry lock sets depth 0 and
frees every type list with its references, then the pending list, deque, queue and locks.

## 5. Worked example

Targets A (type 5) and W (type 0) registered, drain-all true. Post m1 (type 5), then m2 (type 7,
toFront true). Queue = [m2, m1], count 2. Tick: limit = 0 + 2. Take m2: send -> W. Take m1: send ->
A then W. With drain-all false and W posting m3 while handling m2, m3 would wait: `taken` reaches 2
= limit after m1. With drain-all true m3 is delivered in the same tick.

## 6. Open questions

See docs/specs/questions/framework-gz-services.md.
