| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-formats28: formats replacement, engine | Codex / replace-formats28 / detached HEAD; implementation 1d36a4038; ABI exports unchanged; no seam reservations | WIP; Rust 7850/7850, clippy 0 diagnostics, CTest 118/118; formats shadow 8957/8957 exact; action corrupt; soak ended 2030; questions none | 1. Obtain the current house replace list. 2. Reduce load overhead and investigate action shutdown. 3. Repeat house acceptance and enqueue ready only when all criteria pass. |

- Scope: default-off `formats` selects the existing FC4 exemplar/cohort resource factories and their property holders/values over fc4-formats. `exemplars` behavior is preserved; selecting both installs once.
- Packed-file segment replacement is deferred. No segment replacement credit is claimed.
- Shared game evidence root: `C:/Users/tlori/FreeCity4/build-runs/game-results/`; each label below has `.log` and `.verdict` there.
- (a) NOT MET: the exact house default list was not supplied; a text request is pending. Clean-room rules forbid reading/running tools/regress.ps1, so no house-list equivalence is claimed.
- Limited-profile soak `replace-formats28-soak-1`: `enable=simulator,budget,formats`; requested 2032-01-01, verdict **ended** at 2030-04-01, ref_warnings=0; log:9893,10149. The game shut down the city before the deadline.
- (b) PASS: `replace-formats28-shadow-2` clean, ref_warnings=0; 8957/8957 file exemplars exact, 148756 properties; log:1237.
- Store shadow: own/effective exemplars each 8959/8959, own/effective cohorts each 388/388, zero missing/failure differences; `replace-formats28-shadow-2.log`:1247-1251.
- DBPF bounds remain exact: 447304/447304, 16396/16396, 23852/23852; `replace-formats28-shadow-2.log`:6321,6493,6580. Segments remain native.
- (c) NOT MET: suite row `gen27-konradshohe-s0-default`, limited replacement profile above; `replace-formats28-gen-2` ran all 944 actions and reached 2028-01-28, then **corrupt** during shutdown: foreign_exceptions=1, ref_warnings=0; log:18586,18810. Cause is unassigned.
- `replace-formats28-gen-1` was rejected before launch because script was outside [ini]; corrected under a new label. It is not game evidence.
- Load-time FAIL: identical RelWithDebInfo DLL, slot 1, `replace-formats28-load-off-2` versus `replace-formats28-load-on-2`: LoadCity entry-to-return 0.940 s versus 1.464 s (+55.7%); PostCityInit interval 0.679 s versus 0.970 s. Both clean, ref_warnings=0. Timing markers are `autoload: calling LoadCity`, `autotest: city loaded`, and `autoload: LoadCity`.
- Initial Debug pair `replace-formats28-load-off-1` / `replace-formats28-load-on-1`: 0.959 s / 1.893 s (+97.4%); both clean, ref_warnings=0. These are distinct from the optimized measurements.
- Native factory fixture is 7/8 raw matches: add_slots_b differs only in EAX 2 versus Boolean 1 for slots 14/16. data-formats section 8.2 EX52 f explicitly explains surviving reference counts versus portable Boolean returns; property results match. `replace-formats28-load-on-1.log`:260,264.
- Final `qbuild all`: 7850 Rust tests passed, clippy diagnostics=0, 118/118 CTests passed (93.2 s); logs in this worktree: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; native verdict `build-runs/build-results/qbuild-20261026-034006-build-ctest.verdict`.
- A per-file /O2 candidate failed with MSVC D8016 and was fully reverted; CMake source is unchanged. The final local build cache uses RelWithDebInfo. Final game DLL SHA256 prefix: 6161762f.
- Requests used the shared main queue, parallel=1, at most three outstanding. No game request remains outstanding; no spec behavior was guessed and no spec questions were filed.
- Formats is **not ready for the replace list**. Next: use the actual house profile for off/on timing and the 2032/action checks; bisect the action shutdown against the same profile without formats; profile the factory bridge if overhead remains above 20%.
