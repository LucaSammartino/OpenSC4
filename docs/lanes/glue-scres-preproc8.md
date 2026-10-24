| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-scres-preproc8 physical preprocessing | Codex / glue-scres-preproc8 | in progress | Restore merged replay, implement SCR11, gate and measure 2032 raw replay |

## Handoff
- Goal: SCR11 physical file preprocessing, 16 vectors, full natural scres replay exact to 2032.
- Merged requested branches at 866a2a718; octopus emitted packed-refs permission diagnostic but committed.
- Merge tree unexpectedly removes replay7 files; inspect branch diff and restore only replay work before implementation.
- Restored replay files and SCR11 spec/answered entry using apply_patch; preserved current ABI93 and overlay configuration.
- Open questions file already has no SCR11 entry. Read replay7 report and SCR11 section12 with all 16 vectors.
- Implemented production SCR11 filter with invocation-local flag, removed foreign preprocessing replay and stale SCR11 difference label.
- Focused core SCR-V93–V108:16/16; capi raw replay:3/3 passed. First wrapper attempt failed before execution (Git Bash mapping denial), direct Cargo worked.
- Applied rustfmt check diffs via apply_patch to the owned tests; final format check and full gate next.
- Next: qbuild all (native verification must use coordinator queue), then combined/default 2032 turbo runs.
- No game request outstanding. No ABI export additions intended beyond inherited replay branch.
- Worktree detached by supervisor; commit normally, do not update refs.
