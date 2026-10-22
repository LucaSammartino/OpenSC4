# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE1 - section 4:** What exact byte encodings and malformed-input outcomes do GetCharLength, GetPreviousCharLength, GetGlyphCountFromString and the pending-input helpers implement for selectors 8/100 and other selectors? The editor calls these primitives; numeric selector values alone do not establish UTF-8/DBCS or Unicode grapheme behavior. [unknown; preserve the explicit foreign boundary]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE5 - sections 7-8; text-edit-layout.md:** Remaining paths: Page-key versus arrow preferred-X cache use/reset (pseudocode suggests pages measure current X while arrows reuse cache; exact instruction audit pending), single-line caret-in-view three-quarter-width stepping, complete AddScrollBar construction/resource negotiation and SelectCurrentWord/composition repair. Native word/composed stepping predicates, line/index conversion, pixel-hit mapping, visible-span metrics, character/pixel scroll offsets, scrollbar size/range arithmetic, page-scroll operations and cursor-scroll timer routing are now specified. Foreign encoding/font/bar implementations remain boundaries; no whole-owner credit.

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** Attribution is reconciled in text-edit-attribution.md: actual coverage assigns 150 bodies / 19,999B, including the 134 direct entries and sixteen measured thunks/helpers. Complete remaining U/B checklist entries (thin text/caption adapters, color/outline bounds, exact line/index conversion, scroll helpers, image setters and generated action helpers) from native bodies before complete-owner credit. [partial specification only; no whole-owner or widget-family credit]
