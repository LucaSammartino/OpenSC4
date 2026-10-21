# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run4 open-ended)

**train38 landed f81962bbea:** External PRs3/4 integrated with both histories after conflict and security review. Preserved newer core/shadow/replace metadata, regenerated symbol tables byte-for-byte, tools81PASS. No executable contribution or blocking security findings; path-example ambiguity filedPA3. Full gate63/63PASS; game regression skipped for unchanged runtime after Windows elevation prevented launch. Specs only, no replacement/shadow claim. Coverage44.3/33.2/16.0/17.6; rounds88 (open-ended).

**train37 landed c6dcdfe5af:** Seven-lane batch gate and full union propdev/ports regress PASS. 1555/565, file/INI and language cores added. Propdev replacement promoted. Ports remain candidate-only: PD12, saved-city Read and advisor-approved growth unresolved. Advisor map scaffolding only; AD4/AD5 pending. Renderer precision/format questions remain; no new shadow credit. Coverage44.0/33.2/16.0/17.6; rounds82 (open-ended).

**train36 landed 002ab4e0f7:** Four-lane docs batch: complete8888 and8 alpha-plane specs; EX41-EX45 factory/resource contracts answered; SP3 filed for4444 caller/ABI bridge, no guessedhook. Emulatorproof supports specifiedonly, no in-game shadowverifiedcredit. Train35 engineattempt was notlanded becausemain toolschanged; its code will beregated/regressed in a fresh batch.77tools tests passed forresolverfixee0b5350. Coverage44.0/31.0/16.0/17.3; rounds75 (open-ended).

**Run4 checkpoint75 (open-ended):**75 rounds used;12 trains landed. Coverage42.9/31.0/16.0/17.3, baseline delta+6.1/+2.7/+3.9/+1.4 points. Parked3/3: glue-terrain3, re-terrain4, re-widgets4, re-application4, re-particles4, re-editors4, re-driverdraw4, re-rzservices4. Train35 correctedRustmodule union, gatePASS/candidatepropdevregress running. Simgridsround2 joint/reloadvalidation inprogress, no promotionyet. Target6-8/max3glue; nextcheckpoint100.

**train34 landed 7bcfbaadd5:** Seven-lane batch gate and full union withcitysit PASS.4444 core complete, four exactRZ file owners and two exactlanguage owners specified; CmdLine/SysServPtr uncredited. Disaster rectangle host fix measured fire124/124,riot481/481 samecells(ref0,resync0); historical riot mismatch not reproduced. Citysit realreplacement promoted; indicator/Lua stubs remain CS12/CS13, explicitly unresolved. NM10/NM11 answered frombytes, runtimefix pending; NR13 blocks internal networkrule boundary. RZ parked3/3, preserveworktree. Coverage42.9/31.0/16.0/17.3; rounds68 (open-ended).

**train33 landed 17d49e451a:** Traffic TS67 phase-entry fix, complete565 span spec, advisor AD1-AD3 answers. Gate and fulldefault35-key union regress PASS; no shadow resync/suppression. Advisor UI null/no-op disproven, privateglobal consumers and read-only external map layout specified. Coverage42.1/30.0/15.4/16.5; rounds64 (open-ended).

**User policy:** no total round limit. Continue until user STOP, existing STOP, or no work passes check. Target6-8 live lanes (coordinator target8), max3glue, max3rounds per lane. Refill immediately after slots/check pass; no fourth round or bypass of denial. Usage/rate limit pauses refills until clear. STOP: `python tools/lanes.py stop "<reason>"`, land on-disk ready work, push, end.

**Coverage after train38:**44.3/33.2/16.0/17.6; baseline36.8/28.3/12.1/15.9, delta+7.5/+4.9/+3.9/+1.7 points. 15 trains landed; 88 rounds used, no total cap. Next25-round checkpoint 100.

**Live authority:** `build-runs/run4-coordinator-state.json`, `run4-launch.py` (saved guarded Workflow,maxRounds1,ctx150k,calls150). Never use direct/unguarded agents. After compaction/restart reread this section and run lanes.py guard before spawning. Fresh slots/check --touches required, --resume for an existing worktree; deps must be on main.

**Train authority:** `build-runs/merge-train/state.json`; train38 landed, train39 gating (8888/8 cores and NO15-NO17 answers). Start next batch immediately when2+ ready; guard and main ancestry remain authoritative.

**Parked at3/3, never fourth:** re-legacycontrol4(603ed696,45/57functions;12interpreters andLC4/LC5/LC8 remain;partialdocs queued, preserveWT), re-rzservices4(File/INI/Directory/OFStream complete; CmdLine slot10/SysServPtr incomplete), glue-terrain3(97b1c237), re-driverdraw4(partial DX7, landed32), re-editors4(partial curves, landed30), re-particles4(partial drawing, landed30), re-widgets4(partial base, landed28), re-application4(partial57/322, landed27), re-terrain4(answers landed25). Preserve parked worktrees, even when supported portions landed.

**Terrain:** crash fixed earlier, final diagnostics trace apparent stall to missing TerrainReconciliation/ConnectionsReady leaving dialogs enabled. Auto-close d8 reached2032 with refwarning1, NOT a pass. Tick/reconcile behavior, texture step7part2/TR37/Save unfinished. TR42/TR43 filed (preference width, edge comparison); no replacement credit or fourth round.

**Other pending:** eight lanes active at round90: automata reference verification, exemplar and tutorial replacement, citydetail validation, advisor UI RE, legacy-control/view3D RE round2, budgetwindow core. Simgrids has one final round after automata warnings are fixed; netmgr has one final round after NM12 RNG contract is answered. Advisor final implementation awaits AD4/AD5, only one logical round remains. Effects spawn denied overlap with view3D; do not bypass.

**Choosing work:** unblocked parked lanes with answered causes first (within3-round cap), then work one validation step from landing, then coverage-report subsystem bytes descending: coreyes/replaceabsent glue; specyes/coreabsent core; nonexact shadow verify (contract gap becomes a question); specabsent RE. Max3glue. Keep earlier skip of glue26 and abi/unreconciled-rows.tsv.

**After every train:** gen_handoff_tables.py, coverage.py, coverage_treemap.py; one status line here, commit/push. `run4-post-train.py` applies explicit tested maps and question-index upkeep after actual PASS+land. Remove landed nonparked worktrees only after verifying lane ancestor of main and clean state; keep branches and ignored evidence. Never remove active, unlanded or parked worktrees. C:>85% stops spawning until cleanup. Keep this section about60lines; archive older status lines to [today's history](history/handoff-2026-10-21.md).

**Main/tools:** keep checkout clean; tool changes require full tools tests and immediate separate commit. Latest81PASS. Candidate union supports --add-shadow and --add-replace.37 default replacement keys includepropdev,citysit,occmgr,propmgr,lotcfg; ports andsimgrids not defaults.

**Runtime:** elevated game queue6slots(pid23444) restarted after Windows elevation blocked non-elevated launches. Native CLI BG_WAIT_CEILING_MS=0; qwait without--fresh, unique labels. RE preflight must GO. Never set gate verdicts by hand.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 236 (train37 PASS; propdev replacement and four core modules); 235 (train34 PASS; citysit replacement, disaster exacttrace,4444core,RZ/language specs); 234 (train33 PASS; traffic TS67 corrected,565 complete, advisor blockers answered); 233 (train32 three-lane PASS;1555 spec and GZ queue-manager core); 232 (train31 PASS; formats read-shadow exact;4444 span spec complete). See `docs/manual-checks.md`.
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
