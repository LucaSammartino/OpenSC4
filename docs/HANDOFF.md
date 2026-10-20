# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).

## 0a. Since the pause: efficiency work landed (2026-10-20 evening) - read before restarting lanes

The user wants to restart at full throttle on these changes. Why: the day's $184.71 OpenRouter bill was 85% agents
re-reading cached context (lanes started every call at 172k tokens and grew to 450-970k). See
[ops/efficiency-log.md](ops/efficiency-log.md).

**Landed on main:**
- **`3975a42e`:** main builds again (train10's two merge breaks). The automata sweep's pin is now 0, per spec §12
  quirk 10: Read keeps the streamed `+0x0C`. **This resolves train11's gate failure in item 1 below:** 14 -> 0 is
  the correct result. Re-cut integration/11 on main and re-gate it.
- **Lean lanes (`0e1ebcca`):**
  - Agent types `fc4-lane` and `fc4-lane-re` with a minimal tool list (probe: first call 7.4k tokens vs 39k for a
    default subagent, subscription route).
  - Stateless lanes: a `## Handoff` section in docs/lanes/<lane>.md, a restart in a fresh agent past 150k context
    or after an API error.
  - Quiet wrappers in `.claude/lane-bin/`: `qbuild`, `qgrep`, `qwait` (one blocking wait, no polling).
- **Forge (`b8628bfe`):** `tools/forge.py` writes a class from its spec in a scripted loop with no agent. The
  pilot did History, Parks and Dispatch for $0.41 in total; History and Parks passed their class tests, and Parks
  ran clean in game. See [ops/forge-pilot.md](ops/forge-pilot.md). Use it for classes that have class tests.
  Backends: `openrouter` (key in `%USERPROFILE%\.config\fc4-forge\openrouter.key`, a test key) or `claude-code`
  (the user's subscription through the `claude` CLI, about 1.3k tokens of overhead).
- **Build and gate (`445ed0ac`, `4c601fac`):**
  - sccache for C++ and Rust, /Z7 debug info. A fresh-worktree build + ctest went from 304 s to 168 s.
  - `fc4verify.py quick` for lane checks, reached as `qbuild.py quick`; a one-line plugin edit is checked in 3 s.
  - The train's `land` now requires a parsed post-merge gate record for the exact head. A hand-set `gate.ok` no
    longer lands anything.
  - `engine/tests/abi_entries.inc` is untracked again.

**Restart checklist:**
1. **Start a NEW coordinator session in the main checkout.** Agent types load only at session start; an old
   session's lanes fall back to every tool and 172k context (fc4-lane.js logs it).
2. **Check the first lane's first-call context** in its transcript, or with `.claude/lane-bin/ctx.py`. On the GLM
   route it should be far below 172k. If it is still near 172k, turn off the Chrome, computer-use and browser
   connectors for the coordinator session (docs/ops/coordinating.md "Lane context").
3. **Re-gate train11** on the new main (item 1 below); `land` refuses without the gate record.
4. **Track the day:** `python tools/spend.py <openrouter csv> --coverage-delta <pts>` adds a row to
   ops/efficiency-log.md. The baseline is $46 per implemented point.
5. The "no new lanes" rule (item 5 below) was the pause rule. The user said to restart at full throttle once
   back: confirm with them, then fill the slots.

**Restart state (2026-10-20 ~21:45, new coordinator session in the main checkout):** checklist done: new session
started; first-call context of the first lanes measured **7.8k tokens** (`ctx.py glue21fix2 --first`), the lean
harness works; the "no new lanes" rule is lifted by the user's restart order (full throttle). Done: re-answers34's
stale tip fixed (queue file now points at `0eb16695`), train10 re-cut as **cores27 only** (batch seq 10;
`3975a42e`'s automata-pin fix made the old train11 gate failure moot; glue21 re-queues when PL17 is fixed) —
gate PASS (clippy, build, ctest), regress running. Spawned 8 lanes: glue21fix2 (PL17, on the glue21 branch),
glue24fix2 (the tick-5500 int3, on the glue24 branch), re-netocc (the network occupants spec, 754 fns —
re-specs22's work is NOT recoverable: its branch had no commits and no queue file; re-do it), re-terrain3d
(TR21, terrain.md §15.5), devperf2 (UpdatePopulation 7.7x → target 2x), re-occprops (specify the occupants and
props pool, 453 fns), cores28 (MySimPool + MySimDispatch cores), glue29 (the cSC4NetworkTool replacement glue
over cores27's fc4_nettool ABI). Pending after train10 lands: land, gen_handoff_tables, coverage, push;
re-answers34 merges in the next batch.

## 0. Current state: pause 2026-10-20 ~19:10 (user closing the session)

Main is `12e79cfc`, pushed. Today landed: train6 (8 lanes), train8 (6), train9 (cores26), train10 (glue19),
re-answers27/32/33 doc merges, 25 measured ABI rows applied to interfaces.tsv (the sdk-errata slot-14 x87 claim
is wrong), and ~10 union-glue land mines fixed (see the operator notes below). Coverage: **implemented 25.3%,
verified 10.5%** (from 21.3/9.5 this morning). Token ledger: docs/token-ledger.md.

**Open state to resume (in this order):**

1. **train11 is mid-batch**: integration/11 (a23223be) = glue21's refcount fixes merged (operator-resolved),
   built on main's tip. Its gate FAILED once on the stale-`+0x0C` corpus count (14 -> 0,
   `formats/tests/automata_records.rs` asserts 14) - the same failure train10's first gate had; it was a
   real ordering difference there. Diagnose before landing: either glue21's branch regressed the
   renormalisation path or the merged tree mixed versions. State: current=train11, head a23223be,
   gate ok=False.
2. **glue21's lane result arrived** (tip `1c637e99`): the refcount chain fixed (QI AddRefs, the creation
   reference handed over, slot 20/21 rebuilt); its gate-11 run found a NEW fault (PL17, the game's own
   CreateLotObjects AV at SimCity 4.exe+0x60D4B5, ecx=0) - filed in
   docs/specs/questions/prop-lotconfig.md. Its queue entry (build-runs/merge-queue/glue21.json in the
   worktree) is still `wip`: PL17 must be fixed or accepted-as-question before it lands.
3. **glue24** (tip `b2e676f9`): runs clean to 2032 (tools=13 configs=543) and the save round trip clean;
   one intermittent debug-CRT int3 at tick ~5500 in 3 of 5 runs, no stack. Two fix rounds died on API
   errors mid-chase (a symbolizer tool is in its worktree's build-runs/). Resume the fix round or
   accept NM11 as a filed question and land as-is with the int3 documented.
4. **Finished, waiting to land (paid, not counted in coverage yet)**: re-specs22 (the network-occupants
   spec, 754 fns), cores27 (the network-tools core, ABI 75 - its queue entry says the train bumps the ABI
   once and takes history line 75), re-answers34 (EF1-EF10). Check each worktree's
   build-runs/merge-queue/*.json and merge through the train.
5. **No new lanes** unless the user says so; glue28 was halted (zero output, worktree clean at main's tip).
6. After every landing: `gen_handoff_tables.py`, `coverage.py` + `coverage_treemap.py`, push; check
   `git merge-base --is-ancestor <lane> main` for every lane the train lists as landed.

**Operator notes for union glue** (this session's list, all found the hard way): the resolver's unions can
swallow the ABI history's closing `*/` (check the line before `#define FC4_ABI_VERSION` ends with `*/`);
duplicate fc4.h's asserts tail (diff the tail against main's before gating); two lanes can take the same
seam op (grep `kFC4Seam.* = n` for duplicates); `#include` pairs, runner exec chains, and switch statements
can glue onto comment lines; conformance modules can duplicate; the gate must run with the worktree checked
out at the batch's head. A flaky plugin_autotest_test under parallel ctest passes standalone. A lane whose
transcript grows but writes no files for 45+ min is exploring, not working - kill it.

**Coverage** (`python tools/coverage.py`, on main): 29,481 functions / 5,586,418 bytes; **32.9% specified /
25.3% implemented / 10.5% verified / 12.1% replaced**. Never let it go down.

**Open items, ranked:**
1. `enable = automata` - LANDED (train10); the riot/fire host tables and U-Drive-It are wired; AT20 filed.
2. `enable = propmgr`/`lotcfg` (glue21): refcounts fixed, PL17 open, then lands.
3. `enable = netmgr`/`netlot` (glue24): clean runs + save round trip done; the int3 question open.
4. `enable = terrain`: needs TR21's terrain 3D view contract (`terrain.md` §15.5).
5. Performance: glue27 measured the development glue at 0.75x the original's cost per game day; UpdatePopulation
   is still 7.7x (glue27's report).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 194 (glue19 shadow 3, clean, ref_warnings=0; the automata shadow exact on the final tip); 193 (glue19 shadow 2, clean, ref_warnings=0; the automata shadow exact 73/73); 192 (glue19 shadow 1, clean but ref_warnings=426; found the shadow's reference-token bug); 191 (glue19 reload, clean, ref_warnings=0; the saved city round-trips under the exact shadow); 190 (glue19 save, clean, ref_warnings=0; SaveCity inside the exact shadow run, file kept for the reload); 189 (glue19 savesave, ended, ref_warnings=0; SaveCity inside the exact shadow run, 464 lines); 188 (glue19 riot 2, clean, ref_warnings=0; the shadow exact, the driver idle as expected with the original layer); 187 (glue19 riot 1, clean, ref_warnings=0; the shadow exact through a scripted riot's load); 186 (glue19 shadow 4, clean, ref_warnings=0; the automata shadow exact on the final tip); 185 (glue19 enable 1, corrupt; QueryInterface returned the wrong sub-object); 184 (glue19 enable 4, corrupt; the Init-phase fault after the records Read, so the classes stay unregistered); 183 (regress train8, PASS, both runs clean to 2032 (the batch's own regress, recorded after the land)); 182 (glue27 base at equal pacing (frame budget 5 ms), clean, 58.29 days/s); 181 (glue27 traffic replacement + its shadow self-check, clean and exact to 2032); 180 (glue27 tract replacement + its shadow self-check, clean; styles and record exact); 179 (glue27 tract + development after the speed fixes, clean; UpdatePopulation 11.3x -> 7.7x); 178 (glue27 tract + development replace, clean, 51.89 days/s); 177 (glue27 tract replace alone, clean, 50.98 days/s); 176 (glue27 development replace after the speed fixes, clean, 68.89 days/s); 175 (glue27 development at equal pacing after the speed fixes, clean, 61.21 days/s); 174 (glue27 development at equal pacing (frame budget 5 ms), clean, 53.58 days/s); 173 (glue27 development replace, clean, 43.92 days/s); 172 (glue27 base (all original), clean, 29.90 days/s); 171 (regress glue22-effects-crash, FAIL (the lane's pre-train diagnosis run, adopted by the train)); 170 (regress glue22-shadow-hook, clean (the lane's pre-train run, adopted by the train)); 169 (regress glue22-shadow-drawpass, clean (the lane's pre-train run, adopted by the train)); 168 (regress glue22-effects-shadow, clean (the lane's pre-train run, adopted by the train)); 167 (regress glue18-shadow-1, clean (the lane's pre-train run, adopted by the train)); 166 (regress glue18-repl-9, corrupt (the lane's pre-train run, adopted by the train)); 165 (regress glue18-latency-1, clean (the lane's pre-train run, adopted by the train)); 164 (regress train6-main, PASS, both runs clean to 2032; the train6 batch's post-merge gate on main (cargo 3400/0, clippy clean, ctest 62/62)); 163 (regress apply15b-hw2-shadow, clean (the lane's pre-train run, adopted by the train)); 162 (regress apply15b-cores, PASS, clean to 2032 (the lane's pre-train run, adopted by the train)); 161 (glue23 demolition pricing differential, clean, 501760 prices compared 0 mismatched); 160 (glue23 enable = demolition, clean, the replacement loads and initialises); 159 (glue23 shadow regression check, clean, demolition_shadow 73/73 exact); 147 (disasters shadow exact - fire 256/256, riot 352/352, no differences); 146 (regress apply15c, clean, terrain shadow 92/92); 145 (disasters shadow, riot 267/267 exact, fire 303/343 (DS38 + a deferred-spread lot lag)); 144 (disasters shadow after the riot walk fix; riot 396/396 exact, fire 212/215); 143 (disasters shadow after the resync fix, clean (one ref warning)); 142 (disasters shadow before the resync and riot-walk fixes, clean); 141 (regress voidbool, PASS, lists as run 138); 140 (regress abitidy2, PASS, lists as run 138); 139 (regress masterlist, PASS, lists as run 121: it ran at 05:51 before runs 133-138 and is recorded after them); 138 (regress rl5, regresslists5, PASS, the first lists with `terrain_shadow`; baseline `build-runs/regress-rl5.out`); 137 (regress cores15, PASS, lists as run 132); 136 (regress glue16, PASS, lists as run 132); 135 (regress glue13, PASS, lists as run 132); 134 (regress cores14c, PASS, lists as run 132); 133 (regress cores12b, PASS, lists as run 132); 132 (regress rl4, regresslists4, PASS, the first lists with `lotdev_shadow`; recorded after masterlist's regress had finished, whose record follows); 131 (regress seeding, PASS, lists as run 121); 128 (regress apply11traffic, PASS, lists as run 121); 127 (regress autoloadfix, PASS, lists as run 121); 126 (regress apply13, PASS, lists as run 121); 125 (regress apply12, PASS, lists as run 121); 124 (regress cores16, PASS); 123 (regress cores12a, FAIL: no city load; merge reverted); 122 (regress abitidy, PASS, lists as run 118); 121 (regress rl3, PASS) is the first with the traffic lists. See `docs/manual-checks.md`.
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
