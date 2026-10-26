| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| landvalue-terrain27 (engine): terrain edits | Codex / landvalue-terrain27 / reserved branch landvalue-terrain27 (detached HEAD) | blocked; commits 17fb01f02, 6fdc0541d, ea650db78; ABI/seam exports unchanged; quick cargo 648/648, build PASS; game clean/ref0 but shadow nonexact; LV10 filed | 1. Resolve LV10/LV9 host/refresh contract. 2. Complete all gate. 3. Require repro, both gen27 targets and house acceptance before ready. |

- Minimal repro added to suite.tsv: engine/tests/scenarios/scripts/lv-terrain27.script, four first-month terrain edits and two month ends.
- Repro exact criterion FAIL: lvterrain27-repro-base1 clean/ref0, 3/4 exact; March6 intrinsic 930/4096, total 637/4096 differ, max45; April pass exact. Evidence: MAIN/build-runs/game-results/lvterrain27-repro-base1.log:4235 and :4351; matching .verdict.
- Input probe measured: lvterrain27-repro-inputs1 clean/ref0, 1/4 exact. January29->30 vertex(57,61) 228.07106->270, slope(7,7) 8.65548706->13.0744085. Evidence: MAIN/build-runs/game-results/lvterrain27-repro-inputs1.log:1928 and :1948; matching .verdict.
- February6 probe mean orig/ours 270.02023/270.02063; earlier day3/10 diagnostic reproduces intrinsic/mode/mean/total exactly, without comparison credit. Evidence: same input-probe log:3725. Native capture remains 0.
- Differing repro corner altitude(1,1) was 272.988159 before February19 and 0 after; the March6 replay reads 0 twice. Evidence: same input-probe log:3868, :3881, :4559, :4564. Samples identify our reads, not the original's native state boundaries.
- Rust behavior fix NOT made: sections2/5/6.1 already require the implemented live per-state reads and dirty-only terrain notification. Supported native capture is still gated by section6.1.4/LV9. LV10 asks whether another refresh is required: docs/specs/questions/landvalue.md:49.
- Unit test added with measured altitude/slope inputs: terrain_edit_between_statistics_and_intrinsic_keeps_each_states_inputs, including next-month statistics refresh; engine/capi/src/landvalue_tests.rs.
- gen27 Konradshohe s0-default exact criterion NOT VERIFIED; s0-edge exact criterion NOT VERIFIED; no generated-scenario reruns after the spec dependency was identified.
- House 73/73 unchanged criterion NOT VERIFIED; no house rerun or fidelity credit claimed.
- Latest quick PASS: cargo fc4-capi 648 passed/0 failed, build 47/47 edges; build-runs/build-results/qbuild-20261025-235432-quick.verdict. Earlier quick also passed.
- Full gate NOT GREEN: first attempt canceled before final probe correction; final-source attempt canceled while compiling Rust crates, no test verdict; clippy and CTest not reached. Evidence: build-runs/qbuild-test.log. Neither incomplete attempt is passing evidence.
- Standalone focused cargo attempt also canceled during compilation; q.py invocation failed with Windows signal-pipe error (build-runs/q/20261025-233253-24004.log). No extra passing credit.
- fmt check reports differences in landvalue_tests.rs (first at existing line42); no broad reformat applied.
- MAIN = C:/Users/tlori/FreeCity4. All game requests used its shared queue; no request remains outstanding. WIP merge packet only; no ready enqueue.
- Next: resolve LV10/LV9 before changing behavior or selecting historical inputs; complete validation and all acceptance runs after an approved implementation path exists.
