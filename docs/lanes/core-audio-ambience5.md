| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-audio-ambience5 (portable ambience manager) | Codex / `core-audio-ambience5` | in progress | Implement owner, focused tests, full gate |

## Handoff
- Goal: close cSC4AudioAmbienceManager §§1–7; foreign layer interface §8.
- Implemented: migrated existing owner to engine/core/src/audio_ambience with compatibility re-export; foreign layer interface, timer/filter/process startup hooks, guaranteed block progress, null guards, safe tuning, explicit AA10 Unsupported.
- Owner closure table and all manager §10 vectors mapped in module README; layer-only vectors 9–18 outside owner.
- Focused tests: 44 passed, 0 failed (build-runs/q/20261023-115959-43812.log). fmt owned files passed.
- Earlier focused result 41/1 retained in build-runs/q/20261023-115552-41184.log; fixed old cell/world and missing-margin expectations. Initial relative-path fmt invocation failed; absolute-path fmt passed.
- Questions appended: AA12 BeginUpdate coordinate type, AA13 duplicate alarm removal, docs/specs/questions/audio-ambience.md. Existing AA10 remains open for absent clock only.
- Next: run mandatory qbuild all once on stable source, inspect terminal result, report and enqueue.
- No game requests; no ABI/plugin changes authorized by scope.
