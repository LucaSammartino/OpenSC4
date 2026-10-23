| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gdriver5: DirectX 7 core driver | Codex / `core-gdriver5` (engine lane) | ready; cargo 6307/0, clippy 0 diagnostics, CTest 68/68; ABI unchanged; RD9-RD11 open | (1) Supervisor publish detached commits and review owner closure. (2) Resolve spec questions before production backend integration. |

## Implementation

`engine/core/src/gdriver` implements the specified DirectX 7 owner state in portable
Rust: render states, sticky errors, scenes/loss/restore, modes, context/presentation,
matrices/viewports, texture names/surfaces/uploads/stages, primitive batching and
vertex-buffer ownership. `backend::Device` extends the existing director `Host`;
foreign operations use its 32-bit identity and memory/reference contracts. The
director's foreign DX7 factory remains the integration boundary. No Windows APIs,
plugin or C ABI wiring were added, and no ABI exports/version changed.

The module README's **Owner closure** table maps each applicable spec subsection
for cGDriver, cPrimitiveManager and cTextureManager to code and named tests. It
distinguishes implemented behavior from unresolved behavior and fixture inputs.
Known D3DLight constructor fields are represented; auxiliary operations remain RD7.

Round 2 source milestones:

- `3c769fe7`: Enable now ignores the primitive flush result, preserving the sticky
  error across failed queued draws and state calls as section 5.4 requires.
  Disable caps 0-4 still replaces it using the state-call result. Added failure,
  no-op/deferred capability and lifecycle cleanup tests, plus the known Init
  word write of 7 (its purpose remains RD2; constructor value is unspecified).
- `99246501`: expanded owner closure mapping and tests for all interface aliases,
  shared reference counts, matrix modes/unchecked errors, viewport reset/clamping,
  unsupported blits and known light fields. There are 32 gdriver tests, including
  the named recording-backend vectors DX7-01 through DX7-18.
- Prior source milestone `6e5bda0d` includes the software 64-significand-bit
  projection accumulator and failure/lifetime fixes. The independent rational
  discriminator W=30, m0=3e2f59ff, m3=40a4645f expects 30888889; f64 gives
  30888888. The implemented accumulator passes that discriminator.

## Measured checks

Final stable-source command: `python C:/Users/tlori/FreeCity4/.claude/lane-bin/qbuild.py --wt C:/Users/tlori/FreeCity4/.claude/worktrees/core-gdriver5 all`.
Operation 92089 reached terminal **qbuild: PASS**; no duplicate operation was launched.

| Check | Result | Evidence under this worktree's build-runs/ |
|---|---|---|
| Cargo workspace | 6307 passed, 0 failed, 0 ignored; sum of all 34 result rows; includes 32 gdriver tests | qbuild-test.log |
| Clippy with warnings denied | PASS, 0 diagnostics | qbuild-clippy.log |
| Final explicit owned-file fmt | PASS, 2 files, 0 diff lines (mod.rs/tests.rs); earlier 9-file check passed | qbuild fmt output |
| Native build | PASS, 50/50 edges, 96.4 s | fc4verify-build.log |
| Full CTest | PASS, 68/68, 164.5 s | fc4verify-ctest.log |
| Queued native verdict | PASS, 265.2 s | build-results/qbuild-20261023-213110-build-ctest.verdict |

The requested initial `verify` also completed: label
`qbuild-20261023-211813-build-ctest`, build 213/213 and CTest 68/68, overall PASS
115.3 s. Its session 15450 is terminal. Session 50064 was not collected in round 2.
All operation IDs above are finished; no request is outstanding.

Failed evidence is distinct from the final gate: the pre-fix Enable regression
failed with error 1 instead of sentinel 6 (`gdriver5-r2-enable-before-fix.log`).
A closure test initially expected one device release instead of two separately
owned context/texture-manager references; its expectation was corrected
(`gdriver5-r2-cleanup-test-before-correction.log`). The corrected focused run
passed 31/31 before the last test expansion. Round 1's preprecision clippy
failure is preserved in `gdriver5-preprecision-clippy-failed.log`; its intermediate
gate/source snapshots are not claimed as final-source evidence.

Game runs and shadow comparison: **n/a**. This portable core change has no plugin,
replace key, shadow key or C ABI integration. Recording foreign-device tests are
conformance evidence, not measured in-game equivalence. No game request was made.

## Questions and remaining boundaries

RD9-RD11 were filed in `docs/specs/questions/renderer-driver.md` and remain open:

- **RD9:** sections 2, 4.1, 4.4 and 10.1: complete constructor/reset values,
  final-release destruction, Shutdown's unnamed helper, adapter-info/fallback.
  Unknown stage defaults and helpers use an explicit Unsupported boundary.
- **RD10:** sections 8.2-8.3: absent compacted vertex elements, incomplete quads
  and aggregate queued-failure selection. Code comments document the literal
  implemented policy without presenting it as settled native behavior.
- **RD11:** section 8.6: FPU control-word, exceptional values and NaN payloads.
  Finite conformance uses software 64-bit-significand round-to-nearest-even;
  exceptional behavior is not claimed bit-exact to the native FPU.

Existing spec boundaries remain: **RD6** in sections 9.2, 9.4 and 9.6 (ordinary
format ranking/converters), **RD7** in sections 12.1 and 14.1 (auxiliary
interfaces/light range), **RD8** in sections 4.3 and 14.2 (device/depth selection).
These are explicit foreign resolution/Unsupported boundaries, not invented
native results. **RD1** retains caller intent while the null-write hazard is
represented by Fault; **RD2** retains the Init word's purpose; **RD3** retains
unknown descriptor meanings; **RD4** retains the external loss-message handler.
Specified writes and message dispatch are implemented despite those meaning gaps.

## Next steps

The supervisor can publish these detached commits to the reserved lane branch,
review the README owner table and claim coverage rows. Resolve the questions above
before production backend/plugin integration. No coverage-map edits, main merge,
push or shared-ref changes were made by this lane. The ready merge-queue entry
uses the full actual HEAD of the final report commit.
