| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-regionalcity9 (regional city core) | Codex / core-regionalcity9 | in progress | Implement owner and conformance tests |

## Handoff
- Goal: implement cSC4RegionalCity from regional-city, regional-city-records and foundations §11.
- Worktree reused at detached HEAD; no plugin or C ABI changes authorized.
- Done: owner state, statistics, deal lifecycle, codec and connection algorithm implemented; initial vectors added.
- Diagnostic: first cargo check found a closure borrow overlapping job-field assignment; fixed by computing both results before assignment.
- Next: finish host-effect/cache tests and owner closure table, run focused tests, fmt and full gates.
- Question RY7 records literal Read edge-case choices; raw codec round-trip is separate from object Read normalization.
- q.py Bash wrapper fails CreateFileMapping before execution; direct Cargo works.
- Required final checks: qbuild all and verify; no game integration in this lane.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-regionalcity9-r1-manifest.json.
