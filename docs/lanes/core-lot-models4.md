| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lot-models4 (portable lot models) | `core-lot-models4` / `core-lot-models4` (engine lane, detached reserved worktree) | wip: portable decisions and explicit missing contracts; tested source `fd9def0c`; cargo 4326/0, clippy 0 diagnostics, ctest 67/67; 30 core tests, 6 vectors; ABI unchanged; no game/shadow; LM2–LM8 filed | (1) Publish LM2–LM8 contracts and reconcile LM1; (2) implement missing wall/native/cell contracts before complete-owner credit; (3) regenerate question index in an authorized coordinating/spec lane. |

## Result and scope

Implemented portable state and published rendering decisions for foundation,
retaining-wall and cell-texture models in `engine/core/src/lot_models/`.
This is a **WIP portable subset**, not complete-owner credit for the task's
3 owners / 82 functions / 11439 bytes. The public spec does not enumerate those
82 bodies, and the permitted SDK search found no owner declarations. No ABI,
maps, native analysis, CAPI or plugin source was consulted for missing behavior.

The per-owner and per-entry complete/missing checklist is
`engine/core/src/lot_models/README.md`. Providers load owned texture resources or
sample primitive lot/terrain/light data; none delegates an owner result. Getters
borrow immutable state; there is no RefCell cache. Rust ownership releases only
acquired texture leases; native reference-count initial values/order and base/pool
behavior remain contracts, not guesses.

## Changes

- Foundation: labelled five-face geometry/spans, normal lighting, water-aware
  maximum border drop, upward-rounded 16 m extension that never raises the base,
  forced rebuild, per-zoom side/top fallback, sort, highlight/cache decisions and
  ordered visible-side/top draw plans. Native vertex/UV order is not fabricated.
- Wall: explicit sampled-edge initialization, level-segment omission,
  seam-expanded gap bands, texture/fallback/sort/facing/cache decisions and flat
  flag test. Exact triangle generation returns `WallTriangleLayout`; recolouring
  remains explicitly pending because its formula and triangle construction are
  unpublished. No original wall method supplies either result.
- Cell: explicit record and corner-cycle contract, rotation/mirroring, flat-lot
  versus terrain placement with primitive lighting, inset/widened bounds,
  per-zoom FSH lookup with alpha metadata, opaque/alpha/tinted/pass decisions,
  clamp/linear-filter/tint/NoAddSigned effective combiner inputs, and flat-strip
  versus forced terrain-primitive requests. Native combiner instruction records
  and the LM1 bypass/modes are not invented.
- Six language-neutral executable `lm01_*`–`lm06_*` vectors cover all §8 examples,
  with explicit numeric expectations and added boundary cases. The negative
  runner test changes each operation's expectation and requires rejection.
- Thirty focused core tests cover next-f32 height boundaries, water precedence,
  forced/no-up rebuilds, cache order, primary/fallback failures, absent bindings,
  all facing rotations, flat/slope queries, tint and force boundaries, invalid
  input atomicity, lease replacement/drop counts and Send+Sync getters.

## Measured checks

- Initial quick: core 2449 passed / 0 failed, dependents compiled, native
  206/206 edges; 274.6 s. Only commit metadata/docs changed during that run.
- Final implementation quick: core/conformance 3649 passed / 0 failed,
  dependents compiled, native 4/4 edges; 148.4 s; no tree-change warning.
- First full attempt: workspace 4326/0 and CTest 67/67 passed; clippy found two
  constant assertions in the new tests. Commit `fd9def0c` moves those checks to
  actual returned draw states, preserving coverage and implementation behavior.
- **Corrected frozen-source `qbuild all`: PASS**, workspace **4326 passed /
  0 failed / 0 ignored**, clippy **exit 0 / 0 diagnostics**, native build **2/2
  edges (0.9 s)**, full CTest **67/67 (134.7 s)**. No source edits during the gate.
  Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- Formatting: seven newly authored Rust files pass the quiet check; the
  test-only correction also passes. The three existing registry/dispatcher files
  have only additive module/dispatch edits; their check reports 172 ordering and
  layout diff lines. Existing lists were not mass-reformatted.
- Game/shadow: **n/a**, core-only and unbound; no game requests outstanding.
  No C ABI export/version changes; generated `abi_entries.inc` not edited.

## Questions and remaining contracts

Appended only to `docs/specs/questions/lot-models.md`; existing LM1 preserved:

- LM2: complete body inventory, native/base construction, interfaces, pool and
  reference-count lifecycle.
- LM3: foundation native geometry/UV order, border enumeration, exact precision
  and step/rebuild details.
- LM4: wall cell ranges, across-cell mapping, offsets, equality/crossing triangles,
  UVs, vertex colours and flat-neighbor suppression role.
- LM5: factory namespaces/failures/retries/cache semantics, missing-binding draw
  behavior and key overflow. Literal provisional policies are documented in code.
- LM6: native cell corner/strip cycle, terrain order, colour representation and
  placement update triggers.
- LM7: native draw-state/combiner records, per-mode operations, alpha and ordering.
- LM8: numeric precision, exceptional values, coordinate overflow and invalid
  enum domains. Portable f32 inputs have explicit domain validation.

Question index regeneration is left to an authorized coordinating/spec lane:
this task permits editing only this spec's question file and forbids tools/.
Implement the missing contracts after publication, then add native integration
and real-game shadow validation in a separately authorized lane. No promotion
to complete, shadowed or replaced owners is claimed here.

## Commits and operating facts

- `2f51613f`: portable owners, public checklist, questions and first tests.
- `e3a01615`: executable vectors, formatting and expanded boundary/lifetime tests.
- `fd9def0c`: corrected full-gate test lint; final tested source.
- Reserved detached worktree retained; no branch/ref update, merge or push.
- q.py cannot launch Bash here (`E_ACCESSDENIED`); capped native PowerShell and
  qbuild work. Commit commands succeed despite a nonfatal packed-refs lock
  permission message; HEAD is the source of truth.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-lot-models4-r1-manifest.json`.
