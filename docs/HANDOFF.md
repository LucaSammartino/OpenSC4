# Handoff: state of FreeCity4 (end of the second session, 2026-10-13)

This file is for the agents that take turns on this repo. Read it first, then update it before you stop.
The progress checklist in [README.md](../README.md#progress) tracks milestones; this file tracks the work in flight.

## 0. Start here

> **Third session (2026-10-13 evening to 10-14; in progress, the user is away).** Read this block first; the rest of
> §0 describes the end of the second session.
> - **M1 is complete.** In-game checks (runs 50–54): save → reload, pause, all three speeds, emergency pause. **Clock
>   differential: identical to the original** (runs 56/57, 240 days; runs 58/60 in `clock_probe = 2`, which holds
>   the week agent once to exercise the §5.3 repeat, 316 entries identical). Tooling: `clock_probe` (plugin,
>   `FC4ClockProbe.h`) + `tools/clock_diff.ps1 -A <original log> -B <our log>`.
> - **Unattended runs are now really unattended:**
>   - `popup_dialogs = 0` turns the game's popup dialogs off: no advisor popup, no hidden pause. **Since `d161d20`
>     it sets the flag only when ConnectionsReady `0x6AC284F3` arrives** (and true again at each later city load).
>     Before that (`5e39f3c`, at PostAppInit) it silently stopped ConnectionsReady, so **power, plumbing and
>     neighbours stayed disabled in runs 58–64, including the run 61 soak** (foundations.md §11.6, CR1; runs 64/65
>     isolate it, run 66 shows the fix). Generic prompts answer by themselves (a posted `WM_CLOSE` does nothing).
>   - `autotest_end = quit` (`22769f3`) ends a run through `cISC4App::RequestQuit(false, false)`: the full city and
>     app Shutdown path runs and counts toward the verdict (run 59: grids released, `ref_warnings=0`). Use it for
>     every glue lane's run.
>   - `tools/sc4ctl.ps1` / `tools/ingame_run.ps1` (`f850598`, `0e2c238`, `def371e`, `cce7b98`): posted clicks, real
>     Ctrl+S save, `-Actions`, automatic dialog closing, city backup and restore, and a 60 s wait for a previous
>     game process to exit. Control map: `docs/manual-checks.md` runs 47–54.
>   - Long runs (soaks, bisections) are started detached (`Start-Process powershell -WindowStyle Hidden`), because a
>     background tool call stops after 10 minutes; PowerShell `*>` output is UTF-16 (`tr -d '\000\r'` before grep).
>     **Never `tail -F` the plugin log from a monitor**: the orphaned tails lock `FreeCity4.log` and the next run's
>     harness cannot truncate it (it copied the soak's log instead). Poll with grep.
> - **Soak (run 61, 48 game years, 7 replacements): clean**, `ref_warnings=0`. Two findings: the ConnectionsReady
>   problem above, and **demand frozen**: the demand glue cannot read the BuildingProfile capacity lists or fill
>   `GetLocalPopulationSummary` (STLport `std::map` layout undocumented; RE ABI lane `re-abi`).
> - **Merged this session:**
>   - traffic network core (`dd240c9`: TN1–TN20 filed); residential core (`737bcc5`: RS7–RS17 filed);
>   - host grids released with `FC4_RELEASE_CREATED` (`bbe9181`);
>   - spec answers part 1 (`ac19c1e`), part 2 (`5b305c6`), part 3 (`6d0a68a`: CR1 ConnectionsReady, TN4–TN14),
>     part 4 (`a4d54b0`: PG14 `dirtyWater`, PG1–PG3, TN15–TN18, RS7–RS17);
>   - zones ZQ2/ZQ3/ZQ5/ZQ6 follow-up (`8ee3253`: tests 30–32 as vectors `zn15_32`–`zn15_35`; ZQ7–ZQ10 filed);
>   - **pollution glue + grid shadow** (`d743db2`; ABI 17; PG1–PG14 filed). In game: run 67 clean over 73 months;
>     run 68 **shadow identical on all eight grids for 94 months**, one flag-only difference (`dirtyWater`, PG14);
>     the PG1–PG3 `docs/abi` rows corrected and their waivers dropped (`6aea1a5`);
>   - **protection glue (fire + police) + grid shadow** (`29b3fcb`; ABI 18; PR1–PR13 filed);
>   - **crime glue + grid shadow** (`941c34f`; ABI 19; 91 plugin switches; CQ1–CQ15 filed);
>   - **demand glue fix** (`ae82e65`): capacity maps and `GetLocalPopulationSummary` through the game's STLport map
>     (`FC4GameMap.h`; switches `demand_capacity`, `demand_direct_calls`; DQ3–DQ8 filed);
>   - landfill tests 1–23 + **neighbours core** (`026fd9e`; LF1–LF5, NB1–NB17); `exemplar.rs` (`64a25d2`; EX1–EX20);
>     **pathfinder core** (`36ababe`; 28 `pf11_*` vectors; PF1–PF15);
>   - RE: `re-abi` (`4611f56`: STLport map/set layout, map `operator[]` direct call, BuildingProfile layout);
>   - harness: observe line shows the power getters (`7c94135`), `tools/regress.ps1` + `tools/run_summary.ps1`,
>     **autotest verdict `ended`** when the game shuts the city down by itself (`0173fe3`; run 69 had none),
>     `tools/compare_soaks.ps1` (two soaks side by side), and an `ingame_run` verdict read race fixed (`e5fd34a`);
>   - **protection fix** (`f7a9f16`): a host grid keeps its geometry only once non-empty (fire was 0×0 after a load;
>     pollution had the same latent fault; HG1 filed);
>   - **answers applied** (`4619a6f`): PG14 `dirtyWater`, TN1–TN20 in the traffic network core (TN21–TN25 filed).
>   - **aura glue + grid shadow** (`7c3f3a2`; ABI 20; seam op 14; 105 plugin switches; AU1–AU12 filed);
>   - RE `re-answers2` part 1 (`3844508`: CQ1–CQ15, PR4, PR7, DQ9); `cRZString` layout in `docs/abi/layouts.md`
>     (`2f37965`, PR3: no risk, the SDK's `SC4String` matches);
>   - **diagnostics** (`f518e37`): the foreign-exception report lists the registers and the return addresses into
>     the executable; heartbeats end with the memory (address space used, largest free block, private bytes);
>     `tools/addr2func.py` maps executable addresses to function names (coordinator only, reads RE material);
>   - **REH core** (`e520128`, education and health; RS7–RS17 applied; EH1–EH14 filed).
>   - **flora glue + census shadow** (`f8ebce0`; ABI 21; seam op 17; FQ1–FQ14), **landfill developer glue + shadow**
>     (`c2ada06`; ABI 22; seam op 18; 125 plugin switches; LG1–LG20), **traffic simulator core** (`a11eb7d`, M5;
>     29 `ts11_*` vectors; TS1–TS34), **power shadow** (`290093b`; ABI 23; seam op 21; PW1–PW7), **plumbing shadow**
>     (`571a47e`; ABI 24; seam op 22; PS1–PS7);
>   - RE `re-answers2` part 2 (`363bddb`: PR1–PR13, DQ3–DQ8, PG4–PG13 in the specs) and its `docs/abi` corrections
>     (`7d2f368`: lot `GetCenter` and city slots 96/97 return nothing; our `GetCenter` declaration is now `void`).
>     **Protection and landfill still call city slot 97 through a float-returning pointer (PR6)**: next apply lane;
>   - tools: `regress.ps1` runs every shadow (flora, landfill, power, plumbing added) and all thirteen replacements;
>     `run_summary.ps1` reads `mismatched N`, `clean`, flora's dated month lines and `Shutdown:` lines.
>   - **land value + flammability shadows** (`9bb91f2`; ABI 25; seam ops 23–24; LV1–LV4, FL1–FL4);
>     **`apply2`** (`2800741`: DQ9's float sum order, CQ1–CQ15 in the crime core and glue, PR4/PR7 comments; seam
>     ops 26–27; CQ16–CQ22 filed); RE **`re-answers3`** complete (`4e389d6`, `68ad36d`: PF1–PF15, ZQ4/ZQ7–ZQ10,
>     LF1–LF5, NB1–NB17, EX1–EX20 in the specs; **LF4 changes the zot spiral: `lf11_11` must be regenerated**; PF5,
>     NB2, NB3, NB6, EX3–EX5, EX8, EX10, EX12, EX14, EX17 differ from our code).
>   - **`apply4a`** (`8d014d8`: EX1–EX20 in `engine/formats` and the core variant, the 24-bit text parser,
>     `df5_01`; EX21–EX33 filed); **x87 fix** (`5ee5c52`, includes `abi-x87`; seam ops 28–31; DQ10–DQ12 filed);
>     **`apply4b`** (`58e1226`: PF1–PF15, ZQ4/ZQ7–ZQ10, LF1–LF5 (`lf11_11` regenerated), NB1–NB16 in the cores;
>     TN26–TN28, ZQ11–ZQ12, LF6, NB18 filed); **residential glue + shadow** (`bc7ffa0`, M4; ABI 26; seam op 25;
>     136 plugin switches; RG1–RG23 filed); **`apply3`** (`1d1274d`: PR1–PR13, DQ3–DQ8, PG4–PG13; ABI 27, seam op 32:
>     the protection shadow reads the original's unadjusted grids after a vtable check; PR14–PR16, PG15, DQ13 filed).
> - **Fixed on main (`5ee5c52`); runs 83/84 count 0 x87 leftovers, the 900 s soak over 35 game years included.**
> - **Run 74, the likely cause (audit workflow + coordinator check of the instructions): our methods corrupt the
>   game's x87 register stack.** The ABI extractor tagged as `x87` every method whose float work ends in a popping
>   store (`fstp [mem]`), so we declared `float`/`double` where the game returns nothing (or a bool in AL): demand
>   node slots 16/17 (`AddToSupplyValue`/`AddToDemandValue`, called on every demand propagation step), demand
>   simulator slot 8 (`CalculateJobsPerUnitOfDemand`, which **also writes the wrong elements**: the game's record is
>   four floats and the outputs are `[1..3]`), weather slot 11, variant slot 41. Each call leaves one value on the
>   game's x87 stack; at eight the next load gives NaN silently (exceptions masked), and NaN as an integer is a
>   garbage size: run 74's signature. The crime probe saw 3 leftover values in most months of runs 74–79.
>   **Branch `abi-x87` (`a91e140`, not on main until the fix lands, since the conformance test then fails on our
>   declarations):** `tools/abi_extract.py` decides x87 returns by a forward x87-depth walk (depth 0 at each call,
>   1 if the code after it uses the call's value); 63 interface rows and 105 vtable rows corrected, PG hand answers
>   kept; `docs/abi/README.md` states the rule; demand.md/weather.md errata (jobs `f32(u32 p) × 0.01f`, not `p/100`).
>   Also found: plumbing releases the pipe-burst `AttemptDisasterStart` result, which it never acquired.
> - **In game this session after the merges:** run 69 (soak 2, 8 replacements): the game ended the city in 2057,
>   bankrupt (funds −103,047). **Runs 71/72 settle it: with our power and with the original's, the city follows the
>   same path and ends in 2060-12 at about −100k funds** (aging plants, no player); our power tracks the original. Run 70 (`regress r941c`): PASS; **protection shadow 194/194 clean (first in-game
>   protection run)**; **but the fire replacement's grid is 0×0 after a load, so its cycle never runs** (lane below).
>   Crime shadow clean (0 mismatched cells, hotspot equal) in the first crime run.
>   Run 73 (`regress r4619`): PASS, fire fixed, pollution shadow 94/94. Run 75b (`regress r7c3f`, first aura run):
>   PASS; the aura shadow is close but not exact (AU13–AU15). **Run 74 (soak 3, all ten replacements on
>   Konradshohe): `corrupt` 15 s after load**: the game's own allocator (`0x5B9FC0`, behind `operator new`) got NULL
>   from `malloc` for a block over 4 KiB and wrote through it; no FreeCity4 frame on the stack. Memory was not short
>   (run 76: 722 of 2047 MiB used, largest free block 955 MiB), so the game asked for a garbage size. Runs 76 (300 s)
>   and 77 (900 s soak) repeated run 74 with the new report, then run 79: **no fault in 3 repeats**. A static audit
>   workflow (five RE auditors, adversarial verify) is looking for game code that sizes an allocation from data we
>   hand it. **Runs 77/78 (all ten replacements against all original, Konradshohe): the same funds curve for 36
>   game years and the same end, 2062-06 at −100k** (run 75's survival to 2064 was variance).
>   **Run 80 (`regress r2900`)**: thirteen replacements clean; landfill shadow 94/94; power shadow exact on all 213
>   cycles and 94 months (bookkeeping lines off by ~3 consumers, PW8); flora registry identical, one tree in 23 of
>   94 months (FQ15).
>   **Run 81 (`regress r571a`, first plumbing shadow)**: PASS; plumbing 186/191: the grid identical cell by cell in
>   every replayed pass; 4 passes differ in `demand` only (PS8, like PW8), one pass reads a producer's production as
>   0 (PS9, shadow-side).
>   **Run 83 (`regress r7c5c`, the x87 fix)**: PASS, x87 leftovers 0; demand 2963/2963 after the jobs fix;
>   flammability 69/70 (FL6: a region at about 1.25 times ours, the ratio of FL5). **Run 84 (soak 3 again, all
>   thirteen replacements + simulator + budget, 900 s)**: clean, 0 leftovers, 2026 → 2061; **the city path differs
>   from runs 77/78**: higher tax income from about 2041, and the collapse comes later (+79,924 at 2061-09 against
>   about −80k). **Run 85** (all original, same DLL) spans run 84: variance, not a bias.
>   **Run 86 (`regress rbc7f`, first in-game residential)**: PASS with fourteen replacements; our residential clean
>   for 7 game years; its shadow exact on 12 of 15 daily states (all grids, fields, properties and scalars), the
>   proximity states 1–3 differ (RG23); budget 30/31 (BQ3). **Run 87 (`regress r1d12`, apply3)**: PASS; PR5's
>   unadjusted grids equal every month. `tools/regress.ps1` now replaces residential by default.
>   **Run 88 (save → reload, fourteen replacements)**: our residential's `Write`/`Read` work (population, wealth
>   split and EQ/HQ restored); after the in-session save, land value's own `ReleaseGrid` released the last reference
>   to its three grids from `Read` (3 reference warnings at Shutdown): lane below.
> - **Lanes running** (clean-room engine lanes in `.claude/worktrees/agent-*`; RE lanes in
>   `.claude/worktrees/re-answers4`, `re-answers5`):
>   - **neighbours glue + shadow** (`cFC4NeighborsSimulator`, `neighbours_shadow`; ABI 28, seam op 33; renumber at
>     merge);
>   - **development glue + shadow** (BDS `0x8990BE7A` and the growth developers `0x0990C138` as one unit,
>     `development_shadow`; ABI 29, seam op 34);
>   - **`apply5`**: AU1–AU15 (the aura shadow rebuilt on the original's state field, §14.3), EH1–EH14, FQ1–FQ15 (the
>     64 m index query of FQ15, inherited flora properties); ABI 30 / seam op 35 if needed;
>   - **land value grid references after a save** (`FC4_RELEASE_CREATED` or the shared host grids; audit of every
>     glue's Read → Shutdown references; seam op 36 if needed);
>   - RE `re-answers5` (TS1–TS34, PW1–PW8, PS1–PS8, LG1–LG20) and `re-answers6` (LV1–LV4, FL1–FL6, DQ10–DQ12,
>     CQ16–CQ22, TN21–TN25, HG1, the crime errata; then RG, TN26–28, ZQ11–12, LF6, NB18, EX21–33). RE `re-answers4`
>     is merged (`79d619c`; its `docs/abi` rows in `83aa768`).
>   Merge order rule: merge one lane, then have the next lane `git merge main` itself (it knows its files), then
>   gate its worktree; renumber ABI/seam ops/switch counts at that point. If `git merge-tree` shows no conflict,
>   the coordinator may merge main into the lane's worktree itself and gate that.
> - **Open follow-ups:**
>   - residential: the RG23 answer (proximity grids), then an apply lane;
>   - after the land value fix: run 88 again (save → reload), expecting `ref_warnings=0`;
>   - run 85 settled run 84: the original's own path spans it (no bias; `docs/manual-checks.md`);
>   - zones glue (M4); development glue (BDS, growth, tract and lot developers: building-development.md §9);
>     traffic glue (M5);
>   - after each merge that stages a new DLL: `tools/regress.ps1 -Tag <tag> -Dll <dll>` (all shadows at once + all
>     replacements at once, one table; `tools/run_summary.ps1 -Log <log>` summarises any single run).
> - **Coordinator gate:** `build-runs/coord-full-verify.ps1 -Engine <worktree>\engine -Tag <tag>` (cargo summed,
>   clippy, configure/build/ctest; it copies the SDK into worktrees that lack the submodule), then `rustfmt --check`
>   on the touched files, run from `engine/` (it needs `engine/rustfmt.toml`; skip `lib.rs` roots).
> - Test city backups (verified hashes) are in `build-runs/city-backups/`.

**Main is `ac809c0`. It is clean and verified, and nothing is running.** No agent is alive, no game process is
running, and `git status` is clean. The engine tree is unchanged since `42f7649`, where the coordinator verified:
- `cargo test --workspace --target i686-pc-windows-msvc`: **820 passed / 0 failed**;
- `cargo clippy --workspace --all-targets -- -D warnings`: **clean** (a new clippy error is a regression now);
- ctest: **18 / 18**.

`FC4_ABI_VERSION` is **16**. Logs are in `build-runs/`: `cargo-test-main-demandfix.txt`, `clippy-main-demandfix.txt`,
`ctest-main-demandfix.txt`.

**What the second session did** (27 commits from `1eb5dd6` to `ac809c0`):
- **The M2 exit is met.** The budget and demand replacements match the original exactly, proven by two new opt-in
  **same-process shadow differentials** (`budget_shadow`, `demand_shadow`). Each runs our core on the original's
  live inputs at the same moment, read-only, and diffs bitwise. Separate-run comparisons cannot do this, because the
  original game is not deterministic from run to run.
  - **Budget:** 5 of 5 month-ends on Spandau and 35 of 35 on Konradshohe.
  - **Demand:** 160 of 160 days and 6 of 6 modifier checks.
  - **The only difference ever found** was a 4-ulp tax-modifier order of operations. It was traced in the exe,
    specced (demand.md §8.1) and fixed (`42f7649`). Details in §5.3 and §5.3b.
- **The plumbing and flammability glue now runs in game** (§5.1b). The water network, fire rolls and the
  cross-subsystem `IsCellWatered` read all work. A **use-after-free on fire ignition** (a borrowed manager was
  released) was found by the refaudit and fixed (`95db886`).
- **Also merged:**
  - the formats lane (`b5bdf8a`, green; the game-data sweep caught an XA/LTEXT regression);
  - the lot_developer clippy cleanup (`e1ce77c`);
  - `FC4_RELEASE_CREATED` in the refaudit, for temporaries the game hands us (`a07aa6a`).
- **In-game runs 33–45** are in `docs/manual-checks.md`.

**Do next, in this order:**
1. **M3: glue for the remaining grid sims:** protection, flora, pollution, crime, aura. Use plumbing and flammability
   as the template: a month status line from day one, refaudit-clean, and Konradshohe for runs.
   - The M3 exit is "grid snapshots match within tolerance". A **grid shadow** in the style of the budget and demand
     ones (our core on the original's inputs, diff the grid) is the obvious tool, so build one per subsystem as part
     of its glue.
2. **An autotest option to dismiss or suppress advisor dialogs.** Long unattended runs keep stopping at them (runs 40,
   41 and 45; §8). This is small engine work, and it unlocks multi-year runs.
3. **Open spec questions:**
   - **DQ2:** can the CO/I economy bonuses be read? Ours assumes 0, and no mismatch has shown up.
   - **BQ1, BQ2:** budget shadow edges.
   - **RS1–RS6:** residential. RS1 is load-bearing.
   - **ZQ2–ZQ6:** zones.
   - **Flammability §13.1:** the fire manager reads our grid during hotspot fires as well.

   All are in `docs/specs/QUESTIONS.md` except the last, which is in §5.1b.
4. **Unmerged branches**, unchanged this session (§5.4b): `residential-core` (no simulator tests),
   `m6-landfill-neighbors` (test stub, neighbors not started), `m5-traffic-network` (does not compile).
5. **M1 leftovers:** save → reload, pause, emergency and all speeds. Also check `FC4HostGrid.cpp:153`, which may need
   `FC4_RELEASE_CREATED` (§5.3b).

**Waiting on the user (§7):** the manual checklist, SDK errata upstreaming (public), publishing the repo (public),
and deleting the KillaVolt clone.

**Gotchas that cost time this session** (details in §3 and §8):
- Autoload works only within the FC4 Test region.
- Spandau has no water; Konradshohe does.
- Advisor dialogs freeze the date, and the heartbeat's `hidden` field is *not* the sign of it.
- PowerShell 5.1 mangles `"` inside native-command arguments. Write commit messages to a file and use `git commit -F`,
  UTF-8 **without BOM**.
- A worktree an agent creates may start from an old commit, so have it `merge --ff-only main` first.
- Re-run every lane's tests yourself before merging; this session that caught unformatted files the agent had not
  reported.

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
- **Current Plugins state, safe for manual play (verified 2026-10-13, after run 45):**
  - DLL `EC1811B2…` with its **matching** PDB `13036349…`: main `42f7649` built by `coord-verify-lane.ps1`. Power,
    plumbing, flammability and both shadows are off by default. The previous main build is kept as the
    `.main-95db886` pair.
  - ini: `budget_default = 1` alone.
  - Backups alongside, each a matching DLL/PDB pair: `.run33-main-1eb5dd6`, `.run37-flammlog`. Older DLL-only backups
    (`.run31-lane1-prestage` etc.) and the old **mismatched** PDB (`.pdb.run33-prestage-MISMATCHED`) are kept too.
  - *(Earlier versions of this file named three different staged DLLs in §3, §5.7 and §8. This line replaces all of
    them.)*
- **Run wrapper:** writing the ini from bash breaks it (§8). The tested way to run is from PowerShell, building the ini
  string with real `` `n `` newlines and calling `& tools\ingame_run.ps1 -Ini $ini`. Afterwards, check the ini bytes for
  backticks and kill any orphaned game process.
- **Autoload only works within the region the game starts in (FC4 Test).** A city in another region is refused
  (`no city of the current region has the save`). FC4 Test has its own copy of `City - Konradshohe.sc4` (`864E0A88…`),
  which **has a water system** (2 producers, 403 pipes). Spandau has none. Use Konradshohe for plumbing and fire work.
- The run history (through run 45) and the manual checklist for the user are in `docs/manual-checks.md`.

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
  - **The budget differential is DONE: ours matches the original exactly** (run 39, the shadow differential, merged
    `6ca4b65`). All 12 tax types, total tax, income, expense and net are identical over five month-ends, on the same
    lots at the same moment (§5.3).
  - **Demand matches bit-for-bit too** (the demand shadow, merged `a07aa6a`, ABI 15 → 16; runs 42/43): 320 of 320
    daily updates identical across 173 nodes, the active values and the neutral rate. One 4-ulp tax-modifier month
    (run 43) was traced to the operation order and fixed in `42f7649`. Run 44 is bit-identical everywhere.
    **The M2 exit is met** (§5.3b).
- **M3:**
  - All ten grid-sim cores are done.
  - Land value glue is done (off by default).
  - **Power glue is merged (`63e88f1`) and runs clean** — runs 27/28, `AUTOTEST VERDICT: clean`, production 5997→6000,
    409 cells powered, 0 unpowered (§5.1). The crash was a cross-thread `RefCell` race in the clock, not power.
  - **Plumbing + flammability glue merged** (`9f741de`) **and now run in game** (runs 33–38, §5.1b). It found and fixed
    one real bug: a use-after-free on fire ignition (`95db886`). `FC4_ABI_VERSION` is now 16 (15 for the budget shadow `6ca4b65`, 16 for the demand shadow `a07aa6a`), and the plugin switch
    count is 57.
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
- **M6:** `engine/formats` was merged green on 2026-10-13 (`b5bdf8a`, §5.4b). The `exemplar.rs` rows of spec §11 are
  still open. Specs landed:
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

> **Second session, 2026-10-13 (evening).** Merged to `main`:
> - `e1ce77c`: the lot_developer clippy cleanup.
> - `b5bdf8a`: the formats lane.
> - `95db886`: the flammability fire fix and its month status line.
>
> **Later, merged `6ca4b65` (the budget shadow differential, ABI 15).** Main is verified there: cargo 814/0, ctest 18/18,
> clippy clean (`cargo-test-main-shadow.txt`, `ctest-main-shadow-merge.txt`, `clippy-main-shadow.txt`).
>
> **Main was verified at `95db886`:** `cargo test --workspace` 811 passed / 0 failed. `cargo clippy --workspace
> --all-targets -- -D warnings` exits 0 (**the first time it is clean**). `ctest` 18 of 18. Evidence is in
> `build-runs/`: `cargo-test-main-b5bdf8a.txt`, `clippy-main-b5bdf8a.txt` and `ctest-main-firefix-merge.txt`. The fire
> merge touched only `engine/plugin` and `engine/tests`, so the cargo numbers still hold.
> In-game runs 33–38 are recorded in `manual-checks.md`.

### 5.1b Plumbing + flammability in game — WORKING, one use-after-free found and fixed
- **Plumbing (runs 33, 34, 37):**
  - On Spandau, which has no water system, the passes advance and nothing re-enters.
  - On Konradshohe (2 producers, 403 pipes), watered cells (729 → 702) equal building cells every month, consumption
    equals demand, and production holds at about 39.8k.
  - A full `Shutdown` path ran clean in run 34.
- **Flammability (runs 37, 38).** It now logs a `flammability month:` line with `cycles`, `endOfCycles`, `fireRolls`,
  `ignited=ok/calls`, `wateredQueries/watered/noPlumbing` and `nested=grid/flammable/hotspot/average`.
  - Fire rolls come about every 40 days.
  - The cross-subsystem `IsCellWatered` read is live: 22.8k queries, about a third watered, `noPlumbing=0`.
- **The bug (run 37, fixed `95db886`):** `IgniteCell` and `SetFireCause` each released the *borrowed* disaster
  manager, so every ignition dropped two references the game owned and freed the fire object under it. Run 38 went
  through a real ignition and a full `Shutdown` with **0 reference warnings**. Test-seam op 9 in `plugin_load_test`
  pins it.
- **Explained, not bugs:**
  - `average=0` for the first ~40 days after a load is spec §14.1 quirk 1.
  - `reentrant=1` after a fire is the game's fire manager reading our grid once during `IgniteCell` (`nested=1/0/0/0`).
    It is read-only and benign, but spec §13.1 only documents that read for probability-mode fires, and this was a
    hotspot fire. That is a small spec gap worth an RE look.
  - The NEGATIVE per-class lines in a simulator-Shutdown refaudit are a window artifact: the budget Shutdown summary
    resets the counters just before. Trust the per-object `over-release` / `released last reference` warnings instead.
- **Still unverified:** neighbour deals (no neighbour connections in the test cities), pipe bursts and distress, the
  disaster path, save → reload with plumbing on, and the effect of an ignition on the burning city beyond the first
  frames.

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

### 5.3 Budget differential (M2 exit) — DONE: ours matches the original exactly (runs 39/41)
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
- **Steps (2) and (3) are done (runs 35 and 36, same DLL `5A295D1C…`, Spandau, only `disable = simulator,budget` vs
  `disable = simulator` differing).** Full table in `manual-checks.md`.
  - **Expenses:** every department identical in all five months.
  - **Totals:** funds after five months are 71,265 (original, run 35), 71,325 (original, run 29) and 71,298 (ours).
  - **Per-type tax is not comparable across separate runs.** Two runs of the *original* from the same save differ by
    up to 4 in types 4, 6 and 9 in month 1, even with identical month-1 population. The one value outside that spread
    is **type 3: ours 26, original 20 in both runs.**
  - **Observer artifact:** at the 2001-12 → 2002-01 boundary the month snapshot sometimes fires before the collection
    (runs 28 and 35 repeat December).
- **The shadow differential is DONE, and the budget matches exactly (run 39, merged `6ca4b65`, ABI 14 → 15).**
  - **How it works:** with `disable = budget` and `budget_shadow = 1`, our simulator delivers SimNewMonth to the
    original budget's month agent. Just before that delivery, our shared tax scan (`core/src/budget/tax.rs`, called
    through `fc4_budget_shadow_tax`) runs over the live lots through a read-only host table. The original's per-type
    results are then read back through its getters and diffed.
  - **Result:** on Spandau, five month-ends, every value identical, `mismatches=0`. The tax-type-3 gap between runs 35
    and 36 was the original's run-to-run drift.
  - **Limits:**
    - It needs **our** simulator. With the original clock it logs INACTIVE, because hooking the month would mean
      writing to the game's agent list.
    - The income/expense columns check only how the settlement adds things up: the "ours" side uses the original's
      department expenses, which already matched in runs 35/36.
    - On January 1, income/expense can't be observed (QUESTIONS BQ2).
    - Lot order may differ, which can shift float sums by about ±1 per 1000 lots (QUESTIONS BQ1). None showed up.
  - **Done on Konradshohe as well:** run 40 matched 2 of 2 months. Run 41 matched **35 of 35 month-ends** (2026-02 →
    2028-12, two January settlements, 681–740 lots), with no mismatch line at all.

### 5.3b Demand shadow differential — merged `a07aa6a`; DEMAND NOW BIT-IDENTICAL (`42f7649`, run 44)
- **Closed:** the 4-ulp month below was our core's operation order. Every input was bit-identical; the exe
  (`0x006A6070`, 12 written-out copies, one per type) computes `r24(r24(maxRate·0.1f) · r24(1.0f/neutral))` with the
  reciprocal computed once. The RE lane wrote this into demand.md §8.1 (`616f7b7`, DQ1 answered, DQ2 about the CO/I
  bonuses still open), and the engine lane implemented it (`b759997`). Run 44: 160/160 days and 6/6 modifier checks
  bit-identical, including the formerly failing 2001-10-01. **With the budget result, the M2 exit is met.**
- **How it works:** with `demand_shadow = 1` (our simulator, the original demand), at each NewDay delivered to the
  original demand simulator, our shared propagation (`core/src/demand/shadow.rs`, `fc4_demand_shadow`) runs on the
  original's node states read just before. The original's states just after are diffed bitwise. SimIdle tax/economy
  modifier changes are compared too.
- **What it cannot read:** the CO/I economy bonuses, so ours assumes 0. A nonzero bonus would show up as an economy
  modifier mismatch on Co$$, Co$$$ and I-*; none has.
- **Results:** runs 42 and 43 had 160 of 160 days bit-exact each. Modifier checks were 6 of 6 and 5 of 6.
  - **The open item:** on 2001-10-01 in run 43, 11 tax modifiers came out orig `0.0832541287`, ours `0.0832540989`
    (4 ulp); I-R matched. The engine lane is asked whether it is the core or the shadow's sampling.
- **The lane's spec questions,** not yet in QUESTIONS.md:
  1. Is the link factor `percent·0.01`, and the `maxRate·0.1` term, computed with double or f32 constants?
  2. Can the CO/I economy bonuses be read anywhere?
  3. Is the `maxRate = neutral·10` fallback right when there is no tax history?
  4. Can a census finish inside NewDay's `DoIdle(5)`?
  5. Can `DoIdle(5)` change node min/max or region use?
- **Refaudit gained `FC4_RELEASE_CREATED`** for temporaries the game creates for us (the per-call resource key list).
  `FC4HostGrid.cpp:153` may need it too.

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
> **Superseded by §5.1b (second session):** it has now run in game and works, and the one bug it had was fixed in
> `95db886`. The original warning is kept below for history.
>
> ~~**Do not treat this as working.**~~ Per the lane's own report it had **NEVER executed in game**. run32 was the single
> attempt and produced no information at all (see the run32 lessons below). Every simulation path — the water pass,
> neighbour deals, the budget department, the grid swap, the `SetWateredState` direct call, the daily flammability
> refresh, the fire roll, and critically the cross-subsystem `IsCellWatered` read — is **unverified**. The host tests
> cover only the no-city neutral path. This is merged because it is complete, compiles, and passes every test it has;
> the in-game proof is still owed.

**Not merged, and why:**
- **formats — MERGED in the second session as `b5bdf8a`** (branch tip `9ec18da`, worktree and branch removed).
  - **The 3 failing tests:** all three were stale tests, not wrong code, and each was rewritten to assert the spec:
    first copy wins for duplicates (§2.3), an entry listed in the DIR that fails the QFS check fails the read (§2.6),
    and one bad entry fails the whole open (§2.3).
  - **The lane also fixed** a `u32` overflow panic in the hole check.
  - **The game-data sweep caught a real regression.** `is_ltext` accepted 1,302 `XA` audio clips once trailing bytes
    were ignored, so `XA` is now rejected; §6.1 marks `is_ltext` as tools-only.
  - **The sweep now matches main line for line:** 22 .dat files and 183,205 entries; 417 install saves plus 587
    Documents saves; 0 failures. It is `tests/game_data.rs`, which also runs inside `cargo test --workspace`.
  - **Coordinator-verified:** 811/0, fc4-formats clippy and fmt clean.
  - **Open:** the seven `exemplar.rs` rows of §11. Also, the index-offset check is skipped when there are 0 entries
    (§2.1 is silent on that), and 24-byte index 7.1 entries are still read with a warning, which the original refuses.
  - The history below is the pre-merge state.
- ~~**formats** (branch `lane/formats-original-compat`, tip `456dac3`, clean, 3 ahead / 36 behind main): **NOT GREEN.**~~
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
§10.2 items 2, 3 (partly) and 4, and lot-developer §12.2 Q8–Q10, both in `94d8cdf`. So the queue is: **ZQ2–ZQ6, RS1–RS6, BQ1–BQ2 (budget shadow, added in the second session), and landfill §10.2 items 1 and 3's unresolved half** (which brush sets `terrainMessageData4` to 1, 2 or 3 —
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
  recorded. *(The Plugins state that was recorded here is stale; §3 is current.)* Lane 1 staged `FreeCity4.dll` =
  `F921001D…` (tag `run28-threadfix`) at that time and reset `FreeCity4.ini` to `budget_default = 1` alone. The Spandau city file
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
    clean+merged, then `--force`. **After the second session only two worktrees remain, `m6-landfill-neighbors` and
    `residential`, both unmerged.** Historical note: `formats-compat`,
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
1. **Locate the §5.2 trigger**, i.e. what in our process raises PC to 53-bit during app init. The workaround (re-assert
   at PostAppInit) has **landed** (`63e88f1`), so this is no longer urgent. The next step is a minimal attach-only DLL.
2. ~~**Redo the §5.3 budget differential**~~ **Done:** runs 35/36, then the shadow differential (run 39, `6ca4b65`),
   which matches exactly. The next M2 item is the **demand** differential.
2b. **Glue for the remaining grid sims:** protection, flora, pollution, crime and aura. Plumbing and flammability are
   proven (§5.1b), so their pattern is the template.
3. ~~**Merge lane 1 / lot developer / zones**~~ **DONE** — lot developer `9889f65`, zones `f56e52a`, lane 1 `63e88f1`,
   each independently verified before merge. Merged main re-verified after the lane 1 merge: cargo 808/0, ctest 17/17.
4. ~~**Reset `Plugins\FreeCity4.ini`**~~ Done: `budget_default = 1` alone, with main's DLL/PDB pair staged (§3).
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
| **Grid-sim glue** (wiring cores into the game) | `ac1485cbac8eb97a9` | `m3-glue` | **MERGED `9f741de`**; worktree removed, branch deleted. Coordinator-verified: ctest 18/18, cargo 808/0. **Now run in game (second session, §5.1b)** |
| **engine/formats divergence fixes** | `a7aa00813e71c6ad5`, then a second-session agent | — | **MERGED `b5bdf8a`** (second session): 3 stale tests fixed per §11, XA/LTEXT regression caught by the sweep, 811/0; worktree and branch removed (§5.4b) |
| **lot_developer clippy cleanup** (second session) | one-shot agent | — | **MERGED `e1ce77c`**: 21 lints, with one scoped `allow`; workspace clippy is now clean |
| **Flammability observability + fire fix** (second session) | one-shot agent | — | **MERGED `95db886`**: month status line, the use-after-free fix, test-seam op 9 (§5.1b) |
| **Budget shadow differential** (second session) | one-shot agent | — | **MERGED `6ca4b65`** (ABI 15): `budget_shadow = 1` with `disable = budget`; runs 39–41 exact (§5.3) |
| **Demand shadow differential** (second session) | one-shot agent, resumed twice | — | **MERGED `a07aa6a`** (ABI 16) + the order-of-operations test `a000bd2`; also `FC4_RELEASE_CREATED` (§5.3b) |
| **Demand tax-ratio answer** (RE side, second session) | one-shot agent | main checkout | **COMMITTED `616f7b7`**: demand.md §8.1 from the exe (`0x006A6070`), §4.1 link factor; DQ1 answered, DQ2 filed |
| **Demand tax-ratio fix** (second session) | one-shot agent | — | **MERGED `42f7649`**: reciprocal-of-neutral order per §8.1; run 44 bit-identical |
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

**Second session end state (2026-10-13, evening).** Every lane of this session finished on its own, and every one was
verified and merged or committed. Their worktrees and branches were removed after a clean-and-merged check. **No agent
is running and none needs resuming.** The only worktrees left are the two unmerged branches from the first session
(`m6-landfill-neighbors`, `residential`); `m5-traffic-network` has a branch but no worktree.

The pattern that worked, and that a next coordinator can reuse:
- One clean-room engine agent per task, in an isolated worktree.
- The coordinator re-runs ctest, cargo and clippy, and checks rustfmt on touched files, before every merge.
- An in-game run after every merge that touches the plugin.
- When a difference is found, the engine lane decides whether it lies in the core or in the test harness, and turns
  it into a precise question. An RE agent answers it from the exe *into the spec*, and an engine agent implements
  from the spec. That loop closed the demand 4-ulp case in about an hour without crossing the clean-room wall.

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
- **`cargo clippy --workspace --all-targets -- -D warnings` IS green on main since `e1ce77c`** (second session). Keep it
  that way: a lane that adds a clippy error is now a regression. The note below is history. The real count was 21
  (12 in core, plus 9 in `conformance/src/lot_developer.rs` that the core errors had hidden), not 15. Also, `cargo fmt
  --check` now shows 29 pre-existing diffs on main, not 20, two of them in `core/src/lot_developer/tuning.rs`.
- ~~**`cargo clippy --all-targets -- -D warnings` is NOT green on main** — 15 errors,~~ all in
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
    *(Done in the second session: every DLL staged since run 33 went in with its own PDB, and backups are now made as
    pairs. The mismatched one is kept as `FreeCity4.pdb.run33-prestage-MISMATCHED`.)*
  - **SOLVED in the third session (runs 48–53): the harness now closes dialogs itself.** Each advisor popup calls
    `HiddenPause()` on our simulator, so **`hidden` IS the signal** (1 per open popup); the claim below that it is not
    was wrong. Esc/Enter do nothing; Close does. `tools/ingame_run.ps1` runs `tools/sc4ctl.ps1 clear` on every new
    HiddenPause after load (posted clicks, no focus, finds the Close button by its look, also the "City Saved" OK)
    and saves a PNG of each dialog. advisors.md §6.8.1 explains the mechanism; the `popup_dialogs = 0` ini switch
    (`cISC4App::SetPopupDialogsEnabled(false)`) suppresses advisor popups at the source once merged. The old notes:
  - **Advisor dialogs pause the simulation and the autotest does not undo it** (the user confirmed this on 2026-10-13).
    An advisor popup that asks for something holds the city. **The signal is the heartbeat date stopping while
    `+N OnTick` keeps counting** (with `pause 0`). `autotest_unpause` does not undo it.
    - **The `hidden` field is NOT the signal.** In run 41 it reads 1 in 79 of 83 heartbeats, including stretches where
      the date raced ahead (2026-08 → 2028-12 in about two minutes). Its meaning is not established. Don't key on it.
    - **What it cost:** run 40 sat at 2026-03-03 for its last ~400 s and covered 2 months, not the planned span. Run 41
      sat at 2026-05-03 for ~7.5 min until the dialog was dismissed by hand, then still covered 35 month-ends.
    - **How to read a run:** a frozen date with ticks still counting is an advisor pause, not a stall or a bug.
    - **For long unattended runs**, an engine-side fix is needed: an autotest option to dismiss or suppress advisor
      dialogs. Until then the user may click them away.
  - **Someone may close the game by hand during a run** (it happened in runs 34 and 38). The harness then reports
    `exited (code 0) without a verdict`. Read the log tail. An orderly `PostAppShutdown` / `DLL_PROCESS_DETACH (process
    exit)` is a manual quit, not a crash, and it usefully runs the `Shutdown` path that the autotest's TerminateProcess
    never reaches.
- **README checklist:** keep the Progress checklist in `README.md` current whenever something lands on `main`.
- **Status reports for the user:** plain language, short, and say what changed and what is next.
- **Memory:** the coordinator's notes are in
  `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\freecity4-project.md`. They cover:
  - the three allocators: game instances C/B/A, never free across them;
  - the cRZString layout;
  - autoload;
  - the Spandau test city.
