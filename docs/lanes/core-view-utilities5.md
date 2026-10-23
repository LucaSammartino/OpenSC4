| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities5 (SC4ViewUtilities owner closure) | Codex / `core-view-utilities5` | in progress | Audit existing module against current spec; close all settled behavior and conformance coverage |

## Handoff
- Goal: close SC4ViewUtilities in portable core only; no ABI/plugin changes.
- Existing module found at engine/core/src/view_utilities; includes prior inventory and VU12/VU13 numeric limitations.
- Done: audit recorded in 27dd3666; added three explicit §12 cached getters and tests for constructor/stale reads.
- Done: Owner closure table covers all spec sections; conformance map covers 21 vectors with explicit checked/numeric limits.
- Done: exact §11.5/16 city, §11.18 integer stage/ties and §11.20 resource-order vectors added.
- Focused tests: 46 passed, 0 failed (build-runs/q/20261023-132012-9632.log); final tie assertions await full gate.
- Formatting: absolute-path owned-file check passed; first relative-path invocation failed to resolve paths (no source failure).
- Source milestone: a18c9282 includes final source and conformance assertions; no source edits after full gate launch.
- Required qbuild all IS RUNNING: exec session 37922. Collect THAT session with write_stdin (60000 ms); never restart it for progress.
- Full gate so far: cargo 5875 passed / 0 failed (27 result summaries), clippy -D warnings clean.
- Logs: build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-configure.log, fc4verify-build.log.
- Native build log reached [217/217] link steps; cmake/ninja/link still active, no CTest result yet. No terminal PASS/FAIL.
- Next: collect session 37922 to terminal result, then report/commit/queue if clean; diagnose actual failures before any rerun.
- Source: docs/specs/view-utilities.md; existing VU12/VU13 remain open; appended VU15 for unspecified block scalar width / unrepresentable float32 vector 18.
- No new ABI/plugin wiring. Native numeric completion remains open in §2.2 (arithmetic §§2–7), §§7.1/12 index extremes, and §§4.2/4.4/11.18/12 scalar input width.
- No game requests; core-only scope.
- Required final gate: qbuild.py --wt <WT> all, once stable.
