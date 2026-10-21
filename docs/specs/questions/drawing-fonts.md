# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->
<!-- from: core-fonts4 (2026-10-21) -->
- **FN7 - §7 AddCharData advance:** the advance is given only for advanceScale == 1.0 ("linear or metric advance / 65536"). Which advance (linear or metric) is used, and what is the advance when advanceScale ≠ 1.0? For an unloadable glyph, what are the box height, yOffset and advance? [provider's one advance value; scaled = truncate(adv/65536 · advanceScale); box charHeight rows at yOffset 0, advance = charHeight/2]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN8 - §7 underline/strike:** how are `uOffset` (`0x00896618`) and the line thickness computed (face fields, size-dependent)? Is the strike line as thick as the underline? [both taken from the face provider as pixels; one thickness for both]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN9 - §7 font's own map `0x00898EB2`:** what does the source-type 2/3 fallback lookup map (ch → glyph index) and from what data? [provider hook, default 0 = no glyph]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN10 - §5/§7 source type of a font:** where does a `cGZFont` get the source type tested in SetSize and AddCharData (a field, the catalogue, GetFontSourceTypeFromFileName on filePath), and when? [GetFontSourceTypeFromFileName(filePath) at Init]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN11 - §2 constructor defaults:** the initial values of aaMode, aaGamma, aaSharpness, scaleX/Y, advanceScale, codePage, defaultChar, colours and +0x50 after `new cGZFont`. [scales, advanceScale and gamma 1.0; all others 0]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN12 - §9 palette channels:** which pixel format do `RZ::GS()` +0x90/+0x8C split and compose (A8R8G8B8, the display format)? Is alpha interpolated too? [A8R8G8B8, all four channels]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN13 - §8 words and characters:** `UTF16GetWord` (what ends a word, are trailing spaces part of it), the "break characters" of mode 1, a "Thai cluster" in mode 2, a "composed character" in GetStringWidth, the whitespace set dropped by GetStringVisibleWidth, GetAdvanceWidth with no glyph, whether mode 2 can return 0 units (CalculateNumberOfLines would not advance), and whether CR LF is consumed as one line end or two. [word = non-blank run + its spaces/tabs; break = space, tab; cluster and composed char = one UTF-16 unit; whitespace = 0x20, 0x09..0x0D; no glyph → 0; mode 2 takes at least 1 unit; CR and LF each end one line]
<!-- from: core-fonts4 (2026-10-21) -->
- **FN14 - §12 FontAcquire:** with `cache` and no catalogue entry for the path, is the font recorded (a new entry?) or not; and does `RZParseDelimitedText` keep empty fields ("Arial,,Tahoma", "") as names? [not recorded; empty fields dropped, so "" gives n = 0]
