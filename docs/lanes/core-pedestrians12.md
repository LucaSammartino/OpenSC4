| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians12: pedestrian owners | Codex / core-pedestrians12 | in progress | Implement core pedestrian and one-shot owners, test and gate |

## Handoff
- Goal: close cSC4Pedestrian and cSC4PedestrianOneShot in engine/core/src/pedestrians using automata records.
- Done: bf0b16ff6 initial handoff; implemented both core owners, shared records/traffic, ten focused tests and Owner closure README. Formatting passes (3 owned files).
- Next: collect focused cargo sessions 41288 (old syntax error, still terminating) and 12709 (corrected source, queued behind its lock); fix diagnostics, then all and verify once stable.
- PN1 initial-parameter operation is now specified in pedestrian PR conformance; play-count consumer remains open. PN2 belongs to fire crew outside these owners.
- No game requests; no ABI or plugin wiring planned.
- PN3 appended for literal summary choices (cadence, crowd, strike inset, riot randomness); these paths execute. Inherited vehicles VH errors remain inherited, not newly settled.
- Focused logs: build-runs/pedestrians-focused-2.log (old syntax error); pedestrians-focused-3.log (current operation). q.py Bash wrapper failed with Windows mapping access denied; direct cargo uses --manifest-path engine/Cargo.toml.
