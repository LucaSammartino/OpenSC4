| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-font-core-answers4: answer drawing-fonts FN7-FN14 | `re-font-core-answers4` / (RE spec lane) | ready: FN7-FN14 all answered (all DIFFERS) in docs/specs/drawing-fonts.md §2, §5, §7, §8.1, §8.4, §9, §12 (937f09df, 371bb523, 9e01312f, FN12 412827fb, proof 52ac7052), archived; REA not used | 1. core-fonts: replace the core defaults per the DIFFERS list below (FN12: palette split/compose via screen colour type, low-16-bit cut, no alpha lerp). |

## Report

Evidence: static only — `tools/re_query.py` export (asm, callers, callees), decomp/ and `build-runs/peek.py`
(pefile constants) plus `build-runs/charsets.py` (dumps the hash-set character classes from asm + data tables).
REA was not queried.

Answers (every one DIFFERS from the core-fonts4 default):
- FN11 (§2): constructor `0x00899AAB`: pointSize = charHeight = 16, aaMode 1, +0x50 = 2, gamma 1.0, sharpness 0.0,
  scales/advanceScale 1.0, defaultChar 0x20; FreeType holder with load flags 0x000A.
- FN10 (§5): source type = holder+0x24 from a content probe at face open (`0x008A1BD8`): sfnt with DSIG → 4, sfnt
  with loca → 1, "%!" → 2, else the open fails; type 3 unreachable. Not the file name.
- FN7 (§7): advance from face+0x80 (fixed-size face) or face+0xD0; s == 1.0 → /65536; else ftol(adv·s·2^-16) with a
  −1 rule for spaces at 0.98 < s < 1 (constants 0x00AC56E0 = 0.98, 0x00AC276C = 2^-16). Missing glyph = hollow
  (h/2)×(h/2) outline at yOffset 0, advance ftol(h/2·s); U+200B/FEFF empty, advance 0.
- FN8 (§7): thickness by pointSize only: underline 1..4 at <16/<26/<36, strike 1..4 at <24/<48/<72; underline
  starts at ascent − yOffset + thickness.
- FN9 (§7): name map: holder cache, then 479-entry code → glyph-name table at 0x00B150F0, then "uniXXXX".
- FN13 (§8.1, new §8.4): UTF16GetWord line-break classes (tables listed), word = run + trailing break-after chars;
  Thai cluster ≤ 4 units, width of the first unit only; visible-width space set = 0x20, U+2002..U+200B; CR LF =
  two line ends; mode 2 takes ≥ 1 cluster; GetAdvanceWidth with no record → 0.
- FN14 (§12): FontAcquire(path, cache) with no catalogue entry creates one (family added to the set);
  RZParseDelimitedText keeps empty inner fields, drops an empty trailing one, "" → 0, last field keeps trailing
  blanks (trim bug at 0x00910C0F).

- FN12 (§9, round 2): `RZ::GS()` (iid 0x0073283C → subobject +0x1C, vtable 0x00ADAA58) +0x90/+0x8C (0x0098EA00 /
  0x0098E9B0) forward to the screen buffer (graphic system +0x100) slots +0x7C split (0x00825F80) / +0x78 compose
  (0x00825DA0); format = screen colour type: 7 (A4R4G4B4, depth <= 16) or 9 (A8R8G8B8). Per-type table in §9.
  The font cuts fg and bg to their low 16 bits before the split (MOVZX 0x00898414 / 0x00898433): red is 0 in
  entries 8..119 on a 32-bit screen. Alpha not interpolated (compose forces 0xF / 0xFF). Signed IDIV truncation.
  k = 14 block overwritten by fg (only k = 0..13 survive). No screen buffer → entries 8..119 uninitialised.
  Proof: docs/specs/proofs/font_palette_fn12.py (unicorn on the shipped bytes; 0 mismatches, vectors in §9;
  needs unicorn+pefile, installed into build-runs/pylib for the run).

All questions answered; none open. No new questions filed. No ABI rows. Static only (no REA).
