# Player-drive owner

Portable `cSC4ViewInputControlPlayerDrive`, from `docs/specs/drive-tool.md`.
The existing owner implements ordered primitive plans behind `Primitives`;
it introduces no C ABI or plugin wiring. Picks and city conversions are foreign
service operations; the mode, command and cleanup decisions belong to this core.

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
