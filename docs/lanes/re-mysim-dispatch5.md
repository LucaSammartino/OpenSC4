| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysim-dispatch5 (dispatch trip contract, MS17) | Codex / `re-mysim-dispatch5` | in progress | Trace dispatch transitions, record bytes and RNG; extend spec and disposition MS17 |

## Handoff
- Goal: specify MS17 dispatch owner without engine edits.
- Done: reused reserved detached worktree; read lane rules and open MS17; located Windows dispatch export/decomp.
- Next: query dispatch functions and read targeted existing spec sections.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export query `tools/re_query.py`.
- No game requests, ABI proposals or engine checks planned (documentation-only RE lane).
