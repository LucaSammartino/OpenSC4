| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-simgrid-sg3: simulator owner grid inventory | Codex / re-simgrid-sg3 (detached worktree) | in progress | Locate SG3 closure rows; verify owner getters, geometry and lifetime; publish spec and vectors. |

## Handoff

- Goal: answer SG3 for owner-exposed simulation grids at PostCityInit and monthly reads.
- Done: reused reserved worktree; read rules and simgrid section 6.
- Facts: SG3/report are in ../glue-simgrid-lots-shadow5/docs (milestone 375f8115), absent from this checkout/main; report has no Owner closure table, only SG3 discovery blocker. SG1/SG2 remain parked.
- Findings: plumbing swaps published/working pointers; residential triple getters AddRef each result (006F72C0/006F7300/006F7340), contrary to section 6.1 blanket borrowing claim. Weather moisture is a vector object, not SimGrid; flora owns none.
- Scope: no engine reads/edits; no code output; map-view, transient and scratch grids excluded.
- Next: read engine lane Owner closure open rows, query export, extend section 6.2 and conformance section, move SG3 answers, index and doccheck.
