# Animation tick manager

Status: first dedicated owner publication, Windows GOG 1.1.641.0. **Specified** means the stated normal-path
rule follows from instructions; **partial** identifies a bounded rule with an outstanding dependency;
**unknown** supplies no compatibility rule. These labels do not imply native execution. All vectors in §8
are proposed, independently derived examples, never executed against the original. Implementers use this
document, not the original or analysis artifacts.

## 0. Evidence and outline

Sections: §1 measured inventory; §2 interfaces and return channels; §3 construction, references and service
lifetime; §4 registration and mutation; §5 tick order and messages; §6 time and pause controls; §7 consumer
reconciliation; §8 distinguishing vectors; §9 limits and questions.

Primary evidence is the matching export's function ranges and byte-bearing instructions, queried with
`re_query.py func`, `asm --bytes`, `callees`, `xrefs` and `vtable`. Local matching decompilation was navigation
support only. Literal data below was independently read from the original PE sections. The executable is
7,524,352 bytes, version **1.1.641.0**, SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`;
`export-summary.txt` has that exact hash, program `SimCity 4.exe`, image base `0x00400000`.
This identifies the export; it does not establish a live REA target. Original round 1 stopped at `1172c1cb`
with `target_unavailable`, "No app is open", and zero published behavior credit. This explicitly authorized
round 2/3 used export/instruction evidence without repeating that failure. Live pseudocode availability is
still unknown. No execution, game requests or implementation tests occurred.

## 1. Measured attributed-body inventory

Body bytes are sums of exported function-body ranges, not address spans, estimates, executed bytes or a
claim that every instruction has a complete public contract. The bounded owner region `0x00447E30` through
factory `0x00448FF0`, plus folded setter `0x004FAD10`, contains **44 bodies / 4,286 bytes**. This includes the
shared AddRef body and owner-specific container helpers. Constructor ranges are discontiguous; its 434
bytes exclude the gap. Every body is listed separately, including adjustment thunks and the catch fragment.

| Entry | Bytes | Attribution / contract | Status |
|---|---:|---|---|
| `00447E30` | 8 | shared AddRef, §3 | specified |
| `00447E40` | 27 | Release, §3 | specified |
| `00447E60` | 125 | QueryInterface, §2 | specified |
| `00447EE0` | 76 | PauseAnimationSystem, §6 | specified |
| `00447F30` | 14 | IsAnimationSystemPaused, §6 | specified |
| `00447F40` | 38 | SetMinAndMaxTickTimeDeltas, §6 | specified |
| `00447F70` | 32 | StepAnimationSystem, §6 | specified |
| `00447F90` | 56 | EnableAutoStep, §6 | specified |
| `00447FD0` | 50 | SetNewPriority, §4 | specified |
| `00448010` | 153 | Init; external framework ownership not closed, §3 | partial |
| `004480B0` | 122 | pointer-node erase (export name says insert), §4 | specified |
| `00448130` | 63 | pointer lookup by identity, §4 | specified |
| `00448170` | 178 | FindTickableObject, §4 | specified |
| `00448230` | 22 | advance tick iterator before callback, §5 | specified |
| `00448250` | 234 | grow/rehash registration table, §4 | specified |
| `00448340` | 152 | nondeleting destruction, §3 | specified |
| `004483E0` | 6 | class ID result `A9C73857` | specified |
| `004483F0` | 8 | QI adjustment from +4 | specified |
| `00448400` | 8 | AddRef adjustment from +4 | specified |
| `00448410` | 8 | Release adjustment from +4 | specified |
| `00448420` | 8 | QI adjustment from +8 | specified |
| `00448430` | 8 | AddRef adjustment from +8 | specified |
| `00448440` | 8 | Release adjustment from +8 | specified |
| `00448450` | 8 | QI adjustment from +12 | specified |
| `00448460` | 8 | AddRef adjustment from +12 | specified |
| `00448470` | 8 | Release adjustment from +12 | specified |
| `00448480` | 8 | QI adjustment from +16 | specified |
| `00448490` | 8 | AddRef adjustment from +16 | specified |
| `004484A0` | 8 | Release adjustment from +16 | specified |
| `004484B0` | 8 | deleting-destructor adjustment from +16 | specified |
| `004484C0` | 8 | Init adjustment from +12 | specified |
| `004484D0` | 379 | Shutdown normal path; callback teardown hazards, §3 | partial |
| `00448650` | 220 | serialization write, auxiliary slot 3, §2 | partial |
| `00448730` | 140 | deferred FIFO drain, §4 | specified |
| `004487C0` | 8 | Shutdown adjustment from +12 | specified |
| `004487D0` | 30 | deleting destructor, §3 | specified |
| `004487F0` | 654 | tick normal path, §§5–6 | specified |
| `00448A7E` | 38 | exception cleanup fragment; propagation dependency, §9 | partial |
| `00448AB0` | 187 | serialization read, auxiliary slot 4, §2 | partial |
| `00448B70` | 255 | AddTickableObject, §4 | specified |
| `00448C70` | 433 | RemoveTickableObject, §4 | specified |
| `00448E30` | 434 | constructor, §3 | specified |
| `00448FF0` | 27 | factory, §3 | specified |
| `004FAD10` | 13 | shared step-delta setter, §6 | specified |

Two further shared bodies are reached by this owner's interfaces: timestamp getter `0040D390` (4 bytes)
and message receiver `006B8AA0` (14 bytes). Including those gives **46 distinct bodies / 4,304 bytes**.
They are specified below, counted once, and not claimed as new exclusive owner code. Generic timer,
message, allocator, framework and STL dependencies are outside this denominator. No coverage table is changed.

## 2. Identity, signatures and output contracts

**Specified.** Class ID `0xA9C73857` is both the literal class-ID getter (`004483E0`) and the built-in class
table entry. Construction supplies service ID `0x86AD10EC` and order `0xFFE17B80` to the system-service base
(`00448E3F–00448E5C`, `0091DE0F`). The manager interface IID is `0x86AD10ED` (`00447E64`).

Primary vtable `0x00A86760` uses the object base. Calls are x86 thiscall with ECX as this, callee-popped
arguments. A boolean is guaranteed only in AL; callers must not read its upper EAX bytes as a normalized
boolean. No primary method returns an x87 value. `u32` describes observed unsigned comparison/storage;
the original source-language declaration is not asserted where it cannot be distinguished.

| Slot | Method and inputs | Pop bytes | Defined result | Body |
|---:|---|---:|---|---|
| 0 | QueryInterface(u32 iid, pointer-to-pointer out) | 8 | AL boolean | `00447E60` |
| 1 | AddRef() | 0 | EAX u32 new count | `00447E30` |
| 2 | Release() | 0 | EAX remaining count or zero on destruction | `00447E40` |
| 3 | Init() | 0 | AL true on normal return | `00448010` |
| 4 | Shutdown() | 0 | AL true on normal return | `004484D0` |
| 5 | AddTickableObject(messageTarget2*, u32 priority) | 8 | AL true on normal return | `00448B70` |
| 6 | RemoveTickableObject(messageTarget2*, u32 priorityOrAll) | 8 | AL true, including absent target | `00448C70` |
| 7 | FindTickableObject(messageTarget2*, u32 priorityOrAll) | 8 | AL membership boolean; no output reference | `00448170` |
| 8 | SetNewPriority(messageTarget2*, u32 priority) | 8 | no defined semantic result; final Release leaves EAX residue | `00447FD0` |
| 9 | DoAnimationTick() | 0 | EAX u32, §5 | `004487F0` |
| 10 | GetTickTimestamp() (descriptive name) | 0 | EAX stored u32 at +0x58 | `0040D390` |
| 11 | SetMinAndMaxTickTimeDeltas(u32 min, u32 max) | 8 | no defined result | `00447F40` |
| 12 | PauseAnimationSystem(bool pause) | 4 | no defined result | `00447EE0` |
| 13 | IsAnimationSystemPaused() | 0 | EAX normalized 0/1 | `00447F30` |
| 14 | EnableAutoStep(bool enable, u32 intervalUs) | 8 | no defined result | `00447F90` |
| 15 | SetStepTickTimeDelta(u32 deltaUs) | 4 | no defined result | `004FAD10` |
| 16 | StepAnimationSystem() | 0 | no defined result | `00447F70` |

Void-facing declarations for the resultless methods are an interoperability choice: instructions establish
the stack cleanup and lack of a consistent success result, not the original C++ spelling. Do not reinterpret
residual EAX/AL as bool. Bool inputs read the low byte of their four-byte argument.

QI succeeds for IID 1 and `86AD10ED`, writing the base pointer; for `452294AA`, writing base+4; and for
`E4FDA3D4`, writing base+8. Each success calls the primary AddRef once. All other IDs delegate to the
service base at +12 (`00447EA8–00447EAF`). That base accepts `287FB697`, returns base+12 and calls its
AddRef thunk (`0091DD67–0091DD7E`). Other IDs reach `009457C6`, which returns false without clearing
the output. Thus an unsupported IID leaves the caller's output sentinel unchanged. There is no output-null
guard. Apparent null-this branches in the adjustment cases still dereference this; they are not safe APIs.

The +4 interface is a message target with vtable `00A86750`: adjusted QI/AddRef/Release followed by
shared `006B8AA0`. That receiver accepts one message pointer (pop 4), reads its type through message slot 4,
ignores it and returns AL true. **It does not call DoAnimationTick.** The +8 interface, vtable `00A86738`,
has adjusted QI/AddRef/Release, write(stream*) `00448650`, read(stream*) `00448AB0`, and class-ID getter
`004483E0`. Read/write each pop 4 and return integer 0/1 in EAX. Read rejects a 16-bit version other than 1;
write emits version 1; both end by negating the stream error boolean (`00448718–00448728`,
`00448B55–00448B64`). **Partial:** full record order, stream semantics and loaded-reference ownership
are not established; TK2 forbids inventing a persistence format from these observations.

The +12 service vtable is `00A86704`; its QI/AddRef/Release and Init/Shutdown adjust to the base object.
Its service ID is stored at object+0x18, order copies at +0x1C/+0x20 and running byte at +0x24. Shared
slots read/write these fields (`009CEDE1`, `00894F6D`, `0099BE66`, `00932B83`, `0091DD91`);
its message slot is the pop-4/AL-true stub `004FC900`. The +16 inherited unknown vtable `00A866EC`
includes the adjusted deleting destructor. These auxiliary interfaces are identified, but a complete generic
framework service ABI is outside this owner publication (TK1).

## 3. Construction, reference lifetime and shutdown

### 3.1 Creation and state — specified

Factory `00448FF0` requests **0x128 bytes** from GlobalHeapAlloc. A null allocation returns null without
constructing; success constructs and returns the base pointer in EAX. It does not QI, AddRef or Init.
The constructor initializes the primary reference count (+0x28) to **zero**, initialized and in-tick bytes
(+0x2C/+0x2D) false, and message pointer (+0x30) null (`00448E89–00448E95`).

Both embedded stopwatches (+0x38 and +0x98) use timer unit 3. Bounds are 2000/500000 microseconds;
timestamp (+0x58), frame counter (+0x5C), smoothed rate (+0x60), signed pause depth (+0x84), step-pending
(+0x8C), auto-enable (+0x8D), auto interval (+0x90), and four bucket timestamps (+0x118..+0x124) start
zero. Step delta (+0x88) starts **100000**. The eight phase masks are `[3,5,3,9,3,5,3,1]`
(`00448E9A–00448EDB`, `00448F96–00448FD6`). Four separate registration sets, the pending FIFO and
suppression set start empty. Initial registration hash capacity is **193**: constructor selects the first
prime at least 100 from `00A829B0`, whose first entries are 53,97,193,389,769 (`00448F31–00448F80`).

AddRef increments the 32-bit primary count and returns it; no atomics or overflow guard (`00447E30`).
Release uses an **unsigned** comparison: counts above one decrement; zero or one invoke the deleting
destructor through the +16 interface with delete flag 1 and return zero (`00447E40–00447E5A`).
Thus Release at the freshly constructed zero count destroys the object; this is not saturating Release.

### 3.2 Init and registered-service ownership — partial

On first Init only: set initialized true; clear pause depth and pending step; restart the main stopwatch;
obtain the framework and call its slots 3 and 9, in that order, with the service pointer base+12
(`00448020–00448054`). Their return values are ignored and the framework pointer has no null guard.
The concrete framework constructor installs vtable `00A8C970` (`00472E18`); literal entries resolve Init's
calls to `0087AC3C` / `00879BB9` and Shutdown's calls to `00879C6C` / `0087A90E`. **Unknown:** exact
framework-side retention, registration failure handling and caller scheduling are TK1; identifying these
targets alone does not prove that the framework owns one or two references. Their container/auto-reference
helpers require further dependency study; they are not added to the owner byte inventory.

If no reusable message exists, allocate 0x2C bytes, construct the standard message, AddRef it once, set type
`86AD10EE`, set void1 to the base manager, and assign its retained unknown to the base manager
(`00448060–0044809F`). Literal message vtable `00ACEC00` resolves setters to `009132DB`, `00913322`
and `0091346B`; the last tails to `005447B0`, which AddRefs the new non-null unknown before releasing
the old one. Therefore the reusable message holds **one primary manager reference**, in addition to
any external/framework references. The void1 pointer itself is only stored, not independently retained.
This is a real reference cycle broken by Shutdown. Allocation failure is not a false Init result: after a
null allocation the body dereferences the null message. Init returns AL true on every normal path.
Repeated Init while initialized does nothing. It does not reset the frame, timestamp, bucket timestamps,
bounds, auto-enable or step delta. These statements are local-owner effects, not framework guarantees.

### 3.3 Shutdown and destruction — partial

When initialized, Shutdown first clears the message's retained unknown (releasing that manager reference),
releases the message, and nulls the message pointer. It obtains the framework; if non-null it calls slots
10 then 4 with base+12, ignoring results; then invokes the service-base no-op AL-true Shutdown
(`004484E4–00448522`, shared `00949ADE`). The caller must keep the manager alive across this sequence;
there is no temporary self-AddRef before breaking the cycle.

It releases **each queued operation's** retained target without executing the operation, deletes the queue
nodes, resets the FIFO and suppression set (`00448527–00448583`). It then visits all four live registration
sets and releases every entry once, clearing their nodes (`00448588–00448624`). A target in two buckets
therefore gets two releases. Finally clear initialized, pause depth and pending step; reset the main timer;
return AL true (`0044862A–0044864A`). Bounds, phase counter, timestamps, auto flag/interval, step delta
and the auto timer are not reset by this body. When not initialized, Shutdown returns true immediately;
registrations added before Init are consequently not released by that no-op Shutdown.

The destructor is different: `004487D0` calls `00448340`, optionally frees the 0x128-byte allocation when
delete-flag bit 0 is set, and returns the original pointer. `00448340` destroys container storage and base
subobjects but **does not call Shutdown, release registered targets, or release the reusable message**.
Require orderly Shutdown before final Release for balanced references; do not silently claim destruction
provides that cleanup. Callback-triggered Shutdown/final Release and reentry from target Release during
queue drain are not given safe semantics here: the traversals have no general lifetime shield (TK3).

## 4. Registration, identity and deferred mutations

### 4.1 Stable calls outside a tick — specified

Priority classification uses unsigned comparisons (`00448BFA–00448C1D`):

| Priority | Bucket | Scheduled phases |
|---|---:|---|
| 192..4294967295 | 0 | all |
| 128..191 | 1 | 0,2,4,6 |
| 64..127 | 2 | 1,5 |
| 0..63 | 3 | 3 |

There is no exact-priority ordering within a band and no upper-priority rejection. Each bucket is a set
of the **exact interface pointer supplied**, not QI identity. Lookup hashes pointer>>2 modulo capacity
and compares raw pointers (`00448130`). Add checks only the selected bucket. If present, return true
without any target AddRef. If absent, call target AddRef once, grow/rehash if needed, and insert one entry
(`00448C29–00448C66`; insertion helper `0072BA50` compares identity and prepends a new chain node).
The same pointer can independently occupy several buckets and own one reference per bucket.

Find with priority `0xFFFFFFFF` searches buckets **3,2,1,0** and returns whether any contains the pointer;
all other values search only their classified bucket (`00448170–00448221`). It neither AddRefs nor changes
the sets. During a tick it still sees the live sets, not pending operations or suppression.

Remove with `0xFFFFFFFF` removes **one** occurrence: first found in 3,2,1,0 order. It does not remove all
memberships. Specific-priority Remove tries that band first; if absent, it calls Remove(target,FFFFFFFF),
so it can remove an entry in another band. If present, erase its node then Release the target once; absence
everywhere still returns true (`00448CF8–00448E20`). The misleading exported name at `004480B0`
is not behavior evidence: instructions unlink/free a node and decrement the set size.

SetNewPriority first AddRefs the target temporarily, removes one occurrence with the all-band sentinel,
adds at the requested priority, and Releases the temporary reference (`00447FD0–00448001`). It does
not require prior membership. It does not collapse other-band duplicates. A null target is not validated
before dereference on insertion/change/queued operations; there is no promised graceful invalid-pointer
result. These methods assume valid target interfaces and successful storage allocation.

### 4.2 Calls made while in-tick — specified normal path

Add and Remove both bypass immediate set mutation. Each calls target AddRef **once per request**, appends
an operation to the FIFO, adds the raw target pointer to the suppression set, and returns true. Add records
its requested priority. Remove records no effective priority: draining always uses `FFFFFFFF`, even if the
original removal requested another band (`00448B80–00448BEA`, `00448C84–00448CEC`). Duplicate requests
are not coalesced in the FIFO; only suppression membership is unique. A queued Add of an existing target
therefore suppresses its later deliveries just as Remove does.

On normal outer-tick exit, clear in-tick, then drain FIFO in request order (`00448A55–00448A65`). The
drain invokes ordinary Add for add records and ordinary Remove(target,FFFFFFFF) for remove records,
then Releases each operation's temporary reference (`00448744–00448777`). Once all operations have
been applied, free queue nodes, reset the FIFO and clear suppression (`00448779–004487B9`). This yields
safe ordinary self-removal/addition during DoMessage without invalidating the live tick iterator.
It does not guarantee safety for arbitrary target AddRef/Release side effects during draining (TK3).

## 5. Tick delivery and iteration

**Specified normal path.** A recursive DoAnimationTick while in-tick returns **zero**, before any frame
increment, clock read or queue drain (`00448816–00448823`, `00448A6A`). Otherwise increment the 32-bit
frame counter, take phase = its low three bits, set in-tick, read the main stopwatch and store its low EAX
word at +0x58 (`00448829–00448853`). This happens even if the subsequent sweep is paused.

For an admitted sweep (§6), visit buckets **0,1,2,3**, only when their bit in the phase mask is set. The first
call from a fresh counter uses phase **1**, not 0. For each scheduled nonempty bucket, calculate its delta
as in §6, then update the reusable message data2 and data3 once before iterating that bucket
(`00448915–00448993`). Enumerate hash slots in increasing index order, following each slot's chain;
the iterator advances **before** calling the target (`004489A4–004489CD`, `00448230`, `0047C1A0`,
`004B0390`). Initial hash capacity is 193; rehash `00448250` moves nodes by pointer hash and prepends
chains, so insertion order is not a stable overall delivery-order contract.

Immediately before each potential delivery, skip the target if its pointer is in the suppression set
(`004489D2–00448A05`, `006F1530`). Otherwise directly call target slot 3, DoMessage(reusableMessage),
and ignore its result (`00448A07–00448A12`). No message-server subscription, posting, per-target
AddRef/Release or per-target message copy occurs. A target present in multiple scheduled buckets can
receive multiple callbacks, unless an earlier callback suppresses it. An added target cannot join this
sweep; removing a not-yet-visited target suppresses it immediately. A delivered target's own self-removal
does not undo its current callback. A recursive tick cannot drain those requests early.

| Message field | Meaning / storage evidence |
|---|---|
| type | `0x86AD10EE`, set during Init; standard-message +0x0C, setter `009132DB` |
| void1 / data1 bits | manager base pointer; standard-message +0x10, setter `00913322` |
| data2 | clamped unsigned microseconds at +0x14, setter `00913330` |
| data3 | float32 millisecond bits at +0x18, setter `0091333E` |
| retained unknown | manager reference at +0x24, assignment `0091346B` → `005447B0` |

A scheduled bucket's last timestamp is updated even if empty, or if every delivery is suppressed
(`00448A22–00448A25`). Unscheduled buckets keep their timestamp. The EAX return starts at min for an
admitted sweep. After each **nonempty** scheduled bucket, replace it by that bucket's clamped delta only
while the current result is <= the then-current min (`00448A14–00448A1F`). With controls unchanged,
this is the first nonempty bucket delta strictly above min, or min if none is above min; it is not a sum,
frame count, target count or minimum of all bucket deltas. A paused/skipped sweep returns zero. Finally
clear a pending step after a completed sweep, clear in-tick and drain requests.

Callbacks receive the same mutable message. Fields are not restored between targets within a bucket;
consumer mutation may affect subsequent recipients. The next nonempty scheduled bucket rewrites data2
and data3, but does not reinitialize type/void1/retained unknown. No immutable-message guarantee is claimed.

## 6. Time arithmetic and pause/step transitions

### 6.1 Timestamp and clamp — specified at the owner boundary

Timers are unit-3 microsecond stopwatches (`00448E63`, `00448E9A`, `0088FEDF`). GetElapsedTime's unit-3
path converts its accumulated 64-bit host-counter ticks using float factor `00B54364`, then `_ftol2`
(`00890259–00890262`); the factor is independently identified by GetMicrosecondsPerTick `0088FF2B`.
The manager stores/uses **only EAX's 32 bits**. Host calibration, conversion overflow and absolute host
clock values belong to [timers.md](timers.md), not a new native-measured timer guarantee here.

All owner timestamp addition/subtraction is modulo 2^32, with unsigned clamp comparisons. For each
nonempty scheduled bucket: normally use stored-now minus its last timestamp; when step-pending is set,
first overwrite stored-now with that bucket's last timestamp plus stepDelta. Clamp the resulting raw delta
first down to max, then up to min (`0044891F–00448965`). Thus equality stays unchanged; step delta is
also clamped. The timestamp saved afterward is the **unclamped** stored-now, not last+clampedDelta.
There is no backward-clock detection: a backward sample becomes a large unsigned delta, usually max.

Step mode's timestamp is bucket-relative. The same +0x58 field is overwritten for each nonempty
scheduled bucket; its final value can therefore reflect a later bucket's different last timestamp. A scheduled
empty bucket neither adds stepDelta nor clamps anything: it copies the current shared timestamp into its
own last timestamp. Step-pending remains true throughout the sweep unless callback code changes it.

For milliseconds, instructions load the clamped dword as signed into x87, add exactly 2^32 if its sign bit
was set, multiply by the **float32 literal bits `0x3A83126F`** (`00A867A4`, approximately
0.0010000000474974513), then store/pop to float32 before passing its bits (`0044896B–00448993`).
The unsigned correction literal is float32 `0x4F800000` at `00A80AA8`. This is not double-literal 0.001
nor integer division by 1000. The owner does not change the process x87 rounding/precision controls;
bit-exact extremes remain conditional on the floating environment described by foundations.

SetMinAndMax stores both u32 inputs, replaces zero min by 1, then raises max to min when unsigned max<min
(`00447F40–00447F65`). It permits equality and values above INT_MAX. SetStepTickTimeDelta simply stores
all 32 bits, including zero and FFFFFFFF (`004FAD10`); it does not normalize to the bounds.

On phase 0 only, a separate float32 smoothed rate is updated using the modulo-32-bit difference from
the previous **stored shared timestamp**, plus 3. Convert that unsigned value through the same signed
x87 load/correction; divide float32 literal 1000000 (`00A867AC`) by it; subtract the old rate, multiply
by exact float32 0.125 (`00A867A8`), add the old rate and store float32 (`00448856–00448884`). This
happens before pause filtering. It has no clamp or zero-denominator guard and is not the slot-10 getter.

### 6.2 Pause, step and auto-step — specified normal path

Pause depth is a 32-bit counter, not a boolean or saturating nesting level. Pause(true) increments it;
Pause(false) decrements it without a floor. IsPaused returns signed depth>0. Tick admission, however,
tests **depth != 0** (`00447EE0`, `00447F30`, `00448891–004488D6`). Negative depth can therefore
report not paused while still suppressing delivery. Overflow/underflow wraps the stored bits.

StepAnimationSystem calls IsPaused: if true, set pendingStep=true; otherwise call Pause(true) and do
not arm a step (`00447F70–00447F8F`). The first Step from depth zero only pauses; the second arms one
sweep. Repeated Step calls before a tick coalesce to one flag, and do not add nested pauses when depth>0.

EnableAutoStep always stores its interval, even if enable is unchanged. On an enable-byte transition,
store the byte, then Restart the auto stopwatch when enabled, or Reset it when disabled. Repeating enabled
with a new interval does not restart it (`00447F90–00447FC7`). Pause(true) also Restarts that stopwatch
when auto is enabled and the **old signed depth <= 0**, before incrementing depth (`00447EEB–00447F17`).

At the beginning of each nonrecursive tick, if auto is enabled and depth is nonzero, compare unsigned
auto elapsed with interval. At elapsed>=interval, set pendingStep and **Reset**, not Restart, the auto
stopwatch (`00448887–004488B9`). Reset clears both start and accumulation (`0088FF0A`); an unchanged
positive interval consequently does not yield periodic future steps without another restart. With interval
zero, even reset elapsed zero satisfies equality, so each tick can arm a sweep. Auto enable itself does not
pause, and no auto elapsed check occurs at depth zero.

After that check, if depth!=0 and pendingStep is false, skip the sweep. Otherwise perform the ordinary
phase-selected sweep; a step does not force all four buckets. Clear pendingStep at sweep end, without
decrementing depth. No callback/function-pointer argument is stored or invoked by these controls.
Pause changes during DoMessage do not re-test admission for the current sweep; later buckets still run.
Other control changes may affect later bucket computations because fields are read again. Arbitrary
destruction or exception paths during a callback remain partial (TK3).

## 7. Reconciliation of existing publications

| Prior input | Verdict | Owner evidence and qualification |
|---|---|---|
| INVENTORY-M6 F4; class/service IDs; primary ABI slots 5/6 | MATCHES | §1–§2, constructor/QI/factory and literal vtable. This is inventory/identity, not fresh consumer coverage. |
| lot-developer §6.7.8 priority bands, preincrement, masks, defaults, direct delivery and message fields | MATCHES | §§4–6 with unsigned widths and literal float multiplier now explicit. Existing consumer findings remain prior work. |
| lot-developer §6.7.8 "step uses last+100000" | PARTIAL | Matches constructor default; configurable u32 step delta, per-nonempty-bucket shared timestamp overwrite and subsequent clamp matter (§6). |
| re-answers30 / terrain TR27 `0xC0` tick registration | MATCHES (manager side) | Bucket 0 is every admitted sweep. Terrain-side water gating was not re-investigated or claimed as new work. |
| effects §6 data2 microseconds | MATCHES | `00448981–00448987`, field setter `00913330`. Effects time dilation remains its own consumer contract. |
| effects §6.1 nonzero pause skips all target ticks | PARTIAL | Nonzero depth only skips when pendingStep is false; auto/manual stepping permits a sweep (`004488C8–004488D6`). |
| effects §6.1 `EnableAutoStep(1, EffectsTickCallback)` on manager +0xCD8 | DIFFERS | Original call `00594EE1–00594EF3` uses **address of the embedded +0xCD8 object**, pushes zero and `0058D9A0`, and calls slot +0x24. This manager's EnableAutoStep is +0x38 with a u32 time interval, not a callback. No ownership of that embedded object is inferred here. |
| effects §6.1 framework drives manager; effects callback once per manager step; paused game stops it | UNKNOWN at this owner boundary | Export has zero direct calls to `004487F0` and only its vtable data reference `00A86784`; an indirect caller must be traced separately. The manager's own message receiver does not tick (§2). TK1 retains the external scheduling question. |

These reconciliations do not silently rewrite consumer specs or claim that export access repaired REA.

## 8. Proposed literal distinguishing vectors

None of these vectors has been run in the original, a native hook or an implementation. They are review
inputs derived independently from the contracts above. Unless stated otherwise: valid live manager/targets,
Init completed, stable bounds 2000/500000, no callback side effects, and no serialization.

| Vector | Setup / action | Expected observable result |
|---|---|---|
| V1 full cycle | Fresh frame=0; register one target in each bucket; invoke eight admitted ticks | Phases `1,2,3,4,5,6,7,0`; ordered bucket lists `[0,2] [0,1] [0,3] [0,1] [0,2] [0,1] [0] [0,1]`; callback totals `8,4,2,1`. |
| V2 band edges | Distinct targets at priorities `63,64,127,128,191,192` | Buckets `3,2,2,1,1,0`; priorities 64 and 127 share a cadence, as do 128 and 191. `FFFFFFFF` is bucket 0 for Add but the search sentinel for Find/Remove. |
| V3 stable duplicate | Add A at192 twice, then191 once | Target reference delta `+1,0,+1`; two memberships. In phase2 A receives twice, bucket0 before1. |
| V4 same-target remove | A in buckets0,1,3; Remove(A,FFFFFFFF), then Remove(A,64) | First removes bucket3 only, -1 ref. Specific bucket2 is absent so fallback removes bucket1, -1 ref. A remains bucket0. Absent-everywhere Remove still returns true with zero ref effect. |
| V5 same-band change | A solely in bucket0; SetNewPriority(A,200) | Temporary +1, remove -1, insert +1, temporary -1; final count unchanged and one entry. Unregistered A instead gains one live entry/reference. |
| V6 self-removal / recursion | A in buckets0 and2 on phase1. In its bucket0 callback queue Remove(A,192); recursively invoke tick | Queued request +1 temporary; recursive result0, no phase change; A skipped in bucket2. Drain ignores192 and removes bucket2 (search3→0), releases registration and temporary refs. Bucket0 survives. |
| V7 add/remove FIFO | X absent. During callback Add(X,192), Remove(X,192) | X gets no callback now. Queue retains +2; drain insert +1, all-band remove -1, temporaries -2; final absent, net0. Reverse requests leave X present with net+1. |
| V8 queued duplicate suppression | A already bucket0 and1; before its bucket1 visit queue Add(A,192) twice | Both operations retain A (+2); bucket1 callback skipped. Drain finds duplicate adds, no live AddRef; releases both temporaries. Live memberships unchanged. Find during callback still returns true. |
| V9 clamp boundaries | A alone in bucket0; each independent case last=10000, now=`11999,12000,12001,509999,510000,510001` | Raw `1999,2000,2001,499999,500000,500001`; data2 `2000,2000,2001,499999,500000,500000`; saved last is the corresponding raw now. |
| V10 wrap/backward | Independent cases last=`FFFFFFF0`,now=`00000020`; last=10000,now=9999 | Raw48→data2=2000; raw`FFFFFFFF`→data2=500000. No signed-negative rejection. |
| V11 bound equality | SetMinMax(0,0); then SetMinMax(7,3); then SetMinMax(9,9) | Stored pairs `(1,1)`, `(7,7)`, `(9,9)`; any delivered raw delta clamps to that equal bound. SetMinMax(`80000000`,1) stores both as `80000000` under unsigned comparison. |
| V12 step delta edges | Paused, step armed, A only bucket0 last10000; independent stepDelta=`0,1999,2000,500000,500001` | data2=`2000,2000,2000,500000,500000`; final timestamp=`10000,11999,12000,510000,510001`. |
| V13 shared step timestamp | Phase1, A in0 last10000, B in2 last20000, step armed, stepDelta100000 | A delta100000/timestamp110000; B delta100000/timestamp120000; final getter120000, bucket0 last110000, bucket2 last120000. Empty bucket2 instead copies110000. |
| V14 pause nesting | Depth0; Step, tick, Step, tick, tick | Depth remains1; first tick skips and returns0; second performs its scheduled phase and clears pending; third skips. All three nonrecursive calls increment phase/read main timer. |
| V15 underflow | Depth0; Pause(false); IsPaused; tick; Step | Depth bits `FFFFFFFF`; getter false; tick skips. Step calls Pause(true), yielding depth0 without arming pending. |
| V16 auto threshold | Enable(true,100); Pause(true); auto elapsed99 tick then elapsed100 tick | First skips; equality arms one sweep and resets timer. Subsequent elapsed0 ticks skip at interval100. Re-enable(true,0) updates interval without restart; subsequent ticks meet0>=0 and step. |
| V17 return channel | Phase1 with bucket0 delta2000 and bucket2 delta10000; separately bucket0 delta9000 and bucket2 delta3000 | EAX10000 in first case, EAX9000 in second; never sum/min of both. Empty admitted sweep returns min; recursive or pause-skipped call returns0. |
| V18 references and QI | Fresh factory pointer; QI(86AD10ED,out); QI(452294AA,out); unsupported IID with sentinel out; Release twice without Init | Counts0→1→2; successful pointers base and base+4. Failed QI false leaves sentinel. Releases return1 then0 and destroy. Fresh zero-count Release also destroys. |
| V19 orderly teardown | Keep external owner reference, Init; register A in0 and1; ordinary Shutdown outside a tick. Separately consider a seeded state with two outstanding FIFO operations on B | Message holds +1 manager until Shutdown clears it. Shutdown releases A twice. In the seeded cancellation case it releases B's two temporary references without applying either request. This seeded state is a conceptual branch vector, not proof of a safe public-call sequence that preserves pending work outside a tick; callback teardown remains TK3. |
| V20 message floats | Delivered deltas2000,100000,500000 under usual nearest rounding | data3 float32 bits `40000000`, `42C80001`, `43FA0001` (2.0, about100.0000076, about500.0000305 ms); distinguish float32 multiplier from exact double0.001. No native bit-verification credit. |

For target ordering within a hash slot, test pointer identities explicitly; do not use host allocation order
as an oracle. Proposed tests can verify pointer-set and suppression behavior without assuming a portable
address ordering. V19's canceled-queue condition is a lifetime scenario to arrange safely, not an instruction
to trigger unsupported callback teardown.

## 9. Remaining limits

Open questions [TK1–TK3](questions/animation-tick-manager.md) delimit framework service/scheduling ownership,
serialization and exceptional/destructive reentrancy. The catch fragment `00448A7E–00448AA3` visibly clears
in-tick and drains pending work before a runtime call at `009EF84F`; exception-type handling and unwind
effects are not established. Do not promise rollback, swallowed exceptions or safe destruction from it.
No cross-thread safety is established: state and references use ordinary loads/stores. Invalid interfaces,
storage exhaustion after factory allocation and consumer message corruption have no graceful-failure
contract. An implementation may add defensive behavior, explicitly as a choice, without claiming it is
measured original behavior.
