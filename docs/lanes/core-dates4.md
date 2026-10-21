| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-dates4: dates and times core (cRZDate, cRZTime, cGZDate per docs/specs/dates.md) | `core-dates4` / (engine lane) | in progress | see Handoff |

## Handoff

- Goal: pure-Rust owners of docs/specs/dates.md in engine/core/src/dates/ (calendar u32 math, cRZTime with typed
  host time-zone input, cGZDate 46-slot COM model), unit tests for spec §8, conformance vectors dt8_*.
- Done: worktree created from main a01b2687.
- Next: write engine/core/src/dates/{mod.rs,time.rs,com.rs,tests.rs}; register `pub mod dates;` in core lib.rs.
- Facts: existing engine/core/src/julian.rs is a signed i32 helper (simulator); dates/ is separate, u32 exact.
  No plugin/CAPI/CMake/ABI edits (task). Core-only lane: no game runs planned.
