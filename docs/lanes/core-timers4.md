| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-timers4 (engine: timers.md cRZTimer / cRZCallbackTimer in Rust core) | `core-timers4` / `.claude/worktrees/core-timers4` | in progress | see Handoff |

## Handoff
- Goal: implement docs/specs/timers.md owners `cRZTimer`, `cRZCallbackTimer` in engine/core/src/timers/ (core only, no glue).
- Done: worktree created at main 530577d7.
- Next: baseline `qbuild all`, then write engine/core/src/timers/{mod,rz_timer,callback_timer,tests}.rs.
- Facts: design = injected counters (TimerHost: qpc/rdtsc/sleep0) + Calibration{F,C,o, f32 factors}; products via x87::mul_i64_f32, trunc via x87::ftol.
