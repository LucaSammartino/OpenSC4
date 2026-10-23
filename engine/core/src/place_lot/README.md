# Place-lot owner

Portable `cSC4ViewInputControlPlaceLot` from city-tools-input.md §3. Core only;
no plugin, C ABI, game shadow or simulator replacement. `ControlHost`, `Pixel`
and `Vec3` reuse the existing input-control contract, as in `query_tool`.
Foreign traits supply configuration/building records, renderer and UI primitives,
terrain, lot developer, budget, demolition and zoning services. They must never
call native place-lot owner methods. Selection, validation, facing, price,
placement ordering and input/message state transitions belong here.

`Effect::Unsupported(Open)` explicitly records open behavior. In particular,
`FeedbackState` is a diagnostic state record, not a request to delegate
`SetVisualUserFeedback` to the host. Preview building marker/view selection and
effect lifecycle are published, but their CT17 model/prop/tint/ring algorithms
remain open. Toll booths fail closed; ordinary lots have full geometry/facing.
Getters use shared references; there is no RefCell. Identities are portable
tokens, not pointers. Persistent acquired services/cursors and created preview
effects have separate release effects. Shutdown is explicit because Drop cannot
access a foreign host. The unchanged shared input-control contract provides
capture/stack and sound transport; this module introduces no second base class.
Native layout/vtable addresses and allocation size are not portable state.

Integer/float operations follow the spec literally on finite representable city
coordinates, positive lot dimensions/cell width and nonoverflowing costs. The
unsigned world rectangle stores the constructor's -1 sentinel as `u32::MAX` and
compares its signed interpretation, preserving the empty initial rectangle.
The cost indicator displays the low dword. A zero plop price skips development
cost but demolition cost is still added, per §3.10's separate ordered steps.

## Owner closure

Each row is for `cSC4ViewInputControlPlaceLot`. Test names are in `tests.rs`.
All published settled behavior is implemented. Exact closure remains blocked
only by the questions listed below; these paths must not be called exact shadows.

| Spec section | Code item | Test / open portion |
|---|---|---|
| 3.1 Identity, creation, defaults, inherited input | `PlaceLot::new`, `ID`, `CURSOR`, `should_stack`, `unhandled_input`; inherited shared `ControlHost` | `construction_and_ct3_lifecycle` |
| 3.2 Init, pickers, filters, price, preview view, holds | `init`, `pick`, `resolved_building`, `build_preview` | `activation_pickers_filters_and_failure_paths`, `construction_and_ct3_lifecycle`; preview model/ring: open CT17 |
| 3.3 Shutdown | `shutdown`, `release_configuration_effects`, `stop_previews` | `construction_and_ct3_lifecycle`, `messages_stop_only_running_release_both_and_open_boundaries`; configuration model effects: open CT17 |
| 3.4 Messages | `message`, `stop_previews` | `messages_stop_only_running_release_both_and_open_boundaries`; rebuild visuals: open CT17; ring-finished body: open CT22 |
| 3.5 Rotation keys | `key_down` | `ct4_rotation_keys_top_translation_and_manual_path` |
| 3.6 Left click, world exclusion, count, sound | `mouse_down` | `ct5_click_sound_repeat_rectangle_and_plop_limit` |
| 3.7 Cursor move, validation, auto/manual paths, cost/text/delta feedback | `mouse_move`, `can_place`, `automatic_facing` | `mouse_feedback_text_cost_exit_and_failed_coverage`, `ct4_rotation_keys_top_translation_and_manual_path`; WM slot 19: open CT2; toll branch: open CT16; rendered feedback/flag: open CT17 |
| 3.8 Exit | `mouse_exit` | `mouse_feedback_text_cost_exit_and_failed_coverage`; rendered feedback: open CT17 |
| 3.9 Cell geometry | `cell_coverage`, `coverage` | `conformance_odd_even_rotated_cells_and_edges` |
| 3.10 Validation, precedence, codes, price, demolition | `can_place`, `demolition`, `CREATION_MESSAGES` (zones.md §8.4.1) | `validation_precedence_price_codes_and_demolition_flags` |
| 3.11 Automatic facing and frontage | `automatic_facing`, `street_count`, `choose_facing` | `conformance_frontage_hysteresis_ties_and_unplaceable_rotations`; CT24: detailed formula retained at exactly four more points |
| 3.12 Commit, tutorial/funds, filter, zones, lot/building, refund, aura, connections, effects | `place_lot`, `demolish_accepts`, `Zone`, `Demolition` | `commit_order_money_aura_connections_and_effects`, `conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot`, `commit_guards_unknown_cost_free_zones_and_sizes`; toll selection: open CT16; exact ring rendering: open CT17; unspecified zoning flags/failure handling: open CT23 (literal sequence/default service arguments retained) |
| 3.13 Toll booths | `toll_booth`, `Open::TollBoothCt16` | `activation_pickers_filters_and_failure_paths`, `messages_stop_only_running_release_both_and_open_boundaries`; open CT16 (no native helper callback) |
| 3.14 Feedback | `build_preview`, `feedback`, `stop_previews`, marker/view selection | `construction_and_ct3_lifecycle`, `messages_stop_only_running_release_both_and_open_boundaries`; open CT17 |
| 4 Place-lot quirks 9–11 | inclusive repeat exclusion, `choose_facing`, failure deposit | `ct5_click_sound_repeat_rectangle_and_plop_limit`, `conformance_frontage_hysteresis_ties_and_unplaceable_rotations`, `conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot` |
| 6 Place-lot conformance 3–5 | lifecycle, keys, click | `construction_and_ct3_lifecycle`, `ct4_rotation_keys_top_translation_and_manual_path`, `ct5_click_sound_repeat_rectangle_and_plop_limit` |
| 6 re-plop conformance six bullets | odd/even snapping, frontage, hysteresis, exclusion, failed-builder deposit | `conformance_odd_even_rotated_cells_and_edges`, `conformance_frontage_hysteresis_ties_and_unplaceable_rotations`, `ct5_click_sound_repeat_rectangle_and_plop_limit`, `conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot` |

Section 6 tests 1–2 concern the shared base owner, and tests 6–10/re-query concern
`query_tool`; they are not additional place-lot algorithms. This module checks
the base lifecycle behavior it exercises (idempotent init/shutdown, cursor ID
preservation, unknown cursor no-op and ordered releases) without duplicating its
COM QueryInterface/reference-counter implementation.
