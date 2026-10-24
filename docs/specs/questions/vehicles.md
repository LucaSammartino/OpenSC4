# vehicles: open questions
<!-- Questions about [vehicles.md](../vehicles.md). Append new entries at the END; when one is answered, move it to answered/vehicles.md. Rules: README.md. -->

<!-- from: re-vehicles (2026-10-21) -->
- **VH1 - §3, §5, §6:** the U-Drive-It physics of the vehicle, train and tank (`DoPlayerDriveTick`,
  `PlayerDriveUpdateSpeed`, the derail draw, the tank shot) in full. [as read]
  Remaining evidence needed: instruction-verified throttle/steering tween equations,
  on-network versus off-road displacement and collision ordering, train curve/derail
  probability with its exact random range and comparison, and tank targeting,
  projectile effect/damage parameters and firing cooldown. Existing outlines do not
  specify those algorithms. Locate the named tick/command functions and
  PlayerDriveCheckForDerail in the export before examining their numeric branches.
  No physics closure is claimed by the route-speed factors.

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH5 - §5: train-owned algorithms.** Supply car count/model/length/colour
  selection and random draws, car placement/following geometry and random sound
  thresholds, StartTrip inputs/path protocol, message payload, and the failure
  accident parameters. What does TrainCar's network-placement override change?
  [Do not delegate these owner algorithms to a game vehicle or foreign manager;
  composition/following/trip return Unsupported. Known ownership, teardown,
  collision-speed copying and minimum collide timeout are implemented.]

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH7 - §2/§3, automata.md §6.3/§6.5: route representation and geometry.**
  Name the traffic record's unnamed fields and route bytes, define lane-bit
  matching (equality or overlap), empty route/missing-occupant failure, path-point
  reconstruction after Read, path advancement inputs/outputs, negative step and
  leader-gap boundary details, bridge/slope speed factors, and default route-end
  behavior. [Route triples stay opaque; lane bits compare equal literally;
  network geometry is foreign, owner flags/tweens/anchoring remain in core;
  incomplete route setup reports Unsupported.]
  **2026-10-24 narrowing (re-vehicles-vh11):** vehicles.md §12.9 now specifies
  masked-equality OR matching, signed advancement, placement/reattachment, gap
  boundaries, route-end behavior and network/turn/slope speed factors with vectors.
  Read itself makes no geometry query. Still open: establish the serialized
  receiver/layout relationship (record network ref `+0x1E4` versus live occupant
  `+0x1EC`, serialized deque `+0x224` versus live `+0x22C`), name the stream's
  scalar aliases from that relationship, and identify the later load-to-live
  binding call. Route byte 3 is preserved and unused by the inspected geometry
  consumers; its producer-side meaning also remains unestablished. Do not infer
  an eight-byte adjustment, native save defect or successful reconstruction.
