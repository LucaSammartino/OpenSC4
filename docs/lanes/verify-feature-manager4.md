| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| verify-feature-manager4: independent native query verification | `Codex` / `verify-feature-manager4` (engine lane) | in progress | Audit FM3/FM4; implement independent native query/lifetime verification; gate and game sessions. |

## Handoff
- Goal: compare app-owned native cSC4FeatureManager query behavior with independent Rust inputs under published contracts.
- Base: detached HEAD 17319e21; reuse reserved worktree, never update refs.
- Done: initialized accounting; read lane format and published identity/lifetime contracts.
- Next: baseline qbuild all; inspect feature manager core/plugin and provider contracts; commit code by call 30.
- Restrictions: no native Init/Shutdown/reset/seeding; borrowed app getter and owned QI hold only; FM1/FM2 excluded.
- Game requests: none outstanding. Unique labels required; wait without qwait.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-verify-feature-manager4-r1-manifest.json.
