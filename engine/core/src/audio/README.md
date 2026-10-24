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
| COMDirector, audio §§1.1,11 | Existing identity/order records; **pending executable lifetime** | Existing fixtures |
| AEMSEventSystem / AEMSSoundBank, audio §10.7 | Existing data/header model; **pending owner lifetime/IO** | Existing fixtures; AU1 open for bank magic only |
| EmanationModel2D / 3D / 3DEffects, audio §§3.4,19 | **Pending settled interface/defaults**; transforms open: AU41 | AU9-06 pending |
| SemManager, audio §19 | **Pending settled interface/teardown**; construction/rollback open: AU41 | Owner tests pending |
| Listener, audio §18 | **Pending settled dispatch/zoom/notification**; geometry/failure lifetime open: AU40 | AU9-01–05 pending |
| ScapeManager, audio §20 | **Pending settled interface/local cleanup**; full groups/resources open: AU42 | AU9-07–09 pending |
| EventHandlerDisasterFire, audio §7.4/20.3 | **Pending settled lifetime IO**; full owner open: AU44 | Owner tests pending |
| PreferencesWinProc, audio §21 | **Pending settled slider/notification IO**; full control/song rules open: AU43 | AU9-10–15 pending |
| 83 leaf classes, audio §11 | Existing measured shapes/QI/predicates; slot 5 open: AU5 | Existing leaf tests |

## Lower sound-system owners

The [sound-system owner table](sound_system/README.md) maps the separate HIT,
freshness, framework sound, director and playlist contracts to code/tests and
SS1–SS11 open questions.
