| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-landfill-shadow12: landfill owner shadows | Codex / glue-landfill-shadow12 | in progress | Inspect owner probes, implement supported comparisons, gate and run to 2032 |

## Handoff
- Goal: measure exact shadows for all three landfill owners in the house city through 2032.
- Done: implemented LG33 good-stream holder-rejection handling; focused reader tests 2/2 pass (build-runs/focused-reader.log). Extended occupant probe to 11 cases and 7 reloads; corrected LG32 void SetPosition call.
- Next: stable-source full gate, queued native verification, then house city to 2032.
- Clean room: tools/ tables and regress script are forbidden by the overriding clean-room rule; coordinator must handle their credit/profile updates.
- Existing LG31 explicitly parks native zone verification: no public callable contract. Do not claim zone verification through developer serialization.
- No game request outstanding. q.py failed because Git Bash CreateFileMapping is denied; capture helper uses direct subprocess instead. Initial fmt used wrong relative root; applied absolute-path fmt suggestions.
