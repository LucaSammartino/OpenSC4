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
- **M4:**
  - Cores done: BDS, growth developer, tract developer. Lane 3 also did a consolidation pass (shared `fc4_core::stl`
    STLport sort/hash map/set; `BuildingProfile` moved into `development.rs`) and seed derivation (foundations §6.3:
    `rng::derive_seed`, `from_session`).
  - Lot developer and zones cores are in progress (§5.4).
- **M5:** specs done (traffic simulator, network, pathfinder). No cores yet.
- **M6:** specs landed:
  - `data-formats.md` (601f3a9);
  - lot/occupant save records (0460f3a);
  - city save records, validated against 417 saves (e69009f).

## 5. Open threads, most urgent first

> **Rewritten 2026-10-13** after this session closed §5.2, §5.5, §5.6 and most of §5.7, and made large progress on
> §5.1 and §5.4. The old text of those sections is in git history (`c255c0f`). Commits from this session, all on
> `main`: `8fdea4e` (spec answers), `ea602a4` + `e647189` (x87), `c7d1e51` + `9c02e76` (manual-checks/differential),
> `4de4f46` + `0c83333` (build/KillaVolt removal), `f46f2f6` (README progress).

### 5.1 Power glue — FIXED, pending lane 1's report and merge
- **Was:** release DLL F5439E03… crashed `0xC0000409` ~0.3 s after PostCityInit (run 23), no `power recompute`.
- **Now:** the panic-hook work was checkpointed to lane 1's branch (`8129d67`) so it could not be lost, and lane 1
  resumed it. **Run 26 (`build-runs/run26-power-enabled.log`, DLL `569FF4D8…`) is clean and power works:** 14
  `power recompute` lines, first one `production 5997 consumption 1038 demand 1038 powered 385 unpowered 0`. This
  closes both the run-23 crash and the run-22 "production 0" gap.
- **Lane 1** (agent id `af1db58965f8ea547` this session; worktree `agent-af159bf9f570ec0f6`): power fix + panic hook
  committed on its branch, has merged main, and is now working the x87 trigger hunt in `engine/core/src/simulator.rs`
  (see §5.2). **Not yet merged to main** — the coordinator merges after lane 1 reports tests green. Confirm its
  branch tip is its own commit (last observed `02dc7cf`, a merge of main; the power fix came in via `8129d67`).
- **Next:** merge lane 1 once it reports; keep `FreeCity4.ini` in a manual-play state afterwards (see §5.7 — it was
  left in run-26 test config with autoload + auto-unpause).

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

### 5.4 M4 lot developer and zones cores (lane 3) — in progress, both committed on their branches
- **The zones spec gap is CLOSED.** zones.md round 6 (§7.10 DrawStreet, §7.11 ExtendAsStreets, §7.12 LayInJogStreets,
  §7.13 connection test, tests 19–29) is all present; bf8cbd7 filled it. No need to re-check.
- **Both helpers resumed and committed** (their prior uncommitted work was checkpointed first: `a6acd74` lot
  developer, `bd77fc3` zones):
  - **zones** (agent `a4b7ddc5791bcf0c3`, worktree `agent-ab69d8e3cec021271`): tip `e7e921d` "zones conformance:
    runner and parcellization vectors (tests 19-21, 27, 29)", plus `979dd2f` core. Still iterating (more vectors).
  - **lot developer** (agent `a9254b7ecd0aea910`, worktree `agent-a324dd1b5caa1d469`): merged main, writing
    `simulator.rs`/`tests.rs` and `ld*` conformance vectors. This lane stopped mid-task twice with narration but no
    tool calls; resumed each time with "stop narrating, write the files". Watch for that failure mode.
  - Both have merged `main` (commit `8fdea4e`), so they see the corrected lot-developer/building-development specs.
    **Note the BD13 correction:** `UpdateLotElements` gets the whole 32-bit kind and compares `kind != 5` (NOT
    `kind & 0xFF != 0`) — pinned by lot-developer test 27. I mis-stated this in the first brief and corrected both.
- **After that:** M4 glue (GrowthDevelopers adapter, `demand_active_value`, developer token = BDS ID), then the
  growth budget calibration in the original (building-development.md §12).

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
- **`docs/manual-checks.md`: DONE this session** (`c7d1e51`, `9c02e76`) — runs 22–26 recorded, the safe-DLL note
  corrected (now `569FF4D8…`), and the x87/differential findings written up. **One thing to reset:** `Plugins\
  FreeCity4.ini` is still in run-26 test config (`enable = power`, `power_report`, `observe`, `autotest_seconds`,
  `autotest_unpause`, `autoload_city = …Spandau.sc4`). Before the user plays by hand, set it back to
  `budget_default = 1` alone, else manual play drops them into Spandau on the clock. Lane 1 was asked to reset it.
- **`docs/abi/vtables.tsv`: checked, clean** (committed at 601f3a9; `git status docs/abi/` empty). Consistent.
- **SDK errata draft** (`docs/sdk-errata-draft.md`, 289 findings, fully drafted): the remaining work is re-checking
  against *current* upstream nsgomez/gzcom-dll and then **upstreaming, which is a public action needing the user's
  explicit OK.** Backlog item — do not act autonomously.
- **Then plumbing, protection and flora glue** (lane 1). Apply the run-22 lesson: `Read` runs before `Init`, so never
  size saved state from the city inside `Read`.

## 5b. Backlog for manual check / blocked-on-user (2026-10-13)
Things I could not finish autonomously, or that need a human decision:
1. **Locate the §5.2 trigger** — what in our process raises PC to 53-bit during app init. Two experiments queued for
   lane 1 (fine CW logging across the PreAppInit→PostAppInit window; minimal attach-only DLL). If lane 1 can't find
   it, apply the §18.2 workaround (re-assert mode 0 at PostAppInit) and log it.
2. **Redo the §5.3 budget differential** — only valid after #1; then re-run observer-only and diff against the
   Spandau save oracle.
3. **Merge lane 1 / lot developer / zones** once each reports tests green (coordinator does the merge + README tick).
4. **Reset `Plugins\FreeCity4.ini`** to `budget_default = 1` for manual play (see §5.7).
5. **User decisions** (unchanged, §7): run the manual checklist; upstream the SDK errata (public); publish the repo
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
| **Lane 1** — engine glue, in-game fixes, x87 trigger hunt | `af1db58965f8ea547` | `agent-af159bf9f570ec0f6` | power fixed (run 26), panic hook + power committed on branch (via `8129d67`), merged main; now in `simulator.rs` on §5.2 |
| **Lot developer** core | `a9254b7ecd0aea910` | `agent-a324dd1b5caa1d469` | merged main; writing `simulator.rs`/`tests.rs` + `ld*` vectors. Stopped-with-no-tool-calls twice — resume firmly |
| **Zones** core | `a4b7ddc5791bcf0c3` | `agent-ab69d8e3cec021271` | committed `979dd2f` (core) + `e7e921d` (conformance runner, tests 19-21/27/29); still adding vectors |

Prior-session lanes, now finished or superseded (their work is on `main`): spec answers (`8fdea4e`), M6 save records
and data formats (`e69009f`, `0460f3a`, `601f3a9`). The coordinator (this session) did the RE-side x87 work and the
§5.6 build changes directly on `main` — no separate agent.

Other `.claude/worktrees/agent-*` folders are older, finished lanes. They are safe to clean up once their branches are
confirmed merged (`git branch --merged main`).

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
- **Checking results:** check exit codes, not "FAILED" text, because "0 failed" matches too.
- **README checklist:** keep the Progress checklist in `README.md` current whenever something lands on `main`.
- **Status reports for the user:** plain language, short, and say what changed and what is next.
- **Memory:** the coordinator's notes are in
  `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\freecity4-project.md`. They cover:
  - the three allocators: game instances C/B/A, never free across them;
  - the cRZString layout;
  - autoload;
  - the Spandau test city.
