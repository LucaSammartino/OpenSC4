# Vehicle owners

Clean-room core owners for `cSC4TrafficAutomaton`, `cSC4Vehicle`, `cSC4TrainCar`
and `cSC4Train`, from vehicles.md §§2–5, §12 and inherited automata contracts.
This module has no plugin registration or C ABI exports. Foreign interfaces
provide network geometry, terrain, models, effects, controllers and random draws;
they do not execute traffic, vehicle or train owner algorithms in the game.

## Owner closure

No owner is fully implemented. The table lists the remaining open questions
explicitly; implemented clarification vectors do not imply full owner closure.

| Owner | Implemented contracts | Questions blocking full implementation |
|---|---|---|
| cSC4TrafficAutomaton | Constructor (§12.3); inherited codecs and state stack; animation/tweens, straight/path movement, collision/wait/station; retained-node routing, neighbor ranking and look-ahead (§12.5); generic state/countdown/hop/attach pieces (§12.6); Init model-failure/loaded ordering and teardown | VH7 route representation, reconstruction and remaining geometry; VH9 remaining inherited state/controller and base dispatch contracts; VH10 fresh Init slot 134 |
| cSC4Vehicle | Traffic composition; constructor and known Init additions (§12.3); VehicleRecord codec; collision direction, damage and blur ring (§12.4); plummet, splash and PullOver (§12.2); state/route-end handlers; blur/splash teardown | VH1 player driving; inherited VH7/VH9; VH10 drive-parameter slot 158 and inherited fresh Init |
| cSC4TrainCar | Vehicle composition; constructor/classification/Init flags (§12.3); exact version-2 wrapper (§12.1); inherited runtime state and teardown | VH5 network-placement override; inherited VH1/VH7/VH9/VH10 |
| cSC4Train | TrainCar constructor/Init semantics, family/count and speed draws (§12.3); nested version-2 wrappers and ordered append-on-Read car references (§12.1); minimum collide timeout, collision-speed propagation and owned teardown | VH5 composition/model selection, following, placement, trip/message/accident contracts; VH11 loaded car identity/runtime binding; inherited VH1/VH7/VH9/VH10 |

VH2/VH3/VH4/VH6/VH8 are implemented by the corresponding §12 clarifications.
Remaining VH9 gaps stay open even though §12.6 supplies partial state contracts.
`Unsupported` is a typed result, never an instruction to call the game owner.

## Init and lifetime

`construct` takes runtime tuning/random inputs. `Default` is portable staging
storage for codecs, rather than a substitute for the native constructor.
Init returns `Result<bool>`: `Ok(false)` is the documented model failure;
`Err(InitVh10)` means an unspecified owner body was reached. Known additions
retain their documented order, including Occupant Init on model failure and
Train/TrainCar additions after the inherited result. Missing Init work remains
an error on repeat calls until shutdown, so partial Init never becomes closure.
There are no provisional route-init or drive-parameter callbacks.

Read is city-free. Vehicle rejects live initialized, splash or blur resources
before consuming bytes. Its nested record is atomic; the Train wrapper is
deliberately nontransactional and appends non-null car references. TrainCar and
Train use the shared stream reference protocol, never raw pointer bytes.

Train has one ordered `cars` list. Each entry contains its serializable identity
and optional owned runtime car. Both Read/Write and runtime operations use that
list. Read preserves existing entries and appends unresolved references without
inventing runtime cars. Collision reports VH5 when any entry is unresolved,
before side effects. Writing an owned runtime car with a null identity fails
before output (VH11); otherwise it could silently disappear on the next Read.
Shutdown releases runtime cars, clears identities and the trailing network,
and shuts down the engine. Vehicle shutdown releases every retained blur model
and splash exactly once; Read cannot discard live blur handles.

Times are milliseconds except tween/fade seconds. PullOver reconstructs its
transient vectors on the first positive loaded tick as §12.2 specifies.

## Verification map

`tests.rs` covers route consumption, collision/wait, straight/path movement,
record truncation, splash lifetime, state timeout and train teardown.
`tests/closure.rs` covers §12 construction/Init, both train wrappers, collision
and damage, blur ring, loaded plummet/PullOver, inherited state vectors and the
Init/blur/car-identity audit regressions. `tests/routing.rs` covers §12.5 retained
nodes, mutation/miss behavior and neighbor ranking. The nearest eligible leader
vector also lives in `engine/conformance/tests/vehicles.rs`.
