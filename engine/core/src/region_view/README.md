# cSC4WinRegionView closure

Source: `docs/specs/region-view.md`. No ABI or plugin wiring. Native windows use
`ui_windows::base::Windows`; text drawing uses `widget_controls::Draw`.
City entries and their snapshot surfaces remain caller-owned. Foreign resource
loads, canvas surfaces and effects are explicitly created/released through traits.

| Spec section | Code item | Spec-derived test / boundary |
|---|---|---|
| 1 object | `RegionView::new`, `CityRecord` | `selection_settings_signpost_placement_and_open_child_contract`; owner-supplied region key, open RGV1 |
| 1.1 init | `RegionView::init` | `failed_canvas_init_leaves_view_initialised_and_retry_succeeds_without_canvas` |
| 1.2 lifetime/area/transparency | `set_area`, `shutdown`, `point_transparent` | failed-init test checks shutdown; native child hit testing is a Host primitive |
| 2 cities | `CityEntry`, selection/settings below | RGV-C4 and selection tests below |
| 2.1 selection/settings | `add_city`, `select_city`, `clear_selection`, settings, `invalidate_all` | `selection_settings_signpost_placement_and_open_child_contract` |
| 2.2 positions/hits | `draw_position`, `playfield_rect`, `city_at_point`, `clear_cities` | `rgv_c4_runs_and_reverse_overlap` (RGV-C4) |
| 2.3 entry | `CityEntry` | RGV-C4 supplied hit runs; builder is open RGV5 |
| 3 canvas | `Canvas`, `Tile`, `CanvasHost` | RGV-C1..C3 check shape, dirty flags and surface identities |
| 3.1 init/shutdown | `Canvas::{init,shutdown}` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C1); failed view-init quirk |
| 3.2 invalidation | `Canvas::invalidate_rect` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C3) |
| 3.3 scrolling | `Canvas::scroll_to` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C2) |
| 3.4 render | `Canvas::render` | `canvas_render_cleans_once_and_lock_failure_does_no_work` |
| 3.5 update | `Canvas::update`, `UpdateHost` | `update_without_2d_interface_restores_renderer_and_viewport` |
| 3.6 vector helpers | Rust `Vec<Tile>` and explicit surface ownership | RGV-C2 checks surface/dirty pairing |
| 4 tile drawing | `RegionView::{render_tile,render_canvas}` | `tile_composition_map_selection_and_open_render_contracts`; explicit Unsupported for open RGV3 icon blitter and RGV4 mayor baseline |
| 4.1 image helpers | `image::{build_runs,copy_and_tint,blit_image_spans,fill_coverage}` | `image_edges_runs_tint_lock_quirk_and_clipped_blend`; mask builder open RGV5 |
| 5 update city | `RegionView::update_city` | `poi_projection_effect_replacement_and_global_invalidation` |
| 6 signposts/scroll | `update_signpost`, `set_scroll_pos` | `selection_settings_signpost_placement_and_open_child_contract`; explicit Unsupported for open RGV6 child interface |
| 6.1 signpost contents | `signpost_data`, `rating_fraction` | `rgv_c5_signpost_values_negative_budget_and_open_price`; explicit Unsupported for open RGV2 price source |
| 7 animator | `Animator::{message,shutdown}` | `rgv_c6_bob_and_counted_child_lifetime` (RGV-C6) |
| 8 quirks | lifetime/image tests above | source lock is intentionally retained; unsafe undersized target is Unsupported |
| 9 cases | tests above | RGV-C1..C6 covered |
| 10 not covered | foreign traits | screen, fonts, surfaces, effects remain external owners |
| 11 inventory | section mapping above | function addresses are not runtime dependencies |

Open RGV1 requires an explicit opaque region key; no guessed manager identity.
Open RGV5 permits consumption of supplied masks, not invention of the snapshot
mask builder. `CityRecord` reuses `regional_city::RegionalCity` for transient
interface snapshots; no city metadata is stored in the view's state.
