# Handoff (start here)

This file is for the agents that take turns on this repo, the coordinator first. Read it before you start and update
§0 before you stop. Milestones are tracked in the [Progress checklist](PROGRESS.md); the in-game run
log is [docs/manual-checks.md](manual-checks.md); history is `git log`. This file holds only the current state and the
operating knowledge that is still true.

## 0. Current state

### Wind-down 2026-10-14 ~16:20 (usage limit): how to resume

Every lane was asked to stop at a clean point, commit everything on its branch (`WIP:` commits where tests did not pass)
and report its state. Worktrees are under `.claude/worktrees/` and are kept (never delete them). To resume a lane,
`SendMessage` its agent ID if it is still reachable; otherwise start a fresh lane in the same worktree with the
original prompt from `build-runs/lane-prompts/<lane>-prompt.txt` plus the "next steps" below.

| Lane | Worktree / branch | Agent | State at the wind-down | Next steps |
|---|---|---|---|---|
| zones2 (zone developer / lot manager) | `agent-a04b0cfb664b373d3` | `a04b0cfb664b373d3` | `d2a3c81`: stage 1 (capi host `fc4_zonedev`, 38 functions, 12 tests) done; merged main; cargo 1948/0, clippy clean, ctest 45/45; no game runs | fix the lot manager slot return types in `FC4Interfaces.h` (§11.3: slots 8-10 count, 17/19 float ST0); `Direct::GlobalHeapAllocate`; write `cFC4ZoneDeveloper`; register; switches; shadow; seam op 53; `zonedev_shadow` turbo run. Lot creation waits for re-specs11's zones item. Questions 1-9 in its report |
| histclock (history + clock glue) | `agent-ac7e10a873454fec7` | `ac7e10a873454fec7` | `40cc0c6`: stage 1, `cFC4HistoryWarehouse` (`enable = history`), clean in game (`hc-history-alone-1`, ref_warnings=0, 143 series); full cargo/clippy/ctest not rerun at this tip | history shadow (design in its report), then `cFC4Clock24`, the clock shadow, the save sweep mode; ABI 55 |
| re-specs10 | `re-specs10` | `a576c1f88a67e75e5` | **merged** (`26a1c51`) | apply its ABI rows: `build-runs/pending-abi/re-specs10-rows.tsv` (215 interfaces.tsv rows + allocators.md + layouts.md parts; `genrows.py`/`finalize.py` rebuild it from the specs); file its suggested changes to other specs (in its report: foundations §6.2, lot-developer slot 63, pollution +0x9FC, protection §8.3, SDK errata) |
| cmglue (commercial, industrial, civic, parks glue) | `agent-a64ddc3745fd7ef6a` | `a64ddc3745fd7ef6a` | no code yet (design only; baseline 1936/0, ctest 45/45 at `2f54919`) | restart from `build-runs/lane-prompts/cmglue-prompt.txt`; its plan: commercial/industrial classes, commercial shadow (CO$$/CO$$$ after each rebuild), civic glue with a 193-bucket status mirror (needs a hash map with erase in `FC4GameList.h`), civic shadow, parks glue, parks shadow, save sweep records. 3 questions (Lua registry node is a stub; `cIGZCommandParameterSet` rows; civic slot 7 tooltip string) |
| re-specs11 | `re-specs11` | `a51611144f724e0f2` | **merged** (`d8b55ee` §17 geometry; `91770df` lot developer Q3/Q7, GF10, GF11) | item 4 NOT started: zones.md §14.3 Q2-Q8 and the §13.1 lot constructors `0x006BF430` / `0x006C1E50` (**zones2 waits on it**); graphics-formats §8.5 tests 44+ (worked examples in its report); its QUESTIONS section (LD1 slot 50 box, GF13 NaN orientation, GF14 automata driver, ZQ14+). ABI rows: `build-runs/pending-abi/re-specs11-rows.txt`. Use the disassembly helpers in its session scratchpad folder `re11`, not the root `img.pkl` |
| traffic phase 2 | `agent-ae77339e1cc706c17` | `ae77339e1cc706c17` | `b8e858f` **WIP** (only core traffic tests ran: 138 passed): TS35-TS49 applied with vectors; the source iterator is O(1) (full-city traffic overhead gone: 26.5 days/s against 30.9 without traffic); the crime-lot ref warning fixed (`traffic-full-7` clean, ref_warnings=0); `path_info` wired; the core can be single-stepped for a shadow | run the full suite on `b8e858f`; write the shadow host, record reader and driver (`core/src/traffic_simulator/shadow.rs`), capi `traffic_shadow.rs` (ABI 52), plugin `FC4TrafficShadow`, seam op 51, runs with `traffic_shadow = 1`; merge main. TG answers from re-answers10 group 2 (when done) apply here |
| tractglue (tract developer) | `agent-ae81a8bc4c10d6880` | `ae81a8bc4c10d6880` | **merged `76bf685`** (gate tract-final: cargo 1946/0, ctest 46/46; regress `mtract` run 109 PASS): shadow exact (`tract-t6s`: 4092 calls compared, 0 mismatches; UpdatePopulation 601524 compared, 0), ours clean at normal speed to 2038 (`tract-t5r`); cargo 1946/0, clippy clean, ctest 46/46; ABI 48, seam op 45, 196 switches | done. Next: add tract to the regress replace and shadow lists. Optional: requeue `tract-t6t` (turbo replace) and `tract-t6v` (save) from its `done/` requests. 19 questions in its report (Grow is 1.5-2.5x the original; a pooled allocator for the staticlib would help all subsystems) |
| lotglue (lot developer) | `agent-ab9c3d71e1a39f129` | `ab9c3d71e1a39f129` | `96844a5` **WIP, capi does not build**: §17 geometry in core (40 core tests, 30/30 `ld` vectors) with the host trait reworked | write vectors `ld16_28`-`ld16_38`; port `capi/src/lotdev.rs` and `fc4.h` to the new trait; full tests; then the plugin class, seam op 52, shadow, runs. Questions LD-Q1..LD-Q11 in its report; ABI 53 |
| apply10a (DG answers) | `agent-a2141e71c5cc30e86` | `a2141e71c5cc30e86` | `e812753` **WIP, does not build**: core DG5/DG6 loaders and foundation slot 23; baseline runs clean (`a10a-base-dev` already has ref_warnings=0, so DG28 does not show) | capi host + `fc4.h` (ABI 56), plugin callbacks (foundation class `0x49C05C8F`, IID `0xC8FCE101`), `bd11_00` vector update, DG11 present flag, DG4 sum, citations, DG28 check, runs `a10a-after-*`. 5 questions in its report |
| apply10b (EX/GF7/PR/LG/AO) | `agent-a852c6ec244071bc2` | `a852c6ec244071bc2` | `d069ab4`: EX35 (two roundings; 0 shipped values affected), EX36 cited, EX34 strip; formats tests pass; full suite not rerun; baseline runs clean | EX38 error log, GF7 readers (`fc4-dat models`; survey in its `build-runs/apply10b/rktprobe`), EX39 empty vectors (+ budget items question), PR17/PR18, AO1 ranks, LG21 view route; full tests; runs `apply10b-*-1` |
| re-answers10 | `re-answers10` | `af850af1f5c7bf5b5` | group 1 **merged** (`622086f`): CK7 matches (spec typo), **CK14, HW6, HW7, HW12 differ** (to apply in histclock/history core) | group 2 TG1-TG22 (research notes in its report: TG1 likely differs; TG2 table of needed callbacks), then ZG18-21, NG35-44, AQ/FL, PW/FS/DG28, DG31 amounts, RN1, cores10b questions |

*Last updated 2026-10-14, at the merge of cores10b (`9f0eed2`), after run 108.*

- **main:** `FC4_ABI_VERSION` 48 (tract merge `76bf685`). Last gate: the tract lane's final gate (tract-final), cargo 1946/0;
  clippy clean, ctest 46/46. Test switches `sim_turbo`, `autotest_end_date`, `script`, `close_dialogs` (engine/README). Every glue except the simulator and the budget is off by default (`enable = <key>`).
- **Milestones** (details in [PROGRESS.md](PROGRESS.md)): M0, M1 and M2 done. **M3 exit met in run 103** (regress r6c): every grid
  shadow exact, including power 463/463, plumbing 179/179 and landfill 89/89, zero INCOMPLETE lines. M4: all cores
  merged; glue merged for residential, zones phase 1 and development (BDS + growth developers, opt-in), but the
  development glue fails in game (runs 101-102). M5: all three cores merged; traffic glue in a paused lane. M6: the
  landfill, neighbours, exemplar and formats work is merged.
- **Latest in-game run:** 103 (regress r6c, PASS with 16 replacements). See `docs/manual-checks.md`.

### Lanes

Every lane is running (resumed 2026-10-14 14:05 after an API session limit stopped them all at 13:57; resume with
`SendMessage`, and tell the lane which of its queued runs finished meanwhile). **Engine lanes test their own DLL
in game** through the run queue (`docs/harness.md`; the queue is `tools/game_queue.ps1 -Slots 6`, started detached
from the main checkout, log `build-runs/game-queue.log`) and must report request labels and verdict lines. Lane
prompts are in `build-runs/lane-prompts/` (gitignored, on disk); new engine lanes start from
`engine-lane-header.txt` + `selftest-addendum.txt` + the task. A worktree lane cannot read files outside its
worktree: paste what it needs into the message, or copy files into its `build-runs/`. Resume a stopped lane with
`SendMessage` to its agent ID.

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| harness (turbo, end date, script, save, dialog closer) | `a313b6961fe7dfb95` | `agent-a313b6961fe7dfb95` | - | done; merged `6147be6`, `0786064`, `dba09eb` |
| development glue fixes (devglue2) | `a054ba812e30f4fd3` | `agent-a054ba812e30f4fd3` / `devglue2` | - | merged `38524aa` |
| traffic glue (M5): phase 1 merged `850b567` (ABI 44, seam op 42); phase 2 running | `ae77339e1cc706c17` | `agent-ae77339e1cc706c17` | ABI 52, seam op 51 | running |
| tract developer glue (M4) | `ae81a8bc4c10d6880` | `agent-ae81a8bc4c10d6880` | ABI 45, seam op 45 | running |
| power shadow under turbo | `affee80aaa8e17945` | `agent-affee80aaa8e17945` | - | merged `c367684`: exact under turbo and at normal speed (the shadow's bookkeeping, not the core); PW9-PW11 |
| RE answers 8 | `a8519c0be49880b49` | `re-answers8` | - | merged `957e766`; ABI rows `a6e3d36` |
| apply9a: neighbours NG4-NG25, incl. NG11 (slots 32/33 return int64 in EDX:EAX; the plugin declares int32) | `a56eb55371f9ce77d` | worktree | ABI 45, seam op 48 | merged `bf0e4b1`; NG35-NG44 |
| apply9b: zones ZG2-ZG17, landfill tiles LG27-LG29, budget BQ4/BQ7, crime CQ23 | `a2dc48d26cc52d9a2` | worktree | - (none used) | merged `969527d`; ZG18-ZG21 |
| apply9c: graphics formats FS4-FS14, GF1-GF9 | `a1ec5d982a5986c5a` | worktree | - | merged `f427e44`; FS15-FS17 |
| apply9d: education/health AQ1-AQ3, flora AQ7/AQ10, residential RG24, flammability FL7/FL8 | `a1a2a3d5ee6186779` | worktree | - (none used) | merged `ccfe2b4`; AQ11-AQ13, FL10-FL13 |
| RE specs 9 (commercial/industrial, civic, parks, history warehouse, 24-hour clock) | `a3547ddd1232ef1f1` | `re-specs9` | - | merged `a3599a0`; ABI rows `340ad36` |
| cores10a: history warehouse + 24-hour clock cores | `aaba26b7267188fb1` | worktree | ABI 46 | merged `0757437`; CK7-CK15, HW5-HW12; glue not started |
| cores10b: commercial + industrial + civic buildings + park manager cores | `a195154e182349440` | worktree | ABI 47 | merged `ab3c1c3`; CM4-CM9, IN2, CB6-CB14, PK4-PK9, RN1; glue not started |
| RE specs 10: city situation manager, demolition utility, dispatch manager, airport/seaport developers, prop developer (city detail manager if time) | `a576c1f88a67e75e5` | `re-specs10` | - | running |
| RE answers 9: DG1-DG27, DG29, DG31, LG21-LG26, AO1-AO2, PR/RG/NB/PG, EX34-EX39, ZQ13, GF7 | `a847980b29901cc38` | `re-answers9` | - | merged `364894a`; ABI rows `c422501` |
| RE answers 10: CK7-CK15, HW5-HW12 (first), TG1-TG22, ZG18-ZG21, NG35-NG44, AQ11-AQ13, FL10-FL13, PW9-PW11, FS15-FS17, DG28, DG31 per-day amounts | `af850af1f5c7bf5b5` | `re-answers10` | - | running |
| RE specs 11: lot developer geometry (levelling, foundations, slot 39, boundary textures; Q3, Q7), graphics GF7/GF10/GF11, zones §14.3 Q2-Q8 and lot creation (for zones phase 2) | `a51611144f724e0f2` | `re-specs11` | - | running |
| lotglue: lot developer replacement (`enable = lotdev`) and shadow (`lotdev_shadow = 1`); Q4 slots wait for re-specs11 | `ab9c3d71e1a39f129` | worktree | ABI 53, seam op 52 | running |
| zones2: zone developer / lot manager replacement (`enable = zonedev`) and shadow (`zonedev_shadow = 1`); lot creation waits for re-specs11 | `a04b0cfb664b373d3` | worktree | ABI 54, seam op 53 | running |
| histclock: history warehouse and 24-hour clock glue (`enable = history`, `clock24`), shadows, save sweep for the corpus | `ac7e10a873454fec7` | worktree | ABI 55, seam op 54 | running |
| cmglue: commercial/industrial, civic and park glue (`enable = commercial, industrial, civic, parks`), three shadows, save sweep records | `a64ddc3745fd7ef6a` | worktree | ABI 58, seam op 57 | running |
| apply10a: development answers DG4, DG5 (no extra AddRef; DG28), DG6 (BDS foundation and wall slots 23/26/29), DG11 | `a2141e71c5cc30e86` | worktree | ABI 56, seam op 55 | running |
| apply10b: exemplar EX34/EX35/EX38/EX39, GF7 readers, protection PR17/PR18, landfill LG21, agent order AO1 | `a852c6ec244071bc2` | worktree | ABI 57, seam op 56 | running |
| harness tooling 2: regress through the queue with coordinator priority, 6 slots, queue status, clearer rejections | `aebacb404dae356d7` | worktree | - | merged `4e489e9`; queue restarted with `-Slots 6` at 14:51 |

Merged today, tooling: harness switches (`6147be6`, `0786064`), modal-dialog spec (`baa5d0a`), game slots and the
run queue (`9e0fa74`: up to 3 games at once; `regress -Parallel` 164 s, `-Turbo -Parallel` 94 s to 2032), the
dialog closer (`dba09eb`: `close_dialogs = 1`; in game it closed 5 advisor popups by button with popups on).
The traffic simulator answers TS35-TS49 (re-answers8) go to the traffic glue lane after its phase 1.

### Open items, most useful first

1. **Development glue works in game** (devglue2, merged `38524aa`): catalogue 744/744 types and 281 families, the
   shadow exact, the replacement grows like the original at equal pacing. Open: our development unit is about 3x
   slower per game day; DG28-DG31 (DG31: the original's results depend on game days per real second).
2. **Landfill game tiles** (LG27-LG29): every tile fails at the last step (run 103). Answered: the original calls
   `SetDefaultExemplar` on the holder sub-object at +0x20 (layouts.md); apply9b applies it.
3. **Zones:** the shadow differs only by ZG17 (answered; apply9b applies it). The dezone tool path is untested in
   game. Phase 2 (zone developer, lot manager) and the lot developer glue have not started.
4. **Traffic glue** phase 1 merged (`850b567`, `enable = traffic`, clean in game); phase 2 running (TS35-TS49, the
   O(N^2) lot scan, host callbacks, a traffic shadow for the M5 exit). **Tract glue** lane running.
5. Residential buzzards never trigger in Konradshohe; a growth probe on a large city is optional (attempt cost grows
   with lots).
6. **QUESTIONS.md restructure** (open index, archive answered sections): after re-answers8 merges. The audit proposal
   is in this session's notes; the short version is in §6.
7. The x87 precision trigger (what raises the main thread to 53-bit during app init) is still unknown; the 24-bit
   re-assert makes it harmless (foundations §18-§19). `setup_ghidra.ps1 -Mac` is untested.

## 1. The project

We are reverse engineering SimCity 4 Deluxe (GOG 1.1.641, exe SHA-256 `3bc5c7fe…a4b`) and building a clean-room,
GPL-3.0 reimplementation that replaces the game piece by piece, OpenRCT2-style: a Rust core (`engine/core`), file
formats (`engine/formats`), a C ABI (`engine/capi`), a C++ GZCOM plugin (`engine/plugin`, builds `FreeCity4.dll`,
loaded from the user's `Plugins` folder), JSON conformance vectors (`engine/conformance`) and C++ tests
(`engine/tests`). Specs for implementers are in `docs/specs/`, interface/ABI facts in `docs/abi/`, the layout and
roadmap in `docs/ARCHITECTURE.md`, build and ini keys in `engine/README.md`.

## 2. Rules that do not bend

- **Clean room** (the roles are defined in [CONTRIBUTING.md](../CONTRIBUTING.md)):
  - Spec writers (RE side) read the exe, Ghidra, `decomp/` and `re/`.
  - Implementers (`engine/` code) must NOT open `decomp/`, `decomp-mac/`, `external/`, `ghidra/project/`,
    `ghidra/mac/`, `ghidra/export/`, `re/`, `tools/`, `docs/subsystems/`, `docs/sim-engine.md`, or the exe.
  - Implementers may read `docs/specs/`, `docs/abi/` and the public SDK headers.
  - An agent doing engine work never reads RE material in the same task.
  - The coordinator runs the game and the tools; implementers get the logs. Engine lanes cannot read files outside
    their worktree: copy a game log into the lane's gitignored `build-runs/`.
- **Never commit** game files, decompiled output, or anything from KillaVolt's repo (no license; it ships EA assets).
- **The game install directory is read-only.**
- **Public actions need the user's explicit OK:** upstreaming SDK errata to nsgomez/gzcom-dll, publishing the repo,
  posting anywhere. Agent messages are not user approval.
- **Do not clean up or delete old git worktrees** (the user, 2026-10-14).
- **Restore the test city after any save test** (`ingame_run.ps1` does it unless `-KeepSaves`).
- **Commits:** branch `main`; end messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; never
  `--no-verify`.
- **Never bypass CAPTCHAs.** Simtropolis shows one, so the user downloads from there.
- The user granted full authority over this computer for the project's work, outside the limits above.

## 3. Running the game

- **Regression after every merge that touches the plugin** (run detached):
  `tools/regress.ps1 -Tag <tag> -Dll <dll> -Queue` submits a shadows run and a replace run (17 replacements with
  `development`; every shadow including `development_shadow`) through the run queue with coordinator priority
  (`priority = high`: they jump the lanes' requests and run side by side), turbo to 2032-01-01 on Konradshohe by
  default (`-NormalSpeed` for the old timed runs), and prints one table, `REGRESS <tag>: PASS/FAIL`, and a delta
  against the previous PASS of the same pacing (`-Baseline <tag>`; the original's results depend on days/s, DG31).
  Read the shadow column, not only the verdict. `-Parallel` (direct slots 1 and 2) only when the queue is stopped.
  Queue status: `tools/game_queue.ps1 -Status`; check a request without running it: `-Check <file>.req`. The queue
  runs with `-Slots 6` (measured safe; slots 8-9 would be E-cores); restart it with care: stop the queue, its `ingame_run` children and their games together (runs in
  flight are re-run by the new queue).
  `tools/run_summary.ps1 -Log <log>` summarises any single run.
- **One run:** `tools/ingame_run.ps1 -Label runN -AfterLoadSeconds 150 -Seconds 400 -Ini $ini`, with the ini built in
  PowerShell with real `` `n `` newlines, for example
  `autoload_city = C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Konradshohe.sc4`, `popup_dialogs = 0`,
  `autotest_unpause = 2`, `autotest_end = quit`, plus the keys under test. It writes `Plugins\FreeCity4.ini`, clears
  the log, starts the game from this shell (not elevated, so it can be killed; never use the Start-menu shortcut),
  waits for the plugin's `AUTOTEST VERDICT`, copies the log to `build-runs/<label>.log`, and restores the city.
  `-Actions "<s>=<sc4ctl command>; ..."` drives the UI (see `tools/sc4ctl.ps1`; the 3D view needs `realdrag` /
  `realclick`, posted clicks work for the UI).
- **ini keys:** `enable = a,b` / `disable = a,b` choose subsystems; `<sub>_shadow = 1` runs a same-process shadow;
  `budget_default` is the value of every game-call switch the ini leaves unset (not "all subsystems on"). Full list:
  `engine/README.md` and `engine/plugin/FC4Config.h`.
- **Test cities:** autoload works only inside the region the game starts in (FC4 Test). Konradshohe (`864E0A88…`)
  has a water system (2 producers, 403 pipes); Spandau has none. Backups with verified hashes are in
  `build-runs/city-backups/`.
- **Plugins folder:** every harness run rewrites the ini and stages its own DLL. Before manual play, stage a main build
  (DLL and PDB as a pair) and reset the ini. Read the staged DLL's hash from the log's attach line.
- **Verdicts:** `clean`, `corrupt` (a foreign first-chance AV), `stalled`, `ended` (the game ended the city itself).
  `0xC0000409` is a Rust panic (`panic = "abort"`).
- **Popups:** each advisor popup calls `HiddenPause()` on our simulator, so `hidden` in the heartbeat counts open
  popups. Esc/Enter do nothing, Close works. Use `popup_dialogs = 0` (set at ConnectionsReady, so power, plumbing and
  neighbours still connect); the harness also clears dialogs it recognises.
- **Hazards:**
  - Launch long runs detached (`Start-Process powershell -WindowStyle Hidden ...`); a background tool call stops
    after 10 minutes. PowerShell `*>` output is UTF-16: `tr -d '\000\r'` before grep.
  - Never `tail -F` the plugin log from a monitor: the orphaned tails lock `FreeCity4.log`. Poll with grep.
  - If a run aborts, check for a live `SimCity 4` process before the next one.
  - Writing the ini from bash passes `` `n `` literally; build it in PowerShell and read the bytes back.
  - A manual quit shows as `exited (code 0) without a verdict` with an orderly `PostAppShutdown` in the log.

## 4. Milestones

See the [Progress checklist](PROGRESS.md). Keep it current at every merge.

## 5. Lessons that engine comments and lane prompts cite

### 5.1 Threading
The game calls our getters from a second thread while the main thread is inside `OnTick`. Shared state that getters
read must not live in a `RefCell` (run 23's crash). Details: foundations.md §19.1-§19.3.

### 5.1b Reference counting
Never Release a pointer you did not AddRef or create (run 37: a borrowed disaster manager released on ignition freed
the fire object). Use `FC4_RELEASE_CREATED` for temporaries the game creates for us, `FC4_ACQUIRED` / `FC4_RELEASE`
for references we take. In a simulator-Shutdown refaudit, NEGATIVE per-class lines are a window artifact (the budget
summary resets the counters); trust the per-object `over-release` / `released last reference` warnings.
`Read` runs before `Init`: never size saved state from the city inside `Read`.

### 5.2 x87
The game runs its main thread at 24-bit precision; our DLL's presence raises it to 53-bit during app init, and the
plugin re-asserts 24-bit at PostAppInit (foundations §18). Declaring `float`/`double` where the game returns nothing
leaves values on the x87 stack and eventually produces NaN sizes (run 74); `docs/abi/README.md` states the return
rule. Ghidra renders the 42-byte FPU helper `0x008929E0` as an empty function, and `PreFrameWorkInit` does not
decompile: read the bytes on that path.

### 5.3 Differentials
Separate runs of the original are not deterministic, so compare with **same-process shadows**: our core runs on the
original's live inputs at the same moment, read-only, and the results are diffed bitwise (budget, demand and every
grid sim have one). Separate-run soaks (`tools/compare_soaks.ps1`) are only for long-run paths.

### 5.3b Demand
The demand shadow closed a 4-ulp difference by matching the exe's operation order (demand.md §8.1). The method that
worked: the engine lane turns a difference into a precise question, an RE lane answers it into the spec, an engine
lane implements it.

## 6. Coordinating lanes

- **Spawn** one clean-room engine agent per task, in an isolated worktree, from a prompt like those in
  `build-runs/lane-prompts/` (clean-room rules, first steps, build/test commands, shared-file rules, the task, the
  report). Reserve an ABI number and seam op per lane. A new worktree may start from an old commit: the lane runs
  `git merge main` first.
- **Gate** before every merge: `build-runs/coord-full-verify.ps1 -Engine <worktree>\engine -Tag <tag>` (cargo test
  summed over every `test result:` line, clippy `-D warnings`, CMake build + ctest; a private `_MSPDBSRV_ENDPOINT_`
  per gate). Then `rustfmt --check --edition 2021` on the touched `.rs` files from `engine/`. Flaky under load: two
  flora lock-timing tests; rerun before blaming the lane.
- **Merge order:** merge one lane, then have the next lane `git merge main` itself, or, if
  `git merge-tree --write-tree --name-only main <branch>` prints only a hash, merge main into its worktree yourself
  (`git -C <wt> -c submodule.recurse=false ...`, add files explicitly: `commit -a` fails on the worktree's submodule
  pointer). Renumber ABI / seam ops / switch counts at that point; rerun `python tests/gen_abi_entries.py` from
  `engine/` (revert `abi_entries.inc` if only line endings change). Merge with `git merge --no-ff -F <msgfile>`.
- **After the merge:** copy the DLL/PDB to `build-runs/dlls/FreeCity4-<tag>.*`, run regress, record the run in
  `docs/manual-checks.md`, file the lane's questions in `docs/specs/QUESTIONS.md`, tick docs/PROGRESS.md.
- **Pause / stop:** `TaskStop`, resume with `SendMessage`. After a hard stop, check the worktree's *unstaged* diff:
  the agent may have been writing the fix to something it had just committed.
- **QUESTIONS.md:** answered entries are struck and kept (`~~**ID**~~ ... ANSWERED`). Open entries are found with
  `grep -nE '^(- )?\*\*[A-Z]{2}[0-9]+' docs/specs/QUESTIONS.md`. Planned: move fully answered sections to
  `QUESTIONS-answered.md`, add a generated open-question index and a prefix legend (first-generation PR/PW numbers
  were reused); new questions continue the subsystem's existing prefix.

## 7. Waiting on the user

1. Two human checks: the budget window's look and the cheats (`docs/manual-checks.md`).
2. Upstream the SDK errata (public; `docs/sdk-errata-draft.md` needs a re-run first).
3. Publish the repo (public).
4. Delete the local KillaVolt clone (`external/sc4-decomp`, `external/extracted`); download the Mac Rev A binary.
5. Keep or delete `build-runs/windown-backup-20261013/` (every lane it covered has committed).

## 8. Working conventions

- **Bootstrap the harness with the engine** (the user, 2026-10-14): as more of the game is ours, use it to make the
  feedback loops short and automated (a turbo clock, plugin-side scripted actions instead of screen clicks, end
  dates instead of timeouts). Prefer a tooling switch in the plugin (off by default, logged) over UI automation.
- **Run cargo from `engine/`**, never the worktree root: there `cargo fmt --check` prints help and exits 0.
- **`fmt --check` has pre-existing diffs on main** (8 files in `capi/src` and `core/src/lot_developer/tuning.rs`).
  Compare against main's list; add none; do not mass-reformat. **Clippy is clean**; a new error is a regression.
- **Read results, not exit codes alone:** sum every `test result:` line from a captured file (never a `tail`); read
  ctest's verdict line (`0 tests failed` matches a grep for "failed").
- **Git:** prefer `git -C <path>` over `cd` (a stray `cd` once committed into another lane's worktree). Before deleting
  any branch, prove nothing on it is unique. Worktrees made by an elevated session need
  `safe.directory = C:/Users/tlori/FreeCity4/.claude/worktrees/*` (in the global git config).
- **PowerShell 5.1** mangles `"` in native arguments: write commit messages to a file, UTF-8 without BOM,
  `git commit -F`. PowerShell variable names are case-insensitive (`$a` clobbers `$A`).
- **`build-runs/` is gitignored:** gate logs, run logs, DLL copies and lane prompts live there; record the numbers in
  docs, not the files.
- **Build infra:** with many lanes building, the shared 32-bit `mspdbsrv` runs out of address space (LNK1318); the
  gate and lane scripts set a private `_MSPDBSRV_ENDPOINT_`.
- **Status reports for the user:** plain language, short, what changed and what is next.
- **Memory:** the coordinator's notes are in `C:\Users\tlori\.claude\projects\C--Users-tlori-FreeCity4\memory\`.
