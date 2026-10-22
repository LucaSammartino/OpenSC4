# SC4ViewUtilities contract inventory

Source: `docs/specs/view-utilities.md` §§1–7,9,11–12, including the independent
native prose vectors and VU2–VU11 answers; model-instances §§2.1/9.1 describe the
same picking/traversal operation. No model-instance implementation was changed.

The catalogue is **50 entries / 19748 bytes**, not 49 / 19670: 48 attributed
entries / 19574 bytes (including the free rotation helper, 78 bytes), plus two
separately owned min/max helpers / 174 bytes. The table below maps each body
exactly once. It records portable implementations/adapters, **not 50 native-exact
bodies or whole-owner completion**. Numeric limits VU12/VU13 remain open.

Status: I = integer/bit/copy semantics implemented under stated valid-input and
nearest/even masked-exception preconditions; P = arithmetic/order implemented
under the portable numeric policy below, native bit identity unproved; C = checked
boundary replaces indeterminate memory/fault behavior; R = Rust lifetime adapter,
not a native vtable/heap implementation. A row may have multiple statuses.

| # | Spec entry / address | Implementation (status) |
|---|---|---|
| 1 | Constructor 007F6DF0 | `ViewUtilities::new`, `Default` (I/R) |
| 2 | UpdateCameraPosition 007F83F0 | `update_camera_position` (P/C) |
| 3 | GetCameraViewAnglesForRotation 007F7780 | `get_camera_view_angles_for_rotation` (P/C) |
| 4 | GetObjectYRotationAngle 007F6CB0 | `get_object_y_rotation_angle` (P) |
| 5 | GetCityWidthFromOrthoView 007F8300 | `get_city_width_from_ortho_view` (P/C) |
| 6 | GetZoomAndRotation 007F66A0 | `get_zoom_and_rotation` (I) |
| 7 | ViewportUnitsToWorldUnits 007F6690 | `viewport_units_to_world_units` (P) |
| 8 | GetFrustumAABBCityClipped 007F7F60 | `get_frustum_aabb_city_clipped`, ordered clipping (P/C) |
| 9 | IsBoundingBoxInViewFrustum 007F6770 | `is_bounding_box_in_view_frustum` (P) |
| 10 | IsBoundingBoxInDrawFrustum 007F6910 | `is_bounding_box_in_draw_frustum` (P) |
| 11 | IsPointInViewFrustum 007F7E50 | `is_point_in_view_frustum` (P) |
| 12 | ProjectPointToViewCoordinates 007F77D0 | `project_point_to_view_coordinates` (P) |
| 13 | ProjectPointToFixedViewCoordinates 007F6710 | `project_point_to_fixed_view_coordinates` (P) |
| 14 | ProjectPointFromViewCoordinatesToViewportCoordinates 007F66C0 | `project_point_from_view_coordinates_to_viewport_coordinates` (P) |
| 15 | ProjectPointToViewportCoordinates 007F7850 | `project_point_to_viewport_coordinates` (P) |
| 16 | ProjectPointToViewportCoordinates2D 007F78E0 | `project_point_to_viewport_coordinates_2d` (P) |
| 17 | ProjectPointToViewportZ 007F78A0 | `project_point_to_viewport_z` (P) |
| 18 | UnProjectPointFromViewCoordinatesToWorldCoordinates 007F7920 | `unproject_point_from_view_coordinates_to_world_coordinates` (P) |
| 19 | UnProjectPointFromViewportCoordinatesToWorldCoordinates 007F79B0 | `unproject_point_from_viewport_coordinates_to_world_coordinates` (P) |
| 20 | ProjectPointFromViewportCoordinatesToFixedViewCoordinates 007F8390 | `project_point_from_viewport_coordinates_to_fixed_view_coordinates` (P) |
| 21 | ProjectBoundingBoxToViewportRectangle 007F7A40 | `project_bounding_box_to_viewport_rectangle`, bias conversion (P/I) |
| 22 | ViewportPositionToRay 007F9080 | `viewport_position_to_ray`, failure preserves both outputs (P) |
| 23 | GetRayIntersectionWithGroundPlane 007F6AB0 | `get_ray_intersection_with_ground_plane`, unguarded (P) |
| 24 | GetViewportPointIntersectionWithGroundPlane 007F9160 | `get_viewport_point_intersection_with_ground_plane` (P/C) |
| 25 | RayIntersectsGroundPlane 007F6B50 | `ray_intersects_ground_plane`, tolerant predicate (P) |
| 26 | GetViewportRectGroundCorners 007F9EA0 | `get_viewport_rect_ground_corners`, TL/TR/BL/BR (P/C) |
| 27 | GetCityCornerPosition 007F6BA0 | `get_city_corner_position`, W-for-z quirk (I) |
| 28 | GetVectorForEdge 007F6D10 | `get_vector_for_edge_into`, signed/partial writes (I) |
| 29 | GetVisibleEdges 007F7750 | `get_visible_edges`, wrapping signed remainder (I) |
| 30 | ClipWorldPositionToCityPosition 007F7710 | `clip_world_position_to_city_position` (I) |
| 31 | ClipPositionToCity2D 007F6620 | `clip_position_to_city_2d` (I) |
| 32 | RoundDownToNearestBlockBoundary 007F6D70 | `round_down_to_nearest_block_boundary` (I/C) |
| 33 | RoundUpToNearestBlockBoundary 007F6DB0 | `round_up_to_nearest_block_boundary` (I/C) |
| 34 | BuildDrawFrustumDataFromViewportRect 007F9F50 | `build_draw_frustum_data_from_viewport_rect` (P/C) |
| 35 | GetGroundPlaneVisibleAreaRectangle 007F9B40 | `get_ground_plane_visible_area_rectangle` (P/C) |
| 36 | TrianglePick 007F6AF0 | `triangle_pick`, rounded hit Euclidean distance (P) |
| 37 | MeshPick 007F91C0 | `mesh_pick_checked`, first improvement (P/C) |
| 38 | ModelPick 007F9A40 | `model_pick_checked`, all meshes/reset transform (P/C) |
| 39 | MinOfFourFloats 007F6550, separate owner 87B | `min_of_four_floats`, ordered strict scan (I) |
| 40 | MaxOfFourFloats 007F65B0, separate owner 87B | `max_of_four_floats`, ordered strict scan (I) |
| 41 | RotateVectorByViewRotation 007F6CC0, free helper 78B | `rotate_vector_by_view_rotation` (I, quiet-NaN/finite inputs) |
| 42 | Primitive triangle count 0080AE80 | `PrimitiveGroup::triangle_count`, signed raw count (I) |
| 43 | Primitive vertex indices 0080AEF0 | `primitive_vertex_indices`, unknown preserves output (I) |
| 44 | Iterator deleting destructor 0080B000 | borrowed `TriangleIterator` scope/drop, no buffer ownership (R; native flags/heap ABI not implemented) |
| 45 | Iterator vertex copy 0080B020 | `Mesh::from_vertex_data`, `Mesh::vertex`/`next_triangle`, checked 12-byte reads including overlap (I/C) |
| 46 | Iterator index decode 0080B090 | `decode_indices`, eight formats; numeric extremes incomplete VU13 (I/C, partial) |
| 47 | Iterator select group 0080B320 | `TriangleIterator::select_group`, indexed/nonindexed first offset (I/C/R) |
| 48 | Iterator next 0080B3B0 | `TriangleIterator::next_triangle`, signed stop preserves output (I/C) |
| 49 | Point-array AABB 005D6900, reach helper 668B | `point_array_aabb`, `Aabb::from_points`, first NaN retained (I/C) |
| 50 | Cross 005D6F10, reach helper 59B | `Vec3::cross` unaliased; `cross_in_storage` sequential aliased writes (P/C) |

## Internal work and external inputs

- `Frustum::build`: half-extents without recentering, six planes, two ground
  lines, and previous-far-corner compounding for perspective. City clipping uses
  subject 3/0/ground(0)/ground(3), clockwise city edges z>=0/x>=0/z<=D/x<=W,
  ordered emission, boundary duplicates and capacity truncation.
- `Frustum::contains_box`: strict support-corner tests against four sides only.
- Owned `OnceLock` caches: full viewport ground corners and visible rectangles.
  Both invalidate on update/draw; cached getters take shared references and
  `ViewUtilities` is `Send + Sync`. Visible rectangles cast four fresh full-view
  rays on their own miss. Full draw populates only the ground-corner cache;
  narrowed draw casts only its four rectangle rays, leaving that cache clear.
- `PrimitiveGroup::triangle_count` and `triangle_indices`: all eight §7.1 rows,
  including degenerate types 3–5 and unusual types 6–7. This supplies the named
  count/index behavior at 0080AE80/0080AEF0. Raw counts are signed, not saturated;
  undersized strips stop without writing. The prescribed mesh walk uses only
  u16 indices; the eight-format generic neighbour is separately exposed.
- `Mesh::from_vertex_data`: owned little-endian XYZ data, u16 stride/count and
  optional u16 indices. Transforms cover every kind and first-closer stopping;
  model traversal resets the transform and continues across all meshes.
- `CameraState` copies raw camera/control fields. `CameraRayProvider` supplies
  only the camera's eye ray, as §4.1 requires. `ModelInstance` supplies only raw
  transform data. No owner algorithm or result is provided by either interface.
- Rust renderer/city setters and snapshot getters are input/inspection adapters,
  **not** extra lifecycle/setter bodies. The native 50-body family has no utility
  Init/Shutdown/renderer setter/camera setter/AddRef/Release/deleting destructor.

## Numeric limits and checked boundaries

The portable evaluation policy uses nearest/even IEEE f32 value operations,
with exact integer accumulation for yaw/rotation-only angles and f64 intermediates for normalization,
viewport scale extents, row sums and known default-transform stores. No FMA is
used. The shipped angle/table words are exact; yaw stores once after the
rotation multiply/add. sx/sy inverses use stored scales; sz inverse uses the
retained quotient. Up is formed before right normalization. The first default
transform vertex preserves the documented different x/y versus z store order.
Trig uses portable f64 libm then a float32 store, **not x87 emulation**. The spec
does not promise portable libm bit identity. Neither f32 expressions nor f64
intermediates are asserted equivalent to every inherited x87 precision/rounding
mode. VU12 requests that mode and the remaining store graph; P rows remain
numerically incomplete for native-bit purposes. Tests use exact expectations
where representable and tolerances for analytic floating results, never a
native-exact label for those tolerances.

Native UB/faults have explicit checked adapters: missing eye ray makes the ground
wrapper return false and preserve out (native returns true with indeterminate
coordinates); rectangle ground outputs return None; failed visible-area queries
cache None until invalidated. Near-parallel clip intersections return Err and
city-clipped bounds return None. Block zero and IDIV overflow return None;
negative blocks and wrapping arithmetic are supported. Masked `_ftol2` invalid
conversion follows the documented zero low word. Failed narrowed draw retains
old geometry/flag after rectangle stores/cache invalidations. Full draw copies
view geometry before its checked ground failure, preserving old draw ground data.
Out-of-range zoom returns None rather than reading unrelated memory.

Malformed mesh references abort with `InvalidMeshReference`, preserving triangle
output on a failed iterator step; checked model traversal aborts on that error,
and bool adapters return false. They are never
silently skipped. Sufficient overlapping vertex storage with stride below 12 is
accepted. Generic numeric indices narrow double to f32 then floor only within
the explicit finite bias range [-2^22,2^22); outside it the decoder preserves
outputs and returns None pending VU13. Rust resources/iterator have owned/borrowed
lifetimes, not the native heap/vtable destructor ABI. Quiet NaN comparisons and
signed-zero writes follow the stated operations; signaling exceptions and
non-default processor modes are not emulated.

Native cached getters 007F7A00/007F8290/007F8340 are outside the 50-row accounting;
Rust snapshots expose their cached data. Shared constructor 0080AE60 is not a
51st body. Geometry triangle/plane/clipping dependencies remain geometry-owned.
No exact implemented-byte subtotal is inferred from mapped/adapted rows.

No ABI, plugin, default/shadow/replacement registration or runtime claim.
