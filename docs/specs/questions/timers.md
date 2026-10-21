# timers: open questions
<!-- Questions about [timers.md](../timers.md). Append new entries at the END; when one is answered, move it to answered/timers.md. Rules: README.md. -->

<!-- from: re-timers (2026-10-19) -->
- **TM1 - §2.2, §2.4:** what is the framework timer service (`0xA76C78E7`, interface `0x276C784B`) that
  `cRZCallbackTimer` asks for, and the built-in default object at `0x00B62AB0` (built by `0x009335F4`) it falls back
  to? Which thread calls the handler, and what do the fourth argument of the service's slot 3 (1 for `StartOnce`, 0
  for `Start`) and the fifth (always 0) mean?

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
