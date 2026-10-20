# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).

## 0. Current state: live 2026-10-20 ~15:00 (coordinator session, user pausing)

Main is `91e94795` and pushed. Since the morning stop note (below): train6 landed and was fully verified (all
eight lanes by ancestry; cargo 3400/0, clippy clean, ctest 62/62, regress run 164 PASS both runs clean to 2032);
the effects core takes cores19's implementation and cores25's divergent copy is retired; the landing's union-glue
artifacts are fixed (capi_smoke brace, FC4Observer glued statements and includes, FC4Subsystems registry rows,
the conformance runner union with cores19's ef17 vectors); re-answers27 and re-answers32 (docs) are merged - the
PO questions are answered (the 0x88 pool is cSC4PropOccupantAutomataController's, +0x24 is
cISCExemplarPropertyHolder, the 3D seam is cSC43DPlaceableObject, no terrain re-solve, slots 5/6 are
Get/SetPropPlacementProperties). Coverage: specified 32.9% / implemented 23.1% / verified 10.5% / replaced 12.1%.

**Standing rule from the user (2026-10-20): NO NEW LANES.** Drain the queue and wind the in-flight lanes down
into main as they finish.

**In flight, resume like this:**
1. **train8** is merged onto `integration/8` (re-answers32, glue27, cores23, cores20, glue22, glue18; glue22 and
   glue18 were operator-resolved: FC4Subsystems registry + terrain-shadow unions) and regressed PASS (1 shadow
   line worse vs train6, aura 71/71). The gate still says FAIL: the last run had one flaky
   `plugin_autotest_test` under full ctest load (it passes standalone; a confirm rerun was under way at pause).
   Next session: rerun `python tools/merge-train.py gate`, then `regress`/`land` (the batch state's head is
   `4ee25a206a`, kept in build-runs/merge-train/state.json). Verify ancestry for all six lanes after the land.
2. **glue19** (branch `glue19`, tip `243f82ff`): the automata QI IID fix is committed and the classes registered;
   remaining: the wrapper wiring steps (2)-(5) of docs/lanes/glue19.md. Resume its lane, no new spawn.
3. **glue21** (tip `baf55a41`): GLUE21-1/2 implemented over the re-answers31 answers; remaining: the in-game
   re-run of glue21-gate-1 with the shadows exact, then the merge-queue entry. Resume, no new spawn.
4. **glue24** (tip `592a9c35`): create_tool and the 543 type-0x21 exemplars are wired; remaining: the
   enable = netmgr,netlot run to 2032, the save round trip, §8.6 props/flora, and the regress shadows list.
   Resume, no new spawn.
5. After those three land, the queue's remaining READY lanes (glue27 partially in train8; nothing else new) are
   done unless the user re-opens lane spawning. re-answers27's 25 pending ABI rows still need a conformance gate
   (build-runs/pending-abi/re-answers27-rows.tsv).

**Operator notes for this session's failures** (new, beyond [ops/coordinating.md](ops/coordinating.md)):
- The resolver's unions can glue a closing `*/` off the ABI history line so `#define FC4_ABI_VERSION` lands
  inside the comment ("undeclared identifier" everywhere); check the line ends with `*/` before the define.
- Two lanes took seam op 59 (citysit shadow, glue18 terrain glue): renumber the newer to the next free op and
  check `kFC4Seam* = n` for duplicates whenever a merge adds seam ops.
- fc4.h's tail can come out with a duplicated asserts block; diff the tail against main's before gating.
- A flaky `plugin_autotest_test` under parallel ctest (passes standalone): rerun once before bisecting.

**Coverage** (`python tools/coverage.py`, on main): 29,481 functions / 5,586,418 bytes; **32.9% specified / 23.1%
implemented / 10.5% verified / 12.1% replaced**. Never let it go down; regenerate the report and map after every
merge.

**Open items, ranked:**
1. `enable = automata` (glue19, tip 243f82ff): QI fix committed; finish the wrappers + the host tables.
2. `enable = propmgr` / `lotcfg` (glue21, tip baf55a41): implemented, needs the clean in-game run.
3. `enable = netmgr`/`netlot` (glue24, tip 592a9c35): wired, needs the clean run and the save round trip.
4. `enable = terrain`: needs TR21's terrain 3D view contract (`terrain.md` §15.5).
5. Performance: the development glue is ~3x slower per game day; the tract developer's `Grow` is 1.5-2.5x the
   original (glue27 measures it).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 182 (glue27 base at equal pacing (frame budget 5 ms), clean, 58.29 days/s); 181 (glue27 traffic replacement + its shadow self-check, clean and exact to 2032); 180 (glue27 tract replacement + its shadow self-check, clean; styles and record exact); 179 (glue27 tract + development after the speed fixes, clean; UpdatePopulation 11.3x -> 7.7x); 178 (glue27 tract + development replace, clean, 51.89 days/s); 177 (glue27 tract replace alone, clean, 50.98 days/s); 176 (glue27 development replace after the speed fixes, clean, 68.89 days/s); 175 (glue27 development at equal pacing after the speed fixes, clean, 61.21 days/s); 174 (glue27 development at equal pacing (frame budget 5 ms), clean, 53.58 days/s); 173 (glue27 development replace, clean, 43.92 days/s); 172 (glue27 base (all original), clean, 29.90 days/s); 171 (regress glue22-effects-crash, FAIL (the lane's pre-train diagnosis run, adopted by the train)); 170 (regress glue22-shadow-hook, clean (the lane's pre-train run, adopted by the train)); 169 (regress glue22-shadow-drawpass, clean (the lane's pre-train run, adopted by the train)); 168 (regress glue22-effects-shadow, clean (the lane's pre-train run, adopted by the train)); 167 (regress glue18-shadow-1, clean (the lane's pre-train run, adopted by the train)); 166 (regress glue18-repl-9, corrupt (the lane's pre-train run, adopted by the train)); 165 (regress glue18-latency-1, clean (the lane's pre-train run, adopted by the train)); 164 (regress train6-main, PASS, both runs clean to 2032; the train6 batch's post-merge gate on main (cargo 3400/0, clippy clean, ctest 62/62)); 163 (regress apply15b-hw2-shadow, clean (the lane's pre-train run, adopted by the train)); 162 (regress apply15b-cores, PASS, clean to 2032 (the lane's pre-train run, adopted by the train)); 161 (glue23 demolition pricing differential, clean, 501760 prices compared 0 mismatched); 160 (glue23 enable = demolition, clean, the replacement loads and initialises); 159 (glue23 shadow regression check, clean, demolition_shadow 73/73 exact); 147 (disasters shadow exact - fire 256/256, riot 352/352, no differences); 146 (regress apply15c, clean, terrain shadow 92/92); 145 (disasters shadow, riot 267/267 exact, fire 303/343 (DS38 + a deferred-spread lot lag)); 144 (disasters shadow after the riot walk fix; riot 396/396 exact, fire 212/215); 143 (disasters shadow after the resync fix, clean (one ref warning)); 142 (disasters shadow before the resync and riot-walk fixes, clean); 141 (regress voidbool, PASS, lists as run 138); 140 (regress abitidy2, PASS, lists as run 138); 139 (regress masterlist, PASS, lists as run 121: it ran at 05:51 before runs 133-138 and is recorded after them); 138 (regress rl5, regresslists5, PASS, the first lists with `terrain_shadow`; baseline `build-runs/regress-rl5.out`); 137 (regress cores15, PASS, lists as run 132); 136 (regress glue16, PASS, lists as run 132); 135 (regress glue13, PASS, lists as run 132); 134 (regress cores14c, PASS, lists as run 132); 133 (regress cores12b, PASS, lists as run 132); 132 (regress rl4, regresslists4, PASS, the first lists with `lotdev_shadow`; recorded after masterlist's regress had finished, whose record follows); 131 (regress seeding, PASS, lists as run 121); 128 (regress apply11traffic, PASS, lists as run 121); 127 (regress autoloadfix, PASS, lists as run 121); 126 (regress apply13, PASS, lists as run 121); 125 (regress apply12, PASS, lists as run 121); 124 (regress cores16, PASS); 123 (regress cores12a, FAIL: no city load; merge reverted); 122 (regress abitidy, PASS, lists as run 118); 121 (regress rl3, PASS) is the first with the traffic lists. See `docs/manual-checks.md`.
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
