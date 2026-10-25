# S3 corrected-source result

SPIKE VERDICT: INCONCLUSIVE

Final source: entry bridge e0a12625b, cached-input repair a91f41851. Historical
rounds 1 and 2 remain separate. All acceptance numbers below pool BOTH natural
sources and the fixture source from the two final runs, not just the cleanest
natural run. The cohort remains 20, including two unsupported entry encodings.

| Label | Verdict | Ref warnings | Foreign exceptions | Date | Real seconds | DLL fingerprint |
|---|---|---:|---:|---|---:|---|
| inplace26-fixture-3 | clean | 0 | 0 | 2028-01-03 | 10.562 | 4b38a40e |
| inplace26-natural-3 | clean | 0 | 0 | 2028-01-17 | 10.812 | 4b38a40e |

Final gate: cargo 7840 passed / 0 failed, clippy 0 diagnostics, native build
48/48 edges, CTest 115/115. Owned Rust fmt: 1 file, zero diff lines. Native build
264.5 s, CTest 342.4 s. The prior source gate passed 7839 and 115/115. A separate
host focused cargo invocation was cancelled before producing a test result;
it is incomplete, not a passing or failing check. The final queued gate includes
the new non-degenerate ground-line regression test. An initial fmt invocation
used a wrong relative path; the absolute owned-file invocation passed.

| Source | Calls | Samples | Comparisons / exact | Outside signals | Refused | Mutations / detected |
|---|---:|---:|---:|---:|---:|---:|
| natural-3 natural | 170413 | 9095 | 9092 / 9092 | 2 | 1 | 0 / 0 |
| fixture-3 natural | 162446 | 8678 | 8674 / 8674 | 3 | 1 | 15 / 15 |
| fixture-3 fixture | 512 | 512 | 512 / 512 | 0 | 0 | 18 / 18 |
| Total | 333371 | 18285 | 18278 / 18278 | 5 | 2 | 33 / 33 |

1. Exact/confined criterion: **14/20 (70%), 1821 body bytes**, conservatively
   requiring no difference, outside signal OR refusal in either final run.
   Allowing clip3's refused natural aliases gives 15/20 (75%), 1872 bytes.
   These are observed confinement under the stated neighbourhood method, not
   proof of all paths or all process writes. View-viewport and world-viewport
   have zero natural calls; their 16 fixture calls each supply the evidence.
2. Teeth: **33/33 located events in 18 functions**; every installed function has
   one fixture mutation, 15 also have a natural mutation. Window 99 denotes AL;
   other windows index receiver/argument capture. Every mutation changes only
   one 32-bit word (xor 1). Normal exact counters exclude the mutation checks.
3. KILL criterion: **3/20 (15%) outside-signal functions**, **0/20 float-only**.
   Fixed, world-view and box-rectangle changed receiver-region neighbours.
   All five locations are in outside-3.csv. Writer/object attribution is unknown;
   neither the measured upper bound nor float-only count exceeds 30%.
4. Rate: metrics-final.json and metrics-final.py use actual first/evidence commit
   times, including queue/build/game waits, the interrupted check, and both
   rounds. They report 1821 qualified bytes and a 3234-byte all-function ceiling
   against the 11 KB/hour target and 1.1 KB/hour baseline. No coverage credit is
   asserted; body sizes count functions exercised, not full branch coverage.

Confined cohort: city-corner, clip2, cross, ground, primitive-indices, view-box,
view-point, view-viewport, view-world, viewport-fixed, viewport-world,
world-viewport, world-viewport2, zoom-rotation. Rotation and visible-edges fail
closed before patching; published ABI is present. Clip3's receiver/output aliases
are refused, with original native execution unchanged. Other rejected candidates
and reasons are in rejected.md. No missing Rust algorithm was implemented (0 h);
one reused adapter was repaired (11 lines), and all elapsed time is charged.

The observer compares containing committed writable private regions, not whole
allocator graphs. It cannot see writes subsequently undone, unrelated allocations,
image/global writes or excluded active stack frames. Concurrent neighbour changes
can produce false signals. A signal suppresses replay/restoration; native state
is retained. Read-only receiver/input windows are never written back. The native
bridge restores original integer registers, flags and x87/SSE state after replay.

What would decide a further attempt: attribute neighbour changes while observing
all reachable allocations, bind the two unsupported entries, cover aliases, and
measure a broader/larger-body cohort with the now-tested bridge and cached build.
This one-family cohort cannot establish throughput for UI/audio or the other
owners. Even crediting every selected byte cannot meet this round's rate target.
Every new plugin switch remains off by default; the experiment is reviewable
but does not justify replacing the month-end shadow across the engine.
