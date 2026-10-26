# Player-drive owner

Portable `cSC4ViewInputControlPlayerDrive`, from `docs/specs/drive-tool.md`.
The existing owner implements ordered primitive plans behind `Primitives`;
it introduces no C ABI or plugin wiring. Picks and city conversions are foreign
service operations; the mode, command and cleanup decisions belong to this core.
`Primitives` extends the existing `view_input_controls::ControlHost`; EndInput
uses its stack operation only when the inherited base view is present, as in
`city-tools-input.md` §2.7. No base cursor, capture or stack implementation is
duplicated. `Operation::RemoveTopControl` records that shared call in tests;
the player-drive owner does not emit it through its own primitive hook.

`Progress::Partial` and `Operation::Gap` expose omitted steps. Selecting a
dashboard ID does not load a window. Caller-supplied `CameraFields` do not resolve
the open selection from native drive settings/AEMS. No host may substitute a
native player-drive method for an Unsupported boundary. Getters use `&self`.

Tokens borrow identity except the explicitly acquired driven reference, which
is released only by `stop_tracking`. Hover/target ownership and reentrant
callback delivery remain open (DR3/DR6). Callback operations are plans with raw
values, not native deliveries; timer markers imply no units or cadence.

## Section closure

Every worked case below is derived from the spec. Tests are in `tests.rs`;
pre-existing literal cases are in `fixtures.md`. All question IDs refer to the
still-open `docs/specs/questions/drive-tool.md`, including its bracketed notes.

| Spec section | Code item | Worked-case test / remaining Unsupported boundary |
|---|---|---|
| 0 Summary | `mode`, `current` | `choosing_after_driving_finishes_tracking_before_pause_and_drops_hover_identity`; native dashboard/camera update remains open DR1 |
| 1 Making it | `new`, `init`, `shutdown` | `construction_has_literal_bits_and_empty_state`; Init/Shutdown services, four resource interfaces, three command/subscription IDs and native ownership Unsupported: open DR1, DR3, DR8 |
| 2 Choosing mode | `enter_choosing_mode`, `on_mouse_move`, `on_mouse_down_l`, `pick_automaton`, `pick_network_occupant` | `choosing_pauses_all_four_but_simulator_only_if_needed`, `accepted_network_pick_wins_in_choosing_and_driving`, `choosing_empty_click_preserves_pause_and_does_not_start_driving`, `hover_terrain_and_valid_drive_mouse_paths_use_individual_foreign_queries`; non-null hover ownership/text and missing drive state Unsupported: open DR3, DR6; numeric slot/group semantics open DR2 |
| 3 Driving mode | `enter_driving_mode`, `dashboard_selection`, `CameraFields`, `pause_everything` | `driving_projection_orders_raw_operations_and_resumes_conditionally`, `dashboard_selection_is_id_selection_only_and_preserves_zero_override`, `switching_driven_automata_releases_old_before_acquiring_drivable_form`, `raw_camera_values_keep_bits_and_reject_undefined_subtraction_domains`; ShowUI, three audio-table values and native settings selection Unsupported: open DR1, DR4; failure/ownership contracts open DR3 |
| 4 Leaving | `stop_tracking`, `end_input`, `automaton_removed`, inherited `ControlHost` | `stopping_state10_selects_slot13_or_slot53_using_only_published_bits`, `non10_cleanup_uses_literal_scope_without_inventing_a_state_transition`, `end_input_and_escape_order_callbacks_slot18_cleanup_and_base_stack_removal`, `end_input_without_base_view_cleans_up_without_removing_a_control`, `removed_and_notification_identity_matching_is_mandatory`; callback payload/delivery and native state-slot meaning/scope remain open DR2, DR3, DR8 |
| 5 Drive commands | `apply_numeric_command`, `command_effects`, mouse/key handlers, `is_drive_auto_valid` | `numeric_non_toggle_table_down_and_up_has_literal_sound_and_light_order`, `two_toggles_keep_independent_state_and_release_has_no_effect`, `command9_toggles_only_boolean_raw_byte_after_command_setter`, `autorepeat_suppresses_all_queries_and_escape_effects`, `ordinary_keys_never_falsely_bypass_unpublished_key_effects`, `mouse_release_and_right_button_keep_numeric_effects`; key effects, keyboard8, repeat releases and nonboolean byte9 Unsupported: open DR5; semantic command names remain open DR2 |
| 6 Dashboard | `do_win_proc_message`, `show_ui`, `update`, `attach_dashboard_window` | `raw_buttons_messages_and_unsupported_dashboard_model_policies_are_visible`, `dashboard_selection_is_id_selection_only_and_preserves_zero_override`; four button-to-command mappings supported; ShowUI and gauge values/formats Unsupported: open DR1; unknown button return open DR6 |
| 7 Messages and callbacks | `do_message`, `add_callback_target`, `send_callback_message`, `callback_plans`, `reenable_zoom_buttons` | `raw_buttons_messages_and_unsupported_dashboard_model_policies_are_visible`, `removed_and_notification_identity_matching_is_mandatory`, `callback_plans_preserve_values_and_insertion_order_without_invented_refs`; panel policy/alternate model/unknown returns Unsupported: open DR6; callback ownership/delivery/payload open DR3; literal re-enable of both existing buttons follows §7, interpretation still open DR7 |
| 8 Quirks | `pause_everything`, `on_mouse_down_l` | `choosing_pauses_all_four_but_simulator_only_if_needed`, `driving_projection_orders_raw_operations_and_resumes_conditionally`, `accepted_network_pick_wins_in_choosing_and_driving`; pause/resume and network-before-automaton priority supported |
| 9 Open questions | `Unsupported`, `Progress::Partial`, `Operation::Gap`, raw numeric command/slot API | `ordinary_keys_never_falsely_bypass_unpublished_key_effects`, `raw_buttons_messages_and_unsupported_dashboard_model_policies_are_visible`, `failures_preserve_emitted_prefix_and_never_invent_reference_cleanup`; open DR1–DR8 accounted for above, no invented command/slot meanings (DR2) or failure rollback (DR3) |
| 10 Conformance tests | `command_effects`, `on_key_down`, `end_input`, `pause_everything` | §10.1 `two_toggles_keep_independent_state_and_release_has_no_effect`; §10.2 `autorepeat_suppresses_all_queries_and_escape_effects`; §10.3 `end_input_and_escape_order_callbacks_slot18_cleanup_and_base_stack_removal`; §10.4 choosing/driving pause tests above; all four published worked cases executable |
