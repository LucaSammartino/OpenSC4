# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state — USER STOP, 2026-10-23 09:35 BST

**STOP is set** (`python tools/lanes.py stop`). Nothing runs: all nine workers, the train123 pipeline and
the refiller have exited. Clear STOP only when the user explicitly resumes.

**Main:** train123 landed as `cf436ba3` after review. Its gate passed (5746 Rust, Clippy, CTest 68/68), and
the regress was clean with no worsened comparison row vs train122 ([run 300](runs/train123-300.md)). All five
lane tips are ancestors of main. No batch is in flight.

**Coverage** (`tools/coverage.py`): specified 57.8%, implemented 36.1%, verified 19.7%, replaced 18.1%
(3,226,906 / 2,018,463 / 1,099,943 / 1,012,661 of 5,584,842 bytes). Over the 12 hours ending at this stop
(trains 103-123, ~590 commits, +103k lines) this moved by +8.4 KB implemented, +64 KB verified and 0 replaced.
The exact-function credit path (76 B, self-reviewed) was removed in the 2026-10-23 trim. Bounded/partial deliveries earn no owner credit; on resume, prefer finishing owners over
parallel partial slices.

**Unfinished work left in worktrees** (uncommitted or unqueued; inspect before reuse): Automata controller RE,
LotModels, TerrainOverlays, NetworkRoutines, MissingPlugin reporter, MyStreetSim, PropertySetter helpers,
AutoGamePause, RZ SysServPtr. Radius and CurveEditor R2 have READY docs queue entries that were not
independently reviewed. Watercraft RE is WIP and not enqueued. Tooltip, RZ TextFile and native UTF8 are drafts.

**Known issues:** train121 really worsened LandValue (72/72 to 71/72) and the coordinator landed it anyway
([incident](ops/acceptance-incident-train121.md)); every landing must read the full comparison delta, not just
the verdict. Native per-step LandValue capture is still 0. The repo holds about 150 worktrees and 765 branches,
and C: is 82% full.

The stop note this replaces is archived verbatim in
[handoff-user-stop-2026-10-23-0916.md](history/handoff-user-stop-2026-10-23-0916.md).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 300 (FireFlames and DrawContext core contracts, string-encoding and UI-feedback specs, and an unapproved UTF8 qualification draft landed after a clean gate and a regression with no worsened comparison rows; coverage unchanged.); 299 (Bounded Geometry algorithms and six reviewed public contracts landed after actual5726Rust/68CTest and standard game regression with zero worsened comparison rows; original native and capture limits remain.); 298 (Seven bounded portable Sprites, GameLife, Video, Ripples, Transport, TextDecal and standalone UTF8 implementations landed after actual5714Rust/68CTest and standard clean game regression; native and whole-owner limits remain.); 297 (Bounded LocalGrid, RigidBody and EffectsParser algorithms plus strict exact-function accounting tooling landed after actual5584Rust/68CTest/247tools and standard clean game regression; empty ledger adds zero credits.); 296 (Bounded TypeInfo lifecycle/placement algorithms landed after actual5511Rust/68CTest and standard game regression; named gaps and previous failed checks remain recorded.). See `docs/manual-checks.md`.
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
