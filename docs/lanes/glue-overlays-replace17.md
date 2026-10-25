| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-replace17 (portable overlay owner factory) | `Codex` / `glue-overlays-replace17` (engine lane, detached HEAD) | Ready measured partial: cargo 7527 passed, clippy 0 diagnostics, ctest 85/85; clean turbo through 2032, ref_warnings=0; factory 27/27, shadow 73/73, records/replay 292/292; ABI unchanged; native replacement incomplete | Implement specified native routing/resource adapters and validate the full house list before adding overlays to that list. |

Code: default-off `overlays` registers FC4-owned CLSID 0xFC401717/private IID 0xFC401718; native IID 0x895D6544 is rejected. The portable factory owns the production terrain_overlays manager through a Mutex and explicit dimension-bearing inputs.
Controls: Init/Shutdown, binding decals, removal, alpha, visibility and sticky self-lighting. Tests cover flag retention across repeated Init, reset after Shutdown, sentinel no-ops, invalid/removed handles and simultaneous readers/writers.
Final-round code extends the live GZCOM lifecycle probe and real-DLL CTest with those flag and handle checks. No public C ABI exports changed.
Gate: final qbuild all PASS; cargo 7527/0/0, clippy 0 diagnostics; native queue qbuild-20261025-022938-build-ctest, build 55/55 edges, CTest 85/85. Logs: build-runs/qbuild-{test,clippy}.log and build-runs/fc4verify-{build,ctest}.log.
Diagnostic: first gate failed two incorrect sentinel assertions (84/85); corrected to successful no-ops consistent with core. Original failure retained in build-runs/fc4verify-ctest-sentinel-failure.log and build-results/qbuild-20261025-020649-build-ctest.verdict.
`gor17-r5-flags-2032-20261025b 73/73` — clean turbo, end_date 2032-01-01, actual verdict date 2032-01-07, ref_warnings=0; DLL SHA256 prefix 359f37ae; enable=overlays, overlays_shadow=1, overlays_fixture=1.
Live GZCOM factory probe 27/27, mismatches=0; overlay shadow 73/73, records 292/292, replay 292/292, every mismatch count zero; no incomplete samples, panics or structured exceptions.
Evidence: build-runs/game-results/gor17-r5-flags-2032-20261025b.{verdict,log}; completed request under build-runs/game-queue/done/.
Open: native_interception=0. Native Init/history/drawing and resource adapters remain unimplemented; the terrain view still returns native managers. This measured portable factory is not yet a cSTEOverlayManager replacement.
Full-house replacement run remains unperformed; current house list unavailable within permitted sources. Do not append overlays to the house replace list on this evidence.
Questions: none filed; existing SPEC-GAP comments identify unsupported routing. No specs, tools, profiles, coverage files or run records edited.
