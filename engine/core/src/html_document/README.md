# cHTMLDocument

Clean-room core implementation of [html-document.md](../../../../docs/specs/html-document.md).
This extends the existing byte-oriented parser and arena; it does not introduce another parser.
Element constructors are represented by typed `Kind` payloads and recorded setter arguments.
The element owners perform layout, painting, colour/length interpretation and entity scanning.
Fonts, drawing surfaces, window/downloader/image-loader interfaces and file operations remain
foreign `HtmlHost` services. There are no C ABI or plugin changes.

`HtmlDocument` has exclusive mutable access while parsing or processing messages. An adapter must
serialize these operations, including download-table access; host completion callbacks are queued,
not reentrant calls into a borrowed document. The type uses no `RefCell`. Immutable arena getters
may be shared subject to Rust's normal borrowing rules.

Init's interface retention and QueryInterface's primary AddRef are explicit host calls.
`query_interface` is the pure IID/base classifier; `query_interface_into` performs the actual
query, preserves the output on failure, and retains on success. Primary/secondary machine pointer
adjustment and COM storage lifetime belong to a foreign adapter. Rust `Drop` implements the
destructor body and automatic member destruction; raw deleting-destructor allocation flags do
not apply to a Rust value. Shutdown remains idempotent and allows reinitialization.

Malformed FONT SIZE values use the specified signed comparisons and 32-bit address wrapping.
`font_size_word` reads modeled scalar fields and the size table directly. Other in-object dwords
(foreign pointer identities and string/container storage) are supplied explicitly by
`HtmlHost::document_layout_word`; no native pointers or unspecified Rust layout are read.
Adapters must supply those compatibility words, rather than treating a pointer as a font size.
Out-of-object reads provisionally return zero pending **HD7**.

## Owner closure

All rows have owner **cHTMLDocument**. Test names refer to `tests.rs`. The spec has no separate
conformance-test section: its worked examples and boundary cases are the conformance cases below.
Open rows have a literal implementation and tests, but cannot be claimed exact until the question
is settled. No specified parser path is replaced by an Unsupported result.

| Spec section | Code item | Tests / open question |
|---|---|---|
| §1 object layout | `HtmlDocument`, `Style`, `font_size_word` | `constructor_and_font_table`, `font_size_wrapping_and_layout` |
| §1.1 constructor, one-time tables/colours | `new`, `DEFAULT_COLORS`, `TAGS`, `sets` | `constructor_and_font_table`, `lifetime_interfaces_and_shutdown`, `tag_table_and_sets` |
| §1.2 Init | `init`, `HtmlHost::retain_interfaces` | `constructor_and_font_table`, `lifetime_interfaces_and_shutdown` |
| §1.3 base URL | `set_base_url`, `HtmlHost::window_set_base_url` | `head_elements`, `lifetime_interfaces_and_shutdown` |
| §1.4 size table setter | `set_font_size_table` | `constructor_and_font_table` |
| §1.5 COM surface and secondary thunk | `query_interface`, `query_interface_into`, `Base`, `HtmlHost::retain_document` | `lifetime_interfaces_and_shutdown` (both bases, misses, retain count) |
| §1.6 teardown and slots 4,6,13,15 | `Drop`, `shutdown`, `source_text`, `get_font_size_table`, `find_named_anchor`, `set_slot15_byte` | `lifetime_interfaces_and_shutdown`, `inline_elements`, `download_failures_retries_and_cached_delivery` |
| §2.1 shared character helpers | `HtmlHost::is_space`, `HtmlHost::advance`, `find_character` | `foreign_character_boundaries` |
| §2.2 token IDs | `tok`, `get_token` | `tokens_and_attributes` |
| §2.3 tag table | `TAGS`, `tag_id` | `tag_table_and_sets`, `tokens_and_attributes` |
| §2.4 CDATA | `parse_cdata` | `tokens_and_attributes`, `foreign_character_boundaries` |
| §2.5 tokens, skipping, push-back | `get_token`, `skip_until_token`, `skip_document_whitespace` | `tokens_and_attributes`, `repeated_parse_preserves_only_specified_state`, `select_and_script_return_values` |
| §2.6 split/strip | `split_tag`, `strip_white_space` | `tokens_and_attributes`, `foreign_character_boundaries` |
| §3 attribute parsing, lookup, no-op dump | `parse_attribute_list`, `find_attribute`, `dump_unhandled_attributes` | `tokens_and_attributes`, `foreign_character_boundaries` |
| §4.1 grammar sets | `sets` | `tag_table_and_sets`, `dispatcher_and_style_conformance` |
| §4.2 flow/inline dispatch | `parse_flow`, `parse_inline` | `dispatcher_and_style_conformance`, `inline_elements`, `blocks` |
| §5.1 SGML entry and two cursor resets | `parse_sgml_elements` | `entry_points`, `repeated_parse_preserves_only_specified_state` |
| §5.2 HTML entry | `parse_html_document` | `entry_points`, `token_and_entry_boundaries` |
| §5.3 doctype | `parse_sgml_document_type` | `entry_points` |
| §5.4 plain text, preformatted flag | `parse_text_document`, `normalize_segment_text` | `entry_points`, `repeated_parse_preserves_only_specified_state` |
| §5.5 HTML tree creation | `parse_html_elements` | `entry_points`, `repeated_parse_preserves_only_specified_state`; BODY termination dependency open: **HD8** |
| §6 head | `parse_head` | `head_elements`, `token_and_entry_boundaries` |
| §7.1 title/base/script | `parse_title`, `parse_base`, `parse_script` | `head_elements`, `select_and_script_return_values` |
| §7.2 form/special dispatch | `parse_formctrl`, `parse_special` | `forms_select_and_misnesting`, `dispatcher_and_style_conformance` |
| §7.3 BR, INPUT, SELECT, OPTION | `parse_br`, `parse_input`, `parse_select`, `parse_option` | `forms_select_and_misnesting`, `select_and_script_return_values` |
| §7.4 DIV | `parse_div`, `parse_div_actual` | `blocks`, `dispatcher_and_style_conformance` |
| §7.5 block dispatch | `parse_block` | `blocks`, `dispatcher_and_style_conformance` |
| §7.6 FORM | `parse_form` | `forms_select_and_misnesting` |
| §7.7 BODY | `parse_body`, `shared_color` | `body_attributes`, `images_maps_and_downloads`, `token_and_entry_boundaries`; open: **HD8** (loop bullets contradict worked example) |
| §7.8 tables and row groups | `parse_table`, `parse_row_group`, `parse_tr`, `parse_td`, `parse_th` | `tables`, `table_attribute_and_order_conformance` |
| §7.9 list items and bullets | `parse_li`, `create_bullet`, `BULLETS`, `HtmlHost::create_bullet_surface` | `lists_and_bullets`, `bullet_cache_conformance` |
| §7.10 BLOCKQUOTE, HR | `parse_blockquote`, `parse_hr` | `blocks`, `numeric_attribute_conformance` |
| §7.11 font/phrase/span/blink/anchor, numeric conversion | `parse_fontstyle`, `font_attributes`, `parse_phrase`, `parse_span`, `apply_span_style`, `parse_blink`, `parse_a`, `convert_to_u32`, `font_size_word` | `inline_elements`, `dispatcher_and_style_conformance`, `font_size_wrapping_and_layout`, `numeric_attribute_conformance`; open: **HD7** (out-of-object reads only) |
| §7.12 images, maps, areas | `parse_img`, `parse_map`, `parse_area` | `images_maps_and_downloads`, `image_map_and_anchor_boundaries` |
| §7.13 META and LINK | `parse_meta`, `parse_link` | `head_elements`, `token_and_entry_boundaries` |
| §7.14 P/PRE/DL/DT/DD/headings/lists/CENTER/image request | `parse_p`, `parse_pre`, `parse_dl`, `parse_dt_dd`, `parse_heading`, `parse_list`, `parse_div_actual`, `request_image` | `blocks`, `lists_and_bullets`, `repeated_parse_preserves_only_specified_state`, `download_failures_retries_and_cached_delivery` |
| §8.1 alignment | `make_output_style`, `make_relative_to_text_alignment_style` | `alignment_keywords`, `tables`, `blocks` |
| §8.2 text insertion and style stack | `insert_text_segment`, `push_style`, `pop_style` | `inline_elements`, `style_stack_conformance` |
| §9 downloads/images | `do_message`, `load_from_image_file`, `resource_has_been_downloaded` | `images_maps_and_downloads`, `download_failures_retries_and_cached_delivery`, `lifetime_interfaces_and_shutdown` |
| §10 inventory | existing module and rows above | all `html_document::tests` |
| §10.1 entities | `entities::entity_code`, `entities::get_string_for_escape` | `entity_table` |
| §10.2 attribute-map member helpers | `attrs.clear`, Rust member destruction | `tokens_and_attributes`, `token_and_entry_boundaries` |
| §10.3 unnamed owner methods, aliases and foreign constructors | methods mapped in §§1.6,7.8,7.14,8.2; typed `Kind` payloads; shared stubs excluded by spec | tests in corresponding rows |

The two open cases are recorded in `docs/specs/questions/html-document.md`.
The owner table deliberately does not claim real-game shadow verification: this module is not wired
into the plugin, and no ABI exports are added.
