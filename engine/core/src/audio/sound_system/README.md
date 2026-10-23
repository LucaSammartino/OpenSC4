# Lower sound owners

Behavior source: `docs/specs/sound-system.md`. This extends `audio`; it reuses
`snd_event::SndEventStd`, the sound-ID tag helper and existing director identities.
It does not add a second SC4 event handler, ambience manager, device or decoder.
There are no plugin/C ABI changes and no live-playback save format.

Foreign traits supply acquired sound/service references, backend creation and QI,
INI IO/conversion, persist records, resource serializers, converted playlist lines,
decimal-long conversion, scheduling, posting and factory/framework operations.
Arc clones own exactly the references they acquire; removal/drop releases those
references. `InterfaceView` retains the owner and reports secondary-interface
offset metadata rather than treating a secondary pointer as a primary table.
Native adapters must balance native references and framework service QI.

Owner state uses ordinary exclusive Rust mutation; shared freshness state uses
`Arc<Mutex<_>>` and callbacks use `try_lock`. No getter state uses `RefCell`.
All numeric property words preserve their 32-bit representation. Missing register
values and unknown constructor class IDs stay absent rather than acquiring guessed
defaults. Source creation safely rejects unavailable dependencies, as allowed by §4.

The source spec deliberately leaves SS1–SS11 unsettled. Public operations that
would cross those gaps return `Unsupported("SSn")`. Confirmed continuations are
separate: `prepare_play` exposes restart/routing requirements; `write_play_payload`
records the four notified writes; channel `*_after_patch` methods implement the
confirmed IO after a successful patch refresh; `register_classified_target` and
`enumerate_targets` operate already-classified lists without inventing a period
classifier; `parse_converted` appends without claiming Open's reset/append policy.
These methods do not establish that the unresolved prerequisite succeeded.

SS12 records literal list choices: preserve registrations, remove all matching
pointers, drain target changes in request order (handler additions precede removals
as required), and join ordinary relative paths using the base's separator style.
UNC, dot/parent, URL and unresolved format paths explicitly return SS11.

## Owner closure

Tests below are in `tests.rs`; section numbers refer to `sound-system.md`.
“Open” is limited to the uncertainties explicitly reserved by the source spec;
inventory names alone are not claimed as algorithms. All 24 §10 vectors execute.

| Owner | Spec section | Code item and covering test; explicit open boundary |
|---|---|---|
| All interface/service owners | §2 | `SoundRef`, `InterfaceView`, foreign host traits; `track_views_acquire_references_and_preserve_secondary_offsets`, `ss_v08_free_channel_releases_once`; native COM/system-service inheritance stays in adapters |
| cHitTrackPlayer | §3.1 | `HitTrackPlayer::construct`, `query_interface`, embedded `HitChannel`; `track_construction_acquires_services_without_loading`, `track_views_acquire_references_and_preserve_secondary_offsets`; open: SS1 folded/interpreter entries |
| cHitTrackPlayer | §3.2 | `load`, `unload`, `is_playing`, `pause`, `unpause`, `prepare_play`, `play_one`, `write_play_payload`; SS-V01/02/04/05 and `track_pause_properties_and_payload_notifications`; open: SS1 restart Kill/start and argument routing |
| cHitTrackPlayer | §3.3 | property-backed getters, `checked_reg_id`, `execute_command`; SS-V03 and `track_pause_properties_and_payload_notifications`; open: SS1 opcode widths, notes, notifications' downstream interpreter effects, timers and recovery |
| cHitMan | §4.1 | `HitMan::default`, `init`, `alloc_channel`, `free_channel`; SS-V08 and `hit_defaults_and_initialization_services`; open: SS2 file grammar, maps/registration rules, semantics/arbitration, random law, pause, callbacks and shutdown |
| cHitChannel | §4.2 | `HitChannel::new`, `create_sound`, `load_after_patch`, `note_on_after_patch`; SS-V06/07 and `channel_creation_requires_source_qi_and_selects_3d`; open: SS3 complete tables, patch selection, pitch/pan laws and notification dispatch |
| cHitIniFile | §5.1 | `HitIniFile`, `IniFile`, `enum_default_entries`; `ini_delegation_defaults_and_missing_services`; open: SS4 complete sections, type/group token grammar, ordering, duplicates and failure aggregation; no duplicate INI parser |
| cHitPropertySetTrackResource | §5.2 | `ResourceKey`, `HitPropertySetTrackResource`, `TrackResourceIo`, `ResourceFormat`, `LEGACY_PREFIX`; SS-V24; open: SS5 serializers, malformed input, assignment/private-copy ownership and alternate tables |
| cFreshPlayer | §6.1 | `FreshPlayer`, `ScoreInput`, `init_resource`, `init_loaded_score`, `play`, setters, `update_volume`, busy count, `try_callback`; SS-V09, `fresh_initialization_play_and_lock_contention`, `score_resource_failure_does_not_initialize`; open: SS6 dimension acceptance, overloads, score selection, timing, fading and stop/shutdown |
| cFreshCellPlayer | §6.2 | `FreshCellPlayer`, `CellHost`, state predicates, `set_cell`, `kill`, `stop`, `update_source`, two-stage wrapping volume arithmetic and forwarding; SS-V10–14 and `cell_stop_fade_poll_and_scaled_forwarding`; open: SS7 start/spatial policies, external controller and countdown branches |
| cGZSndSys | §7.1 | `SndSys`, consuming/deferred event chain, posting, classified target lists/queues, independent update control, renderer master volume; SS-V15–17, `deferred_handlers_survive_nested_dispatch_and_drain_adds_first`, `target_queues_and_callback_short_circuit`, `target_removal_is_deferred_through_nested_enumeration`, `renderer_volume_and_posting_are_foreign_io`, `payload_overload_reuses_existing_event_fields`; open: SS8 period classifier, slow timer, listener/cache/lifecycle/mapping rules; literal choices SS12 |
| cGZSndMgr | §7.2 | `SndMgr::init`, system-sound mapping, manager interface view, existing tag helper; SS-V18/19 and `manager_successful_init_registers_once`; open: SS9 constructor defaults, creators/cache/autoplay and shutdown |
| cGZSoundCOMDirector | §8 | `SoundDirector::construct`, `on_start`, `pre_app_shutdown`, absent-system `post_app_init`; `director_factories_null_paths_and_panic`; open: SS10 present-system cache operation and inherited/aggregation contracts |
| GZSndSysMacCOMDirector | §8 | `MacSoundDirector::construct`, `on_start`, `MAC_CLASSES`; `director_factories_null_paths_and_panic`; open: SS10 inherited/aggregation contracts |
| cFreshnessCOMDirector | §8 | `FRESH_CLASSES`, `get_freshness_director`, alternate entry, `OnceLock`, registration/exit-cleanup callback; SS-V20; open: SS10 inherited/aggregation contracts |
| cGZPlayList | §9 | `select_format`, `parse_converted`, `is_drive_spec`, `make_absolute_path`; SS-V21–23 and `playlist_absolute_paths_and_unsettled_formats`; open: SS11 Open IO/reset policy, title storage, plain comments/URLs, PLS and complex paths/encoding; literal simple-path joining SS12 |

## Conformance vectors

| §10 vector | Executable test |
|---|---|
| SS-V01 | `ss_v01_nested_loads_ignore_child_failure` |
| SS-V02 | `ss_v02_unload_underflows` |
| SS-V03 | `ss_v03_signed_register_check` |
| SS-V04 | `ss_v04_dependents_outlive_execution` |
| SS-V05 | `ss_v05_repeat_play_does_not_restart` |
| SS-V06 | `ss_v06_channel_note_on_loop` |
| SS-V07 | `ss_v07_channel_failed_load_never_plays` |
| SS-V08 | `ss_v08_free_channel_releases_once` |
| SS-V09 | `ss_v09_uninitialized_freshness_success_without_work` |
| SS-V10 | `ss_v10_cell_state_predicates` |
| SS-V11 | `ss_v11_kill_releases_sound` |
| SS-V12 | `ss_v12_scaled_unity` |
| SS-V13 | `ss_v13_scaled_fraction` |
| SS-V14 | `ss_v14_scaled_signed_truncation` |
| SS-V15 | `ss_v15_event_chain_consumes` |
| SS-V16 | `ss_v16_unconsumed_event_is_false` |
| SS-V17 | `ss_v17_updates_can_be_disabled` |
| SS-V18 | `ss_v18_manager_preserves_partial_init` |
| SS-V19 | `ss_v19_missing_system_sound_is_zero` |
| SS-V20 | `ss_v20_freshness_director_singleton` |
| SS-V21 | `ss_v21_extended_eof_between_records_succeeds` |
| SS-V22 | `ss_v22_extended_bad_record_keeps_previous_entries` |
| SS-V23 | `ss_v23_plain_keeps_first_line` |
| SS-V24 | `ss_v24_resource_write_and_read_are_asymmetric` |
