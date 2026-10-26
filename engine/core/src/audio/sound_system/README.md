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

Higher sound owners' existing `Unsupported("SSn")` boundaries are listed below.
Their continuations include `prepare_play`, `write_play_payload` and channel
`*_after_patch` methods; these do not establish that a prerequisite succeeded.
SS1–SS7's confirmed rules now execute in the core, with worked cases in
`closure_tests.rs`. Track Play writes suppress notifications, channel patch
refresh safely rejects missing backend/QI dependencies, and INI stages preserve
numeric replacement/order and partial success. Foreign interfaces supply COM
calls, inherited INI enumeration/locale comparison, clocks, random draws and the
pitch table; they do not supply the selection, transform or framing algorithms.
The exact remaining SS1 opcode set returns `Unsupported("SS1")`, checked for
every reserved opcode. Runtime-only boundaries are listed below; none is
silently treated as a completed algorithm.
The other owners' closure rows retain their previous scope pending their lanes.

SS8–SS12 implement the specified system, manager, director and playlist contracts.
Target queues drain removals, ordinary additions, then slow additions; handlers
replay pending front insertions before removals. Registrations replace rather than
duplicate active entries. Playlist paths use literal backslash segments without
inserting separators; Open appends and restores raw input before parsing plain M3U.
SS11 still reserves locale output bytes and unchecked IO failures for observation.

## Owner closure

Tests below are in `tests.rs` and `closure_tests.rs`; section numbers refer to `sound-system.md`.
SS-V tests are in `tests.rs`; SS1–SS7 worked cases are in `closure_tests.rs`.
Section numbers refer to `sound-system.md`. “Implemented” applies only to rows
without open IDs; an owner with any open row remains partial.
“Open” is limited to the uncertainties explicitly reserved by the source spec;
inventory names alone are not claimed as algorithms. All 24 §10 vectors execute.

Requested owner credit: cHitChannel (4,814 B), cFreshPlayer (7,845 B) and
cFreshCellPlayer (3,047 B) are implemented with no open IDs. cHitTrackPlayer
(16,012 B), cHitMan (6,664 B), cHitIniFile (5,551 B) and
cHitPropertySetTrackResource (6,092 B) remain partial only at the explicitly
reserved SS1/SS2/SS4/SS5 observations below. The spec narrows those boundaries
to runtime measurements; its worked cases do not close them.

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
| cGZSndSys | §§7.1–7.1.3 | **implemented; SS8, SS12 closed.** `framework.rs`: `SndSys::{update_targets,drain_targets,do_event,init,shutdown}`, extension/listener/cache and renderer operations; SS-V15–17; `closure_tests.rs`: `ss8_period_boundary_and_ss12_category_replay`, `ss12_handler_replacement_and_deferred_front_order`, `ss8_ss12_queues_retain_until_replay_and_shutdown_releases_active_categories`, `ss8_disabled_failed_and_running_init_cache_ownership_shutdown`, `ss8_extension_seed_rule_renderer_identity_and_listener_words`. Native scheduling and factory outcomes are foreign inputs, including failed starts/caches tested here. |
| cGZSndMgr | §7.2 | **implemented; SS9 closed.** `framework.rs`: `SndMgr` defaults, first-registration registries, creator resolution, resource forwarding, autoplay and shutdown/drop ownership; SS-V18/19; `closure_tests.rs`: `ss9_lookup_gate_cache_rejection_and_ordered_uncached_creators`, `ss9_autoplay_failed_play_front_only_cleanup_and_source_survival`, `ss9_resource_arguments_and_playing_front_blocking`. |
| cGZSoundCOMDirector | §8 | **implemented; SS10 closed.** `directors.rs`: `SoundDirector`, shared `DirectorState` dispatch/factories, `post_app_init` CreateInstanceSet, explicit `NativeFault` boundaries; `closure_tests.rs`: `ss10_all_directors_inherited_identity_and_factory_worked_case`, `ss10_children_precede_local_factories_and_successful_qi_transfers_output`, `ss10_post_init_retains_cache_ignores_false_and_distinguishes_nulls`. |
| GZSndSysMacCOMDirector | §8 | **implemented; SS10 closed.** `directors.rs`: `MacSoundDirector`, `MAC_CLASSES`, `MAC_CONSTRUCTION_LAYOUTS`, shared `DirectorState`; `tests.rs`: `director_factories_null_paths_and_panic`; `closure_tests.rs`: `ss10_all_directors_inherited_identity_and_factory_worked_case`, `ss10_children_precede_local_factories_and_successful_qi_transfers_output`. |
| cFreshnessCOMDirector | §8 | **implemented; SS10 closed.** `directors.rs`: `get_freshness_director`, alternate entry, `OnceLock`/exit-cleanup registration, shared `DirectorState`; SS-V20; `closure_tests.rs`: `ss10_all_directors_inherited_identity_and_factory_worked_case`, `ss10_children_precede_local_factories_and_successful_qi_transfers_output`. |
| cGZPlayList | §§9.1–9.4 | **partial; SS12 closed; open: SS11 runtime observations.** `playlist.rs`: `PlayList::{open,get_line,parse_records,parse_pls,make_absolute_path}`; SS-V21–23; `closure_tests.rs`: `ss11_open_appends_plain_comments_urls_and_keeps_entries_on_io_failure`, `ss11_plain_open_restores_raw_input_and_reconverts_first_line`, `ss11_extended_titles_partial_failure_and_pls_worked_cases`, `ss11_line_cursor_repeats_unterminated_tail_and_ignores_conversion_status`, `ss11_literal_backslash_paths_and_ss12_no_separator_insertion`, `ss11_reserved_runtime_io_boundaries_remain_explicit`. Observe original title/path bytes and Open results under recorded locale for 0x80/0xE9/invalid multibyte and UTF-8/UTF-16 BOMs, plus zero-byte successful Read and injected allocation-failure outcomes (§9.1). These cases remain explicit `Unsupported` boundaries, not closed behavior. |
| cHitTrackPlayer | §3.1 | `construct`, `query_interface`, embedded channel; `track_construction_acquires_services_without_loading`, `track_views_acquire_references_and_preserve_secondary_offsets`; open: SS1 folded entries need the §3.3 recording-interface/PC observations |
| cHitTrackPlayer | §3.2 | implemented: `play_with`, `write_play_payload`, load/pause methods; SS-V01/02/04/05, `ss1_mode3_and_play_order_suppress_notifications`; SS1 routing, suppressed flags and Kill/start/dependent/Unpause call order applied; actual foreign calls use `TrackIo` |
| cHitTrackPlayer | §3.3 | `set_register`, `notify_register`, `execute_next`, `timer_callback`; `ss1_byte_width_wait_and_strict_ramp_worked_cases`, `ss1_instruction_budget_reports_after_102_and_preserves_stack_offsets`, `ss1_alias_suppression_checked_register_and_callback_countdown`, `ss1_every_reserved_opcode_remains_explicitly_unsupported`; open: SS1 residual opcodes 0B/0E/10–19/1B–1C/1F–33/3E–49/4B–5C/5F–61, branch/tempo/error continuation and cross-object effects require the exact §3.3 PC/register/interface traces; alias words supplied by adapter |
| cHitMan | §4.1 | `TrackDataBlock::parse`, registration/semantic/group/sequence/duck maps, `random`, pause slots and opaque source forwarding; SS-V08, `ss2_whole_file_duplicate_registration_semantic_duck_and_random`; open: SS2 source payload/control-object policy need slot-3 handler and competing-group recording observations (§4.1); shutdown/callback ordering remains a SPEC-GAP |
| cHitChannel | §4.2 | implemented (SS3): six QI views/table extents, `update_patch`, `update_pitch`, `update_vol_pan`, `notify`, `notify_inherited`, load/note-on; SS-V06/07, `ss3_pitch_pan_volume_patch_and_interface_worked_cases`, `ss3_notifications_preserve_return_asymmetry_and_retained_sets`; safe missing-backend rejection, optional controls and inherited-set ownership covered |
| cHitIniFile | §5.1 | `hit_config.rs` numeric conversion/tokenization and all ordered stages, persistent exclusions/creators, foreign inherited enumeration; `ss4_numeric_order_duplicates_and_failure_continuation`; open: SS4 repeated-section callbacks and locale true/TRUE comparison require the §5.1 inherited-reader observation; missing-backend crashes safely rejected |
| cHitPropertySetTrackResource | §5.2 | legacy framing/version/overlay/defaults/writer, `assign`, `private_copy`, foreign generic serializer selector; SS-V24, `ss5_legacy_framing_defaults_self_reference_writer_and_private_copy`, `ss5_optional_length_magic_skip_and_empty_field_overlay`, `ss5_version_gates_overlay_and_unvalidated_text_tags`; open: SS5 invalid QI, S=0/3 and short-read return/crash/property outcomes need the §5.2 mock-record observation; safe invalid-input rejection used |
| cFreshPlayer | §6.1 | implemented (SS6): initialization/overloads, valid dimensions, selection/scoring, coordinates/budgets/divisions, beat clock, fades, Stop/Shutdown/reseed and lock callbacks; SS-V09, `ss6_block_coordinate_selection_scores_and_separate_random_sources`, `ss6_fade_overshoot_retarget_zero_step_and_clock_worked_cases`, `ss6_stop_shutdown_and_repeated_init_lifetime_order`, `ss6_path_init_ignores_load_boolean_and_preserves_directory`; invalid dimensions/sparse grids safely rejected |
| cFreshCellPlayer | §6.2 | implemented (SS7): `start`, GUID/name selection, spatial/pitch/volume setup, external controller call order/failure state, `update_countdown`, `update`, Stop/Kill and AutoPan; SS-V10–14, `ss7_guid_external_failure_countdown_pan_and_controller_worked_cases`, `ss7_internal_start_ignored_play_failure_and_external_setter_order`; foreign CRC/table/draw inputs explicit; low-bit pan polarity marked SPEC-GAP |
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
