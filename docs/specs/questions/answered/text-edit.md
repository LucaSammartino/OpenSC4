# text-edit: answered questions

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
