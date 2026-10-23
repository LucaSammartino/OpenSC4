| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lighting-close5: lighting owner closure | Codex, `core-lighting-close5` | ready: owner audit and conformance coverage, gate green; LT12/LT13 exactness open | Review and land; analysis follow-up on LT12/LT13 |

## Changes

Audited lighting.md §§0–12 against the portable lighting module. All specified
owner algorithms were already present; no runtime algorithm change was needed.
Added [Owner closure](../../engine/core/src/lighting/README.md) with one row per
owner/section, named code and tests, all 22 conformance vectors, attributed-helper
coverage, foreign-provider boundaries, and explicit exactness limitations.
Updated the module documentation to point to the table.

Added four focused regressions: tint reset recomputes light and gates redraw on
the last-announced threshold; vector 11 checks stored padding versus capacity;
vector 14 checks all four missing-image loads and non-reloading Init/Shutdown;
vector 18 checks the fixed downward/zero vertical basis at 53/64-bit precision.
Code/test/doc milestone: e7487ac2. Interim validation record: 0a88f61f.

## Validation

- Focused lighting suite: **48 passed, 0 failed**. Log:
  `build-runs/q/20261023-124802-37048.log`.
- Explicit owned-file formatting: **PASS**, using absolute paths. Initial
  relative-path invocation failed file resolution; no formatter output was
  written into source. Final formatting and compilation succeeded.
- One required `qbuild.py all`, collected to its actual terminal result:
  **PASS**. Cargo **5865 passed, 0 failed, 0 ignored**, summed over all 27
  `test result:` summaries; clippy **0 diagnostics** with `-D warnings`;
  CMake/native build **217/217 edges**; full CTest **68/68**.
- Logs: `build-runs/qbuild-test.log`, `build-runs/qbuild-clippy.log`,
  `build-runs/fc4verify-build.log`, `build-runs/fc4verify-ctest.log`.
  Native verification took 1142.1 seconds including delayed build-output
  completion; the build itself reported 92.8 seconds, CTest 145.1 seconds.
- Gate emitted `[the tree changed during the run]` because the lane handoff
  was committed while it ran. Only `docs/lanes/core-lighting-close5.md` changed
  after launch; all engine source/tests were stable at e7487ac2. No repeat gate
  or duplicate operation was launched.
- No game request: core documentation/tests only, no plugin/C ABI wiring or
  runtime behavior change. No shadow comparison, game verdict or replacement
  coverage is claimed. ABI exports unchanged.

## Questions and exactness limits

No new questions filed. Existing questions are in
`docs/specs/questions/lighting.md`. Unsupported exactness is limited to the
already unsettled contracts:

- **LT12:** inherited control word and rounding modes on actual entry remain
  unproved. Exact closure stays open for **§2.2, §5.7, §5.8.2–§5.8.4**. Core
  takes explicit 24/53/64-bit register precision, models nearest-even/masked
  exceptions, and does not claim other rounding modes or exception signaling.
- **LT13:** exact NaN payload/sign/exception encodings remain open for **§5.8**.
  Unordered determinant success and quiet-NaN propagation are implemented;
  exact NaN output bit patterns are unsupported.

LT1–LT4 concern setting names or foreign consumers; their raw owner calculations
are implemented. Native pointer adjustment and unwritten constructor storage
are represented by typed interfaces and safe empty storage in the portable
core, not unsafe layouts. Invalid-buffer/index errors are safety boundaries,
not invented native recovery contracts. Whole-owner numeric bit exactness is
not claimed while LT12/LT13 remain open.

## Next steps

Review and land the audit/tests. Resolve LT12/LT13 on the analysis side before
claiming whole-owner numeric exactness. Any future plugin/C ABI integration and
in-game shadow verification are separate work; this lane adds neither.
