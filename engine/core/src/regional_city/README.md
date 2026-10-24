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
the published output on a subsequent finish failure.

## Owner closure

| Owner / spec section | Code item | Test / disposition |
|---|---|---|
| cSC4RegionalCity / regional-city §0 | RegionalCity | defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / regional-city §1 identity and lifecycle | answers_iid, query_interface, add_ref, release; read_package | defaults_identity_life_and_simple_slots; path_gate_output_preservation_finish_and_one_record |
| cSC4RegionalCity / regional-city §2 fields | RegionalCity::new and public saved fields | defaults_identity_life_and_simple_slots; wire_round_trip_header_and_object_read |
| cSC4RegionalCity / regional-city §3 slots | copy_slots, string_slots, separate world-position/base-elevation accessors, explicit population/statistics/position/deal/cache methods | defaults_identity_life_and_simple_slots; moving_and_copy_outputs; preferences_and_setup_city_order |
| cSC4RegionalCity / regional-city §4.1 | init, shutdown | defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / regional-city §4.2 | get_tax_rate, update_city_cache | statistics_conformance_and_insertion_quirks; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §4.3 | get_population, get_extrapolated_population, get_allowable_extrapolation, extrapolate_growth, reset_growth | statistics_conformance_and_insertion_quirks |
| cSC4RegionalCity / regional-city §5 | commodity_slots, get_commodity_balance | statistics_conformance_and_insertion_quirks |
| cSC4RegionalCity / regional-city §6.1 | set_position | moving_and_copy_outputs |
| cSC4RegionalCity / regional-city §6.2 | find_connection, get_all_connections, change_symmetric_connection, mark_connections_for_removal, treat_incoming_as_new, reconcile_connections, create_symmetric_connections | pending_layout_conversion_duplicates_sort_and_pairs; apply_connections_ferry_rollback_skip_pairs_and_poles; edge_bytes_and_remove_cleanup; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §7.1 | setup_preferences, update_preferences, setup_city | preferences_and_setup_city_order |
| cSC4RegionalCity / regional-city §7.2 | update_city_cache | message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §7.3 | get_points_of_interest, update_city_cache | moving_and_copy_outputs; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / regional-city §8 quirks | map/commodity insertion, set_position, get_tax_rate | statistics_conformance_and_insertion_quirks; moving_and_copy_outputs |
| cSC4RegionalCity / regional-city §9 open questions | unknown strings retained; connection algorithm follows records §4 | open: RY2 (meaning only); RR3 (native network call arguments) |
| cSC4RegionalCity / regional-city §10 vectors 1–6 | statistics, movement, tax | statistics_conformance_and_insertion_quirks; moving_and_copy_outputs; conformance regional_city_statistics |
| cSC4RegionalCity / records §0–1 | explicit wire commodity order; workforce address | wire_round_trip_header_and_object_read; defaults_identity_life_and_simple_slots |
| cSC4RegionalCity / records §2.1 | read_package, write_package, read_record, write_record_with | path_gate_output_preservation_finish_and_one_record; non_serializable_element_counts_are_not_adjusted |
| cSC4RegionalCity / records §2.2 | read_path, write_path | path_gate_output_preservation_finish_and_one_record |
| cSC4RegionalCity / records §2.3–2.5 | placeholder; package helpers for caller orchestration | defaults_identity_life_and_simple_slots; save traversal and app Read/Write belong to cSC4App, not this owner |
| cSC4RegionalCity / records §3.1–3.4 | PendingChange codec, queue producers, reconcile_connections | pending_layout_conversion_duplicates_sort_and_pairs; message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / records §4.1–4.4 | prepare_changes, network_type | pending_layout_conversion_duplicates_sort_and_pairs |
| cSC4RegionalCity / records §4.5–4.8 | create_symmetric_connections, update_edges, ConnectionHost | apply_connections_ferry_rollback_skip_pairs_and_poles; edge_bytes_and_remove_cleanup |
| cSC4RegionalCity / records §5.1–5.3 | MessageContext identity checks, do_message, update_city_cache | message_handlers_cache_preservation_and_reconciliation_cases |
| cSC4RegionalCity / records §5.4 | get_workforce_percentage | defaults_identity_life_and_simple_slots |
| Other owners / records §6.1–6.5 | Thumbnail free functions, cSC4City renderer/save bit, region-screen fallback | outside cSC4RegionalCity ownership; no thumbnail fields/methods assigned to this owner |
| cSC4RegionalCity / records §7–8 evidence and quirks | codec, create_symmetric_connections | wire_round_trip_header_and_object_read; apply_connections_ferry_rollback_skip_pairs_and_poles |
| cSC4RegionalCity / records §9 open contracts | semantic network callbacks, stored unknown fields, pointer getter | open: RR3, RR4–RR9 meanings/ABI/native package arguments; specified behavior implemented literally |
| cSC4RegionalCity / records §10 vectors 1,3–10,13 | package gates, queue codec/conversion/dedup/sort/pairs, ferry, rollback, handlers, workforce | named tests above; vector 2 app behavior and 11–12 thumbnails belong to other owners |
| cSC4RegionalCity / foundations §11.1–11.2 | record key, decode_record, read_record, write_record | wire_round_trip_header_and_object_read; version_gates_and_collection_limits; open: RY7 for unspecified malformed Read edge cases |
| cSC4RegionalCity / foundations §11.3–11.5 | extract, update_local_deals, set_local_deals, get_local_deals, update_import_export, return_local_deals | claiming_and_import_export_vectors; conformance foundations_record_and_deal_lifecycle |
| cSC4RegionalCity / foundations §11.6 | do_message, reconcile_connections, queued post contract | message_handlers_cache_preservation_and_reconciliation_cases; edge_bytes_and_remove_cleanup |
| cSC4RegionalCity / foundations §14 vectors 20–22 | wire codec + normalized Read; deal claiming/import/export | engine/conformance/tests/regional_city.rs; extended core version/failure/claiming vectors |

No specified cSC4RegionalCity method is left Unsupported. RY7 records literal
choices for malformed streams, not a deferred method. Existing open questions
about meanings do not prevent implementation of the rules actually published.

## Next native verification

A read-only shadow is feasible next: capture regional interface getters, population
maps, queue-bearing record bytes and connection/deal serialization from the game's
interfaces; compare header fields, commodity order, growth maps, pending changes,
and normalized deal lists against this owner. Raw byte equality must be measured
before object Read normalization. A future plugin seam is needed; this lane does
not claim native or in-game equivalence. Network callbacks need native RR3
qualification before replacing the original edge-rebuild path.
