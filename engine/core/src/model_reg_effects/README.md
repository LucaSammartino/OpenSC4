<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Model registration effects

`ModelRegEffects` implements the `cSC4ModelRegEffects` owner from
`docs/specs/graphics-formats.md`. It is core-only: no C ABI exports or plugin
hooks. Foreign instance/model access, effect creation/control, and CRT locale
byte mapping are supplied through public interfaces. It does not discover effects
for buildings, props, or flora on its own; the embedding owner supplies instances.

The existing `fc4_formats::s3d::Model::parse` reads REGP, including lower-cased
C-string names, duplicate-name append, counts and seven binary32 fields. The core
uses that exact `Regions` map representation rather than adding another parser.
REGP vectors are borrowed: direct additions take lifetime-checked slices and
discovered entries retain keys resolving the attached instance's stable vectors.
The instance provider must keep that immutable model data alive while attached.
The current frame snapshot abstracts temporary animated-interface QI/release.

Call `set_model_instance`, then `init`; attachment alone neither discovers nor
starts. Call `shutdown` for effect Stop(0)/Release before releasing the instance.
Destruction alone deliberately follows the specified native node destruction:
it does not stop or release effect references. This can retain foreign effects
if a client omits Shutdown. Replacement/removal instead use Stop(1)/Release.

Creation boolean and effect output are independent. AddEffect true/null retains
an entry; false/non-null does not release that output. Recreate false retains a
null entry; true/null is an invalid native dereference, exposed as a checked-core
precondition failure. AddEffect also requires a city/manager; only Recreate
gracefully accepts an unavailable manager. Allocation follows Rust allocation
failure handling, without inventing a native false return.

Named controls use the supplied CRT unsigned-byte lowercase table; the default
table implements the specified C-locale ASCII path. Clients can refresh it when
the thread locale changes. This is separate from REGP reader lower-casing.
All control words remain u32, and IsStarted belongs to the foreign effect.

Both updates return `UpdateReport { success: true, ... }`. A null record vector
uses local identity; a nonnull empty/short vector produces a missing-record
diagnostic and no effect calls for that entry, continuing other entries. This is
the spec's checked safety extension to undefined native memory reads, without
clamping, wrapping, or substituting identity. Pointer-wrap-to-zero is not a
representable borrowed Rust slice address. Transform flags use zero/nonzero
shortcuts and the output version is zero. The GF13 zero-axis identity choice is
retained. Normalization preserves (z²+y²)+x² evaluation order and binary32
normalized-component stores, using wider portable arithmetic/transcendentals;
rotation conformance is measured at the specified 1e-6 tolerance, not x87 bit identity.

## Owner closure

Every row is for `cSC4ModelRegEffects`. There are no Unsupported owner sections.
The GF19–GF24 entries still present in the questions file describe the previous
implementation; the current spec answers them in sections 3.11.1–3.11.4.

| Spec section | Code item | Tests |
|---|---|---|
| §3.11 REGP data association and Consumer 1 discovery/name derivation | `Regions`, `effect_name`, `init_effects_list`, `add_effect`, `add_effect_at_region`, `records` (reader reused, not duplicated) | `vector48_names_order_failure_and_repeated_discovery`, `borrowed_vectors_null_vectors_exact_keys_and_replacement`, conformance `gf37_48_49_resource_to_registration_to_transform` |
| §3.11 Consumer 1 transform and GF13 choice | `Transform::from_record`, `compose`, `update_inner` | `vector49_axis_angle_zero_axis_and_extreme_axis_magnitudes`, `noncommuting_rotations_and_effect_placement_order` |
| §3.11.1 lifetime and controls (GF20–GF21) | `new`, `init`, `shutdown`, `set_model_instance`, `instance`, `clear`, `Drop`, `start_all`, `stop_all`, `start_named`, `stop_named`, `get_effect`, `remove`, `names_equal` | `lifetime_same_pointer_shutdown_and_destructor_are_distinct`, `controls_query_effect_state_and_preserve_full_words`, `named_fallbacks_creation_failure_and_crt_byte_comparison`, `borrowed_vectors_null_vectors_exact_keys_and_replacement` |
| §3.11.2 update/motion and undefined index domain (GF23–GF24) | `update`, `update_with_motion`, `update_inner`, `resolve`, `MissingRecord` | `motion_update_order_started_gate_and_unchecked_record_domain`, `missing_model_container_and_empty_map_do_not_discover` |
| §3.11.3 inventory, ordered entry helpers, flags and numeric order (GF19/GF22) | `BTreeMap` owns unique sorted nodes/strings/count; `add_inner` copies key/name and borrows records, `remove`/`clear` erase nodes; `Drop` frees nodes without effect calls; `update_inner`, `Transform::from_record` | `vector48_names_order_failure_and_repeated_discovery`, `borrowed_vectors_null_vectors_exact_keys_and_replacement`, `lifetime_same_pointer_shutdown_and_destructor_are_distinct`, `transform_flag_shortcuts_composition_scale_and_version`, `vector49_axis_angle_zero_axis_and_extreme_axis_magnitudes` |
| §3.11.4 creation, discovery guards, toggle, recreation and failures | `Creation`, `add_inner`, `init_effects_list`, `toggle_named`, `recreate_named`, `start_missing` | `independent_creation_outputs_and_manager_precondition`, `recreate_releases_without_stop_retains_null_entries_and_continues`, `missing_model_container_and_empty_map_do_not_discover`, `named_fallbacks_creation_failure_and_crt_byte_comparison` |
| §8.2 vector 37 (REGP parsing reused by owner) | `fc4_formats::s3d::Model::parse` → `Regions` | formats `t37_regp`, `regp_non_ascii_names`; conformance `gf37_48_49_resource_to_registration_to_transform` |
| §8.5 vectors 48–49 (model-effects portions) | `effect_name`, `init_effects_list`, `update_inner`, `Transform::from_record` | `vector48_names_order_failure_and_repeated_discovery`, `vector49_axis_angle_zero_axis_and_extreme_axis_magnitudes`, conformance `gf37_48_49_resource_to_registration_to_transform` |

The other §8.5 vectors (44–47) specify animated-instance playback, and the last
sentence of vector 49 specifies fire crews; those are separate owners. Effect
placement composition downstream is a renderer/effect concern, with the shared
transform operation checked here for multiplication order. No owner visibility,
camera, public flag API, index getter, or global/local switch is synthesized.
