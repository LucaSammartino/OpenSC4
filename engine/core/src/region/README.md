# cSC4Region

`region/mod.rs` implements the cSC4Region owner in portable Rust from
`docs/specs/region-model.md` §§1–8 and `docs/specs/region.md`. It does not implement
cSC4RegionalTerrainMap, cSC4RegionalTerrainView, nation or region-screen owners.

`RegionHost` supplies the nation's directories, ordered filesystem enumeration,
resource export/localization, framework image decoding, RNG/time, tutorial registry
and regional-city package services. The core performs the application-version gate,
placement, overflow, collision naming, lifecycle and all region-owned decisions.
The external regional-city owner decodes/updates foundations.md §11; its file bytes
are not redefined here. The host must preserve unrelated package records, decode
BMP/PNG top-row first, close packages, and remove partial resource exports.

`codec.rs` supplies the 12-byte signed location record, a successful-operation
byte codec for the region's four INI writes (including existing-file splices),
and uncompressed 24/32-bit BMP decoding plus canonical 24-bit encoding. Each INI
write is a separate host operation so a failed write does not stop later keys.
An application-version record that is missing defaults to 65; an unreadable one
is refused. No region savegame record is specified or invented.

Boxes use stable opaque IDs and one Arc reference per allocated box. Overwriting
a cell retains the replaced box, and zero-area insertion retains an unreachable
box, as specified. Drop deliberately forgets these allocated references, preserving
the specified lifetime even after region destruction. Failed reload leaves the caller's ID stale,
and looking it up returns None instead of dereferencing freed memory (RM17).
Getters have no RefCell; Region and CityRef are Send + Sync. Embedders synchronize
concurrent mutation of Region; individual city projections use RwLock.

The detailed model governs conflicting older summary statements (RQ3). Name
tokens use the specified eight-digit hexadecimal prefix parser (RM15); sanitizing
retains UTF-8 (RM5). Suffixes use the last dot in the entire base path, with an
explicit FilenamePosition failure if no dot survives (RM18). Out-of-range heights
use the specified safe BaseTerrainType failure (RM16).

## Owner closure

Every row below is for **cSC4Region**, unless explicitly marked external.
Test names refer to `tests.rs`.
Each row without open IDs has status **implemented** (external rows are outside
this owner). Open rows below name the remaining observation explicitly.

| Spec section | Code item | Covering test or open contract | Status |
|---|---|---| --- |
| region-model §0 summary | Region | init_idempotent_shutdown_writes_resolved_name_and_retains_locations | implemented |
| region-model §1.1 identity/refcount | query_interface, add_ref, release | identity_fields_and_lifetime | implemented |
| region-model §1.2 all state fields | Region::default, getters/setters, boxes, overflow, locations | identity_fields_and_lifetime; city_boxes_overlap_removal_counted_references_and_list_append | implemented |
| region-model §1.3 all 26 slots | Region methods, BoxId, release | identity_fields_and_lifetime; reload_success_failure_and_delete_city; move_city_collision_serial_rewrite_active_folder_and_source_shutdown | implemented |
| region-model §1.4 construction/destruction | Default, release, Drop | identity_fields_and_lifetime; orphaned_box_references_survive_region_destruction | implemented |
| region-model §2 paths | folder, RegionHost directory services | create_sanitization_templates_starter_and_failure_paths | implemented |
| region-model §2.1 initial naming | set_name, set_directory_name, load_config; nation's enumeration is external | config_conformance_localization_and_range_patch | implemented |
| region-model §2.2 city file selection | load_cities_from_directory | package_version_gate_extensions_and_read_failures | implemented |
| region-model §3 INI format | Config, parse_line/scan_long reuse | config_write_codec_preserves_tail_and_replaces_first_keys | implemented |
| region-model §3.1 config reads | load_config, Config::apply_ini, localization_id | config_conformance_localization_and_range_patch; rm15_name_token_worked_cases (RM15) | implemented |
| region-model §3.2 config writes | shutdown, Config::write_ini_entry | init_idempotent_shutdown_writes_resolved_name_and_retains_locations; config_write_codec_preserves_tail_and_replaces_first_keys | implemented |
| region-model §4 layout image | Image, decode_bmp, RegionHost decoders | location_and_bmp_same_input_round_trips | implemented |
| region-model §4.1 layout fallback | load_city_locations | picture_fallback_keeps_old_locations_if_both_fail | implemented |
| region-model §4.2 pixel scan | Image::locations | location_vectors_colors_fit_and_discovery_order; conformance_distribution_vectors | implemented |
| region-model §5.1 package read | read_regional_city, load_cities_from_directory, AppVersion | package_version_gate_extensions_and_read_failures | implemented |
| region-model §5.2 placement | load_cities, overlaps_occupied_cell | load_cities_overflow_and_x_size_only | implemented |
| region-model §5.3 overflow order | overflow, load_cities | duplicate_timbuktu_vector_and_copy_failure; load_cities_overflow_and_x_size_only | implemented |
| region-model §5.4 queries | get_city, get_all_cities, city_locations | city_boxes_overlap_removal_counted_references_and_list_append; bounds_ignore_extents_and_degenerate_axes | implemented |
| region-model §5.5 insertion/removal | insert_city, remove_city, Drop | city_boxes_overlap_removal_counted_references_and_list_append; remove_erases_rectangle_by_key_even_after_replacement; orphaned_box_references_survive_region_destruction; reload_success_failure_and_delete_city (RM17 stale aliases and counted survivor) | implemented |
| region-model §5.6 bounds | bounding_rect | bounds_ignore_extents_and_degenerate_axes | implemented |
| region-model §6.1 Init | init, RegionGlobals::init_seen | init_idempotent_shutdown_writes_resolved_name_and_retains_locations | implemented |
| region-model §6.2 Shutdown | shutdown | shutdown_overflow_and_ignored_write_failures_delete_recursive | implemented |
| region-model §6.3 Delete | delete | shutdown_overflow_and_ignored_write_failures_delete_recursive | implemented |
| region-model §6.4 creation/starter/templates | create, create_starter, sanitize_file_name | create_sanitization_templates_starter_and_failure_paths (RM5 UTF-8, RM6 empty starter, RQ3 suffixes 2 then 3) | implemented |
| region-model §7 city-owner calls | City/CityRef, RegionHost | move_city_collision_serial_rewrite_active_folder_and_source_shutdown | implemented |
| region-model §7.1 reload | reload_city | reload_success_failure_and_delete_city (RM17 success substitutes caller box; aliases stay stale; failure keeps file) | implemented |
| region-model §7.2 delete city | delete_city | reload_success_failure_and_delete_city | implemented |
| region-model §7.3 move city | move_city | move_city_collision_serial_rewrite_active_folder_and_source_shutdown; move_failures_always_reinit_and_same_region_reassigns_serial (RQ3 result matrix); open: RM7 observe same-region serial/filename changes in play | open |
| region-model §7.4 filenames | save_path_for_city, active_save_path_for_city | filename_conformance_and_serial_generation; rm18_truncated_filename_worked_cases (RM18 whole-path dot and FilenamePosition failure) | implemented |
| region-model §7.5 overflow transfer | move_overflow_to_downloads | duplicate_timbuktu_vector_and_copy_failure | implemented |
| region-model §7.6 tutorial reset | reset_tutorial_city | tutorial_reset_polarity_reload_failure_and_reactivation (RM17 safe failure boundary, RQ3 polarity); open: RM8 observe nation's use of returned bool | open |
| region-model §8.1 terrain type/height | set_base_terrain_type, base_terrain_height, RegionGlobals | terrain_height_table_low_water_bytes_and_thread_safe_getters (RM16 worked values and unchanged state on failure) | implemented |
| region-model §8.2 water | water_preferences | terrain_height_table_low_water_bytes_and_thread_safe_getters | implemented |
| region-model §§9–10 | external terrain-map/view owners | Outside cSC4Region scope; no terrain-map coverage claimed | external |
| region-model §11 external drivers | RegionHost and public lifecycle | init_idempotent_shutdown_writes_resolved_name_and_retains_locations | implemented |
| region-model §12.1 picture counts | Image::locations | conformance_distribution_vectors (synthetic layouts, not shipped-asset validation) | implemented |
| region-model §12.2 duplicate Timbuktu | load_cities, move_overflow_to_downloads | duplicate_timbuktu_vector_and_copy_failure (specified ordered inputs); open: RM12 first-load filesystem order and tutorial rebuild | open |
| region-model §12.3 bounds | bounding_rect | bounds_ignore_extents_and_degenerate_axes | implemented |
| region-model §12.4 LoadConfig | load_config | config_conformance_localization_and_range_patch | implemented |
| region-model §12.5 creation | create | create_sanitization_templates_starter_and_failure_paths | implemented |
| region-model §12.6 Init result | init | init_idempotent_shutdown_writes_resolved_name_and_retains_locations | implemented |
| region-model §12.7 filename | save_path_for_city | filename_conformance_and_serial_generation | implemented |
| region-model §12.8 terrain map | external cSC4RegionalTerrainMap | Not assigned to cSC4Region | external |
| region-model §13 open contracts | contracts and safe failures above | open: RM7 same-region serial/rename in play; RM8 nation's bool use; RM9 tuning values/height and water consumers; RM12 first-load enumeration and tutorial rebuild; RM5, RM6, RM15–RM18 and RQ3 implemented by cited tests | open |
| region §0 summary | Region | init_idempotent_shutdown_writes_resolved_name_and_retains_locations | implemented |
| region §1 identity/state/slots | Default, query_interface, lifecycle and query methods | identity_fields_and_lifetime | implemented |
| region §2 INI | Config, load_config, shutdown | config_conformance_localization_and_range_patch; config_write_codec_preserves_tail_and_replaces_first_keys | implemented |
| region §3 layout | Image::locations, load_city_locations | location_vectors_colors_fit_and_discovery_order; picture_fallback_keeps_old_locations_if_both_fail | implemented |
| region §4 loading/overflow | init, load_cities_from_directory, load_cities, move_overflow_to_downloads | package_version_gate_extensions_and_read_failures; load_cities_overflow_and_x_size_only; duplicate_timbuktu_vector_and_copy_failure | implemented |
| region §5 cell map | BoxId and city-map methods | city_boxes_overlap_removal_counted_references_and_list_append; reload_success_failure_and_delete_city | implemented |
| region §6 creation/move/delete/reset | create, move_city, delete, reset_tutorial_city | create_sanitization_templates_starter_and_failure_paths; move_failures_always_reinit_and_same_region_reassigns_serial; tutorial_reset_polarity_reload_failure_and_reactivation (RQ3) | implemented |
| region §7 queries | bounding_rect, city_locations, base_terrain_height, water_preferences | bounds_ignore_extents_and_degenerate_axes; terrain_height_table_low_water_bytes_and_thread_safe_getters (RM16) | implemented |
| region §8 helpers | read_regional_city, write_regional_city, create_city_serial_number, active_save_path_for_city | package_version_gate_extensions_and_read_failures; filename_conformance_and_serial_generation | implemented |
| region §9 quirks | scan, overflow, move, bounds | location_vectors_colors_fit_and_discovery_order; load_cities_overflow_and_x_size_only; move_city_collision_serial_rewrite_active_folder_and_source_shutdown; bounds_ignore_extents_and_degenerate_axes | implemented |
| region §10 open questions | RegionHost make_writable; save_path_for_city, create_starter | filename_conformance_and_serial_generation; rm18_truncated_filename_worked_cases; create_sanitization_templates_starter_and_failure_paths (RQ2); open: RQ1 observe attribute-2 change and effective file writability | open |
| region §11.1–3 conformance tiles/colors/fit | Image::locations | location_vectors_colors_fit_and_discovery_order | implemented |
| region §11.4 overflow | load_cities | load_cities_overflow_and_x_size_only | implemented |
| region §11.5 INI | Config::apply_ini | config_conformance_localization_and_range_patch | implemented |

An in-game read-only shadow is feasible next after an adapter supplies regional-city
references and RegionHost services. This core-only lane adds no plugin/capi wiring and
requests no game run. Shipped bitmap counts and inferred filesystem behavior require
separate integration evidence; synthetic tests do not establish either.
