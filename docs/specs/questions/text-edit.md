# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE1 - section 4:** What exact byte encodings and malformed-input outcomes do GetCharLength, GetPreviousCharLength, GetGlyphCountFromString and the pending-input helpers implement for selectors 8/100 and other selectors? The editor calls these primitives; numeric selector values alone do not establish UTF-8/DBCS or Unicode grapheme behavior. [unknown; preserve the explicit foreign boundary]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE5 - sections 7-8:** Complete exact word/composed stepping, Page Up/Down preferred-X behavior, pixel-to-line mapping, horizontal visible-span metrics, character-offset scrolling, scrollbar sizes/ranges/resources and cursor-scroll timer motion. Distinct owner bodies exist; broad navigation/scrollbar credit is withheld. [unknown beyond the specified paths]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE8 - section 12; text-edit-inventory.md:** Attribution is reconciled in text-edit-attribution.md: actual coverage assigns 150 bodies / 19,999B, including the 134 direct entries and sixteen measured thunks/helpers. Complete remaining U/B checklist entries (thin text/caption adapters, color/outline bounds, exact line/index conversion, scroll helpers, image setters and generated action helpers) from native bodies before complete-owner credit. [partial specification only; no whole-owner or widget-family credit]
