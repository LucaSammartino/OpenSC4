# Spec: timers (`cRZTimer`, `cRZCallbackTimer`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce (here the host's clocks), so only
the rule is matched, not each value.

Related specs: [clock24.md](clock24.md) (§4.3, the 24-hour clock's timer model and CK2/CK15, which this spec
extends), [foundations.md](foundations.md) (§18, the x87 precision the conversions run at),
[simulator.md](simulator.md) (§5, the slice budget).

## 0. Summary

`cRZTimer` is a stopwatch: an accumulated 64-bit tick count plus a start stamp, read in one of seven units.
`cRZCallbackTimer` calls a function after a delay, once or repeatedly, through a framework timer service. Neither
is saved. All their values come from the host's clocks *[compat-limit]*; what must match is the arithmetic below.

## 1. `cRZTimer` *[compat]*

### 1.1 State (24 bytes, usually embedded in another object)

The constructor `cRZTimer(unit)` (`0x0088FEDF`, 98 call sites) stores the unit and zeroes the rest.

| Offset | Field | Initial |
|---|---|---|
| `+0x00` | `unit` (int32, §1.2) | the constructor's argument (`SetUnits` changes it) |
| `+0x08` | `start`: the raw counter value when it was started (u64); **0 means stopped** | 0 |
| `+0x10` | `accumulated` raw ticks (u64) | 0 |

### 1.2 Units

| `unit` | Counter read | Reported as |
|---|---|---|
| 0 | the CPU time-stamp counter (`cpuid` then `rdtsc`) | raw cycles |
| 1 | `QueryPerformanceCounter` | raw ticks |
| 2 | `QueryPerformanceCounter` | nanoseconds |
| 3 | `QueryPerformanceCounter` | microseconds (the 24-hour clock's unit, clock24.md CK2) |
| 4 | `QueryPerformanceCounter` | milliseconds |
| 5 | `QueryPerformanceCounter` | seconds |
| 6 | `QueryPerformanceCounter` | minutes |
| other | as unit 1 | always 0 |

Units 2–6 convert raw ticks with a float factor `ticks × f[unit]`, where `f` is
`1e9 / F`, `1e6 / F`, `1000 / F`, `1 / F` and `(1/60) / F` for `F` the performance-counter frequency (§1.5).

### 1.3 Methods

| Method | Address | Behaviour |
|---|---|---|
| `SetUnits(u)` | `0x00890181` | `unit := u`; `start := 0`; `accumulated := 0` |
| `Reset()` | `0x0088FF0A` | `start := 0`; `accumulated := 0` (unit kept) |
| `IsTimerRunning()` | `0x0088FEFB` | `start ≠ 0` (EAX 0 / 1) |
| `Start()` | `0x008905C4` | `start :=` now (the unit's counter, §1.2). It does not test whether it already runs: a running timer loses its open interval (clock24.md §4.3) |
| `Stop()` | `0x008905F0` | if running: `accumulated += interval(now − start)`, `start := 0`. **Now is always read with `QueryPerformanceCounter`**, also for unit 0, whose `start` came from `rdtsc` *[quirk]* |
| `Restart()` | `0x0089058F` | `start :=` now; `accumulated := 0`, running or not (clock24.md CK15) |
| `GetElapsedTime()` → u32 | `0x00890198` | `t := accumulated`, plus `interval(now − start)` when running; units 0–1: the low 32 bits of `t`; units 2–6: `trunc(t × f[unit])` as a 32-bit integer; other units: 0 |
| `GetElapsedTime(uint64& out)` | `0x00890279` | the same `t`; units 0–1: all 64 bits; units 2–6: `trunc(t × f[unit])` in the low half, 0 in the high half; other: 0 |
| `GetElapsedTime(float& out)` | `0x00890377` | the same `t` as a float; units 2–6 multiplied by `f[unit]`; other units 0.0 |
| `SetElapsedTime(uint64 v)` | `0x0089045A` | units 0–1: `accumulated := v`; units 2–6: `accumulated := trunc(v × g[unit])` (`v` loaded as a signed 64-bit integer); other units: unchanged. Then `start :=` now, so the timer runs |
| `SetElapsedTime(float v)` | `0x00890501` | units 0–1: `accumulated := trunc(v)`; units 2–6: `trunc(v × g[unit])`; other units: unchanged. Then `start :=` now |
| `Delay(n)` | `0x0089064D` | a temporary timer of the same unit is started; until its `GetElapsedTime()` reaches `n`, `Sleep(0)` |
| `GetTimerCycle(out)` | `0x0088FF19` | writes `QueryPerformanceCounter` into `out` |

`interval(d)` is the overhead rule: with `o` the calibrated call overhead (§1.5), `d − o` when `d > o`, else **1**
(at least one tick per interval; clock24.md §4.3). Unit 0 uses the same `o`, measured on the performance counter
*[quirk]*.

All three `GetElapsedTime` forms (TM2, confirmed on the instructions):

- On a running unit-0 timer the open interval's "now" is the **cycle counter** (`cpuid`+`rdtsc` at `0x008901C5`,
  `0x008902A7`, `0x008903A4`), as in `Start`; units 1–6 read `QueryPerformanceCounter`. Only `Stop` mixes the
  two clocks.
- "Running" is the whole 64-bit `start` being non-zero. `d = now − start` is a 64-bit wrapping subtraction and the
  test `d > o` is **unsigned** 64-bit (`0x008901EC`–`0x008901F8`).
- For units 2–6 `t` is loaded as a **signed** 64-bit integer (`fild qword`, `0x00890233` …) and multiplied by the
  float `f[unit]`; the float form loads `t` the same signed way for units 0–1 (`0x00890449`). A `t` of 2⁶³ or more
  is therefore read as negative.
- A negative `unit` reports 0 (0.0), like any unit above 6.

`g[unit]` is the stored reciprocal `1 / f[unit]` (§1.5). `trunc` is the CRT's float-to-integer truncation toward
zero to 64 bits, `_ftol2` (`0x009EEF04`, which other specs call `_ftol`); the 32-bit `GetElapsedTime` returns its
low half. The products run on the x87 unit at the caller's precision (24-bit on the game's main thread,
foundations.md §18), so a value near an integer can truncate one lower, or one higher, than exact arithmetic
*[compat-limit]*.

### 1.4 Static factor getters

Six functions return one stored float on the x87 stack (each pops 0): nanoseconds per counter tick `0x0088FF24`,
microseconds `0x0088FF2B`, milliseconds `0x0088FF32`, seconds `0x0088FF39`, minutes `0x0088FF40`, and
milliseconds per **CPU cycle** `0x0088FF47`. The command-system support (time-stamped output) and the glyph-texture
code call them.

### 1.5 Calibration at program start (`0x0088FF4E`, from the static initialisers)

1. Raise the process to real-time priority class and the thread to time-critical, measure the two frequencies
   (`0x0088FDC0`), then restore both priorities.
2. `0x0088FDC0(&F, &C)` (as clock24.md §4.3 describes): up to five times, read `QueryPerformanceCounter` and the
   cycle counter; if the ratio of the two **raw values** (cycles / ticks) is within (0.98, 1.02), the performance
   counter is the cycle counter and `C := F`. Otherwise `C` is measured: `SleepEx(100, alertable)` repeated until at
   least 0.33 s of performance-counter time has passed, then `C := trunc(Δcycles / Δseconds)`. `F` is always
   `QueryPerformanceFrequency` *[compat-limit]*.
3. Store the five counter factors of §1.2 and their reciprocals, then the same five for cycles (`1e9 / C` …
   `(1/60) / C`) and their reciprocals, as floats.
4. The overhead `o` is the **smallest** difference of two back-to-back `QueryPerformanceCounter` calls over 8 tries
   (stored as u64); a cycle overhead (8 `cpuid`+`rdtsc` pairs) is stored too but no `cRZTimer` method reads it.

*[choice]* FreeCity4 may take `F` from the OS and `C` from any estimate, and may measure `o` the same way; tests
must inject `F`, `C` and `o`.

## 2. `cRZCallbackTimer` *[compat]*

A heap object (vtable `0x00AD1F14`) that asks a timer service to call it back.

### 2.1 State

| Offset | Field | Initial |
|---|---|---|
| `+0x04` | uses the framework service (bool) | false before the constructor's `UseTimerService(true)`, then true only if the framework service was found (§2.3) |
| `+0x08` / `+0x0C` | callback function / its argument | 0 / **not written** by the constructor (only `+0x08` is zeroed, `0x00932C41`; the argument is read only when a function is set, so it is never observed) |
| `+0x10` | `delay` (u32) | 100 |
| `+0x14` | `period` (u32) | 1000 |
| `+0x18` | running (bool) | false |
| `+0x19` | one-shot (bool) | false |
| `+0x1C` | the service's timer ID | 0 |
| `+0x20` | the timer service (counted reference, or the built-in default `0x00B62AB0`, or null, §2.3.1) | default (never AddRef'd) |
| `+0x24` | an optional counted object kept alive while the timer runs | null |

Every live callback timer is in a global set (`0x00B62A9C`) guarded by a critical section (`0x00B62AA8`); the
constructor adds it, the destructor removes it.

### 2.2 Methods (vtable `0x00AD1F14`)

| Slot | Method | Address | Behaviour |
|---|---|---|---|
| 0 | deleting destructor | `0x00932C9C` | destructor (`0x00932AFE`): leave the set (under the lock); if running (`+0x18`) call `Stop` directly; release `+0x24` if non-null and null it; `+0x08 := +0x0C := 0`; if `+0x20` is non-null: release it **only when `+0x04` is true**, then `+0x20 := null` (§2.3.1). Free with flag bit 0 |
| 1 | `SetDelay(v)` | `0x0093294A` | `delay := v`; a running timer is restarted (below); true |
| 2 | `GetDelay(out)` | `0x009329A0` (shared getter) | `out := delay`; true |
| 3 | `SetPeriod(v)` | `0x009329AE` | `period := v`; a running timer is restarted (below); true |
| 4 | `GetPeriod(out)` | `0x00932A04` (shared getter) | `out := period`; true |
| 5 | `Start(period, object)` | `0x00932CB8` | refused (false) when running or when no callback is set. Keeps `object` (AddRef) in `+0x24`; a non-zero `period` replaces `period`; running := true; `ID := service.AddTimer(handler, this, period, 0, 0)` (service slot 3). An ID of 0 undoes it (not running, object released) and returns false; else true |
| 6 | `StartOnce(delay, object)` | `0x00932D21` | as slot 5 but one-shot: the delay is the argument, or `period` when it is 0, and it is stored in **neither** `delay` nor `period` (only passed on); `+0x24 :=` object only when it is non-null; running := true, one-shot := true; `ID := service.AddTimer(handler, this, delay, 1, 0)`. An ID of 0 clears running **and** one-shot, releases the object, `+0x24 := null`, false |
| 7 | `Stop()` | `0x00932A12` | if it has an ID: `service.RemoveTimer(ID)` (service slot 4), release `+0x24`; ID, running and one-shot := 0. True |
| 8 | `IsRunning()` | `0x00932B83` (shared) | `+0x18` |
| 9, 10 | `SetCallback(fn, arg)` | `0x00932A4A` (shared) | `+0x08 := fn`, `+0x0C := arg`; true |
| 11 | `UseTimerService(bool)` | `0x00932A5D` | unchanged flag: nothing, **true**. Otherwise `+0x04 :=` the argument. True: ask the framework for system service `0xA76C78E7` (interface `0x276C784B`); if found, call its slot 3 (the scheduler it owns, §2.5, a plain getter: no AddRef), and if that differs from `+0x20` release the old one (when non-null, **even the default**), store it, and AddRef it when non-null (a null result is stored, `+0x04` stays true, §2.3.1); release the system service; true. Neither branch touches the ID or the running flag (§2.3.1). Not found (or no framework): `+0x04 := false`; unless `+0x20` is already the default, release it and store the default; **false**. False: `+0x20 :=` the default **without releasing** the old service, true *[quirk]* |
| 12 | `GetTimerService()` | `0x009D8564` (shared getter) | `+0x20` |

**Restart** (slots 1 and 3): keep a reference on `+0x24`, call `Stop` (slot 7), then call slot 5 or 6 by the
one-shot flag with `(0, null)`, and put the kept object back in `+0x24`. `Stop` has already cleared the one-shot
flag, so the restart is **always slot 5**: a running one-shot timer becomes periodic with `period` *[quirk]*.

`delay` (`+0x10`) is stored, reported and restarts the timer, but neither start passes it to the service: it has no
other effect in this class *[quirk]*.

### 2.3 Construction (`0x00932C23`)

Sets the initial values of §2.1 (`+0x20 := 0` first), joins the global set, stores the default service in `+0x20`
**without AddRef** (`0x00932C86`) and sets `+0x04 := false` (`0x00932C8D`; an earlier store of 1 is overwritten),
then calls `UseTimerService(true)` directly (not through the vtable): the flag differs, so the lookup always runs,
and with a framework timer service present it uses that one. Its result is discarded.

### 2.3.1 Service reference counts (TM3)

**The scheduler's counting.** The scheduler's AddRef (slot 1, `0x009334B1`) and Release (slot 2, `0x009334C2`)
count only when its byte `+0x09` ("counted") is set: AddRef increments `+0x0C` and returns it; Release returns
`count − 1` after decrementing when `count > 1`, else calls its deleting destructor (slot 8) with 1 and returns 0.
When `+0x09` is clear both do nothing and return 1. The scheduler constructor clears `+0x09` and `+0x0C`; the
**default** (`0x00B62AB0`) is never marked, so AddRef/Release on it are no-ops and it is never destroyed by them.
The framework service's constructor (`0x009304A6`) heap-allocates its scheduler, marks it counted (`0x009334E7`
with 1) and AddRefs it once: count 1, owned by the service.

So the callback timer's unbalanced calls on the default are harmless *[compat]*:

| Event | Calls on the default | Calls on the framework scheduler S |
|---|---|---|
| construction, framework service found, slot 3 = S | 1 Release (no-op) | 1 AddRef |
| construction, no framework or service not found | none (`+0x20` already the default) | none |
| `UseTimerService(false)` while on S | none (stored, no AddRef) | **none** (leak, quirk 8) |
| `UseTimerService(true)` while on the default | 1 Release (no-op) | 1 AddRef |
| destruction, `+0x04` true, `+0x20` = S | none | 1 Release |
| destruction, `+0x04` false | none (`+0x20` is not released whatever it holds) | none |

The destructor's release is gated on the flag, not on the pointer: whenever `+0x04` is false `+0x20` holds the
default (both false paths of slot 11 store it), so the default is released only in the unreachable-in-practice case
where the framework's slot 3 returns the default itself.

**Null scheduler.** If the framework service is found but its slot 3 returns null, the null is **stored**: the old
`+0x20` is released (when non-null; the default's release is a no-op), `+0x20 := null`, no AddRef, `+0x04` stays
true, the system service is released and slot 11 returns true. It is not treated as "not found". A later
`UseTimerService(true)` is a no-op (flag unchanged); `UseTimerService(false)` stores the default. The destructor
skips the release (null). `Start`, `StartOnce` and `Stop` with an ID call through `+0x20` unconditionally, so they
would dereference null in the original *[compat-limit]*: FreeCity4 must not crash there and may treat the call as a
failed `AddTimer` (ID 0) / a no-op `RemoveTimer` *[choice]*. The game's framework service never returns null.

**Switching while running.** Slot 11 changes only `+0x04` and `+0x20`: the ID, running and one-shot flags and the
service's entry stay as they were (no stop, no restart, no re-registration). The entry stays in the **old**
scheduler and keeps firing the handler, which checks only that the timer is alive (§2.4), so the callback keeps
running. A later `Stop` (or a restart through slot 1 or 3, or the destructor's `Stop`) sends `RemoveTimer(ID)` to the
**new** service; its result is ignored, so `Stop` still clears ID / running / one-shot, releases `+0x24` and returns
true, while the old scheduler's entry survives and keeps calling the callback until the timer is destroyed (then the
handler finds it gone and does nothing) *[quirk]*. IDs are per-scheduler counters, so that `RemoveTimer` removes the
new scheduler's entry with the same ID if there is one, which may belong to another timer *[quirk]*.

### 2.4 When the service fires (handler `0x00932B87`)

Under the global lock: if the timer is no longer in the set (destroyed), nothing happens. Otherwise read the
callback and its argument, take a reference on `+0x24`, and for a **one-shot** timer clear it (release `+0x24`,
ID := 0, running := false, one-shot := false). Leave the lock, then call `fn(arg)` (cdecl, one argument) if `fn` is
set, then release the reference taken. A periodic timer stays registered and keeps firing until `Stop`.

How often the service calls back, and on which thread, is the scheduler's behaviour (§2.5).

### 2.5 The timer scheduler (TM1)

Both services are the same class: a scheduler with its own worker thread (constructor `0x009335F4`, vtable
`0x00AD1F6C`, runnable vtable `0x00AD1F68`, 0x50 bytes). The **default** is a static instance at `0x00B62AB0`
(static initialiser `0x00A7933E`). The **framework service** `0xA76C78E7` (constructor `0x009304A6`, added by
`cSC4App::AddSystemServices`, priority 2,999,999) owns a heap instance of it and returns it from its slot 3
(`0x009D8564`). Its own clock is a `cRZTimer` of unit 4 (milliseconds), started at construction; every time below
is that clock's 32-bit `GetElapsedTime`.

Entries (in a vector, in insertion order): ID, function, argument, one-shot, `period`, `due`, fixed-rate.

| Slot | Method | Address | Behaviour |
|---|---|---|---|
| 3 | `AddTimer(fn, arg, ms, oneShot, fixedRate)` | `0x009337F9` | 0 if the scheduler is not enabled (`+0x08`, set at construction). Under its lock: ID := a counter starting at 1 (incremented per call, never 0 in practice); `period := ms`; `due := now + ms`, saturated to `0xFFFFFFFF` on overflow **only when the list was empty**; append. If the list was empty: start the worker thread if it is not alive, and `ResumeThread` it. Always queue a no-op APC to the thread (wakes its alertable sleep). Returns the ID |
| 4 | `RemoveTimer(id)` | `0x0093357B` | under the lock, erase the first entry with that ID: true; none: false (also false when not enabled) |

The 4th argument is the **one-shot** flag; the 5th selects **fixed-rate** rescheduling (`due += period`) instead of
the default `due := now + period`. `cRZCallbackTimer` always passes 0, so its periodic timers drift by the
callback-dispatch latency.

**Worker loop** (`0x009338C2`), while the scheduler is enabled and the list is not empty:

1. Under the lock read `now` once. Walk the list in order; an entry with `due ≤ now` (unsigned) is **copied** to a
   fire list, then a one-shot entry is erased and a periodic one gets `due := now + period` (fixed-rate:
   `due += period`); if the new `due < now`, `due := 0xFFFFFFFF` *[quirk: a fixed-rate entry that falls more than one
   period behind stops firing]*. Every remaining entry contributes `due − now` to the minimum wait `m`.
2. Leave the lock, then call each copied `fn(arg)` in list order **on the worker thread**. A timer removed after the
   copy still fires this once; `cRZCallbackTimer`'s handler (§2.4) checks the live set for that reason.
3. If the list is now empty, the thread suspends itself (`SuspendThread`) until the next `AddTimer` into an empty
   list resumes it. Otherwise with `e = now' − now`: sleep `m − e` ms if `e < m`, else **10 ms**, in an alertable
   `SleepEx` (the time passes through `cRZThread::SleepCurrentThread` as `ms × 1000` in 32 bits, so a wait over
   4,294,967 ms wraps *[quirk]*).

So every `cRZCallbackTimer` callback runs on the scheduler's worker thread, never on the caller's or the main thread
*[compat]*; FreeCity4 may use any single dispatch thread with the same ordering, and tests drive the loop with an
injected millisecond clock.

## 3. Users

- `cRZTimer` (set up with `SetUnits`): the 24-hour clock and the simulator's slice budget (clock24.md), the graphics
  system, the traffic simulator, the disaster layer and the riot and fire instances, the automata script system,
  the input-control dispatchers and the drive control, the region-view clouds. The command-system support and the
  glyph-texture manager read the factor getters (§1.4).
- `cRZCallbackTimer` (constructed inside its owner): the text-edit window, the scrollbar, the tooltip managers,
  the news ticker, the splash screen, the catalog view, the tertiary menu frame, the effects manager, the query tool
  (its coverage-ring callback) and the network stream socket's `Connect`.

## 4. Quirks

1. `Stop` reads the performance counter even for a unit-0 (cycle) timer (§1.3).
2. Unit 0 uses the performance counter's call overhead (§1.3).
3. Every interval counts at least one tick (§1.3).
4. `Start` on a running timer drops the open interval (§1.3).
5. Conversions truncate at the caller's x87 precision (§1.3).
6. `cRZCallbackTimer`'s `delay` is never passed to the service (§2.2).
7. `SetDelay` or `SetPeriod` on a running one-shot timer makes it periodic (§2.2).
8. `UseTimerService(false)` drops the framework service without releasing it (§2.2).
9. The scheduler sleeps 10 ms when an entry is already overdue, saturates a fixed-rate entry that lags, and wraps
   waits over 4,294,967 ms (§2.5).
10. The constructor and `UseTimerService(true)` release the default service they never AddRef'd; harmless because
    the default is not counted (§2.3.1).
11. Switching the service of a running timer leaves its entry firing in the old scheduler; `Stop` removes the ID
    from the new one (§2.3.1).

## 5. Open questions

None (TM1 answered in §2.5, TM2 in §1.3 / §2.2 / §2.3, TM3 in §2.3 / §2.3.1).

## 6. Conformance tests

With an injected counter of frequency `F = 1,048,576` ticks/s (2²⁰: every factor but the minutes one is exact in a
float, so the results do not depend on the x87 precision) and overhead `o = 3`:

1. **Units:** a timer started at tick 1000 and read at 1,049,579 (no stop) has `t = 1,048,576`; it reports
   1,048,576 ticks (unit 1), 1,000,000,000 ns (unit 2), 1,000,000 µs (unit 3), 1000 ms (unit 4), 1 s (unit 5) and
   0 min (unit 6, truncated).
2. **Overhead:** start at 100, stop at 102 (difference 2 ≤ `o`): `accumulated = 1`; start at 100, stop at 103:
   `accumulated = 1`; start at 100, stop at 200: `accumulated = 97`.
3. **Accumulation:** two start/stop pairs of 1003 ticks each give 2000 ticks; `Reset` gives 0 and stopped;
   `Restart` on a stopped timer starts it with 0.
4. **SetElapsedTime:** unit 5, `SetElapsedTime(uint64 2)` gives `accumulated = 2,097,152` ticks and a running timer;
   unit 9, `SetElapsedTime(7)` leaves `accumulated` unchanged and starts the timer.
5. **Unknown unit:** unit 9 reports 0 from all three `GetElapsedTime` forms while counting like unit 1.
6. **Callback timer:** with a scripted service, `Start(0, null)` without a callback returns false; with a callback
   it registers `(handler, timer, 1000, 0, 0)`; `StartOnce(250, obj)` registers `(…, 250, 1, 0)`, and one firing
   calls the callback once and leaves the timer stopped; `Stop` removes the ID; `SetPeriod(50)` on a running
   periodic timer re-registers it with `(…, 50, 0, 0)`, and on a running one-shot timer too (it is then periodic);
   the kept object survives the restart with one reference.
7. **Service counts (TM3)** with two scripted schedulers: the default D (uncounted: AddRef/Release do not change it)
   and the framework's S (counted, count 1 held by the service), a recording framework service F whose slot 3 returns
   S, and a log of every AddRef/Release call:
   - *Default replacement:* construct a timer with F present: calls are exactly `D.Release` ×1, `S.AddRef` ×1
     (S count 2), F looked up and released once; `+0x04` true, service S, slot 11 result discarded. Destroy it
     (never started): `S.Release` ×1 (count 1), no call on D; `+0x20` null afterwards.
   - *No framework:* construct: zero AddRef/Release calls on D, `+0x04` false, service D. Destroy: zero calls.
   - *Leak:* construct with F, then `UseTimerService(false)` → true, service D, `+0x04` false, no call on S (count
     2) or D; destroy: no call on S or D (S count stays 2). `UseTimerService(false)` again → true, no calls.
   - *Null slot 3:* F's slot 3 returns null: construct → `D.Release` ×1, no AddRef, service null, `+0x04` true;
     `UseTimerService(true)` → true with no lookup (flag unchanged); destroy → no service call.
   - *Running switch:* construct with F absent (service D), set a callback, `Start(0, null)` → D registers ID 1
     (period 1000). Make F present, `UseTimerService(true)` → true; calls `D.Release` ×1, `S.AddRef` ×1; D's entry
     ID 1 is untouched, S has none; running still true, ID 1. Let D fire: the callback runs. `Stop()` → true;
     S receives `RemoveTimer(1)` (returns false), D receives nothing; ID 0, not running. D fires again: the callback
     still runs. Destroy the timer (`S.Release` ×1); D fires: nothing is called. Variant: S already holds another
     timer's entry with ID 1: that `Stop` removes it.
