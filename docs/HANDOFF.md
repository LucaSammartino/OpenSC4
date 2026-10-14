# Handoff (start here)

This file is for the agents that take turns on this repo, the coordinator first. Read it before you start and update
§0 before you stop. Milestones are tracked in the [README Progress checklist](../README.md#progress); the in-game run
log is [docs/manual-checks.md](manual-checks.md); history is `git log`. This file holds only the current state and the
operating knowledge that is still true.

## 0. Current state

*Last updated 2026-10-14, at the merge of `apply6c` (`4e6d6b8`) and run 103.*

- **main:** `FC4_ABI_VERSION` 35, 171 plugin switches, 658 conformance vectors. Last gate (apply6c): cargo all green,
  clippy clean, ctest 100 %. Every glue except the simulator and the budget is off by default (`enable = <key>`).
- **Milestones** (details in the README): M0, M1 and M2 done. **M3 exit met in run 103** (regress r6c): every grid
  shadow exact, including power 463/463, plumbing 179/179 and landfill 89/89, zero INCOMPLETE lines. M4: all cores
  merged; glue merged for residential, zones phase 1 and development (BDS + growth developers, opt-in), but the
  development glue fails in game (runs 101-102). M5: all three cores merged; traffic glue in a paused lane. M6: the
  landfill, neighbours, exemplar and formats work is merged.
- **Latest in-game run:** 103 (regress r6c, PASS with 16 replacements). See `docs/manual-checks.md`.

### Lanes

All lanes except `harness` are **paused** (the user, 2026-10-14: wait for the faster harness, it saves tokens).
Resume a paused lane with `SendMessage` to its agent ID; its worktree and transcript are intact. Lane prompts are in
`build-runs/lane-prompts/` (gitignored, on disk).

| Lane | Agent ID | Worktree (`.claude/worktrees/`) / branch | Reserved | State |
|---|---|---|---|---|
| harness: `sim_turbo`, `autotest_end_date`, `script = <file>` | `a313b6961fe7dfb95` | `agent-a313b6961fe7dfb95` | ABI 41, seam op 46 | running |
| development glue fixes (devglue2) | `a054ba812e30f4fd3` | `agent-a054ba812e30f4fd3` / `devglue2` | ABI 42 if needed | paused; fix list sent (below) |
| traffic glue phase 1 (M5) | `ae77339e1cc706c17` | `agent-ae77339e1cc706c17` | ABI 37, seam op 42 | paused |
| tract developer glue (M4) | `ae81a8bc4c10d6880` | `agent-ae81a8bc4c10d6880` | ABI 40, seam op 45 | paused |
| RE answers 8 (ZG17, NG, GF, AQ, BQ, FL, ZG, TS35-49, FS) | `a8519c0be49880b49` | `re-answers8` | - | paused at the FS questions (tip `1f67fe1`) |
| RE specs 9 (commercial, industrial, 24-hour clock, ...) | `a3547ddd1232ef1f1` | `re-specs9` | - | paused (tip `a82a2fd`) |

When the harness lane merges: give `tools/regress.ps1` a turbo / end-date profile, replace UI-driven tests with
`script` actions, then resume the paused lanes.

### Open items, most useful first

1. **Development glue in game** (devglue2): the shadow compared nothing (run 101: it read developer types 0x0001-3
   instead of 0x1010...0x4400); the growth developer's `Read` returns garbage; our catalogue has 91 types against the
   original's 744; C/I population stays 0 and the city goes bankrupt (run 102); 5 reference warnings on class
   `0xA9BD882D`. Also add `development` / `development_shadow` to `tools/regress.ps1`.
2. **Landfill game tiles** (LG27-LG29): every tile fails at the last step (run 103): the class object
   `0x8A22BEBE` / `0x2A15F262` is made and is an occupant, but neither it nor its property holder answers the
   exemplar-holder QI `0x0AC2B5F7`. An RE question: how does the original attach the exemplar to a new tile?
3. **Zones:** the shadow differs only by ZG17 (zones cost; with re-answers8). The dezone tool path is untested in
   game. Phase 2 (zone developer, lot manager) and the lot developer glue have not started.
4. **Traffic glue** (paused lane), then calibration; **tract glue** (paused lane).
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

- **Regression after every merge that touches the plugin** (run detached, it takes about 10 minutes):
  `tools/regress.ps1 -Tag <tag> -Dll <dll>` stages the DLL+PDB pair, then runs a shadows run and a replace run (16
  replacements) on Konradshohe and prints one table; `REGRESS <tag>: PASS/FAIL`. Read the shadow column, not only
  the verdict. `tools/run_summary.ps1 -Log <log>` summarises any single run.
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
6. Word what extracted game facts may be committed (`re/tuning/*.json` and `re/class_id_table.tsv` are committed).

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
