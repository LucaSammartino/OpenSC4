# widget-toolkit: answered questions (archive)

<!-- from: core-uiwin-close13 (2026-10-24) -->
- **WT14 - sections 12 and 14.5 opaque graphics and slot 150:** section 12 explicitly leaves graphics slot meanings and create_private_buffer internals unsettled. Please publish the allocation/replacement/failure and ownership contract of concrete slot 150 and any further base-owned operations hidden by the opaque graphics calls. [execute the specified ordered calls via Host/NativeOp, propagate allocation failure as specified; do not invent concrete allocation or opaque service implementation]
  **ANSWERED 2026-10-24 (re-widgettk-wt14):** MATCHES — §§14.5.1–14.5.3 specify destructive draw-service replacement versus fallback object reuse, the signed both-nonpositive rejection, by-value format `(1,0)`, output/reference effects and failure handling, graphics slot meanings, and 14 conformance vectors. §14.6 specifies borrowed drawing caches and temporary interface ownership; §12 removes the obsolete base-window gap. Graphics internals remain service-owned. The destroy path tests existing fade/paint bits rather than setting them; reported backend failure is distinguished from unsafe native allocation exhaustion.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- **WT10 - sections 7.2 and 14.3 filter mutation:** what does RemoveMessageFilter do during nested iteration, and are additions visible to the running pass? The deferred list is named but its insertion/removal/ownership rules are not published. [snapshot each pass; defer removals until the outermost pass exits; additions appear on the next pass, including a nested pass]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — section 7.2 specifies a live list, same-pass additions, owning duplicate deferred requests, and a drain only at the end of an outer PlotComposite filter pass, never at DoMessage return. Pending removals still receive nested callbacks. Repeated unmatched queued removals expose a sentinel-erasure bug rather than harmless deduplication. Sections 10.1 and 14.3 give vectors and the corrected paint/message distinction.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- **WT11 - section 7.3 type 1:** DoMessage dispatches to the void ChildDelete slot, but what byte return does it expose afterwards? [perform the deletion and return false; do not invent a handled value from a void slot]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — section 7.3 specifies unmodified post-call AL. Base ChildDelete tail-forwards manager DestroyWindow's byte, including true when queued. A custom void override supplies no defined byte by its declaration alone. Section 10.1 includes true and noncanonical-byte vectors.

<!-- from: core-uiwin-close11 (2026-10-24) -->
- **WT12 - sections 1, 2, 4 and 8 lifetime boundaries:** please specify cGZWin QueryInterface output/ref effects, initial reference count, final Release/deleting-destructor coupling, manager destruction while old COW blocks retain references, and invalid/self/cyclic parenting inputs. [model valid acyclic registered UI trees with a caller-owned creation reference; implement ordinary wrapping reference arithmetic separately from explicit manager destruction; do not claim native lifetime/ABI closure]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — sections 2, 4.2 and 8.1 specify zero initial count; QI success output/AddRef and untouched failure output; wrapping AddRef but destruction at Release counts zero or one; virtual deleting slot 148(1); retained COW references delay final destruction/unregistration. Manager requests queue without ownership/detachment while busy and otherwise orchestrate Shutdown and reference releases, not unconditional free. Direct parenting has no validity/self/cycle guard. Section 10.1 gives lifetime and boundary vectors; WT1's unexported base table remains separate.

<!-- from: re-widgets4 (2026-10-21) -->
- **WT5 - section 4.2:** cGZWin::ChildDeleteAll 0x0099DD6F loops on the current front child until the list is empty; does cIGZWinMgr::DestroyWindow (slot 23) always detach the child from its parent synchronously, also when the manager is busy and defers the destroy? If not, the loop would not terminate. [assume it detaches synchronously]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — evidence needed for WT12 also settles this older question: busy DestroyWindow only queues a raw pointer and returns true, without detaching. Section 4.2 specifies the resulting nontermination for nonempty ChildDeleteAll under persistent busy depth; section 8.1 gives the manager addresses.

<!-- from: re-gzwin-spec9 (2026-10-24) -->
- **WTC1 - widget-custom.md section 1:** settle constructor/secondary-vtable and nondeleting teardown attribution around 0095B9A1.
  **ANSWERED 2026-10-24 (re-gzwin-spec9):** MATCHES — constructor 0095BAB3 adds only the custom tables after base construction; the four-slot secondary interface at +0xD8 shares primary QI/reference lifetime and returns the complete pointer through AsIGZWin. Teardown 0095B9A1 directly calls base Shutdown then base destruction. See widget-custom.md section 1 and section 3 vectors.
<!-- Archive for [widget-toolkit.md](../../widget-toolkit.md). Lanes do not read this file. Open questions are in ../widget-toolkit.md. -->

<!-- from: re-widgets4 (2026-10-21) -->
- **WT4 - section 3.1 (and ui-windows.md section 2.2):** cGZWin::SetFlag 0x0099DB6B returns the whole flag word in EAX (0x0099DCD9), not 0/1 as ui-windows.md section 2.2 states; should ui-windows.md be corrected to point here? [yes]
  **ANSWERED 2026-10-21 (re-widgets4):** DIFFERS — SetFlag has one exit (every branch jumps to 0x0099DCD9, MOV EAX,[ESI+0xC8]; RET 8), so it returns the whole new flag word; ui-windows.md section 2.2 row 68 corrected to point to widget-toolkit.md section 3.1.
