| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-construction-replace18 (construction identity) | `Codex` / `glue-construction-replace18` (engine lane) | Ready for measured shadow hardening only: cargo 7528 passed, clippy clean, CTest 85/85; both 2032 runs clean, construction 73/73, ref_warnings=0. Construction replacement NOT implemented. ABI unchanged. | Implement the core-backed construction owner and factory, add registration CTest, then run construction-only and current full replacement profiles before adding the key to the house list. |

Code: reject a null expected occupant before QI, preventing null/null conversion results from passing identity comparison. Tests cover both valid and broken conversions without acquiring a reference for missing expected identity.
Includes the inherited a9be4f77a interface identity tests and live shadow mapping. The shadow remains default-off via construction_shadow; its scope is position, class, type and identity, excluding records, lifecycle, schedule and rendering.
Gate: qbuild all PASS; cargo 7528/0, clippy 0 diagnostics, native build 53/53 edges, CTest 85/85. Evidence: build-runs/qbuild-{test,clippy}.log and build-runs/fc4verify-{build,ctest}.log; native queue qbuild-20261025-020825-build-ctest.
construction18-r3-shadow-2032-20261025b 73/73. Clean through 2032-01-09, 27.782 simulation seconds; 43 nonempty samples; ref_warnings=0, foreign_exceptions=0, rtc=0, watchdog_stalls=0, overruns=0, Rust panics=0.
construction18-r3-house-2032-20261025b 73/73. Clean through 2032-01-01, 113.250 simulation seconds; 2 nonempty samples; ref_warnings=0, foreign_exceptions=0, rtc=0, watchdog_stalls=0, overruns=0, Rust panics=0.
Both are turbo Konradshohe runs; DLL SHA256 prefix e2584546. The second uses the 49 explicit replacement keys documented in manual-checks run 672. Neither enables construction. Each label's .req, .log and .verdict is under build-runs/game-{queue,results}/.
Missing: cFC4Construction owner, native lifecycle/render/record adapter, construction subsystem registration and registration CTest. No construction replacement credit or permission to append construction to the house list is claimed. No missing spec contract was established in this round; this is incomplete implementation work.
Questions: none. No specs, ABI exports, coverage files, regress scripts or run records edited.
