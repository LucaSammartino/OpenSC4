<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Audio core

SC4 family closure is in progress. The event handler now implements the extended
audio.md §7 algorithms; the remaining family owners are listed explicitly below.
No plugin, C ABI, audio device or game data is embedded in this module.

## Foreign boundary

Use `AudioEventHandler::construct`, `init_with`, `do_message`, `do_modal_message`,
`do_event`, `shutdown_with` and `destroy_with`. `EventHandlerHost` supplies single
foreign slot operations; branching, selection, subscriptions and state stay in
core. Payload and position QIs have explicit releases. Pointer transfer consumes
the supplied object reference without releasing the message. Position event words
are truncated signed integers. Modal mouse handling forwards its specified false
return; other non-null main messages return one. Unchecked inputs are diagnosed
with assertions, never fabricated successful requests.

The old `new/init/handle_play/handle_stop` APIs remain isolated fixture helpers;
they do not acquire foreign services. Torch history is process-global behind a
mutex; `do_event_with_torch` accepts shared process state for deterministic replay.
Destruction releases services but does not Shutdown, unsubscribe, unregister,
stop sounds or release the spec's leaked raw modal-window references.

## Owner closure

Tests are in `event_handler/owner_tests.rs` and `event_handler/closure_tests.rs`
unless qualified. Resolved AU13–AU16 diagnostic enum variants remain for source
compatibility, but the extended algorithms no longer emit them. AU45 records the
completed dispatch table's conflict with the older four-operation fixture list.

| Owner / spec section | Code item / status | Test |
|---|---|---|
| EventHandler §1.2–1.3, §7.1 | identity constants, `construct`, zero words, selector 4 | `owner_identity_and_interfaces`; `construction_registration_and_shutdown_preserve_city_set` |
| EventHandler §7.1.1 | `init_with`, `shutdown_with`: partial registration, retries, optional filesystem, preferences, reference balancing | `service_failures_preserve_registration_and_constructor_members`; `lifecycle_retries_optional_filesystem_and_shutdown_failure` |
| EventHandler §7.2, §6.4, §13 | `message_route`, `do_message`, city subscriptions, ignored types; clarification AU45 | `complete_dispatch_table_has_only_four_automata_types`; `extended_routes_execute_without_algorithm_gaps`; `subscribed_delivery_only_hears_automata_in_city_but_direct_posts_work` |
| EventHandler §7.2.1 | `handle_radio`, `handle_settings`, EC4/EC5 queueing, transferred reference consumption | `station_defaults_explicit_fade_and_same_station`; `transferred_reference_and_radio_control_rules`; `message_return_values_radio_slots_and_deferred_station_message` |
| EventHandler §7.2.2 | `handle_dispatch`, event creation/submission/release and capability-only notifications | `dispatch_categories_and_capability_notifications` |
| EventHandler §7.2.3 | `do_modal_message`, LIFO stack, transition-only subscriptions | `modal_stack_transitions_and_inclusive_mouse_bounds` |
| EventHandler §7.2.4 | `handle_modal_mouse`, inclusive bounds, forwarded false, QI/window balancing | `modal_stack_transitions_and_inclusive_mouse_bounds` |
| EventHandler §7.3 | `handle_automata`, packed start/position/stop words, no-argument getters, failed-QI rejection | `packed_play_truncates_and_releases_after_both_posts`; `play_stop_position_and_payload_release_follow_spec_order`; `entry_failures_balance_only_successful_queries_and_main_still_returns_one` |
| EventHandler §7.3.1 | `alternate_automata`, selector matrix and global ID IO; failed direct QI position remains explicitly indeterminate | `alternate_automata_matrix_and_indeterminate_position` |
| EventHandler §7.3.2 | `handle_construction`, slots 13/14/15, positional QI chain | `construction_matrix_and_acquired_cleanup` |
| EventHandler §7.5, §9.5 | `do_event`, `enumerate_callback`, `handle_torch`, shared `TorchState`, start/stop/volume helpers | `callback_return_qi_independence_and_destructor_not_shutdown`; `enumeration_callback_preserves_the_property_and_balances_qi`; `torch_shared_history_volume_gate_wrap_and_sticky_mode` |
| EventHandler §7.6 | `destroy_with`, service destruction and intentional raw-window leak | `callback_return_qi_independence_and_destructor_not_shutdown` |
| EventHandler §8.7, §12, §14.11 | sound-system chain, no save record | `full_chain_crosses_handler_and_sound_system_without_a_save_record` |
| EventHandler §14.5, §14.13 | shared `RateLimiter`, inclusive window | `limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold` |
| EventHandler §16.12–13,17,21,25–42 | handler vectors mapped to rows above; AU45 qualifies vector 12 | owner/closure tests |
| AmbienceManager, audio-ambience §§1–7,10 | Existing portable owner | [Owner table](../audio_ambience/README.md); AA10 open |
| AmbienceLayer, audio-ambience §8, §10.9–18 | **Pending implementation**; existing manager uses a foreign layer | Family closure still in progress |
| SoundManager, audio §8 | Existing instance-table fixture; **pending complete owner execution** | Existing tests; foreign owner tests needed |
| COMDirector, audio §§1.1,1.2,11 | `director::on_start`, `register_classes`, identity/class tables | `executable_startup_interleaves_add_start_and_registers_92_classes`; existing identity/order tests |
| COMDirector, audio §1.1/1.3 application lifetime | open: AU47 service creation/failure/unwind and teardown, constructor interleaving | Measured class set and add/start sequence implemented; no full lifetime claim |
| AEMSSoundBank, audio §10.7 raw file/resource IO | `aems::AemsSoundBank::read_file/read_resource`, twice-size/raw read, cached TGI | `bank_raw_slurp_calls_size_twice_and_never_checks_magic`; `resource_cache_and_success_only_bank_linkage` |
| AEMSEventSystem, audio §§10.7–10.8 settled state/linkage | `aems::AemsEventSystem` service ID/priority, known tuning defaults, successful bank linkage continuation; `data` bank/header layout | `resource_cache_and_success_only_bank_linkage`; existing `data` header tests |
| AEMSEventSystem / AEMSSoundBank, remaining §10.7–10.8 inventory/lifetime | open: AU48 full owner dispatch/lifetime/failure/allocator and tuning error policy; AU1 bank magic | No invented StopAllSounds/registrar success or native error return |
| EmanationModel2D, audio §3.4 / contracts §19.1 slots 0–4 | `emanation::EmanationModel`, `ModelKind::TwoD`: constructor, QI views/AddRef/Release, ID/capability | `emanation::tests::au9_06_constructors_and_shared_interfaces` |
| EmanationModel3D, audio §3.4 / contracts §19.1 slots 0–4 | `emanation::EmanationModel`, `ModelKind::ThreeD`: constructor, QI views/AddRef/Release, ID/capability | same AU9-06 test |
| EmanationModel3DEffects, audio §3.4 / contracts §19.1 slots 0–4 | `emanation::EmanationModel`, `ModelKind::ThreeDEffects`: constructor, QI views/AddRef/Release, ID/capability | same AU9-06 test |
| Each emanation model, contracts §19.1 slots 5–6 | open: AU41 numeric transforms/companion | No fabricated numerical result |
| SemManager, contracts §19 QI/reference/destructor/teardown | `emanation::SemManager`, retained references, lifecycle QI balancing and unregistration | `sem_teardown_queries_balances_and_destruction_only_releases` |
| SemManager, contracts §19 construction/registration | open: AU41 constructor/defaults, registration rollback/caller order | Not exposed as replacement initialization |
| Listener, contracts §18.1–18.2 settled stages | `listener::AudioListener` constructor/QI/Init/Shutdown/city subscriptions/dispatch | `au9_01_02_dispatch_city_lifetime_and_explicit_geometry_gap` |
| Listener, contracts §18.3 output/zoom, §21.4 AU9-03–05 | `calculate_zoom`, `publish_geometry`, `tuning_recalculated`; measured-geometry continuation only | `au9_03_04_threshold_order_and_live_pointer_payload`; `au9_05_zoom_defaults_and_no_intermediate_store` |
| Listener, contracts §18.1–18.3 unsettled stages | open: AU40 camera geometry, tuning array decoding/fallback, cheat, service/allocation/delivery/final lifetime | Dispatcher reports AU40; no claim of full update closure |
| ScapeManager, contracts §20.1 constructor/QI/service values | `scape::ScapeManager`, factory service view, defaults/QI, priority/enabled fields | `au9_07_08_factory_qi_duplicate_and_mask_contracts` |
| ScapeManager, contracts §20.1 group index/missing-key rules | `register_group`, `remove_group`, `matching_groups`, `missing_group_instance`; resource creation remains separate | `au9_07_08_factory_qi_duplicate_and_mask_contracts`; `au9_09_removal_without_sound_system_still_cleans_local_state` |
| ScapeManager, contracts §20.2 lifetime/local instance cleanup | `initialize`, `shutdown`, `remove_instance`, explicit `ScapeGlobals` acquired reference | `scape_service_initialization_and_reference_balance`; AU9-09 test |
| ScapeManager, contracts §20.1–20.2 remaining slots/resources | open: AU42 group format/allocation/resource ownership, parameter smoothing, tick/timer widths, service getters/complete destruction | Local index is not a full group resource implementation |
| EventHandlerDisasterFire, audio §7.4 / contracts §20.3 | `scape::DisasterFireHandler` idempotent Init, subscriptions/acquisition, Shutdown, nonnull occupant forwarding | `fire_requires_server_and_forwards_only_nonnull_occupants` |
| EventHandlerDisasterFire, remaining §20.3 inventory | open: AU44 construction/defaults/QI/message dispatch/removal/full lifetime | Torch belongs to EventHandler, not this owner |
| PreferencesWinProc, contracts §21.1 | Retained window/preferences state; open: AU43 constructor, QI/full dialog/song inventory | No guessed defaults |
| PreferencesWinProc, contracts §21.2 | `preferences::PreferencesWinProc` preference copy, sound-system acquisition/event/message, EndDialog/Shutdown | `au9_10_service_failure_still_posts_and_update_copies`; `au9_13_14_15_selection_bounds_escape_and_window_lifetime` |
| PreferencesWinProc, contracts §21.3–21.4 | `slider_volume`, accepted window/enable gates, Escape/special control, 30-entry arrays | `au9_11_12_slider_endpoints_offsets_and_banks`; `au9_13_14_15_selection_bounds_escape_and_window_lifetime` |
| PreferencesWinProc, contracts §21.3 remaining control matrix | open: AU43 complete type-3 filter matrix/control actions and dialog/song rules | Explicit `PreferencesOpen::Au43` |
| 83 leaf classes, audio §11 | Existing measured shapes/QI/predicates; slot 5 open: AU5 | Existing leaf tests |

## Lower sound-system owners

The [sound-system owner table](sound_system/README.md) maps the separate HIT,
freshness, framework sound, director and playlist contracts to code/tests and
SS1–SS11 open questions.
