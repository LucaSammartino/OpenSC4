# Open spec questions

Questions from engine authors that the spec writers need to answer from the original. Answered questions move into
the relevant spec, and their entry here is deleted.

## Residential (residential.md)

- RS1. Is the `0.01` in the percentage property getters (the `SC4Percentage` / "% × 0.01" readers, and the
  `× 0.01` of §5.6 step 4 and §6.4 slots 30/31) the **float** or the **double** constant? residential's own worked
  examples require double: §15 test 8 needs `pct(80.0)` to be exactly `0.8f32` (`0x3F4CCCCD`) for EQ 50 → 40 and
  HQ 60 → 48 (the float constant gives `0.79999995` = `0x3F4CCCCC`, so `50 × 0.79999995` = `39.999996`), and §15
  test 10 needs `1200 × 0.01` to truncate to 12, not 11 (the float constant gives `11.999999`). But
  `engine/core/src/protection/algo.rs::pct` multiplies by the **float** `0.01`, and protection's vectors pass with
  it. The core uses double `0.01` throughout `engine/core/src/residential/`. Please confirm from the getter's bytes
  (`0x005FDAC0` and friends) whether the constant is a `float` or a `double` literal, and whether one of the two
  subsystems has a wrong worked example or a compensating one-ulp difference. This is a ±1 ulp on every percentage
  in both cores, so it is worth settling once rather than per subsystem.
- RS2. §5.5 step 5 reads the month with `GetSimDate` and ages the tract only when `month = 12`. Is the month the
  simulator's current month at the time state 4 reaches the tract, or a month latched at the start of the pass? The
  core reads it per tract (the literal reading), which can differ only if a month boundary falls inside one
  state-4 pass — impossible in the original (one state per day) but reachable if a host runs a whole pass in one
  call across a date change.
- RS3. §5.5 step 1: the pollution average's accumulator width. REH §4.7 says "u32 division", and the core sums the
  `S × S` `GetPollutionValue` results (each 16 bits) in a wrapping `u32`. With `S = 4` the maximum is 1,048,560, so
  no wrap; a large `S` would wrap. Please confirm the original's accumulator is 32-bit and wraps rather than being
  wider.
- RS4. §5.6 step 4 and REH §4.8 `GetGlobalEQ`/`GetGlobalHQ`: the per-tract traversal order is not stated. Float
  addition is not associative, so the accumulated `sum` depends on it. The core walks the grid's tracts x outer,
  z inner (the same order as `ProcessTwoGridsWithCallback`, simgrid §7.3). Please confirm, or state that the
  original's order is the raw-data order of the grid (which is the same thing for a contiguous row-major grid).
- RS5. §5.3 step 2: the distance sweep's running `d` starts at 255 and computes `d := min(d + 1, v)`. If `d` is a
  byte accumulator, `255 + 1` wraps to 0 and the whole field collapses to 0 on the first sweep; the core widens `d`
  to `i32` and clamps the stored value to `0..=255`, which reproduces the Manhattan distance capped at 255 that
  §5.3 states and that §15 tests 1–3 require. Please confirm the original widens too (or that its `min` is
  evaluated in a wider type), since the two differ for every grid with no seeded tract.
- RS6. §6.2 `GetAverageEQGrid`/`GetAverageHQGrid`: are `min`/`max` updated with every tract including unpopulated
  ones (REH §4.8 says yes: "Each is updated with every tract, populated or not")? The core follows REH. With an
  empty city that gives `min = max = 0.0` rather than the initial `255.0` / `0.0`, which a caller may observe.
  residential.md §6.2 only says "track the stored values", so please confirm REH is normative here.
