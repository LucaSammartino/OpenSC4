| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-timers4 (engine: timers.md `cRZTimer` / `cRZCallbackTimer` in Rust core) | `core-timers4` / `.claude/worktrees/core-timers4` | **ready to merge** (core only, no ABI, no glue). New `engine/core/src/timers/` (rz_timer.rs, callback_timer.rs, 13 tests incl. all six §6 conformance tests); question TM2 filed. Gate: cargo 3993/0, clippy 0, ctest 65/65 | coverage map: point the timers row's core paths at `core/src/timers/rz_timer.rs` (owner `cRZTimer`) and `core/src/timers/callback_timer.rs` (owner `cRZCallbackTimer`); glue/shadow later if a user subsystem needs it; answer TM1/TM2 |

## Report

**What changed** (core only; `engine/core/src/lib.rs` gets `pub mod timers;` at the end):
- `engine/core/src/timers/rz_timer.rs`: `RzTimer` = `cRZTimer` (§1.1-1.3): unit/start/accumulated, `start==0` = stopped;
  SetUnits, Reset, IsTimerRunning, Start (no running test, quirk 4), Stop (always QPC, quirk 1), Restart, the three
  GetElapsedTime forms (units 0-1 raw / low 32 bits, 2-6 `trunc(t x f)` at 24-bit via `x87::mul_i64_f32` + `x87::ftol`,
  other units 0), both SetElapsedTime forms (`g = 1/f`, signed load), Delay (Sleep(0) loop), GetTimerCycle. Overhead
  rule `d>o ? d-o : 1`, wrapping u64 arithmetic. `Calibration` (§1.4/1.5): injected `new(F, C, o)` or `measure(host)`
  (priority raise/restore, 5 raw-ratio tries in (0.98,1.02), else SleepEx(100) loop to >= 0.33 s, min-of-8 overheads),
  float factors + reciprocals for counter and cycles, the six §1.4 getters. Clocks injected via `TimerHost` /
  `CalibrationHost` [compat-limit].
- `engine/core/src/timers/callback_timer.rs`: `CallbackTimer` = `cRZCallbackTimer` (§2): all 13 slots, Drop = destructor;
  `CallbackTimerSet` = the global set + lock + built-in default service + framework lookup (`ServiceLocator`, ids
  0xA76C78E7/0x276C784B); `fire(key)` = handler §2.4 (destroyed: no-op; one-shot cleared under the lock; callback
  outside it; kept object add_ref/release). Restart quirk 7 (always periodic), delay never passed (quirk 6).
  Services injected via `TimerService` (TM1 open).

**Tests** (`engine/core/src/timers/tests.rs`, 13): §6 tests 1-6 verbatim (F=2^20, o=3), unit-0 quirks, wrap/zero-start
edges, Delay, factor getters, calibration both paths, failed AddTimer, fire after destroy, UseTimerService switching.

**Gate**: baseline cargo 3980/0, ctest 65/65; final cargo 3993/0, clippy -D warnings 0, CMake build + ctest 65/65.
No game run: core only, no glue/shadow exists for these owners.

**Questions**: TM2 (docs/specs/questions/timers.md): unit-0 read counter in GetElapsedTime, signed load of `t`,
StartOnce storing its argument, one-shot flag on AddTimer failure, UseTimerService no-change return and the initial
`+0x04`. TM1 (service thread/arguments) remains open; the service is injected.

**Coverage**: owners `cRZTimer` and `cRZCallbackTimer` fully implemented in core (every spec method/quirk); core
paths `core/src/timers/rz_timer.rs`, `core/src/timers/callback_timer.rs` (the map lives under tools/, left to the
coordinator).
