| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propmgr-pl21fix6: deterministic prop shadow, PL21 | Codex; glue-propmgr-pl21fix6 (detached HEAD) | in progress | Apply PL21, gate, three house repeats |

## Handoff
- Goal: correct intermittent GetLotProps shadow list differences using PL21; three exact 220/220 house repeats through 2032.
- Done: PL21 A7/A8/T17 reviewed; core already implements whole-list retention and grid-only appends. Added focused PL21 tests and per-entry mismatch evidence (no exclusions).
- Next: collect house diagnostic pl21fix6-diag-20261024-a (wait session 89210), inspect differences, then implement and gate final source.
- Facts: house diagnostic request outstanding; DLL diagnostic build from 39766bb9c; no ABI changes intended.
- Required gate: qbuild all once on final source; qbuild verify uses coordinator queue.
- House profile read from main build-runs/regress-train172-shadows.ini; reproduce all settings in request.
- Diagnostic verify PASS: 233/233 edges, CTest 72/72, 338.6s. Tree-changed warning: focused tests were added during diagnostic build; final all still required.
- Focused PL21 cargo tests 2/2 PASS; absolute-path fmt PASS. Initial q wrapper failed to start Bash (Win32 error 5); initial relative fmt path failed, corrected to absolute.
