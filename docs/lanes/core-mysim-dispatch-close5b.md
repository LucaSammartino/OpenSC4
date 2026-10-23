| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-mysim-dispatch-close5b (dispatch settled MS18/MS19) | Codex / `core-mysim-dispatch-close5b` (engine lane) | in progress | Implement settled contracts, focused tests, final gate |

## Handoff

- Goal: implement every newly settled dispatch/location owner contract; retain explicit MS18 residuals.
- Done: `fe37fbca` implements MS19 exact equality, unconditional teleport and state-1 commute; five executable fixtures pass.
- Done: settled MS18 geometry, ordinary waits, stop/boarding, final observations/common tail; owner table updated. Conformance 22/22, focused core dispatch 27/27 pass; fmt 5 files clean.
- Next: run qbuild all once on stable source, then queue a unique mysims shadow regression and wait for terminal verdict.
- Remaining Unsupported: MS18 route branch/mask/index selection, neighbour retries, full completion/recovery ordering, input/pylon lifecycle, mission callbacks, delegated RNG. Helpers do not invent route selections.
- Initial MS18 geometry fixture failed because its null point resolved an overlapping lot; corrected fixture uses an occupied point with no lot. Corrected suite passes.
- Focused check: direct cargo test -p fc4-conformance --test mysim_dispatch ms19: 5/5 PASS.
- q.py Git Bash startup failed (CreateFileMapping error 5); evidence build-runs/q/20261023-174301-17120.log; direct Cargo works.
- No ABI/plugin/capi changes; no game request outstanding.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-mysim-dispatch-close5b-r1-manifest.json`.
