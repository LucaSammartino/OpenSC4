# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->

<!-- from: re-drawing4 (2026-10-21) -->
- **FN1 - section 10:** the sharpness exponent is k = pow(sharpness + 1, 1 / L(2)); L is the CRT routine at
  `0x009F2160` (Ghidra FunctionID conflict, named `_cos`; an SSE2/x87 dispatcher). Is L the natural log (k = e at
  sharpness 1) or log10 (k = 10)? [natural log assumed; needs the dispatcher target `0x009FE548` or a live REA read]
- **FN2 - section 11:** the per-character form choice in `MakeSubstitutedString` (`0x00899665`, second loop) uses
  `UTF16GetThaiCharType` classes of the previous and next characters (classes 6..8, 9..11, 13..15; base consonants
  U+0E1B, U+0E1D, U+0E1F, U+0E2C select form 1, tone marks after an upper vowel add 2). The Ghidra flow is unreliable
  here; please give the exact class table and decision order from the instructions. [section 11 states only the
  SARA AM split and the substitution table]
- **FN3 - section 12:** `cGZFontSys::FontAcquire` by family list (`0x00894ADF`): the scoring (start 1000; +0x20 /
  +2 / +4 for name position, +8 when the entry's style is a subset of the request, +0x10 otherwise; style compared
  after masking 0x3C) is garbled in the pseudocode (an unconditional `= 0x20`). Exact score per candidate and the
  substitution walk? [lowest score wins, ties keep the first]
- **FN4 - section 12:** confirm the aaMode rule of `FontAcquire(path, …)` (`0x00895A1C`): monochrome inside the
  system range +0x18..+0x1C, or inside the entry's words 9..10; where are +0x14/+0x18/+0x1C set (a cGZFontSys slot
  not in decomp/)? [as written]
- **FN5 - section 8.2:** `CalculateWidthOfLines` (`0x008968EB`) binary-searches widths mid·10 for mid in [2, 400]
  against lines = height / lineHeight; the return value and the tie branch are lost in the pseudocode. Exact return?
  The 7-argument `CalculateTextArea` (`0x008973D4`, slot 45: default min width 12·width('a'), max 32·width('a'))
  is also unspecified. [unspecified]
- **FN6 - section 1:** `IsCodePageAvailable` (`0x00898A5A`) probes sample characters per code page (e.g. 1251 →
  U+0401, 874 → U+0E01/U+0E2C, 932 → U+3061/U+30A4, 936 → U+70A7/U+7535, 949 → U+C2B9/U+C9D1, 950 → U+6F54/U+70B5,
  1253 → U+0370/U+0393, 1254 → U+011E, 1255 → U+05D0/U+05E2, 1256 → U+0621/U+0641, 1250/1257 → U+0142/U+0160,
  else U+0061) via IsCharAvailable; please confirm the table from the instructions (several branches were removed as
  unreachable). [as listed]
