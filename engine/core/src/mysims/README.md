# MySims core

Behavior source: `docs/specs/mysims.md`. Existing MySim, advisor and street
records are reused. `dispatch.rs` owns persistent job-search state and
`pool.rs` owns template loading/import. Host traits supply external services;
opaque occupant IDs carry identity, while native reference ownership remains
the adapter's responsibility. These Rc-based core hosts are single-threaded;
no plugin or C ABI integration is provided here.

## Owner closure

One row per applicable specification section and owner. Settled MS17 contracts
are implemented. MS18 transport handoffs remain Unsupported; MS19 records the
missing comparison precedence and named teleport contract, not an arrival stub.
No plugin or C ABI integration is claimed.

### cSC4MySimDispatch

| Spec section | Code item or open contract | Coverage |
|---|---|---|
| 1.1 identity, 1.2 ownership | `MySimDispatch::{class_id,interface_id,init,shutdown}`; foreign IDs through `DispatchRecordRefs` | `t39_ids`, `t36_dispatch_find_new_job`, conformance `dispatch_record_preserves_all_opaque_words_and_foreign_references` |
| 2.2 state, 4.6 trip getters | `DispatchState`, `get_state`, four independent locations, default state/previous −1, travel 255 | `ms17_01_identity_and_request_not_arrival`, conformance `ms17_01_dispatch_identity` |
| 3.3 synchronous notifications | `notify_owner` gates all delivery through `DispatchHost::has_owner`; `set_state`, `change_flags`, `report_arrival`, `report_sim_state`; counters; lock released before owner calls | `ms17_02_order_counters_and_reentrant_owner`, `no_owner_suppresses_delivery_without_suppressing_state_changes`, `owner_is_rechecked_between_synchronous_notifications`, `t38_notifications`, conformance `ms17_02_notification_order` |
| 4.1 tracking/home | `check_flags`, `set_default_destination` | `t38_notifications`, `t36_dispatch_find_new_job` |
| 4.6.1 clock, wait timestamp | `minute_message`, `animation_message`, `wait_value`; strict dwell expiration, wrapping signed clock | `ms17_03_clock_wait_and_strict_dwell`, conformance `ms17_03_clock_and_wait` |
| 4.6.1 recovery, destination selection | `animation_message`, `select_destination`, `resolve`; state/notice order and deadlines | `recovery_selection_and_transport_boundaries`, `recovery_home_fallback_and_no_selection` |
| 4.6.1 queue | `add_destination_location`; copy/front/back/replace, no forced expiration or duration clamp | `ms17_05_queue_and_deadline`, conformance `ms17_05_queue_and_deadline` |
| 4.6.1 state-2 settled branches | Player pause, stuck recovery, suspended search, exhausted failure via `TripServices` | `recovery_selection_and_transport_boundaries` |
| 4.6.1 remaining route-ready/switch branch selection | open: MS18, residual branch/mask matrix, route-index selection, neighbour retries, idle/FE/FF/cancellation and completion/recovery notice ordering; `TripOutcome::UnsupportedMs18` | `recovery_selection_and_transport_boundaries`; no successful-arrival fallback |
| 4.6.2 proximity geometry and boundaries | `location_distance`, `destination_proximity`, `within_proximity`; float32 corner-to-rectangle linear distance versus signed wrapping tuning square | Conformance `ms18_01_linear_threshold`, `ms18_02_equality_boundary`, `ms18_03_lot_geometry_direction` |
| 4.6.2 ordinary transport waits | `ordinary_switch_wait`; only different ordinary modes, clears current occupant/held automaton, target wait class | Conformance `ms18_ordinary_switch_wait_classes_and_sentinels` (all 81 pairs) |
| 4.6.2 transit stops/boarding | `transit_stop_update`, foreign `TripServices` queries/dispatch; first matching purpose, group/interface/order filter, timestamp, notice-before-release, strict expiry, stuck/flag masks | Conformance `ms18_04_stop_enumeration`, `ms18_05_wait_boundary`, `ms18_stop_groups_boarding_order_success_and_missing_stop` |
| 4.6.2 FE-marker observations and common tail | `final_marker_observation` then `apply_transport_tail`; neighbour release before geometry, near/far notices before separately observable travel/state/stuck/idle/clear/set tail | Conformance `ms18_final_marker_observations_and_common_tail_order`; selection of tail masks and near/idle outcome remains open: MS18 |
| 4.7 callers, 4.7.1 search/reset/validation | `find_new_job`, `change_job`, `set_current_commute_destination`, `validate_commute_location`, `reset_path`, `home_start_rect` | `t30`–`t37`, vectors `ms12_21`–`ms12_25`, `ms12_28` |
| 4.7.2 inherited persistent pathfinder | `init`, `pathfinders`, separate from MySim CheckCommute | `t36_dispatch_find_new_job`; no MySim owner closure claim |
| 4.11 five tuning values | `DispatchTuning`, `go_to`, `begin_idle`, `begin_located_switch`, `within_proximity`, `ordinary_switch_wait`, `transit_stop_update`, `automaton_cell_changed`; no nonnegative clamps | `ms17_04_tuning_and_signed_boundaries`, conformance `ms17_04_tuning` and MS18 geometry/wait tests; remaining branch selection open: MS18 |
| 5.1 message envelope | `DispatchHost::notify` carries the sub-ID and old/new words; adapter supplies type 0x2BA06B99 | `t38_notifications`, `ms17_02_order_counters_and_reentrant_owner` |
| 6.2.1 record | `DispatchState::{write,read}`, `write_record`, `read_record`, `deferred_setup_message`; all words, refs, route/map/queue; no RNG/padding | `ms17_07_record_bytes_roundtrip_and_deferred_setup`, conformance `ms17_07_dispatch_record`, `dispatch_record_preserves_all_opaque_words_and_foreign_references` |
| 7.2.1 GoTo | `go_to`; acceptance distinct from arrival; cancellation and route-index rejection | `ms17_01_identity_and_request_not_arrival`, `goto_cancellation_rejection_and_player_guards` |
| 7.2.1 DispatchCommute request portion | `dispatch_commute`; residence guard, hours 7/8/17/18, states 2/3 and no-queue-change branches | `ms17_06_commute_requests`, conformance `ms17_06_commute_requests_dispatch_portion`; Leave/Return MySim state/sound/last-player field belong to MySim |
| 7.2.1 TeleportToCommuteLocation and DispatchCommute state 1 | `teleport_to_commute_location`, `dispatch_commute`; invalid copy permitted, wrapping deadline/counter, all state writes before flags-then-state callbacks, then replace queue with home | Conformance `ms19_03_teleport_ordering`, `ms19_04_repeated_invalid_teleport`, `ms19_05_state_one_commute_request` |
| 7.2.1 player control | `can_u_drive_it`, `enter_u_drive_it`, `held_automaton_player_update`, `exit_u_drive_it`; mission services foreign | `ms17_08_player_entry_exit_and_no_synthetic_arrival`, `goto_cancellation_rejection_and_player_guards`, conformance `ms17_08_player_and_rng`; callback details open: MS18 |
| 8 RNG | No dispatch generator, draw or RNG record slot; delegated transport RNG open: MS18 | `ms17_07_record_bytes_roundtrip_and_deferred_setup`, conformance `ms17_08_player_and_rng` |
| 9.1 no simulation feedback, 9.2 dependencies | Query-only `JobPathfinder`, `JobSearchWorld`, `DispatchHost`, `TripServices`; transport/traffic remain foreign | `t30_job_search_setup`, vector `ms12_20_no_feedback` |
| 10 replacement boundary, 11 quirks | Core requests/timing/persistence and settled geometry/handoffs are real; no immediate-arrival stub; residual open: MS18 as listed above | Core/conformance MS17/MS18/MS19 tests; no native replacement/ABI change |
| 12 MS17 specification vectors 01–08 | Eight executable integration tests in `engine/conformance/tests/mysim_dispatch.rs` | All eight dispatch portions; vector 06 MySim state/sound is outside this owner |
| 12.27 MS19 vectors 01–05 | Exact equality, teleport and state-1 request implemented | Five named `ms19_01`–`ms19_05` executable conformance tests |
| 12.28 MS18 vectors 01–05 | Verified geometry and stop contracts implemented independently of unsettled route selection | Five named `ms18_01`–`ms18_05` executable conformance tests, plus three switch/group/tail tests |
| 12.6, 12.21 arithmetic/search | Existing job-search and arithmetic vectors retained | `ms12_06`, `ms12_21`–`ms12_25`, `ms12_29`, `ms12_31` |
| 12.23–26 MySim commute/trip sampling | MySim-owned; no dispatch closure claim | Persistent search kept separate |

### cSC4MySimDispatch::cLocation

| Spec section | Code item or open contract | Coverage |
|---|---|---|
| 4.7.1 defaults/assignment/validation | `DispatchLocation::{default,set_by_occupant,by_occupant,is_valid}`; neighbour connection order | `t40_location_contract`, `t33_job_search_neighbor`, vectors `ms12_21`–`ms12_25` |
| 4.7.1 world/cell arithmetic | `world_to_cell`, `set_cell_position`; preserve other fields | `t42_ms13_arithmetic_and_home_rect`, `ms12_29_location_arithmetic`, `ms12_31_home_rect` |
| 4.6.1, 4.7.1 exact equality | `same_location` compares both flags, occupant, cells and regional coordinates; ignores world/timing and validity | Conformance `ms19_01_exact_identity`, `ms19_02_null_and_neighbor` |
| 4.6.2 geometric resolution | `geometry_rect`, `location_distance`; occupant lot or cell lot, fallback point, asymmetric corners | Conformance `ms18_01_linear_threshold`, `ms18_03_lot_geometry_direction` |
| 4.6.1 duration/deadline, 4.11 copied clamp | `duration`, `deadline`, `set_duration`, `set_deadline` | `ms17_03_clock_wait_and_strict_dwell`, `ms17_04_tuning_and_signed_boundaries`, `ms17_05_queue_and_deadline` |
| 6.2.1 payload and copy | `DispatchLocation::{read,write}`, value copies; refs use occupant IID | Core/conformance `ms17_07`, conformance opaque/reference roundtrip |
| 12 MS17 01/03/04/05/07; 12.21 | Four independent locations, copied queue, signed timing, no padding | Core/conformance MS17 tests plus existing arithmetic vectors |
| 12.27 MS19 01–02; 12.28 MS18 01–03 | Equality and geometry are independent contracts | Named conformance equality/geometry fixtures above |

### cSC4MySimPool

| Spec section | Code item | Coverage |
|---|---|---|
| 1.1 app/UI template owner | `MySimPool`, `MySimTemplate`, `PoolHost` | `p4_get_templates` |
| 4.12 built-in enumeration, lazy cache, output replacement | `get_mysim_templates`, `enum_keys_callback` | `p4_get_templates`, `p10_ms12_absent_properties`, `ms12_26_pool_records` |
| 4.12 import record reader/writer, portraits and byte paths | `parse_mysims_dat`, `encode_record`, `portrait_source`, `data_file_path`, `userdata_dir` | `p1_record`, `p2_display_path`, `p8_ms12_paths_and_lengths`, `ms12_26_pool_records` |
| 4.12 installation and import lifecycle/cache | `find_the_sims_installation`, `import`, `import_family` | `p5_import`, `p6_find_installation`, `p7_cache`, `ms12_27_pool_import_cache` |
| 4.12 ini reads, required texts, localized comparisons, unsigned/signed formatting | `IniFile`, `import_family`, `member_indices`, `member_key`, `member_face`, `is_male_text`, `make_lower` | `p3_gender`, `p9_ms12_ini_reads`, `p11_ms14_required_reads_and_short_names`, `ms12_30_import_branches` |
| 12.22 MS14 branches | `import_family`, `member_key`, `member_face` | `p11_ms14_required_reads_and_short_names`, `ms12_30_import_branches` |

`n = UINT_MAX` faithfully uses a wrapping, nonterminating member loop; tests
exercise signed formatting directly and finite negative bounds instead. Missing
built-in scalar properties use the spec's defined 0/false defaults and retain
indeterminate-property markers. No game assets or platform services are embedded.
