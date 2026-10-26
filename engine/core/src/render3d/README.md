# City 3D render owner

Source: `docs/specs/render3d.md`. Foreign interfaces use handles and traits;
no C ABI or plugin is added. Existing view3d host boundaries remain unchanged.
The table records whole-section closure, including explicit open boundaries.

| Spec section | Code item | Spec case / test | Closure |
|---|---|---|---|
| 1 Identity, references, lifetime | `Render3d::new`, `query_interface`, `release` | `identity_case_1_and_constructor_defaults`, `last_release_case_2_including_initial_zero` | Complete for constructed owner; initialized teardown in §9 |
| 2 Serialized state | `read`, `write`, `Unsupported::Rnd1StreamSlot8` | `serialization_is_explicitly_unsupported_rnd1` | Unsupported: RND1 open; case 9 deferred |
| 3 Camera, viewport, accessors | `view`, `ViewServices`, `BackingSwitch` | `viewport_and_forwarding_cases_3_4`, `toggles_and_global_backing_cases_5_6`, `borrow_restores_each_camera_mode_and_default_state` | Complete |
| 4 Full redraw | `force_full_redraw`, `do_full_view_update` | `full_redraw_coalesces_case_7_but_always_writes_reason` | UpdateViewportRect deferred with §13 |
| 5 Model queue, quad grid, passes | — | Cases 8, 12–15 | Pending |
| 6 Dirty rectangles, cleanup, shake stop | — | Cases 10–11, D1–D6 | Pending |
| 7 Cross-references | — | Sections 11, 13–15 | Pending |
| 8 Conformance cases | `tests` | Cases 1, 9 boundary | Partial |
| 9 Lifecycle | — | Init/Shutdown guard and property tuples | Pending |
| 10 Screen shake | `Unsupported::Rnd4ShakeOverload` | RND4 overload boundary | Pending; RND4 open |
| 11 View objects, visibility, picking | — | Section 11 worked cases | Pending |
| 12 Backing store | — | Section 12 worked cases | Pending |
| 13 Updates, clearing, driver | — | Case 16 and section 13 cases | Pending |
| 14 Container rectangles, grid rebuild | — | Section 14 worked cases | Pending |
| 15 Toggles, lifecycle remainder, helpers | — | Section 15 worked cases | Pending |
| 16 Inventory | — | Closure audit against preceding rows | Pending |
