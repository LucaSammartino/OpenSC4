| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-legacy-control5: legacy control owner | Codex / core-legacy-control5 (detached HEAD, reserved lane branch) | wip: owner complete for defined inputs; Rust gate passes; CMake compiler probe stalled, CTest unrun | Resolve compiler-probe infrastructure and rerun full gate before landing |

## Report

Extended the existing `engine/core/src/legacy_control` owner. Implementation
milestones: 71d76437 and 40c2deb9. The README contains the complete owner-closure
matrix and LCV01–LCV33 test index. All 57 published owner-function contracts are
implemented for their defined inputs (14 COM/lifecycle/accessors, SetParameters,
GetScriptFragment, 21 vocabulary functions and 20 owned interpreters). The shared
Custom stub also remains implemented. Existing vocabulary, lifecycle, build,
common, Slider/TreeView/FileBrowser/Grid/Folders/FlatRect/Outline implementations
were retained and audited against their sections.

Added Text, TextEdit, ListBox, Combo, Btn, BMP, Scrollbar, LineInput, Spinner,
TextTicker, SC3WinGen and OptGrp passes. Effects include metadata after failed
resource loads, exact boolean rules, preserving unrelated flag bits, ordered
population and sorting, presence-based autofit, alpha binary32 arguments,
wrapping selection subtraction, unsigned SC3 record IDs, and adding every option
before any option geometry. The SC3 default procedure is allocated through an
individual foreign primitive and installed before the caller procedure handoff.
Layout facts are explicit constants, not a Rust memory-layout assertion.

Windows, rendering, input dispatch, script/resource loading and procedure
callbacks remain foreign operations. No plugin/C ABI wiring, ABI export changes,
ABI version changes, generated ABI entry edits or coverage-map edits.

### Validation

- Final source: 40c2deb9; source has not changed since the final full check started.
- Formatting: qbuild fmt PASS, 8 explicit owned Rust files, zero diff lines.
- Required `qbuild.py --wt <WT> all` was launched once on final source. Its same
  operation (session 54726) was collected through termination; no duplicate full
  check was launched. Cargo passed: **6092 passed, 0 failed, 0 ignored**, summed
  over all 30 `test result:` lines; **34 legacy-control tests passed**. Clippy
  `-D warnings` passed (finished successfully, no diagnostics).
- CMake configure identified MSVC C and C++ compilers, then stalled at
  `Detecting C compiler ABI info` from 17:18:19 for over ten minutes. No cache,
  build or CTest verdict was produced. Interrupted the original lane operation
  with Ctrl-C; it terminated with exit 1. **CMake incomplete; CTest not run**.
  This lane is held wip; it is not a passing full gate.
- Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`,
  `build-runs/fc4verify-configure.log`; local compiler-probe diagnostics in
  `engine/build-x86/CMakeFiles/CMakeConfigureLog.yaml` and CMakeScratch.
  Additional evidence: `build-runs/legacy-control-final-gate.txt`.
- Earlier focused diagnostic: 31 passed / 1 incorrect BMP fixture expectation.
  Corrected the fixture to supply the spec's 40×20 image; final tests passed.
  Preserved in `build-runs/legacy-control-focused-before-fix.txt`. The earlier
  q.py wrapper failed before cargo (Git Bash CreateFileMapping error 5), with
  its own separate log under `build-runs/q/`.
- No game requests: isolated core implementation has no shadow/replacement switch
  or plugin wiring. No native/pixel equivalence or ref-warning result is claimed.

### Questions and remaining boundaries

No new questions filed: existing LC5 and LC8–LC13 cover the unsettled domains.
Exactly these section boundaries remain open, rather than silently successful:

- §6: LC5 default-parent identity (opaque manager slot-3 forwarding implemented).
- §7.6: LC8 per-type fragment writer words (non-owner; partial coverage reported).
- §§7.5, 8–9, 13.3: LC9 overflow/non-UTF8 domain and specified deterministic choices.
- §§9.2, 9.5: LC10 duplicate maps and indeterminate Grid scratch/resource domain.
- §§9.11, 9.14–9.20, 13.3: LC11 unchecked indeterminate scan arguments and OOM.
- §§9.20, 13.1: LC12 external default-procedure callback semantics; allocation and
  installation ordering are implemented.
- §9.9 and its font branches in §§9.10–9.14, 9.17–9.20: LC13 named-font helper
  payload/indirect application. These return `Unsupported::Open`. Numeric fonts,
  defaults and older explicitly specified String metadata branches are complete.

Next: diagnose the CMake/Ninja compiler ABI probe in the execution environment,
then run one full gate after that real input correction. Do not merge this wip
queue entry until the full build and CTest have an actual passing verdict.
