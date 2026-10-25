| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-pedestrians-oneshot13 (PN1 one-shot play-count consumer) | `Codex` / `core-pedestrians-oneshot13` (engine lane, detached HEAD) | ready; implementation 86cf7b9c2; focused cargo 32/0; qbuild all PASS: workspace cargo 7505/0/0, clippy 0 diagnostics, native build 219/219, ctest 81/81; 22 PN1 vectors passed, 29 owner rows closed, fmt PASS; ABI unchanged; no game runs or new questions | 1. Supervisor publishes the detached commits to the reserved lane branch. 2. Review and merge the queued tip. |

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
owned Rust files. Workspace Clippy passed with exit 0 and 0 diagnostics.
The required `qbuild all` operation completed PASS: native build 219/219 edges
(11.5 s), CTest 81/81 (208.8 s), native total 237.3 s. The separately requested
queued `qbuild verify` also passed: 266/266 edges (194.5 s), CTest 81/81
(288.6 s), total 485.3 s. Both asynchronous operations were collected to their
actual terminal results; no gate remains outstanding.

Native request labels: `qbuild-20261024-212419-build-ctest` (verify) and
`qbuild-20261024-231044-build-ctest` (all). Full logs are in this worktree's
`build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and
`fc4verify-ctest.log`; the focused log is `pedestrians-focused-1.log`. The native
verifier noted tree changes during the runs: only this lane report changed
after the engine implementation commit `86cf7b9c2`; engine source stayed stable.

No new questions, game requests, plugin/capi changes, C ABI changes, or coverage
map edits. PN1 is fully settled; PN2 is outside these owners and existing PN3
summary interpretations remain as documented in the module README.

## Handoff
- Goal complete: PN1 consumer closed, all 22 vectors passed, all 29 owner rows closed.
- Done: implementation and closure map committed as 86cf7b9c2; gate milestones recorded in 533e7398c, 8de0b700f, 7c0dd2da4, and 438f49b78.
- Done: focused Cargo 32/0, full workspace Cargo 7505/0/0, Clippy clean, fmt clean, queued verify PASS, qbuild all PASS with native build 219/219 and CTest 81/81.
- Next: supervisor publishes the detached commits, then reviews and merges the full tip in build-runs/merge-queue/core-pedestrians-oneshot13.json.
- No outstanding asynchronous operations or game requests. No new questions; no PN1 behavior or owner row remains open.
- No plugin, capi, ABI, or coverage-map changes. Rust source remained stable after 86cf7b9c2.
- Diagnostics retained: q.py Bash mapping error 5 in build-runs/q/20261024-210858-24196.log; direct focused Cargo passed. Relative fmt paths failed; absolute paths passed.
- Native actions used only the coordinator queue. Both native verdicts were PASS; tree-change notices were lane documentation only.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-pedestrians-oneshot13-r1-manifest.json.
