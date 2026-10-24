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
box, as specified. Safe Rust reclaims unreachable allocations when Region itself
is dropped; the original leaks them. Failed reload leaves the caller's ID stale,
and looking it up returns None instead of dereferencing freed memory (RM17).
Getters have no RefCell; Region and CityRef are Send + Sync. Embedders synchronize
concurrent mutation of Region; individual city projections use RwLock.

The detailed model governs conflicting older summary statements (RQ3). Other
literal choices are prefix hexadecimal parsing (RM15), UTF-8 sanitization (RM5),
and appending a filename suffix when truncation removed every dot (RM18).
Only out-of-range base-height lookup is Unsupported (RM16): the spec gives no
portable value for an arbitrary out-of-bounds process-memory read.

## Owner closure

Every row below is for **cSC4Region**, unless explicitly marked external.
Test names refer to `tests.rs`.

| Spec section | Code item | Covering test or open contract |
|---|---|---|
| region-model §0 summary | Region | init_idempotent_shutdown_writes_resolved_name_and_retains_locations |
| region-model §1.1 identity/refcount | query_interface, add_ref, release | identity_fields_and_lifetime |
| region-model §1.2 all state fields | Region::default, getters/setters, boxes, overflow, locations | identity_fields_and_lifetime; city_boxes_overlap_removal_counted_references_and_list_append |
| region-model §1.3 all 26 slots | Region methods, BoxId, release | identity_fields_and_lifetime; reload_success_failure_and_delete_city; move_city_collision_serial_rewrite_active_folder_and_source_shutdown |
| region-model §1.4 construction/destruction | Default, release | identity_fields_and_lifetime |
| region-model §2 paths | folder, RegionHost directory services | create_sanitization_templates_starter_and_failure_paths |
| region-model §2.1 initial naming | set_name, set_directory_name, load_config; nation's enumeration is external | config_conformance_localization_and_range_patch |
| region-model §2.2 city file selection | load_cities_from_directory | package_version_gate_extensions_and_read_failures |
| region-model §3 INI format | Config, parse_line/scan_long reuse | config_write_codec_preserves_tail_and_replaces_first_keys |
| region-model §3.1 config reads | load_config, Config::apply_ini | config_conformance_localization_and_range_patch; open: RM15 |
| region-model §3.2 config writes | shutdown, Config::write_ini_entry | init_idempotent_shutdown_writes_resolved_name_and_retains_locations; config_write_codec_preserves_tail_and_replaces_first_keys |
| region-model §4 layout image | Image, decode_bmp, RegionHost decoders | location_and_bmp_same_input_round_trips |
| region-model §4.1 layout fallback | load_city_locations | picture_fallback_keeps_old_locations_if_both_fail |
| region-model §4.2 pixel scan | Image::locations | location_vectors_colors_fit_and_discovery_order; conformance_distribution_vectors |
| region-model §5.1 package read | read_regional_city, load_cities_from_directory, AppVersion | package_version_gate_extensions_and_read_failures |
| region-model §5.2 placement | load_cities, overlaps_occupied_cell | load_cities_overflow_and_x_size_only |
| region-model §5.3 overflow order | overflow, load_cities | duplicate_timbuktu_vector_and_copy_failure; load_cities_overflow_and_x_size_only |
| region-model §5.4 queries | get_city, get_all_cities, city_locations | city_boxes_overlap_removal_counted_references_and_list_append; bounds_ignore_extents_and_degenerate_axes |
| region-model §5.5 insertion/removal | insert_city, remove_city | city_boxes_overlap_removal_counted_references_and_list_append; remove_erases_rectangle_by_key_even_after_replacement; open: RM17 |
| region-model §5.6 bounds | bounding_rect | bounds_ignore_extents_and_degenerate_axes |
| region-model §6.1 Init | init, RegionGlobals::init_seen | init_idempotent_shutdown_writes_resolved_name_and_retains_locations |
| region-model §6.2 Shutdown | shutdown | shutdown_overflow_and_ignored_write_failures_delete_recursive |
| region-model §6.3 Delete | delete | shutdown_overflow_and_ignored_write_failures_delete_recursive |
| region-model §6.4 creation/starter/templates | create, create_starter, sanitize_file_name | create_sanitization_templates_starter_and_failure_paths; open: RM5, RM6 |
| region-model §7 city-owner calls | City/CityRef, RegionHost | move_city_collision_serial_rewrite_active_folder_and_source_shutdown |
| region-model §7.1 reload | reload_city | reload_success_failure_and_delete_city; open: RM17 |
| region-model §7.2 delete city | delete_city | reload_success_failure_and_delete_city |
| region-model §7.3 move city | move_city | move_city_collision_serial_rewrite_active_folder_and_source_shutdown; move_failures_always_reinit_and_same_region_reassigns_serial; open: RQ3, RM7 |
| region-model §7.4 filenames | save_path_for_city, active_save_path_for_city | filename_conformance_and_serial_generation; open: RM18 |
| region-model §7.5 overflow transfer | move_overflow_to_downloads | duplicate_timbuktu_vector_and_copy_failure |
| region-model §7.6 tutorial reset | reset_tutorial_city | tutorial_reset_polarity_reload_failure_and_reactivation; open: RQ3, RM8, RM17 |
| region-model §8.1 terrain type/height | set_base_terrain_type, base_terrain_height, RegionGlobals | terrain_height_table_low_water_bytes_and_thread_safe_getters; open: RM16 for out-of-range lookup only |
| region-model §8.2 water | water_preferences | terrain_height_table_low_water_bytes_and_thread_safe_getters |
| region-model §§9–10 | external terrain-map/view owners | Outside cSC4Region scope; no terrain-map coverage claimed |
| region-model §11 external drivers | RegionHost and public lifecycle | init_idempotent_shutdown_writes_resolved_name_and_retains_locations |
| region-model §12.1 picture counts | Image::locations | conformance_distribution_vectors (synthetic layouts, not shipped-asset validation) |
| region-model §12.2 duplicate Timbuktu | load_cities, move_overflow_to_downloads | duplicate_timbuktu_vector_and_copy_failure (specified ordered inputs, no runtime listing claim) |
| region-model §12.3 bounds | bounding_rect | bounds_ignore_extents_and_degenerate_axes |
| region-model §12.4 LoadConfig | load_config | config_conformance_localization_and_range_patch |
| region-model §12.5 creation | create | create_sanitization_templates_starter_and_failure_paths |
| region-model §12.6 Init result | init | init_idempotent_shutdown_writes_resolved_name_and_retains_locations |
| region-model §12.7 filename | save_path_for_city | filename_conformance_and_serial_generation |
| region-model §12.8 terrain map | external cSC4RegionalTerrainMap | Not assigned to cSC4Region |
| region-model §13 open contracts | comments, explicit Unsupported | RM5–RM9, RM12 retained; new RM15–RM18 and RQ3; detailed body otherwise implemented |
| region §0 summary | Region | init_idempotent_shutdown_writes_resolved_name_and_retains_locations |
| region §1 identity/state/slots | Default, query_interface, lifecycle and query methods | identity_fields_and_lifetime |
| region §2 INI | Config, load_config, shutdown | config_conformance_localization_and_range_patch; config_write_codec_preserves_tail_and_replaces_first_keys |
| region §3 layout | Image::locations, load_city_locations | location_vectors_colors_fit_and_discovery_order; picture_fallback_keeps_old_locations_if_both_fail |
| region §4 loading/overflow | init, load_cities_from_directory, load_cities, move_overflow_to_downloads | package_version_gate_extensions_and_read_failures; load_cities_overflow_and_x_size_only; duplicate_timbuktu_vector_and_copy_failure |
| region §5 cell map | BoxId and city-map methods | city_boxes_overlap_removal_counted_references_and_list_append; reload_success_failure_and_delete_city |
| region §6 creation/move/delete/reset | create, move_city, delete, reset_tutorial_city | create_sanitization_templates_starter_and_failure_paths; move_failures_always_reinit_and_same_region_reassigns_serial; tutorial_reset_polarity_reload_failure_and_reactivation; open: RQ3 |
| region §7 queries | bounding_rect, city_locations, base_terrain_height, water_preferences | bounds_ignore_extents_and_degenerate_axes; terrain_height_table_low_water_bytes_and_thread_safe_getters; open: RM16 |
| region §8 helpers | read_regional_city, write_regional_city, create_city_serial_number, active_save_path_for_city | package_version_gate_extensions_and_read_failures; filename_conformance_and_serial_generation |
| region §9 quirks | scan, overflow, move, bounds | location_vectors_colors_fit_and_discovery_order; load_cities_overflow_and_x_size_only; move_city_collision_serial_rewrite_active_folder_and_source_shutdown; bounds_ignore_extents_and_degenerate_axes |
| region §10 open questions | RegionHost make_writable, naming/creation | RQ1–RQ2 retained; RQ3 filed |
| region §11.1–3 conformance tiles/colors/fit | Image::locations | location_vectors_colors_fit_and_discovery_order |
| region §11.4 overflow | load_cities | load_cities_overflow_and_x_size_only |
| region §11.5 INI | Config::apply_ini | config_conformance_localization_and_range_patch |

An in-game read-only shadow is feasible next after an adapter supplies regional-city
references and RegionHost services. This core-only lane adds no plugin/capi wiring and
requests no game run. Shipped bitmap counts and inferred filesystem behavior require
separate integration evidence; synthetic tests do not establish either.
