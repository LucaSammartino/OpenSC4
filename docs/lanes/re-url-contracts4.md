| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-url-contracts4 (RE: cRZURL contract gaps, web-network §2, Q4-Q6) | `re-url-contracts4` / `.claude/worktrees/re-url-contracts4` | in progress | answer Q4-Q6, audit remaining cRZURL methods |

## Handoff
- Goal: answer web-network Q4-Q6 (ChangeURL chain, port run/atol, unknown scheme) from asm; audit cRZURL methods not
  yet in §2 (ctor/dtor/copy/assign, setters, other getters) so core-url4 can become the whole owner.
- Done: lane file.
- Next: asm of ChangeURL 00823a90 empty-path branch; server step 008227e0 port run; ParseScheme 00822180 steps 4/5.
