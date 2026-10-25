# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state — RUN 5 END (Codex lane trial), 2026-10-25 10:00 BST

**STOP is set** (`python tools/lanes.py stop`); the watcher and heartbeat are stopped. Live Codex rounds were
killed at the stop; their worktrees and queue entries are kept. Verified reads 27.6 on the train238 report because
landvalue flickered 71/72 there (27.7-27.8 on clean reports). Clear STOP only when the user resumes.

**Coverage** (`tools/coverage.py`): specified 89.2%, implemented 60.2%, verified 27.6%, replaced 27.8%
(run 5 started at 57.8 / 36.1 / 19.7 / 18.1). Trains 126-238 landed (96 batches), plus tloriato/OpenSC4#16
(amarret, 65 RE spec lanes, merged locally, specified +6.7).

**First thing next run:**
- `infra-subsysreg23` (ready, 196f4e2ef) splits `FC4Subsystems.cpp`/CMake/`plugin_load_test.cpp` into per-owner
  fragments. It needs one more merge of main (keys added in trains 231-238), then land it **alone**. It removes
  the hotspot conflicts that forced most recuts this run.
- Shadow summary labels: `nettool_plan_shadow`, `powerline_plan_shadow`, `dbpf_bounds_shadow`, `formats_shadow`
  ran exact in the train237 house shadows run (log lines `nettool plan shadow ... mismatches=0`,
  `dbpf_bounds shadow ... clean`, `formats shadow: total ... EXACT`), but the regress summary emits no
  `<key> N lines` row for them, so coverage.py cannot credit them (cSC4NetworkTool 1.9%, cSC4PowerLineTool 0.26%,
  cGZDBSegment*, exemplar rows). Fix the log prefixes or the summary, then credit. `gzfw_shadow` is credited.
- The merge train's regress runs **main's** `tools/regress.ps1`: shadow-profile keys must be on main before the
  batch whose regress should measure them.

**Ready, unlanded** (queue entries, no credit claimed): harvest-orphans16 (view3d conflicts; r7 merging main),
glue-floraocc-replace22, glue-regionalcity-replace23 (r5: FFI handle owner; Claude review found the replace key
needs an owned RegionalCity ABI, >1k lines, save path), glue-overlays-replace17 (r6: native manager interception),
glue-netlotobj-replace22, glue-netocc-factories24, core-textedit-close14 (newer tip), core-vehicles-close21,
glue-viewlight-shadow25, glue-watercraft-shadow25, glue-cfgrule-shadow25.

**Parked:** glue-lotrec-replace18 (spec §15.1 boundary), fix-shadowcrash23 (Codex cyber filter refused it).

**Known issues:** landvalue 71/72 and propmgr 219/220 flicker one month in some house shadows runs (each costs
~0.2 verified on that report); `plugin_autotest_test` watchdog timings fail under 13-lane load (re-gate passes);
lane worktrees' `engine/build-x86` can hold stale objects whose ninja deps point at `../train-integration`.

The stop note this replaces is archived verbatim in
[handoff-user-stop-2026-10-23-1110.md](history/handoff-user-stop-2026-10-23-1110.md).

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
