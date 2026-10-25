| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| spike-inplace26 (S3 in-place oracle) | `Codex` / `spike-inplace26` (engine lane) | WIP; SPIKE INCONCLUSIVE. cargo 7839/0, clippy clean, CTest 115/115. inplace26-natural-1 clean/ref_warnings=0; inplace26-fixture-1 corrupt/one foreign exception/ref_warnings=0. Provisional exact/confined 7/20; verified bytes 0. ABI unchanged; questions none. | 1. Prove native entry machine-state transparency and isolate the fixture fault. 2. Repeat clean per-function natural/fixture measurements before landing. |

SPIKE VERDICT: INCONCLUSIVE
Opt-in `inplace`, `inplace_mutate`, `inplace_fixture` are all off by default; 20 native-layout bindings reuse production Rust, with no new Rust algorithms.
Code milestones: 6595b1ae5, bff92a97f, 17b7cc4da, 33875e28d; measured evidence b7a878282.
Settled-source gate: cargo 7839 passed/0 failed, clippy clean, native build 46/46, CTest 115/115; no Rust files changed (fmt n/a).
Natural run: 5,324,602 calls, 266,768 sampled, 186,518 compared, 146,979 exact samples, 39,539 differences, 55 outside-change samples, 80,195 refusals.
PASS exact/confined criterion: 7/20 (35%) provisional, below 14/20; 1031 body bytes. Fixture fault prevents independent verification credit.
KILL criterion: outside signals in 6/20 (30%, writer unknown), float-only 0/20; neither exceeds 30%.
Teeth: 11/11 error events detected/located in 10 functions before fixture abort; no final fixture aggregates are claimed.
Rate: 3064 s = 0.851111 lane-hours between first/evidence commits, including waits; verified 0 KB/h, provisional 1.211 KB/h; all-3234-byte upper bound 3.800 KB/h < 11 KB/h target (baseline 1.1).
The natural request put end_date in [ini], where it was ignored: clean 600.625 s to 2055-09-04. The fixture corrected the header and faulted at image RVA 0x003f785d, code 0xc0000005.
Unresolved mismatches: view-box, box-rectangle, world-viewport2, viewport-fixed. rotation/visible-edges failed installation; three installed entries had no natural calls; clip3 natural calls aliased the receiver and were refused.
Capture compares containing writable private committed regions (64 MiB cap); misses undone writes, unrelated allocations/image globals and excluded stack frames; concurrent outside changes have unknown attribution.
Evidence: engine/tests/spike-evidence/spike-inplace26/ (CSV, mutation locations, requests, rejected list, collector, metrics); raw numbers and run references also in build-runs/spike-evidence/.
Next: controlled register/FPU-state and trampoline tests, isolated entry replay, then clean full-cohort counters. Keep WIP; the corrupt fixture blocks ready.
