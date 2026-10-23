| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-local-grid-helper4: portable local grid helper | Codex / core-local-grid-helper4 | ready | Coordinator may land the bounded core implementation; LGH1 remains open |

## Implementation

Added `engine/core/src/local_grid_helper.rs` and one additive module declaration
at the end of `lib.rs`, from public local-grid-helper §§1–6. H is the sole
persistent state. Init passes the actual H field to raw creation and ignores
status. Show queries every request and uses only request/query low bytes.
Shutdown clears H before Release; explicit consuming destruction releases only,
with no query, Stop, field clear, or hidden Rust Drop foreign call.

SetPosition guards null before all service/position reads, captures one service,
owns five independent raw scalar requests and two interleaved floor requests,
then constructs and sends its descriptor. It never caches position, cell,
visibility or scalar replies. Invalid portable qualification returns typed
Unsupported with the completed read/primitive prefix and no transform.

`QualifiedDyadicNearest` explicitly supports the requested finite dyadic domain:
positive power-of-two divisors 2^-16..2^16, bounded lattice-2^-30 x/z and raw
scalar inputs, finite raw y words, and valid integral floor replies. Binary64
stores/intermediates are exact within the proved bound (translation sums need
at most 52 significant bits). Binary32 stores use integer nearest-even rounding.
Scale flags compare the held product, so G7 stores unity while retaining flags 3.
There is no ambient native FP or declared binary32-service guarantee.

The safe semantic descriptor projects exactly 54 defined bytes into caller
backing, preserving unspecified bytes +2/+3. The host gets a temporary synchronous
view. No renderer/pixel/lifetime/native-padding inference follows.

## Independent evidence

Commit `578a1f16` transcribed all G1–G7 (28 expected words), seven B rows and
L1–L14 before implementation/output comparison. Commit `d1c3e8ce` contains the
production code and executable fixtures. See:

- `engine/core/tests/local_grid_helper_vectors.md`: independent literal ledger.
- `engine/core/tests/local_grid_helper.rs`: primitive replies and ordered observations.
- `engine/core/tests/local_grid_helper_domain.md`: domain proof and applicability of every G/B row.

Focused tests passed: 27 integration tests and 2 unit tests (terminal RC=0 each).
These check all public G/B/L cases, actual direct H output address, asymmetric
fresh replies, signed zeros, held-versus-stored flags, unchanged/hidden moves,
clear-before-release versus release-only destruction, nine identity words and
54 defined descriptor bytes, nearest-even ties/carry, domain extrema, and
Unsupported prefixes. Logs: `build-runs/q/20261023-064903-27548.log` and
`build-runs/q/20261023-065011-38356.log`.

## Canonical gate

Exactly one `qbuild all`, on stable committed source, completed with terminal
RC=0. No baseline or extra quick build was run, per the task-specific instruction.

| Check | Measured result |
|---|---|
| Owned-file qbuild fmt | exit=0, both owned .rs files |
| Cargo workspace | 5528 passed, 0 failed, 0 ignored; sum of all 27 result summaries |
| Clippy -D warnings | exit=0, diagnostics=0 |
| Native build | PASS, 217/217 edges, 103.4 s |
| Full CTest | PASS, 68/68, 0 failed, 139.16 s (wrapper rounds to 139.3 s) |
| fc4verify | PASS, 377.0 s |
| Canonical command | actual terminal exit=0 |

Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-configure.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
The same async invocation was collected through actual exit; no cancellation,
restart, process kill, cache RPC, daemon or environment/config intervention.
Native/CTest completed normally; no stats stall was established. A diagnostic
CIM metadata query was denied; no success claim rests on inferred process links.

Lane start HEAD: `15aced92`; initial engine tree:
`27a710dc5a82c241b40f9038c8fb196375e526f3`.
Canonical start/end HEAD both `8692985356380300a2bfd1689c441d7135e0cde5`;
start/end engine tree both `3eb3bf90a56ee930b7f579b731c4e56f43629ea6`.
All five owned engine file SHA-256 values matched before/after; recorded in
`build-runs/local-grid-helper-content-before.json`. Git status was clean at
canonical exit. Only this report changes after that gate.

Formatting used explicit owned files, with no shared-lib reformat. Relative
qbuild fmt paths initially failed; absolute paths passed. q.py single-string
Bash invocation was denied, so focused tests used its direct argument-vector
mode successfully. Detached commits emitted packed-refs permission diagnostics
but succeeded; no shared refs/config/hooks were changed.

## Limits, questions and next steps

No new questions filed; existing LGH1 remains OPEN. No game requests or shadow
runs: all proposed native vectors remain UNEXECUTED. Ref-warning/shadow-exact
requirements are not applicable to this core-only lane. Native build/CTest
results are build/test evidence, not executed-original helper-vector evidence.

No C ABI exports, CAPI/plugin code, defaults, maps, foreign owners, callers,
region behavior or global FP environment changed. No whole-byte or native
helper-coverage credit is claimed. Broader numeric/exception/reentrancy domains
remain Unsupported. Ready for the coordinator to land this bounded core change;
future integration/native comparison must retain LGH1 qualification.
