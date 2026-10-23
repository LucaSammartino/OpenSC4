# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state — USER STOP, 2026-10-23 11:10 BST

**STOP is set** (`python tools/lanes.py stop`). Nothing runs: the nine stopped workers are landed and
the refiller has exited. Clear STOP only when the user explicitly resumes.

**Main:** the nine stopped workers were finished and landed as train124 (8 lanes, [run 301](runs/train124-301.md))
and train125 (`core-my-street-sim4`, [run 302](runs/train125-302.md)); both gates and regresses were clean (the
train124 netmgr 72/73 -> 71/73 dip was run noise, back to 72/73 in train125). Every lane tip is an ancestor of
main. The new core modules are unwired, so coverage did not move. No batch is in flight.

**Coverage** (`tools/coverage.py`): specified 57.8%, implemented 36.1%, verified 19.7%, replaced 18.1%
(3,226,906 / 2,018,463 / 1,099,943 / 1,012,661 of 5,584,842 bytes). Over the 12 hours ending at this stop
(trains 103-123, ~590 commits, +103k lines) this moved by +8.4 KB implemented, +64 KB verified and 0 replaced.
The exact-function credit path (76 B, self-reviewed) was removed in the 2026-10-23 trim. Bounded/partial deliveries earn no owner credit; on resume, prefer finishing owners over
parallel partial slices.

**Unlanded work** (worktrees removed 2026-10-23; every unlanded branch is pushed to origin, and each removed
worktree's build-runs text is in `build-runs/worktree-archive-2026-10-23.tar.gz`, local only):
- `re-curve-editor-views4`: spec transliterates x87 instruction sequences (CONTRIBUTING.md "Specs"); rewrite
  as behaviour before landing.
- Held Codex lanes (`git branch --no-merged main`, about 35): mostly blocked on unpublished contracts; review
  one by one before reviving. `tools-build-singleflight4` and `tools-refill-rea-admission4` extend the removed
  swarm tooling.
- Pre-run parked branches (`glue25`, `glue26`, `glue-terrain3`, `verify-netmgrfix4`, `integration/*`,
  `coordinator/*`) are unchanged.

**Known issues:** train121 really worsened LandValue (72/72 to 71/72) and the coordinator landed it anyway
([incident](ops/acceptance-incident-train121.md)); every landing must read the full comparison delta, not just
the verdict. Native per-step LandValue capture is still 0. C: is 68% full after the worktree cleanup.

The stop note this replaces is archived verbatim in
[handoff-user-stop-2026-10-23-0916.md](history/handoff-user-stop-2026-10-23-0916.md).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 337 (Dormant tutorial/citydetail/variant/feature shadow keys and the traffic sentinel fix landed; landvalue dropped one line (run 5).); 336 (Advisors shadow exact in the house regress and credited; particles, strings, network rules and GZ framework contracts landed (run 5).); 335 (advisors round 2, clean 2032 run, strict shadow 1/73 with all differences explained by AD18); 334 (Advisors shadow and Lua host owners landed; web/network and grid/RNG contracts merged; traffic flake recurred (run 5).); 333 (Seven lanes landed after bisecting out the netocc shadow; effects parser commands claimed; resumes no longer blocked by the unmerged cap (run 5).). See `docs/manual-checks.md`.
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
