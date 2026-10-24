| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-lh6: Lua interception and replay contracts LH6-LH7 | Codex; re-lua-lh6 (detached worktree) | in progress | Resolve owner closure gaps using original export; document vectors and run doccheck. |

## Handoff

- Goal: answer LH6 and LH7 in lua-host, with conformance vectors; no engine changes.
- Done: original-export audit; LH6/LH7 rules in §§1.4.2, 3.5, 8.6, 9.1, 13; 13 prose vectors in §16.2; questions archived.
- Facts: no public complete observer; Lua table execution bypasses interface calls; CRT rand/srand share OS-thread state, not per-VM state.
- Gate: questions index regenerated; questions check, doccheck check and git diff --check PASS.
- Next: final evidence review, then finish report and enqueue full HEAD (kind re).
- Owner report available in worktree/main/owner worktree has no Owner closure table or open: rows; audited its LH6/LH7 Questions section and original questions instead.
- Main evidence root: C:/Users/tlori/FreeCity4; use re_query.py before decomp searches.
- No game requests or ABI proposals.
