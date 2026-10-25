| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| spike-inplace26 (S3 round-3 marginal oracle) | `Codex` / `spike-inplace26` (engine lane) | READY; SPIKE INCONCLUSIVE. Code f93c38824, evidence 04d0fff4d. cargo 7841/0, clippy clean, CTest 115/115. Both r3 runs clean/ref_warnings=0; 273/273 comparisons exact, 8/10 functions qualified; byte rates unknown. ABI unchanged; questions none. | Coordinator review: larger-body ABI/size facts would decide the marginal-rate criterion. |

Round-3 start UTC: 2026-10-25T13:36:50Z (all elapsed time, including waits and fixes, is charged).
SPIKE VERDICT: INCONCLUSIVE
Exact/confined criterion PASS: 8/10 (80%), 273/273 comparisons exact, zero outside signals AND zero refusals across both runs; 2 unbound entries retained in the denominator.
Teeth criterion PASS: 10/10 deliberate one-value errors detected/located in 8 new functions, with zero pre-existing mismatches; scalar window 6, BGRA window 2, vectors window 1, all byte 0.
Rate criterion UNKNOWN: actual marginal and cumulative rates lack published body-byte counts; 0 known new bytes is a lower bound, not a zero-byte implementation.
Known-byte lower bounds: marginal 0 KB/h; cumulative approximately 0.60 KB/h using the accepted 1821 earlier bytes and all time since 11:20:12 UTC; target 11 KB/h, baseline 1.1.
The >=8 KB cohort target was not established. Verifier closes the interval at this final commit, including reporting; it prints precise lane-hours and cumulative lower bound.
KILL criterion not met: outside signals 0/10, float-only differences 0/10. Confinement is net changes in watched neighbourhoods, not whole-process confinement.
Totals: 361 calls, 273 samples/comparisons/exact; natural 233 calls/145 exact samples; fixture 128/128 exact. No ordinary differences.
Game: inplace26-r3-fixture-1 and inplace26-r3-natural-1 clean/ref_warnings=0/foreign_exceptions=0; identical DLL fingerprint 39944fb6.
Natural reach: only lighting vertex colours/global light (192/41 calls); six other qualified functions rely on fixtures. IsNight/system-country installations declined, reason unsupported-entry-or-busy.
Gate: cargo 7841 passed/0 failed, clippy zero diagnostics, native 49/49, CTest 115/115; new Rust fmt 2 files/0 diff; focused native-layout test 1/1.
Gate reported a changing tree: only evidence/test-data changed after engine-code commit f93c38824; engine/core, capi and plugin remain byte-identical to that commit.
Adapters/facility: 252 newly added physical lines, charged completely to this cohort; 25.2 per selected function, 31.5 per installed/qualified function; equal allocation detailed in adapters.csv.
New replay handles native lighting offsets, counted/strided BGRA windows and EAX/AL returns in isolated copies; no round-2 camera word transport or Rust code changed.
Larger bodies considered first: published 1978-byte SetOrientation and 1669-byte UpdateView lack complete entry contracts; UpdateView also requires foreign services. Other exclusions in round-3/rejected.md.
RefPack buffer-role ABI, CalcShadowTerrainRect's unnamed arguments, Fullpath convention and HTML native set bindings remain unpublished; BuildFirstSets explicitly writes nine global trees. No speculative calls made.
Capture watches writable committed private containing regions, 64 MiB cap; misses undone writes, unrelated allocations, image globals and active stack frames; concurrent writers remain unattributed.
All switches default off; replay leaves original native state installed. No missing Rust production algorithm was implemented; no public C ABI/default/coverage changes.
Evidence: engine/tests/spike-evidence/spike-inplace26/round-3/; raw value mirrors/checker in build-runs/spike-evidence/round-3/; verifier --raw build-runs/game-results matches both logs.
Accepted round 2 remains unchanged: 14/20, 1821 bytes, 33/33 errors, 0.875247 KB/h; reproducible evidence remains in the parent folder (85cb9df19).
Questions: none. Next: coordinator decides whether complete larger-function ABI/size facts justify another timed cohort; no new coverage credit claimed.

