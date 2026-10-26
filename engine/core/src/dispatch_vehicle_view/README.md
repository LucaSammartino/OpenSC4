# Dispatch vehicle view

Clean-room portable `cSC4DispatchVehicleView`. No C ABI or plugin wiring.
`TrackInfo` is a separate owner module. Foreign identities are opaque tokens;
resource-handle identity remains separate from the resolved texture identity.
Exclusive mutation and immutable getters use no interior mutable state.

| Spec section | Code item | Worked-case test / boundary |
|---|---|---|
| dispatch-vehicle-view §1.1–1.2 | `DispatchVehicleView`, stable low-byte `insert` | `low_byte_map_is_stable_for_equal_keys` |
| §2 tracking kinds | `track`, `TrackInfo::for_tracking` | `track_limits_global_handles_model_lifecycle_and_removal`, TrackInfo worked kinds |
| §3.1–3.2 limits / lookup | `set_indicator_max`, `find` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §3.3 lock / flash | `lock`, `flash` | frame-walk tests (next section) |
| §3.4–3.8 active / alternate / selection | `active_occupant`, `use_alternate_model`, `is_selectable`, `find_indicator_handle`, `set_active` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 7) |
| §3.9–3.10 hiding / priority | `show_all`, `sort_priority` | `priority_selection_active_frames_and_hidden_fallback` (§8 case 6) |
| §3.11 windows | `rebuild_windows`, `WindowServices` | frame-walk window test (next section) |
| §3.12 containers / ownership | stable `insert`, moved unique processor, `remove_at`, `shutdown` | `track_limits_global_handles_model_lifecycle_and_removal` |
| §4.1 images | `build_display` | `worked_strip_and_quarter_camera_rotation` (§8 case 2) |
| §4.2 rotation | `rotate_quad` | `worked_strip_and_quarter_camera_rotation` (§8 case 3) |
| dispatch §4.1.3 number display / lifetime / messages | `build_display`, `track`, `untrack`, `do_message` | `number_surface_sizing_failure_and_worked_geometry`, lifecycle test |
| Init packed state words | `Services::render_states`, `Unsupported::OpenRenderStates` | open DVV30-1: adapter must supply state words; never invent packed values |

Native layout offsets, deleting-destructor flags and QI adjustor addresses are
adapter concerns, not Rust memory layout. Out-of-range kind/zoom indexing and
the specified null processor dereference return explicit `Unsupported` errors.
Shutdown uses the documented normal per-entry cleanup (SPEC-GAP marked in code).
