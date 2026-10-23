| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-vehicles5 (vehicle owner closure) | Codex / `core-vehicles5` (engine lane, detached reserved worktree) | WIP: owner closure incomplete. Cargo 6244/0, clippy clean, CTest 68/68; focused core 14/14, conformance 1/1. No ABI or plugin changes. VH3-VH9 filed; VH1-VH2 remain open. | (1) Resolve listed spec gaps; (2) complete open closure-table rows before claiming owner coverage |

## Report

The four requested owners are **not closed**. This is a tested core subset and
an explicit gap inventory, not a ready owner-coverage claim. The per-owner,
per-section table is in `engine/core/src/vehicles/README.md`.

Implementation milestone `20b5b702` adds a sibling `vehicles` module. It reuses
automata's records and state stack, rather than adding duplicate codecs or
calling the game's owners. Traffic has route setup/look-ahead, tile/route-order
leader selection, collision and blocked waits, tween advancement and network-
facing path/straight movement. Vehicle adds back-anchored destinations and
measured speed, dispatcher/state handling, loaded plummet/splash, pull-over,
collision lifetime and atomic city-free Read. Known train behavior covers owned
car teardown, measured-speed copying and the minimum collide timeout. Native
constructor flags, train/car records and train generation/following/trips remain
explicit Unsupported results. No fake native layouts or delegated original
vehicle implementation is supplied.

Formatting milestone `7954b9ed`; clippy fixture correction `e788e951`. There are
no C ABI exports, ABI version, plugin registrations or coverage-map changes.

## Validation

- Focused core: 14/14 PASS (`build-runs/vehicles-focused-2.log`). Initial attempt
  preserved in `vehicles-focused-1.log`: 13/14; corrected expected occupant
  loaded-status bit, which the shared codec sets on Read.
- Conformance: vehicles §11.4 PASS 1/1 (`vehicles-conformance-1.log`): nearest
  eligible leader ignores height, and route tile order wins over global distance.
  Vehicles §11.1-3 belong to emergency/fire-truck owners outside the named four.
- Owned-file fmt: six files PASS; changed test fixture file rechecked PASS.
- First full gate: cargo 6244/0, clippy failed three fixture initializer lints;
  native build 217/217 and CTest 68/68 PASS. Failed clippy evidence is preserved
  in `build-runs/vehicles-all-1-clippy.log`. Queue label
  `qbuild-20261023-194941-build-ctest` completed; no duplicate was started.
- Corrected-source full gate: **qbuild PASS**, cargo 6244/0, clippy zero
  diagnostics, native build 49/49 (1.7s), CTest 68/68 (75.2s).
  Queue `qbuild-20261023-195939-build-ctest` completed. Current full logs:
  `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log`. The native wrapper's tree-change note is solely the
  README closure-table expansion; Rust/build inputs stayed fixed throughout.
- Game/shadow: n/a. No plugin/capi integration, no game request or outstanding
  native operation. This does not claim a runtime shadow or game validation.

## Open sections and next steps

New questions are appended to `docs/specs/questions/vehicles.md`; the coordinator
must regenerate the question index using the analysis-side workflow.

| Questions | Sections still open |
|---|---|
| Existing VH1 | vehicles §3/§5 player driving, physics and derail |
| Existing VH2 | vehicles §2 neighbour ranking and look-ahead helper contracts |
| VH3 | vehicles §1/§5, automata §6.4 train/train-car wire layouts, versions, reconstruction/failure |
| VH4 | vehicles §2/§3/§5 construction, inherited Init flags/acquisitions/unwind |
| VH5 | vehicles §5 car selection/length/colour, following/sounds, StartTrip/failure; managers §5 car placement |
| VH6 | vehicles §3/§5 collision direction/transform, damage event/model mapping, motion blur |
| VH7 | vehicles §2/§3 route representation/geometry/failure and lane bits; managers §5 speed/route helpers |
| VH8 | vehicles §3 via automata §6.5 plummet entry conflict/splash fade, PullOver loaded-vector reconstruction |
| VH9 | inherited automata §6.5 active state/controller details and ordering, §7.5 full setters, dispatch §4.3 base contracts |

Resolve those contracts and implement the remaining table rows before claiming
any of the four owners closed. Keep the lane WIP; it is not ready to land as a
completed engine-owner task. Worktree is reserved and detached; the supervisor
publishes commits to the lane branch. No refs were moved by this lane.
