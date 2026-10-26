# widget-toolkit: answered questions (archive)

<!-- from: core-uiwin-close13 (2026-10-24) -->
- **WT14 - sections 12 and 14.5 opaque graphics and slot 150:** section 12 explicitly leaves graphics slot meanings and create_private_buffer internals unsettled. Please publish the allocation/replacement/failure and ownership contract of concrete slot 150 and any further base-owned operations hidden by the opaque graphics calls. [execute the specified ordered calls via Host/NativeOp, propagate allocation failure as specified; do not invent concrete allocation or opaque service implementation]
  **ANSWERED 2026-10-24 (re-widgettk-wt14):** MATCHES — §§14.5.1–14.5.3 specify destructive draw-service replacement versus fallback object reuse, the signed both-nonpositive rejection, by-value format `(1,0)`, output/reference effects and failure handling, graphics slot meanings, and 14 conformance vectors. §14.6 specifies borrowed drawing caches and temporary interface ownership; §12 removes the obsolete base-window gap. Graphics internals remain service-owned. The destroy path tests existing fade/paint bits rather than setting them; reported backend failure is distinguished from unsafe native allocation exhaustion.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- ~~**WT10 - sections 7.2 and 14.3 filter mutation:** what does RemoveMessageFilter do during nested iteration, and are additions visible to the running pass? The deferred list is named but its insertion/removal/ownership rules are not published. [snapshot each pass; defer removals until the outermost pass exits; additions appear on the next pass, including a nested pass]~~
  **ANSWERED 2026-10-26 (re-wt29):** DIFFERS — rechecked the 2026-10-24 answer against the instructions. Section 7.2 specifies a live list, same-pass additions, owning duplicate deferred requests, and a drain only at the end of an outer PlotComposite filter pass, never at DoMessage return. Pending removals still receive nested callbacks. Temporary argument and drain references are balanced separately from list ownership. Repeated unmatched queued removals expose a sentinel-erasure bug rather than harmless deduplication. Sections 7.2, 10.1 and 14.3 give worked cases, counts and the paint/message distinction. Removed the stale duplicate open entry.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- ~~**WT11 - section 7.3 type 1:** DoMessage dispatches to the void ChildDelete slot, but what byte return does it expose afterwards? [perform the deletion and return false; do not invent a handled value from a void slot]~~
  **ANSWERED 2026-10-26 (re-wt29):** DIFFERS — rechecked the 2026-10-24 answer at 0x0099CE8F–9A and 0x0099CEF2–F8: section 7.3 specifies unmodified post-call AL. Base ChildDelete tail-forwards manager DestroyWindow's byte, including true when queued. A custom void override supplies no defined byte by its declaration alone. Section 10.1 includes true and noncanonical-byte vectors. Removed the stale duplicate open entry.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- ~~**WT12 - sections 1, 2, 4 and 8 lifetime boundaries:** please specify cGZWin QueryInterface output/ref effects, initial reference count, final Release/deleting-destructor coupling, manager destruction while old COW blocks retain references, and invalid/self/cyclic parenting inputs. [model valid acyclic registered UI trees with a caller-owned creation reference; implement ordinary wrapping reference arithmetic separately from explicit manager destruction; do not claim native lifetime/ABI closure]~~
  **ANSWERED 2026-10-26 (re-wt29):** DIFFERS — rechecked and completed the 2026-10-24 answer. Sections 2, 4.2 and 8.1 specify zero initial count; QI success output/AddRef and untouched failure output; wrapping AddRef but destruction at Release counts zero or one; virtual deleting slot 148(1), now identified as shared wrapper 0x009D060A. Retained COW references delay final destruction/unregistration. Manager requests queue without ownership/detachment while busy and otherwise orchestrate Shutdown and reference releases, not unconditional free. Direct parenting has no validity/self/cycle guard. Section 10.1 gives lifetime and boundary vectors, including a caller-owned root surviving a manager request. Removed the stale duplicate open entry.

<!-- from: re-widgets4 (2026-10-21) -->
- **WT5 - section 4.2:** cGZWin::ChildDeleteAll 0x0099DD6F loops on the current front child until the list is empty; does cIGZWinMgr::DestroyWindow (slot 23) always detach the child from its parent synchronously, also when the manager is busy and defers the destroy? If not, the loop would not terminate. [assume it detaches synchronously]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — evidence needed for WT12 also settles this older question: busy DestroyWindow only queues a raw pointer and returns true, without detaching. Section 4.2 specifies the resulting nontermination for nonempty ChildDeleteAll under persistent busy depth; section 8.1 gives the manager addresses.
  **Also (re-gzwinmgr7, 2026-10-24):** the busy count is the manager's `+0x18C`; it is above 0 only while the manager broadcasts a posted message with a null target (`ProcessPostedMessages`, `0x009DC2D1`); a busy `DestroyWindow` (`0x009DC4AC`) appends the window once to the condemned list `+0x190` (slot 58); widget-winmgr.md §4.1.

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTC1 - widget-custom.md section 1:** settle constructor/secondary-vtable and nondeleting teardown attribution around 0095B9A1.
  **ANSWERED 2026-10-24 (re-gzwin-spec9):** MATCHES — constructor 0095BAB3 adds only the custom tables after base construction; the four-slot secondary interface at +0xD8 shares primary QI/reference lifetime and returns the complete pointer through AsIGZWin. Teardown 0095B9A1 directly calls base Shutdown then base destruction. See widget-custom.md section 1 and section 3 vectors.
<!-- Archive for [widget-toolkit.md](../../widget-toolkit.md). Lanes do not read this file. Open questions are in ../widget-toolkit.md. -->

<!-- from: re-widgets4 (2026-10-21) -->
- **WT4 - section 3.1 (and ui-windows.md section 2.2):** cGZWin::SetFlag 0x0099DB6B returns the whole flag word in EAX (0x0099DCD9), not 0/1 as ui-windows.md section 2.2 states; should ui-windows.md be corrected to point here? [yes]
  **ANSWERED 2026-10-21 (re-widgets4):** DIFFERS — SetFlag has one exit (every branch jumps to 0x0099DCD9, MOV EAX,[ESI+0xC8]; RET 8), so it returns the whole new flag word; ui-windows.md section 2.2 row 68 corrected to point to widget-toolkit.md section 3.1.

<!-- from: re-widgets4 (2026-10-21) -->
- ~~**WT2 - section 2:** what are cGZWin +0x0C, +0x24..+0x30, +0x34..+0x40, +0xA0..+0xA4 and +0xB8..+0xC4 (the constructor zeroes some, leaves +0x34..+0x40 and +0xA0..+0xA4 untouched)? Candidates: area-to-draw-to and draw-buffer origin. [unknown; implementers should zero them]~~
  **ANSWERED 2026-10-23 (re-gzwin7):** `+0x0C` is the reference count (`AddRef` `0x0094C47C` increments it and
  returns the new value). `+0x24`, `+0x34` and `+0xB8` are the draw area, draw origin and blit rectangles of section
  17.1. `+0xA0`/`+0xA4` are a mouse-down offset (x, y) that only `set_mouse_down_offset` (`0x0099BDA4`) writes; its
  callers are the dialog classes `cGZWinGenDlg` (`0x009C2307`) and `cGZWinMsgBox` (`0x009CD9E2`). Nothing in
  `cGZWin` reads it, so leaving it unset as the constructor does is compatible. widget-toolkit.md sections 2 and 18
  updated.

<!-- from: re-widgets4 (2026-10-21) -->
- ~~**WT6 - section 8.1:** what is the initial value of the window manager's instance-ID counter (manager +0x194) assigned by registration (manager slot 20, 0x009DC586)? [unknown; first ID may be 0 or 1]~~
  **ANSWERED 2026-10-24 (re-gzwinmgr7):** 1. The base constructor `0x009DC5E8` writes `mov dword [esi+0x194], 1`, and
  registration uses the value before incrementing it, so the first registered window without an ID gets 1.
  widget-toolkit.md sections 8.1 and 10 updated.

<!-- from: re-widgets4 (2026-10-21) -->
- ~~**WT1 - section 1:** the cGZWin own vtable 0x00ADC8D8 (written by both constructors and the destructor) has no rows in docs/abi/vtables.tsv; how many slots does it have and which are pure stubs? [assume the cIGZWin 148-slot layout plus slots 148-151 as on cGZWinGen]~~
  **ANSWERED 2026-10-26 (re-wt29):** DIFFERS — section 1.1 specifies 151 callable slots, 0–150, followed by zero padding at 0x00ADCB34; slot 151 belongs only to the generator extension. No callable slot is null or a pure-virtual trap. Constant-result defaults are slot 88 (true) and 129–131, 133–143 (false), with target addresses and argument widths tabulated. Slots 148–150 are the shared deleting wrapper, transparency helper and private-buffer allocator. Section 1.1 gives inventory and callback worked cases. ABI export rows were not changed.

<!-- from: re-widgets4 (2026-10-21) -->
- ~~**WT3 - section 3:** default flag bits 0x100 and 0x8000 are set by the constructor but not tested by cGZWin itself; which subclass or manager code reads them? [unknown; keep them set]~~
  **ANSWERED 2026-10-26 (re-wt29):** MATCHES — keep both bits in default 0x8903. Section 3 identifies concrete readers: cGZWinGenDlg mouse-down/move uses 0x100 to select drag initiation and movement versus parent forwarding; manager SetFocus 0x009DB804 tests 0x8000 as focus eligibility before changing focus or sending notifications. Base SetFlag adds no special side effect for either mask. Section 3 includes drag-offset and two-window focus worked cases.

<!-- from: core-uiwin-close13 (2026-10-24) -->
- ~~**WT13 - sections 12 and 14.7 cGZFadeTmpWin destructor:** section 12 explicitly leaves this derived destructor unspecified. Does it add any teardown beyond the settled base destructor and deleting-wrapper flag contract in section 8.1? Please publish any derived reference effects and deleting-wrapper behavior. [implement the specified constructor, StealBuffer and Plot plus inherited base lifetime; leave derived destructor closure open]~~
  **ANSWERED 2026-10-26 (re-wt29):** DIFFERS — section 14.7 closes teardown statically: fade and base slot 148 share 0x009D060A, which calls the base destructor once, optionally frees storage for argument bit 0, and returns the object address. There are no extra derived reference effects. The constructor variant is owned through base SetParam; initialized Shutdown releases it, while never-initialized destruction frees raw-pointer map nodes without releasing values. Section 14.7 gives initialized, nonfreeing, higher-flag-bit and never-initialized worked cases; section 12 removes the destructor gap. No runtime observation remains required.
