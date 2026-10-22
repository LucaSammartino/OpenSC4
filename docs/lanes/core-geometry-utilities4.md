| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-geometry-utilities4 (pure Rust geometry owners) | `Codex` / `core-geometry-utilities4` (engine lane, detached HEAD) | WIP: published behaviours implemented; cargo 4309/0, clippy clean, ctest 67/67; 20 new tests; GU2-GU6 filed; ABI unchanged; no game run or complete-owner credit | (1) Resolve GU2-GU6; (2) reconcile the 37-function inventory and exact numeric/lifecycle contracts before complete-owner credit. |

## Implementation

Commits: `1b2c8d09` adds the implementation, checklist and initial tests;
`15e6982b` records the quick gate and adds numeric boundary coverage.
The final gate tested the stable code at `15e6982b`; this report changes documentation only.

`engine/core/src/geometry_utilities/` implements every named published behaviour:
the 20 SC4GeometryUtilities functions in §§2–5; cS3DMeshBuilder's published
constructor, reset, setters, primitives, lit BGRA vertices and lazy bounds;
box corner construction and the six ordered quad faces; random directions
using the existing foundation RNGs; and text-texture generation through
injected font/surface/upload interfaces, including plain BGRA and DXT3 alpha.

The per-method checklist and provisional contracts are in the module README.
Camera and network geometry were inspected for reusable complete primitives;
their private/different ray contracts remain independent. The only existing
Rust file edited is the additive module registration at the end of `lib.rs`.

## Measured validation

- Coordinator supplied a fresh baseline; the task explicitly waived another full baseline.
- Quiet quick gate PASS: 2441 core tests, dependent crates compiled, CMake 206/206 edges. One boundary test was added afterward, so the full gate below is the final authority.
- Stable final `qbuild.py all` PASS: cargo workspace **4309 passed / 0 failed / 0 ignored**, including **2442 core tests**; clippy **0 diagnostics** with `-D warnings`; native build **4/4 edges**; full CTest **67/67** (136.8 seconds).
- **20 new tests** exercise all nine published §10 examples, asymmetric geometry, noncommuting rotation/translation, epsilon equality/adjacent values, line hits behind origins, degenerate triangles/segments, NaN box overlap, half-open rectangles, output mutation rules, clipping capacity and fallback, exact RNG draw counts/rejection endpoints, mesh face order/bounds, wrapped/tie-even colours, font restoration, and independent DXT3 bytes.
- Quiet fmt check PASS for all five new Rust files. The touched `lib.rs` fails existing ordering/layout checks; a parent-version snapshot independently fails the same check (`build-runs/baseline-lib-fmt.log`). No shared-file reformat was applied.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`; initial quick logs are `fc4verify-quick-test.log` and `fc4verify-quick-deps.log`.
- `q.py` could not start Bash (E_ACCESSDENIED); native rustfmt output was capped. The prescribed quiet build/fmt wrappers ran successfully with absolute file paths.

## Questions and remaining contracts

Appended **GU2–GU6** to `docs/specs/questions/geometry-utilities.md`:

- GU2: exact x87 associations, store boundaries, constant widths and exceptional colour conversions.
- GU3: builder reset/primitive/empty-mesh/destruction lifecycle details.
- GU4: text-texture failure outputs, restoration/resource order, integer dimensions and row pitch.
- GU5: min/max NaN/tie selection and the native 40-point scratch overflow contract.
- GU6: exact 37-entry / 7509-byte owner inventory and the assignment's transform/sorting contracts, which are absent from this spec.

The code comments identify each literal provisional choice. Numeric operations
use separate binary32 steps matching ordinary foundation r24 arithmetic;
native extended-exponent/store/transcendental equivalence remains unproven.
Safe Rust owns arrays and pixel buffers; host traits carry external resource
ownership. Complete 37/37 owner, native lifecycle, shadow and replacement
credit is withheld pending the published contracts. State is **WIP** despite
the clean gate because the assignment's complete-owner claim is unresolved.

## Runtime and next steps

ABI exports are unchanged. Game runs and shadow comparison are **n/a**:
the assignment is core-only and publishes no shadow/replacement key.
There are no outstanding game requests. Spec writers should resolve GU2–GU6,
then reconcile the checklist and add the resulting exact conformance cases.
The questions index was left untouched as required by this lane's clean-room
edit restriction; regeneration belongs to the coordinator/spec side.
