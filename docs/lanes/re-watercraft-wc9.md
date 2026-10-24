| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-wc9: WC9 and owner closure contracts | Codex / re-watercraft-wc9 | in progress | Verify steering predicates, then remaining lifecycle, animation and heel contracts. |

## Handoff

- Goal: answer WC9, WC3, WC4, WC5 with prose and conformance vectors; no engine reads or edits.
- Worktree reused at detached HEAD; main/export are read only.
- Read core-watercraft9's lane report in its worktree: WC9 exists there but has not reached this worktree.
- Owner closure table is apparently in engine's module README, which this task expressly forbids reading. Use the engine lane report's remaining-work list and spec §10.2 instead.
- Current measured owner attribution in spec: 75 entries / 38,073 bytes, not historical 79 / 38,556.
- Done: WC9 sign gates verified at 0056EC87–0056ECF2; corrected WC8-08 to T=-0.1, L=R=100 and recorded answer.
- Next: finish inherited lifecycle/animation helpers, AI target selection and grounding basis; trace remaining contracts using export and decomp only.
- No game requests, ABI changes, or REA calls.
