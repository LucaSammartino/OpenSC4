| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-fonts: the MXF font format and the draw font renderer, folded into drawing-fonts.md (RE spec lane) | `re-batch5` branch (outside contributor checkout, fork amarret/OpenSC4; on main `a4f059a7`, after re-signposts/re-textdecals) | **ready** (docs and RE tables only). drawing-fonts.md §14 (MXF files: header, the stream's XOR rule, the shipped files) and §15 (the glyph-cache holder labelled `cGZDrawFontRenderer`, summary) added; the MXF gap closed in ui-windows.md UIW-10, view3d.md and INVENTORY-M6; FN23 filed; 46 names (3 new, 43 renamed). Specified coverage +134 B / +2 functions | FN23 (the draw font renderer in full) |

## Report

Static RE only: the instruction bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`), the
Ghidra export, the SDK headers and the shipped `Fonts\*.mxf` files (read only). Written on 2026-10-20 as a separate
`fonts.md` and held back when `main` gained drawing-fonts.md, which is fuller on the font system and the font object;
this fold adds what it lacked.

### What changed

- drawing-fonts.md §14 (new): the MXF header (`MXFN`, big-endian L, key), the stream's rule (`0x00896D9A`,
  `0x00896DDD`: base 9, XOR below L; a file without the magic is read as it is) and the 19 shipped files (L =
  40,000,000, key `0x9D`; 16 TrueType, 3 Type 1 in PFB segments; lengths multiples of 10,000).
- §15 (new, summary): the glyph-cache holder (class `0x8B3255F9`, labelled `cGZDrawFontRenderer`): the cache and its
  rows, the colour table, the hardware path. Its identity is upstream's (re-font-renderer4, font-renderer.md §1),
  which reached the same split the held lane found: the 0x450-byte holder `0x8B3255F9` and the director's 8-byte
  renderer `0x08CDA30B` it holds; the held lane's identity row was dropped in favour of upstream's.
- ui-windows.md UIW-10, view3d.md (§ scope and V3D-9), INVENTORY-M6 §4.2: the MXF part is now specified; the UI script
  grammar stays open.
- Kept upstream: the font system, font object, antialiasing and Thai sections. The held lane's antialiasing exponent
  (`1 / cos 2`) was wrong; upstream's `1 / log10 2` stands.

### Names and coverage

- re/symbols/fonts.tsv: 46 names (3 new: `cGZDrawFontRenderer::AddGlyphToCache`, `cGZFontStream::ReadAt` and `Read`;
  43 replace auto names). The `names_windows.tsv` delta (−43 +46) is identical to `merge_names.py`'s; no non-auto
  upstream name is replaced.
- No coverage row change: the two stream functions fall under the existing `cGZFont*` row; `cGZDrawFontRenderer` stays
  uncredited.

### Measured

`tools/coverage.py` on the local (Steam) export, measurement only: 3,478,182 bytes in 15,345 functions before,
3,478,316 in 15,347 after (+134 B, +2 functions). `docs/coverage-report.md` was not regenerated (the tool accepts only
the GOG exe).

### Questions

- Filed: FN23 (the glyph-cache holder in full; its accounting is font-renderer.md FR3).
- The held lane's FN1 (where the font requests come from) is upstream's answered FN range; not refiled.

### Not covered

FN23.
