| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| landvalue-terrain27 (engine): terrain edits | Codex / landvalue-terrain27 / detached HEAD | wip; all gate PASS; edited shadows nonexact | Supply eligible native capture host, reproduce house baseline, rerun acceptance. |

- Paths: WT is this worktree; M = C:/Users/tlori/FreeCity4; game logs below are M/build-runs/game-results; each has its matching .verdict.
- Code e3769bece/21391f696: state 3 checks land before altitude and stores mean before fallback; progress decoder retains both scans' extrema contributions. No added terrain-change refresh (spec 6.1.6).
- Controlled tests PASS: four terrain/edit/slice/month/fallback tests, two recorded-output tests, all 16 landvalue vectors including new lv16_35. WT/build-runs/qbuild-test.log:251, :666, :1759.
- Repro/suite criterion implemented: lv-terrain27.script preserves all four terrain edits and two month ends; suite.tsv now registers it.
- Repro exact FAIL: lvterrain27-r4-repro2 clean/ref0, 3/4 exact. February6 intrinsic/total each 1 cell, max2; mean/mode/slope exact. Log :3827; final sample :4329.
- gen27 s0-default exact FAIL: lvterrain27-r4-default2 19/23 exact, corrupt/ref0; verdict foreign_exceptions=1; stopped before requested 2028-01-28. Log :1899, :10206, :17553; no clean acceptance credit.
- gen27 s0-edge exact FAIL: lvterrain27-r4-edge1 clean/ref0, 23/27 exact. March6 intrinsic129/total101, slope1; April6 intrinsic1802/total1336, slope1. Log :4021, :4167; final sample :7314.
- House 73/73 unchanged NOT VERIFIED: lvterrain27-r4-house1 stalled/ref0, 0/2 exact; watchdog_stalls=1 at 2026-03-01. Log :1902, :3568, :3786. This incomplete focused control does not replace the supplied baseline.
- Final DLL SHA256 prefix 88a41abf: repro2/default2/edge1/house1 verdicts identify it. No native capture or output-exact edited scenario claimed.
- Earlier artifact 35b1cc03: repro1 clean/ref0 2/4; default1 corrupt/ref0 16/22. Their r4 logs/verdicts are superseded for final-source acceptance; requests were submitted before the corrected all gate completed.
- Preserved input probe: lvterrain27-repro-inputs1.log:1928/:1948 shows vertex(57,61) 228.07106->270 and slope(7,7) 8.65548706->13.0744085 across January30. Corner(1,1) 272.988159->0 after February19 (:3868/:3881); these are observer/replay reads.
- Remaining limitation is specified by 6.1.5/6.1.6: pass-end samples cannot recover retained per-state/per-cell inputs; dirty notifications do not freeze inputs or restart work. Native host admission/exclusion is absent; input_capture=pass-end-sample/native_step_capture=0 remains explicit.
- Diagnostic earlier-day matches in repro/default/edge logs are not passing comparisons. No cell-level native read attribution, historical-input substitution, relaxed tolerance or extra refresh was added.
- Full gate PASS after import fix: 7856 Rust tests, clippy 0 diagnostics, native build99/99, CTest118/118. WT/build-runs/qbuild-test.log, qbuild-clippy.log, build-results/qbuild-20261026-023010-build-ctest.verdict.
- Initial all attempt failed missing CityCells import; 21391f696 fixes it and both recording tests pass. Initial native118/118 alone was not a passing all gate; older canceled attempts remain incomplete.
- fmt PASS for five owned Rust files; no ABI export changes. No new questions: LV9/LV10 now answered; actual host integration and LV2/LV7 runtime capture remain outstanding.
- All requests used M's shared queue; none outstanding. WIP packet only, no ready enqueue or fidelity credit.
- Next: provide spec 6.1.5's eligible host and actual delivery records (including both fallback scans and developer maps); recover the original house-control request/profile, then rerun repro/default/edge and verify house73/73 before enqueue.
