# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-22, run4 open-ended)

**Restart checkpoint, 22:17 BST:** main `f01d1ce4`, train102 landed `54feae7de3` with all10 actual
ancestors proved, real68-test gate and full current40 regression PASS. Postchecks, push and eligible cleanup
completed. Coverage **57.8/36.1/19.7/18.1**; 431 rounds, checkpoint425 recorded, next450. The delta since400
is 0.0/+0.1/0.0/0.0. After three flat trains selection changed to first published WWW/FTP, TextEdit and
lot-editor owners, settled renderer corrections and independent native controls. No partial-owner credit.

**Policy and authority:** RE/spec-writing paused; skip region (LucaSammartino through2026-11-21).
Target9 actual live workers, max3glue and max3 rounds per lane/logical family; no fourth, renamed budget or
provider workaround. Read `build-runs/run4-coordinator-state.json`, `run4-refill-queue.json`,
`merge-train/state.json`, `run4-heartbeat.json` and run guard after restart. These current files outrank
this timestamped checkpoint. State edits use `build-runs/run4_coord_lock.py`.
STOP: `python tools/lanes.py stop "<reason>"`; land ready work, commit, push, end.

**Live9/9 at 22:16:32, glue0/3:** three extra coordinator helpers are reviews/briefs, not implementation lanes.
- verify-flora-records4: final3/3, PID33348.
- verify-language-native4: final3/3, PID22964.
- core-powerline-contractfix4: final2/2, parent1 makes logical3/3, PID38384.
- core-application4: 2/3, PID12812.
- core-textures-contractfix4: final2/2, parent1 makes logical3/3, PID23844.
- core-ftp4: 2/3, PID5732.
- verify-advice-subject4: 2/3, PID26200.
- core-text-edit4: 1/3, PID21236.
- tools-build-singleflight4: 1/3, PID12444.

**Train103 running:** integration `a26ca9328d94ea6d6ca47fc71f36fccb224f9df2`, pipelinePID12384,
`build-runs/run4-train-pipeline.py train103`; status in `train103-pipeline-status.json`.
All four merged, no skips: re-model-effects-contracts4, core-view-inputcontrols4, core-snapshots4,
core-resource-editor4. Clippy clean; native build completed; actual CTest/cargo test compilation running at
22:16. No gate/regress verdict yet. Never set one by hand or restart on observation timeout. Root heartbeat
wait session7619 was live. Verify actual process identity/progress, not an old worker session number.

**Next ready train work:** verify-cmdline-native4 own clean integrated tip
`3a5313c3dce3cee8a709a69ad90e8f277188a49b`: bounded borrowed queries246/246, three clean2032/full-save/
shutdown/ref0 runs; CL1-CL4 and owned construction/reload unverified. Retained worker artifact hash starts
053BD7A0; integrated source needs the next actual train gate/regress. Run records256-258 reconciled;
next unused main train record259. tools-refill-validation4 own clean tip
`452d8b93effe32d5d67ef463c05f19ab245703f9` now ready: actual full pytest **180 passed,1 POSIX-only skipped**,
134.79s, log in its worktree `build-runs/coordinator-full-tools-2220.log`. Earlier missing-pytest failures
remain documented. Combined suite must rerun when tools land. Main's latest full suite remains161 from101.

**Refill queue:** two launchable reviewed final continuations, core-glyph-contractfix4 and
core-paths-contractfix4, each phase2/2 after parent1, logical3/3. Two waiting: core-model-effects-contractfix4
(phase at most2 after parent1) waits re-model-effects-contracts4; first core-lotconfig-editor4 waits
core-snapshots4. Both dependencies are in103. Fresh guards and normal refiller remain mandatory; waiting
briefs do not cover vacancies. RefillerPID23912 was alive; no STOP or provider pause at latest snapshot.
Resume only actual remaining rounds; don't requeue capped work or reopen unresolved native contracts.

**Errors and evidence:** overlapping same-worktree quick/all checks caused locks and overwritten logs.
Singleflight tooling owns the fix; no extra checks while an existing invocation is live. Old worker tool
handles are not durable OS processes. Glyph/paths final briefs name actual remaining clippy errors and
require one stable gate. TextEdit's match lint is fixed in current source but needs measured final validation;
retained CAPI slot42 concurrency failure remains a failure. Flora/application retained Ninja recompaction
PermissionDenied evidence; do not erase it or infer a final pass from partial builds.
Train103 sccache fallback happened910.341s after client start, matching configured900s idle plus10s drain.
Installed0.18 source supports that causal inference; no daemon debug log proves the exit cause. Local fallback
progressed and completed the build. Draft cache follow-up
`build-runs/run4-tools-build-singleflight4-sccache-followup-DRAFT-spec.json` must wait until current worker
finishes and remaining family budget is known. Existing daemon reuse ignores changed child environment;
do not kill the shared server, globally disable caching or queue this draft without review.

**Preserved gaps:** region excluded; paused geometry/lotmodels/sprites/overlay/ripples publications remain
held for contradictory old summaries or missing contracts. Only coherent existing bounded model-effects
publication entered103. No new RE/spec rewrite authorized. Current40 remains40 literal keys, no new defaults.
Native0/0, refusals, resyncs and unresolved provider/factory/typed ABI gaps remain unverified. Historical
train100 hang evidence and native-control failures stay preserved. Provider pause20:15 was cleared with
the real same-model/subscription probe beside the pause file; elapsed time is never clearance evidence.

**Next three actions:** (1) Continue actual train103 gate/regress, handle real verdict, land and prove all4
ancestors. (2) Regenerate handoff/coverage/treemap, doccheck, record actual run259, update this note,
commit/push and clean only eligible landed worktrees; then run every ready entry in the next train.
(3) Maintain9live through reviewed launchable briefs, inspect finished worker diagnostics and give bounded
same-family reliability follow-up its remaining budget. Checkpoint450 next.

**Loop:** while train running use `python tools/lane_heartbeat.py --wait-until train`, otherwise
`--wait-until backlog`, yield at least600s; act on exits0/2/3/4/5 exactly as prompt.txt says.
New heartbeat critical-read exit1 (after tooling lands) requires inspecting the explicit path/error; no healthy
snapshot or pause may be inferred. Use --fresh only immediately after actually rerunning gate/regress.
Before long waits launchable briefs cover open slots. After every train: gen_handoff_tables.py, coverage.py,
coverage_treemap.py, doccheck.py check (fix FAIL), full tools suite if tools changed, one status line, commit,
push and worktree cleanup per ops/rules.md. Every10 trains full restart note; every25 rounds coverage comparison.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 255 (Ten reviewed source and existing-contract deliveries landed without skips after actual gate and full current40 regression.); 254 (Heartbeat backlog/blocking-wait update landed in the very next train after instruction cleanup, with three existing bounded contract publications.); 253 (Instruction cleanup and three bounded core deliveries landed; no native/default or whole-owner promotion inferred. Full tools152PASS, doccheck check ok.); 252 (Current40+span8 replacement profile clean; span8 stays opt-in with SR8-2/3/4 gaps preserved.); 251 (Opt-in span8 constructed native route proven, SR8-2/3/4 preserved; no default promotion.). See `docs/manual-checks.md`.
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
| [history/progress-2026-10-14.md](history/progress-2026-10-14.md) | retired milestone history | nobody by default |
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

[ARCHITECTURE.md#roadmap](ARCHITECTURE.md#roadmap) and [coverage-report.md](coverage-report.md) describe the roadmap and measured progress.

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
