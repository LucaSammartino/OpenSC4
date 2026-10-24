| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft13b (airplane and helicopter core owner closure) | `core-aircraft13b` / `core-aircraft13b` (engine lane) | in progress | (1) Audit aircraft contracts and current core; (2) implement settled owner behavior and conformance tests; (3) run required gates and report. |

## Handoff
- Goal: close cSC4Airplane and cSC4Helicopter core behavior from aircraft.md and applicable automata.md contracts; no capi/plugin/ABI changes.
- Worktree is reserved and HEAD is detached; commit normally, never update refs.
- Done: verified clean detached worktree; audited aircraft §§2-5 and conformance additions, current owners and AF question registry.
- Existing owners already cover parameters, save codecs and airplane scalar/candidate physics. New settled contracts need LOITER, both phase setters, helicopter preset/reconstruction, reset/hover/chase, building lift, runtime ownership, weapon/takeover/spotlight stages.
- Next: implement these settled stages and update obsolete conformance expectations; keep complete flight/AI/player/message gaps explicit under existing AF2-AF6/AF10/AF11.
- Open by instruction: AF2-AF6, AF10, AF11; do not resolve by inference.
- Required gates: focused tests, qbuild all on stable source, and qbuild verify through coordinator queue.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft13b-r1-manifest.json.
- No game request outstanding; core-only task has no plugin wiring.
