# Camera owner method inventory

Behaviour source: `docs/specs/camera.md` §§1–2, 4, 6. `Xform` storage is reused
unchanged. No owner output is delegated. Foreign terrain queries and synchronous
message-server sends are callbacks. Foreign city-corner data uses view-utilities §4.2's published
table, or explicit corner inputs. No ABI/plugin registration is part of this module.

| Owner / spec method | Public core body |
|---|---|
| cS3DCamera constructor / factory | `S3dCamera::new`, `Default` |
| GetGZCLSID / QueryInterface / AddRef / Release | `class_id`, `query_interface`, `add_ref`, `release` |
| deleting destructor | Rust allocation lifetime; reference ledger never frees a borrowed pointer |
| SetViewport / SetSubview / reset subview / SetDepthSize | `set_viewport`, `set_subview`, `reset_subview`, `set_depth_size` |
| SetOrtho | `set_ortho` |
| Perspective state evaluation only | `set_frustum` is an implementation extension; CA3 native entry/signature/counter BLOCKED |
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
| Init / AttachToCamera / DetachFromCamera | `init` accepts an optional camera; `attach_to_camera`, `detach_from_camera`; detach retains Init, destructor does not detach the reference ledger |
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
| SendPostViewChangeMessage / inline senders | `send_post_view_change_message`, synchronous `ViewMessageServer::send` over live `LiveViewMessage::data1/data2`; `messages` / `take_messages` are diagnostic copies only |
| Read / Write | `read`, `write`, existing `InStream` / `OutStream` interfaces |

Other public accessors expose owned state for core consumers and discriminating
tests. `shutdown` is an adapter Detach convenience, retaining Init; there is no
native Shutdown slot. Rust Arc destruction remains allocation policy, separate
from the native reference ledger. Getters are immutable and both owners are `Send + Sync`.

Projection and view caches are maintained eagerly by writers, preserving change
counters while avoiding mutable matrix caches in getters. This is an eager-cache
policy, not the native lazy-cache timing/counter-observation contract. Constructor
projection state is 1; the native cached-counter transition 0→1 on first getter is
not represented. Message callbacks run inside commands without camera locks and
may reenter. Pair payload references address the control fields at callback time;
each post-change triple reuses one message object. A native adapter still needs
message-server-2 slot 3 and native object/field identities. No shadow or real-game
replacement credit is claimed.

## Whole-owner audit against public native inventory (§7)

CA4–CA6 are answered DIFFERS and reconciled here using §6's independent
discriminators, not the parent's 39 tests. CA3 constructor/look-at portions are
resolved; its perspective entry remains blocked. **Neither class is a complete
native owner.** No family, complete-owner, shadow or replacement credit is granted.

| Native body group | Actual body / residual |
|---|---|
| S3D 007FF040 / 005F47F0 / 007FF020 / 007FF130 | QueryInterface/AddRef/Release ledger exists; deleting destruction is Rust lifetime policy, not native heap/free flag dispatch |
| S3D 007FF010 / 007FF930 / 00800320 | ID/constructor implemented; factory allocation is Rust policy |
| S3D 007FF070 / 007FF080 / 007FF0B0 / 007FF150 | Depth/subview/reset/viewport bodies implemented |
| S3D 007FF0E0 / 007FF190 / 007FF3C0 / 007FF3D0 / 007FFED0 | Transform access/reset/counter/rebuild implemented; explicit mutation instead of native mutable transform pointer; eager view cache policy |
| S3D 007FF230 / 007FF250 / 007FF280 / 007FFD60 | Position/remembered-target/look-at implemented; remembered-counter synchronization not modeled |
| S3D 007FF0F0 / 007FF110 / 007FF120 / 007FF2C0 / 007FF2E0 / 007FF340 | Bias/type/state/pixel/ortho/volume bodies implemented |
| S3D 007FF580 / 007FFEF0 | Both projection-state builders implemented; lazy counter timing not modeled; native perspective setter unidentified (CA3) |
| S3D 007FFF10 / 00800120 / 00800340 | Project/UnProject/eye ray bodies implemented |
| Control 007CB940 / 004BDF00 / 007CB920 / 007CBB10 / 007CBBA0 / 007CCBF0 / 007CCC20 | Read/Write/ID implemented; native QI/AddRef/Release/deleting destructor NOT implemented; no automatic ledger detach |
| Control 007CC990 / 007CCC00 | Constructor implemented; native heap factory absent |
| Control 007CB970 / 007CD200 / 007CDB20 | Detach/attach/Init corrected, including camera-less Init and acquired-before-release ordering |
| Control 007CB990 / 007CB9B0 / 007CB9F0 / 007CBA10 | Signed viewport, clip getters and partial-write `rotation_corner_into` implemented |
| Control 007CBC40 / 007624E0 / 007CBC60 | Virtual zoom/magnification/velocity bodies implemented |
| Control 007CBCE0 / 007CBD70 / 007CBE40 / 007CC290 | Conversions/zoom/position implemented; invalid zoom lookup containment is explicitly safety policy |
| Control 007CC000 / 007CC0B0 / 007CC5B0 | Synchronous pre/post/init callbacks implemented; native message allocation/slot binding remains adapter work |
| Control 007CC650 / 007CCC60 / 007CD000 / 007CD330 | Directed clamp/bounds/target clamp/angle scroll corrected; portable mixed float/double arithmetic is not x87 bit-exact proof |
| Control 007CCF80 | Generic update-kind helper NOT exposed; view-utilities update callback and full kind-dependent dispatch remain missing |
| Control 007CD540 / 007CD6A0 / 007CD6C0 / 007CD6E0 | Rotation/custom magnification implemented; separate rotate-left/right command bodies missing |
| Control 007CD810 / 007CDA20 / 007CDAA0 | Target/tweak/city-size bodies corrected; optional foreign view-utilities position update missing |
| Control 007CDC50 / 007CDE60 / 007CE080 / 007CE0E0 | Float zoom/integer pair corrected; separate zoom-in/out bodies missing |
| Associated 007CBA90 / 007CC200 | Ground helper implemented; vector-copy helper uses Rust Vec storage, not an additional camera owner |

Later work must finish the per-class residual bodies and native integration, then
run independent same-process shadow and exercised replacement before promoting
those claims. This answer-correction phase does not reset the logical phase cap.
