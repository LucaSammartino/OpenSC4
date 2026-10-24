# text-edit: answered questions

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** Generated-action copy/allocation failure/unwind; foreign base/font/drawing/bar/timer/dispatch/clipboard outcomes beyond documented call sites; lib::_strncpy padding and search primitives 004427F0/005C23E0 edge/error behavior. Attribution remains 150 bodies / 19,999B; inventory counts alone establish no complete-owner or widget-family credit.
  **ANSWERED 2026-10-24 (re-widget-controls-te13b):** DIFFERS — section 6 specifies owned-string copy, growth capacity, commit ordering and absent local unwind/rollback, including unsafe null-allocation behavior. The adapters supplement specifies fixed-count NUL padding and exact forward/reverse byte search, overlapping matches and owner rejection of end-position empty matches. Sections 5 and 8–10 define foreign services as controlled inputs, without inventing their implementations or recovery. Seven vectors distinguish the new rules. The attribution denominator and conservative P/B classifications are unchanged; TE5 is separately answered.

<!-- from: re-textedit4 (2026-10-22); residual narrowed in round 1 -->
- **TE5 - sections 7–8 and encoding/layout/adapters supplements:** Exact conversion selectors/malformed-input results, Thai context levels and composed lengths?
  **ANSWERED 2026-10-24 (re-widget-controls-te13b):** DIFFERS — text-edit-encoding.md now specifies conversion counts, alias behavior, prefix/error handling, Windows call parameters, intermediate conversion, Thai classes/levels and both composed-length directions. Quirks include low-16 truncation of long UTF8 forms, consumed-but-unwritten UTF16 units, forward/backward 0E33 asymmetry and the unguarded fourth-word read. New vectors distinguish each; Windows code-page tables remain explicit environment inputs rather than unresolved owner behavior.

<!-- from: core-widgets5 (2026-10-23) -->
- **TE9 - section 1 constructor / section 6 history limit:** what initial values are stored in the maximum-undo-level word W+0x184 and preferred-X W+0x170? [The portable byte/state stage requires explicitly resolved history-limit and preferred-X values; no native constructor default is inferred.]
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — the same stores inspected for TE10 explicitly initialize maximum undo levels to `0x4000` and preferred X to `0xFFFFFFFF`. Section 1 Runtime defaults supplies both, citing `0x009BFFBF` and `0x009C000D`; this does not reopen TE5/TE8.

<!-- from: core-widgets-close12 (2026-10-24) -->
- **TE10 - section 1 runtime constructor defaults:** initial colors, four sound IDs, horizontal offsets, visible metrics and line height?
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — section 1 Runtime defaults specifies three `0xFF000000` colors, zero offsets/metrics, initial line height 1 and true image edge option. The four sound words W+0x27C..+0x288 are not initialized by construction; zero is a portable substitute only. Preferred X is `0xFFFFFFFF`, maximum undo levels `0x4000`.

<!-- from: core-widgets-close12 (2026-10-24) -->
- **TE11 - section 8 shared automatic-reflow counter:** exact comparison and increment/decrement scope?
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** DIFFERS — section 8 specifies signed counter < 1, increment/call/decrement around only RecalculateAll's automatic check, normal return only. One check at ordinary counter zero matches the provisional rule, but exception-safe restoration does not.

<!-- from: core-widgets-close12 (2026-10-24) -->
- **TE12 - section 9 background image mode:** Boolean polarity for direct versus edge drawing?
  **ANSWERED 2026-10-24 (re-widgettk-wt13):** MATCHES — false is direct, true is edge. Section 9 also specifies mode 1, signed division of source right/bottom by three and scrollbar strip exclusion only on the edge branch. Section 11 adds distinguishing vectors for all three answers.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE1 - section 4:** Exact byte-length/count and pending-input helper encodings and malformed-input outcomes?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — text-edit-encoding.md records permissive selector-8 byte tables, no remaining-length clamp or later-byte validation in forward length, backward all-continuation N+1, non-continuation glyph counting, exact numeric lead ranges for 932/936/949/950 and byte behavior for 100. Pending C0/C1 lead acceptance disagrees with its expected-tail table; eligible later bytes are counted without trail validation. System selector and composed/UTF16 services remain explicit foreign inputs; no modern Unicode/grapheme claim.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE3 - section 6:** Raw insertion after lowering the maximum and empty RemoveChar safe-call domains?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — ordinary insertion wraps M-L; a deficit of at least two reaches the native string length-failure helper, while deficit one has an invalid source range without that guard. Selectors 8/100 instead retain zero, invoke reason 1 and complete the zero-byte update path. Empty RemoveChar manufactures begin-1/index 0xFFFFFFFF; selector 0/16 ordinary mode with null output reaches string erase's out-of-range failure, while other branches may access the invalid pointer earlier. Section 6 defines valid domains and makes no universal benign or single-exception promise.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE6 - section 9:** Password glyph masks versus byte line/caret indices?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — mask and selection endpoints are glyph-count based, but line starts, spans and caret measurement retain original byte indices. Pixel hit mapping uses selector zero on the mask, converts only its interior-hit branch through original-whole-text LengthFromGlyphCount, and returns original line byte count at the right end. Section 9 specifies numeric hit thresholds and safe-buffer limits; no repaired Unicode rendering is implied.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE7 - section 10:** Destructor member cleanup, image ownership, color bounds and lifecycle failures?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — the W+0x208 destructor helper is a one-byte return; neither editor Shutdown nor destruction releases the retained background image. Scrollbar image setters AddRef before Release even on identity, copy all rectangle words and use image bounds for empty rectangles; Shutdown releases those distinct images. Color indices/output pointers are unchecked; outline setters do not invalidate. Section 10 defines null font/WCM, unguarded cursor-manager/zero-height boundaries and callback null-dispatch handling. Foreign allocation, font, timer delivery and queue cancellation remain explicit service boundaries, not inferred recovery.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE4 - section 6:** Reconcile insertion-action quarter eviction and deletion record creation's absent limit check; does maximum zero keep growing?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — insertion recording erases the oldest floor(n/4) actions when n >= configured unsigned limit, then appends; deletion recording appends without checking that limit. Native range-erase/copy/destruction helpers establish survivor order and owned-string cleanup. Zero does not disable recording: insertion-only growth from empty stabilizes at four, while deletion-only growth is unbounded by this setting. Setter alone never prunes. See section 6 and four independent added vectors.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE2 - section 2:** What does primary hook +0x26C do for reasons 0, 1 and other editing failure reasons?
  **ANSWERED 2026-10-22 (re-textedit4):** DIFFERS — section 10 specifies native 0x009BD9F2 as a sound-service adapter, indexing W+0x27C+4*reason, acquiring and playing through RZ::SS, releasing its temporary handle on success/failure, and returning false for missing ID/service. Its folded font-setter name is misleading. No error-dialog behavior is established.
