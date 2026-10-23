| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rigid-body-2d4 (first native rigid-body simulator contract) | Codex / `re-rigid-body-2d4` | in progress | Establish finite collision family and exact phase boundary; docs only |

## Handoff
- Goal: first bounded native `cSC4RigidBodySimulator2D` spec, independent of particle/emitter contracts.
- Authority: own round-1 manifest `C:/Users/tlori/FreeCity4/build-runs/run4-re-rigid-body-2d4-r1-manifest.json`; detached HEAD intentionally retained.
- Done: first spec publishes native identity, constructor stores, complete finite primary-sector/support/contact-velocity family and six independent proposed vectors; first milestone commit follows.
- Evidence: executable version 1.1.641.0, SHA-256 matches export; static audit finds zero instruction-byte mismatches across 16 owner ranges (not body completion).
- Next: qualify TestForCollision candidate branches and ResolveCollision; then exact phase boundary and questions.
- Scope: canonical full effects(particles,decals,visuals), REGO0, no dependencies; no native/default/owner coverage promotion.
- Restrictions: docs only; no engine, ABI, live calls, game requests, subagents, shared state writes.
- Environment: q.py Bash wrapper fails E_ACCESSDENIED; use bounded native PowerShell reads and Python queries.
- Live status: NoApp/live UNKNOWN preserved; no REA request needed so far.
