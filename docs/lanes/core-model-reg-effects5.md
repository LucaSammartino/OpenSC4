| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-model-reg-effects5 (model effects owner closure) | Codex / core-model-reg-effects5 | **wip: owner implementation closed**, cargo 6106/0, clippy clean; native gate blocked in CMake compiler ABI detection, CTest skipped; ABI unchanged | Rerun native build/CTest in a working Windows build environment before landing |

## Implementation

Extended the existing `engine/core/src/model_reg_effects` module, rather than
duplicating it. The module README contains the Owner closure table for all owner
sections: graphics-formats §3.11 Consumer 1, §3.11.1–3.11.4, §8.2 vector 37
(existing parser reused), and §8.5 model-effects vectors 48–49. No owner section
is left Unsupported or open. No new questions were needed; current spec prose
answers the historical GF19–GF24 entries, which this lane left untouched.

Changes cover constructor defaults, Init/Shutdown, same/null/new attachment,
borrowed instance/effect getters and REGP vectors, ordered discovery/replacement,
full 32-bit controls and effect-owned IsStarted, CRT byte-table name comparisons,
named start/stop/toggle/recreate and fallback creation, independent creation
boolean/output branches, null entries, manager preconditions, removal/clear and
the specified distinction between Shutdown and destruction. Motion forwards a
three-component parameter 15 after each started effect's transform. Transform
updates implement zero/nonzero flag shortcuts, flag propagation, output version
zero, scale/composition order, normalization order and normalized-component
binary32 stores. The GF13 zero-axis identity choice remains explicit.

REGP parsing remains in `fc4_formats::s3d`; no new parser was needed. A new
conformance integration test passes real parsed/lower-cased/duplicate-appended
REGP data into the owner, checking borrowed-vector identity and transforms. The
existing formats integration test was updated to the completed core contract.
No plugin/capi changes, ABI exports/version changes, coverage-map edits or game
requests. Native layout-specific map allocation/rebalance helpers are represented
by Rust ordered-map/string ownership, not new raw-pointer APIs. Destruction
deliberately retains effect references when Shutdown was omitted, per the spec.
Portable numeric results are checked at the specified 1e-6 tolerance; x87 bit
identity is not claimed. Undefined native record reads use the spec's checked
missing-record diagnostic, without clamp/wrap/identity substitution.

## Validation and evidence

- Focused core tests: 12 passed. New conformance integration: 1 passed. Existing
  formats adapter integration after correction: 1 passed.
- Scoped formatting checks: all five owned Rust files clean. `git diff --check`
  clean. Rust workspace declares rust-version 1.75; no newer features introduced.
- First `qbuild.py all`: stale formats adapter produced 11 workspace/clippy
  compile errors. Corrected in 2758f8e8. Its CMake compiler ABI check stalled;
  this lane stopped only its configure PID 6220 after over eight minutes and
  collected the original operation to terminal FAIL (513.3s). Original full
  logs preserved in `build-runs/failed-gate-1/`.
- Final corrected-source `qbuild.py all`: **cargo 6106 passed, 0 failed,
  0 ignored** (31 result summaries); **clippy exit 0, 0 diagnostics**. CMake
  again stalled in compiler ABI detection. The scratch C compiler object existed
  and Ninja dry-run listed compile/link work, but no completed ABI result arrived.
  This corrects the initial handoff's unverified inference that no object existed
  in the first attempt. Only this run's configure PID 8596 was stopped; original
  operation collected to terminal **FAIL (253.5s), CTest skipped**. The final run
  did not report source changing during verification.
- Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`. Initial unrelated q.py launch failed on Bash
  CreateFileMapping; focused test compilation also caught/corrected an E0597 test
  fixture declaration order. Formatting was applied with apply_patch from
  rustfmt --check diffs, never redirected stdout into source.
- Game/shadow: n/a; this task explicitly excludes plugin/capi wiring and has no
  game-facing switch or shadow hook. No request outstanding.

## Commits and next step

- 4444173e: initial lane handoff.
- 6b1e4e59: implementation, owner closure table, focused tests and conformance.
- 2758f8e8: existing formats adapter correction and failed-gate evidence record.

Implementation closure is complete, but the lane is **wip**, not ready to land:
the coordinator must rerun native build/full CTest in a functioning Windows build
environment. No further source/spec work is identified. Worktree remains reserved
and detached; the supervisor publishes the commits to the lane branch.
