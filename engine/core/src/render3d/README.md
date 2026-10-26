# City 3D render owner

Source: `docs/specs/render3d.md`. Foreign interfaces use handles and traits;
no C ABI or plugin is added. Existing view3d host boundaries remain unchanged.
The table records whole-section closure, including explicit open boundaries.

This is a partial owner ready for continued core development. Whole-owner
implementation credit remains pending: Init and initialized teardown, model
processing, drawing and driver orchestration are absent. The public `shutdown`
currently covers only the never-initialized guard. RND1 and RND4 remain explicit
`Unsupported` boundaries; this module cannot yet replace the game's renderer.

| Spec section | Code item | Spec case / test | Closure |
|---|---|---|---|
| 1 Identity, references, lifetime | `Render3d::new`, `query_interface`, `release` | `identity_case_1_and_constructor_defaults`, `last_release_case_2_including_initial_zero` | Complete for constructed owner; initialized teardown in §9 |
| 2 Serialized state | `read`, `write`, `Unsupported::Rnd1StreamSlot8` | `serialization_is_explicitly_unsupported_rnd1` | Unsupported: RND1 open; case 9 deferred |
| 3 Camera, viewport, accessors | `view`, `ViewServices`, `BackingSwitch` | `viewport_and_forwarding_cases_3_4`, `toggles_and_global_backing_cases_5_6`, `borrow_restores_each_camera_mode_and_default_state` | Complete |
| 4 Full redraw | `force_full_redraw`, `do_full_view_update` | `full_redraw_coalesces_case_7_but_always_writes_reason` | UpdateViewportRect deferred with §13 |
| 5 Model queue, quad grid, passes | `QuadGrid`, `GridElement`, `BackingLimits` | `grid_case_13_ties_to_even_and_scale`, `backing_building_case_15_and_category_gates`, `grid_tests` | §5.1 complete; model queue and passes pending |
| 6 Dirty rectangles, cleanup, shake stop | `DirtyQueue`, `add_dirty_rectangle`, `clear_dirty_rectangle_queue`, `stop_screen_shake` | `dirty_tests`: D1–D6 and case 11; `shake_stop_immediate_or_pull_start_back_without_pushing_forward` | §§6.1, 6.3 complete; processing and cleanup pending |
| 7 Cross-references | — | Sections 11, 13–15 | Pending |
| 8 Conformance cases | `tests`, `grid_tests`, `dirty_tests` | Cases 1–7, 11, 13, 15; D1–D6; case 9 boundary | Partial: cases 8–10, 12, 14, 16 deferred |
| 9 Lifecycle | — | Init/Shutdown guard and property tuples | Pending |
| 10 Screen shake | `Shakes`, `ShakePatterns`, `start_screen_shake`, `update_screen_shake`, `stop_screen_shake` | `shake_tests`: S1 parameter handles, S2–S6, immediate/fading stop, process tables; `rnd4_overload_fails_without_starting_a_shake` | Complete parameter-block path; Unsupported overload: RND4 open; portable f64 intermediates replace x87 |
| 11 View objects, visibility, picking | `ViewObjects`, `PickingServices`, `pick_ray`, `pick_terrain_ray`, `pick_terrain_always_ray`, `terrain_polyline` | `picking_tests`: P1–P6, §11.6 worked polyline, outside-start projection, distance/filter ownership and segment end rejection | Complete policy; terrain/camera acquisition awaits Init (§9); portable f64 vector intermediates |
| 12 Backing store | `BackingStore`, `BackingServices`, `save_backing_store`, `restore_backing_store`, `reevaluate_backing_store_object_flags` | `backing_tests`: B1–B4, four probe failure points, reevaluation | Complete |
| 13 Updates, clearing, driver | — | Case 16 and section 13 cases | Pending |
| 14 Container rectangles, grid rebuild | `ContainerGeometry`, `ContainerServices`, `world_to_container_rect`, `visible_container_rect`, `viewport_ray_container_rect`, `move_grid_element`, `rebuild_quad_grid` | `container_tests`: rotation/padding table, ordered clamps/sliver, frustum cache invalidation, ray edges, move flags, rebuild guard/binding; L3 margins | Complete; margins helper awaits Init (§9) |
| 15 Toggles, lifecycle remainder, helpers | — | Section 15 worked cases | Pending |
| 16 Inventory | — | Closure audit against preceding rows | Pending |
