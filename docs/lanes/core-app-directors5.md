| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-app-directors5: director owner closure | Codex; core-app-directors5 (reserved lane, detached HEAD) | ready; core owners and AD-V1–27 covered, full gate PASS | Coordinator review and land; retain existing ADIR1–3 boundaries |

## Result

Implemented the settled app-directors.md contracts for cGZToolsCOMDirector,
cSC4ResourceLibCOMDirector, cSC4COMDirector and cGZResManCOMDirector in
`engine/core/src/application/directors/`. Reused the existing
`game_data_registry::Registry` for cSC4GameDataRegistry; added required-output QI
and the constructor's service ID, service/tick priority and running-state getters.
No plugin or capi wiring, ABI export changes, ABI version changes or coverage-map
edits. The reserved worktree intentionally remains detached for supervisor publication.

The director implementation owns shared identities and wrapping/nondeleting
counts, COM/path initialization, descendant-before-parent child insertion,
class registration and child-first dispatch, factory failure reference ordering,
enumeration phases, unload logic, explicit destruction and singleton phases.
ResourceLib and ResMan preserve their different COM sources and ignore Boolean
results where the spec tests output pointers. ResourceLib binds both resource
types without rollback. Game wires all 14 subdirectors into the global root,
captures global COM once, immediately starts exactly ten, then registers its own
190 classes. Constructors expose 233 total class/factory pairs (2/25/16/190).
COM, children, resource manager and factory-owned services remain foreign traits.

The [Owner closure table](../../engine/core/src/application/directors/README.md)
maps every section and AD-V1–AD-V27 to code/tests. Registry AD-V24 reuses the
existing G12 shutdown test. Independent spec fixtures check the entire Game
Appendix A, ResMan inventory and ResourceLib editor inventory; literal fixtures
cover Tools and ResourceLib's remaining five entries. The final full suite
includes 19 director tests and all 17 registry tests, including G1–G14.

## Verification

- Required `qbuild.py --wt <WT> all`: PASS, collected to actual terminal exit 0.
- Cargo workspace: 6,459 passed, 0 failed, 0 ignored, summed by qbuild over the
  test-result lines; `build-runs/qbuild-test.log`.
- Clippy `-D warnings`: PASS, 0 diagnostics; `build-runs/qbuild-clippy.log`.
- Native CMake build: PASS, 220/220 edges; `build-runs/fc4verify-build.log`.
- Full CTest: PASS, 69/69, 0 failures (163.07 s);
  `build-runs/fc4verify-ctest.log`.
- Explicit-file fmt: PASS, six touched Rust files, 0 diff lines.
- One full gate was launched. Native queue label:
  `qbuild-20261023-213448-build-ctest`; no outstanding operation remains.
- The verifier reported "tree changed during the run" because commit 370e6290
  updated this lane's progress note while native verification waited. The tested
  source is b6e4b72a; `git diff --stat b6e4b72a HEAD -- engine docs/specs docs/abi`
  was empty after the gate. Subsequent changes are report/closure-table prose only.
- Preserved environment failure: q.py could not start Git Bash (CreateFileMapping
  error 5), `build-runs/q/20261023-212723-15396.log`; direct focused cargo tests
  succeeded. The first fmt invocation used relative paths, then absolute paths
  diagnosed width formatting; formatting was corrected with apply_patch and the
  final explicit-file check passed. Neither failure was a full build attempt.
- Real-game runs: n/a. This task adds no plugin/capi integration or shadow;
  no game requests, game files or live replacement claims.

## Open contracts and questions

No new questions were filed. Existing questions remain explicit:

- ADIR1, §2.3 for all four directors: exact native local hash enumeration order.
  Portable ascending-ID iteration preserves the specified class sets and phases.
- ADIR2, §§5.1, 6.1, 7.1: exceptional singleton construction retry/cleanup for
  ResourceLib, ResMan and Game. Tools' separately specified retry is implemented.
- ADIR3, §§5.1/5.4, 6.1, 7.2: unspecified created-object interfaces and algorithms,
  including ResourceLib class 69B6F01C. All registration/dispatch boundaries are
  implemented; decoder/editor/drawing/IO algorithms remain their own owners.
- Existing linked registry GDR1–GDR5 and decoder questions are retained.

The settled director and registry behavior has no remaining implementation gap.
Exact hash-order shadowing, the three exceptional getter policies and unspecified
created-object algorithms await the existing spec questions, not invented rules.

## Commits and next steps

- 89830a05: initial lane record.
- b6e4b72a: implementation, tests and owner table (stable tested source).
- 370e6290: Rust gate pass and pending native verification record.
- Final report commit records the completed native gate and queue readiness.

Coordinator: review the owner table, publish the detached tip to the reserved
lane branch and land it. Claim coverage rows after review. No ABI bump needed.
