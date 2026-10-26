# Dispatch vehicle view

Clean-room portable `cSC4DispatchVehicleView`. No C ABI or plugin wiring.
`TrackInfo` is a separate owner module. Foreign identities are opaque tokens;
resource-handle identity remains separate from the resolved texture identity.
Exclusive mutation and immutable getters use no interior mutable state.

Owner status: implemented for the assigned view and TrackInfo owners, subject
to the explicit Unsupported boundaries and numeric qualifications below.

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| dispatch-vehicle-view §1.1 | `DispatchVehicleView` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §1.2 | `TrackInfo`, stable low-byte `insert` | `low_byte_map_is_stable_for_equal_keys` |
| §2 tracking kinds | `track`, `TrackInfo::for_tracking` | `track_limits_global_handles_model_lifecycle_and_removal`, TrackInfo worked kinds |
| §3.1 | `set_indicator_max` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §3.2 | `find` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §3.3 | `lock`, `flash` | `flash_strict_period_lock_and_nan_angle` |
| §3.4 | `active_occupant` | `priority_selection_active_frames_and_hidden_fallback` |
| §3.5 | `use_alternate_model` | `priority_selection_active_frames_and_hidden_fallback` |
| §3.6 | `is_selectable` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 7) |
| §3.7 | `find_indicator_handle` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 7) |
| §3.8 | `set_active` | `priority_selection_active_frames_and_hidden_fallback` |
| §3.9 | `show_all` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 6) |
| §3.10 | `sort_priority` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 6) |
| §3.11 | `rebuild_windows`, `WindowServices` | `window_filter_uses_or_dimensions_and_releases_exclusion_result` |
| §3.12 containers / ownership | stable `insert`, moved unique processor, `remove_at`, `shutdown` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §4.1 images | `build_display` | `worked_strip_and_quarter_camera_rotation` (§8 case 2) |
| §4.2 rotation | `rotate_quad` | `worked_strip_and_quarter_camera_rotation` (§8 case 3) |
| §5.1 | `update` timer/context gates, window/obstacle setup | `gates_delay_removal_and_hidden_fade_finishes_strictly_below_threshold` |
| §5.2 | `update` ordered walk, ring, terrain, clamp, visibility | `worked_building_ring_and_clamp` (§8 cases 4–5), `identity_draw_order_null_number_texture_visibility_limits_and_simulator` |
| §5.3 | `relocate`, `screen_quads`, `draw_2d` | `closed_conflicts_probe_order_and_blocked_fallback`, `flash_strict_period_lock_and_nan_angle`, draw-order test |
| §5.4 | `draw_world`, rectangle/anchor stores | `world_two_passes_do_not_become_placement_obstacles` (§8 case 5) |
| §5.5 | `update` timer restart and local-frame storage drop | `gates_delay_removal_and_hidden_fade_finishes_strictly_below_threshold` |
| §5.6 | `draw_world`, `draw_2d` state selection | `world_two_passes_do_not_become_placement_obstacles`, `identity_draw_order_null_number_texture_visibility_limits_and_simulator`; open DVV30-1 for packing only |
| §5.7 | `update_model`, parent progress, tint, translation, spin, draw | `model_progress_parent_walk_material_spin_and_call_order`; open DVV30-2 for ambiguous row-2 sum, explicit Unsupported when the alternatives differ |
| §6 | `cSC4ViewInputControlDispatch` | Separate owner outside this task; no input-tool implementation claimed |
| §7 | limit 0, strict flash reset, NaN angle, 7° spin, processor-null boundary | `track_limits_global_handles_model_lifecycle_and_removal`, `flash_strict_period_lock_and_nan_angle`, `model_progress_parent_walk_material_spin_and_call_order`; native null-this is `Unsupported::NativeProcessorAllocation` |
| §8 | worked cases 1–7 | TrackInfo kind test plus view tests above; case 8 is the separate tool |
| §9 | owner inventory | Rust ownership replaces STL/destructor bodies; native pointer adjustors remain adapter work |
| dispatch §4.1.3 number display / lifetime / messages / live texture | `build_display`, `track`, `untrack`, `do_message`, `FrameServices::icon_texture` | `number_surface_sizing_failure_and_worked_geometry`, lifecycle test, `texture_resolution_follows_the_live_handle_on_each_world_pass` |
| Init packed state words | `Services::render_states`, `Unsupported::OpenRenderStates` | open DVV30-1: adapter must supply state words; never invent packed values |

Native layout offsets, deleting-destructor flags and QI adjustor addresses are
adapter concerns, not Rust memory layout. Out-of-range kind/zoom indexing and
the specified null processor dereference return explicit `Unsupported` errors.
Shutdown uses the documented normal per-entry cleanup (SPEC-GAP marked in code).

Finite geometric expressions use f64 intermediates and the specified f32
stores; they do not certify arbitrary ambient x87 precision, extended exponent
or transcendental payload bits. Tests compare exact discrete state/protocol
results and the worked finite geometry, with tolerances for trig. No game
shadow key exists for these modules. Created and acquired references are
released explicitly; call `shutdown` before disposing a view with live entries.
