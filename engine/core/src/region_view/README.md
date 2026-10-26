# cSC4WinRegionView closure

Source: `docs/specs/region-view.md`. No ABI or plugin wiring. Native windows use
`ui_windows::base::Windows`; text drawing uses `widget_controls::Draw`.
City entries and their snapshot surfaces remain caller-owned. Foreign resource
loads, canvas surfaces and effects are explicitly created/released through traits.

| Spec section | Code item | Spec-derived test / boundary |
|---|---|---|
| 1, 1.1, 1.2 object and lifetime | `RegionView::{new,init,set_area,shutdown,point_transparent}` | `failed_canvas_init_leaves_view_initialised_and_retry_succeeds_without_canvas`; owner-supplied region key, open RGV1 |
| 2.1 selection/settings | `select_city`, `clear_selection`, `invalidate_all` | `rgv_c4_runs_and_reverse_overlap`; signpost completion follows in next milestone |
| 2.2 positions/hits | `draw_position`, `playfield_rect`, `city_at_point`, `clear_cities` | `rgv_c4_runs_and_reverse_overlap` (RGV-C4) |
| 2.3 entry | `CityEntry` | RGV-C4 supplied hit runs; builder is open RGV5 |
| 3, 3.1 canvas/init | `Canvas::{init,shutdown}` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C1); failed view-init quirk |
| 3.2 invalidation | `Canvas::invalidate_rect` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C3) |
| 3.3 scrolling | `Canvas::scroll_to` | `rgv_c1_c2_c3_canvas_worked_cases` (RGV-C2) |
| 3.4 render | `Canvas::render` | `canvas_render_cleans_once_and_lock_failure_does_no_work` |
| 3.5 update | `Canvas::update`, `UpdateHost` | foreign draw-context boundary; restore-state test in next milestone |
| 3.6 vector helpers | Rust `Vec<Tile>` and explicit surface ownership | RGV-C2 checks surface/dirty pairing |
| 4 tile drawing | Pending next milestone | Open RGV3 icon blitter and RGV4 mayor baseline |
| 4.1 image helpers | `image::{build_runs,copy_and_tint,blit_image_spans,fill_coverage}` | `image_edges_runs_tint_lock_quirk_and_clipped_blend`; mask builder open RGV5 |
| 5 update city | Pending next milestone | POI/effects test pending |
| 6, 6.1 signposts | `Unsupported::Open("RGV6")` until next milestone | Open RGV2 price source and RGV6 child interface |
| 7 animator | Pending next milestone | RGV-C6 pending |
| 8 quirks | lifetime/image tests above | source lock is intentionally retained; unsafe undersized target is Unsupported |
| 9 cases | tests above | RGV-C1..C4; C5/C6 pending |
| 10 not covered | foreign traits | screen, fonts, surfaces, effects remain external owners |
| 11 inventory | section mapping above | function addresses are not runtime dependencies |
