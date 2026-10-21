# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run4 open-ended)

**User policy:** no total round limit. Continue until user STOP, existing STOP, or no work passes check. Target6-8 live lanes (coordinator target8), max3glue, max3rounds per lane. Refill immediately after slots/check pass; no fourth round or bypass of denial. Usage/rate limit pauses refills until clear. STOP: `python tools/lanes.py stop "<reason>"`, land on-disk ready work, push, end.

**Coverage after train32:**41.4/30.0/12.5/16.5 (specified/implemented/verified/replaced), baseline36.8/28.3/12.1/15.9; delta+4.6/+1.7/+0.4/+0.6 points. Ten trains landed:21,22,25,26,27,28,29,30,31,32.60 rounds dispatched, no total cap. Status checkpoint every25 rounds; next75. Two consecutive trains with no coverage-number change require a new table-based selection.

**Live authority:** `build-runs/run4-coordinator-state.json`, `run4-launch.py` (saved guarded Workflow,maxRounds1,ctx150k,calls150). Never use direct/unguarded agents. After compaction/restart reread this section and run lanes.py guard before spawning. Fresh slots/check --touches required, --resume for an existing worktree; deps must be on main.

**Current train33:** traffic TS67 fix + advisor AD1-AD3 answers + complete565span spec. Integration17d49e451ae2f247ad66ab4ceb25e564dab184b1, gate/regress running. Traffic lane exact72/72turbo to2032 and23/23normal to2028, ref0/resync0; no combined-train verification credit before PASS. Advisor null/no-op UI is disproven; answers must land before implementation.

**Parked at3/3, never fourth:** glue-terrain3(97b1c237), re-driverdraw4(partial DX7, landed32), re-editors4(partial curves, landed30), re-particles4(partial drawing, landed30), re-widgets4(partial base, landed28), re-application4(partial57/322, landed27), re-terrain4(answers landed25). Preserve parked worktrees, even when supported portions landed.

**Terrain:** crash fixed earlier, final diagnostics trace apparent stall to missing TerrainReconciliation/ConnectionsReady leaving dialogs enabled. Auto-close d8 reached2032 with refwarning1, NOT a pass. Tick/reconcile behavior, texture step7part2/TR37/Save unfinished. TR42/TR43 filed (preference width, edge comparison); no replacement credit or fourth round.

**Other pending:** RZ finalround3 targets Directory/OFStream/CmdLine/SysServPtr; File/INI specs ready on its unlanded branch. Initial RZ utility belongs strings; stringsround2 reconciles the copied public utility spec before credit. Core4444, verify-disasters, glue-citysit, RE8888 running. Ledger holds exact states; inspect return JSON before refilling. GZ core partial, only queue manager mapped; GZ1/GZ2/GZ5-GZ7 block broader credit.

**Choosing work:** unblocked parked lanes with answered causes first (within3-round cap), then work one validation step from landing, then coverage-report subsystem bytes descending: coreyes/replaceabsent glue; specyes/coreabsent core; nonexact shadow verify (contract gap becomes a question); specabsent RE. Max3glue. Keep earlier skip of glue26 and abi/unreconciled-rows.tsv.

**After every train:** gen_handoff_tables.py, coverage.py, coverage_treemap.py; one status line here, commit/push. `run4-post-train.py` applies explicit tested maps and question-index upkeep after actual PASS+land. Remove landed nonparked worktrees only after verifying lane ancestor of main and clean state; keep branches and ignored evidence. Never remove active, unlanded or parked worktrees. C:>85% stops spawning until cleanup. Keep this section about60lines; archive older status lines to [today's history](history/handoff-2026-10-21.md).

**Main/tools:** clean checkout; tool changes require `python -m pytest -q tools/tests` and immediate separate commit (latest75PASS). Candidate union regression supports --add-shadow formats_shadow and --add-replace keys, preserving defaults.35default replacement keys includeoccmgr/propmgr/lotcfg. Formats own/effective8959exemplar+388cohort exact31; no factory replacement.

**Runtime:** game queue6slots(pid22296); native CLI BG_WAIT_CEILING_MS=0. qwait without--fresh, unique labels. RE preflight must GO; export-first allowed even if no REA binary open. Gate verdicts never set by hand. Train32 generated QUESTIONS conflict resolved by regeneration from merged sources, questioncheckPASS.

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 233 (train32 three-lane PASS;1555 spec and GZ queue-manager core); 232 (train31 PASS; formats read-shadow exact;4444 span spec complete); 231 (train29 three-lane PASS; TS67 answered, partial occupant records); 230 (train28 five-lane PASS; exact-owner specs and partial networkoccupant core); 229 (train27 two lanes PASS; TS67 filed and partial application spec landed). See `docs/manual-checks.md`.
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
