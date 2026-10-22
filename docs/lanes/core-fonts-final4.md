| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-fonts-final4: apply FN7-FN14 answers to engine/core/src/fonts | `core-fonts-final4` / (engine lane) | ready: commits 0cb60e6c, 93032dfb (FN7-FN14), 50deb37d, e2a6a26f (slot audit: getters, FN17, 6 tests); gate cargo 4111 passed / clippy 0 / ctest 65/65; no ABI change; no game run (core only); 0 missing slot bodies (64 + 25); questions FN15, FN16, FN17 open | 1. Answer FN15 (glyph-name table), FN16 (x87 precision), FN17 (lock slot return paths). 2. A glue lane wires the core to the plugin and shadows it in game. |

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

## Slot audit (round 2, commits 50deb37d + fmt fix)

All 64 cGZFont and 25 cGZFontSys slots checked against §4.1 / §12.1. Every slot has a Rust body. Typed external dependencies, abbreviated: Lib = `FontLibrary`, Face = `FontFace`, Conv = `CodePageConverter`, Rend = `FontRenderer`. "T" means a test in fonts/tests.rs checks the slot's defined behaviour. "T*" means the test was added in round 2.

cGZFont (`GzFont`, refcount on `FontObj`):

| Slot | Rust | Dep | Status |
|---|---|---|---|
| 0 QI / 1 AddRef / 2 Release | FontObj::query_interface, add_ref, release (FinalRelease → shutdown) | - | T, T* (release to 0 shuts down) |
| 3 Init / 4 Shutdown | init, shutdown | Lib file_exists/read_head/open/default_renderer; Face family_name/style_flags/is_fixed_size/char_index | T / T* |
| 5,7,9,10,12,13 | get_name, get_styles, get_face_kind, get_size, get_font_color, get_font_background_color (new) | - | T* |
| 6 / 8 / 14 | get_style / set_style / set_font_color | - | T / T / T* |
| 11 SetSize | set_size | Face set_char_size | T |
| 15-20 | get_/set_antialiasing, get_/set_aa_gamma, get_/set_aa_sharpness (getters new) | aa::build_table | T, T* |
| 21-24 | get_/set_file_path, get_/set_code_page (getters new) | - | T, T* |
| 25 / 26 | is_code_page_available / is_char_encoding_available | Face has_glyph | T / T* |
| 27 / 28 | is_char_available / preload_chars | Conv, Face has_glyph | T* |
| 29-32 | get_/set_scaling, get_/set_advance_scaling | - | T |
| 33 / 34 | get_spacing_adjustment (new) / set_spacing_adjustment | - | T* / T |
| 35-40 | get_line_height, char_width, char_height_args (new, args ignored), size_above/below_baseline, get_advance_width | - | T, T* |
| 41-46 | words_to_fit, chars_to_fit, number_of_lines, width_of_lines, calculate_text_area_box, calculate_text_area | words.rs, thai.rs | T |
| 47 / 48 / 49 / 50 | text_area / get_string_width / get_string_visible_width / format_for_ellipsis | - | T* / T* / T / T |
| 51 / 52 | draw_text_line / draw_text_paragraph | Rend lock/draw/unlock | T* (records count, colour, pen, clip, lock edges) |
| 53 / 54 | get_renderer / set_renderer | Rend attach/detach | T* |
| 55 / 56 | lock_surface / unlock_surface | Rend lock/unlock | T* for the edges; return values and failure paths are a literal reading, see FN17 |
| 57 deleting dtor | Rust Drop of FontObj/GzFont | - | no explicit body (freeing memory has no observable behaviour) |
| 58 / 59 | get_x_offset / appropriate_cache | - | T / T* |
| 60 / 61 | get_char_data / add_char_data | Face load_glyph/glyph_name_to_code; Lib ps_glyph_name, screen_format | T; FN15 (the table data) and FN16 (x87 precision) still open |
| 62 / 63 | copy_glyph_to_source_data / CachedBitmapData::reallocate | - | T through slot 60 only (no direct test) |

cGZFontSys (`GzFontSys`):

| Slot | Rust | Status |
|---|---|---|
| 0 / 1 / 2 | query_interface (Some(true) = second interface), add_ref, release | T* (release to 0 runs shutdown, the cache drops its ref) |
| 3 / 4 | init (registers the 4 names) / shutdown | T / T* |
| 5 / 6 / 7 / 8 | set_code_page, set_renderer (+renderer()), set_default_antialiasing, set_monochrome_size_range | 5 and 6 are plain stores with no test; 7 and 8 T |
| 9 / 10 / 11 | font_acquire_by_kind / font_acquire_path / font_acquire_family | T* / T / T |
| 12 / 13 / 14 / 15 | font_release / add_font / add_font_path / add_all_fonts_from_directory (Lib list_files, system_fonts_dir) | T |
| 16 | slot16 → false | T* |
| 17-23 | remove_fonts_by_family, get_font_info_list (+FontInfoRecord::to_bytes), get_cached_fonts, add_substitution, get_substitutions, is_family_available, get_available_substitution | T |
| 24 deleting dtor | Rust Drop | no explicit body |

Owner helpers: FontObj (refcount behind a Mutex, no RefCell), probe_source_type (FN10), line_thickness (FN8), Palette::build/split_colour/compose_colour/set_current_palette (FN12), scaled_advance (FN7, FN16 open), name_map_index (FN9, FN15 open), encoding::{convert_char_to_unicode, text_as_unicode, code_page_probes}, words::* (FN13), thai::*, aa::*, fontsys::{source_type_from_file_name, parse_delimited (FN14), font_set_name}, director::class_for. All are tested.

What is left, owner by owner: cGZFont has 0 missing bodies, with 3 open questions (FN15 table data, FN16 precision, FN17 lock return paths); slots 62/63 are tested only indirectly. cGZFontSys has 0 missing bodies and 0 open questions; slots 5/6 are untested plain stores. Destructors (font 57, sys 24) map to Rust Drop. This core does not include the renderer (`cGZDrawFontRenderer`), the plugin glue or the CAPI.
