| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| automata-actions30: NM20 terrain-view lifetime (engine) | Codex / automata-actions30 / detached HEAD; reserved branch automata-actions30; code d9dd91d99, fdc7904ce; ABI false | wip; Rust 7961/clippy 0/CTest 130/130; s1 and s2 FAIL 0/216; automata shadow exact 73/73; TV4 filed | 1. Answer TV4. 2. Complete native renderer prerequisites. 3. Rerun s1/s2 216/216 and shadow on final DLL. 4. Enqueue kind engine only after acceptance. |

Merge criterion: automata-actions29, then main, both already contained by starting HEAD ef84805f1; no branch/ref changes.
NM20 implementation: initialize the native view with the primary terrain before readiness; retain stable 0x24-stride terrain/water arrays; unregister/shut down the view before freeing arrays.
Water creation-order fix: terrain-owned and separately city-created water interfaces share the same +0x18 array, including instances created before Init.
Fixture PASS: primary/drawing receivers, slot-15 arguments, normal/forced admission, retained reference, failed/duplicate Init, shutdown order and both water creation orders; `build-runs/fc4verify-ctest.log`, plugin_ini_terrain_shadow_test.
Final gate PASS: `build-runs/qbuild-test.log` 7961 passed/0 failed/0 ignored; `build-runs/qbuild-clippy.log` exit 0/0 diagnostics.
Final native gate PASS: `build-runs/fc4verify-build.log` 29/29 edges; `build-runs/fc4verify-ctest.log` 130/130. Second all followed the water fix; no Rust files changed.
Game evidence below is in main's `build-runs/game-results/`; exact requests are in main's `build-runs/game-queue/done/`.
s1 criterion FAIL: `aa30-nm20-house-s1-b.log` / `.verdict`, 0/216, corrupt on 2026-01-27, native +0x3540A5 reads 0x8, ref_warnings=0; unchanged house replace list.
s2 criterion FAIL: `aa30-nm20-house-s2-b.log` / `.verdict`, 0/216, same first-frame fault and date, ref_warnings=0; unchanged house replace list.
Both b logs record NM20 view initialized, drawing=base+0x0C, retained primary/vertex pointers and ready=1 before the fault; runtime view Shutdown was not reached.
Earlier source FAIL: `aa30-nm20-house-s2-a.log` / `.verdict`, 0 actions, load-time +0x356B5A null water-array read, ref0; corrected by fdc7904ce. Do not use it as corrected-source acceptance.
Shadow criterion PASS: `aa30-nm20-shadow-b.log` / `.verdict`, clean through 2032-01-04, ref0; shutdown 73/73 exact, 438 components, 734254 bytes, 0 mismatches.
Artifact attribution: a used SHA256 prefix b5e305c2; all b games used 0d9c881d. Final same-source gate relink is 08523bbe and has no game acceptance run.
Remaining renderer limits: mirrored normals use core face normals, not smoothed render normals; terrain texture-layer population remains incomplete. No zero-terrain or skip-draw fallback was added.
TV4 filed in `docs/specs/questions/terrain-view.md`: identify the first-frame null read and supplying collaborator/initialization contract; stop this item rather than guess.
NM20-A remains an uncollected runtime observation: original control, native placement result and first pointer/cache write at the old 84/216 boundary. These runs never reach it.
Not enqueued: the 216/216 house acceptance criteria remain unmet; local merge entry is wip despite the passing gate.
