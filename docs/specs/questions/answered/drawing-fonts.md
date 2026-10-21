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
