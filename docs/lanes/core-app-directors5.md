| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-app-directors5: director owner closure | Codex; core-app-directors5 | in progress | Inspect existing modules and implement app-directors contracts |

## Handoff
- Goal: close Tools, game-data registry, ResourceLib, Game and ResMan directors under app-directors.md; core only, no ABI changes.
- Done: initial plan commit 89830a05; implementation b6e4b72a adds director state/dispatch/bootstrap, 233 literal registrations, singleton lifecycle and registry required-output QI/initial service metadata.
- Coverage: Owner closure table in engine/core/src/application/directors/README.md; all AD-V1–27 mapped, AD-V24 reuses existing registry G12.
- Focused tests: director 18/18 and registry 17/17 passed before final inventory/precondition test additions; six owned Rust files fmt clean before final one-line lint cleanup.
- Gate so far: qbuild all workspace tests PASS 6459/6459 (0 ignored), clippy PASS 0 diagnostics; final fmt PASS all six touched Rust files.
- Next: collect the SAME qbuild all operation, exec session 34946, to its terminal result; native queue request qbuild-20261023-213448-build-ctest is outstanding. No duplicate gate/request.
- Open: existing ADIR1 exact local hash order; ADIR2 exceptional singleton construction (non-Tools); ADIR3 created-object interfaces. No new questions.
- Evidence: q.py failed to start Git Bash (CreateFileMapping error 5); log build-runs/q/20261023-212723-15396.log. Direct focused cargo tests succeeded.
- Gate/game: native CMake/CTest queued behind shared game regress; core-only change, no plugin/capi wiring, no game request needed or outstanding.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-core-app-directors5-r1-manifest.json.
