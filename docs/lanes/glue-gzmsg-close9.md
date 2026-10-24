| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-close9: GZ manager verification | Codex; glue-gzmsg-close9 | in progress | Implement GZ11/GZ12, gate and measure to 2032 |

## Handoff
- Goal: acquire the optional native queue manager per GZ12 and compare only GZ11-observable state.
- Done: GZ11/GZ12 implemented explicitly: optional QI acquisition, ownership only on success, bounded count plus lookup baseline, dated lowercase summary.
- Done: eight GZ-V15–V22 tests added; focused core tests 20/20 pass (includes all eight).
- Next: finish explicit-file formatting, commit implementation, full gate and queued native verification, game to 2032.
- Facts: spec §3.2 confirms shipped startup does not register this optional manager. Absence must remain unavailable/mismatches=1; no factory fallback or verification credit.
- GZ11 forbids whole-owner inference even from equal bounded probes/counts. No new unanswered contract found.
- Tool evidence: q.py Bash fails with Windows CreateFileMapping error; direct PowerShell cargo works. qbuild fmt needs absolute owned-file paths.
- Required evidence: stable-source all/verify gates, house shadows run through 2032, clean references and exceptions.
- No game request outstanding; no source edits yet.
