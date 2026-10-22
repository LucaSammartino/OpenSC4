| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-managers4 (automata manager and record owners) | `Codex` / `glue-automata-managers4` (engine lane, detached HEAD) | Blocked on complete owner contracts; questions AD3/AD4 and AT22/AT23 committed. Baseline and final gate PASS: cargo 4213/0, clippy 0 diagnostics, ctest 67/67. No code or ABI changes; no game run or whole-owner credit. | (1) Publish missing lifecycle/body/ABI contracts. (2) Implement complete inherited owners through concrete routes, add independent boundary tests, fullgate and actual exercised 2032/ref0 trace. |

## Outcome

No bounded complete requested owner is supportable from the current published contracts. The route audit and
questions are the deliverable; replacement remains blocked. No engine/plugin/test files changed, no ABI exports
changed, and no enable key, default or map promotion was added. No same-owner native forwarding was introduced.
The existing default automata setting is untouched. Audit/questions milestone: `c43e9b1a`; baseline record:
`22c45031`. The route decision was made before call 30 and before writing any wrapper.

## Concrete routes and inherited owners

The director's 30 factories are enumerated in automata-managers.md §1. None of the four requested base-owner
names has a separate factory there. Absence from this list does not prove C++ abstractness; AD3 asks which
slots are abstract. No new base CLSID is inferred.

| Inherited owner | Published concrete routes | Finding |
|---|---|---|
| cSC4AutomataManager | vehicle A98F9D3B, pedestrian C98F9D17, aircraft C98F9D4A, watercraft A98F9D28 | Base class methods inherited through concrete managers. |
| cSC4TrafficAutomataManager | vehicle A98F9D3B, pedestrian C98F9D17 | Adds slots 68–91; population/creation and lifecycle incomplete. |
| cSC4AutomataController | attractor 6A1628BA, generator 4A1628C5 | Record support does not supply live base lifecycle or resolve accessor offsets. |
| cSC4Automaton | vehicle 896E714A, pedestrian 896E75AF, aircraft C96E75DC, watercraft 096E75F5 and listed variants | Aircraft is explicitly a concrete factory using the automaton constructor; constructor reuse does not establish a base factory. |

City Init asks for manager IID 098F9BF4, controller-manager IID CA14C9FC and script-system IID 8A1B6647
(automata.md §15). Existing QueryInterface branches already answer these IIDs. The historical CLSID/IID
creation fix is already present; repeating it would not replace a live owner.

## Existing path boundaries

- `engine/plugin/FC4Subsystems.cpp` kAutomataClasses registers the six existing concrete city components.
  `cFC4Automata.cpp` CreateManager supplies the four manager CLSIDs to one record adapter.
- `FC4AutomataInterfaces.h` manager view stops at slot 37. The published inherited class tables extend through
  slot 67 (base) and 91 (traffic). Addresses plus semantic summaries do not supply complete callable prototypes.
- `cFC4AutomataManager::DoMessage` drops messages; CanCreateGroup (slot 30), exemplar selection and group-set
  operations are stubs. `engine/core/src/automata/manager.rs` create_automaton explicitly creates record stubs
  for pedestrians/vehicles, returns None for aircraft/watercraft, and lacks the full priority/flag/exemplar path.
- `cFC4AutomatonObject` has void placeholders for real methods, null AsOccupant and manager-session wrapper
  retention. The SDK automaton interface and ABI rows require real signatures. Filling a few slots would
  leave the lifecycle and occupant owner incomplete.
- Core `controller.rs` has record/local state operations; its Init/Shutdown bits do not define the full live
  template, occupant, grid, lifetime and tick contract. No live controller-owner wrapper was added.
- `FC4AutomataHost` uses game pedestrian/controller/disaster interfaces for existing riot/fire/UDI hooks.
  Those cross-owner calls are not implementations of the requested inherited owners.

## Questions

AD3/AD4 are appended to `docs/specs/questions/automata-managers.md`: complete base-manager lifecycle/ABI and
traffic-manager lifecycle/ABI, with concrete routes and derived overrides. Existing AD1/AD2 cover missing
population/recall/creation and base method bodies.

AT22/AT23 are appended to `docs/specs/questions/automata.md`: complete controller lifecycle, live-position/flag
offset reconciliation, and complete automaton lifecycle/occupant/placement/removal ABI. Existing AT21 separately
covers controller-manager final releases. Questions-index regeneration is left to the train: this lane cannot
run tools/ scripts or edit the generated spec index.

## Validation and remaining work

Baseline `qbuild all`: PASS, cargo 4213 passed/0 failed/0 ignored, clippy 0 diagnostics, CMake 206/206 edges,
ctest 67/67 (114.4 s). The verifier reported docs changed during that baseline. Baseline logs are preserved in
`build-runs/baseline-owner-audit/`.

Final `qbuild all` on committed `22c45031`: PASS, cargo 4213 passed/0 failed/0 ignored, clippy 0 diagnostics,
CMake 2/2 edges (0.6 s), ctest 67/67 (76.4 s), fc4verify PASS (77.2 s), no tree-change warning.
Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
No Rust files changed, so fmt is n/a. No new behavior was implemented, so no new boundary tests were added.
Existing passing tests do not establish complete live-owner conformance.

No game request was submitted or remains outstanding. The required actual replacement class/path trace to
2032/ref0 is unmet, and no shadow exactness is claimed. An idle-counter or record-only run would not establish
the requested owner coverage. Once the missing contracts are published, implement through concrete routes,
add independent boundary tests and obtain fullgate plus an actually exercised 2032/ref0 trace before ready.
