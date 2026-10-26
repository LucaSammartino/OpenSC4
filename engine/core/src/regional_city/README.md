# Regional city

`regional_city::RegionalCity` is the clean-room cSC4RegionalCity core owner.
Only docs/specs/regional-city.md, regional-city-records.md, foundations §11 and
the referenced neighbors §2.2 conversion contract supply behavior. No plugin,
C ABI, game layout, thumbnail renderer, application or region replacement is added.

State is owned data with shared connection/deal interface references. A host must
serialize access to a city during mutation; core getters use no RefCell. The
workforce getter returns a reference to the stored float; a future wrapper must
keep the object at a stable address and use the documented pointer return ABI.

`decode_record` is the wire codec, with no localization or deal-list mutation.
`read_record` applies tutorial mapping, localization, establishment rules, merges
both lists into O, makes deals Silent and subscribes messages. The write path
always writes version 13. The synthetic codec test preserves both lists byte for
byte before object normalization. Host-owned elements that lack a serializable
interface are represented by `RecordElements` returning None; their list counts
remain unchanged. Package helpers retain output on stream failure and preserve
the stream result across IsOpen/Close/Shutdown/Release cleanup (RR9).

## Owner closure

Each row without open IDs has status **implemented**. Open rows retain only the
stated runtime observation; their specified core behavior is covered below.
Additional test names refer to `closure_tests.rs`.

| Owner / spec section | Code item | Test / disposition |
|---|---|---|
| cSC4RegionalCity / regional-city §0 | RegionalCity | defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / regional-city §1 identity and lifecycle | answers_iid, query_interface, add_ref, release; read_package | defaults_identity_life_and_simple_slots; path_gate_output_preservation_finish_and_one_record |
| cSC4RegionalCity / regional-city §2 fields | RegionalCity::new, unknown_strings and saved metadata | defaults_identity_life_and_simple_slots; ry2_rr7_rr8_metadata_preferences_and_opaque_string_preservation (RY2, RR7) |
| cSC4RegionalCity / regional-city §3 slots | copy_slots, string_slots, separate world-position/base-elevation accessors, explicit population/statistics/position/deal/cache methods | defaults_identity_life_and_simple_slots; moving_and_copy_outputs; preferences_and_setup_city_order |
| cSC4RegionalCity / regional-city §4.1 | init, shutdown | defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / regional-city §4.2 | get_tax_rate, update_city_cache | statistics_conformance_and_insertion_quirks; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §4.3 | get_population, get_extrapolated_population, get_allowable_extrapolation, extrapolate_growth, reset_growth | statistics_conformance_and_insertion_quirks |
| cSC4RegionalCity / regional-city §5 | commodity_slots, get_commodity_balance | statistics_conformance_and_insertion_quirks |
| cSC4RegionalCity / regional-city §6.1 | set_position | moving_and_copy_outputs |
| cSC4RegionalCity / regional-city §6.2 | find_connection, get_all_connections, change_symmetric_connection, mark_connections_for_removal, treat_incoming_as_new, reconcile_connections, create_symmetric_connections | pending_layout_conversion_duplicates_sort_and_pairs; apply_connections_ferry_rollback_skip_pairs_and_poles; edge_bytes_and_remove_cleanup; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §7.1 | setup_preferences, update_preferences, setup_city | preferences_and_setup_city_order; ry2_rr7_rr8_metadata_preferences_and_opaque_string_preservation (RR8 immediate prefix-free path, no host disk call) |
| cSC4RegionalCity / regional-city §7.2 | update_city_cache | message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §7.3 | get_points_of_interest, update_city_cache | moving_and_copy_outputs; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §8 quirks | map/commodity insertion, set_position, get_tax_rate | statistics_conformance_and_insertion_quirks; moving_and_copy_outputs |
| cSC4RegionalCity / regional-city §9 open questions | unknown_strings retained independently | open: RY2 needs an original nonempty string producer with writer stack; RR3 implemented below |
| cSC4RegionalCity / regional-city §10 vectors 1–6 | statistics, movement, tax | statistics_conformance_and_insertion_quirks; moving_and_copy_outputs; conformance regional_city_statistics |
| cSC4RegionalCity / records §0–1 | explicit wire commodity order; workforce address | wire_round_trip_header_and_object_read; defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / records §2.1 | read_package, write_package, read_record, write_record_with | path_gate_output_preservation_finish_and_one_record; non_serializable_element_counts_are_not_adjusted; rr9_cleanup_preserves_result_and_unrelated_record (RR9 prefix 1) |
| cSC4RegionalCity / records §2.2 | read_path, write_path, finish | rr9_cleanup_preserves_result_and_unrelated_record; path_gate_output_preservation_finish_and_one_record (RR9 IsOpen/Close false preserve success; unrelated key/payload survives) |
| cSC4RegionalCity / records §2.3–2.5 | placeholder; package helpers for caller orchestration | defaults_identity_life_and_simple_slots; save traversal and app Read/Write belong to cSC4App, not this owner |
| cSC4RegionalCity / records §3.1–3.4 | PendingChange codec, queue producers, reconcile_connections | pending_layout_conversion_duplicates_sort_and_pairs; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / records §4.1–4.4 | prepare_changes, network_type | pending_layout_conversion_duplicates_sort_and_pairs; rr4_rr5_west_highway_orientation_singleton_and_pair (RR4 marker orientation; RR5 odd queue creates marker, no placement) |
| cSC4RegionalCity / records §4.5–4.8 | create_symmetric_connections, update_edges, ConnectionHost; FC4RegionalCityBorder.h native calls | apply_connections_ferry_rollback_skip_pairs_and_poles; edge_bytes_and_remove_cleanup; rr3_last_group_mask_and_city_border_byte; engine/tests/regionalcity_border_test.cpp (RR3 slot 11, query flags, tool arguments and last grouped mask) |
| cSC4RegionalCity / records §5.1–5.3 | MessageContext identity checks, do_message, update_city_cache | message_handlers_cache_preservation_and_reconciliation_cases; ry2_rr7_rr8_metadata_preferences_and_opaque_string_preservation (RR7 both cache overloads) |
| cSC4RegionalCity / records §5.4 | get_workforce_percentage returns stored float reference | rr6_workforce_getter_returns_address_of_stored_float (RR6 bits 0x3F000000, core address convention) |
| Other owners / records §6.1–6.5 | Thumbnail free functions, cSC4City renderer/save bit, region-screen fallback | outside cSC4RegionalCity ownership; no thumbnail fields/methods assigned to this owner |
| cSC4RegionalCity / records §7–8 evidence and quirks | codec, create_symmetric_connections | wire_round_trip_header_and_object_read; apply_connections_ferry_rollback_skip_pairs_and_poles |
| cSC4RegionalCity / records §9 open contracts | contracts and preservation tests above | open observations: RR4 property-only marker/vehicle comparison; RR5 stock singleton producer/rendering trace; RR6 slot-13 return consumers; RR7 metadata writer stacks; RR8 preference caller/path/disk trace; RR9 physical allocation after replacement (no engine byte-placement rule) |
| cSC4RegionalCity / records §10 vectors 1,3–10,13 | package gates, queue codec/conversion/dedup/sort/pairs, ferry, rollback, handlers, workforce | named tests above; vector 2 app behavior and 11–12 thumbnails belong to other owners |
| cSC4RegionalCity / foundations §11.1–11.2 | record key, decode_record_into_with, read_record_into_with, factory decoders, write_record | wire_round_trip_header_and_object_read; version_gates_and_collection_limits; ry7_absent_keys_duplicate_commodities_and_pois; ry7_partial_receiver_failure_and_atomic_wrapper_publication; ry7_negative_deal_count_attempts_first_element (RY7) |
| cSC4RegionalCity / foundations §11.3–11.5 | extract, update_local_deals, set_local_deals, get_local_deals, update_import_export, return_local_deals | claiming_and_import_export_vectors; conformance foundations_record_and_deal_lifecycle |
| cSC4RegionalCity / foundations §11.6 | do_message, reconcile_connections, queued post contract | message_handlers_cache_preservation_and_reconciliation_cases; edge_bytes_and_remove_cleanup |
| cSC4RegionalCity / foundations §14 vectors 20–22 | wire codec + normalized Read; deal claiming/import/export | engine/conformance/tests/regional_city.rs; extended core version/failure/claiming vectors |

No specified core method is left Unsupported. In-place Read retains absent keys,
appends POIs and keeps earlier mutations on failure; factory decoders publish only
on success. The private C API wire transport remains an atomic factory decoder.

## Next native verification

A read-only shadow is feasible next: capture regional interface getters, population
maps, queue-bearing record bytes and connection/deal serialization from the game's
interfaces; compare header fields, commodity order, growth maps, pending changes,
and normalized deal lists against this owner. Raw byte equality must be measured
before object Read normalization. A future plugin seam is needed; this lane does
not claim exhaustive native equivalence. RR3 adapter calls are tested by
regionalcity_border_test; remaining observation-only questions are listed above.
