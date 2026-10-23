# Portable audio ambience manager

`AmbienceManager` owns the city-side `cSC4AudioAmbienceManager` behaviour from
`docs/specs/audio-ambience.md`. `AmbienceHost` supplies foreign services and typed
snapshots, and `AmbienceLayer` supplies the layer interface. No plugin or C ABI
is wired. `audio::ambience` re-exports this module for existing callers.

The core owns lists and values; hosts own native services, references and the
city filter. Hosts resolve QI into the typed optional inputs, release native
temporary references, and return owned lists. Native pointers are represented
by stable 32-bit IDs; occupant-plus-sound IDs wrap at 32 bits. The host samples
elapsed milliseconds before `on_tick`; the owner requests timer restart only
on enabled ticks. A budget closure controls subsequent blocks; the first block
always makes progress. No `RefCell` occurs in owner state (test spies use one).

Compatibility choices: city shutdown preserves alarms/tracked occupants, as
specified and required by §10.24; missing/short margins use the spec's safe 70.0
policy; invalid views preserve rectangles; absent preferences yield volume 0.
The process-wide first-rectangle flag belongs to the host, not the manager.
AA10 is explicitly `Unsupported::AlarmCreationWithoutClock`, with no invented
alarm entry. AA12 (world-space `BeginUpdate`) and AA13 (first occupant/kind alarm
removal) record literal interpretations where the spec needs clarification.

## Owner closure

Each row is for **cSC4AudioAmbienceManager**; layer algorithms remain foreign.
Test names below are in `tests.rs`.

| Spec section | Code item | Covering test / status |
|---|---|---|
| §1 Object | `new`, constants, `Stage`, `AlarmEntry` | `constructor_values`, `instance_id_wraps` |
| §2.1 Init | `init`, `SUBSCRIBED_MESSAGES` | `init_subscribes_twelve_in_order`, `init_failures_return_false`, `every_init_failure_short_circuits_in_order`, `lifecycle_hooks_and_process_first_rectangle` |
| §2.2 City init | `handle_city_init`, host filter/layer factory | `city_init_creates_layers_and_drops_failures`, `preference_clear_gives_zero_volume`, `preferences_are_read_for_each_successful_layer_only`, `init_walk_starts_no_sound` |
| §2.3 City shutdown | `handle_city_shutdown`, `release_layers` | `city_shutdown_stops_ticking_and_clears`, `lifecycle_hooks_and_process_first_rectangle` |
| §2.4 Shutdown | `shutdown`, Rust layer drop after `shutdown` | `shutdown_unsubscribes_same_set_and_releases` |
| §3 Sound helpers | `add_occupant_snd_instance`, `remove_occupant_snd_instance` | `snd_instance_needs_flags`, `null_sound_occupants_are_rejected`, `preview_occupant_is_silent` |
| §3.1 Insert/init walk | `handle_occupant_common` | `insert_move_remove_sounds`, `init_walk_starts_no_sound` |
| §3.2 Move | `handle_move` | `move_posts_are_independent_and_need_sound_system` |
| §3.3 Remove | `handle_remove`, `remove_alarm` | `remove_drops_matching_alarm_only`, `duplicate_alarm_removal_uses_first_kind_match`; clarification AA13 |
| §3.4 Conditions/state | `update_lot_condition`, `do_message` | `lot_condition_update_rows`, `condition_changed_needs_lot`, `state_changed_does_nothing` |
| §3.5 Automata/crossing | `handle_automaton`, `play_occupant_sound`, `handle_crossing` | `automata_codes`, `railway_crossing` |
| §3.6 Alarms | `add_alarm`, `update_alarms`, `time_of_day` | `alarm_created_at_current_hour`, `alarm_creation_flags_and_order`, `time_of_day_drops_minutes`, `on_off_alarm_22_to_6`, `on_off_rereads_sound_and_preserves_state_without_property`; open: AA10 only for absent creation clock (`no_clock_no_alarms`) |
| §4.1 Tuning | `read_tuning` | `missing_margins_default_to_70`, `tuning_vectors_replace_pad_and_retain_ids` |
| §4.2 Attention | `update_attention_rect` | `attention_rect_clamps_and_rounds_even`, `invalid_view_keeps_rectangles_and_null_event_is_unhandled`, `extra_margins_do_not_admit_non_city_zoom`, `lifecycle_hooks_and_process_first_rectangle` |
| §5 Tick/block walk | `on_tick`, `do_stage_update`, `seed_iterator`, shared `CellIterator` | `stage_update_cell_order_and_stages`, `zero_budget_still_finishes_every_block_before_end`, `budget_spreads_walk_over_ticks`, `prime_step_block_order`, `iterator_current_carries_over`, `ticking_disabled_does_nothing`; clarification AA12 |
| §6 Events/focus | `do_event`, `do_optional_event`, `do_message(AppFocus)` | `events_master_volume_and_fresh_player`, `invalid_view_keeps_rectangles_and_null_event_is_unhandled`, `focus_pauses_layers_and_exemplar_rereads_tuning` |
| §7 Dispatch | `do_message`, `AmbienceMessage` | `non_payload_message_returns_zero`, occupant/condition/automaton/focus tests |
| §8 Manager calls | `AmbienceLayer`, layer factory/drop | `city_init_creates_layers_and_drops_failures`, `zero_budget_still_finishes_every_block_before_end`, `stage_update_cell_order_and_stages`, `layer_receives_world_rectangle_and_exact_listener_vectors` |
| §§8.1–8.7 Layer internals | Foreign `cSC4AudioAmbienceLayer`, outside named owner | Not implemented or claimed by this owner |
| §9 Open items | `Unsupported` | open: AA10 (absent-clock creation only); AA4/AA11 incorporated |
| §10 Conformance | Tests mapped below | Every manager vector covered; layer vectors delegated |

## Conformance mapping

| §10 vector | Test |
|---|---|
| 1 | `missing_margins_default_to_70` |
| 2 | `init_subscribes_twelve_in_order` |
| 3 | `time_of_day_drops_minutes` |
| 4 | `on_off_alarm_22_to_6` |
| 5 | `preview_occupant_is_silent` |
| 6 | `stage_update_cell_order_and_stages` |
| 7–8 | `lot_condition_update_rows` |
| 9–18 | `cSC4AudioAmbienceLayer` algorithms, foreign owner (§§8.1–8.7) |
| 19 | `alarm_created_at_current_hour` |
| 20 | `insert_move_remove_sounds`, `move_posts_are_independent_and_need_sound_system` |
| 21 | `railway_crossing` |
| 22 | `prime_step_block_order` |
| 23 | `focus_pauses_layers_and_exemplar_rereads_tuning` |
| 24 | `events_master_volume_and_fresh_player`, `city_shutdown_stops_ticking_and_clears` |
