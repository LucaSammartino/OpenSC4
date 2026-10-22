| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-lighting4: lighting owners | Codex / `core-lighting4` (engine lane, detached transport) | WIP: Rust core added, 28 lighting tests; cargo 4201/4201, clippy clean, ctest 67/67; LT5-LT11 filed; ABI unchanged; no game run or whole-owner credit | 1. Resolve LT5-LT11. 2. Complete projection/terrain rectangle and conditional gaps. 3. Coordinator regenerate question index. |

## Result and scope

Added `engine/core/src/lighting/` and one additive registration at the end of `engine/core/src/lib.rs`.
The only other changes are this lane report and appended questions. No C API, plugin, ABI, tools,
generated ABI entries, game data or other owner implementations changed. Sources were the scoped
lighting specification, allowed operating documents, ABI facts and existing Rust core conventions.

## Implemented behavior

- Owned ARGB images with explicit dimensions/stride, decoded-resource pixel copy, minimum-red scan,
  64x64 test pattern and the specified horizontal sampling/row-spill behavior.
- Lighting identity/refcount model, Init/Shutdown subscription ordering and resource ownership.
  The deleting destructor consumes a Box, destroys resources for either flag, and retains destroyed
  storage only when bit 0 is clear. Ordinary Rust ownership also releases all contained references.
- Typed exemplar reads for every listed property; shared persistent statics retain absent values.
  `Parameters::shipped()` explicitly names shipped data, rather than asserting unknown static defaults.
- Day/month maps and key lists, tint/fade, wrap-month updates, threshold handling, untinted night
  decisions, dawn/dusk posts, fractions and redraw events.
- View rotations, cached axis colors (including the -y/up quirk), stale-until-view-change fade gradient,
  Lambert and map colors, model category weights, model normal sampling, byte overloads, terrain
  point/floor colors, and strided model/terrain vertex outputs.
- Rotated height/shadow grids, full square-grid hill sweep with both drift signs, unchanged-rectangle
  retention on skipped sweeps, depth, bend, shadow strength, shadow/decal parameters and overrides.
- All seven typed messages: city/view setup, deferred dirty terrain rectangles, render setting changes,
  exemplar rereads, invalidation and redisplay events. Event rectangle semantics remain provisional LT9.
- The specified affine least-squares subproblem and determinant rejection are implemented locally.
  The full projection does not call this a completed replacement.

Providers expose only primitive external pixels, properties, terrain queries and event delivery.
There is no original same-owner method-output provider or delegation of missing lighting logic.
The manager and shared statics are Send + Sync; getters contain no RefCell.

## Whole-owner completeness and exact remaining methods

**No whole-owner exactness or family coverage credit is claimed.** Of the manager's 30 slot-shaped
operations, 28 have core implementations; this is an implementation inventory, not an exactness score.

- `cImageARGB8`: every algorithm explicitly described here is implemented (pixel copy, test pattern,
  minimum red, sampling). LT7 leaves padding and unavailable-memory/failure semantics unresolved;
  safe errors replace invalid unchecked reads. Do not promote the whole owner as exact.
- `cSC4LightingManager::CalcShadowProjection` (slot 17) is **unimplemented** beyond the separately
  tested least-squares helper. It returns `ContractGap("LT10")` without modifying its output.
- `cSC4LightingManager::CalcShadowTerrainRect` (slot 18) is **unimplemented**. It returns
  `ContractGap("LT11")` without modifying its output. No guessed traversal or original-method fallback.
- `terrain_normal` implements interior reads/lower clamps; upper-edge/outside reads return
  `ContractGap("LT8")` until the numeric epsilon/rounding is specified. This also limits
  `GetModelLight` and `GetTerrainLighting` for those inputs. Rectangular `FindHillShadows` returns
  LT8; square city grids implement the stated sweep. Nonfinite drift skips the sweep.
- Constructor/QI/static initialization choices affect lifecycle and initial announcements (LT5).
  `SetTimeOfDay` uses the literal Euclidean distance (LT6); dependent tint paths share that choice.
  Init/map/minimum-red unavailable-resource policy is explicit but not proven exact (LT7).
- `DoMessage` dirty/invalidation rectangles currently use inclusive vertices and the literal inverse
  grid rotation; conversion to terrain cells and long-end redisplay policy require LT9.

## Verification

- Baseline `qbuild all`: Rust workspace tests, clippy, native build and CTest 67/67 passed.
- Development quick gate passed: 2334 core tests including the first 18 lighting cases;
  dependent crates compiled and native build passed.
- Final `qbuild all` on code commit `da6fff56`: **PASS**, 4201 Rust tests, 0 failures/ignored;
  core 2344 including **28 lighting tests**; clippy exit 0 / 0 diagnostics; native build 4/4;
  **CTest 67/67**, 112.9s (167.1s build+CTest).
- New-module `qbuild fmt`: **PASS**, all eight new Rust files. The shared `lib.rs` check detects
  inherited module ordering/line-wrap differences plus the additive registration; no mass reformat.
  `git diff --check` against the lane's original base is clean.
- Tests cover all nine numeric examples in section 11, tie rounding, row spill, map size/rotation
  quirks, both sweep drift directions, four grid rotations, threshold messages and fractions,
  ownership/drop and partial-copy failure, destructor flag combinations, dirty-operation deferral,
  model normal sampling, decal overrides, all terrain color paths and safe buffer failures.
- Early sweep tolerance was corrected from 0.000020 to 0.000050 for accumulated f32 error;
  clippy MSRV/precedence issues were corrected before the passing final gate.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log` and `qbuild-fmt.log` (where emitted by wrapper).
- Game/shadow: n/a; pure Rust core, no runtime integration or game requests. No outstanding labels.

## Questions and next steps

Appended **LT5-LT11** to `docs/specs/questions/lighting.md`; existing LT1-LT4 preserved.
Only that spec-question file was edited; the coordinator must regenerate the question index.
Resolve the numbered gaps, finish the two missing methods and conditional cases, then rerun
conformance/full gate before claiming an exact owner. Runtime glue/shadow comparison is later work.

## Commits and transport

- `7ad0ac97`: initial core implementation and question filings.
- `9e93c97a`: first passing quick-gate record.
- `4f7dc86f`: expanded terrain/message/rotation conformance.
- `2fecb479`: Rust 1.75 compatibility and clippy corrections.
- `da6fff56`: deleting-destructor allocation flag and ownership test.

Reserved detached worktree was reused; no branch switch, ref update, merge or push.
Git commits succeeded despite the nonfatal packed-refs maintenance-lock denial.
The Bash-backed q.py wrapper is unavailable (E_ACCESSDENIED); native PowerShell formatting worked.
Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-core-lighting4-r1-manifest.json`.
