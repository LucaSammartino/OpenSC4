# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-22, run4 open-ended)
**Coordinator parked re-wingrid4:** 3/3:165/198 functions26103/32735B documented;33functions6632B plusWG3/WG4 remain. No fourth round.

**train70 landed b60a7c3434:** Dual Claude/Codex subscription lane backends tested. Shared guards, rounds, parking and worktrees;115tools tests and real Codex two-call watchdog denial PASS. Full train gate/regress recorded. User requested testing only: STOP set;zero project workers started;no pilot/refill until user resumes. Claude quota pause preserved. No new runtime replacement. Coverage57.6/35.4/16.5/17.9; rounds225 (open-ended).

**train69 landed 1157a92093:** Final outsidePR5 e77f1bf1 included after its second head advance. RM7 proof cited unpublished region-city-dialogs; restored question and retained provisional RM1 timing rather than guessing. PR6/7 alreadylanded68 with full67native/97tools PASS and clean2032/ref0 union regression; security review recorded. Docs-only follow-up, no new runtime claim. Ten original-WT quota resumes, three new largest eligible fresh cores and checked effects retry queued, provider pause retained; no workers spawned,225rounds/nextcheckpoint250. Old merged-lane denial preserved. Coverage57.6/35.4/16.5/17.9; rounds225 (open-ended).

**Open-ended run resumed (2026-10-22):** User explicitly lifted testing-only STOP. Seven Codex workers launched in rounds226-232: span8888 glue, landvalue verify, budget-window and terrain-overlay cores, advisor/span-context/exemplar RE. Max7 live/max3glue/max3rounds retained; shared guard/ledger authoritative, Claude quota pause preserved. Reviewed refiller running; refill on returns. Historical held engine batch resolved63-65; terrain/netmgr holds remain, no fourth rounds.

**train68 landed f259852181:** Reviewed and merged outside PR5 lateste16ff6a8,PR6 and stackedPR7; retained histories, regenerated indexes and preserved current main. Corrected PR5 wildcard, reconciled3region aliases, closed overlay slot20/pedestrian slot158 gaps with public contracts and conformance. Explicit FreeType range extension and TerrainScrubber credit deferred CT3. Security review finds no executable/build/dependency changes or new script/credential patterns.97tools tests PASS on combined metadata; full native gate and house regression PASS before landing. New specs drive specified57.6%; other gains are attribution, no new engine/runtime replacement. Existing name rules exclude9compiler helpers860B; explicit library range unchanged. Provider weekly quota pauses refills, all WIP preserved. Coverage57.6/35.4/16.5/17.9; rounds225 (open-ended).

**train66 landed f8f69ceb5c:** Two-lane docs unblock: TM3 byte-proved default scheduler counting/destructor flag and null slot3 DIFFERS; AA4 Vehicle identity and AA11 independent move callbacks answered. AA10 no-clock compatibility policy remains open. No ABI/runtime changes or new coverage credit; gate/regress skipped for docs-only. Provider weekly quota pauses refills,225round checkpoint pushed; approved capacity fix97testsPASS. Coverage45.4/35.1/16.3/17.8; rounds225 (open-ended).

**Train65 verification gap:** landvalue71/72 (pass43,2029-07-07), verified16.3%; LV2 and preserved trace document read-timing evidence, no algorithm fitting. All10 quota-interrupted lanes preserved in original worktrees; refills paused until provider clears18:00 Europe/London. Capacity fix de1cd3b6 is applied and97testsPASS.

**Run4 checkpoint225 (recorded at225):** 225 rounds, 39 trains landed; coverage45.4/35.1/16.3/17.8, baseline delta+8.6/+6.8/+4.2/+1.9 points; 23 lanes parked. Target10/max3glue; refills paused on provider weekly limit until18:00 Europe/London, all interrupted work preserved. Approved live-worker count fix de1cd3b6 applied,97testsPASS; no guard bypass. Nextcheckpoint250.


**train63 landed e7b3f2d7dc:** Rebuilt failed62 with typed MySim shadow payload decoding and poison-callback regression test. Full66-test gate and current38+citydetail+span565 union2032/ref0 PASS. Lotcfg realpreference host fix removes implementation blocker; citydetail promoted after implemented boundary/native-compatible8byte record proof and jointcleanrun. CD6/CD7/PL22 increase/two-session reload remain testgaps. Span565 remains opt-in, no whole-native replacement credit. Original null-read failing message type was not captured; failedDLL/PDB/evidence preserved. Advisor/MySim sourcephases parked3/3 with AD12-15/MS13-15 and remaining bodies; no fourth rounds, WTs preserved. Coverage45.4/34.5/16.4/17.8; rounds208 (open-ended).

**Parked at3/3: glue-advisors-activate4.** Wrapper/CAPI/weak-drop changes compile and fullgate passes, but no registered factory or advisor game trace. AD12-AD15 plus known value/list/history/world callbacks and system Read/Write remain. No fourth round; preserve this worktree. Compiled inactive stage may be reviewed through the train, without replacement credit.

**Parked at3/3: core-mysims-contractfix4.** MS11/MS12, literal congestion position and FailCommute corrections compile/fullgatePASS. Invented CheckCommute/pathfinder methods removed; real body needsMS15, withMS13/MS14 also open. No wholepool/dispatch credit or fourthround; preserve this worktree. Compiled documented fixes may land through the train.

**Assigned outside work** ([assignments.tsv](assignments.tsv)): the `region` subsystem is assigned to LucaSammartino
(RE specs, by PR) until 2026-11-21. `lanes.py check` refuses a lane that names `region`. Do not spawn region work.

**User policy:** no total round limit. Continue until user STOP, existing STOP, or no work passes check. Target6-8 live lanes, refill up to7 initially (latest user instruction), max3glue, max3rounds per lane. Refill immediately after slots/check pass; no fourth round or bypass of denial. Usage/rate limit pauses refills until clear. STOP: `python tools/lanes.py stop "<reason>"`, land on-disk ready work, push, end.

**Coverage after train70:**57.6/35.4/16.5/17.9; baseline36.8/28.3/12.1/15.9, delta+20.8/+7.1/+4.4/+2.0 points. 43 trains landed; 225 rounds used, no total cap. Next25-round checkpoint 250.

**Live authority:** `build-runs/run4-coordinator-state.json`; tracked `tools/lane_launch.py` and `tools/lane_refill.py` use Codex subscription with shared rounds, guards and reviewed queue. Refiller PID in run4-refill.pid. Legacy build-runs launch/refill scripts remain Claude-only; Claude pause retained. All slots/check/dependency/STOP/parked/3glue gates remain. After compaction reread this section and run guard before adding work. State writes share run4_coord_lock.py.

**Train authority:** `build-runs/merge-train/state.json`; latest landed train70. Start next batch when2+ ready; guard and ancestry remain authoritative.

**Parked at3/3, never fourth:** verify-netmgrfix4(aed635a8,2/73exact,21missing RNGdraw batches,ABIadditionsunreviewed), glue-exemplars4(581778a9,3/3candidateclean2032ref0 butEX46-EX50open; registry conflict waits), core-construction4(f6dddaa7,v4 NO15 reader fixed; OP13-OP16 remain; actual2/3; prior occprops phase landed; remains parked pending OP13-OP16), core-netocc-records4(c5e769ae,partial NO15-NO17 records; NO14/NO18-NO20 remain; logical3/3 with netocc4), re-view3dwin4(78a7bd7c,partial input/AddGUI specs;VW2-VW6/helpers remain;preserveWT/ABI proposals), re-legacycontrol4(603ed696,45/57functions;12interpreters andLC4/LC5/LC8 remain;partialdocs queued, preserveWT), re-rzservices4(File/INI/Directory/OFStream complete; CmdLine slot10/SysServPtr incomplete), glue-terrain3(97b1c237), re-driverdraw4(partial DX7, landed32), re-editors4(partial curves, landed30), re-particles4(partial drawing, landed30), re-widgets4(partial base, landed28), re-application4(partial57/322, landed27), re-terrain4(answers landed25). Preserve parked worktrees, even when supported portions landed.

**Terrain:** crash fixed earlier, final diagnostics trace apparent stall to missing TerrainReconciliation/ConnectionsReady leaving dialogs enabled. Auto-close d8 reached2032 with refwarning1, NOT a pass. Tick/reconcile behavior, texture step7part2/TR37/Save unfinished. TR42/TR43 filed (preference width, edge comparison); no replacement credit or fourth round.

**Other pending:** follow the current guarded refill queue and lane reports. Complete published owner methods before credit; contract gaps remain questions. Prior merged construction/disaster branches were denied resume and remain stopped with worktrees preserved.

**Choosing work:** unblocked parked lanes with answered causes first (within3-round cap), then work one validation step from landing, then coverage-report subsystem bytes descending: coreyes/replaceabsent glue; specyes/coreabsent core; nonexact shadow verify (contract gap becomes a question); specabsent RE. Max3glue. Keep earlier skip of glue26 and abi/unreconciled-rows.tsv.

**After every train:** gen_handoff_tables.py, coverage.py, coverage_treemap.py; one status line here, commit/push. `run4-post-train.py` applies explicit tested maps and question-index upkeep after actual PASS+land. Remove landed nonparked worktrees only after verifying lane ancestor of main and clean state; keep branches and ignored evidence. Never remove active, unlanded or parked worktrees. C:>85% stops spawning until cleanup. Keep this section about60lines; archive older status lines to [today's history](history/handoff-2026-10-21.md).

**Main/tools:** keep checkout clean; tool changes require full tools tests and immediate separate commit. Latest115PASS. Candidate union supports --add-shadow and --add-replace.39 default replacement keys include simgrids,propdev,citysit,occmgr,propmgr,lotcfg; ports remain candidate-only.

**Runtime:** elevated game queue6slots(pid23444) restarted after Windows elevation blocked non-elevated launches. Native CLI BG_WAIT_CEILING_MS=0; qwait without--fresh, unique labels. RE preflight must GO. Never set gate verdicts by hand.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 247 (Gate and union regress required; exemplar opt-in7/8 no promotion; native model answers unblock core.); 246 (Full gate and union regression required; no default additions, inactive advisor wrappers only.); 245 (Typed MySim shadowpayload repair; full66gatePASS, union2032/ref0. Citydetail promoted after lotcfg realpref fix; reload/increase testgaps remain. Span565 opt-in.); 244 (6lanes; gate65PASS andunion2032/ref0. Only complete stream2owners18functions610B corecredited; otherpartialowners/citydetaildefault withheld. Span565 repaired separately.); 243 (NetRule setter+layer-only disaster API; fullgate65PASS; house2032/ref0. Disaster wrapper deferredDS46; no unsupported full-owner credit.). See `docs/manual-checks.md`.
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
