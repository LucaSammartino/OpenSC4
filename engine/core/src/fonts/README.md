# Portable fonts

Behavior source: [drawing-fonts.md](../../../../docs/specs/drawing-fonts.md).
`GzFont` implements the font owner; `CachedBitmapData` holds its rasters and
`FontObj` supplies reference accounting and mutex-protected access. `FontObj::new`
returns a creator-owned reference (the native constructor starts at zero before
the factory acquires it). QueryInterface is represented by a boolean and an
acquired reference, rather than a raw output pointer. A false result does not
acquire anything. Rust ownership frees storage; explicit Release also clears
state for surviving safe handles. Such handles must not be used as live native
objects after final release.

`FontLibrary`, `FontFace`, `CodePageConverter`, and `FontRenderer` remain foreign
traits. No FreeType, platform code-page implementation, default renderer pixel
implementation, plugin wiring, or C ABI is provided here. Hosts supply the default
renderer and screen format through `FontLibrary`. Tests use deterministic foreign
services rather than installed fonts or game data. Surface handles are opaque.

## Owner closure

Each row names the owner's implementation and executable conformance tests.
Section 4.1 is the complete 64-slot inventory; the rows below group its slots by
the spec's behavior sections. Sections 1, 12 and 13 are included to make scope
explicit. cGZFontSys and cFontCOMDirector are separate existing owners and are not
claimed by this audit.

| Owner | Spec section | Code item | Conformance test / remaining open behavior |
|---|---|---|---|
| cGZFont | 1: identity | `font::IID_GZ_FONT`, `GzFont::answers_interface`, `FontObj::query_interface` | `renderer_slots_and_shutdown`, `borrowed_renderer_and_final_release_of_failed_initialisation` |
| cGZFont | 2: object state | `GzFont::new`, `FontObj::new` | `fn11_constructor_defaults`, `init_guards_and_constructor_size_face_attributes` |
| cGZFont | 3: lifetime | `GzFont::{init,shutdown,drop}`, `FontObj::{add_ref,release}` | `init_guards_and_constructor_size_face_attributes`, `destructor_shuts_down_before_releasing_foreign_renderer`, `borrowed_renderer_and_final_release_of_failed_initialisation` |
| cGZFont | 4: attributes and 4.1: slots 0–34, 57 | `GzFont` attribute accessors, `FontObj` reference/interface operations | `setter_clamps`, `slot_getters_round_trip`, `char_available_and_preload`, `attribute_cache_invalidation_and_size_gate` |
| cGZFont | 5: size, metrics, content probe | `font::probe_source_type`, `GzFont::{set_size,get_line_height,char_height,size_above_baseline,size_below_baseline}` | `size_metrics_worked_example`, `fn10_content_probe`, `unsigned_size_adjustment_and_zero_line_height_fault`, `attribute_cache_invalidation_and_size_gate` |
| cGZFont | 6: encoding and availability | `encoding::{convert_char_to_unicode,text_as_unicode,code_page_probes}`, `GzFont::{is_code_page_available,is_char_available,preload_chars}` | `encodings_and_probes`, `char_available_and_preload`; open: FN21 (incomplete input) |
| cGZFont | 7: glyph loading, advances and rasters | `GzFont::{appropriate_cache,get_char_data,add_char_data,name_map_index,copy_glyph_to_source_data}`, `advance::{scaled_advance,X87Control}`, `font::line_thickness` | `glyph_cache_and_widths`, `fn7_advance_and_missing_glyph`, `fn8_line_thickness`, `glyph_unpack_remap_and_decorations`, `fn15_all_477_names_and_every_absent_code`, `fn15_full_lookup_success_cache_failure_retry_and_discarded_aliases`, all four `fn16_*` tests; open: FN18 (unpublished integer conversion domain) |
| cGZFont::CachedBitmapData | 7: allocation and clearing | `CachedBitmapData::{reallocate,clear,offset}` | `cache_growth_preserves_pixels_padding_and_records`, `glyph_unpack_remap_and_decorations` |
| cGZFont | 8: measurements, slots 36, 40, 42, 47–49, 58 | `layout` methods `char_width`, `get_x_offset`, `get_advance_width`, `chars_to_fit`, `text_area`, `get_string_width`, `get_string_visible_width` | `glyph_cache_and_widths`, `fn13_words_and_lines`, `text_area_rules` |
| cGZFont | 8.1: words and line counting, slots 41, 43 | `GzFont::{words_to_fit,number_of_lines}` | `words_to_fit_modes`, `fn13_words_and_lines`; open: FN22 (backing storage and progress preconditions) |
| cGZFont | 8.2: text area, slots 44–46 | `GzFont::{width_of_lines,calculate_text_area,calculate_text_area_box}` | `text_area_rules`, `layout_uses_characters_for_newline_space_and_paragraph_trimming`, `unsigned_size_adjustment_and_zero_line_height_fault`; open: FN22 (dependent line counting) |
| cGZFont | 8.3: ellipsis, slot 50 | `GzFont::format_for_ellipsis` | `ellipsis_worked_example`, `layout_uses_characters_for_newline_space_and_paragraph_trimming` |
| cGZFont | 8.4: words, clusters, whitespace | `words::{get_word,thai_cluster_len,composed_len,is_space}`, `GzFont::get_advance_width` | `fn13_words_and_lines`, `layout_uses_characters_for_newline_space_and_paragraph_trimming`; open: FN22 (lookahead outside bounded text) |
| cGZFont | 9: line drawing, renderer, locking, palette, slots 51, 53–56 | `GzFont::{draw_text_line,get_renderer,set_renderer,lock_surface,unlock_surface,set_current_palette}`, `Palette::build` | `draw_text_line_slot51_and_paragraph_slot52`, `draw_preloads_before_records_and_strict_clip_still_advances_pen`, `renderer_slots_and_shutdown`, `fn17_failed_callbacks_modes_wrapping_and_null_edges`, `palette_rule`, `palette_mru_keeps_four_and_matches_all_colour_bits` |
| cGZFont | 9: paragraph drawing, slot 52 | `GzFont::draw_text_paragraph` (published placement and CR/LF trimming) | `draw_text_line_slot51_and_paragraph_slot52`, `layout_uses_characters_for_newline_space_and_paragraph_trimming`; open: FN19 (pen and return contract), FN22 (dependent line counting) |
| cGZFont | 10: AA lookup table | `aa::{build_table,sharpness_exponent}`, `GzFont::rebuild_aa_table` | `aa_table_rules`, `attribute_cache_invalidation_and_size_gate`; open: FN20 (native floating evaluation beyond the published formula) |
| cGZFont | 11: Thai presentation forms | `thai::{substitution_char,make_substituted_string,thai_char_type}`, `encoding::Converted` source map | `thai_worked_examples`, `fn13_words_and_lines`; open: FN22 (map-back progress domain) |
| cGZFont | 12: font catalogue | No additional cGZFont behavior; catalogue belongs to cGZFontSys. Source-type behavior assigned to cGZFont is covered under sections 5 and 7. | Not an additional owner claim |
| cGZFont | 13: open questions | [FN18–FN22](../../../../docs/specs/questions/drawing-fonts.md) | Unsupported exact-native guarantees listed below |

## Conformance and open domains

The spec has no separately headed conformance-test section. Its worked examples,
normative FN15 name table, FN16 discriminating arithmetic vectors, FN17 callback
vectors, FN12 palette vectors and FN13 class/cluster rules are exercised by
`tests.rs`, `contract_tests.rs` and `owner_tests.rs`. Run them with:

```
cargo test --manifest-path engine/Cargo.toml -p fc4-core fonts::
```

Exact native equivalence remains unsupported only for the existing questions
referenced by spec section 13. These are not claimed closed by the table:

- FN18: nonfinite/out-of-i64 advance conversion and unmasked x87 exceptions.
  The arithmetic helper returns typed errors; legacy glyph entry points panic.
- FN19: paragraph pen and return semantics. The published placement is implemented;
  the existing API has no paragraph pen and provisionally propagates line failure.
- FN20: ambient AA floating precision and foreign math evaluation. The published
  float-temporary formula is implemented literally, without a native bit-exact claim.
- FN21: malformed/truncated encoded input. The existing bounded adapter zero-extends
  a lone UTF-16 byte and limits multibyte consumption to available input.
- FN22: unavailable Thai lookahead or missing width/progress preconditions. Bounded
  walks and zero-progress termination are portable adapter behavior; native fault
  or nontermination equivalence is unsupported.

Explicit null-renderer callback edges and zero-line-height division faults that
the spec does settle are represented by panics, with tests. No new spec choices
are introduced by this closure audit. Full closure awaits answers to FN18–FN22.
