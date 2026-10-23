<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Audio core

The existing audio module models messages, sound events/pools, automaton SFX,
sound managers, effect components, data, directors, and ambience. See `mod.rs`
for its module map. This report scopes owner closure to
`cSC4AudioEventHandler` in `docs/specs/audio.md`.

The handler is **not closed**: the spec names several owner algorithms without
describing their behavior. AU13–AU16 in `docs/specs/questions/audio.md` record
the missing rules. They are closure blockers, not approved exemptions inferred
from the spec's existing open-items table.

## Foreign boundary

`event_handler::EventHandlerHost` supplies sound-system, message-server and radio
operations, service acquisition, capability availability, global vector seed and
random values. `EventHandlerMessage` supplies the message/payload slots and
position lookup. No simulator, audio device, COM implementation, plugin, C ABI
or game data is embedded in the owner. The adapter owns acquired foreign service
references; successful payload QI is balanced by `release_payload` on every path.
The position adapter balances its acquired occupant/vector references. A host
must accept the message's specified slot2 operation on the radio-slot7 path;
AU13 asks which message/interface ownership that operation represents.

Use `AudioEventHandler::construct`, `init_with`, `do_message`/`deliver_main`, and
`shutdown_with` for the foreign boundary. `deliver_main` models subscription
delivery; direct `do_message` calls bypass subscriptions as a direct post does.
Every non-null main message returns 1, including sub-handler rejection; missing
algorithms also carry an explicit `HandlerGap` diagnostic. A null message returns
0. No missing construction/radio/modal algorithm silently chooses a sound.

The old `new`/`init`/`handle_play`/`handle_stop` APIs remain fixture conveniences
for existing callers and vectors. Their word-only event queue is not a complete
foreign-payload implementation: positions are zero placeholders and the stop
arguments are caller supplied. `new` and `Default` now agree. Use
`handle_automata` for the specified payload gates, slot14 argument sequences,
position lookup, sound-system posts, and reference balancing. It forwards typed
vectors to the sound-system adapter; the spec does not define wire packing.
Rate-limiter elapsed time is runtime state supplied by the caller; random values
are drawn only when the magic sound restarts its shared handler timer.

## Owner closure

Each row concerns `cSC4AudioEventHandler` only. Tests are in
`event_handler/owner_tests.rs` unless qualified. Other audio owners are outside
this table. The numbered section-16 vectors continue to exercise the legacy
runner; the owner tests below exercise the foreign boundary directly.

| Spec section | Code item / status | Test / evidence |
|---|---|---|
| §1.2 identity | `audio::CLSID_EVENT_HANDLER`, `EVENT_HANDLER_OBJECT_SIZE` | `owner_identity_and_interfaces`; `audio::tests::object_sizes_match_the_spec` |
| §1.3 service/interface IDs | `HANDLER_SERVICE_IID`, `RADIO_SERVICE`, `SOUND_SERVICE`, `PAYLOAD_IID`, `POSITION_IID` | `owner_identity_and_interfaces`; `construction_registration_and_shutdown_preserve_city_set`; `play_stop_position_and_payload_release_follow_spec_order` |
| §6.4 message contract | `message_route`, `do_message`, `deliver_main` | `complete_dispatch_table_has_only_four_automata_types`; `full_chain_crosses_handler_and_sound_system_without_a_save_record` |
| §7 entry points / callback / destructor | `do_event` reports a gap; `shutdown_with` implements specified cleanup; **open: AU16** for callback and destructor behavior | `missing_owner_algorithms_are_visible_and_do_not_emit_guessed_sounds`; `construction_registration_and_shutdown_preserve_city_set` |
| §7.1 layout / construction / defaults | `construct`, `AudioEventHandler`, `RateLimiter`; Rust collections represent the list, not a native layout; **open: AU14** for omitted field defaults, +0x18 acquisition and lifetime rules | `construction_registration_and_shutdown_preserve_city_set`; `defaults_agree_and_no_capability_is_released_without_acquisition` |
| §7.2 message table | `message_route`, `do_message`; specified radio slots and queued station-one post; **open: AU13** for construction, radio-selection, EC4 message, LAB, mouse and modal algorithms | `complete_dispatch_table_has_only_four_automata_types`; `message_return_values_radio_slots_and_deferred_station_message`; `missing_owner_algorithms_are_visible_and_do_not_emit_guessed_sounds` |
| §7.2 Init / shutdown / city subscriptions | `init_with`, `shutdown_with`, `deliver_main`, `handle_city_init_shutdown`; successful-path registration, two modal + thirteen main + eleven city subscriptions; **open: AU14** for failures/repeated lifecycle calls | `construction_registration_and_shutdown_preserve_city_set`; `service_failure_is_explicit_without_fabricated_registration`; `subscribed_delivery_only_hears_automata_in_city_but_direct_posts_work` |
| §7.3 play / stop / failure / limiter | `handle_automata`, `handle_acquired_payload`, `RateLimiter`; **open: AU14** for precondition failure returns; **open: AU15** for alternate A/B algorithms, packing and failed-vector rules | `play_stop_position_and_payload_release_follow_spec_order`; `entry_failures_balance_only_successful_queries_and_main_still_returns_one`; `limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold`; `missing_owner_algorithms_are_visible_and_do_not_emit_guessed_sounds` |
| §7.4 torch attribution | Names a separate DisasterFire owner; **open: AU16** because §9.5 instead names this owner | No implementation guessed; question records both statements |
| §9.5 torch sound bus | Names this owner's torch helpers without algorithms; **open: AU16** for attribution and sound selection/control rules | No implementation guessed; question records both statements |
| §8.7 full chain | `do_message` → `handle_automata` → foreign sound-system events | `full_chain_crosses_handler_and_sound_system_without_a_save_record` |
| §12 runtime-only / no save record | No handler serialization added; `audio::save` | `full_chain_crosses_handler_and_sound_system_without_a_save_record`; `audio::save::tests::nothing_is_saved` |
| §13 messages | `message_route`, `do_message`, `handle_automata`; **open: AU13/AU15** for listed but unspecified paths | `complete_dispatch_table_has_only_four_automata_types`; `message_return_values_radio_slots_and_deferred_station_message` |
| §14.5 handler-global limiter | `RateLimiter`, `handle_acquired_payload` | `limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold`; `event_handler::tests::one_handler_one_timer` |
| §14.11 translation through sound system | `EventHandlerHost::post_sound`, `post_position` | `full_chain_crosses_handler_and_sound_system_without_a_save_record` |
| §14.13 inclusive 4000 ms window | `rate_limit_window`, `RateLimiter::gate` | `event_handler::tests::rate_limit_window_is_inclusive`; `limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold` |
| §15 existing open items | AU8 keeps radio implementation foreign; owner-specific omissions filed as AU13–AU16 | Explicit `HandlerGap` diagnostics; no owner closure claim |
| §16.12 exact routing and city delivery | `message_route`, `deliver_main` | `complete_dispatch_table_has_only_four_automata_types`; `subscribed_delivery_only_hears_automata_in_city_but_direct_posts_work` |
| §16.13 shared limiter | `handle_acquired_payload`, `RateLimiter` | `limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold`; existing limiter tests |
| §16.17 no audio persistence (owner portion) | No handler save record; `audio::save` | `full_chain_crosses_handler_and_sound_system_without_a_save_record`; existing `audio::save::tests` |
| §16.21 sound manager receives events, not automata messages | `do_message`, `handle_automata`, foreign host | `full_chain_crosses_handler_and_sound_system_without_a_save_record` |
| §16.25 play/stop chain (owner portion) | Actual automaton posts → handler → foreign bus → manager model | `full_chain_crosses_handler_and_sound_system_without_a_save_record` |

Other §16 tests concern event/pool objects, automata, effects, data, leaf classes
or directors, rather than this owner. Their existing implementations are retained.

## Lower sound-system owners

The `sound_system` submodule extends this audio core with the confirmed HIT,
freshness, framework sound, director and playlist contracts of
`docs/specs/sound-system.md`. Its [Owner closure table](sound_system/README.md)
maps each assigned owner/section and all 24 conformance vectors to code/tests,
and identifies the spec's SS1–SS11 open algorithms. It does not change this
README's separate event-handler closure assessment.
