# vehicles: answered questions

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
