| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-terrain-replace14 (terrain replacement interface audit) | `Codex` / `glue-terrain-replace14` (engine lane) | ready, partial: 2e7504714 and 69cd3b6be interface corrections; cargo 7484/0, clippy clean, CTest 81/81; run 14644 shadow clean 98/98 through 2032-01-03; run 14645 terrain-only replacement stalled 2026-05-03; TR44 filed; ABI unchanged; terrain replacement remains default-off and uncredited | (1) Resolve TR42/TR43/TR44 and the specified direct-array/view lifecycle gaps; (2) coordinator supplies house profile and measures 48 keys plus terrain through 2032; (3) credit replacement and update regress profile only after success. |

## Report

Partial closure: the interface corrections are gated and the original-terrain shadow is exact through 2032. The replacement target is not achieved. Terrain remains default-off; no replacement credit or house-profile change is requested.

### Changes

- `2e7504714`: terrain slot 66 returns the view interface at object base+0x0C, preserving null, per terrain.md section 15.5.6.
- `69cd3b6be`: request factory IID `0x6771477D`, store the returned interface minus 0x0C as the object base, per section 15.5.1; retain acquisition/release accounting. The native terrain test now invokes the replacement seam.
- This resume round completes final-source gates and real-game measurements; no engine source changed after those two implementation commits. C ABI exports are unchanged.

### Validation

Final-source `qbuild all` passed: workspace and doc tests summed **7484 passed, 0 failed**, clippy `-D warnings` exit 0 / diagnostics 0, queued native build and full **CTest 81/81**. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

The separately required `qbuild verify` also passed: build 267/267 edges, CTest 81/81, terminal exit 0. I started it before `all` reached its native stage, causing redundant native requests `qbuild-20261024-225556-build-ctest` and `qbuild-20261024-230142-build-ctest`; both were collected to completion. The latter build was incremental, 46/46 edges. Tree-change notes reflect documentation commits only. No gate remains outstanding. Prior-round interrupted evidence remains separately in `qbuild-initial-test.log` / `qbuild-initial-clippy.log`.

`gtr14-r2-shadow-2032-a 98/98` — [run 14644](../runs/gtr14-r2-shadow-2032-a.md), clean through **2032-01-03**, foreign_exceptions=0, watchdog_stalls=0, ref_warnings=0. The harness counts 98 exact lines including shutdown; the plugin's internal counter is 97/97 comparison lines. All 72 month lines through 2032-01-01 are present; 16932 other checks have zero mismatches; unfollowed, followed-late and pending vertices are all zero. Evidence: `build-runs/game-results/gtr14-r2-shadow-2032-a.{log,verdict}`. No normal-speed rerun was needed because no shadow differed. The filesystem-event waiter missed the notification and terminated; the already completed clean verdict was retained without resending.

`gtr14-r2-replace-2032-a` — [run 14645](../runs/gtr14-r2-replace-2032-a.md), **stalled at 2026-05-03**, watchdog_stalls=1, foreign_exceptions=0, ref_warnings=0. This used `enable=terrain` only. It logged view creation, no ConnectionsReady within 30 game days, and hidden=1 at the final heartbeat. It neither reproduced the house profile nor establishes that the reported 0x41800000 crash is fixed. Evidence: `build-runs/game-results/gtr14-r2-replace-2032-a.{log,verdict}`. Waiter completed; no game request remains outstanding.

### Open contracts and next steps

1. **TR44**, filed in [questions/terrain.md](../specs/questions/terrain.md), requests the reported crash receiver/slot plus slot 54 render-object identity and lifecycle. No new duplicate question was filed this round. Existing **TR42/TR43** cover reconciliation preferences and the missing comparison that controls TerrainReconciliation / ConnectionsReady. The new stall is evidence of failure, not proof of its cause.
2. Implement the specified live direct arrays at terrain+0x6C and water+0x18 (section 15.5.11). Current plugin code does not populate them. Complete the view lifecycle: section 15.5.1 requires Init(terrain), while section 15.5.3 exposes no Init vtable slot. Do not invent a native virtual signature or call an executable address as an interface.
3. A clean-room spec/RE lane must supply the missing receiver contract; the coordinator must supply the permitted house-profile inputs and run the 48 keys plus terrain through 2032. Only after a clean replacement run should the coordinator credit terrain owners and append terrain to the regress replace list.

No forbidden RE paths, tools/ files/scripts, game executable, main checkout source, coverage-map or coverage-shadows file was accessed or edited. The requested tools/ edits and coverage-script execution conflict with the lane's clean-room restrictions and remain coordinator work. Shadows remain default-off except through their ini key.

Worktree is intentionally detached; the supervisor publishes commits. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-terrain-replace14-r2-manifest.json`.
