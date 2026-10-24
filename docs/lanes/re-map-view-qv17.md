| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv17 (map-view QV1/2/3/5/11/14/16) | Codex / `re-map-view-qv17` (RE lane; detached HEAD) | in progress | (1) Finish requested dependency merge; (2) settle seven questions from export; (3) doccheck and enqueue. |

## Handoff
- Goal: merge core-map-view-close12, settle QV1 QV2 QV3 QV5 QV11 QV14 QV16 with behaviour and conformance vectors.
- Done: reserved worktree reused; clean initial tree; requested merge started.
- Merge has one conflict in generated docs/specs/QUESTIONS.md; regenerate using tools/questions.py index.
- Next: finish merge, inspect map-view sections/questions and query original functions via main tools/re_query.py.
- Source root: C:/Users/tlori/FreeCity4; edit only this worktree, never engine/.
- REA has no binary; use local export/decomp only. No ABI changes or game runs planned.
- Guard manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-map-view-qv17-r1-manifest.json.
