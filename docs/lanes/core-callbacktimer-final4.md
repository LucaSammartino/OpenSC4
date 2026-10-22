| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-callbacktimer-final4 (engine: timers.md §2.2 `cRZCallbackTimer` exact UseTimerService / service ownership) | `core-callbacktimer-final4` / (engine lane) | **ready to merge** (core only, no ABI, no glue, no game run). Slot 11 exact incl. quirk 8, typed `ServiceLocator` -> `FrameworkTimerService` lookup, scheduler refcounts; 3 new tests. Question TM3 filed. Gate: cargo 4102/0, clippy 0, ctest 65/65 | 1. regenerate docs/specs/QUESTIONS.md (`tools/questions.py index`, lanes may not run it) for TM3; 2. answer TM3; 3. glue/shadow only when a user subsystem needs it |

## Report

**Changed** (`engine/core/src/timers/callback_timer.rs`, `tests.rs`, `mod.rs`; commit a0aacdb5):
- `TimerService` (the §2.5 scheduler, a separate owner) gains `add_ref`/`release`; `remove_timer` returns bool (ignored by the timer).
- New `FrameworkTimerService` (system service `0xA76C78E7`/`0x276C784B`: `scheduler()` = its slot 3, a plain getter; `release()`), and `ServiceLocator::system_service(clsid, iid)` returns it with one reference for the caller. Replaces the old `ServiceLocator::timer_service`.
- Slot 11 `UseTimerService`: no change -> true; `+0x04 :=` argument; true: lookup, if slot 3 differs from `+0x20` (pointer identity) release old, store, AddRef; release the system service; true. Not found / no framework: flag false, release `+0x20` unless it is the default, store default, false. False: default stored without releasing (quirk 8).
- Destructor releases `+0x20` (whichever it is). Slot 6 stores `+0x24` only for a non-null object; slot 5 failure no longer touches the one-shot flag (it never set it). New `uses_default_service()` accessor.

**Audit of slots 0-12** against §2.2-2.4: getters 2/4/8/12, SetCallback 9/10, Start 5 (period replaced even on failure), StartOnce 6 (delay neither stored), Stop 7 (remove + release only with an ID), restart (always slot 5, kept object restored, also after a failed re-add), handler (destroyed timer ignored, one-shot cleared under the lock, ref held across the call): all match. The "unless `+0x20` is already the default" release branch of not-found is unreachable through this class (flag false implies the default).

**Tests** (3 new, all §6 ones kept): `use_timer_service_ownership` (lookup IDs, ref traffic across construct / no-change / quirk 8 / re-enable / drop), `use_timer_service_same_and_missing` (slot 3 == default: no traffic; not found and no framework: false, no release), `start_failure_and_object_refcounts` (slot 5 failure, Stop without ID, failed restart keeps object, slot-6 null keeps `+0x24`, slot-5 null leaks stale ref, destructor remove + service release).

**Questions:** TM3 (a) constructor AddRef of the default / destructor releasing the default, (b) null slot-3 result, (c) slot 11 on a running timer. QUESTIONS.md index not regenerated (tools/ off-limits to lanes).

**Gate:** cargo test --workspace 4102/0, clippy 0, ctest 65/65. No game run: pure core change, no plugin glue or shadow.
