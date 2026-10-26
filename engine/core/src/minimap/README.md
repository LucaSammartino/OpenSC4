# Minimap owners

Portable `cSC4WinMiniMap` and nested `cTZMap`, from `docs/specs/minimap.md`.
Base windows use `ui_windows::base::Windows`; foreign reference, capture and
drawing primitives use `widget_controls::Foreign`. No C ABI or plugin wiring.

Every settled section is implemented below. Open questions are foreign input
boundaries, not guessed native behavior. Tests use recording services; they
establish portable behavior and do not claim a native shadow comparison.

| Spec section | Code item | Worked case / test |
|---|---|---|
| 1 Identity/lifetime | `CLASS_ID`, `query_interface`, `init/shutdown`, `release`, `deleting_destructor` | `constructor_layout_and_inherited_query`; `init_failure_success_defaults_subscriptions_and_idempotent_shutdown`; `final_release_and_non_deleting_destructor_release_owned_resources_once`. **Open MMW4:** surface creation/format remains an opaque Unsupported-default boundary |
| 2 Object layout | `MiniMap`, `new`, inherited window handle | `constructor_layout_and_inherited_query` |
| 3 City attach and size | `set_area`, `init_tz_map`, `on_city_init/shutdown`, `TzMap::init` | `mm_c1_c2_c3_size_scale_and_attached_resize_quirk`; nested owner allocation test |
| 4 Tuning and palettes | `load_tunable_values`, `interpolate_ramp` | `mm_c6_shipped_ramp_and_endpoints`, `missing_tuning_keeps_palettes_but_dirties_map` |
| 5 Terrain/zone map | `update_terrain_and_zone_map`, dirty rects, `resample_block`, `numeric::E` | MM-C4/C5: `mm_c4_c5_height_and_underwater_brightness`; `all_five_resamplers_and_skip`; dirty/rounding tests. **Open MMW1:** raw slot-16 water values required, default `Unsupported::Open` |
| 6 Networks | `SurfaceNetworkFilter`, `update_networks`, `network_changed` | `filter_iids_and_surface_mask`; `checked_pairs_stale_reuse_upscale_single_pixel_and_changes` |
| 7 Update and blits | `visible_recursive`, `update`, `rotate_blt/dual` | MM-C7: `mm_c7_all_rotations_pitches_and_premultiplied_overlay`; `invisible_parent_frees_pixels_and_update_tail_rehooks`. **Open MMW4:** raw surface service/lock operations default to `Unsupported::Open` |
| 8 Paint | `paint`, `dither_rgb8_to_argb4`, `frustum_points`, `marker_points` | `dither_uses_remaining_counts_and_padded_rows`, `frustum_size_minus_one_and_marker_size_rotation`. **Open MMW2/MMW3/MMW4:** raw geometry, marker service and surface operations have explicit Unsupported defaults |
| 9 Input | `center_on_map_point`, `mouse_down/move/up/enter_or_exit` | MM-C10: `mm_c10_center_drag_capture_rotations_and_nan_clamp` |
| 10 Messages/settings | `do_message`, `tick`, `set_flags`, zone/overlay setters and hooks | MM-C8/C9: `mm_c8_tick_exceeds_threshold_once_and_hidden_releases_map`, `mm_c9_flags_unhook_before_change_and_xor_unmasked_value`; message/setter tests. **Open MMW3:** tick's slot-49/slot-6 query defaults to Unsupported |
| 11 Quirks | stored-size resize, x-only scale, persistent network pairs, dirty upper clamp, zone overwrite, single network pixel, paint/input projections | size, cache, dirty, resampler, projection tests above; oversize 4444 reads return Unsupported rather than accessing outside allocations |
| 12 Conformance cases | MM-C1..MM-C10 | `mm_c1_c2_c3_*`, `mm_c4_c5_*`, `mm_c6_*`, `mm_c7_*`, `mm_c8_*`, `mm_c9_*`, `mm_c10_*` |
| 13 Function inventory | methods above, `TzMap::init`, `visible_recursive`, `CLASS_ID` | identity/lifetime, nested allocation and recursive-visibility tests; native subobject adjustments are represented by one shared window identity |

`numeric::E` retains a 64-bit significand for products, sums, square roots and
division, then performs the specified binary32 rounding store. Decimal constants
with no operand width in the spec use binary64 (SPEC-GAP); no host FP mode is used.
Native out-of-bounds dirty/zone/pixel accesses return `Unsupported::Precondition`.

`closure_tests` adds live foreign-service cases for palette alpha policy,
selective transposed terrain blocks, zone overwrite, incomplete terrain blocks,
overlay-only bypass, buffer/surface unlocks on 4444 bounds failure, frustum
edges, alternating marker colours, hidden-map recreation and the raw tick marker.
MMW1–MMW4 refer to the existing open entries in
`docs/specs/questions/minimap.md`; no new question is needed by this implementation.

Adapters must call the owner methods (including its final `release`), not the
base runtime's final release directly. Explicit Shutdown and OnCityShutdown
events precede destruction; a Rust drop alone cannot release foreign handles.
