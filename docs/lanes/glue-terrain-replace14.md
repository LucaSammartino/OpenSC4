| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain replacement contracts) | `Codex` / `glue-terrain-replace14` (engine lane, detached HEAD) | ready, measured partial: zoom payload and once-only initialization corrections; cargo 7484/0, clippy clean, CTest 81/81; shadow 104/104 through 2032-01-12; terrain-only replacement stalled 2026-05-03; ABI unchanged | Land corrections; resolve existing TR42/TR43/TR44 and live-array/view lifecycle gaps; coordinator measures the house profile before replacement credit. |

Code: read the zoom from `*GetVoid2()` per terrain section 2.3/TR23, preserve its bits without clamping, ignore absent payload (`SPEC-GAP`), and synchronize the core level only on change. Pin and enforce the once-only guard at terrain+0x14 before PostCityInit collaborator calls.
Tests: extend the existing native terrain seam with conflicting data1, unaligned payload, repeated zoom, unclamped zoom, zero, absent payload, and repeated PostCityInit. Require at least 27 checks in the native test.

Final-source `qbuild all`: PASS; cargo 7484 passed/0 failed, clippy exit 0/diagnostics 0, queued native build 53/53 edges, full CTest 81/81. Native request `qbuild-20261024-234239-build-ctest` completed; no duplicate verification was launched.
Gate evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Prior failed/interrupted evidence remains in `qbuild-initial-*.log`.

`gtr14-r3-shadow-2032-a 104/104`: clean through 2032-01-12, all 72 monthly lines through 2032-01-01; mismatches=0, other checks 16945/0 mismatched, unfollowed=0, followed-late=0, pending=0, ref_warnings=0. The plugin counter excludes its shutdown summary and reports 103/103; the harness includes it and reports 104/104.
`gtr14-r3-replace-2032-a`: terrain-only replacement stalled at 2026-05-03, watchdog_stalls=1, foreign_exceptions=0, ref_warnings=0; view creation logged, ConnectionsReady absent. This does not establish that the reported virtual-call crash is fixed.
Both runs used the gated DLL (SHA256 prefix e314c24a), Konradshohe, turbo, end_date=2032-01-01. Evidence is `build-runs/game-results/<label>.{log,verdict}`; request records are `build-runs/game-queue/done/<label>.req`. No request remains outstanding. No normal-speed shadow rerun was needed because the shadow was exact.

Open: existing TR42/TR43 reconciliation contracts and TR44 render-object receiver/lifecycle. The specified native view Init/Shutdown have no exposed virtual slot; do not invent signatures or call executable addresses. Terrain+0x6C and water+0x18 live arrays remain unwired, as does terrain record loading. These gaps prevent full replacement closure.
The 48-key house profile was unavailable in permitted lane inputs; neither that replacement run nor the combined shadows profile was measured. Coordinator must supply/run them before credit. Terrain stays default-off and uncredited; coverage maps and regress profiles were not edited. No new question or run-record files were added, per this round's revised rules.
No forbidden RE paths, tools scripts, game executable, or main-checkout source were accessed. C ABI exports unchanged. Queue readiness applies to the measured code corrections only, not to the terrain replacement target.
