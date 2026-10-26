# vehicles: answered questions

<!-- from: core-vehicles5 (2026-10-23) -->
- ~~**VH5 - §5: train-owned algorithms.** Supply car count/model/length/colour
  selection and random draws, car placement/following geometry and random sound
  thresholds, StartTrip inputs/path protocol, message payload, and the failure
  accident parameters. What does TrainCar's network-placement override change?
  [Do not delegate these owner algorithms to a game vehicle or foreign manager;
  composition/following/trip return Unsupported. Known ownership, teardown,
  collision-speed copying and minimum collide timeout are implemented.]~~

  **ANSWERED 2026-10-25 (re-vehicles-vh21):** DIFFERS —
  [vehicles.md §12.11](../../vehicles.md#1211-train-composition-following-and-trip-start-vh5)
  specifies low-byte target counts, one-car-per-tick creation, the caboose family,
  head colour, two-anchor sphere/segment following, fallback/visibility rules,
  and three-input StartTrip. Rail-accident creation `(head,2,20000)` and the cell
  message precede the inherited trip attempt, rather than being caused by its
  failure. The alleged random sound threshold is blocked-train removal:
  speed modifier >f32(0.01), blocked seconds >=10, unsigned draw <0x7FFFFFFF.
  Family/count and constructor randomness remain in §12.3; ownership in §12.8.

<!-- from: core-vehicles5 (2026-10-23) -->
- ~~**VH9 - inherited automaton states and dispatch hooks.** For the named
  traffic/vehicle owners, specify inherited RANDOM_WALK, PLOP_JUMP, ZOT and
  ATTACH_ANIM controller/model contracts and countdown ordering, and the base
  dispatch getters/setters versus emergency-only overrides. The state table
  gives outlines, not complete controller/model rules. [Active underspecified
  inherited handlers return Unsupported; do not run the game's owner. Generic
  DEFAULT/BEE_LINE/CHASE/BUMPED seek, SCRIPTED, disappearance, vehicle IDLE,
  collide, wait, station, plummet and pull-over have core handlers.]~~

  **ANSWERED 2026-10-24 (re-vehicles-vh11):** DIFFERS — [vehicles.md §12.6](../../vehicles.md#126-inherited-state-and-dispatch-contracts-vh9)
  specifies all four handlers, countdown precedence and literal dispatch slots.
  The requested controller attraction is native MAGNET (18), not a separate ZOT
  dispatch case; its displacement has no tick-delta factor. Random walk has
  signed walk/rest timing and repeated turn draws. Base dispatch setters are
  mostly no-ops; emergency transitions must not be applied to ordinary vehicles.

<!-- from: re-vehicles (2026-10-21) -->
- **VH2 - §2:** how `FindBestNeighborConnection` ranks the neighbour connections, and the slots 159, 167, 168 and
  175 it and the look-ahead fill use. [as read]

  **ANSWERED 2026-10-24 (re-vehicles-vh5b):** DIFFERS — [vehicles.md §12.5](../../vehicles.md#125-neighbour-ranking-and-look-ahead-contracts-vh2)
  ranks successful paths by entry count, keeps the earlier tie, and defines finder inputs and connection lookup.
  Slot 159 tweens speed to zero; 167 expands ordinary paths with a guarded terminal fallback; 168 consumes explicit
  cells even on failure; 175 returns a tuning byte. The section specifies packed words, terminal results and
  conformance vectors. Foreign network/simulator ordering remains an explicit service boundary.

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH3 - §1/§5, automata.md §6.4: train and train-car records.** What are each
  class's leading version, exact nested base, field order, car-reference IID,
  network reference placement, version checks, and Read failure/reconstruction
  rules? The class table mentions references but supplies no wire layout.
  [Train/TrainCar Read and Write return Unsupported without consuming/emitting bytes.]

  **ANSWERED 2026-10-23 (re-vehicles-vh5):** DIFFERS — [vehicles.md §12.1](../../vehicles.md#121-train-and-traincar-records-vh3)
  specifies exact version-2 wrappers (Train → TrainCar → Vehicle), car references
  with IID `0x4A416254`, the independent trailing network reference with IID
  `0xA821EF94`, ordered fields, partial consumption on failure, and append/skip-null
  reconstruction. There is no Train wrapper-level geometry reconstruction.

<!-- from: core-vehicles5 (2026-10-23) -->
- **VH8 - §3 via automata.md §6.5: plummet/pull-over entry.** Resolve the
  explicitly conflicting plummet descent constants and give the splash fade
  duration's source/distribution. How are PullOver's unsaved vectors rebuilt
  when Read restores a nonzero elapsed value? [Plummet entry Unsupported;
  loaded five-float state ticks implemented; splash needs explicit seconds;
  pull-over follows the detailed elapsed-counter description literally.]

  **ANSWERED 2026-10-23 (re-vehicles-vh5):** DIFFERS — [vehicles.md §12.2](../../vehicles.md#122-plummet-entry-splash-and-loaded-pull-over-vh8)
  distinguishes entry speed `0xBB88888A`, later acceleration approximately
  −0.00083333347, and splash speed `0x3991A2B5`. Entry wobble uses a uniform
  [1,2.5)-second range draw. Splash uses one uniform [0,1) double narrowed to
  binary32 seconds for both fade and countdown. A nonzero loaded PullOver
  elapsed counter does not rebuild its unsaved vectors.

<!-- from: core-vehicles5 (2026-10-23) -->
- ~~**VH4 - §2/§3/§5: construction and Init.** Which flags/defaults does each
  named owner set, what does traffic Init acquire, and in what order do failures
  unwind? §3 says only "sets its flags". TrainCar's stated 0x10000000 is known.
  [Reuse existing record defaults as portable staging values; Init explicitly
  reports Unsupported rather than treating staging values as native defaults.]~~

  **ANSWERED 2026-10-23 (re-vehicles-vh5b):** DIFFERS — [vehicles.md §12.3](../../vehicles.md#123-construction-and-init-vh4)
  specifies cumulative constructors, runtime speed draws, flags and Init ordering.
  Traffic acquires no additional city services and performs no local unwind.
  Failed model initialization still invokes Occupant Init; derived owners retain
  their side effects. Train Init chooses family/count; later ticks construct cars.

<!-- from: core-vehicles5 (2026-10-23) -->
- ~~**VH6 - §3/§5: crash/damage/blur.** What direction distribution and transform
  does HandleCollision choose, how is ResolveCollision different from
  UpdateAfterDamage, which event ID/payload is raised, how is the damaged model
  selected, and what blur key/fade formula is used? [Resolved collision direction
  can enter the specified final 5000ms COLLIDE; random selection, damage model
  and blur remain Unsupported. The documented accident and damage flag are
  available, with a partial-operation error for the missing model step.]~~

  **ANSWERED 2026-10-23 (re-vehicles-vh5b):** DIFFERS — [vehicles.md §12.4](../../vehicles.md#124-collision-direction-damage-effects-and-blur-vh6)
  defines normalized facing perturbation, damage accumulation and speed reduction,
  health thresholds, accident `(2,30000)`, event `(11,1)` and named injury/crash
  effects. The primary vehicle mesh is not replaced. Blur uses its existing model
  resource key and four ring samples with successively reduced opacity.

<!-- from: core-vehicles-close11 (2026-10-24) -->
- ~~**VH10 - §12.3 Init virtual bodies and managers §5 slot 158:** §12.3 says
  Vehicle loads drive parameters through slot 158; managers §5 describes that
  slot as a random lateral-offset draw. Specify the inputs, fields and ordering
  of the complete slot-158 body, and Traffic's slot-134 body invoked on fresh
  non-straight Init. Is that body BeginRoute or another route operation, and
  what are its empty-route/failure effects? [Init owner additions/model-failure
  ordering are implemented, but the current InitWorld owner-slot hooks are
  provisional and do not count as closed core algorithms. They must be replaced
  with specified core behavior or an explicit open-contract result.]~~

  **ANSWERED 2026-10-24 (re-vehicles-vh11):** DIFFERS — [vehicles.md §12.7](../../vehicles.md#127-init-lateral-offset-and-initial-path-vh10):
  slot 158 only draws a lateral offset. Slot 134 initializes a path from current
  position without consuming the route; it is not BeginRoute. Its failure is
  ignored by Traffic Init. Eight conformance vectors specify these effects.

<!-- from: core-vehicles-close11 round 2 (2026-10-24) -->
- ~~**VH11 - §12.1 train car identity/runtime binding:** after Read appends a
  shared-stream car reference, how does the core bind it to its owned TrainCar
  runtime, and what happens on a missing or duplicate identity? Can a live car
  ever have a null serializable identity? [One ordered list holds each identity
  and optional runtime. Loaded entries remain unresolved; collision reports
  VH5 before side effects until all are bound. Write rejects a runtime car with
  null identity before output rather than silently losing it on reload. No
  guessed car instance or foreign train-owner callback is created.]~~

  **ANSWERED 2026-10-24 (re-vehicles-vh11):** DIFFERS — [vehicles.md §12.8](../../vehicles.md#128-train-references-are-runtime-car-references-vh11):
  resolve the shared object and query the car interface during Read; no separate
  train binding phase exists. Missing references are skipped and duplicates
  alias the same runtime object. Valid TrainCars expose serialization on that
  same object. Five conformance vectors cover identity and aliasing.

<!-- from: re-vehicles (2026-10-21) -->
- ~~**VH1 - §3, §5, §6:** the U-Drive-It physics of the vehicle, train and tank (`DoPlayerDriveTick`,
  `PlayerDriveUpdateSpeed`, the derail draw, the tank shot) in full. [as read]
  **2026-10-25 narrowing (re-vehicles-vh21):** vehicles.md §§12.10, 12.12,
  12.13 and 12.15 specify command/steering accumulation, signed speed/grade/tween
  rules, Train overspeed/cache/braking/derail, head movement geometry and path
  switching, and the different Vehicle/Train substep counts and collision phases.
  Tank aiming, pivot, elevation/power adjustment, sound thresholds, shot-effect
  attributes, f32 power×0.01 scaling and channel-7 cooldown are now specified.
  Remaining evidence needed: finish Vehicle's on-network displacement and
  reattachment versus off-road anchor/steering geometry in
  `0x005603F0/0x0055ED10`. The Tank firing owner makes no direct occupant-damage
  call; projectile-asset simulation remains behind the effects interface.
  VH1 remains open for the Vehicle geometry, with no full physics closure claimed.~~

  **ANSWERED 2026-10-26 (re-vh30):** DIFFERS — [vehicles.md §12.16](../../vehicles.md#1216-vehicle-player-drive-geometry-vh1)
  completes the remaining Vehicle owner geometry: network projection/reattachment,
  signed front-anchor advancement, unnormalized facing smoothing, rear-anchor
  state, off-road yaw/height correction and obstacle response. Seven worked
  cases complement the existing speed, Train, Tank and substep cases in
  §§12.10/12.12/12.13/12.15. Projectile effects remain the documented foreign
  interface boundary; no direct occupant-damage behavior is invented.

<!-- from: core-vehicles5 (2026-10-23) -->
- ~~**VH7 - §2/§3, automata.md §6.3/§6.5: route representation and geometry.**
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
  an eight-byte adjustment, native save defect or successful reconstruction.~~

  **ANSWERED 2026-10-26 (re-vh30):** DIFFERS — [vehicles.md §12.9](../../vehicles.md#129-route-geometry-and-serialization-boundary-vh7)
  and [automata.md §6.3](../../automata.md#63-traffic-automaton-version-2-write-0x00551de0-read-0x00554d20-requires-2)
  establish serialization receiver `S=P+8` from QI, constructor and vtable evidence;
  the apparently different record/live offsets alias the same fields. The scalars
  are the transport selector, saved traffic option byte, packed path key and
  segment index; a worked record preserves both opaque bytes and leaves missing
  path geometry explicitly unbound. Full post-load closure remains narrowed to
  **VH7-R1/R2/R3 in §12.9**: trace the first geometry binding and index changes,
  the option byte's first operational setter/consumer, and the route annotation's
  producer/source. No runtime observation was made and no native save defect or
  successful automatic reconstruction is claimed. This archive records the
  task's permitted runtime-observation narrowing, not a claim those observations
  have already been supplied.
