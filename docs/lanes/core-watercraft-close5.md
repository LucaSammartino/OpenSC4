| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft-close5 (watercraft owner closure audit) | Codex / `core-watercraft-close5` (engine lane, detached HEAD) | WIP: owner remains open under WC3–WC8. cargo 5861/0, clippy 0 diagnostics, CTest 68/68; watercraft 39/39. ABI unchanged; game n/a. | (1) Resolve WC3–WC8 and finish open sections; (2) coordinator regenerate question index; (3) review owner table before claiming coverage. |

## Report

This lane does **not** claim complete `cSC4Watercraft` owner closure. The spec
still lacks executable contracts for several owner calculations and lifecycle
branches. All spec sections, subsections, and five public conformance vectors
are mapped in `engine/core/src/watercraft/README.md` under **Owner closure**.

### Changes

- `432e5530`: implemented literal §6.2 gear targets, braking through zero,
  bounded speed/steering changes, fire success bookkeeping, skier running-state
  controls, tow-button ordering, speed-tween reset and inherited-translation
  request. Added geometry-aware state dispatch that consumes completed dock and
  undock ticks with `PopState`. Added section-by-section owner table and WC8.
- `d3c37207`: persisted selected gear, applied the braking rate while slowing
  toward a same-direction target, and added regression coverage for towing,
  following target, backwards/still timers and inherited AEMS lookup.
- Nine new core tests; 39 total watercraft tests pass. Commands accept only
  foreign effect playback feedback, never native boat computations.
- No capi, plugin, ABI, generated ABI entries, or coverage-map changes.

### Verification

- Focused cargo test before the final four regressions: 35 passed, 0 failed;
  `build-runs/q/20261023-115009-3052.log`.
- Explicit-file formatting passed for the five owned Rust files. Initial
  relative-path fmt invocation failed to resolve its inputs; rerun with absolute
  paths exposed formatting differences, which were applied with patches.
- One stable-source `qbuild.py all`, collected to its actual terminal result
  (exec session 69040, now finished): **qbuild PASS**.
- Workspace tests: **5861 passed, 0 failed, 0 ignored**, including **39/39**
  watercraft tests; `build-runs/qbuild-test.log`.
- Clippy: **exit 0, 0 diagnostics**; `build-runs/qbuild-clippy.log`.
- CMake: **PASS, 217/217 edges, 92.5 s**;
  `build-runs/fc4verify-build.log` and `fc4verify-configure.log`.
- CTest: **PASS, 68/68, 129.4 s**; `build-runs/fc4verify-ctest.log`.
- Full verifier elapsed **1126.4 s**. It marked **the tree changed during the
  run**: the already-present source was committed and the handoff was updated
  while checking. No Rust source bytes changed after the full gate started.
  The lengthy post-link wait resolved without stopping or replacing the check.
- Game/shadow: **n/a**. This module has no plugin wiring; no game request was
  made and none is outstanding. No shadow-exact claim is made.

### Questions and remaining owner sections

Appended **WC8** to `docs/specs/questions/watercraft.md`, recording literal choices
for the summarized command branches (direct signed target, neutral braking,
no leftover braking time, steering sign and simultaneous inputs, multiplicative
counter-steer boost, skier toggle at rest). Exact effect placement and inherited
command translation remain WC6. The generated question index was not edited:
the lane is authorized to append only the spec's question file.

The sections still open are:

- **§1**: acceptance of record versions other than documented v2 (WC3).
- **§3**: unnamed fourth Init flag, inherited Init and resource-failure policy
  (WC3).
- **§4**: complete crash/following/far-docking state ticks and blocked/on-land
  movement results (WC4, WC5, WC7).
- **§5.1–§5.5 and §5.7**: full animation and AI/position/tilt arithmetic,
  wandering geometry/backtracking, compressed route format, berth ownership,
  degenerate/blocked docking and undocking branches, footprints, size classes,
  inanimate collisions and bump response (WC4, WC5, WC7). WC2's external caller
  and unidentified parameter uses remain outside an executable owner contract.
  **§5.6** is implemented.
- **§6.1–§6.3**: complete inherited/player motion and collision tick, exact
  effect placement and inherited commands, following translation rate/radius,
  and confirmation of literal summary choices (WC4–WC8).
- **§7**: nearest-boat distance ties (WC7).
- **§8**: actual crash-sequence computation after the specified idle fallthrough
  (WC5); idle counters and the fallthrough itself are covered.
- **§11 vector 4**: threshold classification is covered for 0.12/0.18, but full
  two-boat bump/crash response is open (WC5). Vectors 1, 2, 3 and 5 are covered.

The coordinator should resolve the above spec questions before claiming owner
coverage, regenerate the question index, then finish the explicit open table
entries. Unsupported calculations remain explicit gaps rather than native
provider calls or successful placeholder ticks.
