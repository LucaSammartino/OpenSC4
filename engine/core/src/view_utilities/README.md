# SC4ViewUtilities contract inventory

Source: `docs/specs/view-utilities.md` §§1–7,9,11 only. This module implements the
38 contracts below (constructor plus 37 named methods). This does **not** reconcile
the task's 49 attributed functions / 19670 bytes: no complete address inventory is
published in that source. Complete-owner credit is explicitly refused (VU4).
These are source-contract implementations; VU5–VU10 prevent a native-exact claim.

| # | Spec entry / address | Implementation |
|---|---|---|
| 1 | Constructor 007F6DF0 | `ViewUtilities::new`, `Default` |
| 2 | UpdateCameraPosition 007F83F0 | `update_camera_position` |
| 3 | GetCameraViewAnglesForRotation 007F7780 | `get_camera_view_angles_for_rotation` |
| 4 | GetObjectYRotationAngle 007F6CB0 | `get_object_y_rotation_angle` |
| 5 | GetCityWidthFromOrthoView 007F8300 | `get_city_width_from_ortho_view` |
| 6 | GetZoomAndRotation 007F66A0 | `get_zoom_and_rotation` |
| 7 | ViewportUnitsToWorldUnits 007F6690 | `viewport_units_to_world_units` |
| 8 | GetFrustumAABBCityClipped 007F7F60 | `get_frustum_aabb_city_clipped`, owned polygon clipping |
| 9 | IsBoundingBoxInViewFrustum 007F6770 | `is_bounding_box_in_view_frustum` |
| 10 | IsBoundingBoxInDrawFrustum 007F6910 | `is_bounding_box_in_draw_frustum` |
| 11 | IsPointInViewFrustum 007F7E50 | `is_point_in_view_frustum` |
| 12 | ProjectPointToViewCoordinates 007F77D0 | `project_point_to_view_coordinates` |
| 13 | ProjectPointToFixedViewCoordinates 007F6710 | `project_point_to_fixed_view_coordinates` |
| 14 | ProjectPointFromViewCoordinatesToViewportCoordinates 007F66C0 | `project_point_from_view_coordinates_to_viewport_coordinates` |
| 15 | ProjectPointToViewportCoordinates 007F7850 | `project_point_to_viewport_coordinates` |
| 16 | ProjectPointToViewportCoordinates2D 007F78E0 | `project_point_to_viewport_coordinates_2d` |
| 17 | ProjectPointToViewportZ 007F78A0 | `project_point_to_viewport_z` |
| 18 | UnProjectPointFromViewCoordinatesToWorldCoordinates 007F7920 | `unproject_point_from_view_coordinates_to_world_coordinates` |
| 19 | UnProjectPointFromViewportCoordinatesToWorldCoordinates 007F79B0 | `unproject_point_from_viewport_coordinates_to_world_coordinates` |
| 20 | ProjectPointFromViewportCoordinatesToFixedViewCoordinates 007F8390 | `project_point_from_viewport_coordinates_to_fixed_view_coordinates` |
| 21 | ProjectBoundingBoxToViewportRectangle 007F7A40 | `project_bounding_box_to_viewport_rectangle` |
| 22 | ViewportPositionToRay 007F9080 | `viewport_position_to_ray` |
| 23 | GetRayIntersectionWithGroundPlane 007F6AB0 | `get_ray_intersection_with_ground_plane` |
| 24 | GetViewportPointIntersectionWithGroundPlane 007F9160 | `get_viewport_point_intersection_with_ground_plane` |
| 25 | RayIntersectsGroundPlane 007F6B50 | `ray_intersects_ground_plane` |
| 26 | GetViewportRectGroundCorners 007F9EA0 | `get_viewport_rect_ground_corners` |
| 27 | GetCityCornerPosition 007F6BA0 | `get_city_corner_position` |
| 28 | GetVectorForEdge 007F6D10 | `get_vector_for_edge` |
| 29 | GetVisibleEdges 007F7750 | `get_visible_edges` |
| 30 | ClipWorldPositionToCityPosition 007F7710 | `clip_world_position_to_city_position` |
| 31 | ClipPositionToCity2D 007F6620 | `clip_position_to_city_2d` |
| 32 | RoundDownToNearestBlockBoundary 007F6D70 | `round_down_to_nearest_block_boundary` |
| 33 | RoundUpToNearestBlockBoundary 007F6DB0 | `round_up_to_nearest_block_boundary` |
| 34 | BuildDrawFrustumDataFromViewportRect 007F9F50 | `build_draw_frustum_data_from_viewport_rect` |
| 35 | GetGroundPlaneVisibleAreaRectangle 007F9B40 | `get_ground_plane_visible_area_rectangle` |
| 36 | TrianglePick 007F6AF0 | `triangle_pick` |
| 37 | MeshPick 007F91C0 | `mesh_pick` |
| 38 | ModelPick 007F9A40 | `model_pick` |

## Internal work and external inputs

- `Frustum::build`: current near/far corners, axis-aligned bounds, six planes,
  two ground lines, inclusive convex city clipping. Perspective uses current
  near corners as the spec's explicit choice, without stale-state compounding.
- `Frustum::contains_box`: strict support-corner tests against four sides only.
- Owned `OnceLock` caches: full viewport ground corners and visible rectangles.
  Both invalidate on update/draw; cached getters take shared references and
  `ViewUtilities` is `Send + Sync`. §6 uses full viewport points literally (VU9).
- `PrimitiveGroup::triangle_count` and `triangle_indices`: all eight §7.1 rows,
  including degenerate types 3–5 and unusual types 6–7. This supplies the named
  count/index behavior at 0080AE80/0080AEF0, without asserting those helpers are
  part of the 49 owner functions. The generic eight-format reader at 0080B090
  belongs to the dependency description; the prescribed walk uses only u16.
- `Mesh::from_vertex_data`: owned little-endian XYZ data, u16 stride/count and
  optional u16 indices. Transforms cover every kind and first-closer stopping;
  model traversal resets the transform and continues across all meshes.
- `CameraState` copies raw camera/control fields. `CameraRayProvider` supplies
  only the camera's eye ray, as §4.1 requires. `ModelInstance` supplies only raw
  transform data. No owner algorithm or result is provided by either interface.
- Rust renderer/city setters and snapshot getters are input/inspection adapters,
  **not** implementations of unspecified native lifecycle/setter bodies.

## Unreconciled owner entries and conventions

- VU4: full 49-function address inventory and missing lifecycle/setter contracts.
  At least 11 attributed entries cannot be matched from the 38 contracts above;
  aliases/helper ownership could change that arithmetic. Do not manufacture an
  inventory or count external geometry helpers as owner bytes.
- VU11: the introductory related-spec paragraph names
  `RotateVectorByViewRotation` (007F6CC0) but provides no body in the task's
  behavior source. It is explicitly **unimplemented**, not silently treated as
  fixed-view projection. The other 10 entries remain unidentified if it belongs
  to the 49; that ownership itself needs reconciliation.
- VU5–VU10: exact float bits/evaluation, geometry dependency edge semantics,
  failed-ray temporaries, rounding/overflow, cache identity, malformed resources
  and matrix orientation. Literal choices are documented beside the code.
- VU2 unused constructor fields remain owned constants; VU3 off-centre frusta
  retain the specified half-extents behavior even when inconsistent with projection.

No ABI, plugin, default/shadow/replacement registration or runtime claim.
