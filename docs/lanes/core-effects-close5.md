| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-effects-close5: effects owner closure | Codex / core-effects-close5 | ready: tested closed subset; full closure still awaits EF18/EF19 | Land the resource-format checks; obtain permitted answers for the remaining contracts. |

## Round 2 result

**Ready for the tested closed subset, not full owner closure.** Round 2 adds
independent effects §3 resource-format checks and expands the owner matrix.
EF18/EF19 remain open; the requested binary investigation and spec rewriting
conflict with the task's explicit clean-room prohibitions. No RE source was
accessed, no spec contract was invented, and no question was marked answered.

- `a275014b7`: hand-built binary fixture with two variable-length component
  names, all 56 opaque placement bytes, negative-zero render storage, every
  following field, exact serialization, and rejection of every truncated prefix.
  This establishes byte preservation/framing, not semantic transform decoding.
- The [owner matrix](../../engine/core/src/effects/README.md) now explicitly
  maps resource and brush/scrubber description contracts to code/tests alongside
  the parser/runtime owners. Missing queue/bootstrap/collection and transform
  contracts remain identified; full owner closure is not claimed.
- Final source gate at `a275014b7d2ae71cf1507f78bcc07e30a0c35cb8`:
  `qbuild all` **PASS**, **6810 passed, 0 failed, 0 ignored**;
  Clippy **0 diagnostics** under `-D warnings`; native build **92/92** in 76.1 s;
  CTest **70/70** in 155.2 s; native overall **PASS**, 234.9 s.
- Native queue label: `qbuild-20261024-020258-build-ctest`, terminal **pass**.
  The same asynchronous operation was collected to completion. No tracked edits
  or commits occurred during the gate. Subsequent edits only correct/report docs.
- Focused test **1/1 passed**; owned-file fmt **2 files, 0 diff lines**.
  Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`. Initial invocation/formatting
  diagnostics and their corrections: `build-runs/round2-focused-evidence.txt`.
- Game: **n/a**, no plugin/capi wiring change or request outstanding; no native
  shadow equivalence claimed. ABI exports/version unchanged.
- Questions: existing **EF18, EF19** in `docs/specs/questions/effects.md`;
  no new question, archive move, or index edit. Do not run prohibited `tools/`
  scripts from this engine lane. A separate clean-room spec answer is required.
- Next: supply the missing permitted contracts, then implement/test them; obtain
  a complete allowed fixture for the §17.13–14 corpus census if required.
  The supervisor publishes this intentionally detached HEAD to the reserved lane.

## Round 1 result

Round 1 ended **wip**. Settled runtime gaps were implemented and all checks passed;
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
the round 1 merge-queue entry was held as `wip`. Round 2 enqueues the tested
closed subset as `ready`, with the remaining gaps stated above.
