| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-lotwall29: wall records and network drag; engine | Codex / close-lotwall29 / reserved branch close-lotwall29 (detached HEAD); code bf2c9113f, e8c569ad0 | ready for exactness review; ABI unchanged; fmt PASS; cargo 7894/7894; clippy clean; CTest 124/124; house clean 73/73; gen27 clean 25/25; questions none; whole-owner credit remains partial | 1. Enqueue engine changes. 2. Credit only complete methods; full wall-owner output tracing remains unimplemented. |

All game logs below are in `C:/Users/tlori/FreeCity4/build-runs/game-results/`.
Wall cause: lot-records §7/T7/T88 makes Read OR mask bits for nonzero hashes; comparing pre-Read Write with post-Read Write was invalid.
Fix preserves every live byte: independently checks Write snapshot and detached native/core Read-then-Write, same reference identities, no live mutation or normalization.
Evidence: `close-lotwall29-house-r1.log`, first monthly wall: expected=visited=captures=exact=106, differences=errors=0; native transition byte 40: 4→6 and 4→7.
House acceptance: `close-lotwall29-house-r1.verdict` clean, ref_warnings=0, 2032-01-06; house shadows profile, no action script.
`close-lotwall29-house-r1.log`: lotwall 73/73 exact samples, 6466 records, complete=1 each inventory, zero mismatches/errors/resyncs; 2809 native Read mask transitions.
Same house log: nettool_drag 73/73 exact, 11680 vectors; wide drag 73/73, 3504 vectors; unavailable=mismatches=resyncs=0 throughout.
Drag cause/fix: network-geometry §3.3.1 NG8 requires bounded two-end extension, including straight pipe/subway drags; shadow lacked live overlap/QI inputs, including julian 0.
Core now uses step displacements and NG8 length/bounds gates; shadow observes the original's complete overlap lists and own-layer/connection QIs. No excluded vectors.
Both house and gen27 log julian=0: nettool_drag vectors=160, exact=1/1, mismatches=unavailable=resyncs=0.
Gen27 acceptance: `close-lotwall29-gen27-r2.verdict` clean, ref_warnings=0; `close-lotwall29-gen27-r2.log` confirms all 944 actions of gen27-konradshohe-s1-default.
Same gen27 log: lotwall 25/25 exact, 2673 records, 1087 native Read transitions; narrow drag 25/25 (4000 vectors), wide 25/25 (1200); zero mismatches/resyncs.
Earlier `close-lotwall29-gen27-r1` was rejected (script outside [ini]); the distinct r2 label passed; smoke-r1 clean, 4/4 exact for all three shadows.
Method evidence: one cumulative line per compared method per month; house 144 wall lines, 288 narrow-drag lines, 288 wide-drag lines, all mismatches=resyncs=0.
Wall Read 0x005E2B40: 6466 calls; Write 0x005E2AB0: 12932 calls; both complete=0: bytes/returns only, reference/QI lifetime calls and nonsaved fields untraced.
All other retaining-wall methods, including lifecycle, setters/getters, geometry/model building and static add/update helpers, are unobserved; whole-owner 3979-byte credit is NOT established.
Drag full methods: GetDraggedCells 0x0062CA30, GetStraightStepCount 0x00407AC0, GetTotalStepCount 0x00809240; house calls=complete=11680 each (wide=3504).
DrawNetworkLine 0x0063AF40 compares return, ordered cells and counts but complete=0: other owner fields and outbound calls untraced; other network-tool methods unobserved.
Tests: wall snapshot vs load, truncation/trailing rejection; NG8 straight/staircase/two-end/type/length/bounds fixtures in the code commits.
Final gate: `build-runs/qbuild-test.log` 7894 passed; `qbuild-clippy.log` zero diagnostics; `fc4verify-build.log` PASS; `fc4verify-ctest.log` 124/124; owned Rust fmt PASS.
No ABI exports added/removed, no spec edits or questions, no coverage-map/regress-list edits. Exact record observations do not imply whole-owner coverage.
