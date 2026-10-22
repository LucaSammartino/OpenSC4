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
<!-- from: core-callbacktimer-final4 (2026-10-22) -->
- **TM3 - §2.2 slot 0 / slot 11, §2.3:** three literal readings of the service reference counts, please confirm or correct. (a) Does the constructor AddRef the default scheduler `0x00B62AB0` when it stores it in `+0x20` ("takes the default service")? The engine does not, so the AddRef/Release pairs on the default are unbalanced: slot 11 releases it when the framework scheduler replaces it, and the destructor releases `+0x20` whichever it is (also the default). [no AddRef; destructor releases the default too] (b) In slot 11 true/found, if slot 3 returns null, is the null stored (released old, no AddRef) or treated as not found? [the engine's slot 3 never returns null] (c) Slot 11 on a running timer: the ID stays registered with the old service and a later `Stop` sends it to the new one (no restart, no re-registration)? [yes, as written] **ANSWERED 2026-10-22 (re-callbacktimer-final4):** DIFFERS — (a) no AddRef on the default (ctor `0x00932C86`), but the destructor releases `+0x20` only when `+0x04` is true (`0x00932B71`), so it never releases the default in practice; the default's AddRef/Release are no-ops because its counted byte `+0x09` is clear (`0x009334B1`/`0x009334C2`), so the ctor's/slot 11's `D.Release` is harmless; (b) a null slot-3 result is stored (old released, no AddRef, flag stays true, returns true; `0x00932AB7`); (c) MATCHES: slot 11 never touches ID/running, `Stop` sends `RemoveTimer(ID)` to the new service, and the old scheduler's entry keeps firing the callback until the timer is destroyed. Spec timers.md §2.2 slot 0/11, §2.3, §2.3.1, §6 test 7.
