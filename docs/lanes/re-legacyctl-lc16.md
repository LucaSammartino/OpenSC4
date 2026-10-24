| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-legacyctl-lc16 (LC14/LC15 native contracts) | `Codex` / `re-legacyctl-lc16` (RE lane) | in progress | 1. Resolve LC14/LC15. 2. Run doccheck. |

## Handoff

- Goal: publish ForceControl/UseWinProc returns and complete build interception boundaries, with vectors.
- Merge of glue-legacyctl-shadow12 in progress; conflicts in engine/capi/src/lib.rs and engine/plugin/FC4Config.cpp resolved with current HEAD versions without reading either file. Other branch changes retained. Integration must be reviewed by coordinator.
- Read requested main owner closure table; report stale open references as well as actual remaining questions.
- Done: merge commit `6301e53bb`; LC14 void contracts and LCV44–45 written from byte-verified instructions.
- Done: LC15 sections 6.1–6.2 and LCV46–50; both questions archived, index regenerated. doccheck PASS via build-runs/lc16-gate.py, log build-runs/lc16-doccheck.log. Initial q.py gate launcher failed before doccheck (Git Bash CreateFileMapping denial); corrected foreground Python launch passed.
- Next: final report and queue. ScriptTag call 0x0094BADA uses primary slot +0x0C; SetParameters has zero exported direct callers and one data reference at 0x00AD536C. Entry span is six bytes to 0x00957AA1; sole return at 0x00957C85. Custom interpreter is a shared one-byte RET, so cannot receive a five-byte entry detour; two builder call spans published instead. Query evidence: build-runs/lc16-boundaries.log.
- No game request needed; inherited run 520 evidence is owner-only, 72/72 exact, not full replay.
