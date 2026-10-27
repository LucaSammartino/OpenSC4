# cWinBMPViewGVExtra

| picture-view.md section | Code | Spec fixture / test | Boundary |
|---|---|---|---|
| 8 Constructor/Init | `AlbumPicture::{new,init}` | `constructor_init_does_not_resize_or_apply_fit`: large at construction, bitmap flag 2, unchanged size produces no effects | Initial fit byte supplied; base picture view init is `PictureHost` |
| 8 SetSizeInOut | `set_size` | `size_switches_use_base_tree_geometry_and_notify_parent`: 32x32 small, 16x16 large defaults, shared tree order, parent 0x9127484A | Base view settings/drawing are individual foreign calls |
| 8 SetShouldFitImageAtSizeIn | `set_should_fit` | Same size-switch fixture: Scroll Lock disables small fit; changing fit applies immediately | Scroll Lock is a foreign input |
| 8 Mouse/DoMessage | `left_click`, `message` | `double_click_under_500_toggles_and_reset_prevents_a_third_toggle`: 499 toggles, 500 forwards, reset starts a new pair | Timer and base view input/message calls are foreign |

This is a portable owner module; `ui_windows::base::Windows` owns identity,
tree order and rectangles. It does not duplicate the picture renderer, codec,
window implementation or native ABI. Lifetime/control binding remains SN7.
