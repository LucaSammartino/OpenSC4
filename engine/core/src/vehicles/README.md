# Vehicle owners

Clean-room core owners for `cSC4TrafficAutomaton`, `cSC4Vehicle`, `cSC4Train`
and `cSC4TrainCar`, from vehicles.md with inherited automata contracts.
No plugin registration, C ABI, renderer, pathfinder, network or occupant manager
implementation is added. `TrafficWorld` supplies network geometry/queries and
render effects; `VehicleWorld` adds terrain, controller and effect interfaces.
Neither interface executes a vehicle owner in the original game.

This is **not full owner closure**. The spec's explicit VH1/VH2 openings and
newly filed VH3-VH9 gaps prevent it. Unsupported is a typed result, not success.
In particular, Train/TrainCar never emit a guessed wire layout, and the foreign
interfaces do not hide car selection/following or other missing owner algorithms.
Native Init is unsupported; Default creates a portable record staging object,
not an asserted reproduction of unlisted native constructor flags.

The existing `automata::{TrafficRecord, VehicleRecord, AutomatonBase, State}`
remain the only codecs and stack implementations. Read is atomic and city-free;
runtime path state/effects are not serialized. The existing occupant codec sets
its loaded-status bit on Read. Vehicle Read refuses live initialized/effect
state; shutdown it first. Train/car Read/Write return Unsupported without
touching the stream. `Vehicle::update_after_damage` performs the documented
accident/disabled flag before reporting the missing model/event operation.

All times passed to ticks are milliseconds. Tweens use seconds. The detailed
elapsed-counter interpretation is used for PullOver; plummet entry's conflicting
constants are not guessed. Loaded plummet fields can tick; creating a splash
requires supplied fade seconds. The two PullOver vectors are transient; native
reconstruction after loading a nonzero timer remains VH8.

## Owner closure

Each applicable section is mapped below. A mixed row lists implemented pieces
and the exact question preventing closure of the rest. Tests without a file
prefix are in `tests.rs`; the leader test is in conformance/tests/vehicles.rs.

| Owner | Spec section | Code item | Test or open question |
|---|---|---|---|
| TrafficAutomaton | vehicles §1; managers §1 | TrafficAutomaton, TrafficRecord | record reuse; no concrete traffic CLSID specified |
| TrafficAutomaton | vehicles §2 animation | animation_tick, advance_tweens | tween_seconds_and_straight_arrival_without_snap; open: VH9 inherited handlers |
| TrafficAutomaton | vehicles §2 routes | TrafficOwner::{set_route_info,begin_route,fill_look_ahead} | route_consumes_first_and_stops_failed_lookahead; open: VH2, VH7 |
| TrafficAutomaton | vehicles §2 outside connections | best_neighbor_connection | open: VH2 |
| TrafficAutomaton | vehicles §2 leaders; §11.4 | find_leader_in_tile, find_leader | vh11_4_nearest_eligible_leader_ignores_height; open: VH7 lane-bit interpretation |
| TrafficAutomaton | vehicles §2 collision; §9.1 | update_collision | collision_flags_wait_and_default_speed |
| TrafficAutomaton | vehicles §2 state effects | EffectAction; collision enter/exit hooks | collision_flags_wait_and_default_speed; open: VH9 full state lifecycle |
| TrafficAutomaton | automata §6.1-6.3 records/states | reused TrafficRecord and AutomatonBase | existing automata codec tests; record_roundtrip_and_every_truncation_atomic_before_init |
| TrafficAutomaton | automata §6.5 movement/wait/station | path_tick, straight_tick, blocked_wait_tick, animation_tick | blocked_wait_obeys_stopped_flag_and_intersection; vehicle_path_anchors_and_measures_but_traffic_does_not; open: VH7, VH9 |
| TrafficAutomaton | managers §4 construction/§5 helpers | init, shutdown, direction_toward | open: VH4; movement_failures_remove_only_once_and_special_route_survives_reverse |
| TrafficAutomaton | dispatch §4.2.1/§4.3 | inherited dispatch info ID stored in base.u1; no emergency override | open: VH9 for base dispatch slot contracts; manager route orchestration is foreign |
| Vehicle | vehicles §1; managers §1 | Vehicle, CLSID_VEHICLE | record_roundtrip_and_every_truncation_atomic_before_init |
| Vehicle | vehicles §3 Init | Vehicle::init | open: VH4 |
| Vehicle | vehicles §3 state tick; automata §6.2/§6.5 | state_tick, countdown, plummet_tick, pull_over_tick | default_and_chase_seek_scripted_does_not; plummet_tumble_then_land_removes_without_pop; pull_over_initializes_moves_and_may_pop_twice; open: VH8, VH9 |
| Vehicle | vehicles §3 collision | enter_collision, collide_tick | collision_final_timeout_disappears_and_train_minimum; open: VH6 random direction/transform |
| Vehicle | vehicles §3 damage | update_after_damage | open: VH6 event/model mapping and ResolveCollision |
| Vehicle | vehicles §3 motion blur | update_motion_blur | open: VH6 |
| Vehicle | vehicles §3 player driving | player_drive | open: VH1 |
| Vehicle | automata §6.1-6.4 save/load | read, write, reused VehicleRecord | record_roundtrip_and_every_truncation_atomic_before_init |
| Vehicle | automata §6.5 path/straight | path_tick, TrafficOwner::straight_tick | vehicle_path_anchors_and_measures_but_traffic_does_not; movement_failures_remove_only_once_and_special_route_survives_reverse; open: VH7 |
| Vehicle | automata §6.5 lifetime/splash | shutdown, pop_state, plummet_tick | splash_survives_until_timer_then_releases_exactly_once; open: VH8 fade source |
| Vehicle | automata §7.5 vectors | reused base.vectors; movement updates world.move_models | vehicle_path_anchors_and_measures_but_traffic_does_not; open: VH9 complete setter/model lifecycle |
| Vehicle | managers §5 route-end/helpers | reached_route_destination, direction_toward | open: VH7 route end/speed factors, VH8 fall entry |
| Vehicle | dispatch §4.2.1/§4.3 | same inherited base.u1; emergency states belong to other owners | open: VH9 base dispatch contract |
| TrainCar | vehicles §1/§5; managers §1 | TrainCar, CLSID_TRAIN_CAR, init | train_shutdown_owns_cars_and_copies_measured_speed; open: VH4 inherited Init |
| TrainCar | vehicles §5; automata §6.4 record | read, write | open: VH3 (never use VehicleRecord as a guessed train-car record) |
| TrainCar | vehicles §5; managers §5 slot 132 | inherited Vehicle movement, no guessed placement override | open: VH5 |
| TrainCar | automata §6.5 state/lifetime | vehicle field, Train::shutdown | train_shutdown_owns_cars_and_copies_measured_speed; inherited VH1/VH4/VH6-VH9 |
| TrainCar | dispatch §4.3 | no train-car-specific behavior assigned | inherited open: VH9 |
| Train | vehicles §1; managers §1 | Train, CLSID_TRAIN | unresolved_train_records_touch_neither_stream_nor_owner |
| Train | vehicles §5 Init/cars | Train::init, owned Vec<TrainCar> | open: VH4, VH5 |
| Train | vehicles §5 Shutdown | Train::shutdown | train_shutdown_owns_cars_and_copies_measured_speed |
| Train | vehicles §5 StartTrip | start_trip | open: VH5 |
| Train | vehicles §5 animation | animation_tick | open: VH5 |
| Train | vehicles §5 collision; automata §6.5 | enter_collision, collide_tick | train_shutdown_owns_cars_and_copies_measured_speed; collision_final_timeout_disappears_and_train_minimum; open: VH6 direction/transform |
| Train | vehicles §5 player driving/derail | player_drive | open: VH1 |
| Train | vehicles §1/§5; automata §6.4 record | Train::{read,write} | unresolved_train_records_touch_neither_stream_nor_owner; open: VH3 |
| Train | automata §6.1-6.3/§6.5 inherited state | vehicle field | inherited Vehicle tests; inherited VH4/VH6-VH9 |
| Train | dispatch §4.3 | no train-specific dispatch behavior assigned | inherited open: VH9 |

vehicles §4 and §§6-8 assign behavior to TrailerVehicle, Tank, EmergencyVehicle,
FireTruck, PatrolCar, RiotPolice, Bus and GetawayVan, outside the four named
owners. Consequently §11 tests 1-3 are emergency/fire-truck conformance, not
evidence for any of these four owners; §11 test 4 is implemented here. No claim
is made that this lane closes the whole vehicles spec or all automata §14 tests.
Relevant automata §14 tests 27/28/30/31/32 have focused owner regressions here;
24/26/29 use the preexisting shared codec/stack tests.
