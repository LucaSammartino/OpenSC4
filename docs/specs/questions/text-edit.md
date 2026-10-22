# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE1 - section 4:** What exact byte encodings and malformed-input outcomes do GetCharLength, GetPreviousCharLength, GetGlyphCountFromString and the pending-input helpers implement for selectors 8/100 and other selectors? The editor calls these primitives; numeric selector values alone do not establish UTF-8/DBCS or Unicode grapheme behavior. [unknown; preserve the explicit foreign boundary]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE3 - section 6:** What is the exact safe-call domain and observed native result of insertion after lowering maximum below existing byte length, and RemoveChar on empty text? Native arithmetic includes unsigned subtraction/underflow; no success or benign no-op is promised. [unknown; keep callers within established domains]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE4 - section 6:** Reconcile insertion-action count eviction (60-byte records, earliest-quarter expression) against native instructions and STL erase helper, and deletion record creation's missing limit check. Does max zero keep growing without eviction? [unknown; no conventional capacity guarantee]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE5 - sections 7-8:** Complete exact word/composed stepping, Page Up/Down preferred-X behavior, pixel-to-line mapping, horizontal visible-span metrics, character-offset scrolling, scrollbar sizes/ranges/resources and cursor-scroll timer motion. Distinct owner bodies exist; broad navigation/scrollbar credit is withheld. [unknown beyond the specified paths]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE6 - section 9:** How do password mask glyph offsets interact with byte-indexed line starts and caret columns for multibyte text? GZPaint uses the mask buffer but some native line/caret coordinates remain byte-based. [unknown; do not promise corrected Unicode rendering]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE7 - section 10:** Complete native destructor helper 0x009BFAE3 and related helpers, background/font/image return ownership, scrollbar image replacement ordering, color override bounds and lifecycle failures. Does native pre-init or resource-null usage have additional required preconditions? [unknown beyond explicitly documented calls]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** Reconcile the brief's cGZWinTextEdit19999B attribution to exact native primary bodies. The export has no such suffix; 134 direct nonnested class labels total 19,415B, all 146 name matches total 19,908B, and unrelated folded names/unnamed helpers require independent attribution. Complete U/B checklist entries (thin text/caption adapters, color/outline bounds, exact line/index conversion, scroll helpers, image setters and generated action helpers) from native bodies before any complete-owner credit. [partial specification only; no whole-owner or widget-family credit]
