| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-ambience5 (portable ambience manager) | Codex / `core-audio-ambience5` | in progress | Implement owner, focused tests, full gate |

## Handoff
- Goal: close cSC4AudioAmbienceManager §§1–7; foreign layer interface §8.
- Spec §§1–7, §8 manager boundary, §9 and §10 read. Existing audio/ambience.rs and tests cover much of owner; migrate with compatibility re-export.
- Audit gaps: budget can stall without visiting a block, no explicit null event, timer/filter hooks absent, stale AA4/AA11 comments, AA10 guesses a no-clock value.
- Next: migrate module, close these gaps, extend focused tests and owner table. AA10 remains explicitly Unsupported.
- No game requests; no ABI/plugin changes authorized by scope.
