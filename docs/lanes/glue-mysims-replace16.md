| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysims-replace16 (dispatch codec and factory boundary) | Codex / `glue-mysims-replace16` (engine lane) | Ready for measured preparatory work: cargo 7509/0, clippy clean, CTest 85/85; mysims and mysimdispatch each 73/73 exact to 2032. Replacement incomplete. | Implement all three owners and concrete dispatch callers before enabling replacement or adding mysims to the house list. |

Code: constructor ABI contract checks; production dispatch record decode/write comparison with sparse and aliased references, routes, models and queue; rejection of truncation, conflicting reference classes and oversized captures.
MS22 excludes only the constructor-unassigned auxiliary flag byte from constructor comparisons; initialized records remain strict. Tests exercise all 256 flag values and transport diagnostic output reuse.
Plugin CTests cover default, requested and mixed tutorial/mysims factory enumeration: pending mysims never registers a partial owner group or prevents tutorial registration. Both shadows remain default-off.
Final gate: `qbuild all` PASS; cargo 7509 passed / 0 failed, clippy 0 diagnostics, native build 48/48 edges, CTest 85/85. Native request `qbuild-20261025-030015-build-ctest` completed; logs in `build-runs/qbuild-{test,clippy}.log` and `build-runs/fc4verify-{build,ctest}.log`.
Game record: `build-runs/game-results/mysims16-codecfinal-2032g.verdict` and matching `.log`, DLL sha256 prefix `81fbba07`, Konradshohe turbo to 2032-01-14; clean, ref_warnings=0, panics=0, exceptions=0. Existing completed verdict retained; subsequent change is test-only.
mysims: `mysims16-codecfinal-2032g 73/73` exact; mysimdispatch: `mysims16-codecfinal-2032g 73/73` exact, every mismatch count zero.
Dispatch scope is constructor/public ABI/header/record codec: records=73, live_headers=0, live_ranges=0. No independent live-trip, transport or full replacement verification claim.
Earlier diagnostic runs `mysims16-recordcodec-2032e` and `mysims16-codecdiag-2032f` remain preserved; their constructor-byte differences were corrected under MS22 before the exact final run.
Missing: cSC4MySim, cSC4MyStreetSim and cSC4MySimDispatch replacement fronts, host integration and concrete caller boundary. Native factories remain retained even when mysims is requested; no house-profile replacement run is claimed.
No questions filed or public C ABI exports changed. No coverage maps, regress scripts or run-record documents changed.
