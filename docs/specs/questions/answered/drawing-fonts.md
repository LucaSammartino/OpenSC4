# drawing-fonts: answered questions (archive)
<!-- Archive for [drawing-fonts.md](../../drawing-fonts.md). Lanes do not read this file. Open questions are in ../drawing-fonts.md. -->

<!-- from: re-drawing4 (2026-10-21) -->
- **FN1 - section 10:** the sharpness exponent is k = pow(sharpness + 1, 1 / L(2)); L is the CRT routine at
  `0x009F2160` (Ghidra FunctionID conflict, named `_cos`; an SSE2/x87 dispatcher). Is L the natural log (k = e at
  sharpness 1) or log10 (k = 10)? [natural log assumed; needs the dispatcher target `0x009FE548` or a live REA read]
  **ANSWERED 2026-10-21 (re-fontanswers4):** DIFFERS - section 10: L is log10, not ln (x87 FLDLG2/FYL2X at 0x009F221F; SSE2 0x009FE548 scales by log10(2) at 0x00AEAE60); k = (sharpness+1)^(1/log10 2) = (s+1)^3.321928, k = 10 at sharpness 1.
- **FN2 - section 11:** the per-character form choice in `MakeSubstitutedString` (`0x00899665`, second loop) uses
  `UTF16GetThaiCharType` classes of the previous and next characters (classes 6..8, 9..11, 13..15; base consonants
  U+0E1B, U+0E1D, U+0E1F, U+0E2C select form 1, tone marks after an upper vowel add 2). The Ghidra flow is unreliable
  here; please give the exact class table and decision order from the instructions. [section 11 states only the
  SARA AM split and the substitution table]
  **ANSWERED 2026-10-21 (re-fontanswers4):** DIFFERS in detail - section 11: class table of UTF16GetThaiCharType (0x0091169F, 17 classes) and the four-rule order; tone marks / U+0E4C add 2 when class(prev) < 10 (not "after an upper vowel"); a one-step carry of form 1 passes through below vowels 6..7 after a tall consonant.
- **FN3 - section 12:** `cGZFontSys::FontAcquire` by family list (`0x00894ADF`): the scoring (start 1000; +0x20 /
  +2 / +4 for name position, +8 when the entry's style is a subset of the request, +0x10 otherwise; style compared
  after masking 0x3C) is garbled in the pseudocode (an unconditional `= 0x20`). Exact score per candidate and the
  substitution walk? [lowest score wins, ties keep the first]
  **ANSWERED 2026-10-21 (re-fontanswers4):** DIFFERS in part - section 12: name score 0 (first name), 2 (later name), 0x20 (no match); 4 applies only to an empty list when entry.kind != kind argument; style +0 equal, +8 subset, +0x10 otherwise (request bits 0x3C cleared); strict < so ties keep the earlier path; substitution walk replaces names not in the installed family set.
- **FN4 - section 12:** confirm the aaMode rule of `FontAcquire(path, …)` (`0x00895A1C`): monochrome inside the
  system range +0x18..+0x1C, or inside the entry's words 9..10; where are +0x14/+0x18/+0x1C set (a cGZFontSys slot
  not in decomp/)? [as written]
  **ANSWERED 2026-10-21 (re-fontanswers4):** MATCHES with additions - section 12: +0x14 default (slot 7 0x00893D87), +0x18/+0x1C system mono range and the per-family map +0x64 (slot 9 0x008954CB); the per-family lookup uses GetName before Init, so the fresh font's empty name.
- **FN5 - section 8.2:** `CalculateWidthOfLines` (`0x008968EB`) binary-searches widths mid·10 for mid in [2, 400]
  against lines = height / lineHeight; the return value and the tie branch are lost in the pseudocode. Exact return?
  The 7-argument `CalculateTextArea` (`0x008973D4`, slot 45: default min width 12·width('a'), max 32·width('a'))
  is also unspecified. [unspecified]
  **ANSWERED 2026-10-21 (re-fontanswers4):** DIFFERS from the bracket - section 8.2: CalculateWidthOfLines returns lo*10 when lo == hi and fits, 0 when nothing up to 4000 fits; slot 45 CalculateTextArea(s,n,rect,minW,maxW,maxH) takes 6 stack args, full rule written.
- **FN6 - section 1:** `IsCodePageAvailable` (`0x00898A5A`) probes sample characters per code page (e.g. 1251 →
  U+0401, 874 → U+0E01/U+0E2C, 932 → U+3061/U+30A4, 936 → U+70A7/U+7535, 949 → U+C2B9/U+C9D1, 950 → U+6F54/U+70B5,
  1253 → U+0370/U+0393, 1254 → U+011E, 1255 → U+05D0/U+05E2, 1256 → U+0621/U+0641, 1250/1257 → U+0142/U+0160,
  else U+0061) via IsCharAvailable; please confirm the table from the instructions (several branches were removed as
  unreachable). [as listed]
  **ANSWERED 2026-10-21 (re-fontanswers4):** DIFFERS in part - section 6: 1250 probes only U+0142 and 1257 only U+0160; cp 0 and 1252 probe U+0061; cp 8/0x10 and unlisted code pages return true without probing.
<!-- from: core-fonts4 (2026-10-21) -->
- **FN7 - §7 AddCharData advance:** the advance is given only for advanceScale == 1.0 ("linear or metric advance / 65536"). Which advance (linear or metric) is used, and what is the advance when advanceScale ≠ 1.0? For an unloadable glyph, what are the box height, yOffset and advance? [provider's one advance value; scaled = truncate(adv/65536 · advanceScale); box charHeight rows at yOffset 0, advance = charHeight/2] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — advance = face+0x80 (fixed-size face) or face+0xD0, /65536 at scale 1.0, else ftol(adv*s*2^-16) with a -1 rule for spaces at 0.98<s<1; missing glyph = hollow (h/2)x(h/2) outline at yOffset 0, advance ftol(h/2*s) (spec §7).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN8 - §7 underline/strike:** how are `uOffset` (`0x00896618`) and the line thickness computed (face fields, size-dependent)? Is the strike line as thick as the underline? [both taken from the face provider as pixels; one thickness for both] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — thickness from pointSize only: underline 1/2/3/4 at <16/<26/<36/else, strike 1/2/3/4 at <24/<48/<72/else; underline first row = ascent - yOffset + thickness (spec §7).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN9 - §7 font's own map `0x00898EB2`:** what does the source-type 2/3 fallback lookup map (ch → glyph index) and from what data? [provider hook, default 0 = no glyph] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — holder map cache, then a 479-entry code->glyph-name table (0x00B150F0), then "uniXXXX", resolved through the face; hits cached (spec §7).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN10 - §5/§7 source type of a font:** where does a `cGZFont` get the source type tested in SetSize and AddCharData (a field, the catalogue, GetFontSourceTypeFromFileName on filePath), and when? [GetFontSourceTypeFromFileName(filePath) at Init] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — holder+0x24 from a content probe at face open (DSIG sfnt 4, loca sfnt 1, "%!" 2, else open fails); type 3 unreachable (spec §5).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN11 - §2 constructor defaults:** the initial values of aaMode, aaGamma, aaSharpness, scaleX/Y, advanceScale, codePage, defaultChar, colours and +0x50 after `new cGZFont`. [scales, advanceScale and gamma 1.0; all others 0] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — pointSize = charHeight = 16, aaMode 1, +0x50 = 2, gamma 1.0, sharpness 0.0, scales 1.0, defaultChar 0x20, others 0 (spec §2).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN13 - §8 words and characters:** `UTF16GetWord` (what ends a word, are trailing spaces part of it), the "break characters" of mode 1, a "Thai cluster" in mode 2, a "composed character" in GetStringWidth, the whitespace set dropped by GetStringVisibleWidth, GetAdvanceWidth with no glyph, whether mode 2 can return 0 units (CalculateNumberOfLines would not advance), and whether CR LF is consumed as one line end or two. [word = non-blank run + its spaces/tabs; break = space, tab; cluster and composed char = one UTF-16 unit; whitespace = 0x20, 0x09..0x0D; no glyph → 0; mode 2 takes at least 1 unit; CR and LF each end one line] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — line-break classes (hard break, break-after, no-start, no-end, CJK, Thai) define words incl. trailing spaces; space set 0x20, U+2002..U+200B; Thai cluster up to 4 units, width of first unit only; CR LF = two line ends; no glyph -> 0 (spec §8.1, §8.4).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN14 - §12 FontAcquire:** with `cache` and no catalogue entry for the path, is the font recorded (a new entry?) or not; and does `RZParseDelimitedText` keep empty fields ("Arial,,Tahoma", "") as names? [not recorded; empty fields dropped, so "" gives n = 0] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — with cache a missing catalogue entry is created (and family added); empty fields are kept except a trailing one, "" gives n = 0 (spec §12).
<!-- from: core-fonts4 (2026-10-21) -->
- **FN12 - §9 palette channels:** which pixel format do `RZ::GS()` +0x90/+0x8C split and compose (A8R8G8B8, the display format)? Is alpha interpolated too? [A8R8G8B8, all four channels] **ANSWERED 2026-10-21 (re-font-core-answers4):** DIFFERS — §9: RZ::GS() +0x90/+0x8C forward to the screen buffer's split/compose (0x00825F80/0x00825DA0), so the format is the screen colour type (7 = A4R4G4B4 at depth <= 16, 9 = A8R8G8B8 above); fg/bg are cut to their low 16 bits before the split (red is 0 in 32-bit mode); alpha is not interpolated (compose forces 0xF / 0xFF); k = 14 is overwritten by fg; no screen buffer leaves entries 8..119 undefined.

<!-- from: core-fonts-final4 (2026-10-22) -->
- ~~**FN15 - §7 name map table:** the 479 {u16 code, PostScript glyph name} pairs at `0x00B150F0` are named only by example (0x20 "space", 0x21 "exclam", 0xB7 "periodcentered", 0x3A5 "Upsilon"). Please list the whole table (code → name, first-of-duplicates already applied) so the core can ship it. [typed provider `FontLibrary::ps_glyph_name`, default none: the lookup goes straight to "uniXXXX"]~~
  **ANSWERED 2026-10-22 (re-font-final-gaps4):** DIFFERS — §7 now references the complete normative 477-row table, docs/specs/data/font-name-map-fn15.tsv, extracted from all 479 records with reproducible executable/TSV SHA-256 and PE offsets. First duplicate wins at 0x00897ECD..0x00897EF7: U+00B7 → periodcentered, U+03A5 → Upsilon. Alias-only-face and U+FFFF vectors distinguish the lookup/fallback sequence.

<!-- from: core-fonts-final4 (2026-10-22) -->
- ~~**FN16 - §7 advance precision:** ftol(adv · s · 2^-16) runs on the x87; which precision control is in force at that point in the game (53-bit MSVC default, or 24-bit as left by Direct3D)? With 24-bit the product adv · s rounds to a float before the truncation. [53-bit: computed in f64, exact for |adv| < 2^29]~~
  **ANSWERED 2026-10-22 (re-font-final-gaps4):** DIFFERS — §7: application startup explicitly sets x87 PC 24 (0x0045236C → 0x005F85E0 → 0x008929E0), overriding the CRT 53-bit default. The font inherits current precision/rounding for its two multiplies (0x008991C5..0x008991D1, 0x008991FE..0x0089920A), then _ftol2 truncates. adv=65535, s bits 3F800080 yields 1 at PC24 versus 0 at PC53/64. Also corrected the proved narrow-band non-space branch: it ignores s and uses integer adv/65536 (8 px at s bits 3F7D70A4 remains 8, not 7). No claim of invariant PC across arbitrary callers.

<!-- from: core-fonts-final4 (2026-10-22) -->
- ~~**FN17 - §9 Lock/UnlockSurfaceForFontUse (slots 55/56):** §9 says only that nested locks are counted and that the 0→1 and 1→0 edges call the renderer (+0x14/+0x18). What do the slots return, and what happens (a) on the 0→1 edge when no renderer is set or the renderer +0x14 call fails (is the count still incremented?), and (b) on Unlock with the count already 0 (no-op, or does it wrap / still call +0x18)? The core currently returns false and leaves the count unchanged in (a) and (b), true otherwise.~~
  **ANSWERED 2026-10-22 (re-font-final-gaps4):** DIFFERS — §9: explicit slots always return boolean true on normal return, ignoring renderer results; Lock increments after an optional old-count-0 callback, Unlock decrements after an optional old-count-1 callback. Unlock at 0 wraps to FFFFFFFF with no callback. Null renderer on an edge faults before changing count, rather than returning false. Dispatch/count/AL anchors 0x00896BB3/BB6/BBC and 0x00896BE7/BEA/BF0; §9 has failure, nesting, underflow and null-edge vectors.
