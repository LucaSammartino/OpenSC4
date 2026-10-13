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
- [ ] Save → exit → reload with our simulator and budget: the date, funds and departments survive.
- [ ] Pause, unpause and all three speeds. Emergency pause, if a disaster can be triggered.
- [ ] Cheats: `RiskyMoney`, `WeaknessPays`, `Moolah` (if enabled) do what the original does.

## Decisions for you

- [ ] Upstream to gzcom-dll: the SDK has several wrong signatures and return types (see `docs/abi/` and
      foundations §17 errata). Report them to the SDK maintainers? (Public action, so it needs your OK.)
- [ ] Repo hosting and publication (GitHub, project name, README wording) when you want it public.

## Done / answered

- 2026-10-12: language question → logic stays clean-room; interface facts (`docs/abi/`) are open to
  implementers.
