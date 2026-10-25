| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| nettool-cost27 (Tempelhof road cost diagnosis), kind engine | Codex / `nettool-cost27` / reserved branch `nettool-cost27`, detached HEAD | blocked on NT36; commits `f07c2a23b`, `464c18187`; ABI unchanged, no seam reservation; cargo 7844/0, clippy 0, CTest 117/117; `nettool-cost27-tempelhof-diag1` clean, ref_warnings=0; network exact 0/1, powerline exact 1/1 | 1. Answer NT36 in `docs/specs/questions/network-tools.md`. 2. Implement the admitted constraint behavior and add its unit fixture. 3. Recheck Tempelhof 1/1 and Konradshohe 73/73, including powerline, then enqueue ready. |

Added bounded single-cell mismatch diagnostics in `engine/plugin/FC4NetToolDragShadow.cpp`.
Logs live terrain and retained corners, water/depths, height range per 16 m, edge distance, occupant presence and zone.
Reproduced the exact road `(86,3)-(86,3)` cost=11/10 difference; all other public fields match.
Request used Tempelhof, seconds=150, timeout=400, nettool_plan_shadow=1, powerline_plan_shadow=1, close_dialogs=1, autotest_unpause=2, autotest_end=quit.
Result: clean, foreign_exceptions=0, ref_warnings=0, unchanged date 2000-01-01; shadow julian=0.
Terrain corners: (291.402679,293.308929,290.792847,289.532623), vertex IDs (473,602,603,474), stride=129.
Dry, empty, unzoned cell; nearest edge=3 cells; height range/16=0.236019135.
Replay retains four corners at 290.792847; absolute residual sum=4.386138916015625.
Section 8.1's factor 0.1 rounds that earthwork to 0, giving the replay's legitimate price 10.
Section 8.1.4 does not admit native retained inputs from public totals. Geometry section 9.8's plane is currently host-supplied; native constraint/result fixture remains unresolved.
Filed NT36 requesting the ordered plane/range/equality inputs and retained network result; no price adjustment or guessed native corners.
No Rust change/unit fixture: awaiting the rule needed to fix the divergence. Konradshohe was not rerun on this diagnostic-only change.
Powerline code unchanged; Tempelhof powerline remains exact 1/1, six vectors; network has one mismatch among twelve vectors.
Full gate: qbuild all PASS, cargo 7844 passed/0 failed, clippy 0 diagnostics, native build 46/46, CTest 117/117.
Gate logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; request `qbuild-20261025-200949-build-ctest`.
Game evidence: `build-runs/game-results/nettool-cost27-tempelhof-diag1.log` and `.verdict`.
Queue state wip: diagnostic/question only, task blocked rather than ready.
