| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-automata-groups4 (portable group owners) | Codex / `core-automata-groups4` (engine lane, detached HEAD) | WIP: 13 explicitly published distinct bodies implemented; full gate at `3f4e7562` cargo 4307/0, clippy 0 diagnostics, ctest 67/67 PASS; 14 owner tests and 3 vectors; OG3/OG4 filed; no ABI or game binding | (1) Publish OG3/OG4 contracts; (2) reconcile the actual 15-body owner inventory before owner credit; (3) coordinator regenerates the question index. |

## Result and scope

Implemented the explicitly published portable `cSC4OccupantGroupTemplate` and
`tKeyEffect` bodies in `engine/core/src/occupant_groups/`. The maintained
per-owner complete/missing checklist is `engine/core/src/occupant_groups/README.md`.
It enumerates every published entry and its tests. Counting the shared true
filter body once gives 13 explicitly named distinct bodies; the brief claims
15 functions / 1662 bytes across two owners without publishing that exact list.
**WIP: do not grant 15/15 function, byte, or complete-owner credit pending OG3/OG4.**

The module owns group state/defaults, hierarchy queries, own-ID filtering,
temporary RNG sequencing, lazy/stale ID caching, key-effect deep copying and map
clear, owned destruction order, deleting-destructor flag handling, unsigned
reference-count transitions, known interface identity mapping and the inherited
version-3 base record. Getter/cache/draw state uses locks/atomics, with no RefCell.
Parent links and controller identities do not acquire/release COM references.
Foreign providers supply only property values, random draws or opaque owned
parameter blocks; no owner result is delegated.

The primitive audit reused `rng`, `seeding` and `stream::ResKey`.
`automata/script.rs` is a provisional name/ID/controller facade with replacement
choices, so it remains untouched. No Lua importer, TTT evaluator, script-manager
registry, container reference policy, built-ins or linking implementation is
claimed. No C ABI/plugin changes, native allocator/variant binding, shadow,
replacement, saved-city registration, or game requests were added.

## Validation

- Final **frozen-source** `qbuild all` at `3f4e7562`: **PASS** cargo workspace
  **4307 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, native build
  **4/4 edges** (73.4 s), ctest **67/67** (126.1 s); fc4verify PASS (201.7 s).
- Fourteen owner tests discriminate constructor defaults, zero/self/ancestor
  queries, null depth, cache staleness/empty rebuild/duplicates, exact f64 filter
  boundaries and draw suppression, NaN/infinity/zero/negative shares, seed
  fixups/sequences, low-byte commands, independent string/flag copies, unchanged
  failure output, owned destruction order, deleting flags, unsigned Release
  boundaries, concurrent unique releases/single destruction and second-thread
  getters. Base-record malformed/truncated versions are covered too.
- Three independent public vectors: `occupant_groups_chain_cache`,
  `occupant_groups_filter_boundaries`, `occupant_groups_key_copy`. They start
  from already-imported owner states and do not claim §10 import conformance
  items 1–5 or 9. A negative-runner test rejects incorrect expectations and
  unknown operations/queries. The workspace turbo-off test passes; injected
  shared turbo controls execute through the existing harness.
- Focused quick PASS: core+conformance **3629/0**, dependent crates compiled,
  native build **4/4**. An earlier turbo-off runner integration failure was
  fixed before this result.
- First full diagnostic run at `8b00441a`: cargo **4306/0**, ctest **67/67**,
  but two clippy errors in Release (deprecated atomic helper and arithmetic
  idiom). Fixed with a compare/exchange loop compatible with the declared Rust
  1.75 minimum; focused clippy then passed. The observed failure justified the
  final full rerun; no pre-edit full baseline or edit/build race was used.
- Formatting checks pass for the owner module/tests and conformance owner/main
  runners. The two shared root `lib.rs` files retain preexisting ordering and
  wrapping drift (also verified on baseline snapshots); their module-list edits
  remain additive, without mass reformatting. `git diff --check` passes.
- Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`. First failed-gate evidence is
  retained in `build-runs/gate-8b00441a-{test,clippy,ctest}.log`. No build or
  game request remains outstanding. Game verdict/ref-warning/shadow counts: n/a.

## Questions and next work

Appended **OG3 and OG4** to `docs/specs/questions/occupant-groups.md`:

- OG3 requests the actual 15-body per-owner inventory and any missing
  `tKeyEffect` lifecycle contracts; no methods were inferred from its name.
- OG4 requests explicit inherited initial-count/overflow and QueryInterface
  acquisition/failure/output contracts. Current initial count zero is the
  commented literal reading of the shared-base description; the QI helper
  provides identity mapping only and does not invent acquisition side effects.

OG1 (manager table order) and OG2 (playback bone default) remain outside these
owners. Empty stored bones are preserved. The coordinator must regenerate/check
the question index: this clean-room lane cannot run `tools/questions.py` or edit
the generated index. Await published contracts, then reconcile the owner
checklist before claiming complete-owner coverage. No self-restart is requested.

## Commits

- `1d9cec5a`: substantive owner state/algorithms/lifetime and initial tests; OG3/OG4.
- `8b00441a`: independent vectors, base-record/identity tests and owner checklist.
- `3f4e7562`: Rust-1.75-compatible Release fix and concurrent-release test; final gated sources.

This report-only commit follows the gated source revision. The worktree remains
detached as reserved; the supervisor publishes commits to `core-automata-groups4`.
