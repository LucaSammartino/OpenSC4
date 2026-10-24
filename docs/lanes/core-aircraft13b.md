| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft13b (airplane and helicopter core owner closure) | `core-aircraft13b` / `core-aircraft13b` (engine lane, detached HEAD) | In progress (r2): cargo 7431/0/0; clippy found 3 needless test borrows, now corrected; focused aircraft_settled 10/10 and fmt 1 file PASS. Initial native gate pending; final qbuild all required. No ABI change, game run or new question. | Collect initial native operation, then run final qbuild all on committed correction; report verified partial closure. |

## Handoff

- Goal: finish the workspace gate and implement remaining settled aircraft contracts.
- Starting tip: `dadd406e7`; implementation `5d59a8396`, fixture correction `4ba6b67cd`.
- Prior native verify: PASS, build 264/264, CTest 81/81 (`8b52caea1`).
- Prior cargo/clippy attempts terminated; none remains running or outstanding.
- r2 Cargo PASS: 7431/0/0; aircraft 16, aircraft_settled 10, airplane 10, helicopter 14.
- Evidence: `build-runs/qbuild-test.log`; transport `build-runs/aircraft-r2-all-initial.log`.
- Initial qbuild all still running, session 44670; Cargo PASS, clippy FAILED (3 needless test borrows), native queued.
- Native request: `qbuild-20261024-212010-build-ctest`; no duplicate request.
- Corrected all 3 borrows in `aircraft_settled.rs`; focused suite PASS 10/10; fmt PASS (1 owned file).
- Evidence: `build-runs/aircraft-r2-focused.log`, `aircraft-r2-fmt-corrected-absolute.log`.
- One Cargo job/debug=0. Diagnosed BelowNormal starvation (59 CPU s/19 min); restored only lane compilers/Cargo to Normal.
- Owner audit: settled sections and vectors 1-27 already covered; no additional settled behavior identified.
- Next: collect session 44670 to terminal; run `build-runs/aircraft-r2-gate.py all final` on this correction.
- Keep AF2-AF6, AF10 and AF11 explicit; no capi/plugin/ABI wiring.
- Existing detailed r1 report below preserves incomplete gate evidence.
- Detached HEAD: commit normally; supervisor publishes reserved lane branch.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft13b-r2-manifest.json`.

## Changes

Implementation commit `5d59a8396`; legacy chase fixture/documentation correction
`4ba6b67cd`; native gate record `8b52caea1`. Initial scope audit `9b81f5303`.
The worktree is intentionally detached; the supervisor publishes commits to the
reserved `core-aircraft13b` branch. No branch/ref/config changes were made.

- Airplane: constructor ammo/drop/countdown defaults, signed zero-ammo decrement,
  remaining-countdown LOITER with capped physics seconds before pursuit, dispatch
  state-5 request, Read preserving unsaved runtime, and ordered phase sound actions.
- Helicopter: exact preset-0 words; first runtime allocation with player refresh
  before whole-record AI overlay; no repeated initialization; PhysicsInit phase 0;
  reset/hover controls; chase lead, 192/32/opacity gates, teleport/scaled rotor
  synchronization and returned movement/height actions; two-sample building
  clearance with auxiliary/home maximum and descending-demand quirk.
- Ordered helicopter phase sounds, engine/embedded notifications and velocity
  reduction; allocation fault diagnostics without rollback; post-base Shutdown
  actions in specified order, including false propagation and independent controls.
- Shared slot-7 weapon action stage, exact missile/laser fallback and use-count
  wrapping; helicopter takeover record precedence/speed fallback/gain refresh;
  spotlight pair activation gate with explicit pending geometry continuation.
- `engine/conformance/tests/aircraft_settled.rs` adds independent vectors 14-27.
  Updated older phase, LOITER, read and chase expectations to settled contracts.
  Existing parameter/codec/scalar/candidate-motion coverage is retained.
- Replaced both module README owner closure tables with per-section code/test/open
  rows. No plugin/capi/ABI changes, no coverage-map edits, no forbidden sources read.

The stages return typed requests for foreign actions; they do not claim complete
flight ticks, complete native Init/Shutdown or guessed steering/projectile laws.
Native attachment ownership is a documented adapter precondition; no borrowed
native pointer is released by this portable core.

## Validation and preserved evidence

- Required native command: `qbuild.py --wt <WT> verify`, request
  `qbuild-20261024-200154-build-ctest`, terminal PASS. Build 264/264 edges,
  248.1 s; CTest 81/81, 291.1 s; total 541.4 s. Logs:
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
  The verdict noted tree changed during run: the legacy chase fixture and shared
  phase/chase documentation were corrected while it ran. Runtime source was the
  committed implementation; this is not a completed final workspace gate.
- qbuild fmt PASS: 9 explicit owned Rust files, diff_lines=0. Rechecked the two
  subsequently corrected files: PASS, 2 files, diff_lines=0.
- First focused wrapper failed before compilation: Git Bash CreateFileMapping
  Win32 error 5. Evidence: `build-runs/q/20261024-194343-34628.log`.
- Native focused attempt 1 terminal FAILED: invalid Copy derive on the action
  result's Vec fields. Fixed before implementation commit. Evidence:
  `build-runs/aircraft-focused-1.log`.
- Native focused attempt 2 did not progress beyond core compilation for over
  20 minutes; stopped and collected terminal exit 1, without a test verdict.
  Evidence: `build-runs/aircraft-focused-2.log`. No focused-test pass is claimed.
- Required `qbuild all` attempted on final runtime source. First attempt stopped
  at core compilation after diagnosing multi-GB compiler processes under shared
  load. Evidence: `build-runs/qbuild-test-incomplete-debug.log`.
- Retried with changed build inputs: CARGO_BUILD_JOBS=1,
  CARGO_PROFILE_DEV_DEBUG=0 and CARGO_PROFILE_TEST_DEBUG=0. This attempt also had
  no output beyond core compilation before context wind-down. Stopped and
  collected terminal exit 1; evidence: `build-runs/qbuild-test.log` and
  `build-runs/qbuild-test-incomplete-nodebug.log`.
- Workspace cargo totals and clippy: **unverified**, not zero-failure passes.
  No gate or game request remains running or outstanding. No game shadow is
  claimed: this is a core-only task, with AF11 still open for safe native inputs.

## Remaining spec contracts and questions

No new question filed. Existing AF2-AF6, AF10 and AF11 remain explicit open rows.
Settled portions of these questions were implemented, rather than treating each
whole question as Unsupported. AF1/AF7/AF8/AF9 are not reopened.

- aircraft §3.2 / §4.2 lifecycle remainder: constructor fields, probes,
  attachment creation, pre-base Shutdown scaling, full native Init order and
  physics-driven phase triggers: AF4 (with physics triggers also AF2).
- aircraft §§3.3/4.2 and automata §§6.5/6.6: complete airplane steering,
  runway/approach/fire sequencing, helicopter MoveXZ and non-chase mission/claim/
  return/arrival sequencing: AF3; exact dispatch/message transitions: AF6.
- aircraft §3.4 complete force frame, angular/environment/collision corrections,
  and §4.2 complete rotor model: AF2. Non-finite physics equivalence: AF10.
- aircraft §§2/3.4/4.3: complete player tick/command mapping, cadence, projectile
  setup and spotlight geometry/smoothing/terrain transforms: AF5.
- Native read-only helicopter shadow access/independent input history: AF11.

## Next steps

1. On this committed worktree, run the required final `qbuild all` when compiler
   resources permit. Do not infer a cargo/clippy pass from the native CTest pass.
   The last attempt used one job and debug-symbol generation disabled; preserve
   the incomplete logs above before a later run overwrites qbuild-test.log.
2. Fix any concrete diagnostics and run focused tests, then repeat the final gate
   on stable source. All older operations were terminated and collected; there
   is no session to resume and no duplicate check to wait for.
3. Review the two module README owner closure tables. Keep the existing open AF
   rows; do not wire plugin/capi or claim trajectory/shadow equivalence in this lane.
4. After a completed gate, update this row/report and queue ready with actual HEAD.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft13b-r1-manifest.json`.
