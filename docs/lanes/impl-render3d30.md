| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| impl-render3d30: cSC43DRender core (engine) | Codex / WT impl-render3d30 / reserved branch impl-render3d30 (detached HEAD) | continue; partial owner; round code commits 614031682, fc33e723f, 35357e27a; ABI/seams none; cargo 8017/0, clippy 0 diagnostics, CTest 130/130; game/shadow n/a; no new questions, RND1/RND4 open | 1. Complete model queue/passes and initialized teardown. 2. Finish remaining closure rows. 3. Gate and enqueue kind engine after owner closure. |

Acceptance: core module and 16-row closure table exist at `engine/core/src/render3d/README.md`; whole-owner implemented credit remains pending.
This round: §10 parameter-block shakes and §6.3 stop, §11 view objects/visibility/picking, §14 container geometry and grid rebuild policy.
Shake cases: S1 parameter handles, S2–S6, fade/immediate stop and process pattern tables; 8 new tests (`build-runs/qbuild-test.log`).
Picking cases: P1–P6, worked clipped polyline, outside-start projection, segment rejection and guards; 10 new tests (`build-runs/qbuild-test.log`).
Container cases: four rotations, ordered clamps/sliver, separate frustum caches, ray edges, movement/rebuild and L3 margins; 6 new tests (`build-runs/qbuild-test.log`).
Tests: 56 renderer tests pass; core unit 5320/0 (summary line 7985), workspace 8017/0 (`build-runs/qbuild-test.log`).
Full gate: one qbuild all invocation PASS; Clippy 0 diagnostics (`build-runs/qbuild-clippy.log`).
Native: build 207/207 edges PASS (`build-runs/fc4verify-build.log`); CTest 130/130 PASS (`build-runs/fc4verify-ctest.log`).
Formatting: 17 owned Rust files, 0 diff lines; exact terminal summaries and source qualifications in `build-runs/render3d-r2-verification.txt`.
Foreign references, projection, terrain/model hit tests, timers and random draws use traits; process shake tables are supplied separately like backing buffers. ViewChange reuses view3d.
Portable f64 approximates the specified x87 vector/pattern intermediates; marked SPEC-GAP and called out in closure rows.
RND1 serialization and RND4 convenience shake overload remain explicitly Unsupported and open in `docs/specs/questions/render3d.md`; no new questions.
No C ABI, plugin or ABI-version changes; game/shadow n/a for this unwired core round. Ready enqueue not attempted; WIP queue only.
Remaining: §4 UpdateViewportRect; §5 model queue/passes; §6.2 processing/cleanup; §9 lifecycle; §§13,15 drawing/driver/helpers; §7 cross-reference, §8 remaining cases, §16 inventory audit.
Important: no Init API yet; Shutdown still only implements the never-initialized guard. Terrain/camera acquisition and container-margin initialization await §9; complete initialized teardown before glue.
Next: finish §5, then §9; finish §13/§6.2 and §15; audit the remaining rows, preserve RND1/RND4 boundaries, run the gate after code changes, and enqueue kind engine only after whole-owner closure.
