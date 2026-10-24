| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-vehicles-close11: vehicle owner closure | Codex / core-vehicles-close11 | ready | Integrate audited contracts; answer VH1/VH5/VH7/VH9/VH10/VH11 before full owner closure |

## Round 2 result

Ready to integrate the audited portable-core changes. No C ABI export or plugin
registration changed. **None of the four owners is fully implemented.**

| Owner | Status | Open questions blocking full implementation |
|---|---|---|
| cSC4TrafficAutomaton | Partial, explicit Unsupported contracts | VH7 remaining route geometry/reconstruction; VH9 inherited states/controller and base dispatch contracts; VH10 fresh Init slot 134 |
| cSC4Vehicle | Partial, explicit Unsupported contracts | VH1 player driving; inherited VH7/VH9; VH10 drive-parameter slot 158 and inherited fresh Init |
| cSC4TrainCar | Partial, explicit Unsupported contracts | VH5 network placement override; inherited VH1/VH7/VH9/VH10 |
| cSC4Train | Partial, explicit Unsupported contracts | VH5 composition/following/trip/model/placement/message/accident algorithms; VH11 identity/runtime binding; inherited VH1/VH7/VH9/VH10 |

The module's [owner table](../../engine/core/src/vehicles/README.md) maps the
implemented contracts and tests. VH2/VH3/VH4/VH6/VH8 clarification vectors are
implemented; the partial VH9 clarification does not close its remaining gaps.

## Changes

- Removed provisional `InitWorld::initialize_route` and `load_drive_parameters`
  callbacks. They were missing owner algorithms, not foreign services. Init now
  returns `Result<bool>` with persistent `InitVh10` errors for missing work,
  rather than silently succeeding on a repeat call. Documented known side
  effects retain their order, including model failure and Train/TrainCar additions.
- Vehicle shutdown releases all retained blur models and splash handles exactly
  once. Read rejects live blur handles before consuming input, preventing leaks.
- Train now has one ordered car list containing each shared-stream identity and
  optional owned runtime. Save and runtime operations use that same list. Read
  appends unresolved references without inventing runtime cars. Collision rejects
  unresolved cars before effects/random draws; shutdown clears both identities
  and owned runtimes. Writing a runtime car with null identity fails before output.
- Added meaningful Init-error persistence, live-blur Read/shutdown, runtime car
  identity/save and unresolved-collision regressions. Replaced the stale README
  closure table with the current per-owner status.

Production audit commit: `25e7f0dfa`. Final test-fixture lint fix: `7ea565a9a`.
Earlier clarified owner implementations remain in `6eb5508cf`/`5b11c610a`.
Later commits record evidence only; engine source stayed stable during the
corrected full gate and game smoke.

## Verification

- Focused Cargo: **32 matching tests passed**, zero failed (session 91599 terminal).
- Formatting: **11 explicit owned Rust files, zero diffs**; the final fixture
  correction also passed its one-file formatting check.
- Corrected `qbuild all`: **PASS**, terminal session 59704 collected. Workspace
  Cargo summed over all **44 `test result:` lines: 7070 passed, 0 failed, 0 ignored**.
  Clippy with `-D warnings`: **PASS, 0 diagnostics**.
- Native verification: **PASS, 46/46 build edges**, 2.4s; **CTest 80/80**, 187.2s;
  complete native operation 194.0s, job `qbuild-20261024-113023-build-ctest`.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.

Failed evidence is separate: initial focused test 31/32 failed because the new
fixture omitted a reference resolver (corrected fixture, then 32/32 passed),
recorded in `build-runs/round2-focused-failure.txt`. First full gate passed
7070 tests, 256/256 native edges and 80/80 CTest, but failed clippy on one
`field_reassign_with_default`; all four logs are preserved as `*-r2-first.log`.
The prior round's failed clippy log remains `qbuild-clippy-r1-failed.log`.

## Real game

Unique label: **core-vehicles-close11-r2-smoke-20261024-a1-501**.
[Run record](../runs/core-vehicles-close11-r2-smoke-20261024-a1-501.md) (local run 482).
Built DLL SHA prefix `f408380f`; Konradshohe, turbo, target 2027-01-01.
Verdict **clean** at 2027-01-02: exceptions/stalls/rtc/overruns/**ref_warnings=0**;
Rust panics, structured exceptions, shadow INCOMPLETE and other warnings all 0.
Simulation 7.828s, 43.56 days/s; queue wall 25s, slot 1.

Waited once using the event-based local PowerShell waiter, without qwait, and
collected that operation's terminal result. Bounded log grep confirms the clean
AUTOTEST verdict. No requests or checks remain outstanding.

Owner shadow: **n/a**. These portable core owners have no plugin registration or
game shadow path. The run proves DLL/city/shutdown compatibility; it does not
claim owner algorithm conformance. No shadow difference or turbo-only mismatch
was observed or asserted.

## Questions and next steps

- Newly appended **VH11**, `docs/specs/questions/vehicles.md`: loaded car
  identity/runtime binding, missing/duplicate identities and null live identities.
- **VH10** from round 1 remains open; the provisional hooks it identified are
  now removed. VH1/VH5/VH7/VH9 remain the other closure blockers above.
- Coordinator should regenerate the question index with its permitted tooling;
  this lane only edits the allowed question file and does not run `tools/`.
- Obtain those contracts before claiming full owners, then implement them in
  core and add native glue/shadows as a separate scope. No ABI bump in this lane.

The reserved worktree is intentionally detached. Commits succeeded normally;
Git emitted a denied shared `packed-refs.lock` maintenance warning, which was
not worked around. The supervisor publishes the final HEAD to the lane branch.
