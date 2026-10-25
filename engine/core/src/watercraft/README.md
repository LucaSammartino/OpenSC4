# Watercraft core

Clean-room Rust `cSC4Watercraft` owner at `engine/core/src/watercraft/`, from
`docs/specs/watercraft.md` and its automata references. No capi/plugin wiring or
ABI changes. `Fleet` owns state and tow links; foreign services supply terrain,
resource acquisition and berth arbitration, never native owner calculations.

## Owner closure

**Implementation in progress, not owner closure.** `pending` means specified work
still to implement, not an Unsupported contract. `open` means the current spec
itself leaves the behavior unsettled. WC1–WC12 are answered in the current spec;
there are no remaining spec-side owner questions.
Settled work still marked `pending` below is implementation debt, not a spec gap.
The implemented column uses module paths relative to `engine/core/src/watercraft/`.

| Owner | Spec section | Implemented (module path / item) | Test or remaining work |
|---|---|---|---|
| cSC4Watercraft | §0 | `Watercraft`, `Fleet` | Scope. |
| cSC4Watercraft | §1 | `WatercraftRecord`, runtime fields | `record_only_saves_version_and_base_and_rejects_truncation`; pending remaining inherited/runtime fields. |
| cSC4Watercraft | §2 | `DriveParameters`, `Group` | Existing absent/zero/ancestor parameter tests. |
| cSC4Watercraft | §3 | `owner.rs::initialize_runtime`, `states.rs::shutdown` | Init factory failure and repeated-init tests; pending inherited lifecycle integration with the concrete model loader. |
| cSC4Watercraft | §3.1 | constructor, codec Write, `erase_travel_point`, `sign` | `wc3_l01_l05_l06_constructor_erase_and_sign`; pending type ID, deletion/release order and interface model. |
| cSC4Watercraft | §3.2 | endpoints, `maximum_extent`, `Footprint::build` | `wc5_f01_through_f04_model_footprint_builder`; pending outside-corner helper vectors. |
| cSC4Watercraft | §3.3 | `movement_possible`, docking cleanup, straight integration, `Station::lot_position` | Movement ordering tests; `edge_tests.rs::wc7_b01_b04_requests_and_cleanup_use_lot_position_not_berth_or_runtime`. |
| cSC4Watercraft | §3.4 | `angular_proximity`, `quantize_heading`, `on_land`, `end_towing` | Angular tests and `edge_tests.rs::wc3_on_land_samples_cell_height_only_inside_with_strict_water_comparison`; pending reentrant tow reference tests. |
| cSC4Watercraft | §3.5 | `mod.rs::DriveParameters`, `owner.rs` lifecycle prefix | Parameter tests; pending complete inherited lifecycle and effect-binding integration. |
| cSC4Watercraft | §3.6 | `models.rs::load_models` | `model_tests.rs::wc3_ml01_*` through `wc3_ml04_*`: primary setup, radius, partial failure/retry, auxiliary filtering and reference transfer. |
| cSC4Watercraft | §3.7 | `models.rs::acquire_model`, `add_model_instance`, `load_auxiliary_model` | `model_tests.rs::wc10_r01_*` through `wc10_r04_*`, `wc10_a01_*` through `wc10_a05_*`. |
| cSC4Watercraft | §3.8 | `models.rs::position_model_instance`, `publish_models`, `remove_models` | `model_tests.rs::wc11_p01_*` through `wc11_p04_*`, `wc11_u01_*` through `wc11_u07_*`. |
| cSC4Watercraft | §4 | `states.rs`, `runtime.rs::runtime_state_tick`, `crash.rs::crash_state_tick` | `common_dispatch_crash_completion_pops_and_idle_fallthrough_adds_only_once`, `crash_dispatch_double_time_outside_and_effect_order`; inherited enter result remains pending. |
| cSC4Watercraft | §5.1 | `runtime.rs::animation_tick_core`, `operations.rs`, `subsidiary.rs`, `models.rs` | Ordered core dispatch and concrete model publication exist; dedicated end-to-end animation fixture and effect/model-command execution remain pending. |
| cSC4Watercraft | §5.1.1 | `physics.rs` | `wc4_m01_m02_*` through `wc4_m07_*`, `wc4_v01_through_v04_*`. |
| cSC4Watercraft | §5.1.2 | `count_idle_always`, budget accounting | Existing idle tests; manager population writer outside this owner. |
| cSC4Watercraft | §5.1.3 | `physics.rs::slew_ai_targets`, `targets.rs` | `wc2_p02_*`, `wc4_ai03_through_ai05_asymmetric_turn_signs_and_publication`; pending caller-removal/shore-contact gates. |
| cSC4Watercraft | §5.1.4 | `inherited.rs`, `runtime.rs::animation_tick_core`, `subsidiary.rs::inherited_animation` | `wc4_i01_i03_inherited_dispatch_and_completion`, round 2 seek vectors; runtime calls concrete owner state dispatch. End-to-end fixture pending. |
| cSC4Watercraft | §5.1.5 | `subsidiary.rs::Subsidiary`, `runtime.rs::refresh_rope` | `subsidiary_visibility_suppression_completion_and_strict_lighting`; concrete rope refresh/preconditions implemented, dedicated rope fixture pending. |
| cSC4Watercraft | §5.1.6 | `commands.rs::SpeedTween::sample` | `wc3_t01_through_t04_sampling`. |
| cSC4Watercraft | §5.1.7 | `subsidiary.rs::update_model_effects`, `../model_reg_effects/` | `effect_tests.rs::wc4_fx01_*` through `wc4_fx03_*`: inherited motion, transform-before-parameter, exact frame 3, stopped effect and detached/empty controller. |
| cSC4Watercraft | §5.2 | `routes.rs::pick_location`, `fill_route_points`, `operations.rs::candidate_directions` | `picker_fills_ten_core_points_and_ai_consumes_without_host_targets`, `wandering_search_stops_after_fifteen_rounds_and_resets_failed_targets`; existing route/backtrack vectors. Additional composed overlap/backtrack cases pending. |
| cSC4Watercraft | §§5.2.1–5.2.2 | `targets.rs::ai_motion_core`, `ai_motion`, `ai_midpoints` | WC4-AI01–AI10 in `closure_tests.rs`, plus `runtime_tests.rs::picker_fills_ten_core_points_and_ai_consumes_without_host_targets`. |
| cSC4Watercraft | §5.3 | `dock_requests`, `docking_tick`, cleanup, `Station::lot_position` | Berth/queue and final approach tests; `edge_tests.rs::wc7_b01_b04_*` separates lot, berth, runtime and inherited positions. |
| cSC4Watercraft | §5.3.1 | `docking_tick` final approach | Strict pre-motion completion, unclamped time and antiparallel sign implemented; pending all named edge vectors. |
| cSC4Watercraft | §5.3.2 | `runtime.rs::docking_tick_core`, `movement.rs` final entry path | Core far-call AI/movement/collision integration implemented; dedicated far/land-continuation vector pending. |
| cSC4Watercraft | §5.4 | station and neighbor setup, `set_neighbor_route_core` | Existing station tests; `edge_tests.rs::wc7_r07_*`, `wc7_r08_b02_*` cover nullable occupant and water-height skip points. |
| cSC4Watercraft | §5.4.1 | `RouteReader`, quantization, reservation cleanup | `wc7_r03_through_r06_*`; `edge_tests.rs::wc7_r07_*`, `wc7_r08_b02_*`, `compressed_route_coordinates_wrap_as_signed_sixteen_bit_cells`, `wc7_b01_b04_*`. |
| cSC4Watercraft | §5.5 | `undocking_tick` | Motion/completion test and `edge_tests.rs::undocking_missing_occupant_blocked_and_land_continue_have_separate_results`. |
| cSC4Watercraft | §5.6 | movement gates | Existing flag/bounds/crash-state ordering test; pending inherited FindState order audit. |
| cSC4Watercraft | §5.7 | `detect_collision` | Outside removal precedes boat/shore scans; boat crash returns early, bumps continue to shore. Focused outside/shore tests; pending comprehensive composed boat/shore vectors. |
| cSC4Watercraft | §5.7.0 | `Footprint::overlaps`, `find_overlapping_boat`, `respond_to_boat_collision` | SAT contact and size-tie/bump/player-victim tests; pending all scan/size vectors and two-slot AEMS event observation. |
| cSC4Watercraft | §5.7.1 | `crash.rs::crash_sequence_tick`, `crash_state_tick`, `CrashEffectSet`, `runtime.rs::runtime_state_tick` | `crash_dispatch_double_time_outside_and_effect_order`, `crash_effect_refresh_orders_transform_before_start_and_requires_gated_effects`, `common_dispatch_crash_completion_pops_and_idle_fallthrough_adds_only_once`. |
| cSC4Watercraft | §5.7.2 | `collision.rs::bump_rollback`, `crash.rs` | WC5-S01–S07, WC5-K01–K04 and `closure_tests.rs::wc12_h11_*` implement the settled p[24] heel endpoint. |
| cSC4Watercraft | §5.7.3 | `Footprint::build` | `wc5_f01_through_f04_*`; missing geometry retains caller's explicit scratch validity/axis. |
| cSC4Watercraft | §5.7.4 | `inanimate_collision` | `wc5_i01_through_i05_*`, all 32 table cells and scan order. |
| cSC4Watercraft | §5.7.5 | `crash.rs::heel_tick` | `wc5_h01_through_h05_heel_axes_time_and_boundaries`, `wc5_h06_through_h10_contact_order_and_strict_gates`, `wc12_h11_*` through `wc12_h13_*`; contact banks identity and samples a fresh terrain normal before reading the owner transform. |
| cSC4Watercraft | §6.1 | `runtime.rs::player_drive_tick`, `movement.rs::coast`, `operations.rs::release_tow_ahead`, `physics.rs` | `wc6_10_full_drive_ignores_base_false_and_uses_two_times`; inherited channel saturation/sentinel vector. |
| cSC4Watercraft | §6.2–§6.2.3 | Float `DriveCommand`, nonlinear/inverse curves, `CommandChannel`, `SpeedTween`, nullable group | WC8/WC9 and WC6 command vectors; `edge_tests.rs::missing_group_reloads_all_defaults_and_ignores_every_drive_channel`. Pending tow-effect transform/refresh and complete drive-tick integration. |
| cSC4Watercraft | §6.3 | `following_tick`, tow hooks | `following_uses_endpoint_gap_and_slack_does_not_publish`; `edge_tests.rs::wc7_t03_t05_endpoint_gap_zero_time_slack_and_missing_tug`. |
| cSC4Watercraft | §7 | `do_message`, insertion-order fleet scan | `edge_tests.rs::wc7_t01_t02_*`, `tow_message_rejected_first_state_does_not_retry_and_unarmed_messages_forward` cover list order, equality and first-candidate rejection. |
| cSC4Watercraft | §8 | `runtime.rs::runtime_state_tick` | `common_dispatch_crash_completion_pops_and_idle_fallthrough_adds_only_once`. |
| cSC4Watercraft | §9 | `borrowed_debug_extra_text`, shared `DEBUG_TEXT` | `edge_tests.rs::debug_text_is_borrowed_from_one_shared_object_and_replaced_by_the_next_boat`; safe serialized shared borrow, same object across boats. |
| cSC4Watercraft | §10 | Contract boundary | WC1–WC12 answered. Remaining `pending` rows are implementation debt, not spec gaps. |
| cSC4Watercraft | §11 | `tests.rs`, `physics_tests.rs`, `collision_tests.rs`, `round2_tests.rs`, `closure_tests.rs`, `runtime_tests.rs`, `model_tests.rs`, `edge_tests.rs`, `effect_tests.rs` | All 23 newly settled WC10/WC11/WC12 vectors, ML01–ML04, FX01–FX03, route-fill/wrap, station-lot, towing and shared debug cases are covered; other pending vector coverage is listed above. |

The record test `record_same_input_round_trip_stabilizes_inherited_loaded_status`
checks equality of a decoded record and its same-input second round trip, and byte
stability after the inherited loaded-status normalization.

An in-game shadow is feasible after an adapter exists: capture tuning, positions,
headings, dt and model/terrain geometry, then compare proposed runtime and visual
transforms, route coordinates, state transitions and collision decisions. The full
owner shadow requires pending lifecycle/runtime integration. No game run was
requested for this unwired core module.
