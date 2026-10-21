| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lot-record-answers4 (spec answers LR1-LR5 for lot-records.md) | `re-lot-record-answers4` / (spec lane) | in progress | - |

## Handoff
- Goal: answer LR1-LR4 (LR5 if supported) in docs/specs/lot-records.md with byte/caller-backed rules; move answered entries; questions.py index.
- Done: lane file; LR1-LR4 spec text + T17-T20 (a1051b84); LR1-LR4 moved to answered, QUESTIONS.md reindexed (72fe4661); LR5 partial: section 7.1 wall add path + GetLotWallEdgeFlags + T21.
- Next: LR5 rest: remove path (add=false, 0x005E3270-0x005E32DD) and UpdateRetainingWalls 0x005E37F0; then finish report; REA has no app open: static fallback (re_query + decomp/).
- Facts: build-runs/vt.py dumps vtables from the exe; build-runs/mvq.py + answers.txt moves answered LR entries.
