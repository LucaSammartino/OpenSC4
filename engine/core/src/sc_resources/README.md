# SC resources

Portable owners from `docs/specs/sc-resources.md`. No plugin or C ABI wiring.
Reuse `persistence_runtime::{SerialRecord, Resource, Buffer}`, `rz_streams`
typed output and `effects_parser::script` switch/position readers. Foreign
traits supply files/watchers, buffer primitives and ordinary pixel conversion,
and graph surveying/serializable callbacks. Parsing, dispatch, wire formats,
image decoding, 4444 diffusion and graph output ordering stay in this module.

## Owner closure

Tests are in `engine/conformance/tests/sc_resources.rs`. SCR-V01–V68 all have
named executable tests. The JSON `sc_resources_lexical.json` also runs through
the standard runner. SCR1–SCR5 and SCR7 are answered; no section remains open.
`builtin_float_width_and_byte_scanner` additionally covers §3.3's binary32
promotion before formatting and §6.2's byte-oriented numeric prefix scans.

| Owner / spec section | Code item | Covering tests |
|---|---|---|
| Shared contracts §1 | SerialRecord, Resource, Buffer; re-exported switch readers | persistence_runtime, rz_streams, effects_parser suites and owner scenarios below |
| nSCRes §2.1 | lexical::{next_token, split, count_arguments}; find_switch, find_positional | scr_v01, scr_v02, scr_v31_nested_and_adjacent_tokens, scr_v32_comment_precedes_quotes; SW1–SW8 |
| nSCRes §2.2 | Expression::{sum, product, power, term, factor}, english_decimal, Extended, parse_float/int/uint/bool, EXPRESSION_PRECISION | scr_v03, scr_v04, scr_v33–scr_v37; decimal_extended_intermediates |
| nSCRes §2.3 | parse_enum, color, color_alpha, vector2, vector3 | scr_v22–scr_v24 |
| cCommandParser §3.1 | CommandParser lifecycle, version, register, register_commands, reset, clear_active_macro, no_op, get/remove_variable, namespace stack | scr_v05, scr_v06, scr_v38_parser_lifecycle, scr_v39_namespace_lookup, scr_v40_exact_variable_removal |
| cCommandParser §3.2 | dispatch, substitute_variables, substitute, Macro, push_block_with_end, Command::end_block | scr_v06, scr_v07, scr_v41_embedded_substitutions, scr_v42_macro_embedded_parameters, macro_substitution_and_nested_switch_validation |
| cCommandParser §3.3 | builtin, decimal_atol, transformed_value | scr_v43–scr_v48 (set variants, version, trace, list, end, namespace, eval, arrayCreate) |
| cFileParser §4.1 | FileParser lifecycle, set_parser, getters, retained Watcher handles | scr_v08, file_required_optional_order_and_hooks |
| cFileParser §4.2 | parse_all, parse_range, message, recursive parse_file and contextual unwinding | scr_v25, scr_v49–scr_v52, file_required_optional_order_and_hooks, include_suppresses_only_open_error |
| cSLTokenRecord §5.1 | TokenRecord lifecycle/QI, attachment, getters/setters | scr_v09, scr_v10, token_read_failure_cleans_attachment_and_fields |
| cSLTokenRecord §5.2 | Descriptor, END, Values wire map, create_field, field, read/write_tokens, BinaryIo | scr_v11, scr_v12, scr_v27, scr_v53_binary_all_wire_types, scr_v54_binary_strict_boundary, scr_v55_binary_string_equality, scr_v56_token_name_is_not_payload |
| cSLTokenSerialRecordIO_Text §6.1 | TextIo, TextNumber, Values; binary uint64 read | text_comments_narrowing_and_all_value_writers, scr_v59, scr_v60 |
| cSLTokenSerialRecordIO_Text §6.2 | character, delimited, read/write_definition, read/write_number/key/string/values, legacy_float | scr_v13–scr_v15, scr_v26, scr_v57–scr_v62 |
| cGZPersistBufferResource §7.1 | PersistBufferResource lifecycle/interfaces/buffer/set_buffer, ImageResourceBase, BufferProvider/Aggregator | scr_v16, scr_v63_image_key_type, aggregated_image_identity_and_destruction |
| cGZPersistBufferResource §7.2 | load_record/file_bmp/file_tga, bmp_translation, bmp_stride, rle8, decode_tga, independent create/lock flags | scr_v17–scr_v19, scr_v64_record_uses_gap_as_pixels, scr_v65_image_failure_state, scr_v68_dither_selection, bmp_file_palette_raw_and_rle8_escapes, bmp_four_bit_nibbles_and_dither_boundary, tga_rle_alpha_and_unsupported_depth |
| cGZPersistBufferResource §7.3 | diffuse_4444, store_4444 | scr_v66_bmp_diffusion, scr_v67_argb_diffusion, scr_v68_dither_selection, diffusion_serpentine_rows |
| cGZPersistASCIIWriter §8.1 | AsciiWriter reference lifecycle/stack, output_stream, SurveyorFactory | scr_v21, ascii_references_identity_and_invalid_context |
| cGZPersistASCIIWriter §8.2 | scalar, array, string, object, reference, unknown_reference, type_info, write_root/graph, hex, key | scr_v20, scr_v21, scr_v28–scr_v30, ascii_all_scalar_arrays_strings_and_inline_object, ascii_references_identity_and_invalid_context |
| All owners §9 | Independent conformance requirements | All 68 named scr_vNN tests plus ownership, error and cross-method scenarios |

## Compatibility boundaries

Text output uses SetFieldString, including each fragment's length prefix.
Delimiter input reads raw bytes and does not unwrap those fragments. Binary
fixed widths require counter < size; strings permit equality after the serial
read. Shutdown is void and drops any write failure. The text uint64 read
delegates binary IO even though its writer emits decimal text.

Expression precision is modeled as shared process state; a future native
adapter must mirror it to the floating-point environment. The English decimal
helper preserves wrapped exponent powers and 64-bit fractional intermediates.
Empty-stack namespace pop, invalid ASCII contexts and malformed RLE do not
acquire an invented native success contract. ASCII invalid contexts are safely
rejected; malformed images are bounded before indexing. BMP loading changes
the retained buffer after creation, with no rollback on a later failure.

Foreign-created objects carry creation references. Borrowed buffers are
retained before replacement, queried interfaces are acquired, and destruction
detaches the aggregator before releasing it and its buffer. File watcher
handles retain their own cleanup capability. Graph identity is independent of
display names and keys.
