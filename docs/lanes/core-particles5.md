| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-particles5: particle owner closure | Codex / core-particles5 | in progress | Audit existing modules against particles spec, implement and test owner closure |

## Handoff
- Goal: close the seven assigned particle owners against particles.md; core only, no ABI changes.
- Done: extended existing emitter with full creation/motion records and draw-before-finish commands; added geometry, decal, dynamic, description/sequence and noncontact body contracts.
- Done: implementation milestone be820a12; focused pre-review run passed 43 particle tests; later ownership/bootstrap audit corrections require final verification.
- Gate 1: 6456 tests passed, 0 failed/ignored; clippy failed on four redundant conversions/casts, now fixed; full failed evidence build-runs/first-gate-clippy.log.
- Next: collect SAME qbuild all session 84690, native queue qbuild-20261023-214445-build-ctest (pre-review source); then run all once on corrected stable source. Do not duplicate pending operation.
- Review corrections: uninitialized geometry teardown source leak, pending decal handle retry leak, body angle/friction Unsupported prefix, constructor empty containers, explicit PT13 bootstrap boundary.
- Open: PT1/2/3/8/9/10/11 existing numeric limits; filed PT12 primary-sector spec conflict and PT13 ordinary bootstrap/transform inventory.
- Native game requests: none; core-only and no plugin/capi wiring.
- Closure map: engine/core/src/effects/particle_owners/README.md (includes precise unresolved domains).
- Constraints: renderer/terrain remain foreign traits; no plugin/capi edits; no game integration requested.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-particles5-r1-manifest.json.
