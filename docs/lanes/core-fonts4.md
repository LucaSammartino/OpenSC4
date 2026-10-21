| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts4: clean-room cGZFont / cGZFontSys core from docs/specs/drawing-fonts.md | `core-fonts4` / (engine lane) | ready: commits e127cba9, 38bfefea, f610db16 on main 2b91de8c; gate cargo 3994 passed (baseline 3980, +14 fonts tests) / clippy 0 / ctest 65/65; no ABI change; no game run (core only, no glue allowed); questions FN7-FN14 filed | 1. Regenerate QUESTIONS.md (tools/questions.py index) for FN7-FN14. 2. Apply answers to FN7-FN14. 3. A later glue lane may wire engine/core/src/fonts to the plugin and shadow it in game. |

## Report

What changed (Rust core only; no plugin, CAPI, FC4Subsystems, CMake or tools edits; `pub mod fonts;` appended to engine/core/src/lib.rs):

- `engine/core/src/fonts/mod.rs`: external-library boundary traits `FontLibrary` (file exists/open/read head/system fonts dir/list files/default renderer), `FontFace` (FreeType contract: family, style, fixed flag, char index, set char size → `SizeMetrics` A/B/C, load glyph at 2/256 levels), `CodePageConverter`, `FontRenderer` (the cGZDrawFontRenderer side, interface only). No values come from the original methods.
- `font.rs` (`cGZFont`): §2 state, §3 Init/Shutdown/Release (refcounted `FontObj`, Mutex not RefCell), §4 setters and clamps, QI, §5 SetSize and metrics (worked example tested), §7 GetCharData/AddCharData/CopyGlyphToSourceData (MSB-first unpack, aa mapping, underline/strike rows, cell widening), §9 SetRenderer, nested surface locks, palette MRU and builder, DrawTextLine record emission.
- `layout.rs`: §8 CharWidth, XOffset, AdvanceWidth, StringWidth, VisibleWidth, CharsToFit, WordsToFit (modes 1/2), NumberOfLines, WidthOfLines (binary search), both CalculateTextArea forms, TextArea, FormatForEllipsis (worked example), DrawTextParagraph.
- `encoding.rs` (§6 conversion and probe table), `thai.rs` (§11 forms, classes, carry rules; all four worked examples), `aa.rs` (§10 table), `cache.rs` (§7 CachedBitmapData, ReallocateSourceData).
- `fontsys.rs` (`cGZFontSys`, all 25 slots of §12.1): Init names, source-type sniff, AddFont(path/font), AddAllFontsFromDirectory, both FontAcquire forms with scoring and earlier-path ties, FontAcquireByKind, FontRelease, mono ranges (insert-unique, pre-Init empty-name lookup), RemoveFontsByFamily, GetFontInfoList (0x88-byte records), GetCachedFonts, substitutions (slots 20-23 worked example), Shutdown, QI.
- `director.rs`: cFontCOMDirector class table and OnStart identities.

Owners, precise: implemented in core per spec: `cGZFont` (64 slots; DrawTextLine/Paragraph only up to the renderer call), `cGZFont::CachedBitmapData`, `cGZFontSys` (25 slots), `cGZFontSys::FontFixedInfo`, `cFontCOMDirector` (identities only: no COM registration glue). Remainder: the literal choices behind FN7-FN14 (advance with advanceScale ≠ 1, failed-load box, underline offset/thickness, fallback map, font source type origin, constructor defaults, palette pixel format, word/cluster/whitespace rules, cache recording without entry, empty list fields); no in-game shadow. `cGZDrawFontRenderer` not claimed.

Questions: FN7-FN14 appended to docs/specs/questions/drawing-fonts.md (QUESTIONS.md not regenerated: tools/ is off limits for this lane).
