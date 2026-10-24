| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-automata-crash6: owners capture reference crash | Codex, `glue-automata-crash6` | in progress | Diagnose capture IDs, test, gate, three 2032 runs |

## Handoff
- Goal: fix stale/foreign capture replay references in default-off automata owners shadow.
- Done: copied supplied crash evidence to build-runs/; confirmed GetRef dereferences the capture table without lifetime ownership.
- Implemented: automata captures opt into retained, QI-normalized serializable references; reject class/ID collisions before replay QI; audit replay QI balance.
- Added native DLL seam regression for adjusted effect interface, writer-released temporary, duplicate/null IDs, unknown ID and in-range wrong-class ID.
- Next: run final all gate and queued native verify on stable source; inspect terminal verdicts.
- Required: focused native bad-ID test; final all and queued verify; three unique exact 2032 game runs.
- Keep owners shadow default-off. Coordinator should add automata_owners_shadow to the house profile after exact validation (tools are outside lane scope).
- No outstanding game request. No spec behavior or ABI export changes; this is capture tooling ownership.
