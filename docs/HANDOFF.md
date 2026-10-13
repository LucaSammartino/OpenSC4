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

### 5.1 Power glue crashes on the first day (run 23)
- **What happened:** release DLL F5439E03… (commits 2c77349 + be7bb41 + 6aa6735, merged to main as ef4159c) was run
  with `budget_default=1, power_report=1, observe=1`. It died with `0xC0000409` about 0.3 s after PostCityInit.
- **Last log lines:** `DepartmentBudget::IsLocallyFundedObjectInDepartment` and `GetLocalFundingPercent`. No
  `power recompute` line was ever written.
- **Diagnosis:** almost certainly a Rust panic in the power core or capi. Prime suspect is the new "Read keeps the
  saved words, Init sizes the bitmaps from the city" change.
- **Log:** `build-runs/run23-power-wires.log`, also copied to lane 1's `build-runs/`.
- **Lane 1 (agent `af159bf9f570ec0f6`, worktree `.claude/worktrees/agent-af159bf9f570ec0f6`):** asked to add a panic
  hook that logs the message, reproduce the crash, fix it and add a regression test.
  - It stopped when the previous session ended, with **uncommitted work** in its worktree:
    `engine/capi/src/panic_log.rs`, `engine/plugin/FC4FailFast.{cpp,h}`, `power/tests.rs`, `fc4.h`, `lib.rs`,
    `dllmain.cpp`, `FC4TestSeam.*`, `CMakeLists.txt`, `plugin_load_test.cpp`.
  - Resume it with SendMessage (to `af159bf9f570ec0f6`), or finish that work in the same worktree.
- **Next:** rebuild, stage the DLL, run again, and confirm non-zero production in the `power recompute` lines.

### 5.2 x87 precision: the cause is in the game, not in our code
- **Observation (run 23 log):** CW is `0x007F` (24-bit) at DLL attach, PreFrameWorkInit and PreAppInit. By
  PostAppInit enter it is `0x027F` (53-bit), and it stays 53-bit through ticks and city load.
- **What it means:** something inside the game's own app init (between PreAppInit and PostAppInit) switches the
  precision. Our DLL has no control-word writes (lane 1 checked), and the guard (be7bb41) logs this as a WARNING.
- **Consequence:** the original game likely runs its simulation at 53-bit too. So the run-19 "wrong precision"
  conclusion in `docs/manual-checks.md` (d90019e) needs revisiting: 53-bit may simply be the real runtime state.
- **Next (RE side):** find what in app init sets PC=53. Candidates are a D3D/DirectDraw device created with
  FPU-preserve, or a DLL's CRT init (`_fpreset`/`_controlfp`).
  - Decide which precision the reference behaviour uses and record it in foundations §18.2.
  - Then redo the budget differential (5.3).

### 5.3 Budget differential (M2 exit)
- **Status:**
  - Tax by type differs for types 4/6/9.
  - Non-tax income is 35 vs 28.
  - The tax multiplication order is fixed (6aa6735, b15_02 now 6992).
- **Candidates:** agent order, precision (5.2), or the lot set.
- **Lane 1 queue, after 5.1:**
  1. observer logs of agent order per tick/month;
  2. an `observe_lots` per-lot dump;
  3. non-tax income parts.
- **Then:** rerun the original game (run-19 style, `budget_default = 0` with the observer) against ours on Spandau.

### 5.4 M4 lot developer and zones cores (lane 3)
- **Lane 3:** agent `aa70dc8fc55381850`, branch tip 6d09937, merged to main.
- **Helpers:** worktrees `agent-a324dd1b5caa1d469` (lot developer) and `agent-ab69d8e3cec021271` (zones) have
  **uncommitted work**:
  - lot developer: `lot_developer/{mod,geometry,tuning}.rs`;
  - zones: `zones/{mod,region}.rs`.
  - Both are probably stopped too.
- **Briefs:** seed from the session seed; pin draw sequences. Zones is headless-limited: anything that needs the 3D
  view becomes a spec question. `CreateLotIfPossible` with facing −1 must make exactly one U(4) draw on a shared
  `ZoneDeveloperStatics` (session 0 gives seed 0x8C09C68A, the same seed the tract harness uses).
- **Spec gap:** lane 3 reported that zones.md round 6 (DrawStreet §7.10, ExtendAsStreets §7.11, LayInJogStreets
  §7.12, connection test §7.13, tests 19–29) was referenced but missing. bf8cbd7 (parcellization geometry) may have
  partly filled it. Check before resuming the zones helper.
- **After that:** M4 glue (GrowthDevelopers adapter, `demand_active_value`, developer token = BDS ID), then the growth
  budget calibration in the original (building-development.md §12).

### 5.5 Spec answers sitting uncommitted in the main working tree
- Two spec agents answered every queued question:
  - `ad8d875e09fe30aeb`: GD2–GD5, TD1–TD5, BD13;
  - `a88ab5b1b74d78d9a`: BD1–BD11, BD14.
- **Not committed:** their edits are in the main working tree. `docs/specs/QUESTIONS.md` is now empty, and there are
  edits to building-development, demand, growth-developer, lot-developer, power, tract-developer, traffic-network and
  traffic-simulator (+373/−94).
- **Next:**
  - Review that each removed question has its answer in the named spec.
  - Commit as "specs: answer GD2–GD5, BD1–BD11, BD13–BD14, TD1–TD5".
  - Tell lane 3 so the cores can drop their assumptions.

### 5.6 Removing the KillaVolt clone from the rebuild instructions (user request, nearly done, uncommitted)
- **Goal:** the user asked to take "clone KillaVolt" out of the README. Copying their content into the repo is not an
  option: there is no license, and it is decompiled output plus game files. The user asked about that, and was told
  why.
- **How:** we regenerate everything from the user's own exe and open sources.
- **New and changed files (all uncommitted in the main tree):**

  | File | Change |
  |---|---|
  | `tools/setup_ghidra.ps1` | `-Windows` imports `bin\SimCity4.exe` as "SimCity 4.exe", attaches 0xC0000054's FID DBs, auto-analyses, discovers functions and exports tables to `ghidra/export/windows`. `-Mac <path>` does the same for the Mac Rev A Intel binary, plus named pseudocode in `decomp-mac/`. Both refuse to overwrite an existing project; `-ProjectRoot` builds elsewhere. |
  | `ghidra/scripts/FC4ExportTables.java` | new: writes functions/function-details/calls/string-references/symbols/data-references TSVs, `instructions.asm` and `export-summary.txt` (the formats symport reads) |
  | `ghidra/scripts/FC4AttachFid.java` | new: attaches the FID databases before analysis |
  | `ghidra/scripts/FC4DiscoverFunctions.java` | new: creates a function at each orphan instruction that nothing falls into and that is 16-aligned or referenced |
  | `ghidra/scripts/FC4ApplyNames.java` | now creates a missing function at any listed name address, not only manual ones; logs `created=` |
  | `tools/symport/features.py`, `vtables.py` | read `ghidra/export/{windows,mac}` |
  | `tools/symport/merge_names.py` | reads library names from our own Function ID results (symbol source ANALYSIS); the KillaVolt community JSON is gone |
  | `re/symbols/community.tsv` | the 4 effects addresses, from caspervg/sc4-effects-extensions, byte-verified |
  | `tools/build.ps1` | new `-SkipMatch` (apply the committed names without the Mac reference) |
  | `README.md` | rebuild steps rewritten |
  | `CONTRIBUTING.md`, `.gitignore` | new output paths |

- **Verified:** a fresh project in scratch (`%TEMP%\claude\…\scratchpad\ghidra-fresh`) decodes 2,192,685
  instructions; the deep pass had 2,192,647. After discovery it has 35,554 functions, against their 35,629:
  - 35,532 are the same functions;
  - 35,414 of those have the same body size.
- **Found and fixed on the way:** spec agents had hand-edited the generated `re/names_windows.tsv` with sources
  `manual:m4zone|m4open|m5path|m6data|lua-host|qdrain` that had no source files, so a regeneration dropped them.
  - They are now `re/symbols/{m4zone,m4open,m5path,m6data,lua-host}.tsv`.
  - The `qdrain` row (0x6E3820 = `cSC4PollutionSimulator::Dispose`, formerly misnamed GetUnusedCapacity) was folded
    into `grid-sims.tsv`.
  - `0x6E01A0 cSC4PoliceSimulator::GetJails` (6bfb824) was added to `grid-sims.tsv`.
  - **Rule from now on:** add hand names to `re/symbols/*.tsv`, never directly to `re/names_windows.tsv`.
- **Current regenerated `re/names_windows.tsv` vs the committed one:** all manual and auto names are kept. The library
  names changed:
  - 137 are renamed (FID gives `luaK_self` where the old candidates had `_luaK_self`);
  - 44 are lost (STLport locale `do_get`/`do_put`, a few CRT);
  - 382 are new.
  - **Decide:** accept as-is (my recommendation), or add the 44 as a fallback.
- **The KillaVolt copies:**
  - `ghidra/export/windows` currently holds a copy of the fresh export.
  - The committed-name baseline is in scratch: `names_windows.kv.tsv`, `labels_windows.kv.tsv`,
    `win_features.pkl.kv`.
  - `re/symport/win_features.pkl` was regenerated from our own export.
- **Still to do:**
  1. Run `build.ps1 -SkipMatch` against the fresh project (`-ProjectRoot` is not in build.ps1, so point it there
     temporarily). Check `missing=` and `created=` from FC4ApplyNames, and that the decomp export works.
  2. Regenerate `docs/abi` if names changed (the docs/abi generator lives in `tools/`; see `docs/abi/README.md`).
  3. Commit.
  4. The user decides whether to delete the local `external/sc4-decomp` + `external/extracted` clone. Our current
     `ghidra/project` was originally copied from KillaVolt's project. Replacing it with a fresh one (setup, then
     build) is cleaner, but costs about 30 minutes of Ghidra plus re-applying names.
  5. The Mac reference (optional, needed only to re-run symport): the user must download the Mac "Rev A" Intel
     executable from Simtropolis' SC4 Maxis files (CAPTCHA). The `-Mac` path in `setup_ghidra.ps1` is untested
     (`-loader MachoLoader -processor x86:LE:32:default`; a fat binary may need slice selection).

### 5.7 Smaller items
- **`docs/manual-checks.md`:**
  - Record run 22 (power fix: clean, but production 0) and run 23 (crash + x87 finding).
  - Fix the "safe DLL" note: it now names 81AC46F4… with power off.
- **`docs/abi/vtables.tsv`:** it showed a big pending diff while spec agents were working. Check that it is
  committed and consistent before trusting it.
- **SDK errata draft** (`docs/sdk-errata-draft.md`, 289 findings): must be re-checked against current upstream
  nsgomez/gzcom-dll. Upstreaming needs the user's OK.
- **Then plumbing, protection and flora glue** (lane 1). Apply the run-22 lesson: `Read` runs before `Init`, so never
  size saved state from the city inside `Read`.

## 6. Agents and lanes

All background agents from the previous session most likely stopped when it ended. Check each worktree for
uncommitted work before resuming one with SendMessage (a send resumes it from its transcript).

| Agent ID | Role | Worktree / state |
|---|---|---|
| `af159bf9f570ec0f6` | **Lane 1**, engine glue and in-game fixes (clean-room) | `agent-af159bf9f570ec0f6`, uncommitted panic-hook work (5.1) |
| `aa70dc8fc55381850` | **Lane 3**, M4 cores and consolidation (clean-room) | `agent-aa70dc8fc55381850`, clean at 6d09937 |
| `a324dd1b5caa1d469` / `ab69d8e3cec021271` | lane 3 helpers: lot developer / zones | uncommitted (5.4) |
| `a77052740dcf936b1` | **Spec lane** (RE side): questions, seeding, calibration counters, M6 formats | main tree |
| `ad8d875e09fe30aeb`, `a88ab5b1b74d78d9a` | spec answers | uncommitted in the main tree (5.5) |
| `a24cdd2f56c8d9e0a`, `a59eaa0284a708729`, `aeb5a531488334f35` | M6 save records and data formats | landed (e69009f, 0460f3a, 601f3a9) |
| `a7a7dc539205c049c` | tuning catalogue consolidation | unknown; check |

Other `.claude/worktrees/agent-*` folders are older, finished lanes. They are safe to clean up once their branches are
confirmed merged.

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
