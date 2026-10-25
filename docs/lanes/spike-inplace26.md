| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| spike-inplace26 (S3 in-place oracle) | `Codex` / `spike-inplace26` (engine lane) | READY; SPIKE INCONCLUSIVE. Code e0a12625b/a91f41851, evidence 85cb9df19. cargo 7840/0, clippy clean, CTest 115/115. inplace26-fixture-3 and inplace26-natural-3 clean/ref_warnings=0; 18278/18278 comparisons exact. ABI unchanged; questions none. | 1. Coordinator review of the S3 result. Any broader experiment is optional. |

SPIKE VERDICT: INCONCLUSIVE
PASS exact/confined criterion: 14/20 (70%), 1821 body bytes, requiring zero outside signals AND zero refusals across both final-source runs; 15/20 if clip3's refused aliases are allowed.
Teeth criterion: 33/33 deliberate one-word errors detected/located in 18 functions (18 fixture, 15 natural); the two unbound entries inject no error.
Rate criterion FAIL: 7490 s = 2.080556 lane-hours (6595b1ae5 at 11:20:12 to evidence 85cb9df19 at 13:25:02 UTC); 1821 bytes = 0.875247 KB/h, baseline 1.1, target 11.
Even all 3234 selected bytes would give 1.554393 KB/h; waits, builds, the interrupted check and both rounds are charged, report-only commits excluded.
KILL criterion not met: outside signals in 3/20 (15%, writer unknown); float-only differences 0/20. Scope is net neighbourhood changes, not whole-process confinement.
Final aggregate: 333371 calls; 18285 samples; 18278 compared/exact; 5 outside-change samples; 2 refused receiver/output aliases; no unmutated numeric/boolean differences.
Game: inplace26-fixture-3 clean (512/512 fixture comparisons exact); inplace26-natural-3 clean; both ref_warnings=0, foreign_exceptions=0, DLL fingerprint 4b38a40e.
Gate: cargo 7840 passed/0 failed, clippy zero diagnostics, native 48/48, CTest 115/115; owned Rust fmt 1 file/0 diff. Earlier separate focused cargo check was cancelled, not credited.
Default-off entry bridge preserves incoming/outgoing registers, flags and x87/SSE state; seven synthetic cases cover native cleanup, expected volatile results and nesting.
Ground-line input repair in the reused Rust adapter removes the remaining boolean mismatch; SPEC-GAP derives section 3.2 lines from cached corners because their native word order is unpublished.
Adapters: 20 entry lines + shared 29-line packet map + 3-line callback + 11 Rust input lines = 63; machine-state bridge 139 lines. Missing production algorithms: none, implementation time 0 h.
Not qualified: fixed/world-view/box-rectangle have neighbour signals; clip3 aliases are refused; rotation/visible-edges fail closed on entry encoding. All other rejected candidates and reasons are in rejected.md.
Capture compares containing writable private committed regions (64 MiB cap); misses undone writes, unrelated allocations/image globals and active stack frames; concurrent changes have unknown attribution.
Outside signals suppress replay/restoration; original native state remains installed. Receiver/input windows are never written back. All switches remain off by default.
Initial fixture-1 corrupt and bridge-only round-2 mismatches remain historical, separate evidence; final-source acceptance pools all natural and fixture observations, including the mutation run's natural calls.
Evidence mirrored in build-runs/spike-evidence/ and committed engine/tests/spike-evidence/spike-inplace26/; verify-observations.py checks counters/locations, metrics-final.py recomputes rates.
A further experiment would need attributed/allocation-complete write observation, alias coverage, two missing bindings and a broader/larger-body cohort with cached builds; this cohort cannot establish UI/audio throughput.
Next: coordinator decides whether S3 merits further investigation; no engine coverage credit is claimed.
Round-3 start UTC: 2026-10-25T13:36:50Z (all elapsed time, including waits and fixes, is charged).

