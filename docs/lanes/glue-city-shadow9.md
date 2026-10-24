| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-city-shadow9: city shadow | Codex / glue-city-shadow9 | in progress | Inspect contracts and implement city comparison. |

## Handoff
- Goal: default-off city_shadow; independent Rust comparison at load/month; real city to 2032.
- Done: implemented bounded geometry shadow, 18 fields/load and month; HG8 filed for absent city persistence contract.
- Next: focused tests, full all gate, queued native verify, real city house run to 2032.
- Facts: save.rs is simulator v4, NOT city. No city scalar contract; all methods except PositionToCell/CellWidthX/Z uncompared.
- Diagnostics: relative fmt paths failed (rerun with absolute); q.py Bash blocked CreateFileMapping Win32 error 5; use direct PowerShell cargo.
- Constraints: no RE sources; no tools/ reads/edits; no debug registers; detached HEAD commits.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-city-shadow9-r1-manifest.json.
