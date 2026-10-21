| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fontowners4: complete cGZFontSys / cGZFont owner audit in drawing-fonts.md | `re-fontowners4` / (RE spec lane) | ready: all 25 cGZFontSys slots (section 12.1) and all 64 cGZFont slots (section 4.1) inventoried with rules; remainder empty for cFontCOMDirector, cGZFont, cGZFont::CachedBitmapData, cGZFontSys, cGZFontSys::FontFixedInfo (75941238, 05890681); no questions; REA not used | 1. Coordinator: map the cGZFont*, cGZFontSys* and cFontCOMDirector rows of tools/coverage-map.tsv to drawing-fonts.md. 2. cGZDrawFontRenderer (vtable 0x00AC14BC) stays uncredited, separate lane. |

## Report

Evidence: `tools/re_query.py` (asm, callees), `decomp/cGZFont.c`, `decomp/cGZFontSys.c`,
`decomp/_unnamed/region_00890000.c`, and the vtables read from bin/SimCity4.exe with `build-runs/vt.py` /
`build-runs/inv.py` (pefile; inv.py prints each slot's address, export name, size, and whether the spec cites it).
REA was not queried (static export sufficed).

Changes in docs/specs/drawing-fonts.md:
- Section 1: cGZFont and cGZFontSys marked complete; precise coverage claim with empty remainder for the five owners;
  cGZDrawFontRenderer explicitly out of scope and uncredited.
- Section 4.1 (new): all 64 cGZFont slots with address, name, family (getters / setters / metrics / drawing / cache)
  and the rule or the section that holds it. New rules: QueryInterface (0x10673960 or 1), AddRef, GetName,
  shared getters for slots 7, 9, 10, 12, 13, 53; IsCharAvailable (slot 27); PreloadChars (slot 28); CharHeight
  ignores 2 stack args; deleting destructor (slot 57).
- Section 8 table: TextArea is TextArea(s, n, rect) (3 args, RET 0xC), the old one-argument form was wrong.
- Section 8.2: per-variant table for CalculateTextArea slot 46 (empty text also sets bottom = top + lh).
- Section 8.3 corrected: FormatForEllipsis returns the original length; when the ellipsis is not narrower than
  the rect, the text becomes empty (the old text said "truncate to what fits").
- Section 12: slot numbers fixed (SetRenderer is slot 6, SetMonochromeSizeRange slot 8; AddFont(path) slot 14).
- Section 12.1 (new): cGZFontSys state offsets and all 25 slots: QueryInterface (1, 0x00673960 -> this;
  0x287FB697 -> this+4), Release, Shutdown (what it clears and keeps), FontAcquireByKind (slot 9, sets
  SetFontColor(colour, 0x08000000)), FontRelease (slot 12), AddFont(font) (slot 13), RemoveFontsByFamily (slot 17,
  always false, family set not updated), GetFontInfoList (0x88-byte records), GetCachedFonts (no AddRef),
  AddSubstitution/GetSubstitutions/IsFamilyAvailable/GetAvailableSubstitution (keys case-sensitive byte order,
  0x004B5CA0; GetSubstitutions loops to max, past the vector: conforming rule given), deleting destructor; a
  worked example for slots 20-23.

Questions: none filed. ABI rows: none. Engine/tools untouched.
