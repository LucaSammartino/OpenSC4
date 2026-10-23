| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animsprite5: animated sprite owner | Codex / `core-animsprite5` | in progress | Reconcile existing owner with current spec, implement remaining settled contracts, validate. |

## Handoff
- Goal: close cS3DModelInstanceAnimatedSprite against animated-sprite.md, core only, no ABI/plugin changes.
- Worktree reused at detached HEAD; initial status clean, no commits ahead of main.
- Existing module has partial implementations and AS1/AS3-AS9 gaps; read current spec before deciding remaining work.
- Implemented in working source: constructor/shared setters/getters, byte frames/controller snapshot, reverse stepping, partial load rollback, general raw selection formulas and finite placement schedules.
- Focused attempt log build-runs/q/20261023-140537-11028.log failed because core forbids unsafe: removed inline x87 backend, replaced by safe exact dyadic carrier. This replacement still needs focused compile/tests and validation of Unsupported propagation.
- Next: finish numeric checked boundaries/tests, append AS10 (LookAt spills/native angle qualification) and AS11 (foreign factory contract), replace README with Owner closure table, run focused cargo then final qbuild all.
- Source remains WIP: nonzero hardware trig/PC-RC qualification open AS6; numeric mode cannot be guessed. Keep explicit Unsupported.
- No game requests outstanding. Full final qbuild all required.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-animsprite5-r1-manifest.json.
