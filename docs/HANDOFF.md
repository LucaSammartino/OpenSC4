# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-23, run4 open-ended)

**Continue toward 100%; no STOP requested.** Previous snapshot preserved verbatim in [history/handoff-run4-2026-10-23-0128.md](history/handoff-run4-2026-10-23-0128.md). Fresh state/processes outrank this note. Main contains train106 landing `9393b6eeff9d740f5687226f87f79a627d182c59`; postchecks and restart updates pushed througha226e9db. Coverage **57.8/36.1/19.7/18.1**, seven flat trains. Selection prioritizes qualifying/landing proven bounded DBPF native transport and completing current owners, preserving every unsupported domain. Checkpoint450 measured zero change since425; next475. Next required tenth-train full restart note110.

**Capacity verified01:33:14 BST: 3/9 live, round454, zero pending/launchable, six open slots.** Persistence finished READY and is independently cleared below. Refiller28668 is alive; provider unpaused, STOPfalse, no stalled/land-lag alert. All three helper agents were used for result, refill, conflict and candidate reviews; helpers do not count among9. Canonical short train heartbeat returned actual exit4 BACKLOGLOW; no long heartbeat wait with launchable backlog below open slots.

| Lane | Actual PID | Created BST | Original family |
|---|---:|---|---|
|core-legacy-control4|12136|01:17:26|FINAL3/3|
|core-player-drive4|2664|01:22:56|FINAL3/3|
|core-scene-renderer4|16588|01:25:15|FINAL3/3|

Legacy final continues cleanf72643de after the two redundant test-binding deletions; previous5162/0 and22 fixtures passed, but actual Clippy failed and old native/final exit remainsUNKNOWN. One stable corrected-source final gate is required. Player final reuses quick3126/0 with26 fixtures and native4/4; quick terminal/cache exit remainsUNKNOWN, scoped formatting and one real final all still required. Scene final reuses quick3113/0 with13 fixtures/native4/4; formatting actually failed, terminal/cache exitUNKNOWN, final all/Clippy/CTest unrun at round2 end. Both refreshed old PID/creation/cwd/output/service-child proofs found no surviving old operation before final launch. Preserve every failure/partial result, no repeated unchanged quick or speculative source expansion. Read copied `coordinator-{player-drive,scene-renderer}-final-review.md` and Legacy reconciliation in build-runs.

**Persistence READY for next train108:** own clean `35e3ed379ec25f6b766b25fc43c5c9fefa15f713`, exact ready queue, engine35d3aa7e identical to actually gatedf32318db. Final original3/3 was report/queue only; retained5098/0 in22 summaries, all18 runtime fixtures, Clippy0/native4/4/CTest68/68 verified from raw logs, eight evidence hashes and10 exact source/test blobs. Independent01:31:01 process/cwd/creation/service/output audit found no own worker/check/writer/listener4863 and no unknowns. Preserve round2 context151k/150k queue denial and historical baselineE0433/20of68; six owners remain partial, PR1-3/PR9-13 open, KeyList separately inherited, no native/shadow/default claim. Read `coordinator-persist-runtime-ready-review.md`. No repeated checks were run for final reporting; combined integration still required.

**Current train107:** FTP + LotEditor integrated at `c18a86d83220a72cb6efc75fbdf96453ceb5550b`, clean treef1d913fd. Actual pipeline7512 created01:20:06. Gate **PASS:5262/0/0 Rust in22 summaries, Clippy0/63.7s, native189/189, CTest68/68**;11 real inherited native warnings retained, no cache-EOF diagnostic. DLL SHA256c4f3b75e656d1a69e088721a156ea7a6e1c1aae554378cfdbee0832d0414c925. Complete451064B LastTest archived under gate-logs/train107, SHA569bd26294cd203dd0b5eb3da609554f8733565cda9f0cc1d952437f66b021d0. Read `coordinator-train107-gate-review.md`. Actual regression RUNNING at01:33:14: pipeline7512→33444→PowerShell13988, game15320 created01:28:56 in slot1, exact original executable/UserDir command verified. Replacement finished clean2032/ref0/116.344s; shadows started01:28:53 and lacks final artifact/verdict. No script/save in requests. Collect existing pipeline; do not restart or manufacture acceptance. Both included deliveries are ABIfalse; current C/Rust version87. Main has only subsequent docs changes.

- FTP clean delivery `2e892b0edd404e6fce43e78614050b2c1f00ad3a`, enginee3282697 unchanged from actually gated69ab9dba. Actual5071/0,32 fixtures, Clippy0/native4/4/CTest68/68. Report preserves prior failed final lint/native-incomplete attempts and the exact root stats-client27536 interruption, which allowed the same validator to continue; cache causeUNKNOWN, no daemon/policy/check restart. Q1/3/7/9/10 and native/whole-owner/default limits retained.
- LotEditor clean delivery `f2d91859b9b38315663b09030d4005cb541c0ddc`, engine9fcb4627 unchanged from0161ff69. Independently recomputed5159/0,19 fixtures, Clippy0/native4/4/CTest68/68 and all report/log hashes. Full owner remains16 focused/36 partial/52 unsupported/5 foreign; LE3-6 withheld. Report-only tree movement does not require a gate repeat.

**DBPF skipped by107, still READY/unlanded:** clean own `f0734d29a27fbb084bdc5ada0c608f7a88ce3a86`, enginefc5c5a93 unchanged from retained source02d08687/gate304fdfba. Actual4962/0, Clippy0, CTest68/68 retained; original family3/3 exhausted. Assembler attempted all3 READY entries but refused an additive `engine/capi/include/fc4.h` end-of-header conflict. After107 lands, coordinator must immediately do strictly mechanical conflict integration onto new main, preserving all original history/exports and all3 new declarations; **no fourth worker implementation round**. Review `build-runs/coordinator-dbpf-header-conflict-review.md`: exact14-line block SHA79f08f7f, Config keys and variant→Dbpf FormatsShadow hook order, CMake registration, fresh post-land writer/drain checks. Normal next ABI-changing batch bumps once to88 if no intervening ABI train, then real combined gate/regression. Never pick whole files ours/theirs or hand-set a gate.

DBPF frozen R2 DLL3d1eee0c/PDBbc8ceb0d and all16 fixtures unchanged. Actual final matched budget-ON current40 OFF-first/ON both clean2035/ref0/full save/aftermark/shutdown; independent reads match10 constructed+153 saved records for metadata/raw/payload. **Three EX55 native-enumeration differences remain** (empty FALSE plus omitted DIR in QFS/save), along with EX54, prior exit1/ref4/unknown failures. This is bounded transport qualification, not whole file-owner/replacement/new-save reload/native enumeration equivalence. All10 attempts have runs273-282 with original paths/tags; run283 is landed106, **284 reserved107,285 reserved108,next unreserved286**. Exact clean numeric-field rg scan across113 main/worktree run directories proved285 unassigned, then reservation updated under coordinator lock; `coordinator-train108-run285-reservation-proof.json` retains command/paths/exit1/empty stdout+stderr. Reservation authority: `coordinator-dbpf-run-reservations-20261023T0106.json`; future train record still requires actual acceptance.

**Last landed train106:** source76361f39, clean tree393b1e0a, actual **5211/0/0 Rust in22 summaries, Clippy clean, native50/50, CTest68/68**; gate1980.6s, finished01:08:01. DLL SHA2562943708f58f4884c234abbee74271d19c45d91896bc07f40adc53b95f595a1c6. Standard regressionPASS01:17:15; landed01:18. Exact Textures deliveryce3bf3e3 is ancestor main; no skips. Both standard profiles clean2032/ref0/full app/service shutdown; neither scripts a save, so no full-save/reload credit. Actual106 netmgr72/73,20 resyncs,6 uncompared configuration draws, NM10=106 differences/31lines; aura71/71 and traffic71/71 retain transient-copy/RNG/undefined-trip exclusions. Read `coordinator-train106-{gate,regress}-review.md`. Complete raw LastTest.log archived under gate-logs/train106 with pinned SHA0d5813e4 before107 overwrite; root's later DLL archival attempt correctly refused the already-changed build artifact, no new106 binary was claimed. Run283 documents acceptance. Required generation/coverage/treemap/doccheck passed; tools unchanged, retain actual full-main104 **180 passed/1 POSIX skip/92.67s**. Textures clean landed unparked worktree removed after fresh process/service/game/ancestry proofs; branch and all build-runs evidence2.2MiB retained, build caches removed. Earlier105/104 facts and cleanup evidence remain in history.

**Errors and operational truth:** Glyph's01:02:32 provider pause was `Connection reset without closing handshake`, not a reported weekly usage limit. One real read-only single-call probe01:10:15-19 completedAVAILABLE/exit0; exact events/evidence beside archived resolved pause in build-runs/provider-pauses. Only that pause was cleared under lock; error rounds remain consumed. Refiller28668 started01:11:01 with actual owner lock; heartbeat's old marker23912 caused a false dead report because daemon startup never writes it. Root verified current creation/cwd/argv/open owner-lock and updated only operational marker under lock at01:17, proof `coordinator-refiller-marker-20261023-0117.json`; no duplicate dispatcher or tooling repair. Scene's exact00:21:20Z `failed to record rollout items: thread ... not found` remains an actual rollout-recording error; structured WIP/result/report/queue exists, so fatal/compiler/disappearance cause is not inferred.

Read `build-runs/coordinator-round-end-process-audit.md`: repository supervision has no normal-exit child-tree kill, no fatal-hook files exist, and both old Player/Scene CLI streams completed successfully while check commands had no completion/exit records. Player result01:19:04→absence01:20:39; Scene01:21:21→absence01:22:54 bracket disappearance only. CLI executor source is unavailable; session/job cleanup remains plausible, UNKNOWN. Player had no stderr error, so Scene's rollout error cannot explain both. No compiler failure established. Future workers must collect long commands before their final response or preserve unknown terminal outcomes and reconcile on resume; do not promise asynchronous command survival.

**Capped/held failures stay unlanded; no fourth or relabelled repair:**

- ModelEffects final `25901f8e` parked WIP: actual first CAPI slot44 test FAILED expectedwait1/got0. Later Clippy/native/CTest and nested5149/0 do not erase it. Lock/test source matches main/train105; missing B-attempt acknowledgment suggests timing but cause remains unproved. Original verify-occmgr same-name resume was DENIED already merged, no alternate child/guard bypass. Read `coordinator-occmgr-lock-sync-HELD-review.md`.
- Glyph final committed904a29cd parked WIP, dirty existing unvalidated float-literal patch preserved. Actual5093/0 and32 glyph tests but excessive_precision ClippyFAIL, native2/2 and CTest20/68 incomplete; provider reset and exact worker-described cancellation retained. No root source commit/check/ready override.
- Singleflight81e1667d WIP despite207 pytestPASS: authenticated cache-service ownership, real Rust/C++ reuse and full drainage unimplemented; requested iphlpapi probe PermissionError13/WinError31 causeUNKNOWN. Advice88d5f0fb WIP despite5040/0/68 gate and clean full-save harness: **36/321 native DIFF**, AD19-21 open. Flora0b7a17a0 WIP: native1/4 incomplete/Ctest skipped, old exitUNKNOWN, fresh reload controls both stall. Tutorial paired controls remain corrupt; Lua native callbacks/comparisons0 remains unqualified.
- New first-owner audit `coordinator-first-owner-refill-ranking-20261023-0124.md` found large particle/resource/string/runtime/RZ rows already implemented, capped or unpublished, not launchable reserve. Existing remaining native candidates lack required contracts/natural-use proof. Full MapView windows guard DENIED until LotEditor actually lands; conditional `run4-core-map-view4-FUTURE-spec.json` is not queued/launchable. Fresh full-key GO required after landing. TextEdit remains blocked by Legacy; FileBrowser depends on TextEdit. AudioAmbienceLayer full audio denial by capped HitTrack and SGLDX7 full driver denial by verify-gdrivers-native4 remain. No unchanged denial retries or narrowing. RenderProperties21 live-value differences, Ordinance explicit park/SV3 and native typed-binding holds remain. Ninja restat/parser draft is held on build-infra/singleflight, not an authorized fourth.

**Standing authority:** target9 actual workers/max3glue; max3 rounds per logical family, inherited corrections consume original budget. RE/spec-writing paused; assigned region excluded through2026-11-21. No provider/account failover, cap/guard workaround, native-oracle outcomes, coverage/default promotion or arbitrary game retry. Use actual OS identity/cwd/argv/service-owned compiler outputs and durable source/log provenance; old sessions/missing files/supervisor exit0 do not prove a gate. No cache stats RPC for liveness. Root's exact historical client-only exceptions are not general kill/restart authority; daemon/compiler/policy remain untouched. State mutations only through `build-runs/run4_coord_lock.py`. STOP outranks all targets.

**Next three actions:** (1) Collect current107 pipeline; at actual PASS land, prove both delivery ancestors, run all required postchecks/run284/status/commit/push and eligible archive/cleanup. (2) Immediately mechanically reconcile skipped DBPF onto landed main and fresh full-key-check conditional MapView; collect the four final workers without duplicate checks, queue every actually READY delivery for108. (3) Maintain honest refill selection/alerts and compare coverage at475; no long wait while reviewed launchable briefs do not cover open slots. Do not mark the open-ended goal complete or blocked while live workers/train can advance.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 283 (Bounded portable texture/material corrections landed after the actual combined5211-test Rust gate,68-test native harness and standard regression; native binding and unsupported domains remain withheld.); 272 (Three bounded portable application, power-line and path deliveries landed without skips after actual5193 Rust tests,68-test gate and standard regression; unsupported owner and native domains remain explicit.); 271 (Three reviewed verifier and tooling deliveries landed without skips after actual5142 Rust tests,68-test gate, standard regression and full combined tools validation; raw native differences retained.); 270 (Bounded language lifecycle controls passed with42 actual providers; foreign-host and open-question differences remain withheld); 259 (Four reviewed deliveries landed without skips after real5140 Rust tests,68-test gate and standard regression PASS; raw native gaps preserved.). See `docs/manual-checks.md`.
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
