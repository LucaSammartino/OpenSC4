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
  - The differential is open: tax by type differs for types 4/6/9, and non-tax income is 35 vs 28.
- **M3:**
  - All ten grid-sim cores are done.
  - Land value glue is done (off by default).
  - Power glue is merged but **crashes** (§5.1).
  - Plumbing, protection, flora, pollution, crime, aura and flammability glue are not started.
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
- **M5:** specs done (traffic simulator, network, pathfinder). **First core drafted but does not compile** — see §5.4b.
- **M6:** specs landed:
  - `data-formats.md` (601f3a9);
  - lot/occupant save records (0460f3a);
  - city save records, validated against 417 saves (e69009f).

## 5. Open threads, most urgent first

> **Rewritten 2026-10-13** after this session closed §5.2, §5.5, §5.6 and most of §5.7, and made large progress on
> §5.1 and §5.4. The old text of those sections is in git history (`c255c0f`). Commits from this session, all on
> `main`: `8fdea4e` (spec answers), `ea602a4` + `e647189` (x87), `c7d1e51` + `9c02e76` (manual-checks/differential),
> `4de4f46` + `0c83333` (build/KillaVolt removal), `f46f2f6` (README progress).

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
- **Next:** merge lane 1's branch (`73b8861`, `98c6a95`) to main once §5.8's ABI fix is in, since that test currently
  fails.

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
- **Workaround if the trigger proves unavoidable** (written up in §18.2): re-assert 24-bit ourselves at PostAppInit by
  calling the game's own unconditional helper `0x008929E0` with mode 0. Must be logged — it changes what the
  *original* subsystems compute while our DLL is loaded.

### 5.3 Budget differential (M2 exit) — blocked on §5.2; the old differential was invalid
- **The run-19-vs-run-20 differential cannot be used.** Both had our DLL loaded, so **both ran at 53-bit**; run 19 is
  not the original's true behaviour. It also moved two variables at once (our budget vs original, *and* precision),
  so the tax-by-type differences (types 4/6/9) and the non-tax income gap (35 vs 28) can't be attributed. Details in
  `docs/manual-checks.md` (commit `9c02e76`).
- **A 24-bit oracle now exists:** parsed directly from the Spandau save's budget record `0xE990BE01` (file
  byte-identical to backup `6982A842…`): `funds 69986, lastTaxIncome 866, ytdTaxIncome 6078, lastMonthIncome 904,
  lastMonthExpense 796`, per-group `monthlyTax [574,28,0,34,0,0,0,0,0,228,0,0]`. Caveat: the save is at its own
  month, not run-19/20's 2001-10, so it is a consistency check / matched-date target, not a line-by-line diff.
- **Correct path to close it:** (1) fix or work around the §5.2 trigger so the original runs at 24-bit in-process;
  (2) re-run observer-only to capture the original's true 24-bit first month; (3) diff that against our budget on the
  same city/month (one variable). The tooling to parse the save oracle lives in `tools/dbpf.py` (used it this session).

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

### 5.4b In-flight lanes — committed on their branches but NOT mergeable yet (2026-10-13 wind-down)
The user asked to wind down without losing work. Every lane was told to finish its current item, commit checkpoints, and
report — not to merge or push (the coordinator merges after verifying). State as of this writing:
- **traffic-network** (branch `m5-traffic-network`, tip `c45c377`): ~2,000 lines drafted across mod/model/rebuild/switches/
  tunnels/ferries/subnetworks/water/neighbours/tuning. **Does not compile** — ~13 mechanical borrow-checker errors
  (`&*h` deref of `Rc<dyn NetworkHost>`; immutable reads held across mutable `set_connection` writes; two `CAR`-as-u32 vs
  usize call sites). The module is **commented out of `lib.rs`**, so its tests have never run. `cargo build -p fc4-core`
  is green only because the module is excluded. No conformance vectors yet. `GetNeighborPoolPropertyHolder` seam is
  stubbed and flagged in-code as a coordinator question. Worktree removed (branch retains all commits).
- **landfill + neighbors** (branch `m6-landfill-neighbors`, tip `53fc7b9`): landfill ~60% written, **0% compiled** —
  files are deliberately not declared in `lib.rs`. Done: mod/algo/tuning/occupant/zone. Missing: simulator.rs (the
  `LandfillDeveloper` itself), tests.rs, the `pub mod landfill;` line, conformance runner + `lf_` vectors. **Neighbors
  core not started.** The 768 passing tests only confirm the pre-existing tree is still green; nothing in `landfill/` is
  verified. Design note: `dezone_cell` does §4.4.8 steps 1–3 and 5 and *returns* the 1×1 region for the caller to rezone,
  putting the re-entrant `PlaceZone`→`DoZoneChange` loops in simulator.rs; zones are `Rc`-held to survive self-removal.
- **residential** (branch `residential-core`): agent still running at wind-down; ~1,563 lines (geometry/mod/tuning) were
  untracked, backed up to `build-runs/windown-backup-20261013/`. Has since wired `lib.rs`. Verify before trusting.
- **grid-sim glue** (`ac1485cbac8eb97a9`), **power-glue crash fix** (`af1db58965f8ea547`), **engine/formats divergence**
  (`a7aa00813e71c6ad5`, branch `lane/formats-original-compat`): still running at wind-down.
- **Backup:** `build-runs/windown-backup-20261013/` holds a read-only copy of 5,067 lines that existed only as untracked
  files across four worktrees (landfill, residential, traffic-network untracked, m3-glue's `cFC4Plumbing.h`, plus zones and
  fpu uncommitted diffs). Safe to delete once every lane has committed.

### 5.5 Spec answers — DONE, committed
All 25 queued questions (GD2–GD5, BD1–BD14, TD1–TD5) reviewed against their diffs and committed as `8fdea4e`;
`docs/specs/QUESTIONS.md` is empty. The two core lanes were told to merge main and drop their assumptions (both
have). Nothing further unless a new question is queued.

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
  first that no merged worktree held uncommitted work. 3 active lanes remain. `git fsck` reports only harmless
  dangling commits.
- **Then plumbing, protection and flora glue** (lane 1). Apply the run-22 lesson: `Read` runs before `Init`, so never
  size saved state from the city inside `Read`.
- **Coordinator clean-room note:** whoever picks this up should not do engine work in the same task as RE work. This
  session read `decomp/` and the exe extensively for §5.2, so it deliberately did not read or run anything under
  `engine/`; merging a lane means reading its *test output*, not its source.

### 5.8 `abi_conformance_test` fails on a wrong SDK/ABI return channel
- **Symptom:** `cISC4NetworkTool` (PowerLine) slot 33 `GetBridgeAltitudeOverride` is declared 32-bit/void but the
  game returns in x87. It passed before this session's merge; `docs/abi/interfaces.tsv` (142 lines) and
  `vtables.tsv` changed in it. Lane 1 proved it is pre-existing, not caused by its work, by stashing all its
  changes and rebuilding.
- **Fix (RE/coordinator remit, not an engine change):** correct the declared return type to `float` in
  `docs/abi/` and the regenerated `interfaces.tsv`, and re-check the same for any other slot whose return channel
  moved in the regeneration.
- **Blocks:** merging lane 1's branch, since the workspace ctest is not green until this is corrected.

## 5b. Backlog for manual check / blocked-on-user (2026-10-13)
Things I could not finish autonomously, or that need a human decision:
1. **Locate the §5.2 trigger** — what in our process raises PC to 53-bit during app init. Two experiments queued for
   lane 1 (fine CW logging across the PreAppInit→PostAppInit window; minimal attach-only DLL). If lane 1 can't find
   it, apply the §18.2 workaround (re-assert mode 0 at PostAppInit) and log it.
2. **Redo the §5.3 budget differential** — only valid after #1; then re-run observer-only and diff against the
   Spandau save oracle.
3. **Merge lane 1 / lot developer / zones** once each reports tests green (coordinator does the merge + README tick).
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
| **Lane 1** — engine glue, in-game fixes, x87 §18.2 workaround | `af1db58965f8ea547` | `agent-af159bf9f570ec0f6` | `019ce21` "fpu: re-assert the game's 24-bit precision at PostAppInit" + `engine/tests/fpu_logic_test.cpp` on branch, unmerged; 6 files modified uncommitted. Waiting for its in-game run to show department expenses move 122/104 → **123/105** |
| **Lot developer** core | `a9254b7ecd0aea910` | — | **MERGED `9889f65`**; worktree removed, branch deleted |
| **Zones** core | `a4b7ddc5791bcf0c3` | `agent-ab69d8e3cec021271` | **MERGED `f56e52a`**; worktree and branch not yet removed |
| **Traffic network** core (M5) | `a8060ce00faeba239` | `m5-traffic-network` | finished at `c45c377`; ~2,000 lines drafted, **does not compile**, excluded from `lib.rs`; worktree removed (§5.4b) |
| **Landfill + neighbors** cores (M6) | `ad6d35101273a7757` | `m6-landfill-neighbors` | finished at `53fc7b9`; landfill ~60% written, **0% compiled**, not in `lib.rs`; neighbors not started (§5.4b) |
| **Residential** core | `a4bf4a1401fb0a69f` | `residential` | still running; ~1,563 lines backed up, `lib.rs` now wired (§5.4b) |
| **Grid-sim glue** (wiring cores into the game) | `ac1485cbac8eb97a9` | `m3-glue` | still running; `bfb19ad` plumbing C ABI (1,156 lines) + `cFC4Plumbing.h` uncommitted |
| **engine/formats divergence fixes** | `a7aa00813e71c6ad5` | `lane/formats-original-compat` | still running |
| **Lot developer spec answers** (RE side) | `a4ccb0d58282e86e1` | main checkout | still running |
| **Landfill/neighbors spec answers** (RE side) | `a38c56ea07811b096` | main checkout | still running |

Prior-session lanes, now finished or superseded (their work is on `main`): spec answers (`8fdea4e`), M6 save records
and data formats (`e69009f`, `0460f3a`, `601f3a9`). The coordinator (this session) did the RE-side x87 work and the
§5.6 build changes directly on `main` — no separate agent.

**Wind-down state (2026-10-13):** every lane was told to finish its current item, commit checkpoints, and report — and
explicitly *not* to merge or push. Merges are the coordinator's job after independent verification. The pattern that
worked: read the lane's report, re-run its test command myself, compare the counts, then merge. That caught nothing wrong
this time but is what makes a merge trustworthy. Untracked source in worktrees is **not** recoverable if an agent dies —
see the backup in §5.4b.

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
- **README checklist:** keep the Progress checklist in `README.md` current whenever something lands on `main`.
- **Status reports for the user:** plain language, short, and say what changed and what is next.
- **Memory:** the coordinator's notes are in
  `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\freecity4-project.md`. They cover:
  - the three allocators: game instances C/B/A, never free across them;
  - the cRZString layout;
  - autoload;
  - the Spandau test city.
