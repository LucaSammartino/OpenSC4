# Network tools

Portable network-tool state and orchestration. `state.rs`, `place.rs`, `resolve.rs` and
`host.rs` implement the shared tool contract. `power_line.rs` and its `power_line/`
submodules implement `cSC4PowerLineTool`; geometry, network/occupant managers,
terrain, resources and rendering remain foreign interfaces. There is no plugin or C ABI wiring.

## Owner closure

Rows below cover each section assigning behavior to cSC4PowerLineTool. Names refer to
items/tests in this module unless a different module is shown. Base-tool sections that
assign no power-line-specific behavior retain their existing owners. Undefined native
construction values for spacing/clearance are not exposed as uninitialised Rust memory:
`PowerLineParams::default` represents post-Init defaults, and `init` sets them before use.

| Owner / spec section | Code item | Test / boundary |
|---|---|---|
| cSC4PowerLineTool / network-tools §1 identity, §2 state | `ToolState::new(5)`, `PowerLineTool`, `init` | `construction_and_deleting_destructor_are_structural_not_preview_undo`, `pl_l1_l2_l3_defaults_reinit_and_once_only_resource_failure` |
| cSC4PowerLineTool / network-tools §9 override | `do_network_specific_placement`, `check_for_reserved_tiles` | `pl_n1_n2_n3_wrapper_attempt_and_real_success_gate`, `reserved_tiles_*`, `reserved_tiles_zoned_does_not_stop`, `pl_r3_reserved_unknown_result_updates_previous_without_updating_last` |
| cSC4PowerLineTool / network-tools §18 conformance | Shared road/pipe tests belong to base/occupant owners; power-line override uses the portable vectors below | Power-line-specific §18.12 is road demolition of a pole, owned by base commit, not this tool; `t18_12_power_pole` in `tests.rs` |
| cSC4PowerLineTool / network-geometry §11 cost/position/reserved walk | `cost_of_solution`, `compute_pole_position`, `check_for_reserved_tiles` | `cost_of_solution_shipped_values`, `reserved_tiles_*`, `pl_g5_g6_sentinel_nan_and_special_height` |
| cSC4PowerLineTool / network-geometry §18.18 conformance (power-line cost) | `check_for_reserved_tiles`, `cost_of_solution` | `reserved_tiles_*`, `cost_of_solution_shipped_values`; other vectors belong to geometry/bridge owners |
| cSC4PowerLineTool / network-occupants §4.5 creation | `create_power_pole`, `create_record_pole` | `create_power_pole_reuses_existing`, `pl_p1_u1_u2_preview_ownership_and_manager_null_asymmetry` |
| cSC4PowerLineTool / power-line-tool §1 lifecycle | `init`, `shutdown`, `release_pole_infos`, `destruct`, `deleting_destructor` | `pl_l1_l2_l3_defaults_reinit_and_once_only_resource_failure`, `construction_and_deleting_destructor_are_structural_not_preview_undo` |
| cSC4PowerLineTool / power-line-tool §2 wrapper/highlight | `do_network_specific_placement`, `terrain_highlight` | `pl_n1_n2_n3_wrapper_attempt_and_real_success_gate`, `pl_h1_valid_skip_and_invalid_red_are_separate_passes` |
| cSC4PowerLineTool / power-line-tool §3 failure text | `failure_reason` | `pl_f1_f2_f3_only_last_anchor_and_no_zero_reason_fallback` |
| cSC4PowerLineTool / power-line-tool §4 records | `append_pole`, `link`, `PoleInfo`, `WireCell` | `ordered_adjacency_groups_link_every_pair_without_dedup_and_do_not_invent_a_producer` |
| cSC4PowerLineTool / power-line-tool §4.1 pole selection | `determine_pole_positions`, `pivot`, `intercept` | `pl_g1_g2_spacing_is_in_steps_at_pivots`, `pl_g3_g4_segment_ray_is_inclusive_and_clearance_does_not_raise_pole_base`, `pl_i1_to_i6_ordered_descriptor_forced_and_or_predicates`, `pl_b1_zone_closure_requires_last_clear_index_strictly_above_zero`, `pl_b2_b3_building_termination_and_zone_reference_asymmetry`; open: NT26 (fresh spacing vector), NT18 (group producer only) |
| cSC4PowerLineTool / power-line-tool §4.2 position | `compute_pole_position` | `pl_g5_g6_sentinel_nan_and_special_height` |
| cSC4PowerLineTool / power-line-tool §5 placement | `place_poles`, `create_record_pole` | `pl_p1_u1_u2_preview_ownership_and_manager_null_asymmetry`, `pl_p2_real_repair_calls_draw_once_and_recursive_place_disallows_more_repairs`, `pl_ep1_ep2_saved_endpoint_flags_and_duplicate_repair_entries`, `literal_fresh_axis_pivots_preserve_nt26_and_native_release_pass_skips_wire_vector_zero` |
| cSC4PowerLineTool / power-line-tool §5.1 undo | `undo_preview` | `pl_p1_u1_u2_preview_ownership_and_manager_null_asymmetry`, `pl_ep1_u1_draft_transfers_descriptor_and_restores_both_directions` |
| cSC4PowerLineTool / power-line-tool §5.2 support | `enable_connection`, `copy_wire_references` | `pl_s1_s2_enable_and_reference_copy_do_not_deduplicate` |
| cSC4PowerLineTool / power-line-tool §5.3 connection builder | `connect` | `pl_cn1_cn2_cn4_connection_gates_and_owned_storage`; open: NT25 (nonpositive allocation count) |
| cSC4PowerLineTool / power-line-tool §5.4 regional Attempt | `attempt_neighbor_connections`, `RegionalWorld` primitives | `pl_ra1_rejection_is_atomic_and_missing_cell_info_still_probes_region`, `pl_ra2_append_both_vectors_preserve_entries_and_place_removes_before_creation`, `pl_ra3_diagonal_low_mode_and_clamped_key_keep_external_coordinates`, `regional_directions_diagonal_single_cell_and_compatibility` |
| cSC4PowerLineTool / power-line-tool §5.5 curve arithmetic | `CurveParams::load`, `control_points`, `sample_cubic` | `pl_cn3_independent_default_curve_and_literal_sample_count`, `invalid_sample_count_is_reported_without_clamping_or_synthesizing_geometry`; open: NT25 |
| cSC4PowerLineTool / power-line-tool §6 reserved tiles/zone update | `check_for_reserved_tiles`, `update_on_zone_change` | `reserved_tiles_zoned_does_not_stop`, `pl_r3_reserved_unknown_result_updates_previous_without_updating_last`, `pl_z1_zu1_zu2_disabled_and_signed_pole_query_order`; open: NT22 (wire split) |
| cSC4PowerLineTool / power-line-tool §6.1 classification/probes | `classify`, `probe`, `World::allocate_wire` | `pl_c1_c2_zone_asymmetry_and_last_qualifying_occupant`, `pl_c3_c4_last_pole_and_wire_qi_ownership`, `pl_p1_u1_u2_preview_ownership_and_manager_null_asymmetry` |
| cSC4PowerLineTool / power-line-tool §6.2 relocation | `find_new_pole_position` | `pl_rl1_rl2_rl3_unsigned_rectangle_and_null_other`, `pl_rl2_nonnull_other_replaces_anchors_after_release_and_solves_without_regional_wrapper` |
| cSC4PowerLineTool / power-line-tool §6.3 wire split | `update_on_zone_change` endpoint-overlap selection and guards | `pl_zw1_zw2_zw3_zu3_endpoint_overlap_and_withheld_backward_split`; open: NT22, entire split withheld before detachment because backward exit is unproved |
| cSC4PowerLineTool / power-line-tool §6.4 zone query/cleanup | `update_on_zone_change` | `pl_z1_zu1_zu2_disabled_and_signed_pole_query_order`, `pl_zw1_zw2_zw3_zu3_endpoint_overlap_and_withheld_backward_split`; open: NT22 only for wire repair |
| cSC4PowerLineTool / power-line-tool §6.5 containers | `containers.rs` thirteen helpers | `pl_co1_co2_structural_copy_and_erase_keep_independent_inner_buffers` |
| cSC4PowerLineTool / power-line-tool §6.6 group producer | No inferred producer; caller supplies `groups`; consumer implemented by `place_poles` | open: NT18; `ordered_adjacency_groups_link_every_pair_without_dedup_and_do_not_invent_a_producer` |
| cSC4PowerLineTool / power-line-tool §7 resources | `static_init`, `read_power_pole_infos`, `init` | `resource_enumeration_destroys_ini_before_releasing_file_and_last_key_wins`, `pl_l1_l2_l3_defaults_reinit_and_once_only_resource_failure` |
| cSC4PowerLineTool / power-line-tool §8 conformance | All PL vectors mapped above; PL-R tests in parent `tests_power_line.rs` | PL-I1–I6, B1–B3, EP1–EP2, C1–C4, S1–S2, RL1–RL3, ZW1–ZW3, L1–L3, N1–N3, H1, F1–F3, G1–G6, P1–P2, U1–U2, R1–R3, Z1–Z2, CN1–CN4, ZU1–ZU3, RA1–RA3, CO1–CO2; open: NT22 (ZW3/ZU3), NT26 (G1 fresh-drag assumption) |
| cSC4PowerLineTool / power-line-tool §9 inventory/regional Place | `place_neighbor_connections` | `pl_ra2_append_both_vectors_preserve_entries_and_place_removes_before_creation`; open: NT24 (46-versus-49 inventory provenance, no missing code invented) |

`World` owns external pole-occupant allocation/initialisation and COM calls. `RegionalWorld`
owns regional manager queries/writes and drawing; Attempt/Place decisions run here.
`Resources` owns resource access; the once-only latches and property ordering run here.
`Reference` is a raw token: structural copies/drops never implicitly AddRef or Release.
Native pointer layouts, allocator identities and vtables are represented as portable data,
not ABI promises. No owner completion beyond the explicit published open questions is claimed.
