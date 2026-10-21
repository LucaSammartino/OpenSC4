# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run4 open-ended)

**train51 landed 8c5249f634:** Five-lane engine batch; span registry conflicts excluded. Full65-test gate and default38 replacement plus occupant-manager follow shadow union PASS/ref0. Occupant manager72/72 with zeroresync, zero native-driven silent moves, followed queries and native call tracking; follow mode added to house profile after actual candidate proof. Landvalue returned72/72 but prior70/72 remains under verify-landvalue49 investigation. Advisor/netrules/budget/MySim scopes partial, new contract questions published, no whole-owner credit. Tool candidate profile89tests PASS. Span conflict fixes prepared separately. Coverage44.3/34.5/16.4/17.8; rounds149 (open-ended).

**train50 landed 1e932067a6:** Two-lane docs unblock: DS42-DS44 answered, native disaster managers must be preserved because consumers cast slot7 results; global is populated by SL before layer Init. FN1-FN6 answered, whole font-owner remainder still uncredited. No ABI/runtime changes, gate/regress skipped. Landvalue train49 dropped72/72->70/72 (intrinsic deltas1-2); verify-landvalue49 dispatched with both frozen evidence sets. Target10/max3glue; fresh checks resumed as slot activity aged out. Coverage44.3/34.5/16.0/17.8; rounds147 (open-ended).

**train49 landed 07da738603:** Held construction/exemplar interaction reconciled against current main: canonical transform counter retained; both registry entries preserved and EX seam renumbered70 to avoid automata69. Full64-test gate and current38-default-plus-exemplars candidate union regress PASS, ref0. EX remains opt-in, EX46-EX50 open, replacement-side comparison earns no native-exact credit. Construction OP13-OP16 still open; no whole-owner credit. Target10; slot guard currently blocks refills. Coverage44.3/34.5/16.0/17.8; rounds142 (open-ended).

**train48 landed a92b32881d:** Two-lane docs-only unblock batch: audio ambience AA1-AA3 contracts closed; AA4 identity remains. Disaster layer replacement correctly withheld pending DS42-DS44 factory, cached global and concrete-consumer contracts. No runtime/ABI changes and no whole-owner coverage claims. Target raised to10 live lanes, max3glue; external contributor reservation changes owned by another session. Coverage44.3/34.5/16.1/17.8; rounds140 (open-ended).

**Assigned outside work** ([assignments.tsv](assignments.tsv)): the `region` subsystem is assigned to LucaSammartino
(RE specs, by PR) until 2026-11-21. `lanes.py check` refuses a lane that names `region`. Do not spawn region work.

**train47 landed aec5f2810b:** Five-lane docs-only unblock batch: MS8-MS10 allinferences resolved; BW1-BW4/BW6-BW9 contracts answered with explicit coreDIFFERS (BW5 runtime modifier test open); EF11-EF13 tick/select/emitter corrections;1555 nativebridge and r24 confirmed, constructedpixel route labelled and shippedproducerquestions open. Render3D spec stillpartial at actual3/3, parked; no wholeowner credit. No ABI/runtime changes, gate/regress skipped. Checkpoint125 committed; successful prior landed phases do not consume newlane caps. Coverage44.3/34.5/16.1/17.8; rounds130 (open-ended).

**train46 landed 58f47b0eb2:** Two-lane engine batch; exemplar registry conflict excluded instead of holding the batch. SimGrid factory promoted after clean current37+simgrids joint2032/ref0,136read/write grids and original-class reload116/116digestsref0; full64-test gate and candidate union regress PASS. SG1 remains open; no helper or shadow credit. Trip/base-texture cores credited only; lot/business-unit/retaining-wall owners withheld pendingLR1-LR5. Per-lane counters corrected for advisor/audio follow-ups: prior landed setup phases do not consume these lanes'3-round caps; no fourth round or guard override. Coverage44.3/34.5/16.1/17.8; rounds128 (open-ended).

**Run4 checkpoint125 (open-ended):**125 rounds;20 trains landed. Coverage44.3/34.4/16.1/17.6, baseline delta+7.5/+6.1/+4.0/+1.7points. 16 lanes parked;6 actual live. Train46 (simgrids + lot records) gate64/64PASS, candidate regression running; EX registry conflict excluded. Netmgr final2032/ref0/resync0 remained2/73 with21RNGstate differences, parked without exactcredit. Nextcheckpoint150.

**User policy:** no total round limit. Continue until user STOP, existing STOP, or no work passes check. Target10 live lanes (user raised for CPU/memory headroom), max3glue, max3rounds per lane. Refill immediately after slots/check pass; no fourth round or bypass of denial. Usage/rate limit pauses refills until clear. STOP: `python tools/lanes.py stop "<reason>"`, land on-disk ready work, push, end.

**Coverage after train51:**44.3/34.5/16.4/17.8; baseline36.8/28.3/12.1/15.9, delta+7.5/+6.2/+4.3/+1.9 points. 26 trains landed; 149 rounds used, no total cap. Next25-round checkpoint 150.

**Live authority:** `build-runs/run4-coordinator-state.json`, `run4-launch.py` (saved guarded Workflow,maxRounds1,ctx150k,calls150). Never use direct/unguarded agents. After compaction/restart reread this section and run lanes.py guard before spawning. Fresh slots/check --touches required, --resume for an existing worktree; deps must be on main.

**Train authority:** `build-runs/merge-train/state.json`; latest landed train51. Start next batch when2+ ready; guard and ancestry remain authoritative.

**Parked at3/3, never fourth:** verify-netmgrfix4(aed635a8,2/73exact,21missing RNGdraw batches,ABIadditionsunreviewed), glue-exemplars4(581778a9,3/3candidateclean2032ref0 butEX46-EX50open; registry conflict waits), core-construction4(f6dddaa7,v4 NO15 reader fixed; OP13-OP16 remain; actual2/3; prior occprops phase landed; remains parked pending OP13-OP16), core-netocc-records4(c5e769ae,partial NO15-NO17 records; NO14/NO18-NO20 remain; logical3/3 with netocc4), re-view3dwin4(78a7bd7c,partial input/AddGUI specs;VW2-VW6/helpers remain;preserveWT/ABI proposals), re-legacycontrol4(603ed696,45/57functions;12interpreters andLC4/LC5/LC8 remain;partialdocs queued, preserveWT), re-rzservices4(File/INI/Directory/OFStream complete; CmdLine slot10/SysServPtr incomplete), glue-terrain3(97b1c237), re-driverdraw4(partial DX7, landed32), re-editors4(partial curves, landed30), re-particles4(partial drawing, landed30), re-widgets4(partial base, landed28), re-application4(partial57/322, landed27), re-terrain4(answers landed25). Preserve parked worktrees, even when supported portions landed.

**Terrain:** crash fixed earlier, final diagnostics trace apparent stall to missing TerrainReconciliation/ConnectionsReady leaving dialogs enabled. Auto-close d8 reached2032 with refwarning1, NOT a pass. Tick/reconcile behavior, texture step7part2/TR37/Save unfinished. TR42/TR43 filed (preference width, edge comparison); no replacement credit or fourth round.

**Other pending:** simgrids landed46 after current37-key joint2032/ref0, original reload116/116 and candidate union PASS; coverage upkeep now promoting key. Netmgr finalparked2/73 despitezeroresync. Advisor final implementation and audio follow-up caps corrected: prior successfully landed phases remain in total, but do not consume this lane's3-round cap. Advisor round2, audio round3, netrules implementation round2 and occupant-manager verification active. EX registry conflict and construction transform conflict remain separate; no whole-owner credit for partial specs.

**Choosing work:** unblocked parked lanes with answered causes first (within3-round cap), then work one validation step from landing, then coverage-report subsystem bytes descending: coreyes/replaceabsent glue; specyes/coreabsent core; nonexact shadow verify (contract gap becomes a question); specabsent RE. Max3glue. Keep earlier skip of glue26 and abi/unreconciled-rows.tsv.

**After every train:** gen_handoff_tables.py, coverage.py, coverage_treemap.py; one status line here, commit/push. `run4-post-train.py` applies explicit tested maps and question-index upkeep after actual PASS+land. Remove landed nonparked worktrees only after verifying lane ancestor of main and clean state; keep branches and ignored evidence. Never remove active, unlanded or parked worktrees. C:>85% stops spawning until cleanup. Keep this section about60lines; archive older status lines to [today's history](history/handoff-2026-10-21.md).

**Main/tools:** keep checkout clean; tool changes require full tools tests and immediate separate commit. Latest89PASS. Candidate union supports --add-shadow and --add-replace.38 default replacement keys include simgrids,propdev,citysit,occmgr,propmgr,lotcfg; ports remain candidate-only.

**Runtime:** elevated game queue6slots(pid23444) restarted after Windows elevation blocked non-elevated launches. Native CLI BG_WAIT_CEILING_MS=0; qwait without--fresh, unique labels. RE preflight must GO. Never set gate verdicts by hand.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 241 (Five lanes; gate65PASS; both game profiles2032/ref0. OCC72/72, resync0, independent followed core; Landvalue72/72 (prior49 mismatch under investigation). Partial advisor/netrules/budget/MySim, no whole-owner credit.); 240 (train46 PASS; simgrid replacement promoted, trip/base-texture cores); 239 (train45 PASS; command-line core and partial URL/network records); 238 (Dates/commands cores, automata teardown ownership, opt-in tutorial/citydetail union; no exemplar candidate or default promotion); 237 (train39 PASS;8888/8 cores and network-occupant contract answers). See `docs/manual-checks.md`.
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
