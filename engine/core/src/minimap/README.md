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
