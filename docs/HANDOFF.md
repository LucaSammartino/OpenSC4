# Handoff (start here)

The one page every agent reads first: the current state, and where everything else is. The coordinator updates §0
before it stops and moves the stop note it replaces, verbatim, to [docs/history/](history/). Operating knowledge
lives in [docs/ops/](ops/); history is `git log` and [docs/history/handoff-2026-10.md](history/handoff-2026-10.md).
Keep §0 to the current state: what is true now, what is parked, and what is next. Narration goes in `git log`.

## 0. Current state — USER STOP, 2026-10-23 09:16 BST

The user stopped the goal and authorized halting. **STOP is set** through normal
`python tools/lanes.py stop`. No new work, worker rounds, refills, probes, source changes,
publications or validation runs. The three collaboration helpers have stopped. Running
workers receive the normal guard instruction to commit their handoff and return; their
process presence below is a snapshot, not a claim that all have exited.

**Last landed train122:** `d9c8a9a351b0639d1e50acb295ea27d0bc2a322a`, 08:43:32.
Postchecks and reports pushed `8e69a1ac`; latest main before this stop note `b5e4b7c3`.
SysPtr/MyStreet/PropertySetter/NetworkUI/AutoPause/MissingPlugin/Geometry exact supplied
tips are all ancestors. Gate5726Rust/0/0 in27 summaries, Clippy31.1s, native50/50,
CTest68/68,231.4s. Both standard game profiles clean2032/ref0/Release0/detach,
124.109s/608.469s. Root and Live independently inspected the full comparison delta:
ZERO worse. LandValue71/72 to72/72 reflects the new sample, not a capture fix.
All seven train122 worktrees now removed with branches and ALL ignored evidence retained:
the final RE MissingPlugin target was independently qualified by exact WT/manifest/branch
identity after the generic substring checker confused it with the active CORE name.
Proof `build-runs/coordinator-train122-missing-plugin-exact-cleanup.json` records ordinary
cleanup success; original generic false-positive evidence remains unchanged.

**Train123 is NOT landed.** Integration `34f4dadfd42897c69a78964cb9a98d49769b211d`
contains DrawContext `fab671ba`, FireFlames `a2fa7c19`, Encoding `d38a9b12`,
UIfeedback `82bdc668`, UTF8 qualification DRAFT `e083d3cd`. Each bounded delivery
was independently reviewed; the UTF8 qualification intentionally remains owner=null,
approved=false and earns no credit. Gate PASS09:12:14: 5746Rust/0/0 in27
summaries, CTest68/68,244.1s; exact record `build-runs/gate-train123.json`.
Pipeline19860 started09:07:45 and is already running the standard regression.
It stops at ready-for-review and DOES NOT automatically land. Do not start a duplicate
game/check. If still running at a later resume, collect that existing operation.
Run300 reserved; next301 requires a fresh collision reservation. Artifact archive
`build-runs/gate-artifacts/train123/manifest.json` pins DLL18485248B
SHA25690016d63e651d36bb3581fcbae4c941b1d1ea2ab211b54c3a2b09ada1aa69b74,
PDB175255552B SHAd7f04de793093032e682db7c787cc778d8eaa95ac0a027ca67b1a386fb3eb93b,
LastTest495821B SHA8bc54db93880b890f7add5e5ab9792a88d248dbf3cae90332d9aa9dcc9b7d92d.
Every actual comparison delta must be reviewed before any future landing.

**Stop snapshot:** heartbeat09:16:48,9 actual backends, no provider pause, zero pending
launchable briefs,581 total rounds. Refiller18188 was identity/owner-lock verified;
STOP prevents dispatch regardless of process lifetime. Latest independent nine-worker
proof09:14:26 is `coordinator-current-worker-identity-091426-3e2e63b0.json`.
A subsequent read verified these exact nine PIDs still present:

| Worker | Backend PID | Original family consumption |
|---|---:|---|
| RE Automata controller lifecycle | 14144 | FINAL3/3 |
| CORE LotModels contracts | 29168 | FINAL parent1+child2=3/3 |
| CORE TerrainOverlays contracts | 25260 | FINAL parent1+child2=3/3 |
| CORE NetworkRoutines | 30232 | 2/3 |
| CORE MissingPlugin reporter | 29944 | 1/3 |
| CORE MyStreetSim | 25188 | 1/3 |
| CORE PropertySetter helpers | 13052 | 1/3 |
| CORE AutoGamePause | 26424 | 1/3 |
| CORE RZ SysServPtr | 38480 | 2/3 |

These are unfinished worker states, not READY qualification. Preserve all dirty source,
evidence, current operations and remaining original budgets during wind-down. No force
process termination or validator/compiler/cache restart was performed. Capacity was not
continuously nine; completions and repeated real provider failures produced vacancies.
The policy now requires nine actual workers plus three independently launchable spares
when resumed, but the stop snapshot has zero launchable spares. STOP overrides that target.

**Other completed or held work:** RadiusR2 and CurveEditorR2 now have own READY docs queues;
their final deliveries are NOT independently reviewed after STOP. Watercraft original
RE3/3 remains WIP: independent review recommends a coherent bounded DOCS publication,
but ROOT has NOT promoted/enqueued it; WC3–7 and the CORE park remain. Tooltip CORE is
a conditional draft, not queued, waiting actual123 publication/full admission and a small
unapplied wording clarification. RZ TextFile research draft is unqueued and shares the
RZ hold with SysServPtr. Preview is blocked by active NetworkRoutines; ClipR2 is blocked
by active AutoPause's full other-free-functions key. Native UTF8 remains conditional on
full strings+other-free admission and a published/version-tested direct-call ABI.
Do not count these drafts or holds as launchable replacements.

**Coverage on main:** specified57.8% (3226982B/14605functions), implemented36.1%
(2018539B/8985), verified19.7% (1099943B/4325), replaced18.1% (1012661B/3899),
fixed denominator5584842B/29536functions. Three exact Registry functions added76B to
specified and implemented only, ownerpartial3/48/native0. Strict unchanged resolver,
treemap/audit and full tools247PASS/1skip45.27s qualified that admission.
No owner, exclusion or denominator adjustment. UTF8 DRAFT is not admitted.
Checkpoint575 was reviewed at actual577: +76/+76/0/0 bytes vs550; next600.

**Errors and binding holds:** train121 LandValue really worsened72/72 to71/72 and withdrew
6711B/27verified functions; root missed its printed worsening before landing. The complete
incident is [recorded here](ops/acceptance-incident-train121.md). Train122's later exact
samples do not erase that failure. Native per-Step capture remains0; safe observer
installation/lifetime remains unproved. Structured worsening enforcement is HELD by
existing tooling claims; the immediate coordinator rule requires complete delta review.

Latest provider error was SysServPtrR1 selected-model-capacity after ONE actual command,
not zero. Same-model AVAILABLE/exit0 at09:07:53 qualified its exact pause; prior21192
positively absent, new18188 verified. ClipR1 had upstream capacity before tools, masked
by the wrapper's handshake diagnostic; both are retained. Missing failure reports were
truthfully committed through normal detached-commit/publisher CAS in existing WTs before
fresh same-family resumes; no source credit, refund, guard edit or fourth round.
Every historical formatter/native/MSRV/unknown terminal remains in the respective report.
TerrainR1's5597 gatePASS and1260-line formatterFAIL are distinct; FINAL source correction
is unfinished. NetworkR1 had11 focusedPASS, dirty partial production and capacity error;
no all had started. Original3/3 holds, KeyList235/238/ref4, REA/Singleflight and all other
parked failures remain binding. Region RE belongs to LucaSammartino through2026-11-21.

**If the user explicitly resumes:** first read prompt.txt, ops/rules.md, this note, fresh
heartbeat and normal guard; clear STOP only on that resume authority. Collect current
train123 and all worker handoffs without duplicate operations. Review full train delta
before normal landing/postchecks/qualified cleanup; review Radius/Curve independently.
Restore actual9 and three disjoint usable spares through normal guards. No automatic
Watercraft promotion, UTF8 admission, unpark, model switch or budget reset.
The previous section0 is archived verbatim in
[handoff-before-user-stop-2026-10-23.md](history/handoff-before-user-stop-2026-10-23.md).

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
