| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts-final4: apply FN7-FN14 answers to engine/core/src/fonts | `core-fonts-final4` / (engine lane) | ready: commits 0cb60e6c, 93032dfb on main 8ef9b49f; gate cargo 4106 passed (baseline 4099, +7 FN tests) / clippy 0 / ctest 65/65; no ABI change; no game run (core only); questions FN15, FN16 filed | 1. Answer FN15 (479-entry glyph-name table) and FN16 (x87 precision). 2. Slot-by-slot audit of the 64 cGZFont / 25 cGZFontSys slots against §4.1/§12.1 (not done here). 3. A glue lane wires the core to the plugin and shadows it in game. |

## Report

Rust core only (engine/core/src/fonts); no plugin, CAPI, CMake, tools, ABI or renderer edits.

- FN11 (font.rs `GzFont::new`): pointSize = charHeight = 16, aaMode 1, +0x50 = 2, gamma 1.0, sharpness 0, scales 1.0, defaultChar 0x20; holder load flags 0x000A and an empty name map.
- FN10 (`probe_source_type`): content probe at Init (sfnt with DSIG → 4, loca/sloc → 1, "%!" → 2, else Init fails); load flags |0x80 (types 1/4) or |0x01 (2/3) are passed to every `load_glyph`.
- FN7 (`scaled_advance`, `add_char_data`): face+0x80 (`FontFace::fixed_advance`) on fixed-size faces, else the glyph advance; s == 1 integer division; 0.98 < s < 1 space shortened by 1; otherwise trunc(adv·s/65536). The missing glyph is a hollow w×w square (w = charHeight >> 1) at the line top, advance trunc(w·s); U+200B/U+FEFF get an empty raster with advance 0; a glyph-done error keeps the loaded record (insert-unique).
- FN8 (`line_thickness`): underline/strike thickness from pointSize only; underline row = ascent − yOffset + T_u; rows clipped to charHeight.
- FN9 (`name_map_index`): holder map, then the PostScript name (`FontLibrary::ps_glyph_name`, the table itself is FN15), then "uniXXXX" via `FontFace::glyph_name_to_code`; only hits are cached.
- FN12 (`Palette::build`, `split_colour`, `compose_colour`, `ScreenFormat`): the screen colour type comes from the caller (`FontLibrary::screen_format`, `ScreenFormat::from_depth`: 7 at ≤16 bpp, else 9); fg/bg cut to their low 16 bits before the split; compose forces the alpha; k = 14 is overwritten by fg; with no screen buffer, entries 8..119 take the caller-supplied `NoScreen { stale_out }` value, so the core never invents a value. All six spec vectors pass.
- FN13 (new words.rs, layout.rs): HB/SB/NS (223 codes, count checked)/NE/CJK/Thai classes, UTF16GetWord, Thai cluster length (the s[3] read past n comes from the caller's buffer), composed-character widths (first unit only), space-class visible width, §8.1 walk with the unsigned fit test, CR LF as two line ends, mode 1 takes the rest of the line, mode 2 clusters with prev rollback and the §11 map step. GetAdvanceWidth returns 0 when the font is not initialised.
- FN14 (fontsys.rs): with `cache`, a missing catalogue entry is created ({[font], family, styles, faceKind, source type 0}) and the family is added to the family set; `parse_delimited` keeps empty fields except a trailing one, trims, handles quotes, caps at 32 names.

Literal readings, each marked in the code: the s = 0.98 float band edge; the mode-2 end bound (cannot be reached with non-negative advances); a quote-strip needs at least 2 characters; f64 product (FN16).

Questions: FN15 (the 479-entry glyph-name table), FN16 (x87 precision control for the advance product), in docs/specs/questions/drawing-fonts.md.

Not done: the explicit audit of all 64 cGZFont / 25 cGZFontSys slots and their helpers against §4.1/§12.1. Only the FN7-FN14 behaviour has been checked against the spec. The open questions FN15 and FN16 remain.
