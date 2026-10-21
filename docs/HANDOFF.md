# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run 4 active)

**train27 landed 66465215fd:** Two-lane batch: application lifecycle/city/save/CLI spec (partial57/322 functions, no whole-class map credit) and traffic TS67 diagnosis/question/test. No traffic behavior change or mismatch suppression; traffic remains nonexact. Application3-round cap stays, nofourth; its landed branch clears the terrain touches blocker subject to fresh checks. Coverage38.7/30.0/12.4/16.5; rounds32/60.

**User limits:** 60 total lane rounds, 3 rounds per lane, target **6 live lanes**, at most **3 glue lanes**. Refill returned slots in the same turn after `lanes.py slots` and `lanes.py check --touches <keys>` pass (`--resume` for the original worktree). Never bypass a denial or unpark without fixing its cause. At 3 rounds without landing park and record the state; never a fourth round. At 60 set STOP with reason `run4 round limit`, land ready work, push and end. A usage/rate limit pauses refills until it clears. User STOP: set STOP immediately, land what is ready, push and end.

**Live ledger:** `build-runs/run4-coordinator-state.json` (32/60 dispatched at this snapshot); `build-runs/run4-launch.py` enforces total/per-lane/live/glue caps, slots/check and subsystem touches, then invokes the guarded saved Workflow once with maxRounds=1. Use no direct/unguarded agents. After compaction/restart reread this section and run `python tools/lanes.py guard` before spawning. Main stays clean. Tool edits require `python -m pytest -q tools/tests` and an immediate separate commit; latest 69 passed.

**Run4 checkpoint30/60:** four trains landed (21,22,25,26). Coverage **38.7/30.0/12.4/16.5** (specified/implemented/verified/replaced), versus start36.8/28.3/12.1/15.9: **+1.9/+1.7/+0.3/+0.6 points**. Next checkpoint45, then60. After every train run gen_handoff_tables.py, coverage.py, coverage_treemap.py; add a line here, commit and push. `build-runs/run4-post-train.py` performs tested owner-map updates and regenerations from an explicit JSON config, only after actual train PASS/landing. Start batches as soon as2+ lanes are ready.

**Train27 landed:** application RE + traffic verification; gate and full35-key regression PASS. Ready next: widget, web/network, GZ services and persistence specs plus network-occupant core (partial, no full-owner credit). Live target6 and actual rounds are in the ledger; main and regenerated tables are pushed.

**Explicit priorities:**
- Aura fixed,71/71; MySims73/73; netmgr72/73 with NM10/NM11 filed and unknown-ini-key warning fixed. No comparison suppression. Traffic TS67 is filed by verify-traffic4; no behavior change. Dependent RE brief is prepared, but verify-traffic4 must land before spawning it.
- Occupant manager landed train22: C++ argument evaluation read the output pointer before the query wrote it. Replacement clean2032; occmgr promoted after union regress. PL19 landed train25: pointer hash(v>>2), per-query ownership, balanced trace; propmgr/lotcfg promoted after35-key union regress. All35 keys are now defaults.
- **glue-terrain3:** run4 round2/3 used;2816e58f fixes the crash but replacement stallsJune2026. WIP, notready. Final round3 brief prepared for stall trace, texture update/TR37/Save. Its latest check was DENIED: touches terrain overlaps unmerged core-netocc4 (SetTerrainHeights); the earlier application overlap has landed. Do not resume until core-netocc4 lands and fresh slots/check pass. TR33-TR41 answers already landed train25; never guess missing contracts.
- Then select eligible work from coverage-report.md subsystem bytes, largest first: specyes/coreabsent ->core; coreyes/replaceabsent ->glue only aftercorelanded; nonexactshadow ->verify; specabsent ->RE. Skip glue26 and docs/abi/unreconciled-rows.tsv.

**Capped / waiting:**
- re-widgets4: parked3/3, ready02ed7fce; cRZWinKeyAccelerator complete, cGZWin partial; nofourthround. Can land supported docs.
- re-application4: parked3/3, readycae0f824, in train27; cSC4App57/322 functions, AP1/AP2 answered, AP3-AP5 open; nofourthround.
- re-terrain4: landed after3/3 rounds, allTR33-TR41 answers; never restart this lane.
- core-netocc4:2/3 rounds, ready2fbe9b89, partial identity/behavior/records; NO15-NO17 block fullowners. No owner implementation credit yet. Land the supported code/questions before dependent RE.

Runtime: six-slot game queue pid22296. Native CLI hosts need CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0; launcher sets it. RE guard preflight must GO. Export-first assembly is permitted by re-workspace.md even without a live REA binary; a workspace no-app diagnostic is separate from guard denial. Game labels unique; qwait without --fresh. Keep own output short. Previous checkpoint lines and train details: [run4 through30](history/handoff-run4-checkpoint30-2026-10-21.md); [run3 stop](history/handoff-run3-stop-2026-10-21.md).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 229 (train27 two lanes PASS; TS67 filed and partial application spec landed); 228 (train26 four lanes, effects core improvements and three focused spec families); 227 (train25 four-lane PASS, PL19 replacement promoted, terrain spec blockers answered, coverage38.7/28.3/12.4/16.5); 226 (train22 PASS, occupant-manager replacement promoted, replaced coverage15.9 to16.2 percent); 225 (train21 PASS, aura restored to 71/71, verified coverage 12.1 to 12.4 percent; netmgr differences documented). See `docs/manual-checks.md`.
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
