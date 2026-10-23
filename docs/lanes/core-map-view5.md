| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view5: map view owner closure | Codex / core-map-view5 | blocked: cargo 6085/0, clippy clean; CMake configure timed out, CTest skipped | Repair native compiler ABI configuration stall, then rerun the gate with HEAD unchanged; resolve documented spec gaps for native closure. |

## Changes

Implementation commit: `b6e06bb2`. Handoff/results milestone: `c261464a`.
Extended the existing `engine/core/src/map_view` owner, with no plugin/capi/ABI
changes. Added foreign Resources, WindowSystem, Renderer and Simulators traits;
owner execution of initialization, source selection, owned exact-integer palettes,
filter/terrain/lot/zone settings, bitmap presentation, custom-view creation and
shutdown/release, legends, buttons/tutorial filtering, reset and refresh timing.
The owner never obtains completed owner algorithms from a foreign callback.

Added `engine/core/src/map_view/README.md` with one closure-table row per spec
section and subsection. Existing helpers and their tests remain; the new interface
conformance suite has 14 tests. No coverage-map or generated ABI entry changes.

## Measured checks

- Focused existing core map-view tests: **24 passed, 0 failed**.
- Focused new conformance tests: **14 passed, 0 failed**.
- Explicit-file qbuild fmt: **PASS**, six owned Rust files, zero diff lines.
- Required final-source qbuild all: **FAIL**, collected its original session 28866
  through its terminal exit 1; no duplicate full check was launched.
- Workspace cargo: **6085 passed, 0 failed, 0 ignored**, summed across all 30
  `test result:` lines in `build-runs/qbuild-test.log`.
- Clippy `-D warnings`: **PASS**, exit 0, zero diagnostics;
  `build-runs/qbuild-clippy.log`.
- Native CMake: **FAIL**, configure timed out after **900 seconds**, stopped at
  `Detecting C compiler ABI info`. Evidence: `build-runs/fc4verify-configure.log`
  and `engine/build-x86/CMakeFiles/CMakeConfigureLog.yaml`. No source diagnostic
  was emitted. CTest was **skipped**, not passed.
- The wrapper additionally reported `the tree changed during the run` because
  the handoff-only commit `c261464a` was made while the native phase waited.
  `git diff b6e06bb2 c261464a --stat` contains only this lane report; engine
  source was stable throughout. On the next run, hold HEAD as well as source
  unchanged until the full operation terminates.
- Earlier q.py focused invocation failed before cargo execution with Git Bash
  CreateFileMapping / Win32 error 5. Direct PowerShell focused tests succeeded.
  Initial relative fmt paths failed; absolute paths passed. Formatting edits
  were applied through apply_patch, without writing rustfmt filename headers.

## Remaining spec boundaries

The table in the module README maps each boundary to code and discriminating
tests. This report does **not** claim complete native owner closure.

- §1: panel visibility matrix (QV3), native defaults/repeated Init (QV9), native
  failure rollback (QV12).
- §2: undocumented missing/malformed property behavior (QV6), fractional ramp
  rounding/stop/end-point rules (QV8). Exact full-range linear ramps execute.
- §3.1: none/default-filter and reset calls (QV1), unresolved property/ramp/
  highlight domains (QV6-QV8), custom attachment and foreign failure edges (QV12).
- §3.2: group memory and panel policy (QV3); the known 0x5008/0x8001 traffic
  branch executes. Failure continuation remains QV12.
- §3.3: unknown group selection (QV3), same-view reset policy (QV12); the literal
  implementation calls SetDataView with its normal same-view guard.
- §4.1: traffic volume (QV2), crime markers/developer layer 5 (QV4), developer
  default layer 1 and debug-grid construction (QV11).
- §4.2: contradictory 16-bit conversion rules (QV5), invalid float domains (QV10).
- §4.3: actual coarse resampling (QV8), lot/incident/damaged-road markers (QV4),
  owner overlay inclusion/geometry (QV10). Explicit-cell overlay helper remains.
- §5: native highlight comparison and unspecified radii (QV7), hotspot/effect
  contract (QV13). Known raw building/radius acquisition executes every tick.
- §6: missing/malformed legend fields (QV6), title/list cardinality and unequal
  row behavior (QV12); valid legends and no-view title execute.
- §7: unknown highlight-driven dirtiness (QV7), empty/failed-update native clock
  commits and wrapping/defaults (QV9), foreign failure policy (QV12).
- §10.1: original index vector blocked by QV5; unsigned formula gives 127 but
  signed formula gives 64 while the vector requires 65. §10.2 executes end to end.
  §10.3 verifies pollution deadlines but cannot claim the actual 16-bit redraw
  (QV5). §10.4 executes selection with an empty map; disposition of an old bitmap
  remains QV9.

## Questions, game, next steps

Appended **QV12** and **QV13** to `docs/specs/questions/map-view.md`; retained
existing QV1-QV11. The coordinator regenerates the question index.
Game: **n/a**, core-only interfaces; no plugin wiring or ABI exports changed,
no request submitted and no outstanding game label. No game/shadow claim.

The merge queue is **wip** because the native gate did not pass. Diagnose/repair
the native CMake compiler ABI stall before repeating the full check; preserve
the failed logs. Then resolve the specific question domains above before claiming
full native closure or adding plugin integration. No build operation remains
outstanding from this run.
