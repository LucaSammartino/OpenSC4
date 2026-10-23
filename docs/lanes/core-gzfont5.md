| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzfont5 (cGZFont owner closure) | Codex / core-gzfont5 | ready: portable destructor fix, seven owner tests, section-by-section owner table; qbuild all PASS (5894/0, clippy clean, CTest 68/68); no ABI change | Resolve existing FN18–FN22 for the remaining exact-native guarantees in sections 6–11. |

## Report

Reused `engine/core/src/fonts`. The implementation already covered the published
cGZFont slot inventory. Fixed one settled lifetime omission: `GzFont::drop` now
calls Shutdown before owned fields release the foreign renderer and face, so
ordinary Rust ownership delivers the renderer detach callback. Explicit Shutdown
and final Release remain idempotent with respect to this cleanup.

Added seven conformance tests covering destruction, Init guards and face state,
setter/cache invalidation, size gating, bitmap growth and padding preservation,
packed raster decoding and AA remapping, decoration geometry, drawing record
source offsets after bitmap growth, strict clipping and pen advancement, and
full-colour palette MRU matching. The module README has an Owner closure table
for every applicable spec section, mapping code items to existing and new tests.
The spec has no separate conformance-test heading; its embedded worked examples,
FN12/FN15/FN16/FN17 vectors and FN13 rules are mapped to executable tests.

The rasterizer, code-page converter, screen-format provider and drawing surfaces
remain foreign traits. No plugin, capi, C ABI exports/version, generated ABI
inventory, or coverage map changed. No game run was requested for this portable
core change; game and shadow verdicts are n/a.

## Verification

- Implementation commit: `b42e3f17`; verification handoff: `3f52f405`.
- Before the fix, focused owner tests: 6 passed, 1 failed, reproducing the absent
  renderer detach. Evidence: `build-runs/q/20261023-133322-9816.log`.
- Formatting passed for owned `font.rs`, `tests.rs`, and `owner_tests.rs`.
- One final `qbuild.py --wt <WT> all`, collected to terminal result: **PASS**.
  Workspace tests 5894 passed / 0 failed / 0 ignored, including core 3770/0 and
  all seven owner tests; clippy `-D warnings` exit 0, diagnostics 0.
- Native build **PASS 217/217 edges**, 120.4 s; full CTest **PASS 68/68**,
  157.5 s. Native verification total 1183.6 s (includes waiting).
- Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`,
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.

## Questions and remaining closure limits

No new questions or spec choices. Existing questions in
`docs/specs/questions/drawing-fonts.md` remain explicitly unsupported for
exact-native equivalence; the published settled behavior is implemented:

- Section 6: **FN21**, malformed/truncated encoded input boundaries.
- Section 7: **FN18**, nonfinite/out-of-i64 conversion and unmasked x87 exceptions.
- Sections 8.1, 8.2, 8.4 and dependent paragraph/map-back paths in 9/11:
  **FN22**, backing storage, lookahead and progress preconditions.
- Section 9, slot 52: **FN19**, paragraph pen and return contract.
- Section 10: **FN20**, ambient AA floating precision and foreign math evaluation.

These limits and the existing bounded/provisional adapter behavior are explained
in the module README. Full native owner equivalence awaits those answers; no
unresolved section is silently claimed closed. The next step is to resolve these
questions and update the corresponding adapters and tests. Ready for review of
the settled implementation and documented owner boundary.
