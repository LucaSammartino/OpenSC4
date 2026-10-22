| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-scriptserver4: ScriptServer owner contract | Codex / `re-lua-scriptserver4` (RE docs, detached worktree) | in progress; inventory pending | 1. Inventory owner bodies. 2. Trace native scheduling and control. 3. Publish contract and vectors. |

## Handoff
- Goal: complete cGZLuaScriptServer owner contract in lua-host.md §6.3; no engine/tools/ABI edits.
- Worktree reused, detached HEAD, initial tree clean; no branch/ref changes permitted.
- Read §1.4 and §6.3, lane/report/merge instructions and short HANDOFF index.
- Published initial inventory, lifetime/Init/Shutdown and control/inspection contract in §6.3; native factory retry and negative pause behavior established.
- Export name inventory: 40 entries / 6724B, not the requested owner total 7700B; reconcile unnamed/folded helpers before completion.
- REA batch failed target_unavailable; fallback to export instructions and existing decomp, no target opening permitted.
- Next: finish RunScriptString instructions, generator protocol, OnTick and global arg callback; then breakpoint bodies and deleting destruction.
- Main evidence root: C:/Users/tlori/FreeCity4; edit only this worktree.
- Constructor 008B07C0, primary vtable 00AC8710; secondary 00AC86C4 and 00AC86DC.
- q.py cannot launch Bash (E_ACCESSDENIED); use direct PowerShell capped output. Set PYTHONIOENCODING=utf-8 for searches.
- No game requests or ABI proposals; no code or coverage promotion authorized.
