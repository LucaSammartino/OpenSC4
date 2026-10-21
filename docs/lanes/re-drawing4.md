| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-drawing4: 2D drawing and fonts spec, font family (cGZFont, cGZFontSys, cFontCOMDirector) | `re-drawing4` / (RE spec lane) | ready: spec docs/specs/drawing-fonts.md (0ccaad58), 13 sections; exact owners cFontCOMDirector (220 B) and cGZFont::CachedBitmapData (56 B); cGZFont, cGZFontSys, FontFixedInfo partial; cGZDrawFontRenderer identity only; questions FN1-FN6; REA unavailable (no app open), export-first evidence | 1. Answer FN1-FN6 from instructions (REA when available) to credit cGZFont/cGZFontSys. 2. Specify cGZDrawFontRenderer (vtable 0x00AC14BC). 3. Span renderers remain unspecified. |

## Report

**What changed.** New `docs/specs/drawing-fonts.md`: font object identity (class 0x1AA404AC, IID 0x10673960,
0x15C bytes, vtable 0x00AC55D8 64 slots), state table, lifetime (Init/Shutdown/Release), setter clamps, size and
vertical metrics with a worked example, encodings, glyph cache and raster rules (mono unpack, aa remap,
underline/strike), measuring and layout, DrawTextLine/DrawTextParagraph record contract, palette build (128
entries, MRU of 4), antialiasing table formula (constants read from the exe), Thai substitution table, font
catalogue (source-type sniffing, default names, AddFont, FontAcquire by path). New
`docs/specs/questions/drawing-fonts.md` FN1-FN6, QUESTIONS.md regenerated.

**Owner bytes (tools/coverage.py --owners on main):** cGZFont 16187, cGZFontSys 8358, cGZDrawFontRenderer 4444,
cFontCOMDirector 220, FontFixedInfo 105, CachedBitmapData 56. Exact: cFontCOMDirector, CachedBitmapData (276 B).
Partial, uncredited: cGZFont (FN1, FN2, FN5, FN6), cGZFontSys and FontFixedInfo (FN3, FN4), cGZDrawFontRenderer.

**Map proposal:** `tools/coverage-map.tsv` rows `cGZFont*` and `cFontCOMDirector` may name spec
`drawing-fonts.md`; leave the other "2D drawing and fonts" rows without a spec.

**Evidence limits.** REA returned target_unavailable (no app open); used re_query.py, decomp/ and
build-runs/peconst.py (reads exe constants). The span renderers (5 classes, ~203 KB, 840 kernel slots, no decomp/)
were judged not specifiable exactly in one round. No ABI rows proposed.
