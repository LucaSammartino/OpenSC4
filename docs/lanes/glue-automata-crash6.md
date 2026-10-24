| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-crash6: owners capture reference crash | Codex, `glue-automata-crash6` | in progress | Diagnose capture IDs, test, gate, three 2032 runs |

## Handoff
- Goal: fix stale/foreign capture replay references in default-off automata owners shadow.
- Done: copied supplied crash evidence to build-runs/; confirmed GetRef dereferences the capture table without lifetime ownership.
- Implemented: automata captures opt into retained, QI-normalized serializable references; reject class/ID collisions before replay QI; audit replay QI balance.
- Added native DLL seam regression for adjusted effect interface, writer-released temporary, duplicate/null IDs, unknown ID and in-range wrong-class ID.
- Round 2: workspace tests 6881 passed, clippy diagnostics=0 on 204ea940d.
- Native verify qbuild-20261024-023540-build-ctest FAILED: FC4MySimsShadow.cpp:1189 C2280; WriteCapture deleted copy operations suppressed the move assignment needed by SimCapture.
- The all gate's native request qbuild-20261024-023814-build-ctest also FAILED with the same C2280; both terminal verdicts remain in build-runs/build-results/.
- Corrected in 208aa2159: explicit ownership-transferring WriteCapture move constructor/assignment; the native seam destroys both moved-from captures before replaying the retained temporary.
- Corrected all PASS: workspace tests 6881 passed, clippy diagnostics=0; native qbuild-20261024-024340-build-ctest build 208/208 and CTest 73/73. Tree-change note was the handoff-only aa13bc251 commit; engine source unchanged from 208aa2159.
- Additional corrected native verify PASS: qbuild-20261024-024436-build-ctest build 46/46, CTest 73/73 (includes automata_capture_test). Both operations collected to terminal results.
- Next: three unique 2032 game runs. Use build-runs/wait-verdict.ps1 (filesystem event wait, existing completed results accepted), NOT qwait; summarize with build-runs/summarize-automata.py.
- Required: focused native bad-ID test; final all and queued verify; three unique exact 2032 game runs.
- Keep owners shadow default-off. Coordinator should add automata_owners_shadow to the house profile after exact validation (tools are outside lane scope).
- No outstanding game request. No spec behavior or ABI export changes; this is capture tooling ownership.
- Game 384 automata-crash6-r2-2032-20261024-a: clean 2032-01-01/ref0; automata and automataowners each 72/72 monthly, 73/73 including load exact; DLL ceebe86d4684c38fc31c2d0730b4591306f2885ece17663e53902c815e5b5484.
- Game 385 automata-crash6-r2-2032-20261024-b: clean 2032-01-01/ref0; automata and automataowners each 72/72 monthly, 73/73 including load exact; same ceebe86d DLL.
- Game 386 automata-crash6-r2-2032-20261024-c: clean 2032-01-02/ref0; automata and automataowners each 72/72 monthly, 73/73 including load exact; same ceebe86d DLL.
- Merged main cleanly before queue preparation; FC4WriteCapture.cpp/.h unchanged, but Rust effects code changed. Renumbered run records 384-386 after main introduced train171 run 381.
- Post-merge all on a2c13e2af: Rust 6888 passed/0 failed, clippy diagnostics=0. Native request qbuild-20261024-030223-build-ctest is pending; collect exec session 58305 (or its build-runs/build-results/*.verdict). Do not duplicate it.
- Next: collect that native result, then full report and READY queue with full actual HEAD (repeat mandated merge immediately before queue). Engine source remains a2c13e2af; later lane-record-only commits do not change it.
- Before READY queue: merge main as explicitly instructed; preserve concurrent variant encoding changes, rerun native gate if FC4WriteCapture changes. Detached HEAD is intentional.
