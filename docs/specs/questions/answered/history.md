# history: answered questions (archive)
<!-- Archive for [history.md](../../history.md). Lanes do not read this file. Open questions are in ../history.md. -->

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**HW1:** The yearly thinning keeps the last point of every `Dispose` range and leaves the last group of every
  `Coalesce` range open. Is that an off-by-one in the original, or intended? [reproduced as is; Konradshohe's 296
  points match this model exactly]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The intent cannot be recovered from the code; the behaviour is as reproduced, and the 296-point corpus match confirms it. Keep it as *[compat]*.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**HW2:** `GetPoint` at an interior point returns the regression value through three points (a smoothing), and
  `GetCount` counts the widened range. Do the graph windows depend on this? [reproduced; the graph and Lua callers
  are not traced yet. A trace of the graph window's calls would settle it]~~ **ANSWERED 2026-10-16 (re-answers22):** Answered (history.md §9, §10). The graph window (`cSC4GraphsViewWinProc`) reads history only through slot 8: one `GetPoint` per series at one date (`0x0076BF69`), and for the line graph one or two `GetPoint` calls per series per sample date (`0x0076C510`, `0x0076C53A`; two keys combined by sum, difference, product or ratio, then scaled). It never calls `GetCount` (slot 5). So the graphs **do** depend on the smoothing: a sample date on an interior point is drawn at the three-point regression value. Reproduce it as is.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**HW3:** The regression sums are `double` variables, but they follow the caller's x87 precision: 24-bit from the
  simulators, 53-bit inside Lua calls (foundations §18). [FreeCity4 evaluates them per caller. A differential test
  through a Lua advisor call would confirm the 53-bit path]~~ **ANSWERED 2026-10-16 (re-answers25):** MATCHES, with the callers corrected. The regression (`0x004756C0` sums, `0x00475770` spreads, `0x00475820` correlation, `0x00475CC0` line) is reached only through slots 5, 6 and 8, whose callers are the Lua callbacks `game.trend_value` / `trend_slope` / `trend_count` (inside the guarded Lua call, 53-bit) and the graph window's `GetPoint` (`0x0076BF69`, `0x0076C510`, `0x0076C53A`; main thread, 24-bit). No simulator calls them (slots 7, 12, 13 only), so "24-bit from the simulators" should read "24-bit from the graph window". history §4.7. In-game check if wanted: breakpoint at `0x004756C0`, read the FPU control word's PC bits (bits 8-9: 0 = 24-bit, 2 = 53-bit) and the call stack (a Lua callback frame gives 2, the graph window gives 0).

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**HW4:** Version 1 records turn `maxCount = 100` into 0, duplicate dates overwrite the last point, and nothing in the
  executable posts `0x89EFA537`. Why `100 → 0`, and does any plugin post `0x89EFA537`? [reproduced; the message stays
  handled for plugins. The city getter is settled: slot 32 (`+0x80`) returns the field that `CreateCityComponents`
  fills with the warehouse (history.md §1); docs/abi still shows a folded name for it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (history.md §3). The four bytes of `0x89EFA537` occur only in the warehouse's `Init` (`0x00476726`), `Shutdown` (`0x00475900`) and `DoMessage` (`0x004770BE`), so nothing in the executable posts it. The reason for `100 → 0` is not recoverable from the code.

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW5:** history §4.7: how `r` is grouped. [the product of the two standardised values, summed, then divided by n]~~ **ANSWERED 2026-10-14 (re-answers10):** the grouping MATCHES (the product of the two standardised values, summed, then divided by n), but the standardised values are `(x − meanX) × (1/sx)` and `(value − meanY) × (1/sy)`, with each reciprocal rounded once; dividing by `sx` changes the result (history §4.7, test 12).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW6:** ranges: how they are walked, and a widened range with from > to and no point in it. [walked like a list;
  the widened case takes the point before the first one at or after `from`, and that point]~~ **ANSWERED 2026-10-14 (re-answers10):** walked like a list from the first point, with unsigned dates. With `from > to` the widened range is the **first point after `to` and its predecessor** (after the two pre-checks: the last point if `from` is after it, the first point if `to` is before it). The bracket (the point before the first at or after `from`, and that point) **DIFFERS** whenever a point lies between `to` and `from` (history §4.6, test 11).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW7:** a `Read` failure. [stops at the first failed read and returns false; series read completely stay, the
  partial one is dropped; a date not after the previous one, with no point yet, is dropped]~~ **ANSWERED 2026-10-14 (re-answers10):** `Read` tests no individual read and runs on with the values the failed reads left, so the series being read keeps the points read before the failure. A duplicate key makes `Read` return false **at once** (the rest of the record is unread); the old spec said it was only discarded. The bracket's stop-and-drop is accepted as *[choice]* for corrupt records; it **DIFFERS** from the original only in the partial series. A date ≤ prev with no point (only date 0) is dropped: MATCHES (history §7, test 14).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW8:** `Store`'s full-queue test. [unsigned: a count at or above maxCount drops one point; maxCount 0 means
  unlimited, so 0xFFFFFFFF never trims]~~ **ANSWERED 2026-10-14 (re-answers10):** the u32 count is compared unsigned with `maxCount`, and 0 means unlimited. The bracket MATCHES (history §4.1).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW9:** `GetMinMaxAverage` with from == to and no point on that date. [`GetPoint` is called twice and its value
  counted twice, as written]~~ **ANSWERED 2026-10-14 (re-answers10):** both flags are tested after the loop, so with `from = to` and no point on that date, `GetPoint` runs twice and its value counts twice. The bracket MATCHES (history §4.7, test 13).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW10:** precision values other than 24. [all treated as 53 bits]~~ **ANSWERED 2026-10-14 (re-answers10):** glue-only: the game never selects the 64-bit mode, so only 24 and 53 occur. The choice is confirmed (history §4.7).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW11:** 24-bit sums of int32 operands. [the exact sum, rounded once]~~ **ANSWERED 2026-10-14 (re-answers10):** each addition takes the exact integer operand and the stored sum and rounds once at the caller's precision. The bracket MATCHES (history §4.7).

<!-- from: From the history warehouse and 24-hour clock cores (2026-10-14) -->
- ~~**HW12:** `RecordHistoryData`'s return value. [true]~~ **ANSWERED 2026-10-14 (re-answers10):** the original returns the final `Release`'s EAX, which is 0 (false) because nothing keeps the message. No shipped caller reads it (48 call sites checked). The bracket **DIFFERS**: return false *[compat]* (history §4.1).

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**HW13:** `Release` on a warehouse that was initialised but never shut down (history §1): does `Release` at a count
  of 1 or less run `Shutdown` first? [`Shutdown` first, then delete]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** (history.md §1). The original's `Release` (`0x00476AA0`) does not call `Shutdown`: its destructor frees only the series map (`0x00476918`–`0x00476952`) and leaves the subscriptions, relying on the city's `Shutdown` call. Running `Shutdown` first is an accepted *[choice]* that changes nothing in the normal order.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**HW14:** when the history shadow sees the original's 5 ms timer split a yearly-pass step (the original answers
  false where the shadow answered true), what is the intended comparison? [the shadow skips the original's repeats
  until the original answers true and counts `split_steps`; 0 in every lane run]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (history.md §4.3). The pass's timer is a local `cRZTimer` with unit 4, milliseconds (`0x0088FEDF(4)` at `0x00475E56`; no longer *[inferred]*). It is read after every cursor advance (`0x00475F1B`), and the call stops at 5 or more (unsigned, `0x00475F27`). The split depends only on real time; the coalesced keys, their order and the final state do not. Skipping the original's repeats until it answers true, and comparing the result, is the right comparison.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**HW15:** the warehouse's raw save record cannot be read in game from outside the class store. [the load comparison
  uses the original's own `Write` into memory; the save sweep (`engine/formats/tests/save_records.rs`) checks the raw
  records of the 417 stock saves]~~ **ANSWERED 2026-10-15 (re-answers12):** MATCHES (history.md §7). No slot returns the raw record, and `Write` (`0x00475D2C`) is the only serialiser, so the original's own `Write` into a memory stream is the read path. The in-memory layout is now in §2 for a direct read.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**HW16:** queue lengths set through slot 13 (the producers' `Init`, not a message) are invisible to the history
  shadow. Should there be a read path for them? [a series that differs only by its queue length is named and counted
  as a mismatch; none occurred]~~ **ANSWERED 2026-10-15 (re-answers12):** Answered (history.md §2). No slot returns a queue length, but the shadow already has two read paths: the original's `Write` (HW15) writes each series' `maxCount`, or the tree can be read directly (key at node `+0x10`, points list `+0x14`, `maxCount` `+0x18`; `0x00476FF2`, `0x00475EC1`). Naming such a series as a mismatch until then is fine.

<!-- from: From the history warehouse and 24-hour clock glue (2026-10-15) -->
- ~~**HW17:** 403 of the 417 stock saves hold version-1 history records, which cannot be rewritten byte for byte (the
  writer always writes version 2; a queue length of 100 reads as 0; unset or repeated dates are folded). Is converting
  on the first save the intended behaviour, since the original also writes version 2? [yes: the sweep reports them as
  converted and checks that the conversion is stable]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (history.md §7). The original's `Write` always writes version 2 (`0x00475D2C`), so it converts a version-1 record on its first save too.
