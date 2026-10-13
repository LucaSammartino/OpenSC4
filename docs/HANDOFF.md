# Handoff: state of FreeCity4 (2026-10-13)

This file is for the agents that take turns on this repo. Read it first, then update it before you stop.
The progress checklist in [README.md](../README.md#progress) tracks milestones; this file tracks the work in flight.

## 1. The project in one paragraph

We are reverse engineering SimCity 4 Deluxe (GOG 1.1.641, exe SHA-256 `3bc5c7fe…a4b`) and building a clean-room,
GPL-3.0 reimplementation that replaces the game piece by piece, OpenRCT2-style. The pieces are:
- **Rust core:** `engine/core`.
- **C ABI:** `engine/capi`.
- **C++ GZCOM shim plugin:** `engine/plugin`. It builds `FreeCity4.dll`, which loads into the real game from
  `Documents\SimCity 4\Plugins`.
- **JSON conformance vectors:** `engine/conformance`.

Specs for implementers are in `docs/specs/`, and interface/ABI facts in `docs/abi/`. The roadmap (M0–M6+) is in
`docs/ARCHITECTURE.md`.

## 2. Rules that do not bend

- **Clean room** ([CONTRIBUTING.md](../CONTRIBUTING.md)):
  - Spec writers (RE side) read the exe, Ghidra, `decomp/` and `re/`.
  - Implementers (`engine/` code) must NOT open:
    - `decomp/`, `decomp-mac/`, `external/`;
    - `ghidra/project/`, `ghidra/mac/`, `ghidra/export/`;
    - `re/`, `tools/`;
    - `docs/subsystems/`, `docs/sim-engine.md`;
    - the exe.
  - Implementers may read `docs/specs/`, `docs/abi/` and the public SDK headers.
  - An agent doing engine work never reads RE material in the same task.
  - Engine lanes run in worktrees and cannot read files outside them. To give a lane a game log, copy it into the
    lane's gitignored `build-runs/`.
- **Never commit** game files, decompiled output, or anything from KillaVolt's repo, which grants no license and also
  ships EA's assets.
- **The game install directory is read-only.**
- **Public actions need the user's explicit OK:** upstreaming SDK errata to nsgomez/gzcom-dll, publishing the repo,
  and posting anywhere.
- **Commits:**
  - Use the branch `main` (not master).
  - End commit messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Never bypass CAPTCHAs.** Simtropolis shows a "verify you are human" page, so the user downloads from there.
- The user granted full authority over this computer for the project's work, outside the limits above.

## 3. How to run the game unattended

```powershell
powershell -ExecutionPolicy Bypass -File tools\ingame_run.ps1 -Label runN -Seconds 400 -AfterLoadSeconds 160 -Ini "budget_default = 1`npower_report = 1`nobserve = 1`nautoload_city = C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Spandau.sc4"
```

- **What the script does:**
  - It writes `Plugins\FreeCity4.ini` and clears `Plugins\FreeCity4.log`.
  - It starts the game from this shell. Started that way the game is not elevated, so it can be killed. Never start
    it from the Start-menu shortcut.
  - It autoloads the city and waits for the plugin's `AUTOTEST VERDICT`.
  - It copies the log to `build-runs/<label>.log`.
- **Exit codes:**

  | Code | Meaning |
  |---|---|
  | `0xC0000409` / `-1073740791` | fail-fast: a Rust panic (`panic = "abort"`) |
  | foreign first-chance AV in the log | heap corruption |

- **Plugins folder:**
  - The DLL is swapped in by hand. Back up the current one as `FreeCity4.dll.<tag>` first, and copy the matching
    `.pdb` with it.
  - The ini key for the developer-wide switch is `budget_default` (1 = all our subsystems on).
  - The per-subsystem keys are in `engine/README.md` and `engine/plugin/FC4Config.h`.
- **Test city:** `Regions\FC4 Test\City - Spandau.sc4`.
  - Backup: `%TEMP%\claude\…\scratchpad\Spandau.sc4.bak`, SHA-256 `6982A842…`; it was unchanged after run 23.
  - Restore the city from the backup after any save test.
- **Current Plugins state, safe for manual play:**
  - DLL `81AC46F4…`: the pre-run-23 build, with power off by default.
  - ini: `budget_default = 1`.
  - Kept alongside: `FreeCity4.dll.run23-crash` (F5439E03…) and `.pre-run23`.
- The run history and the manual checklist for the user are in `docs/manual-checks.md`. Runs 22 and 23 are not yet
  recorded there.

## 4. Where each milestone stands

See the README checklist for the full list. In short:
- **M0:** done.
- **M1:** runs in-game. Save → reload, pause, emergency, all speeds and the differential are not yet verified.
- **M2:**
  - The budget, demand, ordinance and weather cores and glue are done, and the budget runs in-game with no heap
    corruption (runs 17–18).
  - **The x87 workaround is on main (`63e88f1`) and validated against the original**, so the differential is unblocked:
    run 29 (`disable = simulator,budget`, the game computes) gives `dept_expense` 123/105 across all five months,
    matching the unmodified Spandau save, where run 19 with the same config and no re-assert gave 122/104 (§5.2, §5.3).
  - The differential itself still needs redoing against that fixed baseline; the old tax-by-type differences
    (types 4/6/9) and the non-tax gap (35 vs 28) came from a run pair that both computed at 53-bit, so they are not
    attributable (§5.3).
- **M3:**
  - All ten grid-sim cores are done.
  - Land value glue is done (off by default).
  - **Power glue is merged (`63e88f1`) and runs clean** — runs 27/28, `AUTOTEST VERDICT: clean`, production 5997→6000,
    409 cells powered, 0 unpowered (§5.1). The crash was a cross-thread `RefCell` race in the clock, not power.
  - **Plumbing + flammability glue merged** (`9f741de`, from `m3-glue`): coordinator-verified ctest 18/18 and cargo
    808/0; `FC4_ABI_VERSION` 14, plugin switch count 57. **But never executed in game** — every simulation path is
    unverified and the host tests cover only the no-city neutral path (§5.4b). Merged because complete and green, not
    because proven in the game.
  - Protection, flora, pollution, crime and aura glue are not started.
- **M4: COMPLETE — all five cores on main.**
  - Cores done: BDS, growth developer, tract developer. Lane 3 also did a consolidation pass (shared `fc4_core::stl`
    STLport sort/hash map/set; `BuildingProfile` moved into `development.rs`) and seed derivation (foundations §6.3:
    `rng::derive_seed`, `from_session`).
  - **Lot developer merged** as `9889f65` (5,254 lines).
  - **Zones merged** as `f56e52a` (~10,900 lines: manager, developer, parcel, region; ~1,800-line conformance runner;
    30 vectors covering all 29 spec §15 tests). Verified independently before merge: `cargo test --workspace` exit 0,
    808 passed / 0 failed; clippy exit 0 with zero warnings in zones files; `rustfmt --check` clean on all 7 zones
    files. The 20 pre-existing fmt diffs in `engine/capi/src/*_tests.rs` are identical on main and are not a
    regression.
  - **Next M4 work is glue, not cores:** GrowthDevelopers adapter, `demand_active_value`, developer token = BDS ID,
    then the growth budget calibration in the original (building-development.md §12).
  - **A sixth core exists beyond those five, unmerged:** the residential simulator (branch `residential-core`,
    `cc6e25d`). It compiles clean and is wired into `lib.rs`, but `simulator.rs` has zero tests and no conformance
    vectors, so it is unproven WIP. Its RS1–RS6 spec questions are preserved in QUESTIONS.md (§5.4b).
- **M5:** specs done (traffic simulator, network, pathfinder). **First core drafted but does not compile** — see §5.4b.
- **M6:** specs landed:
  - `data-formats.md` (601f3a9);
  - lot/occupant save records (0460f3a);
  - city save records, validated against 417 saves (e69009f).

## 5. Open threads, most urgent first

> **Rewritten 2026-10-13** after this session closed §5.2, §5.5, §5.6 and most of §5.7, and made large progress on
> §5.1 and §5.4. The old text of those sections is in git history (`c255c0f`). Commits from this session, all on
> `main`: `8fdea4e` (spec answers), `ea602a4` + `e647189` (x87), `c7d1e51` + `9c02e76` (manual-checks/differential),
> `4de4f46` + `0c83333` (build/KillaVolt removal), `f46f2f6` (README progress), `9889f65` (lot developer),
> `f56e52a` (zones), `7992647` (M4 docs), `63e88f1` (**lane 1 merged: the x87 re-assert, power glue, the cross-thread
> clock fix**), `bf3f998` (ZQ1 answered from the exe; foundations §19 corrected by lane 1's measurement),
> `ad1a19a` (HANDOFF for the lane 1 merge), and from the final wind-down `7938b38` (residential RS1–RS6 preserved +
> the landfill agent's staged spec answers), `9f741de` (**m3-glue merged: plumbing + flammability glue**), `94d8cdf`
> (both spec agents' final unstaged edits, including the landfill NaN correction that repairs `7938b38`).
>
> **Main is verified at `94d8cdf`:** `cargo test --workspace --target i686-pc-windows-msvc` 808 passed / 0 failed
> (exit 0), `ctest` 18 of 18, and the `engine/` tree is byte-identical to the `9f741de` merge that produced those
> numbers (the two docs commits on top touch only `docs/`). Evidence on disk in `build-runs/`:
> `cargo-test-main-94d8cdf.txt`, `ctest-m3-glue.txt`.

### 5.1 Power glue — FIXED, and the real crash was a cross-thread `RefCell` race
- **Was:** DLL F5439E03… crashed `0xC0000409` ~0.3 s after PostCityInit (run 23). The handoff blamed the power core.
  **That diagnosis is disproven:** run 23 never had power enabled (its log reads `subsystem 'power': off by
  default`), so power code never ran there.
- **Real cause:** the master-clock `Simulator` held its state in a `RefCell`. The game calls our `cISC4Simulator`
  getters from a **second thread** while the main thread is inside `OnTick`; `RefCell` is not `Sync`, so the
  concurrent read panicked and the release build aborted. The panic hook named it:
  `RefCell already mutably borrowed` at `core/src/simulator.rs:473`.
- **Fix (lane 1, commit `73b8861`):** `state` is now a `Mutex`, locked for one `with()`/`read()` section and never
  across `deliver`/`post`/refcount hooks, so a blocked getter waits microseconds, not a tick. `timers` stays a
  `RefCell` (main-thread only). Regression test `a_second_thread_may_read_the_clock_while_a_tick_runs` fails
  **3717 of 4000** reads on the old code with the identical panic and passes on the new.
- **Power now works:** runs 27 and 28 are clean (`AUTOTEST VERDICT: clean`, exit 0, no panic), 41 `power recompute`
  lines, `production 5997 → 6000`, 409 cells powered, **0 unpowered**. Run 22's zero-production gap is closed.
  Run 28 loaded the exact staged DLL `F921001D…` (hash verified in its attach line), so committed source, staged
  DLL and the validated run correspond.
- **Run 26 was a crash, not clean** — the coordinator recorded it wrongly in `c7d1e51`; corrected in `manual-checks.md`
  and `README.md`. Its power numbers were real but written before the death.
- **Verification:** `cargo test --workspace` 714 passed / 0 failed; `cargo clippy -D warnings` clean; ctest 15/15
  excluding `abi_conformance_test`, which fails on a pre-existing ABI declaration bug (see §5.8).
- **MERGED to main as `63e88f1`.** The §5.8 ABI test now passes (see §5.8 for how), so the merge was not blocked.
  Merged main re-verified by the coordinator: `cargo test --workspace` exit 0, 808 passed / 0 failed; ctest 100% of 17
  including `abi_conformance_test`.

### 5.2 x87 precision — RESOLVED (RE side); the trigger hunt is open (engine side)
The previous handoff concluded "the cause is in the game, not in our code" and that the original probably runs at
53-bit. **That is wrong; verifying it in the bytes reverses it.** See `docs/specs/foundations.md` §18.1–§18.2
(commits `ea602a4`, `e647189`) for the full evidence. Summary:
- The **unmodified game runs the main thread at 24-bit for the whole session.** It asserts 24-bit exactly once, as
  the first call of `PreFrameWorkInit` (`CALL 0x005F85E0`, verified in the bytes at `0x0045236C`), and **nothing can
  ever restore it** afterwards — the only unconditional 24-bit setter has exactly one xref, and every other site is
  Lua-related and *guarded*, so it skips both its set-to-53 and its set-back unless already at 24-bit.
- **Our DLL's mere presence raises the main thread to 53-bit** between PreAppInit and PostAppInit. Proven by an A/B:
  run 19 had the simulator **and** budget DISABLED (observer only, nothing replaced) and produced department
  expenses `122/104`; the unmodified game's save of the same city holds `123/105`. Same code, same city, only
  difference is our DLL loaded. Our own budget replacement computes `123/105` — i.e. **we reproduce 24-bit
  correctly and it is the original that computes wrongly inside our process.**
- Two tooling gotchas found on the way: Ghidra renders the real 42-byte FPU helper `0x008929E0` as an **empty
  function** (pseudocode cannot be trusted on this path), and `PreFrameWorkInit`'s **decompilation fails outright**
  (`field129_0x90 does not fit in structure cWinSC4App`), which is why the 24-bit assert was invisible in `decomp/`.
- **The trigger is NOT located.** What raises PC to 53-bit lives in *our* process during app init, so it is an engine
  hunt, not an RE one. Lane 1 has the two runnable experiments (fine-grained CW logging across the ~2.4 s window;
  then a minimal attach-only DLL). Our DLL's own load-time CRT init is excluded (CW is still 24-bit at
  "DLL attach (after our initialisation)"). MXCSR never moves, only the x87 PC field (00→10) — consistent with
  `_controlfp(_PC_53, _MCW_PC)` or an `fldcw` with PC=10. A latent game bug is also documented (the exemplar
  float-expression parser `nSCRes::ParseFloat`→…→`0x0067BA40` sets 53-bit and never restores) but it only fires on
  **text**-format resource parsing, which shipped binary tuning does not trigger — so it is not the cause here.
- **The workaround has LANDED (lane 1, merged `63e88f1`)** — §18.2's re-assert is in: at `PostAppInit` the plugin calls
  the game's own unconditional helper `0x008929E0` with mode 0 to restore 24-bit, and it is logged. Every control-word
  reading after the re-assert is `0x007F` (24-bit) for the rest of the session. Validated against the original, not just
  against ourselves: run 29 (`disable = simulator,budget`, so the *game* computes) gives `dept_expense` 123/105 across
  all five months, matching the unmodified Spandau save; run 19, same config and no re-assert, gave 122/104. (Note run 28
  already showed 123/105 *without* this fix, but that was our budget core emulating 24-bit in software — only a
  `disable = budget` run tests the workaround.) This is what protects the 361 `r24()` worked examples across 12+ specs
  and unblocks every milestone's differential.
- **What is still open is the trigger, not the symptom.** The re-assert makes the wrong precision moot for differential
  work, but *why* our DLL's presence raises PC to 53-bit is still unknown. One narrowing result is in hand: inside the
  ~2 s window our DLL makes exactly two calls — `director Release` and `director AddRef`. The next decisive step is a
  minimal attach-only DLL, not finer logging (§5b item 1).

### 5.3 Budget differential (M2 exit) — UNBLOCKED; needs redoing against the fixed baseline
- **The run-19-vs-run-20 differential cannot be used.** Both had our DLL loaded, so **both ran at 53-bit**; run 19 is
  not the original's true behaviour. It also moved two variables at once (our budget vs original, *and* precision),
  so the tax-by-type differences (types 4/6/9) and the non-tax income gap (35 vs 28) can't be attributed. Details in
  `docs/manual-checks.md` (commit `9c02e76`).
- **A 24-bit oracle now exists:** parsed directly from the Spandau save's budget record `0xE990BE01` (file
  byte-identical to backup `6982A842…`): `funds 69986, lastTaxIncome 866, ytdTaxIncome 6078, lastMonthIncome 904,
  lastMonthExpense 796`, per-group `monthlyTax [574,28,0,34,0,0,0,0,0,228,0,0]`. Caveat: the save is at its own
  month, not run-19/20's 2001-10, so it is a consistency check / matched-date target, not a line-by-line diff.
- **Step (1) is done:** the §5.2 workaround is on main, so the original now computes at 24-bit in-process. Run 29 is the
  proof — `disable = simulator,budget` with the re-assert reproduces the Spandau save's own 123/105.
- **Remaining steps:** (2) re-run observer-only now that precision is correct, to capture the original's true 24-bit
  first month; (3) diff that against our budget on the same city/month (one variable). The tooling to parse the save
  oracle lives in `tools/dbpf.py` (used it this session).

### 5.4 M4 cores — DONE and merged (lot developer `9889f65`, zones `f56e52a`)
Both merged to main and verified green (808 tests pass, exit 0). M4 is complete at five of five cores. Details in §4.
- **The zones spec gap is CLOSED.** zones.md round 6 (§7.10 DrawStreet, §7.11 ExtendAsStreets, §7.12 LayInJogStreets,
  §7.13 connection test, tests 19–29) is all present; bf8cbd7 filled it. No need to re-check.
- **A real bug was found and fixed during zones self-review:** `SimpleParcellize`'s §7.5 step-1 table gives Q=2 the same
  x-line rule as Q=0, but the implementation grouped Q=1 and Q=2, so Q=2 always started at x2 descending and ignored D.
  Invisible to every prior vector (tests 19–26 route through `Parcellize`; test 14 runs `SimpleParcellize` with
  `placeStreets` off). `zn15_31` now pins it, confirmed by reverting the fix and watching the vector fail.
- **A re-entrancy note worth carrying to other lanes** (foundations §19.3 rule 4): a temporary in an `if` condition
  lives through the whole `if` body, so `if self.st.borrow().loaded { …host call… }` holds a `RefCell` borrow across a
  host call even though it looks scoped. Zones audited and fixed seven such sites.

### 5.4b Lane states after the wind-down (2026-10-13, final)
The user ordered a hard stop: every agent was terminated (`TaskStop`, not a queued message), every lane's work is
committed, and **no agent is running** — `ListAgents` reports none. One lane merged; three branches hold work that is
committed and safe but deliberately **not** merged. Nothing is left in a working tree: `git status` is clean on main and
on all three remaining worktrees.

**Merged this session — `m3-glue` → main as `9f741de`.** Plumbing + flammability glue: 17 files, 4,506 insertions,
all under `engine/`. The coordinator verified it independently rather than trusting the lane's report: fresh ctest
**18 of 18 passed** (configure and build exit 0), `cargo test --workspace --target i686-pc-windows-msvc`
**808 passed / 0 failed**, and the lane reports 3,007 slots compared with 0 mismatches in required interfaces and 58
waived. Its merge-base was main's tip at the time, so it merged with **zero conflicts**. Numbers to keep consistent:
`FC4_ABI_VERSION` **14**, plugin switch count **57**. Worktree removed and branch deleted (5 → 4 worktrees).
> **Do not treat this as working.** Per the lane's own report it has **NEVER executed in game**. run32 was the single
> attempt and produced no information at all (see the run32 lessons below). Every simulation path — the water pass,
> neighbour deals, the budget department, the grid swap, the `SetWateredState` direct call, the daily flammability
> refresh, the fire roll, and critically the cross-subsystem `IsCellWatered` read — is **unverified**. The host tests
> cover only the no-city neutral path. This is merged because it is complete, compiles, and passes every test it has;
> the in-game proof is still owed.

**Not merged, and why:**
- **formats** (branch `lane/formats-original-compat`, tip `456dac3`, clean, 3 ahead / 36 behind main): **NOT GREEN.**
  21 passed but **3 failed** in dbpf — `duplicates_last_wins`, `dir_listed_but_not_compressed_is_returned_raw`, and the
  "open-with-warning" sub-case of `bad_inputs_are_errors`. All three are stale assertions encoding the *old* behaviour
  that spec §11 told the lane to change, so they are probably a one-line fix each, but nobody has fixed them.
  `cargo test --workspace` was **never run**, and the real-game-data sweep — the lane's own stated critical regression
  gate — was **never re-run** against its changes and must pass before merge. qfs and ltext are done and tested; dbpf
  items 1–5 have code with tests pending; items 7–14 (all in `exemplar.rs`) are not started. Note a public API change:
  `DirEntry` lost its `instance2` field (no consumer, but it is a breaking change). Blast radius is contained — no
  crate outside `engine/formats` depends on `fc4-formats`, and it edited no docs, so its merge takes main's HANDOFF
  cleanly once green.
- **landfill + neighbors** (branch `m6-landfill-neighbors`, tip `a6ba8ce`, clean, 4 ahead / 17 behind main): compiles
  and is wired in. **The coordinator's earlier note here was wrong** — it said "landfill ~60% written, 0% compiled, not
  declared in `lib.rs`". Verified reality: 8 files, 3,779 insertions, touching only `landfill/` plus one `lib.rs` line
  (`pub mod landfill;`), `cargo build` exit 0 with **zero warnings**, and **12 tests pass**. What is missing is
  verification, not code: `tests.rs` is a **placeholder stub**, there is no conformance runner and no `lf_` vectors, and
  **the entire neighbors core was never started.** So it is compiling-but-untested. It is also 17 commits behind main,
  so a merge would need conflict resolution — new work during a wind-down, hence deferred.
- **residential** (branch `residential-core`, tip `cc6e25d`, clean, 2 ahead / 36 behind main): **compiles clean**
  (0 errors, 0 warnings) with `pub mod residential;` wired into `lib.rs`, and the lane's `cargo test --workspace`
  passed (fc4-core 341, conformance 387). It is therefore *not* a repeat of the `m5-traffic-network` failure mode —
  main stays buildable with it merged. But it is **unproven WIP**: `simulator.rs` has **zero tests of its own** (the 10
  passing tests are all in `geometry.rs` and `tuning.rs`), `record.rs` (the §9 save/load record) was never written, no
  conformance vectors exist, and `ResidentialHost` declares ~50 education/health callbacks that no fake host
  implements, so the trait has never been built against. `DoPopulationChange`, the state machine and the queries are
  written to spec and exercised by nothing.
  Its **RS1–RS6 questions are preserved** in `docs/specs/QUESTIONS.md` (committed `7938b38`). They had to be
  hand-transcribed: the lane's own edit to that file was written against the older `(No open questions.)` base, so it
  cannot merge over main's ZQ1–ZQ6. **RS1 is load-bearing** — whether the `0.01` in the percentage getters is a float
  or a double literal; residential's worked examples require double while `protection/algo.rs::pct` uses float and its
  vectors pass, so one of the two cores carries a ±1 ulp difference on every percentage. It is answerable from the
  getter's bytes at `0x005FDAC0` and friends, and was **not** chased because that is new RE work during a wind-down.
- **traffic-network** (branch `m5-traffic-network`, tip `c45c377`, worktree already removed): unchanged from the
  earlier note — ~2,000 lines drafted, **does not compile** (~13 mechanical borrow-checker errors), module commented
  out of `lib.rs` so its tests have never run. All commits are retained on the branch.
- **The two RE spec-answer lanes were hard-stopped mid-commit**, and the coordinator committed their working trees.
  Their output is real and valuable:
  - `7938b38` — the landfill agent's *staged* answers (which rode into an index that also held the RS1–RS6 edit, so the
    commit message was amended to describe both honestly): assembly-cited `*[compat]*` findings on the provenance of
    `data4` in message `0x278E6003` (exactly one send site, `cSTETerrain::RedisplayTerrain`, always `MessagePost` so
    never synchronous), the correction that `PlaceZone` delivers `0x07699455` **by post** so there is **no re-entrancy**
    inside `DezoneCell`, and what `AttemptAreaLevelling` really does — read from the assembly because **Ghidra's
    decompilation of `0x006C44F0` is unreliable** (it renders the float argument as a pointer and drops the vertex
    average).
  - `94d8cdf` — both agents' *unstaged* final edits. The landfill half is the **correction the agent was writing when
    it was stopped**, and it repairs an error that `7938b38` had just committed: the `FUCOMPP` against `FLT_MAX` is
    followed by `TEST AH, 0x44` / `JP`, so an **unordered (NaN)** `maxAreaSlope` **does** take the `FLT_MAX` branch —
    `7938b38` said the opposite. It also hedges the ten-argument `ModifyTerrainHeight` call whose stack bookkeeping the
    agent had not fully reconciled, and adds §15 tests 22–23. The lot-developer half answers **§12.2 Q8, Q9 and Q10**
    with citations: the night-light prop query uses the **lot's** bounding rect, not the building's (Q8); the anchor is
    never given a designate and in the usual case is not cleared either, because `SetTravelDesignate`'s `old == new`
    early-out makes the two indistinguishable while the designate is null (Q9); and the dissolution migration does
    **not** skip the just-joined member, which destroys a farm unit's designate and makes member order observable (Q10).
  > **Lesson:** a hard stop catches an agent mid-edit, and the fix it was writing can be the thing that corrects an
  > error it had already written. Both agents' last words were "let me fix … then commit". Check the *unstaged* diff
  > after a `TaskStop`, not just the log — and expect it to contradict committed text.
- **Backup:** `build-runs/windown-backup-20261013/` held 5,067 lines that existed only as untracked files. Every lane
  has now committed, so the condition on that directory ("safe to delete once every lane has committed") is **met** —
  but the coordinator left it in place rather than delete during a wind-down. Delete it deliberately, or keep it.

### 5.5 Spec answers — DONE, committed
All 25 queued questions (GD2–GD5, BD1–BD14, TD1–TD5) reviewed against their diffs and committed as `8fdea4e`;
`docs/specs/QUESTIONS.md` was empty at that point. The two core lanes were told to merge main and drop their assumptions
(both have). Nothing further unless a new question is queued.

**`QUESTIONS.md` is no longer empty** (this line was stale). It now holds two sections of *engine-lane* questions that
the coordinator filed from lane reports: **ZQ1–ZQ6** from the zones core (ZQ1 answered from the exe and struck through in
`bf3f998`, so the numbering still matches the lane's report; ZQ2–ZQ6 open) and **RS1–RS6** from the residential core
(`7938b38`). The later spec-answer lanes also closed several questions *inside* the specs rather than here — landfill
§10.2 items 2, 3 (partly) and 4, and lot-developer §12.2 Q8–Q10, both in `94d8cdf`. So the queue is: **ZQ2–ZQ6,
RS1–RS6, and landfill §10.2 items 1 and 3's unresolved half** (which brush sets `terrainMessageData4` to 1, 2 or 3 —
needs the terraforming tool UI traced).

### 5.6 KillaVolt removal from the rebuild instructions — DONE and verified
- Committed as `4de4f46` (tooling + regenerated `re/names_windows.tsv` + the new `re/symbols/*.tsv`) and `0c83333`
  (README records the verified pipeline output). The README rebuild steps no longer mention KillaVolt; the only
  external dependency is `0xC0000054/sc4-ghidra-symbols` (**Apache-2.0**, Ghidra type/FID metadata only — no game
  code or assets). Nothing from KillaVolt was copied in.
- **The §5.6 "accept as-is vs add the 44 as fallback" decision is resolved on evidence:** all 44 lost names are
  `library`-source STLport/CRT internals (locale `do_get`/`do_put`, SEH handlers, `__tzset`, `__fprem`), none
  referenced by any spec, so they are **not** re-added. The ~141 renames are FID giving cleaner spellings
  (`luaK_self`, not `_luaK_self`), also spec-irrelevant.
- **The pipeline is verified end-to-end** (was the one open item in `4de4f46`): `build.ps1 -SkipMatch -ProjectRoot
  <fresh> -DecompOut <scratch>` ran with exit code 0 on a project built from our own exe — `renamed=15859 created=8
  missing=76 failed=191 thiscallConverted=10145`, `labels=856 vtablesTyped=853 failed=0`, 1815 decomp files. All 191
  `failed` are benign ("symbol `_lua_*` already exists" — FID applied the identical name; 0 differ, 0 are
  manual/community). All **906** hand/community names apply cleanly. build.ps1 gained `-ProjectRoot`/`-DecompOut`
  this session to make that testable without touching `ghidra\project` or `decomp\`.
- **`docs/abi` does NOT need regeneration** (the old §5.6 sub-item "regenerate if names changed" is closed). Checked
  directly: of the 9,154 vtable/interface slot targets in `docs/abi/{vtables,interfaces}.tsv`, **zero** were among the
  141 renamed or 44 lost addresses — the entire name churn is in `lib::`/`_lua_*`/STLport names that never appear as
  an abi slot target. `docs/abi` is consistent as committed (601f3a9). (abi_extract.py needs `pefile`, not installed
  in this env, but the target-set check above is conclusive without rerunning it.)
- **Still the user's to decide** (unchanged): whether to delete the local `external/sc4-decomp` + `external/extracted`
  KillaVolt clone, and whether to download the Mac Rev A binary (the `-Mac` path in `setup_ghidra.ps1` is untested).

### 5.7 Smaller items
- **`docs/manual-checks.md`: updated this session** (`c7d1e51`, `9c02e76`, and the run-26 correction) — runs 22–28
  recorded. **Plugins state is now manual-play safe:** lane 1 staged `FreeCity4.dll` = `F921001D…` (tag
  `run28-threadfix`, `.pdb` alongside) and reset `FreeCity4.ini` to `budget_default = 1` alone. The Spandau city file
  is untouched (`6982A842…`). Backups kept in Plugins: `.run28-threadfix-prestage`, `.run27-threadfix-prestage`,
  `.run24-mine`, `.run24-panic-prestage`, `.pre-run23`, `.run23-crash`. No game process left running.
- **`docs/abi/vtables.tsv`: checked, clean** (committed at 601f3a9; `git status docs/abi/` empty). Consistent.
- **SDK errata draft** (`docs/sdk-errata-draft.md`, 289 findings): **re-checked against the current vendored SDK on
  2026-10-13** (commit `4243367`). 252 of 257 header-backed findings still match the SDK text verbatim; the other 5
  differ only by signature normalisation and all still hold; upstream's recent fixes touch interfaces this draft
  reports nothing on. The draft's stale baseline claim (`3a1144f`, "revision unrecorded") was corrected — the SDK is a
  submodule pinned at `5af73c5`, upstream master's tip. **What remains is the upstreaming itself: a public action
  that needs the user's explicit OK.** Do not submit it autonomously.
- **Worktree cleanup: done 2026-10-13.** 18 merged-and-clean `.claude/worktrees/agent-*` were removed with their
  branches (`git worktree remove` + `git branch -d`, which refuses unmerged branches as a safety net). Verified
  first that no merged worktree held uncommitted work. `git fsck` reports only harmless dangling commits.
  - **Lane 1's worktree also removed** after its merge (`63e88f1`), verified clean first: 0 uncommitted files, 0 commits
    ahead of main. It needed `--force` for a reason worth knowing: the repo has a submodule
    (`engine/third_party/gzcom-dll`, the public SDK), and git refuses `worktree remove` on any working tree holding one,
    even when the submodule is clean and at the commit main expects. `submodule deinit -f` inside the worktree
    unregisters it but cannot delete the checkout on Windows (file lock), leaving an empty directory — so re-check
    clean+merged, then `--force`. **Four worktrees remain, all with live lanes or unmerged branches:** `formats-compat`,
    `m3-glue`, `m6-landfill-neighbors`, `residential`. `m5-traffic-network` has a branch but no worktree (its ~2,000
    non-compiling lines are on the branch, backed up).
- **Then plumbing, protection and flora glue** — now the `m3-glue` lane (`ac1485cbac8eb97a9`), which is doing plumbing
  and flammability; lane 1 finished and merged. Apply the run-22 lesson: `Read` runs before `Init`, so never size saved
  state from the city inside `Read`.
- **Coordinator clean-room note:** whoever picks this up should not do engine work in the same task as RE work. This
  session read `decomp/` and the exe extensively for §5.2, so it deliberately did not read or run anything under
  `engine/`; merging a lane means reading its *test output*, not its source.

### 5.8 `abi_conformance_test` — RESOLVED (slot 33 is now report-only, not a failure)
- **Was:** `cISC4NetworkTool` (PowerLine) slot 33 `GetBridgeAltitudeOverride` is declared 32-bit/void in the vendored
  SDK but the game returns in x87. The test failed on it, which was recorded as blocking lane 1's merge.
- **How it actually resolved — not by correcting `docs/abi/`.** The test separates three classes of table:
  *required* (interfaces the plugin actually implements), *waived* (a known SDK/game disagreement on a method no
  plugin source calls, each with a written reason in `kWaivers`), and *report-only* (printed as `note`, never a
  failure). `cISC4NetworkTool` is report-only; the plugin's required table is `cFC4ISC4NetworkTool`, which declares
  slots 3–10 only and passes. Current run on main: `2900 slots compared; 0 mismatch(es) in required interfaces,
  57 waived, 65 in reported-only ones`, exit 0. Slot 33 appears as
  `note … slot 33 GetBridgeAltitudeOverride: declared return 32-bit/void, the game returns in x87`.
- **The guard that makes the waiver safe:** a waiver holds only while no plugin source calls the method. The comment
  above `kWaivers` states the rule — before calling one, declare the interface in `FC4Interfaces.h` with the table's
  signature (as was done for `cISC4NeighborsSimulator`) and drop the waiver. So this cannot silently rot into a real
  ABI bug.
- **Carry-over for anyone wiring a grid sim that needs `GetBridgeAltitudeOverride`:** it is not waived, only
  reported. Calling it means declaring the interface with the x87 return first.

## 5b. Backlog for manual check / blocked-on-user (2026-10-13)
Things I could not finish autonomously, or that need a human decision:
1. **Locate the §5.2 trigger** — what in our process raises PC to 53-bit during app init. Two experiments queued for
   lane 1 (fine CW logging across the PreAppInit→PostAppInit window; minimal attach-only DLL). If lane 1 can't find
   it, apply the §18.2 workaround (re-assert mode 0 at PostAppInit) and log it.
2. **Redo the §5.3 budget differential** — only valid after #1; then re-run observer-only and diff against the
   Spandau save oracle.
3. ~~**Merge lane 1 / lot developer / zones**~~ **DONE** — lot developer `9889f65`, zones `f56e52a`, lane 1 `63e88f1`,
   each independently verified before merge. Merged main re-verified after the lane 1 merge: cargo 808/0, ctest 17/17.
4. **Reset `Plugins\FreeCity4.ini`** to `budget_default = 1` for manual play (see §5.7).
5. **User decisions** (unchanged, §7): run the manual checklist; upstream the SDK errata (public — the re-check
   against the current SDK is done, `4243367`, so the draft is ready to submit once approved); publish the repo
   (public); delete the KillaVolt clone; download the Mac Rev A binary.
6. **Mac reference path** in `setup_ghidra.ps1 -Mac` is untested (may need slice selection for a fat binary); only
   needed to re-run symport, which `-SkipMatch` avoids.

## 6. Agents and lanes

All background agents stop when their session ends. Check each worktree for uncommitted work (or a checkpoint commit)
before resuming; a SendMessage resumes an agent from its transcript **only while its session is alive** — a stopped
one needs a fresh Agent spawn pointed at the same worktree. This session (2026-10-13) respawned the three engine
lanes with **new agent IDs**; the old IDs from prior sessions are dead.

| Role (clean-room side) | Agent ID **this session** | Worktree / branch | State |
|---|---|---|---|
| **Lane 1** — engine glue, in-game fixes, x87 §18.2 workaround | `af1db58965f8ea547` | — | **MERGED `63e88f1`**; worktree removed, branch deleted. Merged main re-verified: cargo 808/0, ctest 17/17 |
| **Lot developer** core | `a9254b7ecd0aea910` | — | **MERGED `9889f65`**; worktree removed, branch deleted |
| **Zones** core | `a4b7ddc5791bcf0c3` | `agent-ab69d8e3cec021271` | **MERGED `f56e52a`**; worktree removed |
| **Traffic network** core (M5) | `a8060ce00faeba239` | `m5-traffic-network` | finished at `c45c377`; ~2,000 lines drafted, **does not compile**, excluded from `lib.rs`; worktree removed (§5.4b) |
| **Landfill + neighbors** cores (M6) | `ad6d35101273a7757` | `m6-landfill-neighbors` | **STOPPED**; finished at `a6ba8ce`, worktree clean. Landfill **compiles** (0 warnings) and is wired into `lib.rs`, 12 tests pass — but `tests.rs` is a stub, no vectors, neighbors not started (§5.4b) |
| **Residential** core | `a4bf4a1401fb0a69f` | `residential` | **STOPPED** (`TaskStop`); finished at `cc6e25d`, worktree clean. Compiles clean, `lib.rs` wired, but `simulator.rs` has 0 tests and no vectors. RS1–RS6 preserved in QUESTIONS.md (§5.4b) |
| **Grid-sim glue** (wiring cores into the game) | `ac1485cbac8eb97a9` | `m3-glue` | **MERGED `9f741de`**; worktree removed, branch deleted. Coordinator-verified: ctest 18/18, cargo 808/0. **Never ran in game** (§5.4b) |
| **engine/formats divergence fixes** | `a7aa00813e71c6ad5` | `lane/formats-original-compat` | **STOPPED**; finished at `456dac3`, clean. **NOT GREEN** — 3 dbpf test failures, workspace tests + real-game-data sweep never run. Do not merge as-is (§5.4b) |
| **Lot developer spec answers** (RE side) | `a4ccb0d58282e86e1` | main checkout | **STOPPED mid-commit**; coordinator committed its tree as `94d8cdf` (lot-developer §12.2 Q8–Q10) |
| **Landfill/neighbors spec answers** (RE side) | `a38c56ea07811b096` | main checkout | **STOPPED mid-commit**; coordinator committed staged answers `7938b38` + unstaged NaN correction `94d8cdf` (§5.4b) |

Prior-session lanes, now finished or superseded (their work is on `main`): spec answers (`8fdea4e`), M6 save records
and data formats (`e69009f`, `0460f3a`, `601f3a9`). The coordinator (this session) did the RE-side x87 work and the
§5.6 build changes directly on `main` — no separate agent.

**Wind-down state (2026-10-13, FINAL):** the user ordered a hard stop — "this has been going on too long. tell every
agent to stop, commit their work. and let's wind down." Every lane was terminated with `TaskStop` (not a queued message,
which can sit undelivered), and `ListAgents` now reports **no reachable agents**. One lane merged (`m3-glue`); three
branches hold committed-but-unmerged work; the two RE spec lanes were caught mid-commit and the coordinator committed
their trees. The merge pattern that worked throughout: read the lane's report, **re-run its test command independently**,
compare the counts, then merge. That is what makes a merge trustworthy — it caught a HANDOFF error (§5.8) and a stale
landfill status this session. Untracked source in worktrees is **not** recoverable if an agent dies; see the backup in
§5.4b. The one hard-stop lesson is in §5.4b: an agent killed mid-edit may have written an error it was *about* to fix —
always check the unstaged diff, which may contradict already-committed text.

## 7. Decisions waiting on the user

1. Run the in-game manual checklist (`docs/manual-checks.md`): budget window, save/reload, speeds, cheats.
2. Upstream the SDK errata. This is a public action.
3. Publish the repo. This is a public action.
4. Delete the local KillaVolt clone, and download the Mac Rev A binary (5.6).

## 8. Working conventions

- **Branches and merges:** lanes work on `worktree-agent-*` branches. The coordinator merges them into `main` after
  tests pass:
  - `cargo test` in `engine/` (use PowerShell, and add `$env:USERPROFILE\.cargo\bin` to the PATH);
  - `ctest` for the plugin;
  - the conformance vectors (about 370).
- **Run cargo from `engine/`, not the worktree root.** There is no `Cargo.toml` at a worktree root, so `cargo fmt --check`
  prints its help text and exits 0 — which looks like a pass. This bit me on 2026-10-13 and I nearly reported a
  formatting-clean result that never ran. Always read the output, never the exit code alone.
- **`fmt --check` has 20 pre-existing diffs on main**, all in `engine/capi/src/*_tests.rs`
  (`budget_tests.rs`, `landvalue_tests.rs`, `test_streams.rs`). They are not a regression from any lane. When checking a
  branch, compare against main's list rather than expecting zero.
- **`cargo clippy --all-targets -- -D warnings` is NOT green on main** — 15 errors, all in
  `engine/core/src/lot_developer/{simulator,tests}.rs`, pre-existing from the lot-developer merge `9889f65`. Reproduced
  on merged main after `63e88f1`, and proven not lane 1's: `git diff --stat 7992647 63e88f1 -- engine/core/src/lot_developer/`
  is empty, so that merge never touched the files. The lints are style, not correctness — 5 `map_or can be simplified`,
  3 `map(f)` where `f` returns `()`, plus an `if let` on `Some` only, a unit-valued let-binding, an `if` with identical
  blocks, and a negated comparison on a partially ordered type. `cargo test` and `ctest` are both green, so nothing is
  broken. Do not read this as your lane's regression; the same compare-against-main discipline as `fmt` applies. Fixing
  the 15 is a small self-contained cleanup, worth its own commit rather than being folded into a feature lane.
- **Git hygiene — a commit can land in the wrong worktree.** On 2026-10-13 a landfill WIP commit appeared on the *zones*
  branch (`e82565e`) as well as its own (`53fc7b9`): identical author timestamps, committer timestamps 7 minutes apart, so
  the same content was re-committed into another lane's worktree. All lanes share one repository, and a compound `cd` in
  bash moves the *session's* working directory, so a lane that cds around can commit somewhere unintended. Nothing was
  lost — I verified every file byte-identical before deleting the stray branch — but:
  - before deleting any branch, diff its tip against where the work *should* live and prove nothing is unique
    (`git diff --name-only <parent> <tip>`, then compare blob hashes);
  - prefer `git -C <worktree>` over `cd`;
  - after a merge, re-run the contamination sweep: each branch should touch only its own module under
    `engine/core/src/<module>/`, and main should contain none of the unmerged ones.
- **Checking results:** check exit codes, not "FAILED" text, because "0 failed" matches too. But also read the verdict
  line — `ctest` prints `100% tests passed, 0 tests failed out of 16`, and grepping for "failed" alone finds the zero.
  Symmetrically, **never read a truncated test total**: piping `cargo test --workspace` through `tail` gave a bogus
  "passed=22" once because only the last few binaries' lines survived. Capture to a file and sum every
  `test result:` line.
- **`build-runs/` is gitignored** (`.gitignore:22`, `build*/`), and always has been — no verification file in it has ever
  been committed, including `ctest-main-f56e52a.txt` and `coord-verify-main.ps1`. So verification evidence lives on disk
  only and the *numbers* go into HANDOFF. Do not try to commit these artifacts; record the counts and where the log sits.
  The reusable verifier is `build-runs/coord-verify-lane.ps1 -Engine <worktree>/engine` (parametrized
  `coord-verify-main.ps1`, sources `vcvars32.bat` and prepends VS's CMake bin so `cmake`/`ctest` resolve outside the VS
  shell).
- **In-game run hazards (learned from run32, the m3-glue attempt).** Two distinct traps, both of which cost a whole run:
  - **Writing the ini from bash passes PowerShell escapes literally.** `"`n"` in a bash heredoc/echo lands as the two
    characters backtick-n, not a newline, so `budget_default` absorbed the entire file and `autoload_city` never parsed.
    The DLL logged a config WARNING, no city loaded, and the game idled at the region view. Write the ini with a real
    newline (a PowerShell here-string in a `.ps1`, or printf), then **read the bytes back** before running.
  - **The harness can time out and exit without killing the game.** The orphaned `SimCity 4.exe` (pid 8016) then blocks
    *every* future run with "BLOCKED: SimCity 4 is already running", so one bad run poisons all later ones until the
    process is killed by hand. After any run that does not report a clean verdict, check for a live `SimCity 4` process
    and kill it before the next attempt.
  - **A staged DLL/PDB pair can be silently broken.** m3-glue overwrote `Plugins\FreeCity4.pdb` with no backup, and lane 1
    left no PDB backup either, so the matching pair is unrecoverable. The DLL is correct (it hashes to lane 1's verified
    build `ce242572…`, matching the `.run31-lane1-prestage` backup) but the PDB does **not** match it, so crash dumps
    would resolve symbols wrongly. **Restage a matching DLL/PDB pair before trusting any future crash dump.**
- **README checklist:** keep the Progress checklist in `README.md` current whenever something lands on `main`.
- **Status reports for the user:** plain language, short, and say what changed and what is next.
- **Memory:** the coordinator's notes are in
  `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\freecity4-project.md`. They cover:
  - the three allocators: game instances C/B/A, never free across them;
  - the cRZString layout;
  - autoload;
  - the Spandau test city.
