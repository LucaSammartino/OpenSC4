| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio-au5 (audio handler AU13–AU16) | `Codex` / `re-audio-au5` (RE) | in progress | Resolve AU13–AU16, add spec conformance cases, run doccheck. |

## Handoff
- Goal: close the four audio event-handler behavior questions with original-program evidence; spec text only.
- Done: §7.1.1 now records constructor defaults, AudioScape acquisition, partial Init failure and repeat lifetime behavior (AU14).
- Next: finish AU13 dispatch, AU15 automata payload and AU16 callback/torch/destruction; move all question records and add vectors.
- Main evidence root: C:/Users/tlori/FreeCity4; edits only in this worktree.
- No ABI changes or game requests planned. The module README is explicitly permitted by this task; no implementation reads.
- Evidence: decomp/cSC4AudioEventHandler.c plus byte-verified export instructions; no REA needed.
- Corrections recovered: failed position QI rejects; radio slot10 interface is released; torch routines belong to the main handler.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-audio-au5-r1-manifest.json.
