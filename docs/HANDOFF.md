# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).

## 0. Current state: stop 2026-10-16 ~21:05 (user halted)

Main is `dfc95263`. The merge train (`tools/merge-train.py`, [docs/merge-train.md](merge-train.md)) is the only
writer of `main`; the merge lock is gone. Full stop note: [history](history/handoff-2026-10.md#stop-notes).

**2026-10-20 (token-efficiency session, no feature work):** the lane tooling changed (HANDOFF index, per-spec
questions, `tools/lanes.py`, `tools/re_query.py`, the `fc4-lane` workflow; see [ops/coordinating.md](ops/coordinating.md)).
`train5` was regressed (PASS; traffic shadow 55/89 vs 60/84, run-to-run noise in an uncalibrated shadow) and
**landed** (`333d314a`, all five lanes verified on main). Its QUESTIONS.md section was ported to the per-spec files.

**Resume in this order** (from the main checkout):

1. **Finish batch `train6`** (the eight lanes the train reported landed in `train3` but that are NOT on `main`).
   `integration/6` in `.claude/worktrees/train-integration` holds 7 of them, merged and operator-resolved but
   **not gated**: `glue17`, `apply15b`, `cores17`, `cores19`, `glue15`, `apply15terrain`, `glue12b1`. Left:
   **`glue12b2`** (ports): conflicts in `FC4Config.cpp` (key list: keep both; the closing `};` is shared),
   `FC4Subsystems.cpp` (includes: keep both) and `cFC4Simulator.cpp` (two shadow comments: keep both). Then write
   one `fc4.h` history line `73 = merge-train batch train6 (...)` from `build-runs/train6-abi-notes.txt` (the
   lanes' own history texts, removed from the header while resolving) and set `FC4_ABI_VERSION` to 73 in
   `fc4.h` and `capi/src/lib.rs`, then `gate`, `regress`, `land`, and check `git merge-base --is-ancestor` for
   all eight. Helpers used: `build-runs/mlane.sh`, `resolve1.py`, `keep_ours_hunks.py`, `fix_fc4h2.py` in the
   integration worktree.
2. **Coverage after train5:** regenerate `docs/coverage-report.md` and `docs/coverage-map.svg` (`coverage3`
   landed, which moves the numbers below).
3. **Then the queue:** `status` / `plan` / `merge-batch --size 12` → `gate` → `regress` → `land`. Ready: `cores20`
   (bridges), `cores23` (UI windows), `cores25` (effects manager), `re-answers27`, `glue18`, `glue22`. Uncommitted
   edits on hold: `glue19`, `glue21`, `glue24`, `re-answers27`: `git -C .claude/worktrees/<lane> status` first.
4. **Operator pass** for `glue22`, `glue23`, `glue18`, `cores20`: the train refused their unions on
   `engine/capi/include/fc4.h` / `lib.rs` / `FC4Subsystems.cpp` / `terrain/water.rs`. Resolve by hand; check that
   `git diff main -- engine/capi/include/fc4.h` drops no line of main's header.

**Coverage** (`python tools/coverage.py`, on main): 29,481 functions / 5,586,418 bytes; **32.9% specified / 21.3%
implemented / 9.5% verified / 12.1% replaced**. Never let it go down; regenerate `docs/coverage-report.md` and
`docs/coverage-map.svg` after every merge.

**Open items, ranked:**
1. `enable = automata`: `QueryInterface` answers the CLSID but never the IID, so `cSC4City+0x1E4` (IID
   `0x8A1B6647`) stays null and the city's Init faults at `0x45BF1B`. Accept the IID, return the sub-object.
2. `enable = effects`: the manager is six sub-objects (vptrs at `+0x00 +0x04 +0x08 +0x0C +0x10 +0x94`); the fault
   is a dispatch on `+0x08` past a one-interface object.
3. `enable = lotcfg`: drive the GZCOM-creatable *manager* (CLSID `0x299062C0`; `FUN_00489910` is the
   allocate-and-construct pair).
4. `enable = netmgr`/`netlot` (glue24, `wip`): crashes at PostCityInit with `tools=0, configs=0`; wire
   `create_tool` and feed the 543 type-0x21 exemplars before `Init`.
5. `enable = terrain`: needs TR21's terrain 3D view contract (`terrain.md` §15.5); `view->Init` dereferences
   `0x00B43CF0` and `0x00B43CD8` with no null check.
6. Performance: the development glue is ~3x slower per game day; the tract developer's `Grow` is 1.5-2.5x the
   original (glue27 measures it).

**Operator gotchas** (full list in [ops/coordinating.md](ops/coordinating.md#merge-train-operator-notes)): never
take a gate from `coord-verify-lane.ps1`'s exit code (it exits 0 when cmake fails); never trust the train's landed
list, check ancestry; never `git checkout --theirs -- fc4.h` (the lane's copy is stale); `build-runs/` `*>` output
is UTF-16 (`tr -d '\0'`); repoint a worktree's `engine/third_party/gzcom-dll/.git` gitdir at
`<main>/.git/modules/engine/third_party/gzcom-dll`; disk: free build caches of merged, clean worktrees only.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 163 (regress apply15b-hw2-shadow, clean (the lane's pre-train run, adopted by the train)); 162 (regress apply15b-cores, PASS, clean to 2032 (the lane's pre-train run, adopted by the train)); 161 (glue23 demolition pricing differential, clean, 501760 prices compared 0 mismatched); 160 (glue23 enable = demolition, clean, the replacement loads and initialises); 159 (glue23 shadow regression check, clean, demolition_shadow 73/73 exact); 147 (disasters shadow exact - fire 256/256, riot 352/352, no differences); 146 (regress apply15c, clean, terrain shadow 92/92); 145 (disasters shadow, riot 267/267 exact, fire 303/343 (DS38 + a deferred-spread lot lag)); 144 (disasters shadow after the riot walk fix; riot 396/396 exact, fire 212/215); 143 (disasters shadow after the resync fix, clean (one ref warning)); 142 (disasters shadow before the resync and riot-walk fixes, clean); 141 (regress voidbool, PASS, lists as run 138); 140 (regress abitidy2, PASS, lists as run 138); 139 (regress masterlist, PASS, lists as run 121: it ran at 05:51 before runs 133-138 and is recorded after them); 138 (regress rl5, regresslists5, PASS, the first lists with `terrain_shadow`; baseline `build-runs/regress-rl5.out`); 137 (regress cores15, PASS, lists as run 132); 136 (regress glue16, PASS, lists as run 132); 135 (regress glue13, PASS, lists as run 132); 134 (regress cores14c, PASS, lists as run 132); 133 (regress cores12b, PASS, lists as run 132); 132 (regress rl4, regresslists4, PASS, the first lists with `lotdev_shadow`; recorded after masterlist's regress had finished, whose record follows); 131 (regress seeding, PASS, lists as run 121); 128 (regress apply11traffic, PASS, lists as run 121); 127 (regress autoloadfix, PASS, lists as run 121); 126 (regress apply13, PASS, lists as run 121); 125 (regress apply12, PASS, lists as run 121); 124 (regress cores16, PASS); 123 (regress cores12a, FAIL: no city load; merge reverted); 122 (regress abitidy, PASS, lists as run 118); 121 (regress rl3, PASS) is the first with the traffic lists. See `docs/manual-checks.md`.
<!-- END GENERATED -->

## Where everything else is

| File | What | Who reads it |
|---|---|---|
| [ops/rules.md](ops/rules.md) | §2: clean room, never commit, public actions, commits | everyone |
| [ops/conventions.md](ops/conventions.md) | §8: cargo from `engine/`, fmt/clippy baseline, git, PowerShell, build infra | everyone; engine and tooling lanes before the first build |
| [ops/running-the-game.md](ops/running-the-game.md) | §3: regress, one run, ini keys, test cities, verdicts, hazards | coordinator; engine lanes with game runs; tooling lanes on the harness |
| [ops/lessons.md](ops/lessons.md) | §5: threading, refcounts, x87, differentials, demand (cited as "HANDOFF §5.x") | engine lanes |
| [ops/coordinating.md](ops/coordinating.md) | §6: spawning lanes, running the merge train, gate, operator notes | coordinator |
| [../.claude/workflows/fc4-lane.js](../.claude/workflows/fc4-lane.js) | the lane template (`Workflow({name: 'fc4-lane', args})`) | coordinator |
| [lane-tools.md](lane-tools.md) | `tools/lanes.py`: slots, check, brief, wait, monitor, ledger | coordinator; RE and tooling lanes (`wait`) |
| [token-ledger.md](token-ledger.md) | measured tokens per lane, merge share, tokens per coverage point | coordinator |
| [merge-train.md](merge-train.md) | how a lane lands: queue file, batches, hotspots | coordinator; every lane at its end |
| [lanes/README.md](lanes/README.md) | lane-file and run-file formats, the queue contract | every lane at its end |
| [lanes/INDEX.md](lanes/INDEX.md) | generated table: every lane's state and next steps | coordinator; a lane resuming another's work |
| [manual-checks.md](manual-checks.md) | generated in-game run log | coordinator; engine lanes comparing runs |
| [PROGRESS.md](PROGRESS.md) | milestones M0-M6 | coordinator |
| [coverage-report.md](coverage-report.md) | measured coverage | coordinator; tooling lanes |
| [specs/questions/README.md](specs/questions/README.md) | the question workflow | RE lanes; engine lanes filing questions |
| [../CONTRIBUTING.md](../CONTRIBUTING.md) | the clean-room roles (the single source) | everyone |
| [ARCHITECTURE.md](ARCHITECTURE.md), [../engine/README.md](../engine/README.md), [harness.md](harness.md) | layout, build, ini keys, switches | engine and tooling lanes |
| [re-workspace.md](re-workspace.md) | RE toolchain and workspace | RE lanes only |
| [history/handoff-2026-10.md](history/handoff-2026-10.md) | older stop notes and superseded HANDOFF text | nobody by default |

## 1. The project

Reverse engineering SimCity 4 Deluxe (GOG 1.1.641, exe SHA-256 `3bc5c7fe…a4b`) into a clean-room GPL-3.0
reimplementation that replaces the game piece by piece, OpenRCT2-style: Rust core (`engine/core`), formats
(`engine/formats`), C ABI (`engine/capi`), C++ GZCOM plugin (`engine/plugin`, `FreeCity4.dll` in the user's
`Plugins` folder), conformance vectors (`engine/conformance`), C++ tests (`engine/tests`). Specs in `docs/specs/`,
ABI facts in `docs/abi/`.

## 2-3. Rules, running the game

§2 moved to [docs/ops/rules.md](ops/rules.md); §3 to [docs/ops/running-the-game.md](ops/running-the-game.md).

## 4. Milestones

[PROGRESS.md](PROGRESS.md); keep it current at every merge. M0-M2 done, M3 exit met (run 103), M4-M6 in progress.

## 5. Lessons

§5 Lessons moved to [docs/ops/lessons.md](ops/lessons.md), same numbering (§5.1, §5.1b, §5.2, §5.3, §5.3b).

## 6. Coordinating lanes

Moved to [docs/ops/coordinating.md](ops/coordinating.md), rewritten for the merge train.

## 7. Waiting on the user

1. Two human checks: the budget window's look and the cheats (`docs/manual-checks.md`).
2. Upstream the SDK errata (public; re-run `docs/sdk-errata-draft.md` first). 3. Publish the repo (public).
4. Delete the local KillaVolt clone (`external/sc4-decomp`, `external/extracted`); download the Mac Rev A binary.
5. Keep or delete `build-runs/windown-backup-20261013/`.

## 8. Working conventions

Moved to [docs/ops/conventions.md](ops/conventions.md).
