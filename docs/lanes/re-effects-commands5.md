| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-effects-commands5 — effects script command contracts | Codex / re-effects-commands5 | in progress | Trace command owners, publish contracts and vectors, answer EF15 and EFP1–EFP6. |

## Handoff
- Goal: publish effects-script command behavior for the engine lane; no engine edits.
- Done: published effects-parser §§9–11, effects §19, and 25 static vector rows; EFP1/EFP4/EFP5 answered.
- Next: establish EFP6 dispatch hooks, EFP2 rotation stores, remaining EFP3 outer recovery, then EF15 remaining owners.
- Corrected prior spec error: AddRef/Release are F3C0/F3D0, not F3E0/F3F0; destructor 005A27F0 -> 0059F4E0.
- Static evidence only; no game requests, no ABI changes. EFP7 deliberately outside task.
- Ignored helpers: build-runs/peek.py (PE data/import/reference reads), inventory.py (registration navigation only).
- inventory.py only handles puVar1/PTR_FUN registrations: select, particleSequence and folded generic handlers NEED manual proof; do not publish its stale descriptor guesses.
- EFP1–EFP6 are in questions/effects-parser.md, EF15 in questions/effects.md.
- Explicitly permitted engine read: main checkout effects_parser/README.md only.
- q.py cannot start Git Bash (CreateFileMapping access denied); use bounded PowerShell calls.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-effects-commands5-r1-manifest.json.
