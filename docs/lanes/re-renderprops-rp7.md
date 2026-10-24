| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-renderprops-rp7 (render properties owner closures) | Codex / `re-renderprops-rp7` (RE lane) | in progress | Locate RP7 and shadow9 owner closure table; verify export and document answers. |

## Handoff
- Goal: answer RP7, add prose conformance vectors, move answered question, regenerate index, pass doccheck.
- Done: reused reserved detached worktree; working tree initially clean.
- Next: verify view snapshot/copy and setter ABI instructions, then specify RP7 live-history limits and observer protocol.
- Facts: RP7 and shadow9 report located in sibling glue-renderprops-shadow9 worktree, docs only. No Owner closure table exists there; its explicit uncompared-owner list is the closure scope.
- RP7 asks for ordered live setter inputs, view acquisition, and a non-mutating observer compatible with SCRes/weather DR0. No public history interface is evident; view embeds a snapshot at +0x380 and scroll end restores by table copying, bypassing setters.
- q.py failed because Git Bash CreateFileMapping returned Win32 error 5; use bounded PowerShell commands.
- No engine files read or edited; no ABI changes or game runs planned.
