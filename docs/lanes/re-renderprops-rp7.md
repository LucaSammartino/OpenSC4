| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-renderprops-rp7 (render properties owner closures) | Codex / `re-renderprops-rp7` (RE lane) | in progress | Locate RP7 and shadow9 owner closure table; verify export and document answers. |

## Handoff
- Goal: answer RP7, add prose conformance vectors, move answered question, regenerate index, pass doccheck.
- Done: reused reserved detached worktree; working tree initially clean.
- Done: §1.5 and §7.4 now specify RP7 and eight instruction-derived vectors; archived the original RP7 from the sibling question file.
- Next: regenerate/check questions, run doccheck, review diff, commit spec milestone.
- Facts: RP7 and shadow9 report located in sibling glue-renderprops-shadow9 worktree, docs only. No Owner closure table exists there; its explicit uncompared-owner list is the closure scope.
- RP7 asks for ordered live setter inputs, view acquisition, and a non-mutating observer compatible with SCRes/weather DR0. No public history interface is evident; view embeds a snapshot at +0x380 and scroll end restores by table copying, bypassing setters.
- q.py failed because Git Bash CreateFileMapping returned Win32 error 5; use bounded PowerShell commands.
- No engine files read or edited; no ABI changes or game runs planned.
- Verified instruction bytes: setters 007DDE60/80/A0; snapshot construction 007F0E32–70; begin 007F6410 and end 007F6160; view public vtable 00ABCE40/QI 007E5630; application getter 0044B660; text helper 007ECDA0.
- Snapshot flag B30 is kNoPartialBackingStoreCopies (not a generic low-resolution flag); copy includes P+8 refcount. Protocol is explicitly adapter policy, not an implemented native API or executed observer.
