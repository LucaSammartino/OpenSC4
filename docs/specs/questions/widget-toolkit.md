# widget-toolkit: open questions
<!-- Questions about [widget-toolkit.md](../widget-toolkit.md). Append new entries at the END; when one is answered, move it to answered/widget-toolkit.md. Rules: README.md. -->

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT7 - section 13.1 complete accelerator lifetime inventory:** please publish the constructor/initial reference count, slot 0 QueryInterface success AddRef and failure output effects, shared AddRef overflow/Release-at-zero behavior, and slot 15 deleting-destructor argument bits, cleanup order and nondeleting behavior. Section 13 names all 16 vtable slots but does not specify these owner bytes/method effects or any nonvirtual constructor/destructor helpers. Is there additional owner behavior outside this inventory? [implement only IID membership, caller-supplied nonzero live count, checked ordinary AddRef/Release and final Shutdown plus Rust-owned map cleanup; no native constructor/QI side-effect/deleting-flag or full-owner claim]

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT8 - section 13 boundary inputs and lifecycle repeats:** does slot 9 OR the full 32-bit vk into the code as the printed formula says, or truncate it to 16 bits as the preceding description suggests? Are repeated Init/Shutdown both successful no-ops? If the service lookup yields no server, does Init still set the flag and return true, and should a later Init retry? [literal printed full-u32 OR/shift formula, repeated calls return true, missing service sets initialized and is not retried until Shutdown; these choices remain unverified]

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT9 - section 13.3 published interface conflict:** the public SDK cIGZWinKeyAccelerator.h lists the type registration overload before the record overload, message lookups before primitive lookups, and a bool GetKeysForAccelerator return, whereas section 13/ABI slots put record registration at 5, primitive lookups at 9/10, message lookups at 11/12 and return the total count at 13. Please confirm the authoritative native signatures/order and return width before an adapter is wired. [pure-core methods follow section 13; no C ABI or native vtable adapter added]

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTG1 - widget-generator.md sections 7 and 8.5:** remaining closure is the linked pixel contract for RZDrawRaisedRect (008D9900) and RZDrawBltEdge (008D8800), including degenerate destination rectangles and the mode-2 source-cell layout. Round 2 specifies the full secondary table, userdata ownership, inverse title geometry, flag side effects, decoration creation/failures, close records, edge/cursor selection and draw-context calls in section 8. [keep owner partial only for these two helper contracts; title-bar owner is separately ranked]

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTH1 - widget-html-window.md sections 1, 3 and 4:** finish constructor/defaults and both secondary vtables; WWW request flags, refresh identity and stale completion handling; completion record/error transitions; option masks, key/link notifications, full scrollbar ranges, background phase and HTML drawing metrics. Primary window dispatch is inherited, while 008FBFE5 is the completion-target handler. [keep owner partial; implement the documented status/scroll and parse replacement boundaries only]

