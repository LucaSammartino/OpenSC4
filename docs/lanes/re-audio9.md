| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio9: cSC4Audio owner contracts | Codex; re-audio9 | in progress | Inventory export owners, extend specs, run doccheck. |

## Handoff

- Goal: complete the cSC4Audio owner and manager-message contracts in audio specs.
- Done: reused reserved detached worktree; export inventory has 15 cSC4Audio owners.
- Next: specify listener, scape service, SEM registration and preferences window gaps.
- Existing event handler, sound manager, AEMS and ambience contracts are substantial; link them.
- Found incorrect listener-pos direction in audio section 13 (listener sends it).
- Scape vtables missing from ABI table; raw image tables at A94340/A94308/A942F8.
- Evidence source: main checkout export and decomp, read only; no REA session available.
- Do not read or edit engine/, README.md, or docs/HANDOFF.md.
