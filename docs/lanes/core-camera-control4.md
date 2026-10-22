| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-control4 (pure camera owners) | `core-camera-control4` / (engine lane) | ready: code aa772446; cargo 4277/0, clippy clean, ctest 67/67; 39 camera tests; smoke core-camera-control4-aa772446-smoke-c8f07a2e clean, ref_warnings=0; camera shadow n/a; CA3–CA6; ABI unchanged | Review CA3–CA6; regenerate question index in train; separate C ABI/plugin binding and camera shadow lane. |

## Report

Implemented the complete published pure-core method inventory for `cS3DCamera`
and `cSC4CameraControl` in `engine/core/src/camera`, registered in core `lib.rs`.
The inventory mapping every specified method to a public body is in
`engine/core/src/camera/README.md`. There are no placeholder bodies or original
same-owner method callbacks. This is core implementation completion; it makes
no original-camera replacement or camera-shadow verification claim.

The camera owns its local/view transforms, viewport/subview, orthographic and
perspective boxes, projection/inverse matrices, pixel/depth offsets, look-at,
Project/UnProject and both eye-ray forms. Existing `Xform` storage is reused
without edits. Caches are maintained by writers; getters have no interior
mutation. Both owners are Send + Sync; camera attachment uses counted Arc/RwLock
ownership, and detach/Drop releases only its acquired reference.

The control owns zoom/rotation/magnification, eye placement, ties-to-even pixel
snapping, camera-plane conversions, six-point scroll polygon, crossing/edge
sliding, commands, ordered typed view messages, lifecycle and the exact 38-byte
version-1 record. The only foreign query callback is the documented terrain
ray cast. Foreign city corners come from the published view-utilities §4.2
table (including the rectangular-city quirk) or explicit corner inputs.
The published 0.48/lift formula is unchanged; CA1 is an interpretation question.

## Measured validation

- Final code commit **aa772446**. Earlier code milestones: **399642e7** and **1f72f8d1**.
- Final `qbuild all` on committed, unchanged code: **PASS**, cargo **4277 passed / 0 failed / 0 ignored**, clippy **0 diagnostics**, CMake **4/4**, ctest **67/67**.
- Core tests: **2410/0**, including **39 camera tests**. Independent expected matrices and literal projected/unprojected points cover asymmetric perspective, subviews, offsets, bias, homogeneous thresholds, transforms/scales, literal eye-ray signs, tables, snapping, messages, records and malformed/degenerate inputs. Scroll tests cover sliding, tiny edges, low-fraction contact, outside starts and step exhaustion; roundtrips are not the sole oracle.
- Iteration quick passed **2406/0**, dependent crates compiled, CMake **4/4**, before the four final edge discriminators were added.
- New Rust module formatting: **PASS, 5 files** through qbuild fmt with absolute paths. `lib.rs` full-file fmt check exposed inherited declaration-order/export-wrapping debt (candidate check: 126 diff lines); only the new alphabetic registration was changed, without a mass reformat. Whitespace check is clean.
- Initial baseline all: cargo **4238/0**, clippy clean, CMake **206/206**, ctest **66/67**; invalidated when editing overlapped the cargo doctest. First quick failures were corrected. Candidate 1f72f8d1 passed cargo/ctest but failed clippy on the two published offset assignments; the final local exemption preserves their operand order, and the complete gate was rerun successfully.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.

## Game result

**core-camera-control4-aa772446-smoke-c8f07a2e: clean**, `ref_warnings=0`,
foreign exceptions/watchdog/RTC/overruns/Rust panics/structured exceptions **0**.
Konradshohe, turbo 1, requested end date 2026-04-01, ended **2026-04-07**,
**1.812 s** in city, slot 1, DLL SHA-256 prefix **c2e70d84**. Shutdown completed.
Exactly one unique request was submitted and waited using the local verdict
helper, without qwait; no outstanding request remains. Verdict and log:
`build-runs/game-results/core-camera-control4-aa772446-smoke-c8f07a2e.{verdict,log}`.
This checks DLL compatibility only: **camera shadow n/a**, because this lane
adds no C ABI or plugin binding. No runtime camera coverage is claimed.

## Questions and next steps

Appended **CA3–CA6** to `docs/specs/questions/camera.md`; every associated
literal choice is marked in code. CA3 covers inferred defaults, degenerate
look-at and the unnamed perspective entry; CA4 covers lifecycle/save/input
edges; CA5 covers command details and message delivery; CA6 covers the inferred
outside-start/slide rule and failed ground intersections. No spec formula was
changed. Only the questions file was edited under docs/specs; the train should
regenerate its generated index.

No C ABI, plugin, tools, generated ABI entries or existing math files changed.
Review the questions and perform C ABI/plugin integration and real camera
shadow verification in a separate lane. Reserved worktree remains detached;
the supervisor publishes the final HEAD to `core-camera-control4`.


### Coordinator review for train79

The source report calls the published method inventory complete. Complete-owner coverage is WITHHELD. The independent 2026-10-22 re-camera-contracts4 audit now resolves CA4–CA6 and CA3's defaults/look-at portions in camera.md, with materially different native rules: box ±1 and remembered up zero; partial record mutation; retained initialised on detach; integer-pair magnification reset before rejection versus rotation preservation; synchronous live-pointer message callbacks; fixed directed scroll half-planes, no outside-start projection, and update messages after ground failure. It also corrects the prior yaw base to −π/8. CA3's perspective setter/counter entry remains unidentified. These findings require implementation reconciliation; the earlier 39 tests are not independent native evidence. This batch remains an unbound pure-core building block, without complete-owner or native replacement credit. Camera smoke checks DLL compatibility, not camera fidelity. See docs/lanes/re-camera-contracts4.md for proof scope and independent discriminators; later native shadow/replacement validation is still required.
