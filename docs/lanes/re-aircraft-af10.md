| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-af10 (aircraft AF3/AF4/AF5/AF9) | Codex / `re-aircraft-af10` | in progress | Trace owner closure questions, write behavior and vectors, run doccheck |

## Handoff
- Goal: close aircraft AF3/AF4/AF5/AF9 using export/decomp evidence only; partial closure is allowed if budget requires.
- Done: AF9 moved to answered; aircraft §4.1.1 specifies thirteen exact preset words, player-before-AI overlay, 400 versus 227 speed quirk, Read/Init reconstruction and phase 0 reset. Added conformance vectors 14–16; questions index regenerated.
- Next: run direct foreground doccheck (q.py Bash wrapper failed with CreateFileMapping Win32 error 5), commit AF9; assess remaining budget for AF3/AF4/AF5.
- No engine edits, ABI changes, or game runs. No REA session is available; use re_query.py and exported pseudocode.
- Main evidence root: C:/Users/tlori/FreeCity4; worktree is this lane directory.
- Preset 0 base 00B09B70, stride 7C; Init always calls InitInstance(0); PhysicsInit sets phase 0 at 005361E1–005361EE. Local read-constants.py reads PE file-backed data; first diagnostic's float column was corrected before specification.
