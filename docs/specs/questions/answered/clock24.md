# clock24: answered questions (archive)
<!-- Archive for [clock24.md](../../clock24.md). Lanes do not read this file. Open questions are in ../clock24.md. -->

## From the commercial, industrial, civic, park, history and clock specs (re-specs9 lane, 2026-10-14)

Open points from commercial-industrial.md, civic-buildings.md, parks.md, history.md and clock24.md. The current
answer is in brackets; each entry says what would settle it.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK1:** Does the clock run while the player pauses the simulation (pause button or speed 0)? [yes *[inferred]*:
  only hidden pauses (through `SC4GamePause` and the scrubber) stop it, and nothing links the simulator's `Pause()`
  or speed to the clock. An in-game check would settle it: pause, then watch the clock display and the lighting]~~ **ANSWERED 2026-10-16 (re-answers25):** MATCHES (yes, checked in the bytes). The simulator's `Pause` (`0x007021E0`), `HiddenPause` (`0x00702200`), `SetSimSpeed` (`0x007026D0`), `DoSetSimSpeed` (`0x00702430`) and `EmergencyPause` (`0x00702540`) never reach the clock. A scan for calls through the clock's `HiddenPause` (+0x68) finds only `SC4GamePause` (city slot 86, `0x005F92EA`) and the scrubber (global `0x00B43DBC`, `0x005BD0D6`, `0x005BD39B`); the clock's `Pause` (+0x60) is reached only by the cheat. The 3D view's sim-pause message handler (around `0x007F5714`) only updates the UI, and `DoTemporarySimHiddenPause` (`0x007E5B40`) pauses the simulator. clock24 §1, §3.1, test 20. In-game check if wanted: press pause, then watch `Update24HourClock` (`0x007EC0B0`) or the clock's seconds at +0x2C keep advancing.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK2:** Is the clock timer's unit 3 microseconds? [yes *[inferred]* from the `1e-6` factor, which gives a
  640-second day at magnification 135. The unit factors from `0x00B54360` onwards are calibrated at runtime.
  Reading that table in a running game, or timing one game day, would settle it]~~ **ANSWERED 2026-10-16 (re-answers25):** MATCHES: unit 3 is exactly microseconds of `QueryPerformanceCounter` time; the table is **not** calibrated, the bracket's wording **DIFFERS**. A static initializer (`0x00A78F7F` → `0x0088FF4E`) calls `0x0088FDC0(&0x00B13420, &0x00B13428)` once at process start; `0x00B13420` is always `QueryPerformanceFrequency` (`0x0088FED4`), and the unit factors `0x00B54360`–`0x00B54370` are built from it (`0x0088FFB9`–`0x0089000D`; µs = (1/F) × 10^6 at `0x00B54364`). Only `0x00B13428` (the CPU cycle rate, measured with `SleepEx` loops when rdtsc and QPC disagree by 2 % or more) and the call overhead (`0x00B54358`, the smallest of 8 back-to-back QPC differences) are measured; the clock uses only the overhead. The clock's constructor pushes unit 3 (`0x0044596F`); units 1-6 read QPC (`0x00890198`). A day at magnification 135 is 640 s plus the truncation loss. clock24 §4.3.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK3:** What is the object at `+0x8C` that `Shutdown` hands the string at `+0x78` to? [dead: nothing in the clock
  sets it, so FreeCity4 omits both fields. A write breakpoint on `+0x8C` would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dead; clock24.md §2). The clock's only writes to `+0x8C` are the constructor's 0 (`0x00445A15`) and `Shutdown`'s clear (`0x0044516B`), and no accessor hands it out.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK4:** A saved `pauseCount > 0` leaves the clock stopped after the load, and only `Stopwatch resume` restarts it.
  [reproduced; the corpus has no such save]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe; the corpus has no such save, so there is nothing more to settle).

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK5:** Each step loses the fraction of a game second left after truncation, so the day is up to about 1.6 %
  longer than 86,400 / magnification real seconds, depending on the idle cadence. [FreeCity4 reproduces the formula. The
  visible day length follows FreeCity4's own idle cadence, which conformance does not test]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (formula reproduced; the visible day length is FreeCity4's own cadence and outside conformance).

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CK6:** The SDK header declares `GetTimePercentage` and `GetVisualTimePercentage` as `float (void)`, but the game
  takes an out pointer and pops 4 bytes. `GetEventHour` takes a dword index, not a `bool`. [the game's form is
  binding (clock24.md §8), and docs/abi should note it. Plugins built on the SDK declaration would corrupt the
  stack in the original too]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). Re-checked: `GetTimePercentage` and `GetVisualTimePercentage` end with `ret 4` (`0x0044543E`, `0x0044546E`), and `GetEventHour` tests the whole dword (`0x00445024`, `0x0044502E`). The docs/abi note is the coordinator's to add.

## From the history warehouse and 24-hour clock cores (filed by the coordinator from the `cores10a` lane's report, 2026-10-14)

cores10a implemented both cores (merged `0757437`, ABI 46): every §12 test of history.md and clock24.md is a unit
test and a conformance vector (`hw12_*`, `ck12_*`). The 421-save corpus checks (history §7.1, clock §12.12) are
left for the coordinator or a glue lane, since engine lanes cannot read the saves.

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK7:** clock §4.1: the percentage factor's bit pattern `0x37C22E45` is 1/43200 (1.0 at noon), but the decimal
  value and §12.1 say 1/86400. [`0x37422E45`, 1/86400; a test pins it]~~ **ANSWERED 2026-10-14 (re-answers10):** the exe's constant (`0x00A86508`) is `0x37422E45` = f32(1/86400); the spec's `0x37C22E45` was a typo. `GetTimePercentage` at noon writes exactly 0.5 (`0x3F000000`); the product is rounded once to float. The bracket MATCHES (clock24 §4.1, test 14).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK8:** the clock step's signedness. [u32: a negative product wraps, then the 1200 clamp applies]~~ **ANSWERED 2026-10-14 (re-answers10):** the step is the low 32 bits of `_ftol`'s int64 truncation toward zero, compared unsigned with 60 and 1200: a product of −1 or less wraps and is clamped to 1200; between −1 and 0, or NaN, gives 0 (no step). The bracket MATCHES (clock24 §4.3, test 15).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK9:** an elapsed time of 2^31 or more. [the float(uint32) idiom]~~ **ANSWERED 2026-10-14 (re-answers10):** the unsigned-to-float idiom (exact load, plus 2^32 when the top bit is set); below 2^31 the product uses the exact `e`, rounded once. The bracket MATCHES (clock24 §4.3).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK10:** `atol` overflow (`Stopwatch magnify`). [wraps]~~ **ANSWERED 2026-10-14 (re-answers10):** the CRT `atol` (`0x009EF7C2`) skips leading white space, takes one sign and accumulates in wrapping 32-bit arithmetic. The bracket MATCHES. Also: `WhatTimeIzIt -1` gives hour 15, and `magnify` converts the signed result (clock24 §4.8, test 18).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK11:** how cheat text is split. [on blanks; double quotes group and are removed]~~ **ANSWERED 2026-10-14 (re-answers10):** the separators are exactly space, tab and newline, and runs count as one. A separator splits only when an even number of quotes has been seen in the current argument, and only an argument that both starts and ends with a quote loses that outer pair. The bracket MATCHES for ordinary text but **DIFFERS** if FreeCity4 removes inner quotes or splits on other white space (clock24 §4.8).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK12:** the cheat registration order. [register, then subscribe, one cheat at a time]~~ **ANSWERED 2026-10-14 (re-answers10):** register `WhatTimeIzIt`, subscribe it, then register `Stopwatch`, subscribe it. The bracket MATCHES (clock24 §4.2).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK13:** `Shutdown` without a cheat manager. [it unregisters only when a manager exists]~~ **ANSWERED 2026-10-14 (re-answers10):** without a cheat manager nothing is unregistered; with one, each cheat in turn has its notification removed, then is unregistered. The bracket MATCHES (clock24 §4.2).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK14:** a saved pause count of −1. [signed i32 counters: Resume does nothing, `IsPaused` is false, so the clock
  runs]~~ **ANSWERED 2026-10-14 (re-answers10):** the counters are signed, `Pause(false)` does nothing at −1 and `IsPaused` is false, but `Init` starts the timer only when both counters are **exactly** 0. So the timer never starts, every step is 0 and the clock stays frozen until `Stopwatch pause` brings the count to 0 and a hidden pause cycle (or pause and resume) starts the timer. The bracket **DIFFERS** ("the clock runs") (clock24 §4.7, test 16).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**CK15:** timer `Restart` on a stopped timer. [starts it]~~ **ANSWERED 2026-10-14 (re-answers10):** `Restart` sets the elapsed time to 0 and the start tick to now, whether or not the timer ran, so a stopped timer starts. The bracket MATCHES. Also: `Start` does not test whether the timer runs (clock24 §4.3 timer model).

## From the history warehouse and 24-hour clock glue (filed by the coordinator from the `histclock` lane's report, 2026-10-15)

About `docs/specs/history.md` and `clock24.md` and the replacements `cFC4HistoryWarehouse` (`enable = history`) and
`cFC4Clock24` (`enable = clock24`) with their shadows (`history_shadow = 1`, `clock24_shadow = 1`); merged as
`efac89a`, ABI 52. The lane numbered these history 1, 7, 8, 11, 12 and clock24 2-6, 9, 10. The current answer is in
brackets.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK16:** `cIGZSystemService::OnTick`: the value the clock's system service returns is not in clock24.md. [true, the
  SDK default]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). The sub-object's slot 10 is the shared default `0x0091559A`: AL = 1, pops 4.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK17:** `GetServiceTickPriority` is not specified. [the service priority, -2,000,000]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §8). Slot 12 (`0x009D4714`) returns the tick priority, which the base constructor (`0x0091DE0F`) sets to the same −2,000,000 as the priority (`0x0091DE34`, `0x0091DE37`).

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK18:** does the system-service base's `QueryInterface` answer any IID besides `0x287FB697` (for example IID 1
  through the +0xC sub-object)? [only `0x287FB697` through the base; the object's QI table answers the other four
  IIDs]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES, with one addition (clock24.md §8). Slot 0 of the `+0x0C` sub-object is a thunk to the clock's own `QueryInterface` (`0x00445A80` → `0x00444E40`), so every pointer answers the same five IIDs: the four of §1, and `0x287FB697` through the base (`0x0091DD67`). IID 1 gives the primary pointer, and any other IID returns false with `*out` unwritten (`0x009457DC`).

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK19:** slots 5, 8 and 11 have no meaningful return value in the spec. [return 1]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice; clock24.md §8). The original leaves EAX as a by-product (slot 8 the day quotient at `0x0044549B`, slots 5 and 11 the last call's result), so returning 1 is safe.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK20:** if a tuning property (`0x897E709F`, `0x09B73421`, `0x09B73422`) is stored as an array, which value is used?
  [the first value]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (clock24.md §6). All three are read with the converting getters (Float32 `0x005FD4F0`, Uint32 `0x005FD480`), so an array gives its first value. An array of count 0 gives 0, and a float hour is truncated (data-formats §5.1).

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK21:** the original's timer at +0x50 (a `cRZTimer`) has no read path or layout in the spec, so the clock shadow
  cannot replay real elapsed time. [the shadow copies each of the original's steps through a synthetic elapsed time at
  magnification 1 (a game-time jump of 61 to 1200 s between idle passes is one step); any other game-time change is a
  jump (`SetHour`, `SetMinute`, `SetSeconds`); messages and derived values are compared]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES, and the layout is now specified (clock24.md §4.3 timer layout). `cRZTimer`: `+0` unit, `+8` start tick (u64, 0 = stopped), `+0x10` accumulated ticks (u64). Unit 3 is QueryPerformanceCounter ticks × 10^6 / frequency, so microseconds (`0x0088FFFF`; no longer *[inferred]*), less a calibrated per-interval overhead (`0x00B54358`). After each step the start tick at clock `+0x58` holds the step's moment, so a shadow can bound each step's real elapsed time. The value read inside the step is not stored, so the synthetic-elapsed replay stays the exact method.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**CK22:** under the turbo clock the framework idle loop is starved (about 24 `OnIdle` passes in 29 s), so the clock
  advances only about one game hour per turbo run and reaches no day/night boundary. Is that acceptable for the
  harness, or should turbo give the idle loop time? [nothing changed; normal-speed runs cover the clock (598 steps,
  323 posts in 330 s)]~~ **ANSWERED 2026-10-15 (re-answers12):** Not an RE question; the bracket stands. Exe fact for the decision (clock24.md §3.1): the clock advances only in `OnIdle`, and a pass advances it by at most 1200 game seconds (`0x00445874`). So a starved idle loop slows the clock in game time too; nothing else advances it. So 24 passes advance the clock by 8 game hours at most, and 12 game hours need at least 36 passes. Normal-speed runs covering the clock is a sound choice.
