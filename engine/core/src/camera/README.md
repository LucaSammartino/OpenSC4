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
| cSC4CameraControl shared AddRef | `add_ref`, `reference_count`; remaining COM contracts open CA10 |
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
| Control 007CB940 / 004BDF00 / 007CB920 / 007CBB10 / 007CBBA0 / 007CCBF0 / 007CCC20 | Read/Write/ID and literal AddRef ledger implemented; QueryInterface/Release/deleting flags open CA10; no automatic ledger detach |
| Control 007CC990 / 007CCC00 | Documented constructor fields implemented; pre-init derived conversion defaults provisional (CA7); native heap factory absent |
| Control 007CB970 / 007CD200 / 007CDB20 | Detach/attach/Init corrected, including camera-less Init and acquired-before-release ordering |
| Control 007CB990 / 007CB9B0 / 007CB9F0 / 007CBA10 | Signed viewport, clip getters and partial-write `rotation_corner_into` implemented |
| Control 007CBC40 / 007624E0 / 007CBC60 | Virtual zoom/magnification/velocity bodies implemented |
| Control 007CBCE0 / 007CBD70 / 007CBE40 / 007CC290 | Conversions/zoom/position implemented; invalid zoom lookup containment is explicitly safety policy |
| Control 007CC000 / 007CC0B0 / 007CC5B0 | Synchronous pre/post/init callbacks implemented; native message allocation/slot binding remains adapter work |
| Control 007CC650 / 007CCC60 / 007CD000 / 007CD330 | Directed clamp/bounds/target clamp/angle scroll corrected; portable mixed float/double arithmetic is not x87 bit-exact proof |
| Control 007CCF80 | Generic update-kind dispatch Unsupported: open CA9; inventory gives computation order but not kind-dependent pre/post messages |
| Control 007CD540 / 007CD6A0 / 007CD6C0 / 007CD6E0 | Rotation/custom magnification implemented; separate rotate-left/right commands Unsupported: open CA8 |
| Control 007CD810 / 007CDA20 / 007CDAA0 | Target/tweak/city-size bodies implemented; tweak notifies optional `CameraViewUtilities` after computation and builds empty bounds under the same guard |
| Control 007CDC50 / 007CDE60 / 007CE080 / 007CE0E0 | Float zoom/integer pair implemented; separate zoom-in/out commands Unsupported: open CA8 |
| Associated 007CBA90 / 007CC200 | Ground helper implemented; vector-copy helper uses Rust Vec storage, not an additional camera owner |

Later work must finish the per-class residual bodies and native integration, then
run independent same-process shadow and exercised replacement before promoting
those claims. This answer-correction phase does not reset the logical phase cap.

## Owner closure

Scope: **cSC4CameraControl only**, against camera.md as read on 2026-10-23.
The owner is **not closed**: CA7 and newly filed CA8–CA10 block a complete
contract. Unsupported entries below have no guessed native implementation.
The cS3DCamera residuals above are outside this lane's owner target.

| Owner / spec section | Core item | Covering test or open contract |
|---|---|---|
| cSC4CameraControl §2.1 state | `Default`, immutable state getters, `add_ref` | `independent_ca4_camera_less_init_detach_and_raw_read`, `tweak_optional_view_observes_computed_camera_before_empty_bounds_build`; pre-init derived storage **open: CA7** |
| cSC4CameraControl §2.2 zoom/projection | `update_camera_zoom_and_rotation_params` | `published_zoom_tables_and_yaws_all_combinations`, `rotation_corners_and_clip_city_minimum` |
| cSC4CameraControl §2.3 placement/snapping | `rotation_corner_into`, `update_camera_position_params` | `eye_offset_and_plane_equation_are_independently_constrained`, `pixel_corner_is_snapped_and_tweak_changes_it_directly`, `independent_ca4_camera_less_init_detach_and_raw_read` |
| cSC4CameraControl §2.4 conversions | conversion methods, `intersect_with_ground_plane` | `camera_plane_conversion_has_independent_axis_and_origin_results`, `ground_intersection_has_exact_threshold_and_rejects_behind`; pre-init scale **open: CA7** |
| cSC4CameraControl §2.5 bounds/clamps | `update_scroll_bounds[_with_corners]`, `clamp_scroll_to_bounds`, `clamp_target_to_scroll_bounds` | `scroll_polygon_preserves_published_lift_and_six_points`, `independent_ca6_outside_boundary_fraction_and_length_discriminators`, §6.1 tests below; CA1 concerns interpretation only, formula implemented literally |
| cSC4CameraControl §2.6 commands/lifecycle | `init`, attach/detach, setters and angle scroll | `independent_ca5_reset_order_rotation_preservation_and_float_equality`, `independent_ca5_tweak_and_signed_viewport_preserve_existing_bounds`, `tweak_optional_view_observes_computed_camera_before_empty_bounds_build`, `scrolling_uses_only_foreign_terrain_query_then_ground_fallback`, `target_validation_and_velocity_nan_only_test`, `attach_detach_init_once_and_native_destructor_does_not_detach`; extra commands named only in §7 **open: CA8**, generic helper dispatch **open: CA9** |
| cSC4CameraControl §2.7 synchronous messages | `LiveViewMessage`, `send_sequence`, `ViewMessageServer` | `synchronous_callbacks_observe_live_fields_reentry_and_reused_message`, `rotation_message_order_and_pairs_are_exact`; generic kind mapping **open: CA9** |
| cSC4CameraControl §2.8 save record | `read`, `write` | `save_bytes_exact_and_init_copies_loaded_current_pair_to_new`, `independent_ca4_failed_stream_attempts_all_destinations_without_rollback`, `record_bad_version_truncation_and_sticky_write_error` |
| cSC4CameraControl §3 users | foreign view/terrain/message traits; core camera handle | `owners_can_be_read_on_a_second_thread`, `scrolling_uses_only_foreign_terrain_query_then_ground_fallback`; native registration/host wiring outside core scope |
| cSC4CameraControl §4 quirks | zoom tables and projection setters | `published_zoom_tables_and_yaws_all_combinations`, `zoom_fractional_magnification_refusal_and_virtual_threshold` |
| cSC4CameraControl §5 questions | literal §2.5 lift; provisional pre-init storage | **open: CA7**; CA1 does not block arithmetic; CA3 is another owner |
| cSC4CameraControl §6 conformance | §2 methods above | cases 1,5–9,12–16 mapped below (2–4,10–11 concern cS3DCamera, also tested) |
| cSC4CameraControl §6.1 scroll discriminators | directed `clamp_polygon`, target clamp, angle scroll | `independent_ca6_outside_boundary_fraction_and_length_discriminators`, `scroll_clamp_disabled_empty_inside_and_outward_cut_are_exact`, `scroll_clamp_corner_reverse_winding_outside_and_short_edge`, `scroll_exhaustion_reaches_all_n_projections_before_returning_original`, `tied_scroll_crossing_projects_right_edge_before_history_stop`, `below_threshold_target_clamp_keeps_target_velocity_and_sends_nothing`, `ground_failure_contracts` |
| cSC4CameraControl §7 native inventory | implemented method inventory above | commands **open: CA8**; generic helper **open: CA9**; QI/Release/deleting flags **open: CA10**; constructor derived fields **open: CA7** |

### Section 6 conformance coverage

Tests are core analytic discriminators, not captured game outputs or a native
shadow. `clamp_polygon_observed` reports completed edge projections to prove
budget exhaustion and first-edge tie behavior; the production wrapper uses a
no-op observer. The former stationary U-polygon test has been renamed: it exits
without an edge selection and never demonstrated exhaustion.

| Spec case | Test |
|---|---|
| 1 tables | `published_zoom_tables_and_yaws_all_combinations` |
| 2 ortho matrix | `specified_ortho_matrix_and_independent_points` |
| 3 project/unproject | `specified_ortho_matrix_and_independent_points` |
| 4 look-at | `look_at_axes_position_and_counter_are_owned` |
| 5 eye placement | `eye_offset_and_plane_equation_are_independently_constrained` |
| 6 float zoom | `zoom_fractional_magnification_refusal_and_virtual_threshold` |
| 7 message order | `rotation_message_order_and_pairs_are_exact` |
| 8 record/version | `save_bytes_exact_and_init_copies_loaded_current_pair_to_new`, `record_bad_version_truncation_and_sticky_write_error` |
| 9 ground ray | `ground_intersection_has_exact_threshold_and_rejects_behind` |
| 10 constructor | `independent_ca3_constructor_and_degenerate_look_at` |
| 11 degenerate look-at | `independent_ca3_constructor_and_degenerate_look_at` |
| 12 failed record mutation | `independent_ca4_failed_stream_attempts_all_destinations_without_rollback` |
| 13 lifecycle/raw data | `independent_ca4_camera_less_init_detach_and_raw_read` |
| 14 magnification/command result | `independent_ca5_reset_order_rotation_preservation_and_float_equality` |
| 15 input/viewport edges | `independent_ca5_reset_order_rotation_preservation_and_float_equality`, `independent_ca5_tweak_and_signed_viewport_preserve_existing_bounds` |
| 16 synchronous observations | `synchronous_callbacks_observe_live_fields_reentry_and_reused_message` |

Foreign `CameraViewUtilities` supplies corner inputs and observes computed camera
position. With no host configured, the existing documented view-utilities corner
table remains the pure-core convenience input. Tweak's optional-view guard is
literal: no host means no bounds rebuild there. The generic helper's host guard
and kind dispatch await CA9. No input host, device, ABI or plugin wiring is added.
