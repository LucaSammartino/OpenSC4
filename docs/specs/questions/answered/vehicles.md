# vehicles: answered questions

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
