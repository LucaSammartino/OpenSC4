| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-rng4 (random numbers and seeding native contract audit) | `glue-rng4` / `glue-rng4` (engine lane; detached HEAD) | Blocked on published native contracts; findings committed in efcc008d. RN4/RN5 filed. No engine/plugin/ABI changes; cargo/clippy/ctest and game n/a. No native shadow or replacement credit. | (1) Publish RN4 fixture and RN5 insertion contracts; (2) prove independent native-original sequences; (3) implement and exercise opt-in rng only if supported. |

## Result

The current published contracts do not permit a native-original fixture or real insertion for the exact owners
`cRZRandom` and `cRZRandomSuper`. This is a contract blocker, not evidence that those owners are intrinsically
irreplaceable. No plugin or core code was changed. New glue owners: 0 of the permitted 3. No same-name
simulation/renderer RNG was substituted and no verification/replacement coverage is claimed.

The reserved worktree was clean at baseline `1c4c54d2`. Commit `efcc008d` records the blocker and files
[RN4 and RN5](../specs/questions/foundations.md). HEAD remains detached; the supervisor publishes the commits.

## Public evidence

- [Foundations section 6](../specs/foundations.md) specifies both algorithms and section 6.3 specifies independent
  per-instance and process-wide seed ownership. The addresses in those behavioral descriptions do not publish
  a complete construction/acquisition, calling, storage, lifetime, disposal or insertion contract.
- Section 6.4 requires identical forced seeds for exact differential sequences. Section 8 item 6 still asks
  for native runtime captures confirming the RNG vectors. Existing Rust/spec vectors are not that capture.
- [The SDK advisor interface](../../engine/third_party/gzcom-dll/gzcom-dll/include/cISC4AdvisorSystem.h) line 45
  exposes `intptr_t GetRandom(void)` and explicitly describes an unrecreated RZRandom. It publishes no typed
  RNG operations, independent instance creation or seed installation. This opaque handle alone cannot establish
  an independent known-state native fixture, and it supplies no substitution point.
- A case-sensitive search for `cRZRandom(Super)?` produced **0 matches** across SDK include files and the reconciled
  ABI `interfaces.tsv`, `vtables.tsv`, `direct-calls.md`, `layouts.md` and `allocators.md`. These are the relevant
  published interface, native-call and lifetime/storage sources. No proposed/unreconciled ABI facts were used.
- The existing [compat adapter](../../engine/plugin/compat/FC4Compat.h) binds `LuaRandomChance` as a Lua callback.
  That callback is not a published isolated RNG instance/seed/operation API or an insertion contract for either
  exact owner. Calling it would not implement Rust RNG replacement.
- [Core RNG](../../engine/core/src/rng.rs), [seeding](../../engine/core/src/seeding.rs) and the
  [foundations conformance runner](../../engine/conformance/src/foundations.rs) already provide Rust-side
  algorithms, fixed clock inputs and sequence checks. Re-running those alone cannot establish native equality.
- A search of `FC4Config.cpp`, `FC4Config.h` and `FC4Subsystems.cpp` found **0 matches** for
  `rng`, `Rng`, `RNG` or `cRZRandom`. No new enable key, registration or forwarding replacement was added.

## Checks and game evidence

The focused contract audit is saved locally in `build-runs/glue-rng4-contract-audit.txt`; its searches above are
reproducible against baseline `1c4c54d2`. Documentation whitespace and question-ID uniqueness were checked.
No `qbuild` baseline/quick/full/fmt run was performed: no code changed, and this task explicitly forbids a blind
baseline rebuild when code is unchanged. Cargo, clippy and ctest results are **n/a**, not passes.

Game runs: **none**; no request or DLL snapshot is outstanding. Native comparisons: **0**, actual intercepted
RNG calls: **0**, resyncs: **0** (no comparison performed). No shadow exactness, ref-warning, save/reload,
shutdown, own-to-2032 or literal-39-key-union-plus-rng result is claimed. Those checks require a supported
implementation; queuing `rng` now would exercise no replacement. Defaults, maps/tools and the C ABI remain
unchanged. No game executable or prohibited analysis input was read.

## Questions and next work

**RN4** requests the published independent game-native fixture contract, including known seed/clock inputs,
complete method ABI and safe instance ownership. **RN5** separately requests the real native insertion path,
caller coverage and stream/lifetime handling. Both are appended to `docs/specs/questions/foundations.md`;
the generated question index is left for the coordinator because this lane may edit only the question file
and may not run `tools/` scripts.

After those contracts are published, first prove native-original versus Rust from identical known inputs
with zero resync. Cover raw and mixed sequences, bounds 0/1/MAX, signed range wrap, seed parity and sentinel/zero
clock paths, the MT 624-word boundary, weighted-choice fallbacks, floating endpoints and Gaussian cache behavior
where supported. Then install only a supported opt-in route, preserve all independent streams across city
loads, and obtain actual game-call evidence. Run focused checks followed by one stable full gate and the
requested own-to-2032 and literal-39-key-union-plus-rng save/reload/shutdown game checks using an immutable DLL
and unique queue labels. Until then this lane is blocked and its merge-queue state is `wip`.
