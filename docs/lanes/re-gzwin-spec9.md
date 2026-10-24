| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwin-spec9: widget owner contracts | Codex; `re-gzwin-spec9` | in progress | Rank catch-row owners; verify and specify uncovered contracts. |

## Handoff

- Goal: extend widget toolkit specifications for uncovered cGZWin owners; no implementation edits.
- Done: initial checkpoint 59e23d78b; ranked 54 catch-row owners in build-runs/ranked-owners.tsv.
- Done: added widget-generator.md with verified dispatch split, default state, client rectangle and lifecycle/callback boundaries; WTG1 records remaining generator contracts.
- Done: generator milestone 59f82984a; added HTML status/scroll/replacement and custom parent-forwarding contracts, with WTH1/WTC1 remaining boundaries.
- Next: run doccheck, review specification accuracy and publish owner ranking/coverage proposals, final report and queue.
- Scope: 15,433 owner bytes newly addressed (Gen 7,049; HTML 8,139; Custom 245), all explicitly partial; existing cGZWin base 12,876 bytes retained. The 100 KB aspiration is not achieved; do not credit whole-owner coverage.
- Lua console sampling exposed unreliable stack recovery; no Lua console rule was added from that evidence.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export queried read-only there.
- No game requests, ABI changes, or coverage-map edits.
- Bash q.py failed at CreateFileMapping (access denied); direct PowerShell tools work. Some cGZWinBMP labels at 007B/007C are suspect cross-owner matches.
