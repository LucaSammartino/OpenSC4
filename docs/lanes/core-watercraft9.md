| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-watercraft9: watercraft owner closure | Codex / core-watercraft9 | in progress | Audit existing module against current spec, close settled gaps, run gates. |

## Handoff
- Goal: close cSC4Watercraft owner in Rust, no capi/plugin/ABI changes.
- Existing `engine/core/src/watercraft/` already contains an implementation and closure audit; inspect before extending.
- Done: reused reserved detached worktree; initial status clean.
- Next: compare README open rows and question contracts with current watercraft spec.
- No game requests or gates outstanding.
