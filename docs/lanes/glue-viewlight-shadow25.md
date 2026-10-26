| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewlight-shadow25 (view and lighting shadow hardening) | `Codex` / `glue-viewlight-shadow25` (engine lane) | ready; cargo 7782/0, clippy zero diagnostics, ctest 107/107; viewlight25-r1-turbo-20261025a clean/ref_warnings=0 through 2032-01-17; lighting 73/73 scoped exact; viewutil-subset 73/73 (34/50 bodies), full viewutil 0/73 (existing VU12) | Resolve VU12 and extend remaining owner coverage before whole-owner VERIFIED credit. |

## Report

- Fixed object-angle negative zero by applying the specified minus to the floating angle; marked the zero-sign interpretation SPEC-GAP (existing VU19).
- Added a Rust transport regression for zero, signed unit rotations and signed-int32 extrema; focused tests 3/3 passed, owned Rust formatting passed.
- Added copied live camera-vector inputs to bounds/min/max shadows alongside their existing probe inputs; each family measured 73/73 exact.
- Lighting query unavailability now fails closed, and all 610 query fields contribute to the monthly aggregate alongside the 14 parameter fields.
- Both existing ini keys remain default-off. No public C ABI exports changed; no coverage/profile files edited.
- Full stable-source qbuild all PASS: Rust 7782 passed/0 failed, clippy diagnostics=0, native build PASS, CTest 107/107. Separate queued verify also passed 107/107.
- Run: `viewlight25-r1-turbo-20261025a`, Konradshohe, turbo to 2032; verdict clean at 2032-01-17, ref_warnings=0, resyncs=0.
- `lighting_shadow`: `viewlight25-r1-turbo-20261025a 73/73`; scope is cSC4LightingManager exemplar parameters, slot-16 presence masks and slot-17 empty projections (624 fields/capture), not whole-owner replay.
- `viewutil_shadow`: `viewlight25-r1-turbo-20261025a 0/73`; `viewutil-subset`: `viewlight25-r1-turbo-20261025a 73/73` across 34/50 SC4ViewUtilities bodies. Object-angle improved to 73/73.
- Only camera-angles and city-width differ, matching existing VU12 examples (408a3ae6/408a3ae7 and 42a73d74/42a73d75). Existing question evidence also records these differences at normal speed; they are not turbo-only findings.
- No questions filed or specs edited. VU19 now has measured exact zero-sign behavior; VU12 remains open. Fourteen view bodies remain unmeasured; lighting needs an ordered lifetime input transcript for whole-owner replay.
- Evidence: `build-runs/game-results/viewlight25-r1-turbo-20261025a.{log,verdict}`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-{build,ctest}.log`. Neither coverage-map owner row is claimed wholly VERIFIED.
