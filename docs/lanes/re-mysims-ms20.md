| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-ms20: dispatch owner closure | Codex, `re-mysims-ms20` | in progress | Trace MS18/MS20 and document conformance cases. |

## Handoff

- Goal: answer MS18 and MS20 with export-backed behavior and prose conformance vectors; no engine access.
- Worktree reused at detached HEAD; initial tree clean.
- MS20 located in the reserved `glue-mysimdispatch-shadow9` worktree; read that lane's uncompared owner contracts. Its measured result is constructor-only, not live owner coverage.
- Added §4.6.3–§4.6.5: route decisions/masks, idle recovery, neighbour retry failure, foreign model inputs and callbacks. Next: finish MS20 interface/default/observability answer, tool lifecycle and conformance vectors; archive questions, doccheck.
- Verified primary vtable at A857CC has nine slots, not a queue/clock/getter interface. Init/Shutdown and most helpers are direct methods. No public complete input observer.
- Verified neighbour helper initializes a self-linked empty list and never populates it (434CC8–434CDE): returns false regardless of retry index.
- Verified near FE arrival writes current duration +7C, not deadline +80 (43629B–4362C5); same-mode switch fallback clears queued destinations (43641F–436454).
- Evidence source: main checkout `tools/re_query.py`, `decomp/`, `ghidra/export/windows`.
