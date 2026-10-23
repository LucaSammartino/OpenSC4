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
does not promise rollback. Event output contains uncomposed names (PT2), not
invented effect lookups. Draw commands preserve slot order, not pixel output.

`Pool` models 128-record blocks and LIFO slot reuse, with heap-success injection
for allocation fixtures. Full ordinary records hold process-wide pool leases;
portable record/list metadata and logical IDs do not reproduce native addresses.
`Description` storage is reused from the existing parser. Disk tractor/wiggle
elements are retained as two vectors at their native field coordinate, separately
from the parser's larger command records. Static constructor globals are explicit
inputs; absent globals stay unknown (PT3).

## Owner closure

Section numbers refer to `docs/specs/particles.md`. Tests below are in `tests.rs`
unless prefixed `particles::tests`; those are the existing emitter fixtures.
Open cells identify the unresolved numeric/native domain, not a successful
replacement of that domain. In particular, decal successful creation stops at
PT9 after owning the returned handle; it does not set started with guessed
projection records. Dynamic ticks with live in-city bodies stop at PT10 rather
than substituting free flight for terrain response.

| Owner / section | Code item | Test or open domain |
|---|---|---|
| cSC4ParticlesEffect §1 | `ordinary::component_kind` | `ordinary_identity_event_boundaries_and_open_physics` |
| cSC4ParticlesEffect §5 | `pool::Pool`, simulator particle leases | `effect_pool_refills_128_reuses_lifo_and_allocation_failure_is_empty` |
| cSC4ParticlesEffect §5.1 | `Motion::init`, simulator `create` | `full_records_share_existing_emitter_and_preserve_init_rng_order`; height helper open: PT1 |
| cSC4ParticlesEffect §6 | `start_checked`, `Motion::prepare_start` | `start_walk_turn_sea_priority_and_last_countdown_draw`, existing start/countdown tests; immediate non-turn walk helper open: PT1 |
| cSC4ParticlesEffect §6.1 | `Motion::new`, `configure_source`, `configure_preferred`, `resolve_collision_effects` | `ordinary_description_preparation_and_parameter_projection`; city volume open: PT1; static baseline open: PT3 |
| cSC4ParticlesEffect §7 | `update_with_draw`, `DrawCommand` | `drawing_dispatch_self_lit_and_empty_no_draw_are_before_finish`; attractor/walk helpers open: PT1 |
| cSC4ParticlesEffect §7.1 | `Particle::update`, `Motion::update`, `update_stepped` | `stepped_kill_uses_old_velocity_and_strict_city_bounds`, existing slot20 boundary tests; tractor/terrain/building/attractor helpers open: PT1 |
| cSC4ParticlesEffect §8 | existing simulator dispatch | `particles::tests::emitter_choice_by_flags` |
| cSC4ParticlesEffect §8.1 | simulator `dispatch` | existing countdown, preroll and delayed-stop tests |
| cSC4ParticlesEffect §8.2 | existing `emit_plain`, `emit_loop_budget`, `emit_rate` | existing plain/burst/rate, scale, period-floor and pre-age tests; full maintain count uses unsigned subtraction |
| cSC4ParticlesEffect §8.3 | `birth_events`, `crossing_events`, `take_events` | `ordinary_identity_event_boundaries_and_open_physics`; name composition open: PT2 |
| cSC4ParticlesEffect §9.1 | checked start/frame, `component_update`, `set_float_parameter`, abstract draw slots | `ordinary_description_preparation_and_parameter_projection`, draw-order test; native transform/other parameter channels open: PT13; packed drawing open: PT8 |
| cSC4ParticlesEffect §9.2 | existing start/stop, deferred removal, logical leases | existing hard/delayed-stop tests, full-record removal test; raw COM tables are adapter concerns |
| cSC4ParticlesEffect §13.2 | `init_native_services`, motion configuration, abstract drawing | open: PT13 for service inventory; PT8 for packed render state/camera/vertices; `ordinary_description_preparation_and_parameter_projection` checks explicit unsupported initialization |
| cSC4GeometrySourcedParticlesEffect §1 | `component_kind` | identity test, including unordered source scale |
| cSC4GeometrySourcedParticlesEffect §14.1 | `GeometryParticles::default`, `shutdown`, `teardown` | PO01–06 test; uninitialized teardown test; center/wave/mesh defaults open: PT3/PT8 |
| cSC4GeometrySourcedParticlesEffect §14.2 | `set_source`, `set_resource`, `set_model`, `set_float` | PO01–06 test; model-center and source dicing open: PT8 |
| cSC4GeometrySourcedParticlesEffect §14.3 | `set_description`, `emit`, `draw_groups`, `physics_is_ordinary` | geometry boundary and material-order tests; generated mesh, alternate draw and nonzero wave domain open: PT8 |
| cSC4DecalEffect §15.1 | `Decal::new`, `start`, `stop`, `teardown` | PO07–18 test; successful projection/configuration open: PT9 |
| cSC4DecalEffect §15.2 | `set_description`, `set_float`, `set_pointer`, `set_transforms` | PO07–18 and decal copy/reference tests; implicit composition, whole-city placement and variance open: PT9 |
| cSC4DecalEffect §15.3 | `update`, `sample`, `sample_colour`, `initial_phase` | PO07–18 test and `decal_sampling_and_pending_handle_cleanup`; full projection/shape and exact renderer rounding open: PT9 |
| cSC4DynamicParticleSystem §16.1 | `DynamicParticles::new`, `add_particle`, `delete_particle`, `link_particles` | PO19–27 and PO22 tests; joint numeric operation open: PT11; radius/key construction open: PT10 |
| cSC4DynamicParticleSystem §16.2 | `do_message`, `tick`, actual integration-call trace | PO19–27 and PO22 tests; terrain/transform construction open: PT10 |
| cSC4DynamicParticleSystem §16.3 | `activate_sector`, `deactivate_sector`, `move_neighborhood`, `draw` | PO19–27 test; key/radius/clipping, nonempty occupant conversion, shading/draw record open: PT10 |
| cSC4RigidBodySimulator2D §17.1 | `simulator::Simulator` lifetime; existing raw `storage` module | PO28–31 test; exact mass/extents open: PT11 |
| cSC4RigidBodySimulator2D §17.2 | `integrate_list`, `advance_time` | PO28–31 test; nonzero angle/cache precision, friction and extent registration open: PT11 |
| cSC4RigidBodySimulator2D §17.3 | `primary_collision_sector`; existing parent narrow-phase helpers | PO28–31 test; impulse/joint/pass order open: PT11; conflicting minimum/maximum spec open: PT12 |
| cSC4SequenceDescription §18.1 | `SequenceDescription`, primitive `read`/`write`, allocation factory retaining `Arc` | PO32–35 test; failures propagate as foreign stream errors |
| cSC4ParticlesDescriptionBase §2 | constructor and both primitive readers | `particle_binary_readers_preserve_field_widths_and_consume_both_layouts`, PO36 test; runtime global defaults open: PT3 |
| cSC4ParticlesDescriptionBase §3 | flag widths/masks and retained raw words | binary-reader test; untraced meanings open: PT1 |
| cSC4ParticlesDescriptionBase §18.2 | `Clone`, `assign` over owned parser storage | PO36 test; no derived refcount field in this value object |
| All requested owners §19.2 | PO01–PO36 groups in core unit tests | proposed portable fixtures, not measured native golden outputs |

The coverage-radius helper delegated by §19.1 is outside these seven owners and
is not duplicated here. PO22 deliberately observes an explicit PT10 result after
the dead sweep and before unknown transform math; PO31 observes PT11 only after
both lists integrate. Those results must not be presented as successful terrain
or collision simulation. No real-game run is applicable to this core-only module.
