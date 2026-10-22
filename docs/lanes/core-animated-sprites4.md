| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animated-sprites4: animated sprite owner | Codex / `core-animated-sprites4` (engine lane, detached worktree) | WIP d3c758b1; cargo 4306/0, clippy 0 diagnostics, ctest 67/67; 29 new tests; AS3-AS9 filed; no ABI or game run | 1. Obtain AS1 and AS3-AS9 contracts. 2. Complete native owner methods and conformance before promotion. |

## Report

**WIP, not a complete owner replacement.** The published spec does not contain
the requested full 60-function inventory or enough behavior to implement the
remaining native owner methods without guesses. The queue entry is explicitly
WIP. No partial implementation is promoted as complete or replacement-ready.

Implemented in `engine/core/src/animated_sprite/` (code commit `d3c758b1`):

- Raw-unit unsequenced animation clock, explicit frame modulo, loop/end/reverse
  interior, prescribed frame-skipping/no-skipping residual quirk, and dirty state.
  Unpublished sentinel/reverse-end/non-progressing cases return explicit errors;
  no conversion of raw timing units and no placeholder native success codes.
- Typed foreign animation/zoom/table/frame/texture records, counted acquisition
  of per-zoom table data through IID 0x69B6F01C, stop-first-missing prefix results,
  direction equation, decoded frame lookup, UV flips and scale epsilon.
- Published constructor material constants, raw highlight/fade/self-lit flags,
  visibility suppression at zoom 0/1 or highlight, and owned basic-instance data.
- Safe Rust 16-bit lifetime arena with explicit initial count, basic-interface
  AddRef, last-release destruction, counted owner replacement and resource drops.
- Published transform-kind algebra and inverse rotation, metric billboard and
  bounds-widening primitives, and first-hit fan-triangle picking orchestration.
  These helpers do not claim the unpublished complete UpdateView/Draw/Pick behavior.

`engine/core/src/animated_sprite/README.md` reconciles every method actually named
by the public spec and distinguishes implemented components from missing native
bodies. The unpublished 60-function list cannot be invented. No CAPI, plugin,
ABI, maps, resource/factory owner, or model_instances module was changed. Foreign
providers return data or TrianglePick primitives, never original sprite-owner
decisions or computed quad output. No forbidden analysis source was accessed.

## Measured verification

- Coordinator's fresh baseline accepted; no pre-edit full gate rerun.
- Focused `qbuild.py quick`: PASS, fc4-core **2439 passed / 0 failed**;
  fc4-capi/fc4-conformance/fc4-formats compile-only; CMake **206/206 edges**.
- Six new Rust files: wrapper rustfmt check PASS. Touched `lib.rs` check exposes
  existing module-order/format drift; unchanged baseline independently also fails
  rustfmt. No unrelated mass formatting. Evidence:
  `build-runs/animated-sprite-baseline-fmt.log`.
- One stable final `qbuild.py all`, source fixed at `d515bdfb`: PASS.
  Workspace cargo **4306 passed / 0 failed / 0 ignored**; clippy **0 diagnostics**;
  CMake **4/4 edges**; full ctest **67/67**, **125.5 seconds**.
- Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- **29 new Rust tests** include five prescribed component vectors and independent
  1/2 timing thresholds, raw-unit/residual clocks, asymmetric frame/base/UV/flags/
  billboard axes/offsets, noncommuting transforms and inverses, empty/failure paths,
  reference overflow/replacement/final-release cleanup and pick short circuit.
  Zoom vector checks visibility; it does not claim a complete draw-command vector.
- Game: **n/a**, no plugin surface or live shadow; no game requests outstanding.
  ABI exports: **unchanged**.

## Questions and remaining work

Filed **AS3-AS9** in `docs/specs/questions/animated-sprite.md`; AS1/AS2 pre-exist.
AS3 requests the owner inventory/defaults/lifetime/transform setter contract;
AS4 loading results/state/reference rules; AS5 sequence stepping, sentinel rates,
raw clock edge cases and returns; AS6 orientation/camera/frame-layout rules;
AS7 quad spaces/caches/draw/pick/bounds/lighting details; AS8 fade flag edges;
AS9 derived public bounds/sort/resource methods and thunks.

Obtain those public contracts, complete the missing owner decisions, add complete
owner conformance vectors, and gate the finished implementation before promotion.
Do not interpret the passing component tests as 60-function completion.

Milestones: `b7843313` lane metadata; `d3c758b1` code/tests/inventory/questions;
`d515bdfb` focused gate and formatting evidence. The final report commit follows
the final gate and changes documentation only. Worktree remains detached for the
supervisor to publish; no branch switches, merges, pushes or self-restarts.
