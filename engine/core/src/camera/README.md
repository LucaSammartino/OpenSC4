# Camera owner method inventory

Behaviour source: `docs/specs/camera.md` §§1–2, 4, 6. `Xform` storage is reused
unchanged. No owner output is delegated. The foreign terrain ray cast is the
only callback. Foreign city-corner data uses view-utilities §4.2's published
table, or explicit corner inputs. No ABI/plugin registration is part of this module.

| Owner / spec method | Public core body |
|---|---|
| cS3DCamera constructor / factory | `S3dCamera::new`, `Default` |
| GetGZCLSID / QueryInterface / AddRef / Release | `class_id`, `query_interface`, `add_ref`, `release` |
| deleting destructor | Rust allocation lifetime; reference ledger never frees a borrowed pointer |
| SetViewport / SetSubview / SetDepthSize | `set_viewport`, `set_subview`, `set_depth_size` |
| SetOrtho / perspective box | `set_ortho`, `set_frustum` (CA3) |
| SetPixelOffset / SetDepthBias | `set_pixel_offset`, `set_depth_bias` |
| GetViewVolume | `view_volume` |
| SetPosition / GetPosition | `set_position`, `position` |
| SetLookAt / GetLookAt | `set_look_at`, `look_at` |
| ResetXforms / LocalXform / ViewXform | `reset_xforms`, `local_xform`, `view_xform`; safe explicit `set_local_xform` |
| UpdateViewXform | `update_view_xform` |
| ProjectionMatrix / _UpdateProjection | `projection_matrix`, `update_projection`; inverse cached alongside |
| GetProjectionType / GetViewState | `projection_type`, `view_state` |
| Project / UnProject / GetEyeRay(s) | `project`, `unproject`, `eye_ray`, `eye_rays` |
| cSC4CameraControl constructor / identity | `CameraControl::new`, `Default`, `class_id` |
| Init / AttachToCamera / DetachFromCamera | `init`, `attach_to_camera`, `detach_from_camera`; `Drop` releases attachment |
| UpdateCameraZoomAndRotationParams | `update_camera_zoom_and_rotation_params` |
| UpdateCameraPositionParams | `update_camera_position_params` |
| ConvertWorldPointToCameraPlanePoint | `convert_world_point_to_camera_plane_point` |
| ConvertCameraPlanePointToWorldPoint | `convert_camera_plane_point_to_world_point` |
| IntersectWithGroundPlane | module `intersect_with_ground_plane` |
| UpdateScrollBounds | `update_scroll_bounds`, explicit foreign-input `update_scroll_bounds_with_corners` |
| ClampScrollToBounds / ClampTargetToScrollBounds | `clamp_scroll_to_bounds`, `clamp_target_to_scroll_bounds` |
| SetZoom / SetZoomAndRotation / SetRotation | `set_zoom`, `set_zoom_and_rotation`, `set_rotation` |
| SetCustomMagnification / GetCustomMagnification | `set_custom_magnification`, `custom_magnification` |
| SetViewTarget / SetViewTargetVelocity | `set_view_target`, `set_view_target_velocity` |
| ScrollByViewportAngle | `scroll_by_viewport_angle` |
| SetCameraPositionTweakAdd / SetCitySize | `set_camera_position_tweak_add`, `set_city_size` |
| SetViewportSize / GetViewportSize | `set_viewport_size`, `viewport_size` |
| GetVirtualZoom / GetDistancesToClipPlanes | `virtual_zoom`, `distances_to_clip_planes` |
| SendPostViewChangeMessage / inline senders | `send_post_view_change_message`, typed ordered `messages` / `take_messages` |
| Read / Write | `read`, `write`, existing `InStream` / `OutStream` interfaces |

Other public accessors expose owned state for core consumers and discriminating
tests. `shutdown` is a pure-core lifetime convenience (CA4); it is not an invented
game callback. Getters are immutable and both owners are `Send + Sync`.

Projection and view caches are maintained eagerly by writers, preserving change
counters while avoiding mutation in getters. Messages capture the ordered values
for a future host adapter; no raw pointers or original message method outputs are
needed. No shadow or real-game replacement credit is claimed.

Coordinator qualification (train79): this is a published-method implementation inventory, not certification of complete native-owner behavior. CA3-CA6 remain unresolved and retain provisional choices. No complete-owner, shadow or replacement coverage is granted by this module.
