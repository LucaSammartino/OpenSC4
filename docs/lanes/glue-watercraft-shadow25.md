| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-watercraft-shadow25 (partial watercraft shadow) | `Codex` / `glue-watercraft-shadow25` (engine lane) | Ready measured work: cargo 7782/0, clippy clean, CTest 107/107; watercraft_shadow exact 73/73, factory probes; movement unmeasured | (1) Land measured shadow. (2) Add native movement/path fixture before full row credit. |

## Report
Added default-off `watercraft_shadow` / `FC4_WATERCRAFT_SHADOW` in the existing automata shadow. Monthly observations compare complete watercraft record roundtrips and position/destination/orientation getters; independent Rust/native constructor, SetState, PushState and PopState probes compare inherited stack counts and top states.
Live boats are enumerated through the native watercraft manager's captured references. Private factory boats receive probes without Init, animation or manager registration. All acquired references are balanced; Rust replay references are released on success and failure.

Full `qbuild.py all`: PASS. Rust 7,782 passed, zero failures/ignored across 52 suites; clippy zero diagnostics; native build 97/97 edges; CTest 107/107. Focused probe test and owned-file formatting also passed.
The initial native gate found three unchecked return channels; annotations fixed them. Failed evidence remains in `build-runs/fc4verify-ctest-failed1.log`; final gate logs are `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, and `fc4verify-ctest.log` under `build-runs/`.

Exact result: `watercraft25-gated-2032-20261025-c 73/73`.
Final turbo run: clean to 2032-01-10, ref_warnings=0, foreign_exceptions=0, rtc=0, overruns=0; DLL SHA256 prefix `49b15293`, slot 1.
Run record: `build-runs/game-results/watercraft25-gated-2032-20261025-c.verdict` and the matching `.log`; request is under `build-runs/game-queue/`.
Every observation had reachable=0, fixtures=1, samples=4, probes=3, bytes=856, mismatches=0: 292 captured fixture samples and 219 state mutations. This city supplied no reachable live watercraft at the observation points.
Earlier artifact runs `watercraft25-turbo-2032-20261025-a` and `watercraft25-final-2032-20261025-b` were also clean and exact 73/73 through 2032, ref_warnings=0.

Covered row: `cSC4Watercraft`, partial identity/codec/getters/inherited-stack surfaces under `watercraft_shadow`. AI/physics/ferry/path stepping remains unmeasured and explicitly logged as such; full VERIFIED row credit remains open. A `SPEC-GAP` comment records the missing callable native path-step boundary.
Questions: none, following this round's no-spec-edit instruction. Public C ABI exports unchanged; no ABI bump. Shadow profiles and coverage maps were not edited.
Next: coordinator review/landing of measured work; establish a native path/tick fixture before extending coverage or claiming full owner verification. The ready-queue target was delayed by the required Rust gate; all checks completed before 10:00.
