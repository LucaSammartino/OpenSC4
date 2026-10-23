| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-netocc-answers5 (network owner questions) | Codex / `re-netocc-answers5` | in progress | Resolve NO33–NO37, then UG5; document remaining gaps |

## Handoff
- Goal: behavior specs for power-pole, pylon and connection owners; no engine changes.
- Done: NO34 answered: signed-int64 property 0x0A2ECED8 supplies quote; corrected test 17's erroneous E9A18D3F coupling.
- Done: NO37 resolved: primary/serializable mapping, one transform, u16 call, string tail, ref IIDs/releases, placement result table, repeat Init and shutdown; tests 19/20 expanded.
- Done: NO35 resolved: record/live mapping, nested counts, wall codec, shared-table vectors, ref IIDs and failure cleanup; test 16 expanded.
- Next: final consistency/diff/question checks and landing report; NO33/NO36/UG5 require a follow-up lane.
- Facts: all record offsets need +C for primary-relative layout. Pylon alleged int64 is GZStr; actual renderer insert is +70, removal +74. NO36 remains open beyond lifecycle/packing inputs.
- Sources: main checkout absolute paths; all edits in this worktree. Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-netocc-answers5-r1-manifest.json`.
- Questions are moved only when the complete requested rule is supported; partial answers stay open.
