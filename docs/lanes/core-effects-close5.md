| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-close5: effects owner closure | Codex / core-effects-close5 | in progress | Audit the permitted resource contracts; gate the closed subset. |

## Handoff

- Round 2 goal: close the supplied contracts and land the closed subset.
- Starting tip: `d86f0c837`; round 1 implementation and validation are recorded below.
- EF18/EF19 are still open in the permitted specs. No RE sources have been accessed.
- The task's requested binary investigation/spec rewrite conflicts with its explicit
  clean-room prohibitions; neither questions nor contracts will be falsely marked answered.
- Done: independent §3 resource-format test passed (1/1; 4453 core unit tests
  discovered); both owned Rust files passed fmt with zero diff lines.
- Done: resource/description owners added to the matrix; EF18/EF19 remain explicit.
- Next: run the final `qbuild all` once on this stable source and collect its
  native operation to completion; report/enqueue only the tested closed subset.
- No game request outstanding; no plugin, ABI, or game behavior change planned.
- Detached HEAD is intentional; the supervisor publishes commits.

## Round 1 result

State is **wip**. Settled runtime gaps are implemented and all checks pass;
the six requested owners cannot yet be claimed closed because the supplied
specs omit the EF18 contracts and EF19 qualification below. No owner coverage
rows should be claimed as fully closed from this report.

## Changes

- `ae1f3b323`: semantic parent rotation/scale/translation inputs for brush and game components; game map apply/remove host primitives with retained opaque map identities; map removal after pause undo on Stop; uint32 parameter-13 semantics; wrapping game count; explicit Unsupported queue/bootstrap boundaries.
- `06bca609a`: identity-parent direct translation path preserves the original binary32 bits, including negative zero. This correction followed review of the explicit §8.3 identity rule after the first full check completed.
- Added five focused runtime tests and one bootstrap/queue Unsupported test. Runtime tests cover both transform owners, brush RNG advancement and restart, map action order and movement, pause/expiry/Stop ordering, map identities, single-update quirks, missing simulator, and parameter width.
- Added the [six-owner section matrix](../../engine/core/src/effects/README.md), linked to the existing detailed [parser command tables](../../engine/core/src/effects_parser/README.md). Existing EC1–EC305 command and DI1–DI5 component-directory coverage is retained.
- No plugin/capi wiring, ABI export changes, ABI version change, game request, or coverage-map edit.

## Measured validation

Final gate ran on committed source at `68e905cf5`, with no tracked edits or commits
until its terminal verdict:

- `qbuild.py --wt <WT> all`: **PASS**, 6809 tests passed, 0 failed, 0 ignored; Clippy exit 0 with 0 diagnostics under `-D warnings`.
- Native build/CTest used the coordinator queue automatically through that same `all` operation: `qbuild-20261024-014615-build-ctest`, terminal verdict **pass**. Build **47/47** edges in 96.6 s; CTest **70/70** in 147.2 s; overall native **PASS**, 245.6 s. No duplicate verify was launched.
- Corrected focused runtime tests: **5/5** passed; 4452 core tests discovered. The bootstrap/queue test also passed in the workspace gate.
- Owned-file fmt: initial eight files passed; the two files changed for the identity-parent correction passed again, both with zero diff lines.
- Final full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Earlier check: 6808 tests, Clippy 0 diagnostics, native build 223/223 and CTest 70/70 passed before the identity-parent correction. Its complete logs are preserved under the corresponding `*.attempt1.log` names, with source/result boundaries in `build-runs/effects-gate-attempt1.txt`. That native run noted handoff-only commits changed HEAD during its wait; the final run has no such warning.
- Initial focused compile mistakes in test RNG setup were corrected before successful focused checks. The q.py Bash launcher failed with CreateFileMapping permission denied; focused cargo was run directly. Initial relative fmt paths failed; absolute owned-file paths passed.

Game runs: **n/a** for this pure Rust change. No outstanding request or native
shadow equivalence is claimed. Existing §17 runtime conformance vectors passed
within the workspace suite. The full shipped-directory census assertions in
§17.13–14 still lack a complete shipped resource fixture; the README distinguishes
that data-corpus limitation from the existing behavior vectors and scoped tables.

## Questions and sections still open

Both new questions are appended to [questions/effects.md](../specs/questions/effects.md).
EF17 remains untouched as directed. The generated question index was not edited.

- **EF18:** effects §6.1 only names `EffectsParser::ParseQueuedFiles`; effects-parser §1 explicitly excludes its algorithm. `EffectsBootstrap` has no assigned algorithm in the supplied spec. These two boundaries return Unsupported. Collection behavior beyond effects-parser §11.1 also remains open: effects §18.1 collection slots and the foreign ID-resolution/copy-validation boundaries in §§19.4 and 19.9. Named/anonymous component identity, replacement and retention remain implemented and tested in `script::Directory`.
- **EF19:** effects §§3, 8.3 and 9.2 do not settle the legacy placement-load mapping, renderer matrix convention and component world-position intermediate order. Semantic transforms implement the literal parent rotation/scale plus translation, with direct identity paths; native numeric equivalence and manager decoding of legacy placements remain open. The already specified parser matrix storage and rotation-store contracts in effects-parser §13 remain implemented, not excluded by this question.

## Next steps

Resolve EF18 and EF19 in the specs, then implement their newly settled contracts
and complete the six-owner closure audit. Supply a complete allowed fixture for
the §17.13–14 data census if those assertions are required for the replacement.
All work is committed on the reserved detached HEAD for supervisor publication;
the merge-queue entry is held as `wip`.
