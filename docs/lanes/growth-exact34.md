| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| growth-exact34 - engine: lotdev exactness investigation | Codex / growth-exact34 / detached HEAD (reserved branch growth-exact34); questions d6acbc7ed; ABI exports unchanged, no seam reservations | wip; no newly exact owner; cargo 8410 passed, clippy 0 diagnostics, CTest 146/146; lotdev exact 0/2, none exact 2/2; both lotdev runs clean/ref_warnings=0; no shadow result; G34-LD1/G34-LD2 filed | 1. Resolve G34-LD1/G34-LD2. 2. Obtain development shadow results through a working game queue. 3. Fix one owner and rerun qcheck plus matching none controls. |

- No implementation behavior changed. Added two evidence-based questions to docs/specs/questions/lot-developer.md; no owner earned exactness credit.
- Full gate: qbuild all PASS; cargo 8410/0/0, clippy -D warnings 0 diagnostics, CMake 3/3 edges, CTest 146/146. No owned Rust files; fmt not applicable.
- lotdev: NOT EXACT, qcheck-1 exit 1; gen27-spandau-s2-default differs in 103 records (89 only relabelled), first shifted ID 4 class A98F9D28, first content occmgr 098F964D/299B2D1B/0; DIGESTs and sampled RNG agree.
- Spandau: golden-27c1ede8a3f940bb8586e7e7cc6007c5, compare-20261028-031440-e64205ad/result; clean/ref_warnings=0. Reference has four construction occupants, replacement three, and one fewer object each of 2977AA47, 4A1628C5, 6A1628BA.
- G34-LD1 asks for the missing-building construction lifecycle/persistence contract; the existing pre-Init safeguard was retained. A logged refused scene is evidence to investigate, not a proven cause for every difference.
- lotdev: NOT EXACT on ui29-volcano; only C9BD5D4A/299B2D1B/0 differs, expected 238079 bytes, actual 238699. Several lots add a zero-valued C999C45E property; the first lot also differs at payload byte 122 (0x26/0x20).
- Volcano: golden-7bf94949f4f34a6ebc39c89e8e5cb747, compare-20261028-031440-6adb2921/result; clean/ref_warnings=0. G34-LD2 asks about nonvirtual zot-helper property creation and remaining daily/counter behavior.
- --replace none: qcheck-2 exit 0, gen27-spandau-s2-default and ui29-volcano exact 2/2; these lotdev differences are absent in the controls.
- Development shadow request g34-dev-shadow-1 remained unclaimed for qwait's nine-minute timeout. Its pending request was cancelled; no game verdict or comparison was obtained, and no shadow credit is claimed.
- The following owners were not changed or rerun; their first differences are the coordinator's baseline, not new measurements:
- residential: NOT EXACT; gen28-konradshohe-s2-grow first delivery 2026-02-07 #21, ui29-bevopolis 2257-04-20 #3381; first content occmgr 098F964D/299B2D1B/0.
- commercial: NOT EXACT; timbuktu-rush-hour one differing record 896E714A/299B2D1B/0; previous round's G33-CM1 remains unresolved.
- development: NOT EXACT; gen27-spandau-s2-default first delivery 2001-09-19 #21, first content 096E75F5/299B2D1B/0; reach29-growth exact only in the supplied baseline.
- demand: NOT EXACT; gen28-konradshohe-s2-default first delivery 2026-05-02 #21, first content 096E75F5/299B2D1B/0.
- zonedev: NOT EXACT; gen28-tegel-s2-default first delivery 2030-04-30 #21, first content occmgr 098F964D/299B2D1B/0.
- lotcfg: NOT EXACT; gen28-tegel-s1-default first delivery 2029-08-30 #21, first content occmgr 098F964D/299B2D1B/0.
- propmgr: NOT EXACT; maxisland-north-forks first shifted ID 21 class 6A26D049, first content occmgr 098F964D/299B2D1B/0.
- exemplars: NOT EXACT; ui29-weburg first delivery 2197-04-12 #21, first content occmgr 098F964D/299B2D1B/0.
- Run directories above are under C:/Users/tlori/FreeCity4/build-runs/golden-runs; qcheck-1.log, qcheck-2.log, qbuild-test.log, qbuild-clippy.log and fc4verify-ctest.log are in this worktree's build-runs.
- No unlanded branches merged, no game files committed, no C ABI exports changed. Queue state is wip because no owner became exact.
