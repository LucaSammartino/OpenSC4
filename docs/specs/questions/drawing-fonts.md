# drawing-fonts: open questions
<!-- Questions about [drawing-fonts.md](../drawing-fonts.md). Append new entries at the END; when one is answered, move it to answered/drawing-fonts.md. Rules: README.md. -->
<!-- from: core-fonts-final4 (2026-10-22) -->
- **FN15 - §7 name map table:** the 479 {u16 code, PostScript glyph name} pairs at `0x00B150F0` are named only by example (0x20 "space", 0x21 "exclam", 0xB7 "periodcentered", 0x3A5 "Upsilon"). Please list the whole table (code → name, first-of-duplicates already applied) so the core can ship it. [typed provider `FontLibrary::ps_glyph_name`, default none: the lookup goes straight to "uniXXXX"]
<!-- from: core-fonts-final4 (2026-10-22) -->
- **FN16 - §7 advance precision:** ftol(adv · s · 2^-16) runs on the x87; which precision control is in force at that point in the game (53-bit MSVC default, or 24-bit as left by Direct3D)? With 24-bit the product adv · s rounds to a float before the truncation. [53-bit: computed in f64, exact for |adv| < 2^29]
