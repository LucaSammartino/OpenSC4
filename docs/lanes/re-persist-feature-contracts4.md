| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-feature-contracts4: PR5, FM3, FM4 native contracts | Codex / `re-persist-feature-contracts4` (RE spec lane; detached HEAD) | in progress | Trace the three native contracts; publish specs, answers and vectors. |

## Handoff
- Goal: answer only PR5 cache predicate/key mutation, FM3 streaming failures, FM4 signed reference-count overflow.
- Worktree reused at the reserved detached HEAD; no engine/core/test reads or edits permitted.
- Done: initial reservation committed as `c0753b35`; queried native bytes and the two necessary feature/INI decompilation slices.
- Done: PR5 differs: both predicates/output share a once-copied current GetKey, not the stored entry key; six independent vectors in persistence-manager.md §10.1a.
- Done: FM3 resolved: separate header-index and entry-gather phases; gathered prefix survives; installed bit/state and scan continue; two file releases after INI AddRef. Seven failure vectors plus acquisition/override controls in feature-manager.md §10.1.
- Done: FM4 matches: native increment stores/returns 80000000; signed Release destroys without decrement and returns EAX=0. Three boundary vectors in feature-manager.md §10.2.
- Questions PR5/FM3/FM4 moved to answered archives; FM1/FM2 remain outside scope.
- Next: regenerate/check question index, bounded diff review, commit the source/spec/answer milestone; run guard in same call.
- Main: `C:/Users/tlori/FreeCity4`; worktree: `C:/Users/tlori/FreeCity4/.claude/worktrees/re-persist-feature-contracts4`.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-feature-contracts4-r1-manifest.json`.
- Bash-backed q.py failed with Bash/Service/CreateInstance/E_ACCESSDENIED; use bounded native PowerShell commands.
- No game runs requested or needed for static proof; no ABI proposals.
