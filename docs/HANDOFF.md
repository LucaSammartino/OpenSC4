# Handoff (start here)

This file is for the agents that take turns on this repo, the coordinator first. Read it before you start and update
§0 before you stop. Milestones are tracked in the [README Progress checklist](../README.md#progress); the in-game run
log is [docs/manual-checks.md](manual-checks.md); history is `git log`. This file holds only the current state and the
operating knowledge that is still true.

## 0. Current state

*Last updated 2026-10-14, at the merge of apply9d (`ccfe2b4`), after run 105.*

- **main:** `FC4_ABI_VERSION` 44, 661 conformance vectors. Last gate: merged main at `ccfe2b4` (main-9d), cargo 1746/0;
  clippy clean, ctest 44/45: `cargo_test` failed once on the timing-sensitive flora thread test under load and passes alone (fix with apply9d). Test switches `sim_turbo`, `autotest_end_date`, `script`, `close_dialogs` (engine/README). Every glue except the simulator and the budget is off by default (`enable = <key>`).
- **Milestones** (details in the README): M0, M1 and M2 done. **M3 exit met in run 103** (regress r6c): every grid
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
| apply9a: neighbours NG4-NG25, incl. NG11 (slots 32/33 return int64 in EDX:EAX; the plugin declares int32) | `a56eb55371f9ce77d` | worktree | ABI 47, seam op 48 | running |
| apply9b: zones ZG2-ZG17, landfill tiles LG27-LG29, budget BQ4/BQ7, crime CQ23 | `a2dc48d26cc52d9a2` | worktree | ABI 48, seam op 49 | running |
| apply9c: graphics formats FS4-FS14, GF1-GF9 | `a1ec5d982a5986c5a` | worktree | - | merged `f427e44`; FS15-FS17 |
| apply9d: education/health AQ1-AQ3, flora AQ7/AQ10, residential RG24, flammability FL7/FL8 | `a1a2a3d5ee6186779` | worktree | - (none used) | merged `ccfe2b4`; AQ11-AQ13, FL10-FL13 |
| RE specs 9 (commercial/industrial, civic, parks, history warehouse, 24-hour clock) | `a3547ddd1232ef1f1` | `re-specs9` | - | merged `a3599a0`; ABI rows `340ad36` |
| cores10a: history warehouse + 24-hour clock cores | `aaba26b7267188fb1` | worktree | ABI 50 | running |
| cores10b: commercial + industrial + civic buildings + park manager cores | `a195154e182349440` | worktree | ABI 51 | running |
| RE specs 10: city situation manager, demolition utility, dispatch manager, airport/seaport developers, prop developer (city detail manager if time) | `a576c1f88a67e75e5` | `re-specs10` | - | running |
| RE answers 9: DG1-DG27 (DG5 first: how a building occupant gets its exemplar), LG21-LG26, AO1-AO2, PR/RG/NB/PG, EX34-EX39, ZQ13 | `a847980b29901cc38` | `re-answers9` | - | running |
| RE specs 11: lot developer geometry (levelling, foundations, slot 39, boundary textures; Q3, Q7), graphics GF7/GF10/GF11, zones §14.3 Q2-Q8 and lot creation (for zones phase 2) | `a51611144f724e0f2` | `re-specs11` | - | running |
| lotglue: lot developer replacement (`enable = lotdev`) and shadow (`lotdev_shadow = 1`); Q4 slots wait for re-specs11 | `ab9c3d71e1a39f129` | worktree | ABI 53, seam op 52 | running |
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

See the [README Progress checklist](../README.md#progress). Keep it current at every merge.

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
  `docs/manual-checks.md`, file the lane's questions in `docs/specs/QUESTIONS.md`, tick the README.
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
