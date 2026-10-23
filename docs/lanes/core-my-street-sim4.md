| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-my-street-sim4: first portable Street owner | Codex, finished by Claude / `core-my-street-sim4` (engine lane) | ready; cargo 5745/0, clippy 0 diagnostics, ctest 68/68; 19 Street tests; no ABI or game run | 1. Answer ST1-ST5 in docs/specs/questions/my-street-sim.md. 2. Wire the owner into runtime registration only in a later lane. |

## Report
- `engine/core/src/mysims/street.rs` is a source-only owner of my-street-sim.md §§2.1-3.9: construction stores and
  helper retention, Init/Shutdown/destroy, class ID, Read/Write wrappers, day/day-events/month gates, tracking getter,
  Attach (ordered classification table, random vehicle state, TrackOccupant), Detach, Show/HideLocation, message
  dispatch, CreateMyStreetSimTool and the private trip update (grid outputs, reverse lot scan with counts, landmark
  name and funding, inclusive-rectangle disaster scan, ordered cleanup).
- The `Host` trait exposes only individual raw primitives (AL versus full EAX, raw handles). The owner makes every
  decision itself. Existing `InStream`/`OutStream` traits are reused.
- Numeric conversions follow `_ftol2` (truncate to qword, low EAX). Percentages use the literal float32-spilled
  reciprocal order at binary64 (PC53). NaN, infinity and values outside the qword range return
  `Unsupported::NumericDomain`, and the already-executed prefix is kept (ST3).
- Null pointers that the spec leaves unchecked return `Unsupported::InvalidOutput`, never a normal false.
- Gate: cargo 5745 passed / 0 failed (summed over every `test result:` line), clippy -D warnings 0 diagnostics,
  CMake build PASS, ctest 68/68.
- Not done (deliberately): no runtime registration (the ordinary MySim alias for the Street class ID is untouched),
  no plugin/C ABI wiring, no native allocation/free or vtable install, no Street snapshot codec or copy constructor,
  no general reentrancy/exception policy (ST5). Tests are the spec's proposed literal rows, not native goldens.
- Questions: none new; ST1-ST5 stay open in docs/specs/questions/my-street-sim.md.
