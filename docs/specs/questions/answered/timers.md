# timers: answered questions (archive)
<!-- Archive for [timers.md](../../timers.md). Lanes do not read this file. Open questions are in ../timers.md. -->

<!-- from: re-timers (2026-10-19) -->
- **TM1 - §2.2, §2.4:** what is the framework timer service (`0xA76C78E7`, interface `0x276C784B`) that
  `cRZCallbackTimer` asks for, and the built-in default object at `0x00B62AB0` (built by `0x009335F4`) it falls back
  to? Which thread calls the handler, and what do the fourth argument of the service's slot 3 (1 for `StartOnce`, 0
  for `Start`) and the fifth (always 0) mean?
  **ANSWERED 2026-10-21 (re-timer-core-answers4):** MATCHES — both are the same scheduler class (`0x009335F4`, vtable
  `0x00AD1F6C`): the default is a static instance, the framework service (`0x009304A6`) owns a heap instance and
  returns it from slot 3 (`0x009D8564`). Callbacks run on the scheduler's own worker thread (`0x009338C2`), outside
  its lock, in list order. AddTimer (`0x009337F9`) argument 4 = one-shot (entry erased after firing), argument 5 =
  fixed-rate rescheduling (`due += period` instead of `due := now + period`); clock is a unit-4 (ms) `cRZTimer`.
  Full rule in timers.md §2.5.

<!-- from: core-timers4 (2026-10-21) -->
- **TM2 - §1.3, §2.2:** four literal readings the engine took, please confirm or correct. (a) `GetElapsedTime` (all
  three forms) on a running unit-0 timer: is the open interval's "now" the cycle counter (as `Start`) or, like
  `Stop`, `QueryPerformanceCounter`? The engine reads the cycle counter. (b) In `GetElapsedTime`, does `t` enter the
  product `t × f[unit]` as a signed 64-bit integer (`fild`, as `SetElapsedTime` says), and is `now − start` compared
  with `o` unsigned? The engine uses signed for the product and unsigned for the comparison. (c) `StartOnce(delay,
  obj)` with a non-zero argument: is the argument stored in `period` (as `Start` does) or in `delay`? The engine
  stores neither. (d) When `AddTimer` returns 0 in `StartOnce`, is the one-shot flag cleared too? The engine clears
  it. Also: `UseTimerService` with no change, what does it return (engine: true), and does `+0x04` start false so
  the constructor's call to slot 11 does the lookup (engine: yes)?
  **ANSWERED 2026-10-21 (re-timer-core-answers4):** MATCHES — (a) cycle counter: `cpuid`+`rdtsc` at `0x008901C5`,
  `0x008902A7`, `0x008903A4`; (b) signed `fild qword` (`0x00890233`…) and unsigned 64-bit compare
  (`0x008901EC`–`0x008901F8`); (c) neither (`0x00932D32`–`0x00932D3B` rewrite only the stack argument); (d) both
  `+0x18` and `+0x19` cleared (`0x00932D77`, `0x00932D7A`); no change returns true (`0x00932AF6`); the constructor
  stores `+0x04 := 0` at `0x00932C8D` and calls `0x00932A5D` directly. Additional rules now in §2.2:
  UseTimerService keeps the system service's slot-3 result, not the service itself, and `UseTimerService(false)`
  overwrites `+0x20` without releasing (quirk 8). Rules in timers.md §1.3, §2.2, §2.3.
