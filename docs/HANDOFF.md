# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state (2026-10-21, run 3 stopped at 8/8 rounds)

**STOP is set: `run3 round limit reached (8/8); land committed ready work only`.** No lane is running,
the merge queue is empty, and main is pushed. No further lane rounds are authorized in this run.
The guard is authoritative; no parking, watchdog or context state was cleared or bypassed.

- **Train19 landed `832c7238e0`:** MySims shadow plus its lifecycle/ref-tracking fixes, and MS1-MS7 RE answers.
  Gate PASS (clippy clean, build, ctest 62/62 including cargo_test); full regress PASS, both runs clean to 2032,
  zero problems and ref_warnings=0. MySims is exact **73/73** in the combined shadow run (run 223; standalone
  run 222 also 73/73). The recorded exclusions still apply; the new MS3/MS5/MS7 engine changes are future work.
- **Train20 landed `37544298d4`:** PL19 answered in prop-lotconfig.md, docs only; no engine change or game claim.
  Slot 9 AddRefs every returned element on every call, including cache hits, on the pointer inserted in the set.
  The PL18 second add is wrong. This does not by itself explain the measured two-adds/drain contradiction:
  inspect cache hits and pointer identity, then validate against glue21's per-op trace before claiming a fix.

**Coverage:** 36.8% specified / 28.3% implemented / 12.1% verified / 15.9% replaced, from train19.
The current regress is authoritative. MySim and MyStreetSim now map to mysims_shadow; untested classes do not.
The earlier lane-DLL regress passed with three worse comparisons versus train17 (netmgr, landvalue, aura);
the full train regression including MySims passed and its own shadow column is in build-runs/regress-train19.out.

**WIP preserved and pushed; none is queued for landing:**

| Branch | Tip | State / next |
|---|---|---|
| `glue25fix3` (continues `glue25`) | `209ff7fc` | gate PASS, ctest 62/62. enable=occmgr game g1 corrupt at 2028-04-01, ref_warnings=0. g2 clean to 2032 was shadow-only, not a replacement pass. Diagnose g1, then replacement game and regress with occmgr. After it lands, add occmgr to tools/regress.ps1 $Replace and the cSC4OccupantManager replace key in tools/coverage-map.tsv. |
| `glue-terrain3` | `d112c328` | round 2 committed view/raw-record glue and terrain Load (c1b06da3, 25a4ec76); the no-code parking condition did not apply. Full gate PASS, ctest 62/62; gt3-repl-4 corrupt before a game date, ref_warnings=0. Diagnose crash; finish Save and TR33-TR37. Its worktree lane file records the exact implementation plan. |
| `glue21` | `405da5f0` | parked implementation. PL19's RE contract is now on main; apply and trace-validate the per-call slot-9 ownership rule before a new game run. No implementation round remained under the 8-round cap. |
| `glue26` | `fca64f23` | skipped as requested; original glue25 branch also retained. |

**Operator notes for resuming:**
- The first three continuation workflows were killed by the CLI's default ten-minute background wait ceiling,
  without final lane reports. Their code was preserved. The coordinator committed and validated the last MySims
  timing edit; terrain/occmgr failures were not relabeled as ready. Future CLI hosts set
  `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0` and wait for TaskOutput; keep the guarded fc4-lane workflow.
- The old game queue's child PowerShell could not load Get-FileHash. It was restarted from Windows PowerShell
  with the working module path (queue pid 22296, six slots); real game verdicts then worked. No harness code changed.
- REA was reconnected before each RE spawn; check and preflight both said GO. Both RE lanes used export bytes.
  A later RE spawn still needs a fresh GO; do not treat an earlier connection as persistent.
- Tool changes were separate commits (370f438d regression list, a9490b4c coverage map), each with tools/tests
  passing 61/61. ABI reconciliation and glue26 were not worked on.
- After compaction/restart, re-read this section and run `python tools/lanes.py guard` before any spawn.
  Do not clear STOP or unpark a lane without the authorization/cause fix required by the user's run rules.

The superseded start note is preserved verbatim in [history/handoff-run3-start-2026-10-21.md](history/handoff-run3-start-2026-10-21.md).

<!-- BEGIN GENERATED: latest-run -->
- **Latest in-game run:** 223 (train19 PASS, MySims shadow exact 73/73 alongside the full regression, ref_warnings=0); 222 (MySims shadow exact 73/73 through 2032, including load, ref_warnings=0); 221 (the network-tool replacement places roads and streets in the real game (inserts=8..69, valid=solved, failure 0), 20/20 scripted placements ok, city runs to 2032 clean with ref_warnings=0.); 216 (component 53 / 11 save round trip, record53 equal at load and save); 215 (devperf2 shadows over the replacement exact after the cell-walk host change). See `docs/manual-checks.md`.
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
