| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gdriver5: DirectX 7 core driver | Codex, `core-gdriver5` | in progress | Read director integration and driver specification; implement and verify owner closure. |

## Handoff
- Goal: implement nSGLDX7 driver, texture and primitive managers in core with foreign device traits, conformance coverage, and owner closure.
- Worktree is reserved with detached HEAD; do not change refs. No plugin/C ABI wiring.
- Done: initial record committed 29774752; assigned renderer-driver sections and director Host/products read.
- Implemented first draft of backend, driver, texture/stage/primitive/vertex/context modules and DX7-01–18 recording tests.
- Focused first-draft Cargo run: 14 passed, 0 failed; qbuild fmt passed for eight owned files after apply_patch formatting.
- The q.py wrapper failed before Cargo (Git Bash CreateFileMapping Win32 error 5); direct Cargo succeeded.
- Follow-up diagnostic audit fixed library unload ownership and stale output adoption; added branch/failure tests, light fields, adapter text and client-rectangle setup.
- Expanded preprecision qbuild terminal FAIL: Cargo 6296/0, clippy four diagnostics (fixed), build 217/217, CTest 68/68; tree changed during queued native run. Session 6440 completed; do not recollect.
- Preserved failed clippy log: build-runs/gdriver5-preprecision-clippy-failed.log; original qbuild-test.log is preprecision evidence.
- Independent rational probe found f64 projection mismatch: W=30, m0=3e2f59ff, m3=40a4645f, expected 30888889 versus f64 30888888. Added binary80-significand implementation and discriminator tests.
- Added known compressed descriptor preference/fallback; ordinary ranking/converters remain RD6.
- Corrected precision/compressed-preference focused run: 24 passed, 0 failed (session 61784 completed).
- Final audit corrected placeholder release, null failure write hazard, capability query in primitive Init, upload unlock cleanup, persistent discard after failed wrap lock and restore-failure capacity preservation; added tests.
- Next: format nine owned files and run final qbuild all on stable source; collect its exact async operation; update report with measured results.
- New questions RD9–RD11 appended to renderer-driver.md questions; existing RD6/RD7/RD8 explicit boundaries.
- No game requests outstanding. No plugin/capi integration, so game checks are not applicable to this core-only draft.
- Source is not yet compiled or owner-closed; do not enqueue ready.
