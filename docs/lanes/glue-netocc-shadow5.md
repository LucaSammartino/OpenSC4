| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netocc-shadow5: network occupant record shadow | Codex / glue-netocc-shadow5 | in progress | Check native record capture contracts, implement and validate if specified. |

## Handoff
- Goal: default-off netocc_shadow record roundtrip at load/month, exact real-city run through 2032.
- Done: b71de76d initialized lane; inspected record shadow helpers and specified record contracts.
- Implemented: segment-record path-info field is now a 12-byte ResKey, not an object reference; regression tests added.
- NO45 filed: literal Read changes saved flags/alpha/transform masks, so unconditional byte equality requires a specified shadow comparison boundary.
- Next: collect focused cargo/fmt session 66928 to terminal, then run required final all gate and native verify if needed. Hold plugin implementation for NO45.
- Constraints: no native-signature guesses; missing contract requires numbered question and wip queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-glue-netocc-shadow5-r1-manifest.json
- Focused cargo running; q wrapper failed (Git Bash CreateFileMapping error), rerun directly in PowerShell. First fmt used relative paths and failed; absolute-path fmt queued after focused cargo in same session.
- No game requests started; no C ABI or plugin changes.
