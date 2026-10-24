| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au50 (audio owner contracts) | Codex / `re-audio-au50` | in progress | Answer AU45–AU49 and AA14; doccheck |

## Handoff

- Goal: close core-audio-close11's six RE questions with prose contracts and conformance cases.
- Done: initial handoff bb6273807; AU45 dispatch vector corrected and source question preserved.
- Next: finish SoundManager AU46/AU49 contracts, then Director/AEMS/Layer lifecycle contracts.
- Verified: play uses manager master volume, not tSound+14; radius derives from signed property 0x38.
- Source questions read with git show core-audio-close11; no closure table exists in its lane report.
- Main source: `C:/Users/tlori/FreeCity4`; edit only this worktree.
- q.py Bash launch fails with Win32 error 5; use capped PowerShell commands directly.
- No engine reads, REA calls, ABI changes or game runs planned.
