| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-audio4 (RE spec: audio ownership reconcile + ambience family) | `re-audio4` / (RE lane) | in progress | see Handoff |

## Handoff

- Goal: reconcile docs/specs/audio.md to exact owner classes (coverage-map rows), then extend the
  largest uncovered coherent cSC4Audio family: ambience (cSC4AudioAmbienceManager 11298 B,
  cSC4AudioAmbienceLayer 14417, cSC4AudioScapeManager 9811, cGroupLayer 2029, cSC4AudioListener 2799).
- Done: lane file.
- Next: REA preflight; survey ambience manager vtable/methods; write docs/specs/audio-ambience.md.
- Facts: per-owner bytes from `tools/coverage.py --owners build-runs/owners.tsv` (audio total 225,440 B).
