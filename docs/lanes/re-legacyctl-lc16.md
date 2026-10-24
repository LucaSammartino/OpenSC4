| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacyctl-lc16 (LC14/LC15 native contracts) | `Codex` / `re-legacyctl-lc16` (RE lane) | in progress | 1. Resolve LC14/LC15. 2. Run doccheck. |

## Handoff

- Goal: publish ForceControl/UseWinProc returns and complete build interception boundaries, with vectors.
- Merge of glue-legacyctl-shadow12 in progress; conflicts in engine/capi/src/lib.rs and engine/plugin/FC4Config.cpp resolved with current HEAD versions without reading either file. Other branch changes retained. Integration must be reviewed by coordinator.
- Read requested main owner closure table; report stale open references as well as actual remaining questions.
- Next: commit merge, query native owner functions and build callers through re_query.py; no REA binary available.
- No game request needed; inherited run 520 evidence is owner-only, 72/72 exact, not full replay.
