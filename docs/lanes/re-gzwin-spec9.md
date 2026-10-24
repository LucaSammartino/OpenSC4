| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzwin-spec9: widget owner contracts | Codex; `re-gzwin-spec9` | in progress | Rank catch-row owners; verify and specify uncovered contracts. |

## Handoff

- Goal: extend widget toolkit specifications for uncovered cGZWin owners; no implementation edits.
- Done: initial checkpoint 59e23d78b; ranked 54 catch-row owners in build-runs/ranked-owners.tsv.
- Done: added widget-generator.md with verified dispatch split, default state, client rectangle and lifecycle/callback boundaries; WTG1 records remaining generator contracts.
- Next: inspect HTML/document window and scripting lifecycle boundaries, then publish owner ranking and conservative coverage proposals.
- Evidence: main checkout `C:/Users/tlori/FreeCity4`; export queried read-only there.
- No game requests, ABI changes, or coverage-map edits.
- Bash q.py failed at CreateFileMapping (access denied); direct PowerShell tools work. Some cGZWinBMP labels at 007B/007C are suspect cross-owner matches.
