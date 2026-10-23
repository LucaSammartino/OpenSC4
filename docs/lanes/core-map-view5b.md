| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-map-view5b: QV12/QV13 owner closure | Codex / core-map-view5b (detached HEAD) | wip: implementation complete; Rust 6231/6231 and clippy pass; CMake configure stalled, CTest skipped | Coordinator must resolve host compiler-check stall and obtain the full C++ gate; QV14 requests exact ring numeric mode. |

## Report

`f287bedc` implements the QV12/QV13 additions in the existing map-view owner.
`b41ecbbc` records the completed Rust verification and then-pending CMake stage.
The worktree remains detached as reserved; no shared refs/config/hooks were changed.

### Changes and coverage

- Selection preserves its same-view guard and nontransactional teardown. Custom creation false continues;
  successful creation calls no-argument initialization even when it returns false. Missing child interfaces
  skip only child work. Ordinary false primitives are separate from invalid adapter/service errors.
- Init ignores ordinary base/subscription false results; missing message server leaves its latch set.
- Buttons and visible resets load the requested legend afresh after ordinary selection failure. Core hides
  the eighteen controls and invalidates the first label before lookup; titles use a scalar/default, rows
  retain colour indices, zero colours skip, RGB drops alpha, and interface failures skip individual work.
  Native one-past-end reads receive the explicitly permitted safe diagnostic after earlier swatch work.
- Crime outputs are raw signed cells. Core computes centred world position and terrain altitude, retains
  a null-occupant radius-100 record, and implements both effect-creation stages, ignored false returns,
  temporary release without Stop, repeated-position replacement, disappearance and kind-zero cleanup.
- Independent EF footprints and EE rings decorate core-owned pixels. Core computes signed rectangle
  scaling, clipping, ring arguments using width on both axes, and the published blend. Buffer failures
  skip only that pass/ring; acquired buffers release. Common building-effect/reference helpers are tested;
  building-set comparison is still QV7, not delegated to a foreign callback.
- README owner closure has every spec section, added §5.1/5.2 rows, and an individual mapping for all twelve
  §10.5-16 vectors. No QV12/QV13 Unsupported owner boundary remains. No plugin, C ABI or coverage-map edits.

### Verification and evidence

- Focused core map-view: 24 passed; conformance: 27 passed, including all twelve new vectors plus common
  building effect/reference checks. First conformance run was 26/27: the old renderer fixture asserted a
  2x1 bitmap; the new 32x24 vector exposed it. Corrected to validate dimensions against pixel count.
- Owned-file qbuild fmt: six explicit files, pass. Initial relative-path invocation used engine-prefixed
  paths despite the formatter running in engine; subsequent absolute paths passed. Local formatting used
  apply_patch and engine/rustfmt.toml. Diagnostics retained in build-runs/focused-failures.txt.
- Required qbuild all was run exactly once on final Rust source and collected to its actual terminal
  result (session 99759, exit 1). Cargo: 6231 passed, 0 failed, 0 ignored across 32 result lines. Clippy:
  exit 0, diagnostics 0. Logs: build-runs/qbuild-test.log and build-runs/qbuild-clippy.log.
- CMake: FAIL, configure 644.5 seconds, no `Build files have been written` line; CTest: skipped, 0 tests
  executed because build did not pass. CMake identified MSVC then stalled in its compiler ABI try-compile
  before project compilation. The configure log and generated CMakeConfigureLog.yaml stopped after the
  compiler-identification/showIncludes prefix. The corresponding Ninja was idle; Win32 process-parent
  diagnostics were denied by the host. Stopped this lane's identified cmake PID 14788 after >10 minutes,
  then collected the same qbuild operation to terminal. No duplicate gate or quick build was launched.
- CMake evidence: build-runs/fc4verify-configure.log and engine/build-x86/CMakeFiles/CMakeConfigureLog.yaml.
  The verifier also reports `the tree changed during the run`: b41ecbbc changed only this progress document,
  with no Rust source or build-input change during verification. This is not a passing full gate.
- Game runs/shadow/ref warnings: n/a. This core owner is not wired to the plugin/capi; no game request was
  made and no native exact-shadow claim is made. ABI exports unchanged.

### Questions and remaining closure

Filed **QV14** at docs/specs/questions/map-view.md: the spec says x87 precision but does not give its
precision-control/rounding-control modes and operation rounding sequence. Ring formula code preserves
the explicit binary32 stores and uses binary64 intermediates as a documented numeric choice; the published
geometry/blend vectors pass. Arbitrary native edge-pixel/weight equivalence remains open. The coordinator
must regenerate the generated questions index; this clean-room lane may not run its tools script.

Remaining open sections: §1 QV3/QV9; §2 QV6/QV8; §3.1 QV1/QV6-8; §3.2-3.3 QV3;
§4/4.1-4.3 QV2/QV4/QV5/QV8/QV10/QV11; §5/5.1 QV7; §5.2 QV10/QV14;
§6 QV6; §7 QV7/QV9; §10.1/10.3 QV5 and §10.4 QV9. §10.5-16 are implemented and tested.

Next: resolve the host compiler ABI-check stall, then obtain a full passing gate before marking ready.
No engine-source work remains for the twelve supplied vectors; exact ring precision awaits QV14.
