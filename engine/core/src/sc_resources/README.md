# SC resources

Portable owners from `docs/specs/sc-resources.md`. No plugin or C ABI wiring.
Reuse `persistence_runtime::{SerialRecord, Resource, Buffer}`, `rz_streams`
typed numeric output and `effects_parser::script` switch/position readers.
Foreign traits provide files/watchers, buffer construction and pixel conversion,
and graph surveying/serializable callbacks. They do not implement these owners'
lexing, dispatch, wire parsing, image decoding or output ordering.

## Owner closure

Tests are in `engine/conformance/tests/sc_resources.rs` unless stated otherwise.
The JSON vector `sc_resources_lexical.json` also runs through the standard runner.

| Owner / spec section | Code item | Tests / explicit open boundary |
|---|---|---|
| Shared contracts §1 | Existing SerialRecord, Resource, Buffer; re-exported switch readers | Existing persistence_runtime, rz_streams and effects_parser suites; new owner scenarios below |
| nSCRes §2.1 | lexical::{next_token, split, count_arguments}; effects_parser::script::{find_switch, find_positional} | scr_v01_grouped_tokens, scr_v02_unterminated_delimiters; existing SW1–SW8 tests; open: SCR1 nesting/escapes |
| nSCRes §2.2 | lexical::{parse_float, parse_int, parse_uint, parse_bool, EXPRESSION_PRECISION} | scr_v03_numeric_expression, scr_v04_boolean_spellings; JSON expression cases; open: SCR1 extended operand grammar/remainder, SCR7 minimal decimal grammar |
| nSCRes §2.3 | lexical::{parse_enum, color, color_alpha, vector2, vector3} | scr_v22_unclamped_color_alpha, scr_v23_missing_vector_coordinates, scr_v24_enum_precedence; JSON components |
| cCommandParser §3.1 | command::CommandParser lifecycle, register, register_commands, BUILTINS, no_op, unsettled_slot | scr_v05_latest_registration, scr_v06_command_failures; open: SCR2 unnamed slots/defaults and unpublished builtin argument contracts |
| cCommandParser §3.2 | CommandParser::{dispatch, substitute_variables, builtin}, macro storage | scr_v06_command_failures, scr_v07_macro_count, macro_substitution_and_nested_switch_validation; open: SCR2 variable parenthesis insertion, SCR7 namespace/parameter boundaries |
| cFileParser §4.1 | files::FileParser lifecycle, set_parser, getters; retained Watcher handles | scr_v08_file_modified, file_required_optional_order_and_hooks |
| cFileParser §4.2 | FileParser::{parse_all, parse_range, message}, parse_file | scr_v25_file_messages, file_required_optional_order_and_hooks, include_suppresses_only_open_error; open: SCR3 relative include resolution/nested recovery |
| cSLTokenRecord §5.1 | token::TokenRecord attach/init/read/write/shutdown/QI, getters/setters | scr_v09_token_attachment, scr_v10_missing_token, token_read_failure_cleans_attachment_and_fields; open: SCR4 other typed wire assignments |
| cSLTokenRecord §5.2 | Descriptor, END, create_field, field, read_tokens, write_tokens | scr_v11_sorted_write_and_terminator, scr_v12_stop_before_end_payload, scr_v27_descriptor_count_must_match, token_read_failure_cleans_attachment_and_fields; open: SCR4 other wire types/binary payloads, SCR7 token name interpretation |
| cSLTokenSerialRecordIO_Text §6.1 | token::{TextIo, TextNumber, Values} twelve typed payload kinds | text_comments_narrowing_and_all_value_writers; generic typed numeric reader retains caller width/sign |
| cSLTokenSerialRecordIO_Text §6.2 | TextIo::{character, delimited, read_definition, read_number, read_key, read_string, write_definition, write_values} | scr_v13_text_descriptor, scr_v14_numeric_prefix_and_zeroing, scr_v15_key_preservation, scr_v26_final_physical_byte_excluded, text_comments_narrowing_and_all_value_writers; open: SCR7 scanner overflow/nonfinite and length-prefix/raw-reader relationship |
| cGZPersistBufferResource §7.1 | image::PersistBufferResource lifecycle, interfaces, buffer, set_buffer; BufferProvider/Aggregator | scr_v16_self_set_buffer, aggregated_image_identity_and_destruction; open: SCR7 unassigned base type (zero key/type) |
| cGZPersistBufferResource §7.2 | load_record, load_file_bmp, load_file_tga, bmp_translation, bmp_stride, rle8, decode_tga | scr_v17_seek_failure_aborts_fallback, scr_v18_bmp_conversion_pairs, scr_v19_tga_vertical_origin, bmp_file_palette_raw_and_rle8_escapes, bmp_four_bit_nibbles_and_dither_boundary, tga_rle_alpha_and_unsupported_depth; open: SCR5 record BMP offsets and 4444 diffusion |
| cGZPersistASCIIWriter §8.1 | ascii::AsciiWriter reference lifecycle/stack, output_stream, SurveyorFactory | scr_v21_ascii_graph_order, ascii_references_identity_and_invalid_context |
| cGZPersistASCIIWriter §8.2 | scalar, array, string, object, reference, unknown_reference, type_info, write_root, write_graph, hex, key | scr_v20_hex_and_key_labels, scr_v21_ascii_graph_order, scr_v28_ascii_bool_array, scr_v29_ascii_empty_array, scr_v30_ascii_null_inline_object, ascii_all_scalar_arrays_strings_and_inline_object, ascii_references_identity_and_invalid_context |
| All owners §9 | Conformance SCR-V01–30 plus ownership/error scenarios | All thirty named scr_vNN tests; JSON cases use independent expected results |

## Compatibility boundaries

Only published wire type 3 (uint32) is assigned in TokenRecord. All twelve
typed IO operations exist independently in TextIo; no vtable-slot-to-wire-type
guess is used. Binary record initialization is represented, but binary field
encoding returns Unsupported(SCR4). Text output literally calls SetFieldString
(length prefix included), while delimiter input reads raw bytes. These are
tested independently; the spec does not establish their roundtrip relationship.
Shutdown is void and drops any write error, as required.

The expression precision flag models shared process state without unsafe CPU
instructions. An eventual native adapter must mirror the transition to its
floating-point environment. No plugin is connected by this lane. Decimal
operands implement the minimum required by the published vectors. Remainder,
mixed nesting/escapes and non-whole-token variable insertion remain explicit
Unsupported domains rather than invented grammar. Unpublished builtins are
registered in the required order and return Unsupported(SCR2).

Record BMP offset policy is explicitly open: supported record BMPs report
Unsupported(SCR5), while invalid BMP headers can fall back to TGA after a
successful absolute seek. Both published file/record stride formulas are
available in bmp_stride; file BMP decoding uses the file formula. Explicit
experimental RecordBmpOffset policies permit testing the settled record
decoder independently of the open offset decision; load_record never picks
one. Color type
7 follows the existing core's 4444 mapping and rejects decoding with SCR5,
instead of silently substituting truncation. Unsupported TGA source depth can
succeed after creation/locking, as the spec requires. Malformed RLE is safely
rejected; no claim is made about native invalid-buffer effects.

Foreign-created objects carry creation references; borrowed buffers are retained
before replacement, queried interfaces are acquired, and destruction detaches
the aggregator before releasing it and its buffer. Watcher handles retain their
own foreign cleanup capability, so FileParser destruction can clean up without
borrowing the filesystem. Graph resource identity is independent of key/name.
Invalid ASCII contexts are rejected, as expressly permitted by §8.2.
