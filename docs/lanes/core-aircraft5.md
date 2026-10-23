| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-aircraft5 (airplane and helicopter owners) | Codex / `core-aircraft5` | in progress | Read shared contracts, implement aircraft owners, focused tests and final gate |

## Handoff
- Goal: close aircraft.md airplane and helicopter owner behavior in core only; no ABI/plugin changes.
- Done: initial handoff e6652bb7; aircraft/shared contracts read; implemented aircraft records/placement,
  airplane parameter inheritance/defaults, altitude/bounds, phase/ammo/loiter and settled chase rules.
- Done: f4f9ac4b records/behavior milestone. Now 16 aircraft conformance tests and 32 existing automata tests pass;
  logs build-runs/focused-{aircraft,shared}.log. All three aircraft.md §8 cases covered; fmt 2/2 clean.
- Done: failed-write home-vector suppression covered; fire-plane station property initialized via a foreign
  station boundary (not a replacement source/manager). AF8 filed for counts over 32.
- Done: README owner-closure matrix created; AF3–AF8 filed. AF1/AF2 already open.
- Facts: native Init/AI steering/physics/player/messages remain spec-blocked; owners are not fully closed.
- Facts: q.py cannot start Git Bash in sandbox (CreateFileMapping error); direct Python cargo runner works.
  Formatting applied through generated apply_patch from rustfmt stdin; no stdout written directly to source.
- Next: run required qbuild all on stable source, collect actual terminal result, record gate and final WIP report.
- No game requests outstanding. Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-aircraft5-r1-manifest.json.
