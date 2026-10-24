| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft13b (airplane and helicopter core owner closure) | `core-aircraft13b` / `core-aircraft13b` (engine lane, detached HEAD) | READY: r4 settled-owner audit complete; source unchanged since full qbuild all PASS on `009941660`: cargo 7432/0/0, clippy clean, native build 214/214, CTest 81/81; owned suites 51/51. AF2-AF6, AF10, AF11 retain explicit open rows. No ABI change or game run. | (1) Coordinator review/publish detached commits; (2) settle the remaining spec contracts before complete native flight ticks and shadows. |

## Current result (r4 audit)

Resumed from `815565e5c` and audited the current aircraft sections against both
owner modules, their closure tables and conformance fixtures. The inherited
implementation already covers the settled owner contracts, including the partial
AF2–AF5 resolutions. No additional settled implementation gap was found, so this
round changes only this report and refreshes the ignored merge-queue entry.

The audit checked parameter/default selection, saved/unsaved state, LOITER,
candidate motion and supplied-lift force assembly, helicopter preset/overlay
order, reset/hover, chase gates, phase actions, building clearance, allocation and
cleanup, weapon requests, takeover and spotlight activation. It also checked
automata §§6.4–6.6's aircraft records, default handlers and mission/phase facts
against the owner tables. Aircraft §8 vectors 1–27 remain covered as listed there.

`git diff --stat 009941660 HEAD` showed only the lane report; the source tested by
the previous terminal full gate is still the final source. Re-read the preserved
`build-runs/aircraft-r3-all.log`: cargo **7432/0/0**, clippy **0 diagnostics**,
native build **214/214**, CTest **81/81**, final **qbuild: PASS**. Focused suites
remain **51/51**. No redundant gate was launched; no operation is outstanding.
No new questions, source changes, ABI/glue changes or game requests this round.
The exact remaining open sections and earlier gate evidence are recorded below.

## Implementation result (r3)

Settled airplane/helicopter owner contracts are implemented and tested, with
per-section code/test/open rows in both module READMEs. Full native ticks remain
explicitly unsupported only at the spec-open boundaries listed below.

Implementation `009941660` adds `ForceScalars::force_with_lift`: the settled sum
of supplied stored lift, orientation-scaled thrust, gravity and opposing drag.
Ground resistance uses the supplied lift's vertical component. It does not
invent AF2's lift direction or claim a complete force/flight tick. An independent
fixture covers phases 0/1/2 and tilted lift with the same magnitude but less
vertical support. Corrected stale helicopter preset and chase-stage comments.
Gate evidence was committed in `95fc75d64`.

- Focused suites: **51/51** (aircraft 16, aircraft_settled 10, airplane 11,
  helicopter 14), including all applicable aircraft §8 vectors 1–27.
  `build-runs/aircraft-r3-focused.log`; fmt **PASS**, 3 explicit changed Rust files.
- Final stable-source `qbuild.py all`: terminal **PASS** on `009941660`.
  Cargo **7432 passed / 0 failed / 0 ignored**, summed over 52 result lines;
  clippy `-D warnings`: **0 diagnostics**. Native build **214/214**, 191.7 s;
  CTest **81/81**, 274.0 s; native total 467.8 s.
- Native verification went through qbuild's coordinator queue, request
  `qbuild-20261024-221517-build-ctest`. No direct fc4verify invocation.
  Logs: `build-runs/aircraft-r3-all.log`, `qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`. No tree-change warning.
- The failed Bash wrapper launch remains in
  `build-runs/q/20261024-214849-13088.log`; it failed before Cargo started.
  The replacement PowerShell focused operation passed and was collected to
  terminal, as was the full gate. No operation or game request is outstanding.
- Build inputs: one Cargo job, dev/test debug info disabled. Only this lane's
  initial Cargo/compiler processes were restored from BelowNormal to Normal;
  the final gate launched at Normal. No shared build configuration changed.
- No new question; existing AF2–AF6, AF10 and AF11 stay open. No ABI/plugin/capi
  or coverage-map change. Game runs/shadows: **n/a**, portable core only;
  independent native helicopter shadow inputs remain AF11.

## Inherited implementation (r1/r2)

Implementation commit `5d59a8396`; legacy chase fixture/documentation correction
`4ba6b67cd`; native gate record `8b52caea1`. Initial scope audit `9b81f5303`.
The worktree is intentionally detached; the supervisor publishes commits to the
reserved `core-aircraft13b` branch. No branch/ref/config changes were made.

- r2 correction `32e3f34b2` removes three needless borrows in conformance Read
  fixtures. The owner audit confirmed coverage of the settled sections and
  conformance vectors 1-27; it identified no additional settled owner contract.
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

Owner tables: `engine/core/src/automata/airplane/README.md` and
`engine/core/src/helicopter/README.md`.

The stages return typed requests for foreign actions; they do not claim complete
flight ticks, complete native Init/Shutdown or guessed steering/projectile laws.
Native attachment ownership is a documented adapter precondition; no borrowed
native pointer is released by this portable core.

## Prior-round validation and preserved evidence

- Final `qbuild.py --wt <WT> all`: terminal **PASS** on stable committed source
  `0c23156bd` (test correction `32e3f34b2`). Cargo totals across every result line:
  **7431 passed, 0 failed, 0 ignored**; clippy `-D warnings`: **0 diagnostics**.
  Native request `qbuild-20261024-213550-build-ctest`: **46/46 build edges**
  (1.8 s), **CTest 81/81** (89.0 s), native total 90.9 s. No tree-change warning.
  Evidence: `build-runs/aircraft-r2-all-final.log`, `qbuild-test.log`,
  `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Final owned suites: aircraft 16/16, aircraft_settled 10/10, airplane 10/10,
  helicopter 14/14: **50/50**. Focused corrected suite separately PASS 10/10:
  `build-runs/aircraft-r2-focused.log`.
- qbuild fmt: r1 PASS on 9 explicit owned Rust files and 2 corrected files;
  r2 PASS on the single changed test file, diff_lines=0, in
  `build-runs/aircraft-r2-fmt-corrected-absolute.log`. The preceding relative-path
  invocation could not find the file; its failed invocation is preserved in
  `build-runs/aircraft-r2-fmt-corrected.log`.
- Initial r2 full gate terminal FAIL: Cargo 7431/0/0, clippy 3 needless borrows;
  native request `qbuild-20261024-212010-build-ctest` PASS (build 214/214,
  217.7 s; CTest 81/81, 272.0 s; total 491.9 s; tree changed during run).
  Preserved transport: `build-runs/aircraft-r2-all-initial.log`; full initial
  logs: `build-runs/aircraft-r2-final-prior-qbuild-test.log`,
  `aircraft-r2-final-prior-qbuild-clippy.log`,
  `aircraft-r2-final-prior-fc4verify-build.log`,
  `aircraft-r2-final-prior-fc4verify-ctest.log`.
- Build inputs: one Cargo job, dev/test debug information disabled. The initial
  compiler had only 59 CPU seconds after 19 minutes at BelowNormal priority;
  restored only this lane's compiler/Cargo processes to Normal. Final gate
  launched at Normal priority. No shared build configuration changed.
- Earlier r1 failures/incomplete attempts remain historical, not passes:
  `build-runs/q/20261024-194343-34628.log`, `aircraft-focused-1.log`,
  `aircraft-focused-2.log`, `qbuild-test-incomplete-debug.log`,
  `qbuild-test-incomplete-nodebug.log`; details are in report commit `dadd406e7`.

Every operation was collected to its terminal result. No build or game request
remains outstanding. Game runs and shadows: n/a for this core-only lane; AF11
still blocks safe independent native helicopter inputs.

## Remaining spec contracts and questions

No new question filed. Existing AF2-AF6, AF10 and AF11 in
`docs/specs/questions/aircraft.md` remain explicit open rows.
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

1. Coordinator: review the two owner closure tables and publish the detached
   commits to the reserved lane branch; retain the explicit remaining open rows.
2. Spec writers: settle the remaining contracts above before full owner ticks,
   native lifecycle/control integration and trajectory/shadow equivalence.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft13b-r4-manifest.json`.
