| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain startup stall) | `Codex` / `glue-terrain-replace14` (engine, detached HEAD) | ready: terrain replacement clean through 2032-01-13; shadow 92/92 exact through 2032-01-06; cargo 7484/0, clippy clean, CTest 81/81; ABI unchanged | Coordinator measures the 48-key house profile before full replacement credit. |

Code: register/remove the terrain animation target; on its second tick after view creation post deferred reconciliation; honor window-manager modal and app popup gates, then post TerrainReconciliation through the game message server.
Diagnosis: r3 watchdog addresses resolve in our DLL to PacingFlush, HostSettle, simulate_agents, on_tick and OnTick. Its message-server pump enters a native modal window loop with thousands of nested simulator ticks on one date. Terrain never registered for animation ticks or initiated reconciliation, so ConnectionsReady never arrived and popup suppression never engaged.
Final replacement logs reconciliation completion and ConnectionsReady on 2026-01-29 and reaches the 2032-01-01 month boundary without watchdog stalls.
Tests: native seam checks view-less ticks, first/second view ticks, saturation and repeated PostCityInit; minimum check count 38. Existing zoom/interface checks remain.
SPEC-GAP TR42: default preference 1 and water's 0xC0 tick priority. SPEC-GAP TR43: no edge comparison/change until its contract is supplied. Existing TR44/view and live-array/load gaps remain; no native signatures or executable addresses guessed.

Final-source qbuild all PASS: cargo 7484 passed/0 failed; clippy diagnostics=0; queued native build 47/47; CTest 81/81. Native request qbuild-20261025-000152-build-ctest; logs build-runs/qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log.
Earlier diagnostics: qbuild-20261024-235155-build-ctest failed an off-by-one test-count assertion; qbuild-20261024-235541-build-ctest failed the return-channel audit. Both corrected before the one final full gate.
gtr14-r4-replace-2032-a stalled 2026-02-03 before tick registration; gtr14-r4-replace-2032-b clean through 2032-01-09 after registration.
gtr14-r4-replace-2032-final: clean through 2032-01-13; foreign_exceptions=0, watchdog_stalls=0, ref_warnings=0.
gtr14-r4-shadow-2032-final 92/92: clean through 2032-01-06; all 72 monthly lines through 2032-01-01 exact, mismatches=0; other checks 16909/0 mismatched; unfollowed=0, followed-late=0, pending=0, ref_warnings=0. Plugin shutdown counter is 91/91; harness includes the shutdown summary.
Final runs: Konradshohe, turbo, end_date=2032-01-01, DLL SHA256 prefix 53ae6fed. Evidence and requests: build-runs/game-results/<label>.{log,verdict} and build-runs/game-queue/done/<label>.req. No request outstanding; exact shadows require no normal-speed retry.
No new questions, specs, run records, coverage maps, tools files or regression profiles edited. Shadows remain opt-in. No forbidden RE sources accessed. Ready scope is the measured terrain-only startup fix; combined house profile and replacement credit remain coordinator work.
