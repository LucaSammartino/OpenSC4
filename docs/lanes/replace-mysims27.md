| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| replace-mysims27 (engine, MySim replacement safety) | Codex / `replace-mysims27`, detached HEAD; supervisor publishes branch | blocked on shared game queue; fixes `9cecd348a`, `712547e54`; C ABI +5 exports; seam none; Rust 7849/0, clippy 0, CTest 118/118; no game acceptance | 1. Resolve pending save request. 2. Supply exact house list and measure all three game criteria. 3. Enqueue after acceptance. |

- Status: mysims is NOT yet ready for the replace list; no enqueue. WIP merge-queue entry only.
- Final gate on source `712547e54`: qbuild all PASS; Rust 7849 passed/0 failed, clippy 0 diagnostics, native 46/46 edges, CTest 118/118. Logs: `build-runs/replace-mysims27-gates/all-3-passed-712547e54/` (qbuild-test, qbuild-clippy, fc4verify-build, fc4verify-ctest).
- Earlier all failed linking the loader test, then failed 2/118 CTests (return annotations and absent test allocator). Both fixed; second failed attempt retained in `build-runs/replace-mysims27-gates/all-2-failed-before-allocator-fixture/`.
- Criterion (1): compiled-order 69-slot cISC4MySim front for MySim/MyStreetSim, shared advisor identity/refcount. `plugin_mysims_replace_factory_test` PASS in final CTest log.
- Criterion (2): advisor-system Write/Read forward to codecs; dispatch references saved/owned and restored dispatch bound at startup. `advisors_tests::mysim_record_round_trip` PASS, including loaded personal name (`qbuild-test.log:23`); real save/reload unmeasured.
- Criterion (3): constructed advisor core published before Init; factory handle transfers to first QI. Five-object `advisors_tests::mysim_factory_before_system_init` PASS (`qbuild-test.log:8`); both native MySim class factories PASS before system Init in the CTest above.
- Criterion (4): Lua tables, fields/context and tuning_constants reader connected per mysims sections 4.4/4.11; live tuning unmeasured.
- MySim state getters use RwLock. UI controls/audio/camera/balloons remain delegated by section 0; move_in_control remains unwired. Scripted move-in supplies the persistence exercise.
- 2032 house replace: NOT RUN; exact default regress replacement list requested asynchronously, no answer received. ref_warnings unmeasured.
- Save request `replace-mysims27-r2-save-1`: REJECTED before game; top-level export_city is invalid (`C:/Users/tlori/FreeCity4/build-runs/game-results/replace-mysims27-r2-save-1.verdict`).
- Save request `replace-mysims27-r2-save-2`: OUTSTANDING in main's shared game-queue, submitted 2026-10-26 00:45:48; still unconsumed at 01:12:30, beyond timeout 330s + 15min. Two qwait calls timed out; no verdict/log or measured persistence.
- Corrected request keeps export_city under [ini], DLL/script paths relative to main. Target export: `C:/Users/tlori/FreeCity4/build-runs/fixtures/replace-mysims27-r2-persist.sc4`.
- Reload: NOT RUN. `build-runs/replace-mysims27-save.script` queries initial state, moves in FC4 Persist, queries, saves; `build-runs/replace-mysims27-reload.script` queries after reload. Action logs name/slot/state/age/wealth/generation/home/bio/portrait.
- Shadow: NOT RUN; required mysims 73/73 exact still pending. No readiness credit from predecessor runs.
- Questions filed: none. No running exec sessions remain. Shared refs/config were not changed; detached HEAD commits await supervisor publication.
- Next: resolve the pending save-2 request before sending another; load its exported slot copy via fixture_city and measure persistence; run native mysims_shadow to 2032 and exact supplied house list + mysims to 2032, all clean/ref_warnings=0. Enqueue only after all three measured proofs; rerun qbuild all only after a further fix.
