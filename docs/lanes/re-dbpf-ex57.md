| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dbpf-ex57: transient DBPF observation | Codex / re-dbpf-ex57 (detached HEAD) | in progress | Resolve EX57 lifecycle and stable-read contract; doccheck. |

## Handoff

- Goal: answer EX57 from glue-dbpf-ex56apply11 using original export only; spec text, no engine reads/edits.
- Done: reused reserved detached worktree; read rules, lane format, EX57 and §8.3.
- Engine report leaves temporary load/save, obliteration and regional segments explicitly unobserved.
- Done: §8.3.1 now specifies typed load/save messages, borrowed/QI ownership, stable read intervals and helper quirks; §10.9 adds nine EX57 vectors. EX57 moved to archive.
- Key findings: PostSave has no void1 and precedes class-object save; regional and obliteration helpers send no ordinary load/save messages. No public all-open-files callback.
- Next: regenerate question index, run doccheck and questions check, review diff and commit milestone.
- Evidence paths: main C:/Users/tlori/FreeCity4; tools/re_query.py; decomp/.
- No ABI changes or game runs planned. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-dbpf-ex57-r1-manifest.json.
