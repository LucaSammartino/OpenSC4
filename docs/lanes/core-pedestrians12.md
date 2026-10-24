| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians12: pedestrian owners | Codex / core-pedestrians12 | in progress | Implement core pedestrian and one-shot owners, test and gate |

## Handoff
- Goal: close cSC4Pedestrian and cSC4PedestrianOneShot in engine/core/src/pedestrians using automata records.
- Done: bf0b16ff6 initial handoff; 7c2648fdf both core owners, shared records/traffic and Owner closure README. Eleven focused tests, including route installation, empty path, version rejection, inherited one-shot serialization and crowd push consumption. Formatting fixes applied.
- Next: run qbuild all on stable source; collect its terminal result, then native verify if all did not include it. No local test operation remains running.
- PN1 initial-parameter operation is now specified in pedestrian PR conformance; play-count consumer remains open. PN2 belongs to fire crew outside these owners.
- No game requests; no ABI or plugin wiring planned.
- PN3 appended for literal summary choices (cadence, crowd, strike inset, riot randomness); these paths execute. Inherited vehicles VH errors remain inherited, not newly settled.
- Focused logs: build-runs/pedestrians-focused-2.log (syntax error, terminal failure, fixed); pedestrians-focused-3.log (obsolete snapshot, interrupted after >14 min compilation; terminal exit 1). q.py Bash wrapper failed with Windows mapping access denied.
- Shared occupant Read marks status 2: round-trip expectations now explicitly include that documented transformation. Production source unchanged since 1fea61697.
