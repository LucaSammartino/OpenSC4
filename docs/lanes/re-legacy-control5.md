| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacy-control5 (legacy control behaviour spec) | `Codex` / `re-legacy-control5` (re) | in progress | (1) Inventory owner; (2) specify contracts and vectors; (3) doccheck and report. |

## Handoff
- Goal: specify cGZLegacyControl in docs/specs/legacy-control.md without engine edits.
- Worktree reused at detached HEAD as instructed; initial status clean.
- Done: read rules, contribution contract and lane format; export search locates owner at 0094c30e onward.
- Next: complete export inventory/vtable query, read owner decomp by bounded ranges, record evidence.
- Main evidence: C:/Users/tlori/FreeCity4/decomp/cGZLegacyControl.c and tools/re_query.py.
- No ABI edits, game requests, or engine reads authorized/needed.
