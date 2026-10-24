# framework-gz-services: answered questions

<!-- from: glue-gzmsg-shadow8 (2026-10-24) -->
- ~~**GZ11 - sections 3/4 read-only native observation:** What public read-only interface enumerates all queue-manager IDs and queue identities, queue contents, and server pending/deferred posts and notification registrations? Alternatively, publish a complete read-only snapshot layout and synchronization/lifetime contract or an observation hook covering every mutation. Section 3 exposes only lookup by caller-supplied ID; pending posts/registrations belong to section 4's server, not the manager. [Default-off bounded diagnostic probes IDs 0 through 255 plus the documented manager service/IID and UINT32_MAX. Capture only numeric membership/identities at load and compare unchanged lookup inputs monthly against that immutable baseline through the core manager, without resync. No assumption that these IDs enumerate the registry, no inference of unobserved mutations or queue contents, and no whole-owner verification credit. A change in observed membership/identity is reported as a mismatch, not repaired. Full state shadow remains blocked.]~~
  **ANSWERED 2026-10-24 (re-gzmsg-gz11):** MATCHES — §3.1 establishes no public complete enumeration, snapshot or mutation-observer hook on these owners. Manager slot 9 is an unlocked unsigned ID-entry count, not an iterator; queue peeks/searches and server ordinary-post count cannot capture all contents, directed posts or registrations. Counted query results protect their objects, not bindings or a cross-object snapshot. §5.3 GZ-V17–V22 establish the diagnostic limits. Full-state shadow remains unsupported by these interfaces.

<!-- from: glue-gzmsg-shadow8 (2026-10-24) -->
- ~~**GZ12 - sections 1/3 native manager acquisition:** In real Konradshohe, `GetSystemService(0x64FA8434, 0x652294BC, &out)` fails at load and all 72 monthly samples through 2032 (`gzmsg-shadow8-r1-2032`, clean shutdown, zero foreign exceptions/reference warnings). Is this service registered in a normal city, conditionally created, or exposed under a different acquisition contract? Please specify how to acquire the existing native manager read-only, including ownership and availability phase. The SDK's EnumSystemServices explicitly leaves its enumerator signature unknown, so it cannot supply a safe fallback. [Keep gzmsg_shadow default-off; unavailable capture is mismatches=1, never an exact empty registry. No native factory calls, service insertion, guessed enumeration callback or object-layout reads. No manager verification credit until acquisition succeeds.]~~
  **ANSWERED 2026-10-24 (re-gzmsg-gz11):** DIFFERS — §3.2 distinguishes registered class factory 0x452294DF from optional service 0x64FA8434. Normal application startup installs both message servers without the manager; there is no guaranteed city/month availability phase or alternative existing-manager lookup. The original service/IID pair is correct when an owner has registered one; success AddRefs, caller Releases, and a missing service returns false leaving output untouched. Factory construction creates a new empty manager and cannot capture existing state. §5.3 GZ-V15/V16 specify absence and successful fixture acquisition; native unavailability retains mismatch/no-credit status.

<!-- from: re-gzservices4 (2026-10-21) -->
- ~~**GZ1 - section 4.1 slot 12:** `0x0092D0B3` forwards to the +8 interface's slot 5 (`0x009D7E63`, returns the
  dword at +0xC of the server). Is that dword the system service's reference count (as the +8 vtable's
  RemoveRef at slot 4 suggests) or another counter? Needs the cRZSystemService layout (`0x0091DE0F`). [reference
  count: unconfirmed]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** MATCHES — shared unsigned system-service reference count at server +0xC; constructor and both interface forwarding paths proved in §4.1, vector GZ-V01.

- ~~**GZ2 - section 2 slots 20/21:** the blocking gets wait on the queue's thread signal with argument 1,000,000.
  Unit (microseconds = 1 s, or milliseconds) and whether a wait resets the signal depend on cRZThreadSignal
  (vtable slot 3 Wait, slot 1 Signal), which this spec does not cover. [unknown; no in-game caller found yet]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** DIFFERS — §2 establishes unsigned microseconds /1000, so 1 second; queue mode-0 Signal pulses rather than retaining a wake token. Wait has no explicit reset and ignored failure/timeout results; vectors GZ-V02/V03 cover lost wake and failure.

- ~~**GZ3 - section 4.2:** the registry depth is only non-zero inside AddNotification/RemoveNotification/MessageSend
  while the registry lock is held, so the pending path needs a re-entrant call on the same thread during those
  windows (no target code runs there). Is any such path reachable in the shipped game, or is the pending list
  dead in practice? [believed dead; implement it anyway for parity]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** DIFFERS — native target AddRef/Release and message GetType run at positive depth; Release/destructor re-entry can defer registrations. §4.2 rejects the dead-path premise; vectors GZ-V06/V07 specify controlled reachability. No measured game occurrence or exhaustive native-target census is claimed.

- ~~**GZ4 - section 4.5:** which game classes use the directed-message slots 7/8 (callers of the interface +0x1C /
  +0x20)? Needed to tell implementers whether directed delivery order matters to observable behaviour. [unknown]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** DIFFERS — §4.5 proves server-1 directed self-posts in cSC4WinIntroVideoScreen DoUserStop/GZPaint and two positive server-2 call sites by native acquisition/dispatch instructions. FIFO directed delivery is observable. No class label is guessed for unnamed bodies and no concrete slot-8 game caller is established; cancellation is specified from its native bodies.

<!-- from: core-gzmessages4 (2026-10-21) -->
- ~~**GZ5 - section 4A slots 7/8:** "Post directed ... as §4.5" for the first-generation server: is the 16-byte
  record copied into the deque (no message reference), and is the target AddRef'd on append and released on
  cancel / after delivery as in §4.5? The core implements copy + target referenced. [implemented literally]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** DIFFERS — §4A: 20-byte entry is raw target plus copied 16-byte message, with no target or message reference calls at append/cancel/delivery. Vector GZ-V08. Source choice comparison uses the filed question only; no implementation was read.

- ~~**GZ6 - sections 4.2/4.3/4.5 reference timing:** (a) does message server 2's MessageSend AddRef the copied
  targets (§4A says server 1 does; §4.3 is silent; the core does not); (b) is the list's reference on a removed
  target dropped while the registry depth is still 1 (the core does, so a re-entrant destructor defers to the
  pending list); (c) are the directed deque's references dropped after `DoMessage` (the core: message, then
  target). [implemented literally]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** DIFFERS — (a) counted snapshot, all target holds released after all callbacks in forward order (§4.3); (b) MATCHES, unlink then Release at depth 1 (§4.2); (c) deque holds released before callback, persistent local pair holds through it and across iterations, with final message-then-target cleanup after unlock (§4.5). Vectors GZ-V05/V06/V09; comparisons use filed choices only.

- ~~**GZ7 - section 2 slots 6/8/10/17/24:** the message equality test (+0x18): is it called on the queued message
  with the argument (`queued.equals(arg)`) or the reverse? The core calls `queued.equals(arg)`. [choice]~~
  **ANSWERED 2026-10-22 (re-gz-messaging-contracts4):** MATCHES — queued receiver passes the search argument, testing only AL at all five native sites (§2, vector GZ-V04).
