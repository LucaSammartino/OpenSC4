| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-ownedrefs4: AT24 aircraft retained references | Codex / `re-aircraft-ownedrefs4` (RE docs; detached worktree) | in progress | Trace native acquisition and teardown; publish bounded AT24 answer. |

## Handoff
- Goal: resolve only AT24 with native aircraft-manager ownership evidence; no engine/tools/ABI changes.
- Worktree reserved, detached HEAD reused; initial status clean.
- Read automata sections 5.2 and 7.1, AT24, lane/merge rules and HANDOFF index.
- q.py Bash wrapper fails with E_ACCESSDENIED; use bounded PowerShell reads and configured Python query tools.
- Done: published AT24 native contract and seven discriminating vector families in automata.md §§5.2.1–5.2.2; moved AT24 answer to archive.
- Native proof: Read transfers two QI refs into map; aircraft Shutdown 0x004FB7D0 releases airport then plane per entry before base Shutdown; deleting destructor 0x004FBCF0 frees storage only.
- Final manager Release 0x00507B50 calls virtual Shutdown before deleting destructor. PreCityShutdown clears base list only; RemoveOccupant releases plane then airport after erase.
- Next: regenerate question index, verify docs-only diff and exact source citations, commit milestone and run guard.
- Residual: actual HostRelease warning phase/receiver/refcounts absent; no blanket annotation or runtime warning verdict from control evidence.
- Control evidence: both train84 ports reload and ports-disabled control show three airplane warnings; this alone proves no ownership rule.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-aircraft-ownedrefs4-r1-manifest.json`.
