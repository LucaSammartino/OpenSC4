| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv5: QV12/QV13 owner contracts | Codex / re-map-view-qv5 | in progress | Verify selection failure, legend and hotspot effects contracts; write specs and vectors. |

## Handoff

- Goal: answer map-view QV12 and QV13 using original executable evidence; documentation only.
- Done: `18cb63ea` QV12 selection/legend contracts. Added QV13 effects/colour contracts and 12 conformance vectors; moved both questions to answered/map-view.md.
- Next: regenerate questions index, run doccheck, review changed docs and commit final report/queue.
- QV13 evidence: hotspot creates a temporary named effect at scale 100, then reconciliation creates the retained effect at scale 200; null-occupant record is replaced every successful poll. EF is footprint colour, EE is ring colour. Helpers 0x0079F620, 0x0079E0C0, 0x0079E130.
- Worktree is detached by coordinator design; commit HEAD normally, do not switch refs.
- Source evidence is in main checkout; edits only in this worktree; never inspect engine implementation.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv5-r1-manifest.json`.
