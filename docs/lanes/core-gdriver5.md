| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gdriver5: DirectX 7 core driver | Codex, `core-gdriver5` | in progress; round 2 | Run final all gate, report and enqueue. |

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
- Round 2 instruction supersedes old gate collection: never collect session 50064. Prior Cargo 6302/0 and clippy/fmt passed; native was incomplete evidence.
- Round 2 verify completed: qbuild-20261023-211813-build-ctest PASS, build 213/213, CTest 68/68. Session 15450 terminal; do not recollect.
- Fixed Enable error preservation using unchecked primitive flush (section 5.4); matrix flush already unchecked as section 8.6 requires. Added sentinel/failure, ignored/deferred capability, Init/shutdown cleanup tests and the specified Init word=7 write (purpose RD2).
- Corrected focused gate: 31 gdriver tests passed, zero failed (session 70182 terminal); explicit two-file fmt passed. Preserved pre-fix sentinel failure and corrected cleanup-test expectation logs in build-runs/gdriver5-r2-*.log.
- New questions RD9–RD11 appended to renderer-driver.md questions; existing RD6/RD7/RD8 explicit boundaries.
- No game requests outstanding. No plugin/capi integration, so game checks are not applicable to this core-only draft.
- Round 2 source milestone 3c769fe7: Enable fix and 31-test focused pass. README now maps every applicable subsection per owner; remaining boundaries are explicitly scoped. Added final matrix/no-op/blit, alias/refcount and light-field coverage; 32 tests await final all gate, two-file fmt passed.
- Next: run final stable-source qbuild all once, collect its terminal result, then commit report and write ready queue with full HEAD. No requests outstanding; no merge-queue entry yet; ABI false.
