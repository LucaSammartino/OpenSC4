# widget-toolkit: open questions
<!-- Questions about [widget-toolkit.md](../widget-toolkit.md). Append new entries at the END; when one is answered, move it to answered/widget-toolkit.md. Rules: README.md. -->

<!-- from: re-widgets4 (2026-10-21) -->
- **WT1 - section 1:** the cGZWin own vtable 0x00ADC8D8 (written by both constructors and the destructor) has no rows in docs/abi/vtables.tsv; how many slots does it have and which are pure stubs? [assume the cIGZWin 148-slot layout plus slots 148-151 as on cGZWinGen]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT2 - section 2:** what are cGZWin +0x0C, +0x24..+0x30, +0x34..+0x40, +0xA0..+0xA4 and +0xB8..+0xC4 (the constructor zeroes some, leaves +0x34..+0x40 and +0xA0..+0xA4 untouched)? Candidates: area-to-draw-to and draw-buffer origin. [unknown; implementers should zero them]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT3 - section 3:** default flag bits 0x100 and 0x8000 are set by the constructor but not tested by cGZWin itself; which subclass or manager code reads them? [unknown; keep them set]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT5 - section 4.2:** cGZWin::ChildDeleteAll 0x0099DD6F loops on the current front child until the list is empty; does cIGZWinMgr::DestroyWindow (slot 23) always detach the child from its parent synchronously, also when the manager is busy and defers the destroy? If not, the loop would not terminate. [assume it detaches synchronously]
<!-- from: re-widgets4 (2026-10-21) -->
- **WT6 - section 8.1:** what is the initial value of the window manager's instance-ID counter (manager +0x194) assigned by registration (manager slot 20, 0x009DC586)? [unknown; first ID may be 0 or 1]

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT7 - section 13.1 complete accelerator lifetime inventory:** please publish the constructor/initial reference count, slot 0 QueryInterface success AddRef and failure output effects, shared AddRef overflow/Release-at-zero behavior, and slot 15 deleting-destructor argument bits, cleanup order and nondeleting behavior. Section 13 names all 16 vtable slots but does not specify these owner bytes/method effects or any nonvirtual constructor/destructor helpers. Is there additional owner behavior outside this inventory? [implement only IID membership, caller-supplied nonzero live count, checked ordinary AddRef/Release and final Shutdown plus Rust-owned map cleanup; no native constructor/QI side-effect/deleting-flag or full-owner claim]

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT8 - section 13 boundary inputs and lifecycle repeats:** does slot 9 OR the full 32-bit vk into the code as the printed formula says, or truncate it to 16 bits as the preceding description suggests? Are repeated Init/Shutdown both successful no-ops? If the service lookup yields no server, does Init still set the flag and return true, and should a later Init retry? [literal printed full-u32 OR/shift formula, repeated calls return true, missing service sets initialized and is not retried until Shutdown; these choices remain unverified]

<!-- from: core-key-accelerator4 (2026-10-22) -->
- **WT9 - section 13.3 published interface conflict:** the public SDK cIGZWinKeyAccelerator.h lists the type registration overload before the record overload, message lookups before primitive lookups, and a bool GetKeysForAccelerator return, whereas section 13/ABI slots put record registration at 5, primitive lookups at 9/10, message lookups at 11/12 and return the total count at 13. Please confirm the authoritative native signatures/order and return width before an adapter is wired. [pure-core methods follow section 13; no C ABI or native vtable adapter added]

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTG1 - widget-generator.md sections 1, 4 and 5:** finish the secondary generator interface inventory and user-data overload signatures; verify inverse title geometry, all generator flag side effects, title/side/color-bar construction resources, close-result constants, edge/cursor selection and bitmap mode pixel contracts. The new spec deliberately closes only lifetime, procedure routing and the client-layout boundary. [keep owner partial; do not infer signatures from misleading exported method names]

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTH1 - widget-html-window.md sections 1, 3 and 4:** finish constructor/defaults and both secondary vtables; WWW request flags, refresh identity and stale completion handling; completion record/error transitions; option masks, key/link notifications, full scrollbar ranges, background phase and HTML drawing metrics. Primary window dispatch is inherited, while 008FBFE5 is the completion-target handler. [keep owner partial; implement the documented status/scroll and parse replacement boundaries only]

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTC1 - widget-custom.md section 1:** settle the complete constructor/secondary-vtable and nondeleting teardown attribution around 0095B9A1; named/folded owner counts alone do not prove closure. [use the established primary forwarding/paint behavior; retain partial coverage]
