| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzmsg-close9: GZ manager verification | Codex; glue-gzmsg-close9 | in progress | Implement GZ11/GZ12, gate and measure to 2032 |

## Handoff
- Goal: acquire the optional native queue manager per GZ12 and compare only GZ11-observable state.
- Done: GZ11/GZ12 implemented explicitly: optional QI acquisition, ownership only on success, bounded count plus lookup baseline, dated lowercase summary.
- Done: eight GZ-V15–V22 tests added; focused core tests 20/20 pass (includes all eight).
- Done: implementation committed as 6c10a0bc0; explicit owned-file fmt PASS.
- Done: stable-source qbuild all Rust phase PASS: 6992 passed, 0 failed, 0 ignored; clippy 0 diagnostics.
- Done: native request qbuild-20261024-074607-build-ctest terminal PASS: build 253/253, CTest 79/79. qbuild all PASS; source unchanged during gate (docs-only commit accounts for tree-changed note).
- Next: game to 2032; record actual gzmsg clean/total. Native verification already collected; do not duplicate.
- Facts: spec §3.2 confirms shipped startup does not register this optional manager. Absence must remain unavailable/mismatches=1; no factory fallback or verification credit.
- GZ11 forbids whole-owner inference even from equal bounded probes/counts. No new unanswered contract found.
- Tool evidence: q.py Bash fails with Windows CreateFileMapping error; direct PowerShell cargo works. qbuild fmt needs absolute owned-file paths.
- Required evidence: stable-source all/verify gates, house shadows run through 2032, clean references and exceptions.
- Current house-shadows ini profile requested asynchronously from coordinator/user (tools/regress.ps1 cannot be read); no answer. Proceeding with focused diagnostic, do not call it house profile.
- Game request gzmsg-close9-r1-2032 about to be submitted; one request only. Collect through build-runs/wait-gzmsg-verdict.ps1 (no qwait).
- Run number scan currently max 462; rescan immediately before reserving final record.
