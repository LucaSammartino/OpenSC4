| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv17 (map-view QV1/2/3/5/11/14/16) | Codex / `re-map-view-qv17` (RE lane; detached HEAD) | in progress | (1) Finish requested dependency merge; (2) settle seven questions from export; (3) doccheck and enqueue. |

## Handoff
- Goal: merge core-map-view-close12, settle QV1 QV2 QV3 QV5 QV11 QV14 QV16 with behaviour and conformance vectors.
- Done: dependency merge 8caf1b84c; generated question-index conflict resolved by regeneration.
- Done: QV5/QV16 instruction-verified; corrected factor defaults and vectors 1/21; moved questions to answered.
- Done: QV11 developer-layer rule and vector 34; layer 1 draws nothing, layers 3/4 identical, ordered debug triples.
- Done: QV1 terrain plumbing/water display, zone alpha default/reset, F1 filter-only override; vector 35.
- Next: doccheck and final partial-ready report; budget wind-down. QV2/QV3/QV14 remain open.
- QV14 evidence: CRT __setdefaultprecision 0x009F6C30 sets PC=53 via controlfp; ring does not set CW.
- QV2 evidence: source 76 builds three shift-1 zero grids; traffic ordinary path consumes slot-12 records,
  special selected view 0x54 consumes compressed ferry routes. Read cSC4WinMapView.c lines 1902-2360.
- Source root: C:/Users/tlori/FreeCity4; edit only this worktree, never engine/.
- REA has no binary; use local export/decomp only. No ABI changes or game runs planned.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv17-r1-manifest.json.
