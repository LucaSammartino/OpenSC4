| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au5 (audio handler AU13–AU16) | `Codex` / `re-audio-au5` (RE) | in progress | Resolve AU13–AU16, add spec conformance cases, run doccheck. |

## Handoff
- Goal: close the four audio event-handler behavior questions with original-program evidence; spec text only.
- Done: 3e2718e2 records AU14 lifetime rules. AU13–AU16 now specified in §§7.1.1–7.6; question entries moved to answered/audio.md; conformance groups 26–42 added.
- Next: regenerate question index, run doccheck, review diff and finish report/queue.
- Main evidence root: C:/Users/tlori/FreeCity4; edits only in this worktree.
- No ABI changes or game requests planned. The module README is explicitly permitted by this task; no implementation reads.
- Evidence: decomp/cSC4AudioEventHandler.c plus byte-verified export instructions; no REA needed.
- Corrections: failed ordinary-play position QI rejects; radio slot10 acquires nothing and its returned raw data1 pointer is released; torch belongs to the main handler.
- Sound-system posts have four data words; apparent getter arguments in prior spec were stacked post arguments. Main mouse can return false.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-audio-au5-r1-manifest.json.
