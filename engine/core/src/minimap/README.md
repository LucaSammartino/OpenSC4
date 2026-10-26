# Minimap owners

Portable `cSC4WinMiniMap` and nested `cTZMap`, from `docs/specs/minimap.md`.
Base windows use `ui_windows::base::Windows`; foreign reference, capture and
drawing primitives use `widget_controls::Foreign`. No C ABI or plugin wiring.

The section closure table will name each implemented section and its spec test.

| Spec section | Code item | Worked case / test |
|---|---|---|
| 2 Object layout | `MiniMap`, `new`, inherited window handle | `constructor_layout_and_inherited_query` |
| 3 City attach and size | `set_area`, `init_tz_map`, `on_city_init/shutdown`, `TzMap::init` | `mm_c1_c2_c3_size_scale_and_attached_resize_quirk`; nested owner allocation test |
| 4 Tuning and palettes | `load_tunable_values`, `interpolate_ramp` | `mm_c6_shipped_ramp_and_endpoints`, `missing_tuning_keeps_palettes_but_dirties_map` |
| 5 Terrain/zone map | `update_terrain_and_zone_map`, dirty rects, `resample_block`, `numeric::E` | MM-C4/C5: `mm_c4_c5_height_and_underwater_brightness`; `all_five_resamplers_and_skip`; dirty/rounding tests. **Open MMW1:** raw slot-16 water values required, default `Unsupported::Open` |

`numeric::E` retains a 64-bit significand for products, sums, square roots and
division, then performs the specified binary32 rounding store. Decimal constants
with no operand width in the spec use binary64 (SPEC-GAP); no host FP mode is used.
Native out-of-bounds dirty/zone/pixel accesses return `Unsupported::Precondition`.
