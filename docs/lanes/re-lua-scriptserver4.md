| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-scriptserver4: ScriptServer owner contract | Codex / `re-lua-scriptserver4` (RE docs, detached worktree) | in progress; inventory pending | 1. Inventory owner bodies. 2. Trace native scheduling and control. 3. Publish contract and vectors. |

## Handoff
- Goal: complete cGZLuaScriptServer owner contract in lua-host.md §6.3; no engine/tools/ABI edits.
- Worktree reused, detached HEAD, initial tree clean; no branch/ref changes permitted.
- Read §1.4 and §6.3, lane/report/merge instructions and short HANDOFF index.
- Published initial inventory, lifetime/Init/Shutdown and control/inspection contract in §6.3; native factory retry and negative pause behavior established.
- Milestone 1 commit 0ff88e05: inventory and lifetime/control publication.
- Attribution reconciled: 51 functions / 7700B; 00677261 (42B) is a false data reach from the string arg (00AC77E8); 50 real bodies / 7658B. Inventory in build-runs/owner-inventory-re-lua-scriptserver4.tsv.
- REA batch failed target_unavailable; fallback to export instructions and existing decomp, no target opening permitted.
- Published creation/ID/error protocol, tick decisions, argument proxy/table, yield hook, destruction and independent vectors. No VM algorithm credited as owner output.
- Milestone 2 commit 50a497be: creation, scheduling, arguments, yield and owner-attribution reconciliation.
- Published slots 11-18, breakpoint state/quirks, notification wire protocol and failure boundaries; shared getters and list copying verified against bytes.
- Next: whitespace/scope review, final report and ready queue. No open owner questions identified; invalidating reentry is native unsafe behavior, not a deferred-control contract.
- Main evidence root: C:/Users/tlori/FreeCity4; edit only this worktree.
- Constructor 008B07C0, primary vtable 00AC8710; secondary 00AC86C4 and 00AC86DC.
- q.py cannot launch Bash (E_ACCESSDENIED); use direct PowerShell capped output. Set PYTHONIOENCODING=utf-8 for searches.
- No game requests or ABI proposals; no code or coverage promotion authorized.
