| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft13b (airplane and helicopter core owner closure) | `core-aircraft13b` / `core-aircraft13b` (engine lane) | in progress | (1) Audit aircraft contracts and current core; (2) implement settled owner behavior and conformance tests; (3) run required gates and report. |

## Handoff
- Goal: close cSC4Airplane and cSC4Helicopter core behavior from aircraft.md and applicable automata.md contracts; no capi/plugin/ABI changes.
- Worktree is reserved and HEAD is detached; commit normally, never update refs.
- Done: verified clean detached worktree; audited aircraft §§2-5 and conformance additions, current owners and AF question registry.
- Existing owners already cover parameters, save codecs and airplane scalar/candidate physics. New settled contracts need LOITER, both phase setters, helicopter preset/reconstruction, reset/hover/chase, building lift, runtime ownership, weapon/takeover/spotlight stages.
- Implemented settled LOITER/ammo/read semantics, ordered phase actions, preset/player/AI initialization, reset/hover, chase gates, building clearance, runtime cleanup, weapons, takeover and spotlight activation stages; added aircraft_settled.rs vectors 14-27 and replaced stale owner closure tables.
- Formatting: qbuild fmt PASS, 9 explicit owned Rust files, diff_lines=0.
- Focused test attempt 1 terminal FAILED (Copy derive on Vec); corrected. Wrapper startup failure and compiler output preserved in build-runs/q/20261024-194343-34628.log and build-runs/aircraft-focused-1.log.
- Next: collect SAME focused attempt 2 (exec session 40815), fix any reported failures, then qbuild all and qbuild verify. Do not launch duplicate checks while this attempt is active.
- Focused attempt 2: build-runs/aircraft-focused-2.log; native logger build-runs/run-native.py avoids Git Bash CreateFileMapping failure.
- Pending native verify: qbuild-20261024-200154-build-ctest, exec session 11556; running marker and fc4verify-{build,ctest}.log under build-runs/. Collect the same operation.
- Post-commit static diagnostic: corrected older helicopter chase fixture's aim/hover expectations to the settled lead point; clarified shared phase/chase docs. No further feature work planned.
- Open by instruction: AF2-AF6, AF10, AF11; do not resolve by inference.
- Required gates: focused tests, qbuild all on stable source, and qbuild verify through coordinator queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft13b-r1-manifest.json.
- No game request outstanding; core-only task has no plugin wiring.
