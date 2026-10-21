| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-timer-core-answers4 (RE: answer timers TM1, TM2) | `re-timer-core-answers4` / `.claude/worktrees/re-timer-core-answers4` | **ready to merge** (docs only). TM2 all MATCHES the core reading; TM1 answered (new timers.md §2.5 scheduler). No open timers questions | core: optional additions from §2.2 (UseTimerService keeps slot-3 result; false path does not release, quirk 8) and §2.5 scheduler if a dispatch thread is modelled |

## Report

Static analysis only (re_query.py asm --bytes, decomp/, PE reads of vtables and the import table); REA was not needed.

**TM2: MATCHES on every point** (the engine's literal readings are right):
- (a) the "now" of a running unit-0 timer in all three `GetElapsedTime` forms is `cpuid`+`rdtsc` (`0x008901C5`, `0x008902A7`, `0x008903A4`).
- (b) `t` is loaded signed (`fild qword`, `0x00890233`…, float form `0x00890449` for units 0-1); `now − start > o` is an unsigned 64-bit compare (`0x008901EC`-`0x008901F8`). Negative unit reports 0.
- (c) `StartOnce` stores the argument in neither field (`0x00932D32`-`0x00932D3B` only rewrite the stack argument); service call is `AddTimer(handler, this, delay, 1, 0)`.
- (d) on ID 0 both running and one-shot are cleared (`0x00932D77`, `0x00932D7A`).
- `UseTimerService` with no change returns true (`0x00932AF6`); the constructor stores `+0x04 := 0` at `0x00932C8D` before a direct call to `0x00932A5D`.
- Extra rules written into §2.2: the kept service is the result of the system service's slot 3 (`0x00932AA2`), the system service itself is released; `UseTimerService(false)` overwrites `+0x20` with the default without releasing the old one (quirk 8). A core that stores the system service itself would DIFFER only in identity, not in observable timing.

**TM1: answered** in new timers.md §2.5. Default (`0x00B62AB0`, static init `0x00A7933E`) and framework service (`0x009304A6`, slot 3 = `0x009D8564` returns its heap scheduler) are the same scheduler class (`0x009335F4`, vtable `0x00AD1F6C`). AddTimer `0x009337F9`(fn, arg, ms, oneShot, fixedRate), RemoveTimer `0x0093357B`; worker loop `0x009338C2` on its own thread (SuspendThread when empty, ResumeThread + QueueUserAPC on add, alertable SleepEx of min-wait or 10 ms). Callbacks run on the worker thread, outside the lock, in list order. Quirk 9 lists the 10 ms overdue sleep, fixed-rate lag saturation to 0xFFFFFFFF and the 32-bit `ms × 1000` wrap.

Commits: 3767a020 (handoff), ff9dcc5b (spec + answered archive + QUESTIONS.md index).
