| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-wt29 / widget-toolkit (RE, docs) | Codex / re-wt29 / detached HEAD; reserved lane re-wt29 | ready; content commits 2d236ed70, 74631e8dd; doccheck passed, questions passed; cargo/clippy/ctest n/a; game n/a, shadow n/a; WT1, WT3, WT10, WT11, WT12, WT13 answered; ABI exports unchanged | 1. Coordinator land queued docs tip |

- Updated [widget-toolkit](../specs/widget-toolkit.md) with rules and worked cases for all six assigned questions.
- WT1: 151 callable base slots; no pure-virtual traps; enumerated constant-result stubs; zero padding is not slot 151.
- WT3: dialog dragging reads 0x100; manager focus eligibility reads 0x8000; preserve default 0x8903.
- WT10–12: revalidated existing answers, documented temporary filter references and a caller-owned root surviving manager destruction.
- WT13: fade/base share deleting wrapper 0x009D060A; no extra derived teardown; documented inherited never-initialized parameter-reference leak.
- Archived WT1/WT3/WT13; reconciled the already-archived WT10–12 with stale duplicate open entries. Regenerated question index: toolkit 5 open / 12 answered.
- Evidence: original export instructions with bytes, targeted decompilation reads, raw image vtable words. REA was not needed.
- Checks: direct doccheck check, questions check and git diff --check passed. No runtime observation remains necessary for these six answers.
- q.py initially failed to launch Git Bash (Win32 error 5); direct PowerShell/Python checks subsequently passed. Git commits succeeded despite a packed-refs.lock permission warning.
- ABI proposals: none; docs/abi and C ABI exports unchanged. No engine files read or changed.
- Remaining questions concern other assigned scopes; no additional owner implementation or in-game run was attempted.
