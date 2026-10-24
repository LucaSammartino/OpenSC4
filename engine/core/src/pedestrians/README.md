# Pedestrian owners

Implemented column path: `engine/core/src/pedestrians`.

This portable core owns `cSC4Pedestrian` and `cSC4PedestrianOneShot`.
`PedestrianRecord`, `AutomatonBase`, stack operations and serialization come
from `automata`; inherited state dispatch, tweens, routes and movement use
`vehicles::TrafficAutomaton` / `TrafficOwner`. There is no plugin or C ABI wiring.
All world callbacks are foreign controller, occupant, renderer, network or
pathfinder operations, not calls to the original pedestrian owner.

The spec leaves the saved pedestrian tail unnamed. It is preserved as the
existing record, without guessing aliases for transient runtime fields.
PN1's initial random parameter is implemented by the later PR conformance
clarification. Only its play-count consumer is explicitly Unsupported here.
PN2 and conformance test 2 concern cSC4SimFireCrew, outside these two owners.
PN3 records literal interpretations of summary-level details; those paths run.
Inherited traffic errors remain the shared traffic module's errors; this module
does not claim to settle that module's VH questions.

## Owner closure

Tests below are in `tests.rs` (names without the module prefix).

| Owner | Spec section | Code item | Test / open |
|---|---|---|---|
| cSC4Pedestrian | pedestrians §1 | `CLSID_PEDESTRIAN`, `PedestrianRecord` | `construction_and_first_parameter_conformance`, `record_roundtrip_and_empty_inherited_tick` |
| cSC4Pedestrian | §2.1 | `construct`, `init`, `Runtime::initial_parameter` | `construction_and_first_parameter_conformance` |
| cSC4Pedestrian | §2.2 | `animation_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4Pedestrian | §2.3 | `enter_state`, `state_tick`, shared stack | `enter_state_flag_table_and_empty_stack`, `queue_strike_and_ignite_dispatch` |
| cSC4Pedestrian | §2.4 | `move_forward_with_collision`, `test_collision_expensive`, `bee_line_tick` | `collision_cadence_box_and_close_random_push`, `collision_movement_rejection_retry_and_ground_flag` |
| cSC4Pedestrian | §2.5 | `crowd_reaction`, `crowd_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4Pedestrian | §2.6 | `queue_tick`, `strike_tick`, riot ticks, `ignite_tick` | `queue_strike_and_ignite_dispatch`, `riot_leader_cell_memory_and_follower_distance` |
| cSC4Pedestrian | §2.7 | `traffic_owner`, `inherited_tick`, `base`, shared records | `record_roundtrip_and_empty_inherited_tick`; inherited VH limitations remain in vehicles |
| cSC4Pedestrian | §6 quirk 2 | `test_collision_expensive` | `collision_cadence_box_and_close_random_push` |
| cSC4Pedestrian | §7 | known initial-parameter operation | `construction_and_first_parameter_conformance`; PN1 consumer applies to one-shot |
| cSC4Pedestrian | §8 tests 1, 3, 4 and PR conformance | constructor, entry, collision, Init | `construction_and_first_parameter_conformance`, `enter_state_flag_table_and_empty_stack`, `collision_cadence_box_and_close_random_push` |
| cSC4Pedestrian | automata §6.4 | shared `PedestrianRecord`, `read`, `write` | `record_roundtrip_and_empty_inherited_tick` |
| cSC4Pedestrian | automata §6.5 (2,3,4,6,8,9,12) | pedestrian state dispatcher | crowd, queue/strike/ignite and riot tests above |
| cSC4PedestrianOneShot | pedestrians §1 | `CLSID_ONE_SHOT`, `PedestrianOneShot` | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §2.1 (inherited) | `pedestrian.construct`, `pedestrian.init` | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §2.2 (inherited) | `pedestrian.animation_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4PedestrianOneShot | §2.3 (inherited) | `pedestrian.enter_state`, `pedestrian.state_tick` | `enter_state_flag_table_and_empty_stack` |
| cSC4PedestrianOneShot | §2.4 (inherited) | `pedestrian.move_forward_with_collision` | both collision tests above |
| cSC4PedestrianOneShot | §2.5 (inherited) | `pedestrian.crowd_reaction`, `pedestrian.crowd_tick` | `bee_line_crowd_reaction_and_animation_transitions` |
| cSC4PedestrianOneShot | §2.6 (inherited) | pedestrian state ticks | queue/strike/ignite and riot tests above |
| cSC4PedestrianOneShot | §2.7 (inherited) | shared traffic and automata records | `record_roundtrip_and_empty_inherited_tick`; inherited VH limitations remain in vehicles |
| cSC4PedestrianOneShot | §3 | `init` instance values, exemplar bounds, count draw | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | §3, §7 play-count consumer | `consume_play_count` | open: PN1 |
| cSC4PedestrianOneShot | §6 quirk 2 (inherited) | pedestrian collision cadence | `collision_cadence_box_and_close_random_push` |
| cSC4PedestrianOneShot | §8 PR conformance | inherited first parameter draw | `one_shot_init_success_failure_and_property_bounds` |
| cSC4PedestrianOneShot | automata §6.4 | `read`, `write`, shared pedestrian record | `record_roundtrip_and_empty_inherited_tick` |
| cSC4PedestrianOneShot | automata §6.5 (inherited) | pedestrian state ticks | state/collision/crowd/riot tests above |

Summary choices tracked by PN3: collision cadence resets after a test without a
catch-up burst; reaction selection is uniform at renderer completion; strike
insets its queried box by one cell; riot turns choose uniformly among walkable
headings on each new road cell; nearby followers mill in a random direction
each tick. These are literal implementations of the stated summaries, not
additional Unsupported branches.
