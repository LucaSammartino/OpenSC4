# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, wind-down)

**Wind-down in effect** (the user, 2026-10-20 21:49 and again 2026-10-21): start no new work. That means no lanes,
no continuation or fix rounds, and no slot refills, whatever any other document says about parallelism, until the
user lifts the rule. Allowed: landing work that is already finished, docs, coverage, pushing main.

**Main** is pushed, the merge queue is empty, and no lane is running. The gate on main passes (clippy clean, build,
ctest 62/62). Coverage: **35.7% specified / 28.3% implemented / 11.2% verified / 12.1% replaced** (implemented went
from 26.0 to 28.3 when the network-tool cores were wired into the map, `de72a3a1`). Run 2 (the lean-lane GLM run,
2026-10-20 20:31 → 10-21 08:24) landed trains 10-18. Its cost and what it bought are in
[ops/efficiency-log.md](ops/efficiency-log.md).

**Parked work** (every branch is on origin; every other worktree branch is fully in main):

| Branch | Tip | What | Next |
|---|---|---|---|
| `glue21` | `405da5f0` | `enable = propmgr`/`lotcfg` glue; glue21fix2 rounds 1-5 | blocked on PL19 (the growth-pass wave drains lot configurations to zero); handoff in `docs/lanes/glue21fix2.md` on the branch |
| `mysims1` | `53403af4` | the MySims shadow, `FC4MySimsShadow.cpp` (~1,750 lines); lane gate PASS | its game run, then the regress, then the train |
| `re-answers35` | `aa328486` | lane file only. MS1-MS7 are unanswered | redo with REA available; the last attempt cost ~$16 for nothing |
| `glue25` | `60a69143` | WIP from the 2026-10-16 halt: the occupant manager's GZCOM class, `enable = occmgr` | 5 days stale; rebase or redo |
| `glue26` | `fca64f23` | WIP from the 2026-10-16 halt: zones ZG22 (slot 19 `*ok`), ZG24 (HighlightParcels region) | 5 days stale; rebase or redo |

**Open items, ranked:**
1. `enable = propmgr`/`lotcfg`: PL19 (glue21 above).
2. `enable = nettool` landed (glue29, train17; run 221 clean in game). Run a regress replace run with it on, so the
   network tool counts as replaced.
3. `enable = terrain`: the terrain 3D view contract (TR21, `terrain.md` §15.5) landed in train14. The glue is next.
4. The MySims shadow (mysims1 above).
5. ABI: 63 lane proposals disagree with main's row for the same key: [abi/unreconciled-rows.tsv](abi/unreconciled-rows.tsv).
   Decide each one; don't apply in bulk.
6. Efficiency, before the next full-throttle run: see "Next levers" in [ops/efficiency-log.md](ops/efficiency-log.md).
   Enforce the lane context budget and stall kill in the harness, and fix the provider routing (the DigitalOcean pin
   did not hold).

After every landing: `python tools/gen_handoff_tables.py`, `python tools/coverage.py` and
`python tools/coverage_treemap.py`, then push. Check `git merge-base --is-ancestor <lane> main` for each landed lane.
Operator notes (union glue, ABI rows, stalls) are in [ops/coordinating.md](ops/coordinating.md) "Merge-train
operator notes".

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 221 (the network-tool replacement places roads and streets in the real game (inserts=8..69, valid=solved, failure 0), 20/20 scripted placements ok, city runs to 2032 clean with ref_warnings=0.); 216 (component 53 / 11 save round trip, record53 equal at load and save); 215 (devperf2 shadows over the replacement exact after the cell-walk host change); 214 (regress-style netmgr shadow, exact 72/73 to 2032 (the one difference is NM10)); 213 (devperf2 UpdatePopulation 3.52 -> 0.74-1.05 us/call, Grow 349 -> 267-413 us/call, shadows exact). See `docs/manual-checks.md`.
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
