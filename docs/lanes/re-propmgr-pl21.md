| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-propmgr-pl21: prop list persistence | `re-propmgr-pl21` / (RE lane, detached HEAD) | in progress | Verify PL21 list versus saved-record membership; document cases and run doccheck. |

## Handoff

- Goal: answer PL21 in prop-lotconfig with source-backed behavior and conformance cases.
- Done: verified slot 23/24, usage filter and both manager writers against export instructions;
  extended A7.2/A8, quirk 20 and six prose conformance cases; moved PL21 to answered archive.
- Next: regenerate questions index, run doccheck, commit the behavior milestone and inspect guard.
- Facts: PL21 is intermittent one-list mismatch with native city records otherwise exact.
- Scope: spec text only; no engine reads/edits, ABI changes, or game runs.
- The Bash q.py wrapper fails with CreateFileMapping error 5; use native PowerShell with capped output.
- Owner closure table is under engine (forbidden); use PL21 and owner lane report as the supplied closure requirements.
- No omitted timed-prop class exists. Fresh queries draw from the saved occupant grid; caller retention
  and changes between Write/query are permitted, but neither is proved to explain the reported game failure.
- Evidence helper: build-runs/filter-vtable.py (read-only PE vtable address lookup).
