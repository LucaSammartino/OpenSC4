| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysims-replace16 (dispatch ABI/codec tests) | Codex / `glue-mysims-replace16` (engine lane) | Ready to land retained tests; replace flip incomplete. Cargo 7647/0, clippy clean, CTest 94/94; native shadows each 73/73 exact. | Implement all three replacement fronts and concrete dispatch callers before adding mysims to the house replace list. |

Retained code: dispatch constructor ABI checks and production record decode/write comparison; sparse/aliased references, routes, models, queue, truncation, conflicting classes and oversized captures are tested.
MS22 excludes only the constructor-unassigned flag byte; initialized records stay strict. Tests cover all 256 flag values and reuse of transport diagnostic output.
Merged main, retaining both sides of the CMake, subsystem registration and plugin test conflicts. No new owners or questions started.
Factory CTests cover default, requested and mixed tutorial/mysims enumeration; the unavailable group never registers partial replacements. Shadows stay default-off.
Final stable-source gate: `qbuild all` PASS; cargo 7647 passed / 0 failed, clippy clean, native build 195/195 edges, CTest 94/94.
Native request: `qbuild-20261025-052529-build-ctest`; logs: `build-runs/qbuild-{test,clippy}.log`, `build-runs/fc4verify-{build,ctest}.log`.
Fresh game record: `build-runs/game-results/mysims16-r5-native-2032h-0534.verdict` and matching `.log`; DLL sha256 prefix `d39221d7`; Konradshohe turbo to 2032-01-05, clean, ref_warnings=0, foreign_exceptions=0; no panic found.
mysims: `mysims16-r5-native-2032h-0534 73/73` exact, every mismatch count zero.
mysimdispatch: `mysims16-r5-native-2032h-0534 73/73` exact, every mismatch count zero.
Dispatch scope: constructor/public ABI/auxiliary/record codec; live_headers=0, live_ranges=0. Live-trip inputs and transport remain uncompared.
Missing: cSC4MySim, cSC4MyStreetSim and cSC4MySimDispatch replacement fronts, host integration and concrete dispatch callers. The replace flip was not implemented.
The fresh request used `enable = mysims`; its log confirms replacement unavailable and native factories retained. This is not a replacement-only or full-house replacement verification. Do not append mysims to the house replace list.
No questions, public C ABI export changes, coverage-map edits, regress-script edits or run-record documents.
