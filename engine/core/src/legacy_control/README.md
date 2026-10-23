# cGZLegacyControl core owner

Source: `docs/specs/legacy-control.md`, §§1–14. This extends the existing owner;
there is no second window implementation and no plugin or C ABI wiring.

`LegacyControl` owns reference effects, construction ordering and two-pass dispatch.
`Globals` is an explicit process context containing the shared vocabulary and the
two unsigned ordered, first-insert-wins maps. Its 21 registration tables include
every published word and alias. `Foreign` supplies individual COM, window,
resource, graphics/font, notifier and procedure-allocation operations. Setter
results never control the successful build sequence. `Value::U32` transports
binary32 alpha argument bits at the documented float-taking slots. Handles are
opaque tokens, never host pointers. The layout constants describe the foreign
32-bit layout; Rust does not promise its own native struct has that layout.

All 57 owner-function contracts have implementations for their defined domains:
14 COM/lifecycle/accessor methods, SetParameters, GetScriptFragment, 21 vocabulary
functions and 20 owned interpreters. The shared Custom stub is also dispatched.
Foreign windows retain responsibility for input dispatch, fitting/clamping,
rendering and procedure callbacks. Tests record effects; they do not establish
pixel equivalence or native-game runtime equivalence.

Open boundaries already numbered by the spec:

- LC5: forward the opaque manager slot-3 result; do not name or reinterpret it.
- LC8: fragment frame/resolution/common words are implemented; per-type writer
  output reports `FragmentCoverage::Partial` where a writer answers QI.
- LC9/LC10: reject overflowing/non-UTF8 inputs, duplicate TreeView map keys and
  indeterminate Grid scratch/resource domains. Earlier expressly granted zero
  and -3 choices remain labeled in code.
- LC11: reject indeterminate unchecked scans and TextEdit outline colour without
  numeric outline. No low-memory recovery or rollback is promised. Core values
  model successful temporary allocation; SC3 procedure allocation can return null.
- LC12: allocate/install the external default procedure through a primitive trait;
  its callbacks remain outside this owner. Caller procedure handoff follows it.
- LC13: new §9.9 named-font branches return `Unsupported::Open` rather than invent
  the helper's payload/application. Older §§9.2–9.6 explicitly specify stored
  text and retain that behavior. Numeric fonts and defaults are implemented.

## Owner closure

Each row is for cGZLegacyControl; function names refer to this module. Tests are
in `tests.rs` and `tests/conformance.rs`. The source spec's §9.4 helper appears
after §9.5 physically; the table follows section numbers.

| Spec section | Code item | Test or explicit open boundary |
|---|---|---|
| 1 Identity | `query_interface`, `Interface`, `OBJECT_SIZE` | `lcv01_03_layout_and_com` |
| 2 Object layout | `VTABLES`, `FIELD_OFFSETS`, constructor state | `lcv01_03_layout_and_com` |
| 3 Slot tables | `VTABLE_SLOT_COUNTS`, owner methods, area stubs, own-control thunk | `release_only_order_refcounts_and_accessor_channels`, `lcv01_03_layout_and_com` |
| 4 Shared vocabulary | `Globals`, `vocabulary::TABLES`, `new`, `register` | `exact_vocabulary_order_ids_aliases_and_maps`, `constructor_failure_retry_maps_survive_and_zero_release_destroys` |
| 4.1 Registered classes | all 21 `TABLES` rows and map registration | `exact_vocabulary_order_ids_aliases_and_maps` |
| 4.2 Word tables | complete `TABLES.words` | `exact_vocabulary_order_ids_aliases_and_maps` against `vocabulary.expected` |
| 5 Lifecycle | `release`, destruction/reference effects | `lifecycle_identity_self_assignment_parent_and_one_shot_procedure`, `release_only_order_refcounts_and_accessor_channels` |
| 6 Build sequence | `set_parameters`, primitive factory and dispatch | `every_published_type_traverses_the_production_builder`, `fallback_attempts_one_lowest_class_then_custom_and_retains_requested_ids`; open: LC5 (opaque default parent) |
| 7 Small methods | owner accessors, parent/procedure/control operations | `lifecycle_identity_self_assignment_parent_and_one_shot_procedure` |
| 7.1 SetParent | `set_parent` | `failed_parent_qi_and_procedure_qi_preserve_only_the_procedure` |
| 7.2 UseWinProc/ForceControl | `replace`, `use_win_proc`, `force_control` | `lifecycle_identity_self_assignment_parent_and_one_shot_procedure` |
| 7.3 DoExceptions | `do_exceptions` | `lcv25_27_generator_options_and_procedure_order` |
| 7.4 GetScriptFragment | `get_script_fragment` | `fragment_wrapper_exact_common_text_and_lc8_partial` |
| 7.5 Property readers | `Properties`, `parse::{scan,tuple,colour,key}` | `property_scan_skips_wrong_types_and_preserves_numeric_output`, `lcv28_32_33_failure_boundaries`; open: LC9 |
| 7.6 Fragment serialization | `serialize`, `FragmentCoverage` | `fragment_resolution_scans_all_21_rows_and_preserves_stored_class`; open: LC8 (non-owner type writers) |
| 8 Common pass | `interpret_common` | `common_geometry_caption_resource_and_unconditional_parameters`, `exact_two_pass_slider_common_init_parent_front_order` |
| 9 Per-type dispatch | `interpret` | `every_published_type_traverses_the_production_builder`, `lcv28_32_33_failure_boundaries` |
| 9.1 Slider | `slider` | `exact_two_pass_slider_common_init_parent_front_order`, `image_reference_and_parameter_effects_are_owned_by_core` |
| 9.2 TreeView | `tree::run` | `tree_position_retention_and_depth_first_search`, `tree_images_maps_root_defaults_and_numeric_font_precedes_string`; open: LC10 |
| 9.3 FileBrowser | `file_browser::run` | `file_browser_keeps_raw_getter_values_and_filter_uses_pair_position` |
| 9.4 Image/font metadata helpers | `image_key`, `font_word`, `controls::font` | `image_reference_and_parameter_effects_are_owned_by_core`; open: LC13 (§9.9 named-font branch only) |
| 9.5 Grid | `grid::run` | `grid_shared_width_header_and_column_only_sizing`, `grid_style_scratch_is_last_value_not_cell_flags_and_partial_scan_is_stale`; open: LC10 |
| 9.6 Folders | `folders` | `flat_rect_outline_folders_and_slider_boundary_effects` |
| 9.7 FlatRect | `flat_rect` | `flat_rect_outline_folders_and_slider_boundary_effects` |
| 9.8 Outline | `outline` | `flat_rect_outline_folders_and_slider_boundary_effects` |
| 9.9 Shared drawing inputs | `controls::{font,colours,resource}`, `feedback::unchecked` | `drawing_setters_fonts_resources_and_scans`; open: LC13 |
| 9.10 Text | `controls::run` | `lcv10_text_fit_presence_and_opacity` |
| 9.11 TextEdit | `controls::text_edit` | `lcv11_12_textedit_flags_and_failed_images`, `drawing_setters_fonts_resources_and_scans`; open: LC11 |
| 9.12 ListBox | `controls::list` | `lcv13_15_list_and_combo_order_and_exact_booleans` |
| 9.13 Combo | `controls::combo` | `lcv13_15_list_and_combo_order_and_exact_booleans` |
| 9.14 Btn | `feedback::button` | `lcv16_17_button_defaults_boolean_timing_and_failed_tip`, `drawing_setters_fonts_resources_and_scans`; open: LC11 |
| 9.15 BMP | `feedback::{bmp,alpha}` | `lcv18_19_bmp_alpha_and_common_override`; open: LC11 |
| 9.16 Scrollbar | `feedback::scrollbar` | `lcv20_22_scrollbar_and_line`, `lcv28_32_33_failure_boundaries` |
| 9.17 LineInput | `feedback::line` | `lcv20_22_scrollbar_and_line`; open: LC11 |
| 9.18 Spinner | `feedback::spinner` | `lcv23_24_spinner_and_ordered_ticker`; open: LC11 |
| 9.19 Ticker | `feedback::ticker` | `lcv23_24_spinner_and_ordered_ticker`; open: LC11 |
| 9.20 SC3/OptGrp | `options::{generator,options}` | `lcv25_27_generator_options_and_procedure_order`, `option_items_and_unsigned_generator_records`; open: LC11/LC12 |
| 10 Compatibility quirks | labeled owner and parser choices | fallback, common, Grid, TreeView, fragment and unchecked-scan tests above |
| 11 Open questions | explicit boundaries above | open: LC5, LC8–LC13 |
| 12 Owner-function inventory | 57 contracts implemented in the existing module | vocabulary fixture, all-type builder and per-section tests above |
| 13 Message/drawing/failure boundary | foreign traits, metadata, ordering, `Unsupported` | `lcv25_27_generator_options_and_procedure_order`, `lcv28_32_33_failure_boundaries`; open: LC9–LC13 |
| 14 Conformance vectors | recording factory/window/resource fixture | LCV01–LCV33 mapping below |

## Conformance vector index

| Vectors | Tests |
|---|---|
| LCV01–03 | `lcv01_03_layout_and_com`; count-zero destruction also `constructor_failure_retry_maps_survive_and_zero_release_destroys` |
| LCV04–05 | `lifecycle_identity_self_assignment_parent_and_one_shot_procedure`, `failed_parent_qi_and_procedure_qi_preserve_only_the_procedure` |
| LCV06 | `lcv25_27_generator_options_and_procedure_order`, `failed_parent_qi_and_procedure_qi_preserve_only_the_procedure` |
| LCV07 | `missing_zero_clsid_and_second_build_leak` |
| LCV08 | `exact_two_pass_slider_common_init_parent_front_order`, `every_published_type_traverses_the_production_builder` |
| LCV09 | `fallback_attempts_one_lowest_class_then_custom_and_retains_requested_ids`, `default_parent_is_exact_opaque_manager_slot_and_all_factory_failures` |
| LCV10–28 | numbered `lcv10_*` through `lcv28_32_33_*` fixtures |
| LCV29 | `common_geometry_caption_resource_and_unconditional_parameters`, `exact_two_pass_slider_common_init_parent_front_order` |
| LCV30 | `release_only_order_refcounts_and_accessor_channels`, `lifecycle_identity_self_assignment_parent_and_one_shot_procedure` |
| LCV31 | `fragment_wrapper_exact_common_text_and_lc8_partial` |
| LCV32–33 | `lcv28_32_33_failure_boundaries` (unsupported edge domain, not native garbage or OOM equivalence) |

Real-game verification is inapplicable to this isolated core-only owner: no
shadow/replacement switch or adapter wiring is added. The gate runs the recording
fixtures and the entire engine workspace and plugin test suite.
