# framework-gz-services: open questions
<!-- Questions about [framework-gz-services.md](../framework-gz-services.md). Append new entries at the END; when one is answered, move it to answered/framework-gz-services.md. Rules: README.md. -->

<!-- from: re-gzservices4 (2026-10-21) -->
- **GZ1 - section 4.1 slot 12:** `0x0092D0B3` forwards to the +8 interface's slot 5 (`0x009D7E63`, returns the
  dword at +0xC of the server). Is that dword the system service's reference count (as the +8 vtable's
  RemoveRef at slot 4 suggests) or another counter? Needs the cRZSystemService layout (`0x0091DE0F`). [reference
  count: unconfirmed]
- **GZ2 - section 2 slots 20/21:** the blocking gets wait on the queue's thread signal with argument 1,000,000.
  Unit (microseconds = 1 s, or milliseconds) and whether a wait resets the signal depend on cRZThreadSignal
  (vtable slot 3 Wait, slot 1 Signal), which this spec does not cover. [unknown; no in-game caller found yet]
- **GZ3 - section 4.2:** the registry depth is only non-zero inside AddNotification/RemoveNotification/MessageSend
  while the registry lock is held, so the pending path needs a re-entrant call on the same thread during those
  windows (no target code runs there). Is any such path reachable in the shipped game, or is the pending list
  dead in practice? [believed dead; implement it anyway for parity]
- **GZ4 - section 4.5:** which game classes use the directed-message slots 7/8 (callers of the interface +0x1C /
  +0x20)? Needed to tell implementers whether directed delivery order matters to observable behaviour. [unknown]
