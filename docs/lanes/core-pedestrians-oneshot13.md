| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians-oneshot13 (PN1 one-shot play-count consumer) | `Codex` / `core-pedestrians-oneshot13` (engine lane, detached HEAD) | in progress; implementation 86cf7b9c2; focused cargo 32/0; workspace cargo 7505/0 including all 22 PN1 vectors; 29 owner rows closed; fmt PASS; native build 266/266 and ctest 81/81 PASS | 1. Collect remaining qbuild all stages. 2. Commit final report and enqueue. |

## Report

Implemented the answered PN1 contract in the portable core. One-shot construction
starts with unsigned byte count 5. Successful Init skips null models, writes only
animation speed, then selects a count with the wrapping unsigned inclusive RNG
span; failure retains the count and speeds. The count accessors have no playback
or lifetime effects.

The one-shot animation tick updates opacity first, advances live models in list
order using milliseconds, and samples only the first live model's signed final
frame. It saturates the count at zero, requests ordinary deferred removal at zero,
and continues advancing later models. Held final frames charge on every direct
call, including zero-ms calls. Pause, stationary flags, pedestrian state dispatch
and animation clocks do not gate this body.

Coverage: PN1-I1–I10 and PN1-T1–T12 each have a named test in
`engine/core/src/pedestrians/tests/one_shot.rs`; the module README maps every
vector and closes all 29 owner closure rows. Focused Cargo passed 32 tests, with
0 failures, including all 22 PN1 tests. The required full workspace Cargo stage
passed 7505 tests, 0 failed and 0 ignored, summed over 51 result lines; it also
ran and passed all 22 PN1 tests. Explicit-file formatting passed on the three
owned Rust files. Queued native verification passed 266/266 build edges and
CTest 81/81. The remaining qbuild all stages are still pending; this report must
be finalized from the operation's terminal result before enqueueing ready.

No new questions, game requests, plugin/capi changes, C ABI changes, or coverage
map edits. PN1 is fully settled; PN2 is outside these owners and existing PN3
summary interpretations remain as documented in the module README.

## Handoff
- Goal: close cSC4PedestrianOneShot sections 3/7 consume_play_count and all owner closure rows.
- Done: setup committed as 9b09c608e; read answered PN1 and pedestrians sections 2.1/3/7/8.
- Done: implementation and README closure committed as 86cf7b9c2: byte count/default 5, null-safe SetSpeed, unsigned wrapping RNG span, accessors, own tick and first-live final-frame consumer.
- Done: added 22 individually named PN1 tests; README closure rows have no open entries and map every vector.
- Done: qbuild fmt PASS on the three owned Rust files (absolute paths required by wrapper).
- Next: collect qbuild all to its terminal result; do not relaunch it for progress. Finalize Report, commit, and enqueue only when its gate passes.
- Constraints: core only; no plugin/capi/ABI/coverage-map edits; use apply_patch for every file mutation.
- Focused operation: session 99323 completed PASS, cargo 32/0 (all 22 PN1 vectors); log build-runs/pedestrians-focused-1.log.
- Diagnostics: q.py could not start Bash (Win32 mapping error 5); direct Cargo used instead. Relative fmt paths failed, corrected to absolute; final fmt exit=0/diff_lines=0.
- Final qbuild all: exec session 76709, started on stable committed source; workspace Cargo PASS 7505/0/0 (51 result lines; PN1 22/22). Clippy still running; collect this same operation. Logs build-runs/qbuild-test.log and qbuild-clippy.log.
- Queued qbuild verify: session 3907 finished PASS; request qbuild-20261024-212419-build-ctest. Build 266/266 edges (194.5 s), ctest 81/81 (288.6 s), total 485.3 s.
- Native logs: build-runs/fc4verify-build.log and fc4verify-ctest.log. Verifier noted tree changed during run: only lane status docs changed (979eecd4d); engine source remained 86cf7b9c2.
- Native commands only through qbuild verify/queue; no pre-edit baseline full build.
- Game runs: none; pure core consumer task.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-pedestrians-oneshot13-r1-manifest.json.
