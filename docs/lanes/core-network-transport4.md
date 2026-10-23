| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-network-transport4: portable socket/stream owner | Codex / `core-network-transport4` (engine lane) | in progress | Implement §§1–6 bounded transport and literal fixtures; one stable full gate |

## Handoff
- Goal: first portable raw-operation socket/stream owner, no native adapter or NameLookup.
- Start HEAD: 15aced92f138f36741d1bba60490ec3dfdbd7917 (detached reserved worktree).
- Read: rules, lane format, network-transport §§1–6 and §8 transport literals; completing §9/questions next.
- Baseline omitted per task's explicit single stable canonical all instruction.
- Next: finish bounded source/type review; author literal expected fixtures before production implementation.
- No game requests; no ABI/plugin/CAPI changes permitted.
