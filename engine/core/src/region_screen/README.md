# cSC4WinRegionScreen closure

Source: `docs/specs/region-screen.md`. This module owns screen policy, uses
`region::Region` for cell lookup/insertion/enumeration, `regional_city` for record
construction, `region_view::RegionView` for selection/drawing state and
`ui_windows::base::Windows` for its base. Existing script-created widget controls
are accessed through `Host`; no controls, foreign windows or simulators are copied.

| Spec section | Code item | Spec-derived test / Unsupported boundary |
|---|---|---|
| 0 summary | `RegionScreen`, `play_selected`, `mouse_down` | `screen_c1_double_click_deadline_and_held_hover`, `screen_c2_large_empty_tile_creates_four_by_four_and_requests_new_city` |
| 1 tuning | `read_tuning`, `Tunable` | `screen_tuning_population_and_snapshot_builder_boundary`; CB1 opacity and CAB/CAC colours; unknown meanings remain open QR1 |
| 2 lifetime | `init`, `shutdown`, base `Windows` | `screen_lifetime_is_explicitly_unsupported_at_qr1`; application services and GUI construction/teardown remain Unsupported, open QR1 |
| 3 cities in view | `put_cities_in_view`, `update_population` | `screen_tuning_population_and_snapshot_builder_boundary`; stops at mask builder open RGV5 or scroll-limits open QR5; no guessed snapshots/limits |
| 4 mouse/keys | `mouse_move`, `mouse_down`, `scroll`, `process_command` | `screen_c1_double_click_deadline_and_held_hover`, `screen_commands_scroll_messages_preferences_and_notifications`; selected presentation is a request to existing controls |
| 5 buttons | `button`, `process_command` | `screen_commands_scroll_messages_preferences_and_notifications`; panel/toggle/whole-region bodies Unsupported, open QR3; snapshot reset stops at open RGV5 |
| 6 cities/regions | helpers below | four §11 cases below |
| 6.1 get/create | `get_or_create_city` | `screen_c2_large_empty_tile_creates_four_by_four_and_requests_new_city` (4×4 at tile, localized name, one insertion) |
| 6.2 launch regional city | `launch_regional_city` | `screen_c3_existing_path_load_and_failed_load_fallback` (LoadCity success; failure clears new-city object before RequestNewCity) |
| 6.3 load region | `load_region` | `screen_c4_load_current_region_has_no_side_effects` |
| 6.4 launch city | `launch_city`, command size parsing | `screen_launch_city_filters_name_size_and_empty_flag` |
| 6.5 import | `button` | explicit Unsupported, open QR2/QR4 (includes unresolved refresh CY8); tested by command test |
| 7 preferences | `save_position`, `save_region_position` | `screen_c3_existing_path_load_and_failed_load_fallback`, command test (truncation toward zero) |
| 8 messages | `message`; command messages route to `process_command` | command test (scroll/start/stop, accelerator post, camera/view-init) |
| 9 quirks | held state, tuning selector | double-click/held-hover and tuning tests; CA2 is never requested |
| 10 open questions | `Unsupported::Open` | QR1..QR5 and view RGV5 boundaries above |
| 11 conformance | `tests.rs` | screen-C1..C4 are the four worked cases, each named above |
| 12 scroll/cloud windows | separate foreign owners | outside this two-owner task; screen-facing contracts below |
| 12.1 scroll window | `scroll` message receiver, foreign scroll producer | command test checks fixed-point displacement; drag/glide owner outside this task |
| 12.2 cloud window | `Host::cloud_opacity` | tuning test checks screen's opacity values; cloud drawing owner outside this task |

`Host` provides primitive application, localization, resource and control calls.
It does not choose city sizes, launch fallbacks, double-click decisions or commands.
No ABI/plugin wiring; live-game shadow comparison is not applicable to these cores.
