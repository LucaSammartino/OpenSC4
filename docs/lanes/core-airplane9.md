| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-airplane9: cSC4Airplane core | Codex / core-airplane9 (detached reserved lane) | ready: round 2 merge fixed; final gate PASS | Land engine/core/src/automata/airplane; resolve AF2–AF6/AF10 before complete native flight |

## Implementation and merge

The airplane owner is `engine/core/src/automata/airplane/mod.rs`, publicly
re-exported through `automata::Airplane`. It implements the settled parameter,
placement/record, independent phase/ammunition/LOITER state, CHASE rejection,
CROP_DUST PopState and isolated numerical-stage contracts. Its README maps
every assigned row to code, conformance coverage or a numbered spec question.

Round 2 merged main in `f6da6ee6d`. Helicopter remains owned by
`engine/core/src/helicopter`; both owners use the shared aircraft codecs,
placement, altitude, clipping, phase and geometry helpers. The shared
DriveParameterSource comment describes the airplane's named record correctly.
`2d291bc71` removes imports left unused after the two owners moved out.
There are 16 shared aircraft, 10 airplane and 14 helicopter conformance tests.
The airplane lane adds no C ABI exports or plugin integration. ABI 98 was
inherited from main; this lane did not bump it.

## Final validation

Stable source and HEAD during the final gate: `fe75b116f073043df2602ddbf198594d5d722498`.
The subsequent report commit changes documentation only.

| Check | Result | Evidence under build-runs/ |
|---|---|---|
| qbuild fmt, five explicit absolute Rust paths | PASS, zero diff lines | Terminal verdict; aircraft.rs, airplane/mod.rs, automata/mod.rs, conformance aircraft.rs and airplane.rs |
| qbuild all workspace tests | PASS, 7021 passed / 0 failed / 0 ignored, independently summed over 44 result lines | qbuild-test.log |
| qbuild all clippy -D warnings | PASS, zero diagnostics | qbuild-clippy.log |
| Native verification build | PASS, 48/48 edges, 81.0 s | fc4verify-build.log |
| Full CTest | PASS, 80/80, 185.3 s | fc4verify-ctest.log |
| Question index and consistency | PASS; AF11 next free | Generated QUESTIONS.md and questions.py check |

Final queue label: `qbuild-20261024-100237-build-ctest`. qbuild and native
verification both reached terminal PASS; no asynchronous operation remains.
The plugin is present at `engine/build-x86/FreeCity4.dll` (20,806,656 bytes).

The first merged gate, `qbuild-20261024-094908-build-ctest`, is distinct failed
evidence in `build-runs/r2-before-unused-import-fix/`: Rust 7021/0/0, clippy unused
imports, build 216/216, CTest 79/80. The smoke test linked Rust ABI98 against a
cached C++ object compiled with ABI97, with a train-integration source path in
its diagnostic. Both local sources declare ABI98. That run also detected HEAD
changes from progress commits; the final run kept HEAD fixed throughout.

The standalone recache diagnostic (`refresh-r2-smoke.cmd`, session 24439) hit a
cache-server startup timeout, was interrupted and collected as exit 1. Its
result is not a passing gate. `compile-r2-smoke.cmd` then recompiled only the
generated local smoke-test object directly with MSVC, exit 0. This corrected
the stale input without changing ABI declarations; the final queued gate
relinked and passed the smoke test. The failed Bash-wrapper log is retained at
`build-runs/q/20261024-095505-29352.log`.

## Questions, game coverage and next steps

Helicopter retains **AF9**. The airplane non-finite physics question is now
**AF10** in `docs/specs/questions/aircraft.md`; all airplane references and the
generated index were updated. The task explicitly authorized running
`tools/questions.py index`; its consistency check also passes.

The remaining spec-settled airplane rows were audited against §§3.1/3.4.1/3.4.2:
the existing helpers and tests already cover them. Complete flight physics,
attitude/corrections/collisions, AI control, lifecycle/effects, player commands
and dispatch still require AF2–AF6. Non-finite numerical equivalence is AF10.
No substitute owner physics or guessed lifecycle behavior was added.

No game request was made, and no native shadow or ref-warning measurement is
claimed. A game comparison needs an observation adapter for matching airplane
parameter, candidate-motion and scalar-stage inputs/outputs. Full trajectory
comparison additionally needs the unresolved owner contracts above.

Ready to land the merge fix and settled airplane owner. The coordinator should
name `engine/core/src/automata/airplane` in the implementation column and route
the remaining questions to the spec authors.
