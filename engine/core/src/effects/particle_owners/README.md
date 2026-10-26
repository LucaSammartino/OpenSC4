<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Particle owner contracts

This module extends `effects::particles` and `rigid_body_2d`, rather than adding
another emitter or narrow-phase implementation. It is core-only. Native services,
renderer records, terrain, resources and stream primitives remain foreign.
It does not register a plugin component or change the C ABI.

Use `ParticleSimulator::set_motion`, `start_checked`, and `update_checked` (or
`update_with_draw`) for full records. The original finish-only projection remains
available to existing effects callers. The checked APIs expose unresolved helpers
as `Unsupported`; an error can follow the specified prefix of state changes and
does not promise rollback. Event suffixes can be drained or fired through the
literal directory contract in §8.3. Draw commands preserve slot order, not pixel output.

`Pool` models 128-record blocks and LIFO slot reuse, with heap-success injection
for allocation fixtures. Full ordinary records hold process-wide pool leases;
portable record/list metadata and logical IDs do not reproduce native addresses.
`Description` storage is reused from the existing parser. Disk tractor/wiggle
elements are retained as two vectors at their native field coordinate, separately
from the parser's larger command records. Static constructor globals are explicit
inputs for the RGB constructor vector; §2's PT3 scalar defaults are positive zero.

## Owner closure

Section numbers refer to `docs/specs/particles.md`. Tests below are in `tests.rs`
unless prefixed `particles::tests`; those are the existing emitter fixtures.
Open cells identify the unresolved numeric/native domain, not a successful
replacement of that domain. Decal creation completes the finite §15.4 trace.
Dynamic ticks with live in-city bodies stop at PT10 rather than substituting
free flight for terrain response. Every row with no open domain has implemented
credit within its stated domain; native verification is a separate claim.

| Owner / section | Code item | Test or open domain |
|---|---|---|
| cSC4ParticlesEffect §1 | `ordinary::component_kind` | `ordinary_identity_event_boundaries_and_open_physics` |
| cSC4ParticlesEffect §5 | `pool::Pool`, simulator particle leases | `effect_pool_refills_128_reuses_lifo_and_allocation_failure_is_empty` |
| cSC4ParticlesEffect §5.1 | `Motion::init`, simulator `create` | `full_records_share_existing_emitter_and_preserve_init_rng_order`; height helper open: PT1 |
| cSC4ParticlesEffect §6 | `start_checked`, `Motion::prepare_start` | `start_walk_turn_sea_priority_and_last_countdown_draw`, existing start/countdown tests; immediate non-turn walk helper open: PT1 |
| cSC4ParticlesEffect §6.1 | `Motion::new`, source/preferred/collision configuration | description-preparation and PO36 tests apply PT3 zero baseline; open PT1: §10 city-volume helper box/service trace |
| cSC4ParticlesEffect §7 | `update_with_draw`, `DrawCommand`, `Motion::validate` | `drawing_dispatch_self_lit_and_empty_no_draw_are_before_finish`, `inactive_attractor_curve_does_not_require_unsettled_helper`; active attractor/walk helpers open: PT1 |
| cSC4ParticlesEffect §7.1 | `Particle::update`, `Motion::update`, `update_stepped` | `stepped_kill_uses_old_velocity_and_strict_city_bounds`, existing slot20 boundary tests; tractor/terrain/building/attractor helpers open: PT1 |
| cSC4ParticlesEffect §8 | existing simulator dispatch | `particles::tests::emitter_choice_by_flags` |
| cSC4ParticlesEffect §8.1 | simulator `dispatch` | existing countdown, preroll and delayed-stop tests |
| cSC4ParticlesEffect §8.2 | existing `emit_plain`, `emit_loop_budget`, `emit_rate` | existing plain/burst/rate, scale, period-floor and pre-age tests; full maintain count uses unsigned subtraction |
| cSC4ParticlesEffect §8.3 | `birth_events`, `crossing_events`, `fire_event`, `fire_events` | event-boundary test; `pt2_literal_suffix_and_directory_call_order` applies PT2 runtime contract; script population remains the separate §8.3 compiler observation |
| cSC4ParticlesEffect §9.1 | `Motion::set_transforms`, `set_float`, `set_pointer`, `draw_factors`, abstract draw slots | `pt13_transform_generations_source_box_ray_and_pointer_channels`, `pt1_worked_prepack_factors_use_aspect_not_opacity` apply PT13/PT1; open PT1: §13.2 slot12 aim consumer; PT8: §21.2 state/vertex/UV/lock captures |
| cSC4ParticlesEffect §9.2 | existing start/stop, deferred removal, logical leases | existing hard/delayed-stop tests, full-record removal test; raw COM tables are adapter concerns |
| cSC4ParticlesEffect §13.2 | `NativeServices::init/shutdown`, `init_native_services`, abstract drawing | `pt13_services_are_idempotent_and_use_two_width_queries` applies PT13 inventory; open PT8: §21.2 packed render-state/camera/vertex capture |
| cSC4GeometrySourcedParticlesEffect §1 | `component_kind` | identity test, including unordered source scale |
| cSC4GeometrySourcedParticlesEffect §14.1 | `default`, `shutdown`, `teardown` | PO01–06/uninitialized teardown tests; ordinary PT3 zero seeds applied; open PT8: §14.1 runtime center/wave words before/after Init |
| cSC4GeometrySourcedParticlesEffect §14.2 | source/resource/model setters, `project_source` | PO01–06; `pt8_null_empty_mesh_and_wave_worked_prefix` null/zero-group cases; open PT8: §21.2 literal nonempty/degenerate mesh, channel/lock/resource failure captures and model center |
| cSC4GeometrySourcedParticlesEffect §14.3 | description/emission/draw dispatch, `wave_prefix` | geometry boundary/material-order tests and PT8 wave worked prefix; open PT8: §21.2 particle displacement/damping and packed alternate draw captures |
| cSC4DecalEffect §15.1 | `Decal::start`, `stop`, `teardown`, `projection` | PO07–18; `pt9_worked_creation_trace_and_independent_flags` tests all 128 flag words; `pt9_start_modes_store_phase_seconds_and_transforms_use_turns`; implemented for finite §15.4 |
| cSC4DecalEffect §15.2 | description/parameter/transform setters | decal copy/reference tests; `pt9_variance_draw_order_and_whole_city_service_case`, PT9 transform/mode test; implemented for finite §15.4 |
| cSC4DecalEffect §15.3 | `update`, sampling, logical `Placement` | PT9 creation/flag/mode tests; open PT9: §15.4 key-boundary scalar/RGB bits, exceptional phases/curves and native placement-record packing; abstract placement is not native byte conformance |
| cSC4DynamicParticleSystem §16.1 | admission/deletion/linking, `Joint` | PO19–27/PO22; `pt10_radius_key_order_reference_counts_and_terrain_prefix` applies PT10 radius and PT11 joint records; implemented for distinct valid endpoints |
| cSC4DynamicParticleSystem §16.2 | `do_message`, `tick`, `terrain_prefix` | PO19–27/PO22 and PT10 floor/gravity prefix; open PT10: §21.3 post-normal-response body words, four city edges and final cached transform; prefix is not a full tick |
| cSC4DynamicParticleSystem §16.3 | sector/neighborhood helpers, static predicate prefix, `draw` | PT10 radius/key/refcount and static-filter tests; open PT10: §21.3 rotated-box description, interface-state filter, shading/draw and lifecycle receiver/service records |
| cSC4RigidBodySimulator2D §17.1 | `Simulator::add_object`, raw `storage` lifetime | PO28–31; `pt11_rb_admission_and_integration_worked_cases` applies RB-A1–A5 finite mass/extent/list rules; implemented for finite admission |
| cSC4RigidBodySimulator2D §17.2 | `integrate_list`, `advance_time`, sector mapping | PO28–31; PT11 RB-I worked cases cover friction, signed angle remainder, caches/remapping and exclusion of static root; implemented for finite integration |
| cSC4RigidBodySimulator2D §17.3 | primary predicate, `resolve`, `collision_phases`, `update_joints` | `pt11_coincident_rb_c0_sector_order_and_joint_equalities`; `rigid_body_2d::tests::rb_p1_p2_unsigned_mixed_halves` applies PT12; open PT11: §7.5 clock/RNG/exceptional and static/low/hash traces; whole collision-cycle dispatch remains unsupported |
| cSC4SequenceDescription §18.1 | `SequenceDescription`, primitive `read`/`write`, allocation factory retaining `Arc` | PO32–35 test, `sequence_factory_retains_description_and_stream_errors_preserve_only_prefix`; success retains the description, primitive failures propagate without rollback |
| cSC4ParticlesDescriptionBase §2 | constructor scalar seeds, both primitive readers | binary-reader and `po36_particle_value_copy_is_deep_and_pt3_scalar_defaults_are_zero` apply PT3; implemented with supplied RGB constructor primitive (§2 gives its location only) |
| cSC4ParticlesDescriptionBase §3 | flag masks and retained raw words | binary-reader test; open PT1: §21.1 flags A 10/04000000 read/branch/particle-word capture |
| cSC4ParticlesDescriptionBase §18.2 | `assign` over owned parser storage | PO36 test covers independent curves, strings, tractor/wiggle and event vectors; native base assignment preserves destination derived refcount and excludes the source's trailing refcount word |
| All requested owners §19.2 | PO01–PO36 groups in core unit tests | proposed portable fixtures, not measured native golden outputs |

The coverage-radius helper delegated by §19.1 is outside these seven owners and
is not duplicated here. PO22 deliberately observes an explicit PT10 result after
the dead sweep and before unknown transform math; PO31 observes PT11 after the
ordinary/low-response phases, excluding explicit static bodies. These results
are not successful terrain or full collision simulation. No whole-owner flip
is implied while that owner's closure has an open row. Game smoke checks plugin
loading only; these core-only paths have no shadow.
