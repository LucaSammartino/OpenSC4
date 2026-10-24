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
- Corrected: explicit ownership-transferring WriteCapture move constructor/assignment; the native seam destroys both moved-from captures before replaying the retained temporary.
- Next: re-run all plus queued native verify on corrected source, then three unique 2032 game runs.
- Required: focused native bad-ID test; final all and queued verify; three unique exact 2032 game runs.
- Keep owners shadow default-off. Coordinator should add automata_owners_shadow to the house profile after exact validation (tools are outside lane scope).
- No outstanding game request. No spec behavior or ABI export changes; this is capture tooling ownership.
- Before READY queue: merge main as explicitly instructed; preserve concurrent variant encoding changes, rerun native gate if FC4WriteCapture changes. Detached HEAD is intentional.
