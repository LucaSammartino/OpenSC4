| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians12: pedestrian owners | Codex / core-pedestrians12 | in progress | Implement core pedestrian and one-shot owners, test and gate |

## Handoff
- Goal: close cSC4Pedestrian and cSC4PedestrianOneShot in engine/core/src/pedestrians using automata records.
- Done: bf0b16ff6 initial handoff; 7c2648fdf both core owners, shared records/traffic and Owner closure README. Ten pedestrian tests, including route installation, empty path, version rejection, inherited one-shot serialization and crowd push consumption. All ten passed inside the full workspace run.
- Next: collect active qbuild all session 82150 to its terminal result, then native verify if all did not include it. Do not launch another check. All source is stable at f43d98b78.
- PN1 initial-parameter operation is now specified in pedestrian PR conformance; play-count consumer remains open. PN2 belongs to fire crew outside these owners.
- No game requests; no ABI or plugin wiring planned.
- PN3 appended for literal summary choices (cadence, crowd, strike inset, riot randomness); these paths execute. Inherited vehicles VH errors remain inherited, not newly settled.
- Focused logs: build-runs/pedestrians-focused-2.log (syntax error, terminal failure, fixed); pedestrians-focused-3.log (obsolete snapshot, interrupted after >14 min compilation; terminal exit 1). q.py Bash wrapper failed with Windows mapping access denied.
- Shared occupant Read marks status 2: round-trip expectations now explicitly include that documented transformation. Production source unchanged since 1fea61697.
- Full gate started 16:45 local; at 17:09 it was still compiling workspace crates, no diagnostics. Console: build-runs/pedestrians-all-1-console.log; test log: build-runs/qbuild-test.log. All remains in the current turn for terminal collection.
- Rust milestone: workspace tests finished with 7,383 passed, 0 failed, 0 ignored across 48 suite summaries; all ten `pedestrians::tests::*` passed. Formats sweeps and doc tests completed. Clippy is checking core/conformance; native result not yet available. The same all operation (82150) is still active.
