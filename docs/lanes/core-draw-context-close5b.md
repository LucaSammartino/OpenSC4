| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-draw-context-close5b (DC9 and owner closure) | Codex / `core-draw-context-close5b` (engine lane, detached HEAD) | WIP: DC9 implemented; 38 owner rows audited, 35 vectors mapped; focused 37/37; cargo 5962/0, clippy 0 diagnostics; CMake configure stalled and stopped after 699.9 s, CTest skipped. No ABI change, game run, or new question. | (1) Resolve native CMake compiler ABI probe stall; (2) run qbuild all with the entire tree fixed; (3) promote queue entry after passing validation. |

## Implementation and closure

`ba033b93` replaces the DC9 fog gap with the settled section 1.5.1 contract.
`turn_on_fog` accepts the enable byte, optional borrowed RGBA, and start/end floats.
Enable, RGB, start and end groups run in spec order and advance independently after
normal driver returns, including ordinary failure values. RGB comparison excludes
alpha, while submission retains the original RGBA reference. Scalar hooks borrow
the respective four-byte argument. No camera, lighting or property query is added.
Sentinels, signed zero, NaNs, noncanonical enable bytes, disabled parameter changes,
and retries follow the published contract. No plugin/capi/ABI changes were made.

`0bc5604e` records the audit of every other owner row. The module README has 38
section-specific rows and maps all 35 published vectors. Its 33 named test
references resolve to existing tests. Sections 13.1/13.2/13.3 are distinguished
from the nine numbered items in section 13. No settled assigned operation remains
unsupported. Core-only push/pop helpers make no native attribution claim, and
legacy Begin/End gap methods remain outside the withdrawn owner requirement.

## Validation

- Focused `cargo test -p fc4-core draw_context`: 37 passed, 0 failed.
- Explicit owned-file qbuild fmt: 3 files, zero diff. Initial relative-path fmt
  invocation failed to find files; absolute paths passed. No formatting was applied
  outside the three owned Rust files.
- `build-runs/audit-draw-context.py`: PASS, 38 rows / 33 referenced tests / 35 vectors.
- One final-Rust-source `qbuild all` operation was collected to terminal FAIL:
  cargo 5962 passed / 0 failed / 0 ignored, clippy exit 0 / 0 diagnostics.
- CMake remained at `Detecting C compiler ABI info`. CMake/Ninja CPU totals did
  not advance between 16:12:40 and 16:19:23. Only this lane's stalled probe processes
  were stopped. Configure reported FAIL after 699.9 s; full CTest was skipped.
- The helper also reported `the tree changed during the run`: documentation-only
  commits were made while validation ran. This attempt is not a passing final gate.
- Evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`, `full-gate-stall.md`, and
  `engine/build-x86/CMakeFiles/CMakeConfigureLog.yaml`.
- No duplicate full check was launched. No game run/shadow is applicable: the
  requested module remains core-only and has no game integration. No request is
  outstanding.

## Questions and next steps

No new question was filed. Only the spec's existing boundaries remain:
DC1 (section 1.1, foreign driver-byte meanings), DC2 (sections 6/9.2, foreign
draw-flag meanings), and DC3 (sections 1.3/1.5, native stack-mutation attribution).
DC9 is closed in code and tests.

Resolve the Windows native CMake ABI probe stall, then rerun the full gate with
the entire tree fixed throughout the operation. Preserve this failed evidence.
The merge queue is WIP until that validation passes; owner implementation itself
is complete against the settled spec. Minimum supported Rust remains 1.75.
