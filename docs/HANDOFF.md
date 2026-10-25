# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state — RUN 6 (verification spikes), 2026-10-25

STOP is not set. Run 6 tests three verification architectures as Codex spike lanes; ordinary implementation lanes
are not running. Landed in trains 239-242: shadow summary labels, Codex outage resume with a 40-minute round cap,
the coverage-credit check at enqueue (`python tools/merge-train.py enqueue ...`; the train refuses entries without
`credit`), and the docs cleanup.

**Coverage** (`tools/coverage.py`, main 041116d01): specified 89.2%, implemented 60.2%, verified 30.6%,
replaced 27.8%.

**Spikes** (worktrees `.claude/worktrees/spike-*`, evidence in `engine/tests/spike-evidence/<lane>/`):
- `spike-tapes26`, S1 interface tapes: **KILL**. The generic untyped recorder loses required input in 32/78
  calibration calls (pointer text replaced by markers). Queue entry held at `wip`; do not land.
- `spike-osboundary26`, S2 OS/DirectX boundary record and replay: round 3 running (per-thread clock).
- `spike-inplace26`, S3 per-function in-place differential: round 2 INCONCLUSIVE (14/20 exact with confined
  writes, every deliberate error detected, 0.875 KB per lane-hour including the facility); round 3 measures the
  marginal cost per added function.

The coordinator is scoring the same seeded mutants with each oracle (unit tests, in-game shadow, in-place) before
proposing a method to the user. The definition of verified and the credit rule for a new oracle are the user's.

**Queue:** the 25 run-5 entries still queued have no `credit`; each needs `merge-train enqueue` at review before
it can land. `infra-subsysreg23` (per-owner subsystem fragments) still needs a merge of main and should land alone.

**Known issues:** landvalue 71/72 and propmgr 219/220 flicker one month in some house shadows runs;
`plugin_autotest_test` watchdog timings fail under heavy lane load (re-gate passes).

The run-5 stop note this replaces is archived verbatim in
[handoff-run5-end-2026-10-25.md](history/handoff-run5-end-2026-10-25.md).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 659 (modelinst partial shadow landed (run 5); renderprops and app exact in the house regress; demand and landvalue flaked.); 652 (partial shadow method audit landed (run 5): 44 functions, 4,070 bytes.); 651 (map view answers complete; render property and application partial shadows landed (run 5).); 649 (renderprops registry and owned fixture exact 73/73 through 2032); 645 (graphics driver partial shadow landed and the 48-key replace list confirmed clean in the house run (run 5).). See `docs/manual-checks.md`.
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
