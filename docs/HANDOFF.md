# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run4 open-ended)
**Coordinator parked re-wingrid4:** 3/3:165/198 functions26103/32735B documented;33functions6632B plusWG3/WG4 remain. No fourth round.


**train59 landed 882cfb7380:** Six-lane engine batch full65-test clean gate and current38+citydetail regression PASS2032/ref0. Complete RZOStream/NullOStream cores credited (18functions610B); timer/otherstream/budget/powerline/model whole-owner credit withheld for declared gaps. Citydetail lifecycle verified but default withheld pending actual lotcfg preference host fix. Powerline NO21 collision renamedNO24; native model NO21-23 preserved. Span565 conflicts excluded, separately reconciled with6CAPItestsPASS. Construction actual2/3 eligible afterOP13-16 answers, finalthirdround queued; prior landed occprops setup not doublecounted. Coverage45.4/34.5/16.4/17.8; rounds188 (open-ended).

**train58 landed 6ea27ce451:** Seven-lane docs unblock: construction OP7/OP13-OP16 correct object-relative record fields and lifecycle; audio AA5-AA9 native DIFFERS; span8888/8 exact bridge ABI with constructed-only evidence. HTML HD5/HD6 and exemplarEX51 native DIFFERS answered, HD7/EX50 remain. URLQ4-Q6 and complete method inventory unblock whole-core completion. No ABI/runtime changes, gate/regress skipped. Checkpoint175 pushed at182 rounds; no unsupported coverage credit. Coverage45.4/34.5/16.4/17.8; rounds183 (open-ended).

**Run4 checkpoint175 (recorded at182):** 182 rounds, 32 trains landed; coverage45.4/34.5/16.4/17.8, baseline delta+8.6/+6.2/+4.3/+1.9 points; 18 lanes parked. Target10/max3glue; reviewed guarded refills continue, activity-based slot cap currently binds. Nextcheckpoint200.

**train57 landed 0683520f22:** Two-lane engine batch: native SetPiece NR14 applied before insertion with typed executable/vtable check; layer-only disaster Rust API staged with six tests. Unbuilt native disaster wrapper remains parkedDS46, original WT preserved. Full65-test gate and house regression PASS; no new default keys or incomplete owner coverage credit. Coverage45.4/34.5/16.4/17.8; rounds180 (open-ended).

**train56 landed 31d43eb53a:** Seven-lane engine batch PASS after compatibility initializer repair. Fullgate and default38+exemplars+span4444+span1555 union2032/ref0. Span4444 constructed198/198 and1555 realCOM-buffer158/158 exact; native-context/shipped dispatch stillunproved, no defaultpromotion. EXnative contract3/4 with EX51 open; no exact/defaultcredit. HTML HD5/HD6 and fontFN7-FN14 prevent whole-core credit. EF12/13 core fixes landed; effects-shadow no-load reproduces frozen baseline51, EF14 open. Landvalue input diagnostics plusLV1, no algorithm/sampling/golden change. NetRule setter and layer-only disaster API conflicts excluded and separately repaired. Coverage45.4/34.5/16.4/17.8; rounds177 (open-ended).

**train55 landed a6225fa2ad:** Two-lane docs unblock: BW12 proves ordinance initialdown hides only count<9, neighbour/general<=9; no guessed common bound. Native565bridge/vtable00AC1844 and caller/lock contracts published; constructed trace route explicit, SR565-5/6 runtime/leak edges open. No ABI/runtime changes, gate/regress skipped. Reviewed-brief refill daemon now runs independently of merge work; every launch still passes slots/check and saved Workflow, max10live/max3glue. Tutorial RE denial preserved pending advisor branch landing. Coverage45.4/34.5/16.4/17.8; rounds169 (open-ended).

**train54 landed f1d0961050:** Three-lane docs batch. AD7-AD10 allDIFFERS resolved (47callback order, validshared RNG/DoLink, cheatGetVoid2, cdecl Lua reload and directory getters); AD11 runtime directory value remains. ModelMaker parked3/3 at20607/34705B,26functions remain; QueryTool parked3/3 at16046/76564B,183functions remain. Partial specs preserved, both WTs kept and no whole-owner credit. No ABI/runtime changes, gate/regress skipped. Coverage45.4/34.5/16.4/17.8; rounds162 (open-ended).

**Assigned outside work** ([assignments.tsv](assignments.tsv)): the `region` subsystem is assigned to LucaSammartino
(RE specs, by PR) until 2026-11-21. `lanes.py check` refuses a lane that names `region`. Do not spawn region work.

**User policy:** no total round limit. Continue until user STOP, existing STOP, or no work passes check. Target10 live lanes (user raised for CPU/memory headroom), max3glue, max3rounds per lane. Refill immediately after slots/check pass; no fourth round or bypass of denial. Usage/rate limit pauses refills until clear. STOP: `python tools/lanes.py stop "<reason>"`, land on-disk ready work, push, end.

**Coverage after train59:**45.4/34.5/16.4/17.8; baseline36.8/28.3/12.1/15.9, delta+8.6/+6.2/+4.3/+1.9 points. 34 trains landed; 188 rounds used, no total cap. Next25-round checkpoint 200.

**Live authority:** `build-runs/run4-coordinator-state.json`; `run4-refill.py` launches reviewed briefs from `run4-refill-queue.json` through serialized `run4-launch.py` and the saved guarded Workflow. PID in run4-refill.pid; quota/errors pause refills. All slots/check/dependency/STOP/parked/3glue gates retained. After compaction reread this section and run guard before adding work. Helpers share run4_coord_lock.py; use its lock for state writes.

**Train authority:** `build-runs/merge-train/state.json`; latest landed train59. Start next batch when2+ ready; guard and ancestry remain authoritative.

**Parked at3/3, never fourth:** verify-netmgrfix4(aed635a8,2/73exact,21missing RNGdraw batches,ABIadditionsunreviewed), glue-exemplars4(581778a9,3/3candidateclean2032ref0 butEX46-EX50open; registry conflict waits), core-construction4(f6dddaa7,v4 NO15 reader fixed; OP13-OP16 remain; actual2/3; prior occprops phase landed; remains parked pending OP13-OP16), core-netocc-records4(c5e769ae,partial NO15-NO17 records; NO14/NO18-NO20 remain; logical3/3 with netocc4), re-view3dwin4(78a7bd7c,partial input/AddGUI specs;VW2-VW6/helpers remain;preserveWT/ABI proposals), re-legacycontrol4(603ed696,45/57functions;12interpreters andLC4/LC5/LC8 remain;partialdocs queued, preserveWT), re-rzservices4(File/INI/Directory/OFStream complete; CmdLine slot10/SysServPtr incomplete), glue-terrain3(97b1c237), re-driverdraw4(partial DX7, landed32), re-editors4(partial curves, landed30), re-particles4(partial drawing, landed30), re-widgets4(partial base, landed28), re-application4(partial57/322, landed27), re-terrain4(answers landed25). Preserve parked worktrees, even when supported portions landed.

**Terrain:** crash fixed earlier, final diagnostics trace apparent stall to missing TerrainReconciliation/ConnectionsReady leaving dialogs enabled. Auto-close d8 reached2032 with refwarning1, NOT a pass. Tick/reconcile behavior, texture step7part2/TR37/Save unfinished. TR42/TR43 filed (preference width, edge comparison); no replacement credit or fourth round.

**Other pending:** advisor activation and MySim contract fixes queued for round2; network model core and font/URL answers running. HTML/font/EX/effects/landvalue follow-up questions queued. NetRule setter and disaster layer API staged in train57; unfinished native disaster wrapper parked on DS46. Span traces are constructed-buffer proof, with no shipped dispatch credit. Partial owners remain uncredited.

**Choosing work:** unblocked parked lanes with answered causes first (within3-round cap), then work one validation step from landing, then coverage-report subsystem bytes descending: coreyes/replaceabsent glue; specyes/coreabsent core; nonexact shadow verify (contract gap becomes a question); specabsent RE. Max3glue. Keep earlier skip of glue26 and abi/unreconciled-rows.tsv.

**After every train:** gen_handoff_tables.py, coverage.py, coverage_treemap.py; one status line here, commit/push. `run4-post-train.py` applies explicit tested maps and question-index upkeep after actual PASS+land. Remove landed nonparked worktrees only after verifying lane ancestor of main and clean state; keep branches and ignored evidence. Never remove active, unlanded or parked worktrees. C:>85% stops spawning until cleanup. Keep this section about60lines; archive older status lines to [today's history](history/handoff-2026-10-21.md).

**Main/tools:** keep checkout clean; tool changes require full tools tests and immediate separate commit. Latest89PASS. Candidate union supports --add-shadow and --add-replace.38 default replacement keys include simgrids,propdev,citysit,occmgr,propmgr,lotcfg; ports remain candidate-only.

**Runtime:** elevated game queue6slots(pid23444) restarted after Windows elevation blocked non-elevated launches. Native CLI BG_WAIT_CEILING_MS=0; qwait without--fresh, unique labels. RE preflight must GO. Never set gate verdicts by hand.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 244 (6lanes; gate65PASS andunion2032/ref0. Only complete stream2owners18functions610B corecredited; otherpartialowners/citydetaildefault withheld. Span565 repaired separately.); 243 (NetRule setter+layer-only disaster API; fullgate65PASS; house2032/ref0. Disaster wrapper deferredDS46; no unsupported full-owner credit.); 242 (7lanes; fullgatePASS; bothprofiles2032/ref0. Constructedspan pixels exact, shippedhits unproved. EX3/4 contractpartial; no candidate defaultpromotion or incomplete-core credit.); 241 (Five lanes; gate65PASS; both game profiles2032/ref0. OCC72/72, resync0, independent followed core; Landvalue72/72 (prior49 mismatch under investigation). Partial advisor/netrules/budget/MySim, no whole-owner credit.); 240 (train46 PASS; simgrid replacement promoted, trip/base-texture cores). See `docs/manual-checks.md`.
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

## 4. Milestones

[PROGRESS.md](PROGRESS.md); keep it current at every merge. M0-M2 done, M3 exit met (run 103), M4-M6 in progress.

## Section numbers cited elsewhere

§2 → [ops/rules.md](ops/rules.md), §3 → [ops/running-the-game.md](ops/running-the-game.md), §5.x →
[ops/lessons.md](ops/lessons.md) (same numbering), §6 → [ops/coordinating.md](ops/coordinating.md), §8 →
[ops/conventions.md](ops/conventions.md).

## 7. Waiting on the user

1. **Revoke the OpenRouter key** pasted into run 2's first message. It sits in plaintext in that session's transcript.
2. Done 2026-10-21: 195 merged worktrees removed (C: 94% -> 72% full). Kept: the five parked branches above,
   `train-integration`, and three app session worktrees (`claude/*`). Their `build-runs/` text files are archived
   in `build-runs/worktree-archive-2026-10-21.tar.gz` (local only). The branches remain, locally and on origin.
3. Two human checks: the budget window's look and the cheats (`docs/manual-checks.md`).
4. Upstream the SDK errata (public; re-run `docs/sdk-errata-draft.md` first). 5. Publish the repo (public).
6. Delete the local KillaVolt clone (`external/sc4-decomp`, `external/extracted`); download the Mac Rev A binary.
7. Keep or delete `build-runs/windown-backup-20261013/`.
