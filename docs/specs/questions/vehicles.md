# vehicles: open questions
<!-- Questions about [vehicles.md](../vehicles.md). Append new entries at the END; when one is answered, move it to answered/vehicles.md. Rules: README.md. -->

<!-- from: re-vehicles (2026-10-21) -->
- **VH1 - §3, §5, §6:** the U-Drive-It physics of the vehicle, train and tank (`DoPlayerDriveTick`,
  `PlayerDriveUpdateSpeed`, the derail draw, the tank shot) in full. [as read]

<!-- from: re-vehicles (2026-10-21) -->
- **VH2 - §2:** how `FindBestNeighborConnection` ranks the neighbour connections, and the slots 159, 167, 168 and
  175 it and the look-ahead fill use. [as read]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH4 - §2/§3/§5: construction and Init.** Which flags/defaults does each
  named owner set, what does traffic Init acquire, and in what order do failures
  unwind? §3 says only "sets its flags". TrainCar's stated 0x10000000 is known.
  [Reuse existing record defaults as portable staging values; Init explicitly
  reports Unsupported rather than treating staging values as native defaults.]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH5 - §5: train-owned algorithms.** Supply car count/model/length/colour
  selection and random draws, car placement/following geometry and random sound
  thresholds, StartTrip inputs/path protocol, message payload, and the failure
  accident parameters. What does TrainCar's network-placement override change?
  [Do not delegate these owner algorithms to a game vehicle or foreign manager;
  composition/following/trip return Unsupported. Known ownership, teardown,
  collision-speed copying and minimum collide timeout are implemented.]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH6 - §3/§5: crash/damage/blur.** What direction distribution and transform
  does HandleCollision choose, how is ResolveCollision different from
  UpdateAfterDamage, which event ID/payload is raised, how is the damaged model
  selected, and what blur key/fade formula is used? [Resolved collision direction
  can enter the specified final 5000ms COLLIDE; random selection, damage model
  and blur remain Unsupported. The documented accident and damage flag are
  available, with a partial-operation error for the missing model step.]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH7 - §2/§3, automata.md §6.3/§6.5: route representation and geometry.**
  Name the traffic record's unnamed fields and route bytes, define lane-bit
  matching (equality or overlap), empty route/missing-occupant failure, path-point
  reconstruction after Read, path advancement inputs/outputs, negative step and
  leader-gap boundary details, bridge/slope speed factors, and default route-end
  behavior. [Route triples stay opaque; lane bits compare equal literally;
  network geometry is foreign, owner flags/tweens/anchoring remain in core;
  incomplete route setup reports Unsupported.]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH9 - inherited automaton states and dispatch hooks.** For the named
  traffic/vehicle owners, specify inherited RANDOM_WALK, PLOP_JUMP, ZOT and
  ATTACH_ANIM controller/model contracts and countdown ordering, and the base
  dispatch getters/setters versus emergency-only overrides. The state table
  gives outlines, not complete controller/model rules. [Active underspecified
  inherited handlers return Unsupported; do not run the game's owner. Generic
  DEFAULT/BEE_LINE/CHASE/BUMPED seek, SCRIPTED, disappearance, vehicle IDLE,
  collide, wait, station, plummet and pull-over have core handlers.]
