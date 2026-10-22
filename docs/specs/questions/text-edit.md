# text-edit: open questions
<!-- Questions about [text-edit.md](../text-edit.md). -->

<!-- from: re-textedit4 (2026-10-22) -->
- **TE1 - section 4:** What exact byte encodings and malformed-input outcomes do GetCharLength, GetPreviousCharLength, GetGlyphCountFromString and the pending-input helpers implement for selectors 8/100 and other selectors? The editor calls these primitives; numeric selector values alone do not establish UTF-8/DBCS or Unicode grapheme behavior. [unknown; preserve the explicit foreign boundary]

<!-- from: re-textedit4 (2026-10-22) -->
- **TE2 - section 2:** What does primary hook +0x26C do for reasons 0, 1 and other editing failure reasons? Call sites establish reason arguments but do not establish downstream sound, callback or error delivery. [unknown; do not infer a UI error message]
