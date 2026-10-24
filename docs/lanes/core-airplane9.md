| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-airplane9: cSC4Airplane core | Codex / core-airplane9 (detached reserved lane) | in progress: round 2 merge fix | Validate both aircraft owners and enqueue ready |

## Handoff

- Goal: merge main, retain airplane and helicopter owners, remove duplicate AF9, run final gate.
- Done: merge f6da6ee6d retains both owners; airplane question is AF10; index/check and three-file absolute-path fmt pass.
- First gate terminal FAIL: qbuild-20261024-094908-build-ctest; Rust tests 7021/0/0, clippy unused imports, native build PASS 216/216, CTest 79/80 (capi_smoke cached ABI97 while both sources specify ABI98). Full logs preserved in build-runs/r2-before-unused-import-fix/.
- Fix: unused imports removed; absolute-path fmt PASS for five files. Focused capi_smoke recache diagnostic running as session 24439; script build-runs/refresh-r2-smoke.cmd.
- Next: collect diagnostic, then run qbuild all once on corrected stable source. Do not commit while final gate runs (HEAD change invalidates its verdict).
- Owners: airplane in engine/core/src/automata/airplane; helicopter in engine/core/src/helicopter; shared aircraft storage/helpers retained.
- No game request outstanding; core-only airplane has no native observation adapter.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-airplane9-r2-manifest.json.

## Implementation

The module path for the coordinator's implemented column is
`engine/core/src/automata/airplane/mod.rs`. It owns `Airplane` and
`AirplaneDriveParameters`, preserving the existing public re-exports. The
shared aircraft/home-placement codec remains in `automata/aircraft.rs`; shared
base/state-stack behavior remains in `automata/automaton.rs`.

Implemented or corrected all settled airplane contracts: exact encoded parameter
defaults, named-field import and whole-record ancestor lookup, signed-zero and
non-finite parameter preservation, independent explicit parameter sets, home
placement/loaded flag and nested record/version/error paths, independent flight
phase/ammo/LOITER state, CHASE rejection and CROP_DUST PopState. The existing
same-input owner record round-trip and every-byte truncated-read tests remain.
The shared station helper now follows the settled signed-count gate and wrapping
32-bit availability masks rather than rejecting oversized counts.

The new module also exposes the specified finite-input time conversion,
pre-force 3D velocity limiter, candidate integration, ground projection, scalar
lift curve and angle, pressure, drag normalization, gravity, rolling resistance
and reverse-throttle gate. These numerical stages are independently testable;
they do not pretend to complete the unresolved force frame or commit a flight
tick. No foreign trait implements the owner's missing control/physics laws.

The module README contains the Owner closure table, mapping sections to code,
tests or open question IDs. Aircraft conformance vectors 1–6 and 8–13 are covered;
vector 7 belongs exclusively to the unassigned helicopter owner (§4.1).
There are 16 shared aircraft tests and 10 airplane tests in conformance.
No plugin, capi, C ABI version/export or coverage-map edit was made.

## Validation

| Check on final Rust source | Result | Evidence in build-runs/ |
|---|---|---|
| Owned-file qbuild fmt | PASS, five explicit files, zero diff lines | Terminal verdict |
| qbuild all workspace tests | PASS, 6994 passed / 0 failed / 0 ignored across 43 result lines (summed independently) | qbuild-test.log |
| qbuild all clippy -D warnings | PASS, zero diagnostics | qbuild-clippy.log |
| qbuild all queued native build | PASS, 207/207 edges, 96.8 s | fc4verify-build-all-final.log |
| qbuild all full CTest | PASS, 79/79, 188.4 s | fc4verify-ctest-all-final.log |
| Separately required qbuild verify native build | PASS, 46/46 edges, 2.5 s | fc4verify-build.log |
| Separately required qbuild verify full CTest | PASS, 79/79, 89.6 s | fc4verify-ctest.log |

Every asynchronous check was collected to its actual terminal PASS. Final full
gate queue: `qbuild-20261024-090054-build-ctest`; separately requested native
verification: `qbuild-20261024-091322-build-ctest`. No operation remains running.

The first full gate also passed (6993 tests, native build 253/253, CTest 79/79),
but a final spec audit found the inherited CROP_DUST handler contradicted the
explicit PopState rule in automata §6.5. Commit `8f75ad6fe` corrected that behavior
and added its regression test, justifying the second full gate. The first run's
logs are preserved separately under `build-runs/pre-crop-dust/`, queue label
`qbuild-20261024-084314-build-ctest`. The initial focused test operation passed
25/25 before relocation/correction and is not used as final-source evidence.
The quiet Bash wrapper failed to initialize (CreateFileMapping error); its
evidence remains at `build-runs/q/20261024-083301-34388.log`. Direct cargo and
both qbuild gates succeeded afterward.

## Open owner contracts and questions

This is ready to review/land for the settled owner contract, not an operational
end-to-end native flight replacement. The spec itself leaves these sections open:

| Section | Remaining contract | Question |
|---|---|---|
| aircraft §2, §3.1, §3.4 player path | Native playerdrive selection, controls, takeover, weapons | AF5 |
| aircraft §3.2 | Probe geometry, full Init/Shutdown order, allocation/failure cleanup, effects and sounds | AF4 |
| aircraft §3.3 and automata §6.5/§6.6 | AI steering and goal transitions, destination sentinel/arrival, runway/approach/fire queries and failures, LOITER boundary/restore/empty ammo | AF3 |
| aircraft §3.3/§3.4 | Aircraft dispatch transitions and message callbacks | AF6 |
| aircraft §3.4, §3.4.1 correction stage, §3.4.2 complete force | Attitude/lift frame, angular update, terrain/runway/collision corrections and complete animation/physics tick | AF2 |
| aircraft §3.4.1/§3.4.2 non-finite inputs | Unordered comparisons, normalization and caller cap behavior beyond the stated finite-input contract | AF10 |

AF2–AF6 already existed and were reused. New **AF10** was appended to
`docs/specs/questions/aircraft.md`, with literal comparisons documented in code.
Round 2 regenerated and checked the question index using the explicit task
exception for tools/questions.py. Helicopter retains its own AF9 question.

## In-game shadow and next steps

No game requests were made and no shadow/ref-warning claim is made. An in-game
shadow is not directly runnable from this core-only module: a separate native
observation seam/adapter must first expose matching inputs and outputs. With
that seam, compare loaded parameter records, capped seconds, pre-force velocity,
scalar terms and pre-correction candidate motion with the specified tolerances.
A trajectory/AI/effect shadow also requires AF2–AF6 to be answered and implemented.

The coordinator can review the owner table and claim the module path, regenerate
the AF10 question index, and route the remaining questions to spec authors. A
separate helicopter lane should consume §4.1/vector 7 and audit its default
CROP_DUST behavior; that owner was deliberately outside this assignment.
