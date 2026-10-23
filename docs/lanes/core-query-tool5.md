| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-query-tool5: query tool owner | Codex / core-query-tool5 (detached HEAD) | wip: Rust green; CMake configuration stalled; spec gaps explicit | Resolve compiler-probe infrastructure and open query contracts before owner closure |

## Implementation

Added `engine/core/src/query_tool`, reusing the existing view-input `ControlHost`,
Pixel and Vec3 support. The core owns creation/IDs, lifecycle and subscriptions,
hover filters/picking decisions, highlighting, timer/message dispatch, ordinary
terrain/network/building tips, modal query and zone dialog orchestration,
recursive control walks, table dispatch, published numeric/text/money fields,
live funding notifications, edits and building-value helpers. Foreign occupants,
simulators, renderer, locale and windows remain primitive traits. No plugin or
C ABI changes, ABI bump, coverage-map edits, or generated ABI-file edits.

The module README has one closure-table row for every section 8 subsection and
the query conformance cases. Unsupported gaps are explicit effects/results;
hosts are forbidden from implementing them by calling native owner methods.
This is not a claim of complete owner closure.

Implementation milestones: `decbe662`, `64190ae5`; final source `cdb8a662` fixes
the two Clippy test-style diagnostics. No source changed during the corrected
gate. All asynchronous checks were collected to their terminal results.

## Validation

- Focused query fixtures: 21 passed, 0 failed, including all 127 filler slots,
  per-source/alias fixtures, conformance 6–10, slider funding before close,
  cancellation/acceptance, ownership balance, Shift miss and numeric boundaries.
- Corrected stable-source `qbuild.py all`: **FAIL** overall; **5952 cargo tests
  passed, 0 failed, 0 ignored**, **Clippy passed, 0 diagnostics**. CMake configure
  stalled again at `Detecting C compiler ABI info`; after 161.1 seconds its
  identified CMake process (25768) was stopped. No `Build files have been written`
  line; build failed, full CTest **skipped**, no CTest pass count claimed.
  CMAKE_BUILD_PARALLEL_LEVEL=1 did not resolve the compiler-probe stall.
- Current evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-configure.log`. This corrected gate had no tree-changed warning.
- Earlier failed gate: 5952 tests passed; Clippy found bool-assert/default-field
  fixture warnings, subsequently fixed. Configure stalled 607.2 seconds; this
  run's identified ninja/cmake (27324/12488) were stopped; CTest skipped. This
  earlier run reported tree changed because the fixture fixes followed completed
  Rust stages. Evidence retained separately as `*-before-fix.log`.
- Final explicit five-file `qbuild fmt`: PASS, diff_lines=0. `git diff --check`
  clean. Formatting fixes used apply_patch from rustfmt --check hunks.
- q.py could not start Git Bash (CreateFileMapping error 5); initial wrapper
  evidence is `build-runs/q/20261023-154334-6224.log`. Direct PowerShell cargo works.
- Game runs: n/a; core-only owner with no plugin wiring. No game request remains
  outstanding. Native shadow equality and ref-warning counts were not measured.

## Questions and exact open sections

Appended CT18–CT20 to `docs/specs/questions/city-tools-input.md`. Existing CT6,
CT7, CT9, CT13, CT14 and CT15 remain relevant. No questions index or answered
archive was edited, and no analysis-side scripts were read or run.

- **8.4:** legend script/placement/minimisation, CT7.
- **8.6:** vehicle/stoplight action meaning, CT9; route generation, CT15.
- **8.7:** building/network traffic-tip writers, CT20; path redraw, CT15.
- **8.11.3:** native callout interface/argument meanings, CT13 (semantic image
  intent is represented).
- **8.11.4:** debug description writer bodies, CT14/CT20.
- **8.12.1:** complete debug readouts and neighbour dialog, CT14/CT20; empty-cell
  failure-sound conflict, CT18 (conformance's explicit sound requirement used).
- **8.12.3:** landfill null-table versus zone-worker dispatch conflict, CT18
  (occupant entries remain null; zone handler dispatches published workers).
- **8.12.4:** MySim icon, CT14; missing slider related IDs/count quirk, CT18;
  airport noise/luggage, best-developer selection, wealth resize, multipoint
  goofy-curve rules and numeric domain, CT19.
- **8.13:** neighbour kind mapping, window lookup and row fillers, CT14. City-edge
  gate, condition sounds, zone dialogs and notification handler are implemented.
- **8.14:** developer selection, wealth resize k/geometry, commute sampling and
  rectangle endpoint convention, CT19. Published math is implemented; inclusive
  aura rect and zero commercial-capacity result are commented literal choices.
- **8.15:** route geometry, travel-type filters/toggles and zoom thickness, CT15;
  legend CT7. Old-path removal and retained building/cell rectangle are implemented.

## Next steps

Resolve the sandbox/native Ninja compiler-probe stall, then run the required all
gate to obtain a real CMake/full-CTest verdict. Obtain the listed spec answers
before claiming full owner closure. The merge queue entry is **wip**, not ready.
