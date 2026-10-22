| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-ownedrefs4: AT24 aircraft retained references | Codex / `re-aircraft-ownedrefs4` (RE docs; detached worktree) | in progress | Trace native acquisition and teardown; publish bounded AT24 answer. |

## Handoff
- Goal: resolve only AT24 with native aircraft-manager ownership evidence; no engine/tools/ABI changes.
- Worktree reserved, detached HEAD reused; initial status clean.
- Read automata sections 5.2 and 7.1, AT24, lane/merge rules and HANDOFF index.
- q.py Bash wrapper fails with E_ACCESSDENIED; use bounded PowerShell reads and configured Python query tools.
- Next: inspect exported Read 0x004FBE70, aircraft constructor/clear/Shutdown/destructor and map reference operations.
- Control evidence: both train84 ports reload and ports-disabled control show three airplane warnings; this alone proves no ownership rule.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-aircraft-ownedrefs4-r1-manifest.json`.
