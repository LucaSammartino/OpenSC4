# Manual checks backlog

Things only a human can do or decide. Newest at the top of each section. Claude keeps this updated.

## Needs you (blocking)

Nothing is blocking. When you have 10 minutes, please do the in-game checks below.

**Currently staged in Plugins** (as of 2026-10-13, ~11:00): DLL `569FF4D8…`, the **run-26** build — the one with the
panic logger and working power. Note `FreeCity4.ini` was left in lane 1's run-26 test configuration
(`enable = power`, `power_report = 1`, `observe = 1`, `autotest_seconds = 120`, `autotest_unpause = 1`, and
`autoload_city = …\City - Spandau.sc4`), **not** normal-play settings: it autoloads Spandau and auto-unpauses, so
manual play with this ini will drop you into that city on the clock. Before playing by hand, set the ini back to
`budget_default = 1` alone. Power is now verified working (run 26), but land value, weather, ordinance and demand
are still off by default. Kept alongside it: `FreeCity4.dll.pre-run23` / `81AC46F4…` (the pre-run-23 build, the last
one known good for extended play before the power work) and `FreeCity4.dll.run23-crash` / `F5439E03…` (the crashing
build, kept as evidence; **do not stage this one**).

Open the budget window and its sub-windows (departments, loans) a few times: they were the last place the allocator
bug could still bite.

One thing to know before you trust any numbers: while our DLL is loaded the game's own x87 precision is 53-bit
instead of 24-bit, so *original* subsystems differ from the real game by about ±1 simoleon per department per
month. See the x87 note below. Our own budget replacement is correct.

## Budget corruption bisection

| Run | Switches | Result |
|---|---|---|
| 8 | all on, message-layout fix | corrupt: NT heap unlink AV right after load. Cause: `GetAllGroups` pushed a vector element holding a string by bitwise copy (double free). Fixed in 8d79c77 (merged). |
| 9 | all on, with the fix | load and first month OK; crash at month end in the game's sound-event pool (exe+0x1E16F5) |
| 10 | `message_posting = 0` | same month-end crash, so messages are not the cause |
| 11 | `lot_scan = 0` | **clean** for two months: the culprit is the lot scan (`ForEachLot`) |
| 12 | `lot_list = 0` (lots walked cell by cell, all per-lot reads on), Spandau via autoload | **clean**, two settlements with real tax income |
| 13 | all on, Spandau | corrupt: the same AV, a week after the first settlement. Reproduces in a built-up city. |
| 14 | list on, all per-lot reads off | corrupt: the `GetLotsInCellRect` list path alone is enough |
| 15 | `lot_release = 0` | corrupt: releasing the lots is not the cause |
| 16 | `lot_list_free = 0` | **clean**: freeing the list is the cause |
| 17 | all on, with the fix (940b3bb, merged in 860d8cf) | **clean** through five month-ends (Sep 2001 to Feb 2002); funds and tax income change each month |
| 18 | all on, every incompatible crossing fixed (6f9dc97: budget vectors, lists and strings in the game's allocator) | **clean** through five month-ends; `GetAllGroups` exercised daily |
| 19 | original simulator and budget, observer only (641cfda) | clean; the baseline for the differential test |
| 20 | our budget, original simulator, observer | clean. **Differential vs run 19, first month** (identical start): health and education expense +1 each (123/122, 105/104); tax by type differs in types 4, 6 and 9 (total income 947 vs 944). Investigated (f59e085): **ours is right**. The real game gives 123/105, as the year-to-date records of the unmodified save show, because it runs x87 maths at 24-bit precision. Run 19 ran at 53/64-bit, so something in the test process reset the precision. That may be our DLL, which would make every original subsystem compute wrongly whenever FreeCity4 is loaded. Being instrumented by the plugin lane. The tax-by-type difference is still open (agent order, precision, or the lot set); a per-lot dump is requested. **[Resolved 2026-10-13: it IS our DLL — see row 24 and foundations §18.2 (ea602a4, e647189). The precision conclusion in this row stands, and ours is confirmed right.]** |
| 21 | `enable = power` (our simulator and budget) | **crash** 0.5 s after load, inside our DLL: a line-item handle of `0x3F800000` (the float 1.0 used as a pointer) reached `SetFullExpenses`. Sent to the plugin lane. Power stays off by default, so manual play is unaffected. |
| 22 | power fix | **clean**, but power production was 0 (no `power recompute` with non-zero output). |
| 23 | `budget_default=1, power_report=1, observe=1`, DLL F5439E03 (2c77349 + be7bb41 + 6aa6735, merged ef4159c) | **crash** `0xC0000409` ~0.3 s after PostCityInit, no `power recompute` line ever written. Last log lines were `IsLocallyFundedObjectInDepartment` / `GetLocalFundingPercent`. Diagnosis: a Rust panic in the power core or capi, prime suspect the "Read keeps the saved words, Init sizes the bitmaps from the city" change. **This run also produced the first x87 control-word measurements** — 24-bit at DLL attach and through PreFrameWorkInit/PreAppInit, 53-bit (`CW=0x027F`) from PostAppInit-enter onward. |
| 24 | same switches as 23, DLL with the panic-hook work (checkpointed 8129d67), lane 1 | **clean** — `AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0)`, no panic, 4378 log lines, budget settled (funds 69986 → 70154, income 964, expense 796, tax 919). Still **no `power recompute` line**, because power was off by default in this run's config (`subsystem 'power': off by default`), so power production is still unverified. The x87 warning fired again: 53-bit from PostAppInit through tick 1, city load, month observation and both budget settlements. |
| 25 | lane 1 control-word investigation build | **clean**. Same 24-bit → 53-bit transition at PostAppInit-enter. |
| 26 | `budget_default=1, enable = power, power_report=1, observe=1`, DLL `569FF4D8…` | **crash** `0xC0000409`, no `AUTOTEST VERDICT` (the line that looked like one is a periodic heartbeat). Power *did* work up to the crash — 14 `power recompute` lines, first reading `production 5997 consumption 1038 demand 1038 powered 385 unpowered 0`, closing run 22's "production 0" gap — but those numbers were written **before** the death. The log ends with two threads interleaved (`tid 23860` in `BudgetSimulator::GetFundingOf`/`GetBudgetItemForPurpose`, `tid 8328` in `GetAllGroups`), which is the cross-thread race found and fixed in run 27. *(This row was originally recorded as clean by the coordinator in `c7d1e51`; that was wrong and is corrected here.)* |
| 27 | thread-fix build (`RefCell` → `Mutex` in the master clock), budget + power + observe | **clean** — `AUTOTEST VERDICT: clean`, exit 0, no panic. 41 `power recompute` lines, `production 5997` → `6000`, 409 cells powered, **0 unpowered**, no shortfall. |
| 28 | same source as 27, rebuilt; the exact staged DLL `F921001D…` (hash verified in its `DLL_PROCESS_ATTACH` line) | **clean** — `AUTOTEST VERDICT: clean`, exit 0, no panic. Same power result. Confirms committed source, staged DLL and the validated run correspond. |
| 29 | `disable = simulator,budget`, observe, with the 24-bit re-assert | **clean**. The game computes: `dept_expense` 123/105 in all five months, matching the unmodified save. The first trustworthy original baseline. |
| 30, 31 | lane 1 thread and final builds | **clean** (`build-runs/run30-threads.log`, `run31-final.log`). |
| 32 | first m3-glue attempt | **no information**: the ini was written from bash with a literal backtick-n, so `autoload_city` never parsed (HANDOFF §8). |
| 33 | main `1eb5dd6` (DLL `33FC0F9B…` with its own PDB), `enable = plumbing, flammability`, Spandau | **clean**. Plumbing passes run (23 → 101), building cells 396 → 412, no re-entry. But Spandau has **no water system** (`producers=0 pipes=0`), so only the empty-network path ran, and flammability logged nothing after `Init`. |
| 34 | same DLL, Konradshohe (Berlin) | Autoload refused the city, which is in another region, and the city was then loaded by hand and the game quit normally. That gave the **first full `Shutdown` path**: `flammability Shutdown (189 cycles)`, `plumbing Shutdown (7 passes)`, exit 0. With water: 2 producers, 403 pipes, watered cells 729 → 702 always equal to building cells, consumption equal to demand. The 16 NEGATIVE refaudit class lines at simulator Shutdown are a window artifact (the budget Shutdown summary reset the counters just before); per-object tracking showed 0 over-releases. |
| 35 | flammability-counter build `5A295D1C…`, `disable = simulator,budget`, Spandau | **clean**. The original budget again: the run-to-run noise baseline for 36. |
| 36 | same DLL, `disable = simulator` (our budget, original clock), Spandau | **clean**. The budget differential (one variable against 35). See the analysis below the table. |
| 37 | same DLL, `enable = plumbing, flammability`, FC4 Test's Konradshohe, 240 s | **clean verdict but `ref_warnings=4`: a real use-after-free.** Flammability works: 4 fire rolls and 4 end-of-cycle passes, roughly every 40 days, and 22,824 `IsCellWatered` queries into plumbing (7,258 watered, 0 "no plumbing"), so the cross-subsystem read is live. On the first real ignition (`ignited=1/1`, 2026-04-18), `IgniteCell`, `Drop` and `SetFireCause` released references to the game's fire object that we never acquired (2 over-releases, 2 last-reference releases), so the object is freed while the game still owns it. Also `reentrant` went 0 → 1 after the fire, and the flammability `average` read 0 for the first two months after load. **Fixed in `95db886`**: the plugin was releasing the *borrowed* disaster manager in both `IgniteCell` and `SetFireCause` (spec §6.8 releases only the QueryInterface'd fire interface). The average of 0 is spec behaviour (§14.1 quirk 1: the cycle position survives `Init`, so the first end-of-cycle after a load is empty). |
| 38 | fire-fix build `69D9CA30…`, same config as 37, 420 s | **The fix holds.** A real ignition (`ignited=1/1`) with **0 over-release and 0 last-reference warnings** in the whole log. `nested=1/0/0/0` shows that `reentrant=1` is the fire manager reading our flammability grid once during ignition, which is benign. The game was closed by hand before the verdict, so this run also went through a full `Shutdown` after the fire (`flammability Shutdown (100 cycles)`), still with no warnings. |
| 39 | shadow build `D69F64FF…` (merged as `6ca4b65`), `disable = budget` (our clock, the original budget), `budget_shadow = 1`, Spandau | **clean, and the budget matches the original exactly.** At each of five month-ends, our tax scan ran on the same live lots (413–417) just before the original settled, in the same process. All 12 tax types, total tax, income, expense and net were **identical every month** (`mismatches=0`, `months=5 clean=5`). On 2002-01-01 income/expense can't be observed because the year-to-date resets (QUESTIONS BQ2), but tax and net still matched. This was also the first in-game run of our clock driving the original budget. |
| 40 | main `6ca4b65` build, the same shadow config, FC4 Test's Konradshohe, speed 1 | **clean, 2 of 2 months matched** (740 and 729 lots). The rest of the run was lost to an advisor dialog holding the city at 2026-03-03 (HANDOFF §8). |
| 41 | same, `autotest_unpause = 2`, 840 s | **clean, and 35 of 35 month-ends matched exactly** (2026-02 → 2028-12, including two January 1 settlements, 681–740 lots). No mismatch or attribution line was written in the whole run. An advisor dialog held the city for ~7.5 min until it was dismissed by hand. |
| 42 | demand-shadow build `AC7D9017…` (+ the coordinator rustfmt commit), `demand_shadow = 1`, our simulator, Spandau | **clean; demand matches bit-for-bit.** All 160 NewDay updates had `mismatches=0`: 173 nodes, 12 active values and the neutral tax rate identical bitwise. 6 of 6 SimIdle modifier checks were exact. The graph built in 141 ms (173 exemplars, 353 links). `ref_warnings=1`: the per-call resource key list we own was released, a false positive (see 43). |
| 43 | the refaudit fix `A27E5D41…` (merged as `a07aa6a`), same config | **clean, `ref_warnings=0`**, and 160 of 160 NewDay updates bit-exact again. **One finding:** on 2001-10-01, 11 of 12 *tax modifiers* differ by **4 ulp** (orig `0.0832541287`, ours `0.0832540989`; I-R matches), on the day the census supply changed. The other 5 modifier checks are exact, and run 42 had that same day exact. Under investigation by the engine lane: a different input or sampling moment is suspected, not the formula. **Resolved by run 44.** |
| 44 | demand fix `5904E092…` (merged as `42f7649`), same config as 43 | **clean, and demand is bit-identical everywhere:** 160 of 160 NewDay updates and **6 of 6** modifier checks, including 2001-10-01 (orig/ours `0.0832541287`, at the same neutral `8.99755573` that run 43 had 4 ulp off). Cause and fix: the exe (`0x006A6070`) multiplies `r24(maxRate·0.1f)` by a float32 reciprocal of neutral computed once per update, rather than dividing (demand.md §8.1, DQ1). |
| 45 | main `42f7649` (`EC1811B2…`), **both shadows** (`disable = budget`, `budget_shadow = 1`, `demand_shadow = 1`), Konradshohe, speed 2, 840 s | **clean, every comparison exact:** budget 4 of 4 month-ends, demand 96 of 96 days (2026-01-28 → 2026-05-03), modifiers 5 of 5, `ref_warnings=0`. Short because an advisor dialog held the city at 2026-05-03 for the rest of the run (the same spot as run 41). |
| 47 | main `42f7649` (`EC1811B2…`), `budget_default = 1`, Konradshohe, 90 s (third session's first independence check) | **clean.** Launch, autoload, verdict and kill all worked unattended. |
| 48 | same, 300 s, watched on screen | **clean.** At 2026-05-03 an advisor popup ("Educators Take Long Recess") held the city, then a second queued behind it. Each popup calls `HiddenPause()` on our simulator (`hidden` 1, then 2). Esc and Enter did not close them; clicking Close did, and the date moved on. The `hidden` heartbeat field *is* the popup signal (HANDOFF §8 said otherwise). |
| 49 | same, 330 s, harness clicks Close (posted messages, no focus) | **clean, 3 popups dismissed automatically** with no screen control. Also showed llama = `SetSimSpeed(1)`, cheetah = `SetSimSpeed(2)`. |
| 50 | same, interactive (`autotest_unpause = 0`, `-NoDismiss`), driven with `tools/sc4ctl.ps1` | **M1 checks pass with our simulator and budget:** turtle/llama/cheetah call `SetSimSpeed(0/1/2)`; the pause button calls `Pause()`, which holds the date (2026-11-16 for over 21 s), and a second click calls `Resume()`. **Save:** real-input Ctrl+S saved (posted keys do not); the save hidden-pauses 4×, writes our simulator (79 bytes), the budget, 18 departments and the line items, then shows a "City Saved" box (OK at client 465,286). The reward-offer popup ("Fight Fires From Above") is taller, so Close moved to y 502. |
| 51 | same DLL, reload of run 50's save | **clean; save → reload verified.** `Read: version 4, 79 bytes, julian 2461377` (the saved 2026-12-02) and `budget Read -> 1 (funds 2577, departments 18, bonds 0)`: the funds after the December settlement at save time (2131 → 2577). Konradshohe then restored from its backup (`864E0A88…`). |
| 52–53 | same, scripted with `ingame_run -Actions` | **clean.** The whole check as a script: speeds, pause, `save` (confirmation seen and closed, file written; the harness restored the city afterwards). Run 53 found that a click right after closing a popup is lost, and that picking a speed while paused calls `Resume()`. |
| 54 | same, interactive: God mode → Disasters → fire on a building | **Emergency pause verified:** `Pause()` (pause 1) → `EmergencyPause()` (pause 2, emergency 1, red border and warning icon) → `EmergencyResume()` ~11 s later (pause 1, emergency 0) → `Resume()` (0), and the date moved on. No fire took hold, so this checks the pause calls, not a burning city. The 3D view ignores posted clicks: choosing the fire and clicking the city needed real input (`sc4ctl realclick`). |
| 55 | main `bbe9181` (host grids released with `FC4_RELEASE_CREATED`), `enable = plumbing, flammability`, Konradshohe, 300 s | **clean, `ref_warnings=0`.** Plumbing on the real network (2 producers, 403 pipes, watered = building cells = 719) and the flammability cycle (period 40). The harness closed 3 advisor popups (4 clicks). The autotest ends with TerminateProcess, so the grid `Release` path itself was not reached (see run 59). |
| 56 | `0e1ae06` + `popup_dialogs = 0`, `clock_probe = 1`, our clock, 240 s | **clean, and no popup at all:** `popup dialogs: disabled (… was enabled=1, now=0) at PostAppInit`, 0 dialog rounds (run 55 needed 4 clicks in the same window). The probe logged 282 agent and 281 posted calendar entries over 240 days. |
| 57 | same, `disable = simulator` (the original clock), 240 s | **clean. M1 clock differential: identical.** `tools/clock_diff.ps1 -A run57 -B run56`: the agent stream (282 entries) and the posted stream (281) match entry for entry over 240 days, 2026-01-28 to 09-24, and both follow the spec's calendar rules (consecutive days, week on Mondays with counter (doy−1)/7+1, month on the 1st, day counter = day of year). The load sequence matches too: speed 0 before the load, then hidden 0 and speed 1. |
| 58 | `clock_probe = 2` (week agent held once), our clock, plumbing + flammability; posted `WM_CLOSE` at 152 s | **clean.** Mode 2 shows the §5.3 repeat: every week message is delivered twice (`… again`). The posted `WM_CLOSE` did nothing (no dialog, the city kept running; with `popup_dialogs = 0` a quit prompt would answer by itself), so scripted quitting moved into the plugin: `autotest_end = quit`. |
| 59 | `autotest_end = quit` (new), `clock_probe = 2`, our clock, plumbing + flammability, 150 s | **clean, `ref_warnings=0` through the whole Shutdown path.** At the deadline `cISC4App::RequestQuit(false, false) -> 1`; plumbing and flammability shut down and released their host grids (the `FC4_RELEASE_CREATED` path of `bbe9181`), the budget and simulator shut down, then PreAppShutdown → PostAppShutdown → PostSystemServiceShutdown (verdict) → process exit, all in 0.3 s. The verdict's julian read 2000-01-01 (our simulator publishes its reset date during Shutdown); fixed to the last day played. |
| 60 | same, `disable = simulator` (the original clock), 240 s | **clean; the mode-2 clock differential is identical too:** 316 agent entries (each week delivered twice, the §5.3 repeat) and 282 posted entries over 240 days match run 58 entry for entry. The quit ending works with the original simulator as well, and both simulators post the same hidden-pause sequence while quitting (1, 0, 1; runs 59/60). |
| 61 | main `22769f3`, soak: `enable = landvalue, weather, ordinance, demand, power, plumbing, flammability`, `popup_dialogs = 0`, `observe = 1`, `autotest_end = quit`, 900 s at speed 2 (2026 → 2074, about 48 game years) | **clean, `ref_warnings=0`, 0 foreign exceptions, no stalls, quit path clean.** R population 15,025 → 28,283, funds 834 → 409,959. Two findings: power and plumbing stayed `enabled=0` all game (ConnectionsReady never arrived; runs 62–66), and demand stayed frozen (the demand glue cannot read the BuildingProfile capacity lists or fill `GetLocalPopulationSummary`, which need the STLport map layout; RE ABI lane). |
| 62 | bisection: `enable = plumbing, landvalue, weather`, `popup_dialogs = 0`, quit, 45 s | clean; plumbing `enabled=0` all 30 months. |
| 63 | bisection: `enable = plumbing, ordinance, demand`, same | clean; plumbing `enabled=0`. None of the soak's other replacements is the cause. |
| 64 | plumbing alone, `popup_dialogs = 0` (set at PostAppInit, as since `5e39f3c`) | clean; plumbing `enabled=0` all 29 months. |
| 65 | plumbing alone, no `popup_dialogs` | clean; plumbing `enabled=1` all 16 months (the harness closed 5 advisor dialogs). **`popup_dialogs = 0` was the cause:** with popups off the terrain edge reconciliation returns before posting, so the regional city never posts ConnectionsReady (foundations.md §11.6, CR1). Every run since 58 that used it had power and plumbing off. |
| 66 | `d161d20` (the flag is set false only when ConnectionsReady arrives), plumbing, `popup_dialogs = 0`, quit, 60 s | **clean; both at once:** `popup dialogs: disabled (… was enabled=1, now=0) at ConnectionsReady` 8 s after start-up, no dialog rounds, and plumbing `enabled=1` all 38 months. |
| 67 | main `d743db2` (pollution glue merged), `enable = pollution`, `popup_dialogs = 0`, quit, 120 s | **clean, `ref_warnings=0`.** `pollution Read -> 1`, Init on the loaded record (avg air/water/garbage 731/872/2848, 12 landfill cells), then 73 month lines over 2,197 days and `pollution Shutdown (2197 days, 73 months)` on the quit path. Garbage was collected (uncollected 2,088,782 → 29,704) and the landfill filled (capacity 190,185 → 3,981). |
| 68 | same DLL, `pollution_shadow = 1` (the original pollution simulator, our core replayed on its record each day and month), 150 s | **clean; all eight grids identical to the original, cell by cell, for 94 months** (2026-02 → 2033-11, 0 mismatches in 1.4 M cells a month), with the end-of-cycle replay in the original's tract order. Every saved field matches except `dirtyWater`: on 104 days the original's is true after the day and ours false, with identical grids (PG14, sent to the RE lane). Not compared: radiation sum, posts, history, budget values, effects. |
| 69 | main `36ababe`, soak 2: `enable = landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution`, `popup_dialogs = 0`, `observe = 1`, quit, 900 s | **no verdict: the game ended the city itself** (2057-10, 10.5 min in; funds 891 → −103,047). 0 foreign exceptions, `ref_warnings=0`, every Shutdown line. Power and plumbing `enabled=1` all 381 months. The city grew until 2051 (R 15,025 → 30,884) and collapsed from 2054-03 (R 25,066 → 10,783 in one quarter, 866 by 2055-07, industry 0). By then both utilities were short: power production had fallen 7,110 → ~5,800 (aging plants, no player to replace them) against a requirement of 11,872, with 195–258 of 568 consumers unpowered; water pipe condition was 0.000 and production (19–37k) short of demand (43.5k), with 0–492 of 572 building cells watered. (Consumption = 110 % of production is the spec's shortfall rule, power.md §6 step 4, and `deficit=0.000` its quirk.) Run 61 kept growing only because power and plumbing were disabled there. Whether the original simulators decline the same way is open (runs 71/72, power shadow lane). Two follow-ups: the autotest now gives verdict `ended` when the game shuts the city down by itself (main, after this run), and a soak with the original power simulator for comparison. |
| 70 | main `941c34f` (crime and protection merged), `tools/regress.ps1 -Tag r941c`: run *shadows* (budget, demand, pollution, protection on the originals) and run *replace* (`enable = landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection`), 150 s each | **PASS: both clean, 0 problems.** Shadows: demand 3034/3034 lines clean; **protection 194/194, the first in-game protection run, fire and police grids identical cell by cell for 97 months**; budget 96/97 (one month's total 4673/4674 with every type equal, the documented float-sum tolerance); pollution 97/225, all from PG14 `dirtyWater` (fix in a lane). Replace: protection Read/Init/month/Shutdown lines, **but fire shows `tracts=0x0 tractsPerDay=0` from Read on, so its daily cycle never runs** (police 64×64). Its grid ref exists but is still 0×0 at Read; fix in a lane. Power ends with 117 unpowered consumers (consumption 7,665 = 110 % of production 6,969, the shortfall rule). |
| 70b | same DLL, `regress -Tag r941c-crime` (crime added: `crime_shadow = 1` in *shadows*, `crime` in *replace*) | **PASS.** **Crime shadow 96/96 clean** (state 4 on 64×64 tracts: 0 mismatched cells and an equal hotspot every month; not compared: the attempt count, police stops and type draws, which are the original's random draws). Protection 192/192, demand 3017/3017, budget 96/96, pollution 96/203 (PG14). Crime replacement: Init on the loaded record (14 types, state 6), 97 month lines (6–12 crimes a month, police stops in 17 of the first 46 months), `crime Shutdown` on the quit path, 0 problems. |
| 71 | main `ae82e65` (demand capacity fix, verdict `ended`), soak: run 69's eight replacements (our power), `observe = 1`, quit, 900 s | **`ended`, 0 problems** (`ref_warnings=0`): the game ended the city itself in 2060-08 (julian 2473795) at funds about −100k. The new verdict and the quit after it worked (`PreCityShutdown` WARNING, `RequestQuit`, verdict at `PostSystemServiceShutdown`). The harness missed the verdict because of a read race, fixed in `e5fd34a`. |
| 72 | same DLL and soak with the **original power simulator** (`power` not enabled) | **`ended`, 0 problems, at the same point: 2060-12, funds −98,495 (ours −100,215).** Side by side (`tools/compare_soaks.ps1`): power production falls the same way from aging plants (7,110 → 4,208 original vs 4,235 ours), the requirement climbs alike (12,065 vs 11,688), and unpowered consumers rise alike (394 vs 282 at the end); funds and R/C/I stay within run-to-run noise. **So our power simulator tracks the original, and run 69's collapse is the game's own outcome for an unattended city that outgrows its plants.** |
| 73 | main `4619a6f` (protection fix, PG14, TN answers), `tools/regress.ps1 -Tag r4619` | **PASS, 0 problems.** **Fire fixed:** `protection month (fire): tracts=64x64 coveredCells=837 tractsPerDay=81`, cycles running. **Pollution shadow 94/94 clean** (PG14 applied: no `dirtyWater` difference left), protection 188/188, crime 94/94, budget 93/94 (the documented float-sum tolerance). Demand 2927/2955: **one new 1-ulp difference in the neutral tax rate** (8.97309399 vs 8.97309303) on the 27 days of one census month, R/C/I supply 27207 / 2569.48 / 10583.1, fractional now that capacity is read (DQ9, sent to the RE lane). |
| 74 | main `4619a6f` (DLL `ccdc3051`), soak 3 on Konradshohe: **all ten replacements** (`enable = landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime`), `popup_dialogs = 0`, `observe = 1`, quit, 900 s | **`corrupt` 15 s after the load (2026-09-26, 8 months).** One first-chance access violation in the game: `SimCity 4.exe+0x1B9FEB`, writing address 0. It is the game's pooled allocator (`0x5B9FC0`, used by the global `operator new` `0x5E55E0`): a block over 4 KiB goes to `malloc`, which returned NULL, and the header write faults. No FreeCity4 frame in the top 1 KiB of the stack; the last calls were the budget getters of the month's UI refresh. Run 76 shows memory was not short, so the game asked for a garbage size. Under investigation (run 76 and later). |
| 75 | same DLL and soak, **all original** (`disable = simulator, budget`) | **clean, 460 months** (2026 → 2064), 0 problems. |
| 75b | main `7c3f3a2` (aura merged, DLL `b0204260`), `tools/regress.ps1 -Tag r7c3f`: the first in-game aura run | **PASS, 0 problems.** Replace (all ten plus `aura`): clean 150 s. Shadows: protection 194/194, budget 97/97, pollution 97/97, crime 97/97, demand 3033/3033. **Aura shadow 6/98 exact:** every cycle 0 to 43 tracts of 1024 differ by 1 to 3, the mayor rating by 1 in 11 cycles; park and landmark maps differ in cycles 2 and 3 only (193 and 208 tracts); three cycles have one large base difference (about 180). Filed as AU13 to AU15. |
| 76 | main `f518e37` (the new report: registers, executable frames; heartbeat memory; DLL `bbcc67ea`), run 74 repeated: all ten replacements on Konradshohe, 300 s | **clean** (2026 → 2041), 0 problems; it passed 2026-09-26 without a fault. Memory flat: address space 715–722 of 2047 MiB used, largest free block 955 MiB, private 273 MiB. **So run 74's NULL from `malloc` was a garbage size (over 955 MiB), not a shortage.** |
| 77 | same DLL, run 74 as a full 900 s soak | **`ended`, 0 problems** (no foreign exception): the game ended the city in 2062-06 at funds −100,263, as runs 71/72 did in 2060. Memory 731 MiB at the end. Run 74's crash has not come back in 2 repeats. (Against run 75 our income looked 5–10% lower from 2041; run 78 shows that was run-to-run variance.) |
| 78 | same DLL and soak, **all original** (`disable = simulator, budget`) | **`ended` at the same julian as run 77 (2474342, 2062-06-01), funds −100,880 (ours −100,263).** Side by side (`tools/compare_soaks.ps1 -A run77 -B run78`): **all ten replacements track the original for 36 game years**: funds 87,350 / 76,525 in 2041 and 29,723 / 20,817 in 2056, income within a few percent, population R/C/I and power requirement, production and powered counts alike, the same end. Run 75 surviving to 2064 was variance. |
| 79 | same DLL and soak, all ten replacements again (third repeat of run 74) | **`ended` 2061-09 (julian 2474069), funds −100,774, 0 problems, no foreign exception.** Nine months before run 78; the same shape. In 2055–2056 the residential population swings month to month (23,457 → 9,183 → 19,528) while the aging plants make 5,450 of a 12,670 requirement: the shortfall noise both builds show, larger here by chance. **Run 74's fault: 0 in 3 repeats (runs 76, 77, 79).** |
| 80 | main `290093b` (flora `f8ebce0`, landfill `c2ada06`, power shadow; DLL `80713991`), `tools/regress.ps1 -Tag r2900` (Konradshohe): **first in-game run of the flora and landfill replacements and of the flora, landfill and power shadows** | **PASS, 0 problems.** Replace (thirteen replacements: the ten, aura, flora, landfill): clean 150 s; `flora month` from 2,606 trees to 2,569 (2026 → 2034, refused 0, createFailed 0); `landfill month` fills 360,812 → 466,497 of 479,997, fallbacks 0. Shadows: **landfill 94/94 exact**; **power: all 213 recompute cycles and 94 month lines exact, cell by cell**; its 94 bookkeeping lines (the original's running counts against a from-scratch rebuild) differ by about 3 consumers and 4 consumption-map cells every month (PW8); **flora registry identical** (22 types, 0 differences), monthly about 2,600 trees match and one differs in 23 of 94 months, mostly a birth date after a predicted rebirth (FQ15); protection 188/188, pollution, budget and crime 94/94, demand 2957/2957, aura 5/95 (AU13–AU15). |
| 81 | main `571a47e` (plumbing shadow; DLL `e6881dc1`), `tools/regress.ps1 -Tag r571a` (Konradshohe): **first in-game run of the plumbing shadow** | **PASS, 0 problems.** Replace (thirteen replacements): clean 150 s (2026-02 → 2034-01). Shadows: **plumbing 186/191**: every replayed pass has the grid identical cell by cell (16,384 cells), producers, connections, produced/consumed, watered cells, pipes and maintenance equal; 95/95 month lines exact (distressed and bursts); 4 passes differ only in `demand` (ours lower by 8 to 51, PS8, like PW8) and one pass (2028-03-04) reads producer 0's production and efficiency as 0 although the produced total matches (PS9, shadow-side). Power 396/491 (the bookkeeping lines, PW8), flora 72/95 (FQ15), aura 3/96 (AU13–AU15); demand 2969/2969, protection 190/190, landfill, budget, pollution and crime 95/95. |
| 82 | main `2800741` (land value + flammability shadows, `apply2`; DLL `f2f43a28`), `tools/regress.ps1 -Tag r2800` (Konradshohe): **first in-game run of the land value and flammability shadows**, and of `apply2`'s crime and demand changes in the replace run | **PASS, 0 problems.** Replace (thirteen replacements): clean 150 s. Shadows: **land value 94/94 exact** (intrinsic, total and slope grids, types, average, mode and mean altitude on every pass; 79 passes matched with the desirability map from before the original's pass, which changed after it read it; `run_summary.ps1` now judges a line by its own `; mismatches=N` total, since this line's note "with the live map total mismatched=391" read as a difference); **flammability 64 of 65 compared cycles exact** (raw grid, adjusted grid within the visit window, average range, multiplier, hotspots, getters); cycle 66 (2033-03-15) has a 2×2 block of adjusted tracts at (7..8, 53..54), 81 against our 65 (FL5). Plumbing 186/187, power 388/481 (PW8), flora 70/93 (FQ15), aura 2/94 (AU13–AU15); demand 2909/2909, protection 186/186, landfill, budget, pollution and crime 93/93. |
| 83 | main `5ee5c52` (`apply4a` + the x87 fix; DLL `7c5c4da6`), `tools/regress.ps1 -Tag r7c5c -Dll ...FreeCity4-x87fix.dll` (Konradshohe): **first run with the x87 leftover guard** | **PASS, 0 problems; x87 leftovers 0 in both runs** (the heartbeat's new field; no `x87 leftovers at` line, no stack-fault line). Replace (thirteen replacements): clean 150 s (2026-02 → 2033-05). Shadows: demand 2963/2963 (the jobs-per-unit fix, `0.01f`, changed nothing the shadow sees), land value 95/95, protection 190/190, landfill, budget, pollution and crime 95/95; plumbing 187/190 (PS8 used totals, PS9 one producer read as 0, now on either side); power 391/486 (PW8); flora 72/95 (FQ15); aura 2/95 (AU13–AU15); **flammability 69/70**: cycle 15 has 595 adjusted tracts at about 1.25 times ours (FL6, the same ratio as FL5). |
| 84 | main `5ee5c52` (DLL `7c5c4da6`), `build-runs/repro74.ps1 -Label run84-soak-x87 -After 900` with all thirteen replacements plus the default simulator and budget (Konradshohe, `observe = 1`): **soak 3 again after the x87 fix** | **clean, 0 problems, x87 leftovers 0 for 35 game years** (2026-02 → 2061-09, 428 months; no `x87 leftovers at` line, no stack-fault line): run 74's fault did not recur. **The city path differs from runs 77/78**: the same start, then higher tax income from about 2041 (R$ and I-D taxes; `tools/compare_soaks.ps1`), and the collapse that took runs 77 (ours) and 78 (original) to about −80k by 2061-09 comes later: funds peak at 163k in 2053 and are +79,924 at 2061-09, with R/C/I 20219/2314/5289 against 16134/1583/3205. Runs 77 and 78 already differ by up to 50% in single tax lines, so one soak cannot separate variance from a bias: run 85 repeats the all-original soak with this DLL. |
| 85 | main `a32dd1a` (DLL `7c5c4da6`), `build-runs/soak-allorig.ps1` (`disable = simulator, budget`, nothing enabled; Konradshohe, 900 s, `observe = 1`): **the all-original soak again, to read run 84** | clean, 2026 → 2061. **The original's own path spans run 84's**: funds 20.6k / 68.1k / 107.2k / 109.3k / 101.6k / 48.9k at 2029/35/41/47/53/59 against run 84's 17.8k / 65.5k / 111.9k / 150.7k / 163.1k / 122.4k and run 78's (also all original) 14.4k / 52.8k / 76.5k / 79.0k / 47.8k / −19.6k. Run 85 tracks run 84 to about 2041 and keeps its population (R 21.6k at 2059 against run 78's 14.4k). **No bias to chase: run 84 is inside the original's variance**; the collapse of runs 77/78 was one draw of it. |
| 86 | main `bc7ffa0` (the residential glue; DLL `2f5a507a`), `tools/regress.ps1 -Tag rbc7f` with `residential` added to the replace list and `residential_shadow = 1` in the shadows run (Konradshohe): **first in-game run of our residential simulator and of its shadow** | **PASS, 0 problems.** Replace (fourteen replacements): clean 150 s, 2026-01 → 2033-06; `residential month` lines every month: population 15,025 → 24,462 (R$ 22,494, R$$ 1,751, R$$$ 217), EQ 58.0 → 38.1, HQ 68.6 → 42.7, life expectancy 67 → 60; the original declines the same way (EQ 42.2, HQ 50.2 in the shadow by 2030, equal on both sides), and run 86's replace run reaches EQ 48.6 / HQ 54.7 at 2030. Save Write/Read lines: not exercised (no save in regress). **Shadow: 661 of 881 lines exact**; each cycle 12 of 15 daily states match on all 25 grids, the saved fields, the 24 per-building properties, funding and the scalars (population, EQ, HQ, life expectancy identical to the last digit). States 1–3 (the proximity grids) differ in about 16,000 of 16,384 cells: the original's corner reads 1, 0, 0, 0 where ours counts down 104, 103, … (RG23). The residential shadow slows the game: the shadows run covered 57 months in 150 s against 95. Budget shadow 30/31: one month's total is 1 above the original's with every tax line equal (BQ3). Others: demand 1737/1737, land value 57/57, protection 112/112, landfill and pollution 56/56, plumbing 108/113, power 265/321 (PW8), flora 42/56 (FQ15), aura 4/57 (AU13–AU15). |
| 87 | main `1d1274d` (`apply3`; DLL `d3ca1ef3`), `tools/regress.ps1 -Tag r1d12` (fourteen replacements, residential included; Konradshohe) | **PASS, 0 problems.** **PR5 in game:** `compat: original fire simulator vtable at 0x00AB1160`, and the original's unadjusted fire and police grids equal ours on every month (protection 372/372, the unadjusted lines included). Residential shadow 1116/1488 (RG23 only), demand 2924/2924, land value 94/94, budget 93/93 (BQ3 did not recur), landfill, pollution and crime 93/93; plumbing 175/187 (PS8, PS9), power 383/476 (PW8), flora 74/93 (FQ15), flammability 67/70 (FL5/FL6), aura 5/94 (AU13–AU15, being applied). The shadows run reached 94 months this time, so run 86's 57 months was load from the lanes' builds, not the residential shadow. |

**Run 39 settles the per-type question below:** on identical state, ours equals the original on every tax type. The
type-3 gap between runs 35 and 36 was the original's run-to-run drift, not a bug.

**Budget differential (runs 35 vs 36, plus 29 as a second original sample).** All runs start from the same save with
the same month-1 population (R/C/I 2314/424/860).
- **Expenses match exactly.** Every department's `dept_expense` is identical in all three runs, in all five months.
- **Income: the original is not deterministic from run to run**, so per-type tax cannot be diffed with separate runs.
  - In month 1, from an identical start, the two original runs (29 and 35) differ in tax types 4 (23 vs 19), 6 (3 vs 5)
    and 9 (243 vs 239). Ours (36) gives 24, 1 and 239.
  - Type 3 is the one value outside the original's observed spread: ours 26, the original 20 in both runs.
  - By month 5 the per-type values spread further as growth diverges.
- **Totals: ours is inside the original's spread.** After five months, funds are 71,265 (35) and 71,325 (29) for the
  original, and 71,298 for ours.
- **An observer artifact, not a budget difference:** runs 35 and 28 report 2002-01 identical to 2001-12. The month
  snapshot at the year boundary can fire before the collection.
- **Next step:** a deterministic tax-by-type diff needs both budgets computed on the same city state in the same process
  (a shadow run of our budget core next to the original's at each collection), not two separate runs. Tax type 3 is the
  first thing to check there.

**The real cause of the run-23/run-26 crashes was not power.** The master-clock `Simulator` kept its state in a
`RefCell`. The game calls our `cISC4Simulator` getters from a **second thread** while the main thread is inside
`OnTick`; `RefCell` is not `Sync`, so the concurrent read panicked and the release build aborted (`panic = "abort"` →
`0xC0000409`). The panic hook added this session named it exactly:
`RefCell already mutably borrowed` at `core/src/simulator.rs:473`. Fixed by making `state` a `Mutex` locked for one
`with()`/`read()` section and never across `deliver`/`post`/refcount hooks, so a blocked getter waits microseconds
rather than a whole tick. Regression test `a_second_thread_may_read_the_clock_while_a_tick_runs` fails **3717 of
4000** reads on the old code with the identical panic and passes on the new.

Two corrections to earlier records: **run 23 never had power enabled** (its log line 20 reads
`subsystem 'power': off by default`), so the handoff's diagnosis — "a Rust panic in the power core or capi, prime
suspect the Read/Init bitmap change" — is disproven; power code never ran in run 23. And the handoff's run-23 ini
(`power_report = 1` without `enable = power`) is why no `power recompute` line could ever appear there. Re-running the
exact crashing binary `F5439E03` (`.run23-crash`) as a control came out **clean** and got past run 23's death point
(`GetTotalFunds #20` vs `#17`), confirming the race is intermittent and power-on merely widened the window.

**x87 precision (runs 23/24, resolved 2026-10-13).** The main thread runs at 24-bit precision in the unmodified game
for the whole session, and **our DLL's mere presence raises it to 53-bit** — verified in the bytes and by an A/B:
run 19 had both the simulator and the budget DISABLED (observer only) and produced 122/104 (the 53-bit result) for
departments `0x09188F42` (p=0.41) and `0x09188F4C` (p=0.35), while the unmodified game's save of the same city
holds 123/105. The Spandau save is byte-identical to the backup (SHA-256 `6982A842…`), so both sides read the same
`ideal=300`. The game asserts 24-bit exactly once, in `PreFrameWorkInit`, and **nothing can restore it afterwards**
(the only unconditional 24-bit setter has one xref; every other site is Lua-related and guarded so that it skips
both calls unless already at 24-bit). Full evidence and the two queued next steps are in foundations §18.1–§18.2.

**Consequence for you and for us:** while FreeCity4.dll is loaded, every *original* subsystem computes at the wrong
precision, so **no differential run against the original code is trustworthy for float-sensitive values** until the
trigger is found. Our own budget replacement computes the department expenses at 24-bit and reproduces the real
game's saved values (123/105). What precision does to *manual play* is bounded by the same effect seen on those
department expenses — about ±1 simoleon per department per month — but that is the size of the precision artifact,
not a proof that our whole budget matches the real game; that match is still the open differential (§5.3 below).
Either way, the game you play with our DLL loaded is not bit-identical to the game without it.

**The run-19-vs-run-20 differential is therefore invalid as written** (handoff §5.3). Both runs had our DLL loaded,
so **both ran at 53-bit**; run 19 is not the original game's true behaviour, it is the original code computing at the
wrong precision inside our process. The comparison also changes two variables at once (our budget vs the original,
*and* 24-bit vs 53-bit), so the tax-by-type differences (types 4/6/9: run 19 `586/28/0/20/27/0/1/0/0/242/5/0` vs run
20 `586/28/0/20/42/0/0/0/0/238/5/0`) and the non-tax income gap (35 vs 28) cannot be attributed to either cause.
**Do not treat run 19 as a reference.** To actually close the budget differential:

1. **Fix or work around the precision leak first** (the two queued steps in foundations §18.2). Only then does the
   original code inside our process run at 24-bit.
2. **Re-run observer-only (run-19-style) after the fix** to capture the original's true 24-bit values for the same
   first month, and diff those against run 20. Same city, same month, one variable.

**A 24-bit oracle already exists in the Spandau save** (parsed directly from the DBPF, budget simulator record
`0xE990BE01`, byte-identical to the backup `6982A842…`). These are the unmodified game's own outputs, so they are
true 24-bit reference values: `funds 69986`, `lastTaxIncome 866`, `ytdTaxIncome 6078`, `lastMonthIncome 904`,
`lastMonthExpense 796`, and per-group `monthlyTax` `[574, 28, 0, 34, 0, 0, 0, 0, 0, 228, 0, 0]` (sum 864; all tax
rates 7.0 except group 8 at 9.0, max 22.0). **Caveat:** the save is at its own month, not run 19/20's 2001-10, so
these are a consistency check and a target for a matched-date run, not a line-by-line diff against run 20 without
first aligning the simulation date.

**Root cause:** the game has three separate allocator instances, each with its own pools
(`docs/abi/allocators.md`). The lot list's nodes come from the game's STL node allocator. The SDK's `SC4List`
freed them through the framework allocator service, so the two instances later handed out the same memory. The fix
is a list type that uses the game's allocator. The other places where the plugin and the game share containers
are being classified now; any mismatch found there is the same kind of bug.

Notes:
- Auto-load works: `autoload_city = <save path>` in `FreeCity4.ini`, so runs need no click.
- The faulting function (exe `0x5E16E0`) is the game's generic small-block pool allocator, not just the sound pool.
- Checked in the exe and matching ours: the list layout, the node allocator, the service refcount, and the +1 reference per lot.

## Progress log while you were away

- M2 core (demand, ordinance, weather) merged, a164fc9. Whole workspace test suite green.
- Specs committed:
  - dbf6237: flora spec; foundations §4.1/§5.2 fixes; M3 open questions resolved.
  - 2412ca3: ABI rows; flammability prop/flora label erratum.
- Implementer questions are queued in `docs/specs/QUESTIONS.md`.
- Budget heap corruption, static lead:
  - Our message object has type and flags at swapped offsets compared with the game's (`docs/abi/layouts.md`).
  - A fix was requested from the plugin lane.
  - This is unproven; the bisection still decides.
- Specs committed:
  - b9da700: fire and police protection.
  - ebc9e88: plumbing.
- Land value simulator and the game-layout message fix merged, e2dc324. Land value is off by default
  (`enable = landvalue`).
- The checked DLL with the message fix is staged in Plugins. The previous one was renamed `*.pre-msgfix`.
- Run 8 starts automatically once you close the game: all budget switches on, with the message fix.
  - If it's clean, the message layout was the cause.
  - If not, the bisection continues.
- Specs: power (b06e048). All open questions are answered (80d86eb … 0d45db2).
- New foundations §18: the simulation runs at 24-bit x87 precision. The specs' worked examples were recomputed
  (cde56f4).
- M3 cores merged (4e7ccbb): grid module, pollution, crime, flammability and aura. 181 conformance vectors and
  155 core tests are green.
- Now running:
  - lane 1: budget test 14 recheck, then the in-game glue for weather, ordinance and demand (off by default);
  - lane 3: grid reconciliation, then the cores for power, plumbing, fire/police and flora;
  - spec lane: education, health and mayor rating.

## To verify in-game (after the bisection finds the culprit)

- [ ] Budget replacement, all switches on: 2–3 months at cheetah speed, money changes each month, budget
      window (income, expenses, departments) looks normal.
- [x] Save → exit → reload with our simulator and budget: the date, funds and departments survive (runs 50/51,
      2026-10-13, done by Claude with `tools/sc4ctl.ps1`).
- [x] Pause, unpause and all three speeds (run 50, scripted again in runs 52/53).
- [x] Emergency pause (run 54: a God-mode fire disaster; `EmergencyPause()`/`EmergencyResume()` reach our simulator).
- [x] Clock differential against the original on the same save (runs 56/57 and 58/60: identical over 240 days,
      `clock_probe` + `tools/clock_diff.ps1`).
- [ ] Cheats: `RiskyMoney`, `WeaknessPays`, `Moolah` (if enabled) do what the original does.

## Decisions for you

- [ ] Upstream to gzcom-dll: the SDK has several wrong signatures and return types (see `docs/abi/` and
      foundations §17 errata). Report them to the SDK maintainers? (Public action, so it needs your OK.)
- [ ] Repo hosting and publication (GitHub, project name, README wording) when you want it public.

## Done / answered

- 2026-10-12: language question → logic stays clean-room; interface facts (`docs/abi/`) are open to
  implementers.
